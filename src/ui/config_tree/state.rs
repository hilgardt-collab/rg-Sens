//! Dialog state: the working model, lazily built editors, live commit and
//! revert.
//!
//! The live panel is the truth. `applied` mirrors what the grid currently
//! shows; editors hold working values; `commit_now` assembles a `PanelData`
//! from them and hands it to `GridLayout::apply_panel_data`. `snapshot` is
//! the state at open, for Revert.

use super::combo_items::ItemDisplayEditor;
use super::node::NodeKind;
use super::schema_editor::SchemaEditor;
use super::util::*;
use crate::config::AppConfig;
use crate::core::{
    global_registry, DisplayerConfig, FieldMetadata, Panel, PanelData, SourceConfig,
};
use crate::sources::ComboSourceConfig;
use crate::ui::background::Color;
use crate::ui::editor_registry::{self, DisplayEditor, EditorPage, SourceEditor};
use crate::ui::theme::{ComboThemeConfig, ThemeRef};
use crate::ui::{
    BackgroundConfigWidget, ColorButtonWidget, ComboReorder, GlobalThemeWidget, GridLayout,
};
use gtk4::prelude::*;
use gtk4::{
    gio, glib, Box as GtkBox, Button, CheckButton, DropDown, Label, SingleSelection, SpinButton,
    Stack, TreeListRow, Widget, Window,
};
use rg_sens_types::schema::TreePage;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::{Rc, Weak};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// Debounce for live commits: coalesces spin-button ticks and slider drags.
const COMMIT_DEBOUNCE: Duration = Duration::from_millis(50);
/// Poll period for editors without a change signal (today only the test
/// source widget); only the visible page is polled.
const POLL_PERIOD: Duration = Duration::from_millis(300);

pub(super) struct GeometryControls {
    pub width: SpinButton,
    pub height: SpinButton,
    pub scale: SpinButton,
    pub translate_x: SpinButton,
    pub translate_y: SpinButton,
    pub z_index: SpinButton,
    pub ignore_collision: CheckButton,
}

pub(super) struct AppearanceControls {
    pub radius: SpinButton,
    pub border_enabled: CheckButton,
    pub border_width: SpinButton,
    pub border_color: Rc<ColorButtonWidget>,
    pub color: Rc<RefCell<Color>>,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum ThemeChoice {
    Inherit,
    Global,
    Named(String),
    Inline,
}

pub(super) struct ThemeControls {
    pub choices: Vec<ThemeChoice>,
    pub dropdown: DropDown,
    /// Says whether the editor below shows a shared theme (an edit makes it
    /// private) or this panel's private theme.
    pub mode_hint: Label,
}

/// Dropdown row for `theme_ref` in `choices` (Inherit, Global, named and
/// built-in themes, then Inline last).
pub(super) fn theme_choice_index(choices: &[ThemeChoice], theme_ref: &ThemeRef) -> usize {
    match theme_ref {
        ThemeRef::Inherit => 0,
        ThemeRef::Global => 1,
        ThemeRef::Named { name } => choices
            .iter()
            .position(|c| matches!(c, ThemeChoice::Named(n) if n == name))
            .unwrap_or(0),
        ThemeRef::Inline { .. } => choices.len().saturating_sub(1),
    }
}

/// Hint shown under the theme dropdown.
pub(super) fn theme_mode_hint(theme_ref: &ThemeRef) -> &'static str {
    if theme_ref.is_inline() {
        "This theme is private to this panel. Pick a theme above to share one instead."
    } else {
        "Showing the theme this panel resolves to. Changing a colour, gradient or font \
         below makes the theme private to this panel (Inline)."
    }
}

/// The editors of one content item, shared by the item's pages (the item
/// page, its Source page and its display section pages).
pub(super) struct ItemEditor {
    pub source_editor: RefCell<Option<Box<dyn SourceEditor>>>,
    /// The source editor has no change signal and must be polled.
    pub needs_poll: Cell<bool>,
    /// `None` for a displayer without per-item display settings
    /// (css_template).
    pub display: Option<Rc<ItemDisplayEditor>>,
    /// The display editor's pages as last adopted (path and widget), so a
    /// rebuilt page can be told from one that survived.
    pub pages: RefCell<Vec<EditorPage>>,
}

impl ItemEditor {
    pub fn cleanup(&self) {
        if let Some(e) = self.source_editor.borrow().as_ref() {
            e.cleanup();
        }
        if let Some(d) = self.display.as_ref() {
            d.cleanup();
        }
    }
}

pub struct State {
    pub(super) panel: Arc<RwLock<Panel>>,
    pub(super) panel_id: String,
    pub(super) grid: Weak<RefCell<GridLayout>>,
    pub(super) app_config: Rc<RefCell<AppConfig>>,
    pub(super) window: glib::WeakRef<Window>,

    // ---- model ----
    pub(super) snapshot: RefCell<PanelData>,
    pub(super) applied: RefCell<PanelData>,
    /// Working copy of the combo source structure (groups, slots). `None`
    /// for standalone panels.
    pub(super) combo_source: RefCell<Option<ComboSourceConfig>>,
    pub(super) theme_ref: RefCell<ThemeRef>,
    /// What the theme editor held after the last programmatic push. A
    /// shared reference whose editor payload differs from this was edited by
    /// the user and becomes `Inline` at the next commit.
    pub(super) theme_baseline: RefCell<Option<ComboThemeConfig>>,

    // ---- editors (built lazily, kept for the dialog's lifetime) ----
    pub(super) source_editor: RefCell<Option<Box<dyn SourceEditor>>>,
    pub(super) display_editor: RefCell<Option<Rc<Box<dyn DisplayEditor>>>>,
    pub(super) display_needs_poll: Cell<bool>,
    pub(super) source_needs_poll: Cell<bool>,
    /// Pages of the display editor (`DisplayEditor::sections`): sections
    /// (one-part paths) and list items (longer paths), each a `Section` node
    /// under its nearest ancestor page, else Display.
    pub(super) display_pages: RefCell<Vec<EditorPage>>,
    pub(super) background_editor: RefCell<Option<Rc<BackgroundConfigWidget>>>,
    pub(super) inline_theme_widget: RefCell<Option<Rc<GlobalThemeWidget>>>,
    /// The one source settings editor a display editor shows inline (the
    /// CSS Template's Bindings page), with its slot.
    pub(super) inline_source: RefCell<Option<(String, Rc<Box<dyn SourceEditor>>)>>,
    /// Item editors by slot, built on first use by any page of the item.
    pub(super) item_editors: RefCell<HashMap<String, Rc<ItemEditor>>>,
    /// Title-bar editors by group, built with the group's page (over the
    /// frame's `group_titles`, so dropped with the Content pages).
    pub(super) group_editors: RefCell<HashMap<usize, Rc<SchemaEditor>>>,
    pub(super) geometry: RefCell<Option<GeometryControls>>,
    pub(super) appearance: RefCell<Option<AppearanceControls>>,
    pub(super) theme_controls: RefCell<Option<ThemeControls>>,

    // ---- UI ----
    pub(super) tree_spec: Rc<RefCell<HashMap<String, Vec<NodeKind>>>>,
    /// Lower-case search text; empty shows the whole tree.
    pub(super) filter: RefCell<String>,
    pub(super) root_store: gio::ListStore,
    pub(super) selection: SingleSelection,
    pub(super) stack: Stack,
    pub(super) breadcrumb: GtkBox,
    pub(super) pages: RefCell<HashSet<String>>,
    pub(super) loading: Cell<bool>,
    pub(super) commit_scheduled: Cell<bool>,
    pub(super) poll_last: RefCell<HashMap<String, String>>,
    /// Labels of the rows currently bound in the list view, by node id, so
    /// summaries can be refreshed in place without rebuilding the tree
    /// (which would drop the selection).
    pub(super) row_labels:
        RefCell<HashMap<String, (NodeKind, glib::WeakRef<Label>, glib::WeakRef<Label>)>>,
    pub(super) alive: Rc<Cell<bool>>,
}

impl State {
    // ------------------------------------------------------------------
    // Model queries
    // ------------------------------------------------------------------

    pub(super) fn is_combo(&self) -> bool {
        self.combo_source.borrow().is_some()
    }

    pub(super) fn source_type(&self) -> String {
        self.applied
            .borrow()
            .source_config
            .source_type()
            .to_string()
    }

    pub(super) fn displayer_type(&self) -> String {
        self.applied
            .borrow()
            .displayer_config
            .displayer_type()
            .to_string()
    }

    pub(super) fn group_id(&self) -> Option<String> {
        self.applied.borrow().group_id.clone()
    }

    pub(super) fn window(&self) -> Option<Window> {
        self.window.upgrade()
    }

    /// Current payload of an `Inline` reference: the themed editor's frame
    /// theme for combos, the inline widget for standalone panels.
    pub(super) fn inline_payload(&self) -> Option<ComboThemeConfig> {
        // The generic theme editor holds the payload whenever it exists
        // (standalone panels, and combos whose editor has no Theme page);
        // a widget-based combo editor keeps it in its frame theme.
        if let Some(w) = self.inline_theme_widget.borrow().as_ref() {
            return Some(w.get_config());
        }
        if self.is_combo() {
            self.display_editor
                .borrow()
                .as_ref()
                .and_then(|e| e.store())
                .and_then(|c| c.theme().cloned())
        } else {
            None
        }
    }

    /// The theme this panel resolves to right now.
    pub(super) fn resolved_theme(&self) -> ComboThemeConfig {
        let theme_ref = self.theme_ref.borrow().clone();
        match &theme_ref {
            ThemeRef::Inline { theme } => self.inline_payload().unwrap_or_else(|| theme.clone()),
            other => self
                .app_config
                .borrow()
                .resolve_theme_for(other, self.group_id().as_deref()),
        }
    }

    /// Field metadata the display and background editors can pick from.
    pub(super) fn all_fields(&self) -> Vec<FieldMetadata> {
        match &*self.combo_source.borrow() {
            Some(cs) => combo_fields(cs),
            None => self.panel.blocking_read().source.fields(),
        }
    }

    // ------------------------------------------------------------------
    // Commit / revert
    // ------------------------------------------------------------------

    /// An editor changed: commit after a short debounce.
    pub(super) fn changed(self: &Rc<Self>) {
        if self.loading.get() || self.commit_scheduled.get() {
            return;
        }
        self.commit_scheduled.set(true);
        let weak = Rc::downgrade(self);
        glib::timeout_add_local_once(COMMIT_DEBOUNCE, move || {
            if let Some(state) = weak.upgrade() {
                state.commit_scheduled.set(false);
                state.commit_now();
            }
        });
    }

    /// Assemble the intended `PanelData` from the editors.
    pub(super) fn build_data(&self) -> PanelData {
        let mut data = self.applied.borrow().clone();

        if let Some(g) = self.geometry.borrow().as_ref() {
            data.geometry.width = g.width.value().max(1.0) as u32;
            data.geometry.height = g.height.value().max(1.0) as u32;
            data.appearance.scale = g.scale.value();
            data.appearance.translate_x = g.translate_x.value();
            data.appearance.translate_y = g.translate_y.value();
            data.appearance.z_index = g.z_index.value() as i32;
            data.appearance.ignore_collision = g.ignore_collision.is_active();
        }
        if let Some(a) = self.appearance.borrow().as_ref() {
            data.appearance.corner_radius = a.radius.value();
            data.appearance.border.enabled = a.border_enabled.is_active();
            data.appearance.border.width = a.border_width.value();
            data.appearance.border.color = *a.color.borrow();
        }
        if let Some(bg) = self.background_editor.borrow().as_ref() {
            data.appearance.background = bg.get_config();
        }

        data.theme = match self.theme_ref.borrow().clone() {
            ThemeRef::Inline { theme } => ThemeRef::inline(self.inline_payload().unwrap_or(theme)),
            other => other,
        };

        if let Some(cs) = self.combo_source.borrow().as_ref() {
            let mut cs = cs.clone();
            for (slot, item) in self.item_editors.borrow().iter() {
                if let Some(cfg) = item.source_editor.borrow().as_ref().and_then(|e| e.store()) {
                    if let Some(slot_cfg) = cs.slots.get_mut(slot) {
                        if slot_cfg.source_id == cfg.source_type() {
                            slot_cfg.source_config = flatten_source_config(&cfg);
                        }
                    }
                }
            }
            data.source_config = SourceConfig::Combo(cs);
        } else if let Some(cfg) = self.source_editor.borrow().as_ref().and_then(|e| e.store()) {
            if cfg.source_type() == data.source_config.source_type() {
                data.source_config = cfg;
            }
        }

        if let Some(cfg) = self
            .display_editor
            .borrow()
            .as_ref()
            .and_then(|e| e.store())
        {
            if cfg.displayer_type() == data.displayer_config.displayer_type() {
                data.displayer_config = cfg;
            }
        }
        data
    }

    /// Apply the editors' state to the live panel.
    pub(super) fn commit_now(self: &Rc<Self>) {
        // A debounced commit may fire after the window closed (panel
        // deleted, dialog dismissed): nothing to apply to, and an error
        // alert on a dead window would be an invisible modal.
        if !self.alive.get() {
            return;
        }
        self.detach_theme_if_edited();
        let data = self.build_data();
        self.apply(data);
    }

    fn apply(self: &Rc<Self>, data: PanelData) -> bool {
        let Some(grid) = self.grid.upgrade() else {
            return false;
        };
        let result = grid.borrow().apply_panel_data(&self.panel, data);
        match result {
            Ok(outcome) => {
                *self.applied.borrow_mut() = self.panel.blocking_read().to_data();
                if outcome.size_reverted {
                    self.loading.set(true);
                    if let Some(g) = self.geometry.borrow().as_ref() {
                        let geom = self.applied.borrow().geometry;
                        g.width.set_value(geom.width as f64);
                        g.height.set_value(geom.height as f64);
                    }
                    self.loading.set(false);
                    self.alert(
                        "Cannot resize panel",
                        "The new size would overlap another panel. The size was kept; everything else was applied.",
                    );
                }
                // Keep the editors' theme in step: an Inline edit must reach
                // the background and item editors, and a shared theme changed
                // in the Themes & Groups window must reach the theme editor.
                let stale = match self.theme_baseline.borrow().as_ref() {
                    Some(baseline) => *baseline != self.resolved_theme(),
                    None => false,
                };
                if stale {
                    self.propagate_theme();
                }
                self.refresh_row_texts();
                true
            }
            Err(e) => {
                log::warn!("Panel {}: apply failed: {}", self.panel_id, e);
                self.alert("Could not apply changes", &e.to_string());
                false
            }
        }
    }

    /// Restore the state the dialog opened with.
    pub(super) fn revert(self: &Rc<Self>) {
        let snapshot = self.snapshot.borrow().clone();
        if self.apply(snapshot) {
            let data = self.applied.borrow().clone();
            self.reset_from(data);
        }
    }

    /// Rebuild editors and tree from `data` (open, revert, type switches).
    pub(super) fn reset_from(self: &Rc<Self>, data: PanelData) {
        self.loading.set(true);
        self.cleanup_editors();
        while let Some(child) = self.stack.first_child() {
            self.stack.remove(&child);
        }
        self.pages.borrow_mut().clear();
        self.poll_last.borrow_mut().clear();

        *self.combo_source.borrow_mut() = match &data.source_config {
            SourceConfig::Combo(cs) => {
                let mut cs = cs.clone();
                cs.migrate_legacy();
                Some(cs)
            }
            _ => None,
        };
        *self.theme_ref.borrow_mut() = data.theme.clone();
        *self.applied.borrow_mut() = data;

        self.ensure_display_editor();
        // Every item shows as a type its source offers (see
        // `coerce_item_displays`); a change here is committed like an edit.
        let coerced = self.coerce_item_displays();
        self.loading.set(false);
        self.rebuild_tree(Some(NodeKind::Root));
        if coerced {
            self.changed();
        }
    }

    pub(super) fn cleanup_editors(&self) {
        self.drop_inline_source();
        for (_, item) in self.item_editors.borrow_mut().drain() {
            item.cleanup();
        }
        for (_, editor) in self.group_editors.borrow_mut().drain() {
            editor.cleanup();
        }
        if let Some(e) = self.source_editor.borrow_mut().take() {
            e.cleanup();
        }
        if let Some(e) = self.display_editor.borrow_mut().take() {
            e.cleanup();
        }
        self.display_pages.borrow_mut().clear();
        *self.background_editor.borrow_mut() = None;
        *self.inline_theme_widget.borrow_mut() = None;
        *self.geometry.borrow_mut() = None;
        *self.appearance.borrow_mut() = None;
        *self.theme_controls.borrow_mut() = None;
        *self.theme_baseline.borrow_mut() = None;
    }

    // ------------------------------------------------------------------
    // Editors
    // ------------------------------------------------------------------

    /// Build the display editor for the current displayer type. Eager for
    /// combos because the tree needs the section names.
    pub(super) fn ensure_display_editor(self: &Rc<Self>) {
        if self.display_editor.borrow().is_some() {
            return;
        }
        let id = self.displayer_type();
        let fields = self.all_fields();
        let Some(editor) = editor_registry::create_display_editor(&id, fields.clone()) else {
            log::warn!("No editor registered for displayer '{}'", id);
            return;
        };
        let editor = Rc::new(editor);
        let resolved = self.resolved_theme();
        editor.set_theme(&resolved);
        let mut cfg = self.applied.borrow().displayer_config.clone();
        if let Some(cs) = self.combo_source.borrow().as_ref() {
            // The source's groups own the structure; the frame caches it.
            cfg.sync_group_structure(&cs.groups);
        }
        editor.load(&cfg);
        // `load` wrote the panel's cached frame theme; a shared reference
        // shows (and is compared against) the resolved theme.
        if !self.theme_ref.borrow().is_inline() {
            editor.set_theme(&resolved);
        }
        if editor.combo().is_none() {
            if let Some(cs) = self.combo_source.borrow().as_ref() {
                // css_template: slot summaries for its placeholder mapping
                editor.set_slot_kinds(combo_slot_kinds(cs));
                editor.set_source_summaries(combo_summaries(cs));
            }
        }
        let weak = Rc::downgrade(self);
        editor.set_on_request_slots(Rc::new(move |wanted| {
            if let Some(s) = weak.upgrade() {
                s.create_wanted_slots(&wanted);
            }
        }));
        editor.set_slot_host(self.slot_host());
        let weak = Rc::downgrade(self);
        let has_signal = editor.set_on_change(Rc::new(move || {
            if let Some(s) = weak.upgrade() {
                s.changed();
            }
        }));
        self.display_needs_poll.set(!has_signal);

        // One child node per section, and one per detached list item
        // (text lines) beneath its section.
        self.adopt_editor_pages(&**editor);
        let weak = Rc::downgrade(self);
        editor.set_on_sections_changed(Rc::new(move |select| {
            if let Some(s) = weak.upgrade() {
                s.refresh_display_pages(select);
            }
        }));
        *self.display_editor.borrow_mut() = Some(editor);
        *self.theme_baseline.borrow_mut() = self.inline_payload();
    }

    /// Take the editor's pages (an empty path addresses nothing).
    fn adopt_editor_pages(&self, editor: &dyn DisplayEditor) {
        let pages: Vec<EditorPage> = editor
            .sections()
            .into_iter()
            .filter(|p| !p.path.is_empty())
            .collect();
        *self.display_pages.borrow_mut() = pages;
    }

    /// Whether the display editor has sections (one-part pages). Without
    /// them its rows sit on the Display page itself.
    pub(super) fn has_sections(&self) -> bool {
        self.display_pages
            .borrow()
            .iter()
            .any(|p| p.path.len() == 1)
    }

    /// The editor's list items changed: drop the stale item pages, re-read
    /// the pages and rebuild the tree, selecting `select` (or its nearest
    /// surviving ancestor) when given, else keeping the current node if it
    /// still exists.
    pub(super) fn refresh_display_pages(self: &Rc<Self>, select: Option<Vec<String>>) {
        if !self.alive.get() {
            return;
        }
        let editor = self.display_editor.borrow().clone();
        let Some(editor) = editor else { return };
        // Same pages, same widgets (an "Edit…" jump): just select.
        let current_pages: Vec<EditorPage> = self.display_pages.borrow().clone();
        let new_pages: Vec<EditorPage> = editor
            .sections()
            .into_iter()
            .filter(|p| !p.path.is_empty())
            .collect();
        let same = |a: &EditorPage, b: &EditorPage| a.path == b.path && a.widget == b.widget;
        let unchanged = current_pages.len() == new_pages.len()
            && current_pages
                .iter()
                .zip(new_pages.iter())
                .all(|(a, b)| same(a, b));
        if unchanged {
            if let Some(path) = select {
                self.select_node(&self.page_node(&path));
            }
            return;
        }
        // Pages that vanished or were rebuilt leave the stack; a page whose
        // widget survived (a section) keeps its entry.
        let stale: Vec<String> = current_pages
            .iter()
            .filter(|old| !new_pages.iter().any(|new| same(old, new)))
            .map(|p| NodeKind::Section(p.path.clone()).id())
            .collect();
        self.remove_pages(&stale);
        self.adopt_editor_pages(&**editor);
        let target = match select {
            Some(path) => self.page_node(&path),
            None => self
                .selected_node()
                .filter(|k| self.node_exists(k))
                .unwrap_or(NodeKind::Display),
        };
        self.rebuild_tree(Some(target));
    }

    /// The node showing `path`, or its nearest existing ancestor, or Display.
    pub(super) fn page_node(&self, path: &[String]) -> NodeKind {
        let pages: Vec<Vec<String>> = self
            .display_pages
            .borrow()
            .iter()
            .map(|p| p.path.clone())
            .collect();
        resolve_page_node(path, &pages)
    }

    fn node_exists(&self, kind: &NodeKind) -> bool {
        match kind {
            NodeKind::Section(path) => self.display_pages.borrow().iter().any(|p| &p.path == path),
            NodeKind::ItemSection(slot, path) => {
                self.item_page_paths(slot).iter().any(|p| p == path)
            }
            _ => true,
        }
    }

    // ------------------------------------------------------------------
    // Combo item pages
    // ------------------------------------------------------------------

    /// The pages of an item's display editor, derived from the frame config
    /// (`ComboDisplayEditor::item_pages`): the tree lists every item's child
    /// nodes without building a single item editor.
    pub(super) fn item_tree_pages(&self, slot: &str) -> Vec<TreePage> {
        self.display_editor
            .borrow()
            .as_ref()
            .and_then(|e| e.combo().map(|c| c.item_pages(slot)))
            .unwrap_or_default()
    }

    fn item_page_paths(&self, slot: &str) -> Vec<Vec<String>> {
        self.item_tree_pages(slot)
            .into_iter()
            .map(|p| p.path)
            .collect()
    }

    /// The node showing an item page, or its nearest existing ancestor, or
    /// the item.
    pub(super) fn item_page_node(&self, slot: &str, path: &[String]) -> NodeKind {
        item_node_for(slot, path, &self.item_page_paths(slot))
    }

    /// The editors of one content item, built on first use by any of the
    /// item's pages (a section may be opened before the item page) and kept
    /// until the structure changes.
    pub(super) fn ensure_item_editor(self: &Rc<Self>, slot: &str) -> Rc<ItemEditor> {
        if let Some(item) = self.item_editors.borrow().get(slot) {
            return item.clone();
        }
        let (source_id, flat) = self
            .combo_source
            .borrow()
            .as_ref()
            .and_then(|cs| cs.slots.get(slot))
            .map(|s| (s.source_id.clone(), s.source_config.clone()))
            .unwrap_or_default();
        let source_editor = if slot_has_source_editor(&source_id) {
            editor_registry::create_source_editor(&source_id)
        } else {
            None
        };
        let needs_poll = Cell::new(false);
        if let Some(editor) = source_editor.as_ref() {
            if let Some(cfg) = source_config_from_flat(&source_id, &flat) {
                editor.load(&cfg);
            }
            let weak = Rc::downgrade(self);
            let slot_name = slot.to_string();
            let has_signal = editor.set_on_change(Rc::new(move || {
                if let Some(s) = weak.upgrade() {
                    s.changed();
                    s.sync_inline_source(&slot_name);
                }
            }));
            needs_poll.set(!has_signal);
        }
        // Items render against the slot's own values (short keys), so their
        // field pickers get the short ids, never the combo-wide prefixed ones.
        let fields = item_fields(&source_id);
        let display = self.display_editor.borrow().as_ref().and_then(|e| {
            let weak = Rc::downgrade(self);
            e.combo().map(|c| {
                c.item_editor(
                    slot,
                    &source_id,
                    &fields,
                    Rc::new(move || {
                        if let Some(s) = weak.upgrade() {
                            s.changed();
                        }
                    }),
                )
            })
        });
        let pages = display.as_ref().map(|d| d.pages()).unwrap_or_default();
        if let Some(d) = display.as_ref() {
            let weak = Rc::downgrade(self);
            let slot = slot.to_string();
            d.set_on_pages_changed(Rc::new(move |select| {
                if let Some(s) = weak.upgrade() {
                    s.refresh_item_pages(&slot, select);
                }
            }));
        }
        let item = Rc::new(ItemEditor {
            source_editor: RefCell::new(source_editor),
            needs_poll,
            display,
            pages: RefCell::new(pages),
        });
        self.item_editors
            .borrow_mut()
            .insert(slot.to_string(), item.clone());
        item
    }

    /// An item's display editor changed its pages (a line added or removed,
    /// the display type switched): drop the stale pages, re-read them and
    /// rebuild the tree, selecting `select` (or its nearest surviving
    /// ancestor) when given, else keeping the current node if it still
    /// exists. The tree itself derives the item's nodes from the frame, which
    /// the editor has already written.
    pub(super) fn refresh_item_pages(self: &Rc<Self>, slot: &str, select: Option<Vec<String>>) {
        if !self.alive.get() {
            return;
        }
        let item = self.item_editors.borrow().get(slot).cloned();
        let Some(item) = item else { return };
        let Some(display) = item.display.as_ref() else {
            return;
        };
        let current: Vec<EditorPage> = item.pages.borrow().clone();
        let new_pages = display.pages();
        let same = |a: &EditorPage, b: &EditorPage| a.path == b.path && a.widget == b.widget;
        let unchanged = current.len() == new_pages.len()
            && current
                .iter()
                .zip(new_pages.iter())
                .all(|(a, b)| same(a, b));
        if unchanged {
            if let Some(path) = select {
                self.select_node(&self.item_page_node(slot, &path));
            }
            return;
        }
        let stale: Vec<String> = current
            .iter()
            .filter(|old| !new_pages.iter().any(|new| same(old, new)))
            .map(|p| NodeKind::ItemSection(slot.to_string(), p.path.clone()).id())
            .collect();
        self.remove_pages(&stale);
        *item.pages.borrow_mut() = new_pages;
        let target = match select {
            Some(path) => self.item_page_node(slot, &path),
            None => self
                .selected_node()
                .filter(|k| self.node_exists(k))
                .unwrap_or(NodeKind::ContentItem(slot.to_string())),
        };
        self.rebuild_tree(Some(target));
    }

    pub(super) fn ensure_source_editor(self: &Rc<Self>) -> Option<Widget> {
        if self.is_combo() {
            return None;
        }
        if self.source_editor.borrow().is_none() {
            let id = self.source_type();
            let editor = editor_registry::create_source_editor(&id)?;
            editor.load(&self.applied.borrow().source_config);
            let weak = Rc::downgrade(self);
            let has_signal = editor.set_on_change(Rc::new(move || {
                if let Some(s) = weak.upgrade() {
                    s.changed();
                }
            }));
            self.source_needs_poll.set(!has_signal);
            *self.source_editor.borrow_mut() = Some(editor);
        }
        self.source_editor.borrow().as_ref().map(|e| e.widget())
    }

    pub(super) fn ensure_background_editor(self: &Rc<Self>) -> Rc<BackgroundConfigWidget> {
        if let Some(bg) = self.background_editor.borrow().as_ref() {
            return bg.clone();
        }
        let bg = Rc::new(BackgroundConfigWidget::new());
        bg.set_theme_config(self.resolved_theme());
        bg.set_source_fields(self.all_fields());
        bg.set_is_combo_source(self.is_combo());
        bg.set_config(self.applied.borrow().appearance.background.clone());
        let weak = Rc::downgrade(self);
        bg.set_on_change(move || {
            if let Some(s) = weak.upgrade() {
                s.changed();
            }
        });
        *self.background_editor.borrow_mut() = Some(bg.clone());
        bg
    }

    /// Push the resolved theme into every editor that renders theme colours
    /// and record what the theme editor holds afterwards as the baseline.
    pub(super) fn propagate_theme(&self) {
        let resolved = self.resolved_theme();
        // For an Inline combo this pushes the editor's own payload back into
        // it. That is wanted: the themed widgets refresh their Display-section
        // colour and font selectors only in `set_theme`, and the push is
        // cheap because the Theme tab setters skip unchanged values (see
        // `GradientEditor::set_gradient_source_config`).
        if let Some(e) = self.display_editor.borrow().as_ref() {
            e.set_theme(&resolved);
        }
        // The standalone theme editor *is* the payload when Inline; for a
        // shared reference it mirrors the resolved theme.
        if !self.theme_ref.borrow().is_inline() {
            if let Some(w) = self.inline_theme_widget.borrow().as_ref() {
                let was_loading = self.loading.replace(true);
                w.set_config(resolved.clone());
                self.loading.set(was_loading);
            }
        }
        if let Some(bg) = self.background_editor.borrow().as_ref() {
            bg.set_theme_config(resolved.clone());
        }
        for item in self.item_editors.borrow().values() {
            if let Some(d) = item.display.as_ref() {
                d.set_theme(&resolved);
            }
        }
        for editor in self.group_editors.borrow().values() {
            editor.set_theme(&resolved);
        }
        *self.theme_baseline.borrow_mut() = self.inline_payload();
    }

    /// Change the theme reference from the dropdown.
    pub(super) fn apply_theme_ref(self: &Rc<Self>, theme_ref: ThemeRef) {
        let theme_ref = if theme_ref.is_inline() {
            // Start the private theme from what the panel resolves to now so
            // switching to Inline does not change its look.
            ThemeRef::inline(self.resolved_theme())
        } else {
            theme_ref
        };
        *self.theme_ref.borrow_mut() = theme_ref;
        self.sync_theme_controls();
        self.propagate_theme();
        self.changed();
    }

    /// The standalone theme editor changed.
    pub(super) fn theme_editor_changed(self: &Rc<Self>) {
        if self.loading.get() {
            return;
        }
        if self.theme_ref.borrow().is_inline() {
            self.propagate_theme();
        }
        self.changed();
    }

    /// An edit in the theme editor while the reference is shared (Inherit,
    /// Global or Named) makes the theme private to this panel. Runs before
    /// each commit. The editor payload is compared with the last pushed
    /// baseline, not with the current resolution, so a shared theme changed
    /// in the Themes & Groups window meanwhile does not detach this panel.
    fn detach_theme_if_edited(&self) {
        if self.theme_ref.borrow().is_inline() {
            return;
        }
        let Some(payload) = self.inline_payload() else {
            return;
        };
        if self.theme_baseline.borrow().as_ref() == Some(&payload) {
            return;
        }
        *self.theme_ref.borrow_mut() = ThemeRef::inline(payload);
        self.sync_theme_controls();
    }

    /// Make the Theme page's dropdown and hint reflect the current reference.
    fn sync_theme_controls(&self) {
        let theme_ref = self.theme_ref.borrow().clone();
        let controls = self.theme_controls.borrow();
        let Some(t) = controls.as_ref() else {
            return;
        };
        let was_loading = self.loading.replace(true);
        t.dropdown
            .set_selected(theme_choice_index(&t.choices, &theme_ref) as u32);
        self.loading.set(was_loading);
        t.mode_hint.set_text(theme_mode_hint(&theme_ref));
    }

    // ------------------------------------------------------------------
    // Type switches
    // ------------------------------------------------------------------

    /// Run `f` on the next main-loop iteration. Structural changes are
    /// triggered from signal handlers of widgets they would destroy (a
    /// dropdown on the page being rebuilt), so the rebuild is deferred.
    fn defer(self: &Rc<Self>, f: impl FnOnce(&Rc<State>) + 'static) {
        let weak = Rc::downgrade(self);
        glib::idle_add_local_once(move || {
            if let Some(state) = weak.upgrade() {
                f(&state);
            }
        });
    }

    pub(super) fn switch_source_type(self: &Rc<Self>, new_id: &str) {
        if new_id == self.source_type() {
            return;
        }
        let new_id = new_id.to_string();
        self.defer(move |state| {
            let registry = global_registry();
            let mut data = state.build_data();
            data.source_config = SourceConfig::default_for_type(&new_id).unwrap_or_default();
            let compatible: Vec<String> = registry
                .get_compatible_displayers(&new_id)
                .into_iter()
                .map(|d| d.id)
                .collect();
            let current = data.displayer_config.displayer_type().to_string();
            if !compatible.contains(&current) {
                let fallback = compatible
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "text".to_string());
                data.displayer_config = default_displayer_config(&fallback);
            }
            if state.apply(data) {
                let applied = state.applied.borrow().clone();
                state.reset_from(applied);
                state.select_node(&NodeKind::Source);
            }
        });
    }

    pub(super) fn switch_displayer_type(self: &Rc<Self>, new_id: &str) {
        if new_id == self.displayer_type() {
            return;
        }
        let new_id = new_id.to_string();
        self.defer(move |state| {
            let mut data = state.build_data();
            let transfer = state
                .display_editor
                .borrow()
                .as_ref()
                .and_then(|e| e.combo().map(|c| c.transferable()));
            data.displayer_config = default_displayer_config(&new_id);
            // Combo → combo keeps layout and content (TransferableComboConfig)
            if let (Some(transfer), true) = (transfer, editor_registry::is_combo_display(&new_id)) {
                if let Some(editor) =
                    editor_registry::create_display_editor(&new_id, state.all_fields())
                {
                    editor.load(&data.displayer_config);
                    if let Some(c) = editor.combo() {
                        c.apply_transferable(&transfer);
                    }
                    if let Some(cfg) = editor.store() {
                        data.displayer_config = cfg;
                    }
                    editor.cleanup();
                }
            }
            if state.apply(data) {
                let applied = state.applied.borrow().clone();
                state.reset_from(applied);
                state.select_node(&NodeKind::Display);
                if let Some(w) = state.window() {
                    w.set_title(Some(&format!(
                        "Panel Properties — {}",
                        displayer_display_name(&state.displayer_type())
                    )));
                }
            }
        });
    }
    // ------------------------------------------------------------------
    // Combo structure
    // ------------------------------------------------------------------

    fn with_combo<R>(&self, f: impl FnOnce(&mut ComboSourceConfig) -> R) -> Option<R> {
        self.combo_source.borrow_mut().as_mut().map(f)
    }

    /// Push the working combo structure into the display editor and rebuild
    /// the Content subtree.
    fn structure_changed(self: &Rc<Self>, reselect: NodeKind) {
        let (summaries, kinds, fields) = match self.combo_source.borrow().as_ref() {
            Some(cs) => (combo_summaries(cs), combo_slot_kinds(cs), combo_fields(cs)),
            None => return,
        };
        // Item pages are keyed by slot; structure changes invalidate them.
        self.drop_item_pages();
        if let Some(e) = self.display_editor.borrow().as_ref() {
            self.loading.set(true);
            if e.combo().is_some() {
                // Reload the themed editor with the new structure so its
                // Layout page (orientation per group) follows.
                if let Some(mut cfg) = e.store() {
                    if let Some(cs) = self.combo_source.borrow().as_ref() {
                        cfg.sync_group_structure(&cs.groups);
                    }
                    e.load(&cfg);
                }
            } else {
                e.set_slot_kinds(kinds);
                e.set_source_summaries(summaries);
                // The editor may keep its rows (same sensors in the same
                // places) and with them the inline settings editor, whose
                // slot may now hold another config (two items swapped).
                self.reload_inline_source();
            }
            self.loading.set(false);
        }
        if let Some(bg) = self.background_editor.borrow().as_ref() {
            bg.set_source_fields(fields);
        }
        self.rebuild_tree(Some(reselect));
        self.changed();
    }

    /// Drop every item editor and every page under Content (the structure
    /// changed: counts, order or a slot's source); they rebuild on demand.
    pub(super) fn drop_item_pages(&self) {
        let ids: Vec<String> = self
            .pages
            .borrow()
            .iter()
            .filter(|id| {
                *id == "content"
                    || id.starts_with("group/")
                    || id.starts_with("item/")
                    || id.starts_with("itemsrc/")
            })
            .cloned()
            .collect();
        self.remove_pages(&ids);
        for (_, item) in self.item_editors.borrow_mut().drain() {
            item.cleanup();
        }
        for (_, editor) in self.group_editors.borrow_mut().drain() {
            editor.cleanup();
        }
    }

    /// Remove built pages from the stack by id.
    fn remove_pages(&self, ids: &[String]) {
        for id in ids {
            if let Some(page) = self.stack.child_by_name(id) {
                self.stack.remove(&page);
            }
            self.pages.borrow_mut().remove(id);
            self.poll_last.borrow_mut().remove(id);
        }
    }

    // ------------------------------------------------------------------
    // Slot host (the CSS Template's Bindings page)
    // ------------------------------------------------------------------

    fn slot_host(self: &Rc<Self>) -> editor_registry::SlotHost {
        let (create, settings) = (Rc::downgrade(self), Rc::downgrade(self));
        let panel = self.panel.clone();
        editor_registry::SlotHost {
            create: Rc::new(move |source, metric| {
                create.upgrade().and_then(|s| s.create_slot(source, metric))
            }),
            settings: Rc::new(move |slot| {
                settings
                    .upgrade()
                    .and_then(|s| s.inline_source_widget(slot))
            }),
            // A once-a-second read for a preview: skipping one is invisible.
            values: Rc::new(move || {
                let key = panel.try_read().ok()?.source_key.clone()?;
                crate::core::global_shared_source_manager()?.get_values(&key)
            }),
        }
    }

    /// A new slot for `(source, metric)`. The editor binds to it as soon as
    /// this returns; the tree and the editor's slot list follow on idle (the
    /// call comes from inside one of the editor's own signal handlers).
    fn create_slot(self: &Rc<Self>, source: &str, metric: &str) -> Option<String> {
        let slot = self
            .with_combo(|cs| add_slot(cs, source, metric))
            .flatten()?;
        self.defer(|state| state.structure_changed(NodeKind::Display));
        Some(slot)
    }

    /// The slot's source config as the dialog holds it now: its item editor
    /// when one is built (it is what `build_data` stores), else the model.
    fn current_slot_source(
        &self,
        slot: &str,
    ) -> Option<(String, HashMap<String, serde_json::Value>)> {
        let (source_id, flat) = self
            .combo_source
            .borrow()
            .as_ref()
            .and_then(|cs| cs.slots.get(slot))
            .map(|s| (s.source_id.clone(), s.source_config.clone()))?;
        let from_item = self.item_editors.borrow().get(slot).and_then(|item| {
            let cfg = item.source_editor.borrow().as_ref()?.store()?;
            (cfg.source_type() == source_id).then(|| flatten_source_config(&cfg))
        });
        Some((source_id, from_item.unwrap_or(flat)))
    }

    /// A settings editor for one slot's source, shown inline by the display
    /// editor. It edits the same config as the item's Source page; the two
    /// are kept in step (`inline_source_changed`, `sync_inline_source`).
    fn inline_source_widget(self: &Rc<Self>, slot: &str) -> Option<Widget> {
        self.drop_inline_source();
        let (source_id, flat) = self.current_slot_source(slot)?;
        if !slot_has_source_editor(&source_id) {
            return None;
        }
        let editor = editor_registry::create_source_editor(&source_id)?;
        if let Some(cfg) = source_config_from_flat(&source_id, &flat) {
            editor.load(&cfg);
        }
        let weak = Rc::downgrade(self);
        editor.set_on_change(Rc::new(move || {
            if let Some(s) = weak.upgrade() {
                s.inline_source_changed();
            }
        }));
        let widget = editor.widget();
        *self.inline_source.borrow_mut() = Some((slot.to_string(), Rc::new(editor)));
        Some(widget)
    }

    /// Make the lent settings editor show its slot's config as it is now;
    /// retire it when the slot holds another source type. The editor on
    /// screen is always the one held here: a page that rebuilt its row has
    /// already asked for a new one.
    fn reload_inline_source(&self) {
        let Some((slot, editor)) = self.inline_source.borrow().clone() else {
            return;
        };
        let current = self.current_slot_source(&slot);
        let same_type = editor.store().map(|c| c.source_type().to_string())
            == current.as_ref().map(|(id, _)| id.clone());
        match current {
            Some((source_id, flat)) if same_type => {
                if let Some(cfg) = source_config_from_flat(&source_id, &flat) {
                    let was_loading = self.loading.replace(true);
                    editor.load(&cfg);
                    self.loading.set(was_loading);
                }
            }
            _ => self.drop_inline_source(),
        }
    }

    pub(super) fn drop_inline_source(&self) {
        if let Some((_, editor)) = self.inline_source.borrow_mut().take() {
            editor.cleanup();
        }
    }

    /// The inline settings editor changed: store its config in the slot,
    /// bring the item's Source page along, refresh what the slot is called.
    fn inline_source_changed(self: &Rc<Self>) {
        if self.loading.get() {
            return;
        }
        let Some((slot, editor)) = self.inline_source.borrow().clone() else {
            return;
        };
        let Some(cfg) = editor.store() else {
            return;
        };
        let flat = flatten_source_config(&cfg);
        match self.current_slot_source(&slot) {
            Some((source_id, current)) if source_id == cfg.source_type() => {
                if current == flat {
                    return;
                }
            }
            _ => return,
        }
        self.with_combo(|cs| {
            if let Some(s) = cs.slots.get_mut(&slot) {
                s.source_config = flat;
            }
        });
        self.loading.set(true);
        if let Some(item) = self.item_editors.borrow().get(&slot) {
            if let Some(e) = item.source_editor.borrow().as_ref() {
                e.load(&cfg);
            }
        }
        let kinds = self.combo_source.borrow().as_ref().map(combo_slot_kinds);
        if let (Some(kinds), Some(e)) = (kinds, self.display_editor.borrow().as_ref()) {
            e.set_slot_kinds(kinds);
        }
        self.loading.set(false);
        self.changed();
    }

    /// The item's Source page changed: the inline editor of that slot follows.
    fn sync_inline_source(&self, slot: &str) {
        if self.loading.get() {
            return;
        }
        let Some((inline_slot, editor)) = self.inline_source.borrow().clone() else {
            return;
        };
        if inline_slot != slot {
            return;
        }
        let Some((source_id, flat)) = self.current_slot_source(slot) else {
            return;
        };
        if editor.store().map(|c| flatten_source_config(&c)) == Some(flat.clone()) {
            return;
        }
        if let Some(cfg) = source_config_from_flat(&source_id, &flat) {
            self.loading.set(true);
            editor.load(&cfg);
            self.loading.set(false);
        }
    }

    /// The CSS Template editor's Scan Placeholders: give the combination
    /// source the slots the template asks for. The editor maps its
    /// placeholders right after this returns, so it gets the new slot list at
    /// once; the tree follows on idle.
    fn create_wanted_slots(self: &Rc<Self>, wanted: &[(String, String)]) {
        let created = self
            .with_combo(|cs| add_wanted_slots(cs, wanted))
            .unwrap_or(0);
        if created == 0 {
            return;
        }
        let lists = self
            .combo_source
            .borrow()
            .as_ref()
            .map(|cs| (combo_slot_kinds(cs), combo_summaries(cs)));
        if let (Some((kinds, summaries)), Some(e)) = (lists, self.display_editor.borrow().as_ref())
        {
            self.loading.set(true);
            e.set_slot_kinds(kinds);
            e.set_source_summaries(summaries);
            self.loading.set(false);
        }
        self.defer(|state| state.structure_changed(NodeKind::Display));
    }

    pub(super) fn set_group_count(self: &Rc<Self>, n: usize) {
        let changed = self
            .with_combo(|cs| {
                let n = n.clamp(1, 16);
                if cs.groups.len() == n {
                    return false;
                }
                while cs.groups.len() < n {
                    cs.groups.push(crate::sources::GroupConfig {
                        item_count: 1,
                        size_weight: 1.0,
                    });
                }
                cs.groups.truncate(n);
                true
            })
            .unwrap_or(false);
        if changed {
            self.defer(|state| state.structure_changed(NodeKind::Content));
        }
    }

    pub(super) fn set_item_count(self: &Rc<Self>, group: usize, n: u32) {
        let changed = self
            .with_combo(|cs| match cs.groups.get_mut(group) {
                Some(g) if g.item_count != n.clamp(1, 16) => {
                    g.item_count = n.clamp(1, 16);
                    true
                }
                _ => false,
            })
            .unwrap_or(false);
        if changed {
            self.defer(move |state| state.structure_changed(NodeKind::ContentGroup(group)));
        }
    }
    /// Size weight of one group (live; the frame copy follows at once, the
    /// source stays the owner).
    pub(super) fn set_group_weight(self: &Rc<Self>, group: usize, weight: f64) {
        let weights = self.with_combo(|cs| {
            if let Some(g) = cs.groups.get_mut(group) {
                g.size_weight = weight.max(0.1);
            }
            cs.groups
                .iter()
                .map(|g| g.size_weight)
                .collect::<Vec<f64>>()
        });
        if let (Some(weights), Some(e)) = (weights, self.display_editor.borrow().as_ref()) {
            if let Some(c) = e.combo() {
                c.set_group_weights(&weights);
            }
        }
        self.changed();
    }

    /// Write a group's title bar into the frame and commit.
    pub(super) fn set_group_title(self: &Rc<Self>, group: usize, title: serde_json::Value) {
        if let Some(e) = self.display_editor.borrow().as_ref() {
            if let Some(c) = e.combo() {
                c.set_group_title(group, title);
            }
        }
        self.changed();
    }

    /// Write the group's item orientation into the frame and commit.
    pub(super) fn set_group_orientation(self: &Rc<Self>, group: usize, orientation: &str) {
        if let Some(e) = self.display_editor.borrow().as_ref() {
            if let Some(c) = e.combo() {
                c.set_group_orientation(group, orientation);
            }
        }
        self.changed();
    }

    /// Copy the slot's source and display settings to the clipboard.
    pub(super) fn copy_item(&self, slot: &str) {
        let mut slot_cfg = self
            .combo_source
            .borrow()
            .as_ref()
            .and_then(|cs| cs.slots.get(slot).cloned())
            .unwrap_or_default();
        // The live source editor may hold edits the working copy has not
        // received yet (`build_data` reads it the same way).
        if let Some(item) = self.item_editors.borrow().get(slot) {
            if let Some(cfg) = item.source_editor.borrow().as_ref().and_then(|e| e.store()) {
                if cfg.source_type() == slot_cfg.source_id {
                    slot_cfg.source_config = flatten_source_config(&cfg);
                }
            }
        }
        let content_item = self
            .display_editor
            .borrow()
            .as_ref()
            .and_then(|e| e.combo().and_then(|c| c.content_item(slot)))
            .unwrap_or_else(|| {
                serde_json::to_value(rg_sens_types::display_configs::ContentItemConfig::default())
                    .unwrap_or_default()
            });
        if let Ok(mut cb) = crate::ui::CLIPBOARD.lock() {
            cb.copy_combo_item(crate::ui::ComboItemClip {
                source_id: slot_cfg.source_id,
                caption_override: slot_cfg.caption_override,
                source_config: slot_cfg.source_config,
                content_item,
            });
        }
    }

    /// Replace the slot's source and display settings with the clipboard's
    /// item, then rebuild the item's subtree and commit (`structure_changed`
    /// drops the item's editors, which held the old item).
    pub(super) fn paste_item(self: &Rc<Self>, slot: &str) {
        let pasted = crate::ui::CLIPBOARD
            .lock()
            .ok()
            .and_then(|cb| cb.paste_combo_item());
        let Some(clip) = pasted else { return };
        self.with_combo(|cs| {
            cs.slots.insert(
                slot.to_string(),
                rg_sens_types::source_configs::combo::SlotConfig {
                    source_id: clip.source_id.clone(),
                    caption_override: clip.caption_override.clone(),
                    source_config: clip.source_config.clone(),
                },
            );
        });
        if let Some(e) = self.display_editor.borrow().as_ref() {
            if let Some(c) = e.combo() {
                c.set_content_item(slot, clip.content_item.clone());
                c.coerce_item_display(slot, &clip.source_id);
            }
        }
        let slot = slot.to_string();
        self.defer(move |state| state.structure_changed(NodeKind::ContentItem(slot)));
    }

    /// Load a pasted displayer config into the display editor: a combo's
    /// frame first follows this panel's source structure and addresses its
    /// item lines by short key, the theme is this panel's resolved one, and
    /// the item editors (copies of the old items) are rebuilt.
    pub(super) fn load_display_config(self: &Rc<Self>, mut cfg: crate::core::DisplayerConfig) {
        if let Some(cs) = self.combo_source.borrow().as_ref() {
            cfg.sync_group_structure(&cs.groups);
            cfg.strip_item_field_prefixes();
        }
        if let Some(e) = self.display_editor.borrow().as_ref() {
            e.load(&cfg);
            e.set_theme(&self.resolved_theme());
        }
        if self.is_combo() {
            self.drop_item_pages();
            self.coerce_item_displays();
            self.rebuild_tree(self.selected_node());
        }
    }

    pub(super) fn set_update_interval(self: &Rc<Self>, ms: u64) {
        self.with_combo(|cs| cs.update_interval_ms = ms);
        self.changed();
    }

    pub(super) fn reorder(self: &Rc<Self>, op: ComboReorder) {
        let reselect = match op {
            ComboReorder::Group { a, b } => {
                self.with_combo(|cs| cs.swap_groups(a, b));
                NodeKind::ContentGroup(b)
            }
            ComboReorder::Item { group, i, j } => {
                self.with_combo(|cs| cs.swap_items(group, i, j));
                NodeKind::ContentItem(format!("group{}_{}", group + 1, j + 1))
            }
        };
        if let Some(e) = self.display_editor.borrow().as_ref() {
            if let Some(c) = e.combo() {
                c.reorder(op);
            } else if let Some(DisplayerConfig::CssTemplate(mut cfg)) = e.store() {
                // Placeholder mappings name their slot; they follow the
                // source that moved (1-based in the slot name).
                use rg_sens_types::combo::reorder::{remap_group_swap, remap_item_swap};
                for m in cfg
                    .mappings
                    .iter_mut()
                    .filter(|m| !m.slot_prefix.is_empty())
                {
                    m.slot_prefix = match op {
                        ComboReorder::Group { a, b } => {
                            remap_group_swap(&m.slot_prefix, a + 1, b + 1)
                        }
                        ComboReorder::Item { group, i, j } => {
                            remap_item_swap(&m.slot_prefix, group + 1, i + 1, j + 1)
                        }
                    };
                }
                self.loading.set(true);
                e.load(&DisplayerConfig::CssTemplate(cfg));
                self.loading.set(false);
            }
        }
        self.defer(move |state| state.structure_changed(reselect));
    }

    pub(super) fn set_item_source(self: &Rc<Self>, slot: &str, source_id: &str) {
        let changed = self
            .with_combo(|cs| {
                let entry = cs.slots.entry(slot.to_string()).or_default();
                if entry.source_id == source_id {
                    return false;
                }
                entry.source_id = source_id.to_string();
                entry.source_config.clear();
                true
            })
            .unwrap_or(false);
        if changed {
            // The new source may not offer the item's display type (a clock
            // shows as text or a face; a metric never as a face).
            if let Some(e) = self.display_editor.borrow().as_ref() {
                if let Some(c) = e.combo() {
                    c.coerce_item_display(slot, source_id);
                }
            }
            let slot = slot.to_string();
            self.defer(move |state| state.structure_changed(NodeKind::ContentItem(slot)));
        }
    }

    /// Make every item's display type one its source offers; returns whether
    /// any changed (a config saved before the clock face existed may show a
    /// clock slot as a bar).
    pub(super) fn coerce_item_displays(&self) -> bool {
        let slots: Vec<(String, String)> = match self.combo_source.borrow().as_ref() {
            Some(cs) => rg_sens_sources::slot_names_for(cs)
                .into_iter()
                .map(|slot| {
                    let id = cs
                        .slots
                        .get(&slot)
                        .map(|s| s.source_id.clone())
                        .unwrap_or_default();
                    (slot, id)
                })
                .collect(),
            None => return false,
        };
        let editor = self.display_editor.borrow();
        let Some(combo) = editor.as_ref().and_then(|e| e.combo()) else {
            return false;
        };
        let mut changed = false;
        for (slot, id) in &slots {
            changed |= combo.coerce_item_display(slot, id);
        }
        changed
    }

    pub(super) fn set_item_caption(self: &Rc<Self>, slot: &str, caption: String) {
        self.with_combo(|cs| {
            cs.slots
                .entry(slot.to_string())
                .or_default()
                .caption_override = caption;
        });
        self.changed();
    }

    // ------------------------------------------------------------------
    // Polling for editors without a change signal
    // ------------------------------------------------------------------

    /// Poll every built editor that has no change signal, whichever page is
    /// visible, so an edit made just before navigating away is still
    /// committed. The serialised configs are tiny.
    pub(super) fn poll(self: &Rc<Self>) {
        if self.loading.get() {
            return;
        }
        let mut current: Vec<(String, String)> = Vec::new();
        if self.source_needs_poll.get() {
            if let Some(json) = self
                .source_editor
                .borrow()
                .as_ref()
                .and_then(|e| e.store())
                .and_then(|c| serde_json::to_string(&c).ok())
            {
                current.push((NodeKind::Source.id(), json));
            }
        }
        for (slot, item) in self.item_editors.borrow().iter() {
            if !item.needs_poll.get() {
                continue;
            }
            if let Some(json) = item
                .source_editor
                .borrow()
                .as_ref()
                .and_then(|e| e.store())
                .and_then(|c| serde_json::to_string(&c).ok())
            {
                current.push((NodeKind::ContentItem(slot.clone()).id(), json));
            }
        }
        if self.display_needs_poll.get() {
            if let Some(json) = self
                .display_editor
                .borrow()
                .as_ref()
                .and_then(|e| e.store())
                .and_then(|c| serde_json::to_string(&c).ok())
            {
                current.push((NodeKind::Display.id(), json));
            }
        }
        let mut changed = false;
        {
            let mut last = self.poll_last.borrow_mut();
            for (id, json) in current {
                // First observation of an editor records the baseline only.
                if let Some(prev) = last.insert(id, json.clone()) {
                    if prev != json {
                        changed = true;
                    }
                }
            }
        }
        if changed {
            self.changed();
        }
    }

    /// Record the current values of newly built editors before the next
    /// tick, so creating a page never reads as an edit.
    pub(super) fn reset_poll_baseline(self: &Rc<Self>) {
        self.poll();
    }
    // ------------------------------------------------------------------
    // Tree
    // ------------------------------------------------------------------

    pub(super) fn rebuild_tree(&self, reselect: Option<NodeKind>) {
        let mut spec: HashMap<String, Vec<NodeKind>> = HashMap::new();
        let mut root_children = vec![NodeKind::Geometry, NodeKind::Theme];
        if !self.is_combo() {
            root_children.push(NodeKind::Source);
        }
        root_children.push(NodeKind::Display);
        if self.is_combo() {
            root_children.push(NodeKind::Content);
        }
        root_children.push(NodeKind::Background);
        root_children.push(NodeKind::Appearance);
        spec.insert(NodeKind::Root.id(), root_children);

        // Sections come first in `sections()`, so a list item always finds
        // its parent page (or Display) already placed.
        for page in self.display_pages.borrow().iter() {
            let parent = self.page_node(&page.path[..page.path.len() - 1]);
            spec.entry(parent.id())
                .or_default()
                .push(NodeKind::Section(page.path.clone()));
        }

        if let Some(cs) = self.combo_source.borrow().as_ref() {
            let groups: Vec<NodeKind> = (0..cs.groups.len()).map(NodeKind::ContentGroup).collect();
            spec.insert(NodeKind::Content.id(), groups);
            for (g, group) in cs.groups.iter().enumerate() {
                let mut items = Vec::new();
                for m in 1..=group.item_count {
                    let slot = format!("group{}_{}", g + 1, m);
                    items.push(NodeKind::ContentItem(slot.clone()));
                    // The item's children: its source settings, then the
                    // pages of its display editor, derived from the frame
                    // (no item editor is built for the tree).
                    let has_source = cs
                        .slots
                        .get(&slot)
                        .map(|s| slot_has_source_editor(&s.source_id))
                        .unwrap_or(false);
                    if has_source {
                        spec.entry(NodeKind::ContentItem(slot.clone()).id())
                            .or_default()
                            .push(NodeKind::ItemSource(slot.clone()));
                    }
                    let pages = self.item_page_paths(&slot);
                    for path in &pages {
                        let parent = item_node_for(&slot, &path[..path.len() - 1], &pages);
                        spec.entry(parent.id())
                            .or_default()
                            .push(NodeKind::ItemSection(slot.clone(), path.clone()));
                    }
                }
                spec.insert(NodeKind::ContentGroup(g).id(), items);
            }
        }
        let needle = self.filter.borrow().clone();
        if !needle.is_empty() {
            spec = self.prune_spec(&spec, &needle);
        }
        *self.tree_spec.borrow_mut() = spec;

        self.root_store.remove_all();
        self.root_store.append(&NodeKind::Root.boxed());
        // Items start folded (29 items with eight children each would bury
        // the overview); the way down to the reselected node stays open.
        self.collapse_items(reselect.as_ref());
        if let Some(target) = reselect {
            self.select_node(&target);
        }
    }

    /// Fold every content item row except the ancestors of `keep`. A no-op
    /// while the tree is filtered: matches must stay visible.
    fn collapse_items(&self, keep: Option<&NodeKind>) {
        if !self.filter.borrow().is_empty() {
            return;
        }
        let keep_ids: HashSet<String> = keep
            .map(|k| self.breadcrumb_path(k).iter().map(NodeKind::id).collect())
            .unwrap_or_default();
        // Back to front: folding a row removes the rows after it.
        for i in (0..self.selection.n_items()).rev() {
            let Some(row) = self
                .selection
                .item(i)
                .and_then(|o| o.downcast::<TreeListRow>().ok())
            else {
                continue;
            };
            let Some(kind) = row.item().and_then(|o| NodeKind::from_object(&o)) else {
                continue;
            };
            if matches!(kind, NodeKind::ContentItem(_)) && !keep_ids.contains(&kind.id()) {
                row.set_expanded(false);
            }
        }
    }

    /// Keep only nodes that match `needle` or have a matching descendant
    /// (Root always stays).
    fn prune_spec(
        &self,
        spec: &HashMap<String, Vec<NodeKind>>,
        needle: &str,
    ) -> HashMap<String, Vec<NodeKind>> {
        fn keep(
            state: &State,
            kind: &NodeKind,
            spec: &HashMap<String, Vec<NodeKind>>,
            needle: &str,
            out: &mut HashMap<String, Vec<NodeKind>>,
        ) -> bool {
            let mut kept_children = Vec::new();
            if let Some(children) = spec.get(&kind.id()) {
                for child in children {
                    if keep(state, child, spec, needle, out) {
                        kept_children.push(child.clone());
                    }
                }
            }
            let has_kept = !kept_children.is_empty();
            if has_kept {
                out.insert(kind.id(), kept_children);
            }
            has_kept || state.node_matches(kind, needle)
        }
        let mut out = HashMap::new();
        keep(self, &NodeKind::Root, spec, needle, &mut out);
        out.entry(NodeKind::Root.id()).or_default();
        out
    }

    /// Whether a node's title, summary, editor labels or page keywords
    /// contain `needle` (lower-case).
    fn node_matches(&self, kind: &NodeKind, needle: &str) -> bool {
        let (title, summary) = self.row_text(kind);
        if title.to_lowercase().contains(needle) || summary.to_lowercase().contains(needle) {
            return true;
        }
        let mut terms: Vec<String> = match kind {
            NodeKind::Geometry => {
                vec!["width height size scale offset translate z-index collision".into()]
            }
            NodeKind::Theme => vec!["theme colour color font gradient inline preset".into()],
            NodeKind::Background => vec!["background colour color gradient image polygon".into()],
            NodeKind::Appearance => vec!["border corner radius style".into()],
            NodeKind::Content => vec!["groups items update interval".into()],
            NodeKind::ContentGroup(_) => vec!["items size weight order".into()],
            NodeKind::Source => self
                .source_editor
                .borrow()
                .as_ref()
                .map(|e| e.search_terms())
                .unwrap_or_default(),
            NodeKind::ContentItem(_) => vec!["caption order source display type height".into()],
            NodeKind::ItemSource(slot) => {
                let mut terms = self
                    .item_editors
                    .borrow()
                    .get(slot)
                    .and_then(|i| i.source_editor.borrow().as_ref().map(|e| e.search_terms()))
                    .unwrap_or_default();
                terms.push("source configuration copy paste".into());
                terms
            }
            NodeKind::ItemSection(slot, path) => self
                .item_tree_pages(slot)
                .into_iter()
                .find(|p| &p.path == path)
                .map(|p| p.search_terms)
                .unwrap_or_default(),
            NodeKind::Section(path) => self
                .display_editor
                .borrow()
                .as_ref()
                .map(|e| e.page_search_terms(path))
                .unwrap_or_default(),
            _ => Vec::new(),
        };
        if matches!(kind, NodeKind::Source) {
            terms.push("source update interval caption".into());
        }
        terms.iter().any(|t| t.to_lowercase().contains(needle))
    }

    /// Set the search text and rebuild the tree, keeping the selection
    /// when it survives the filter.
    pub(super) fn set_filter(self: &Rc<Self>, text: &str) {
        let needle = text.trim().to_lowercase();
        if *self.filter.borrow() == needle {
            return;
        }
        *self.filter.borrow_mut() = needle;
        let selected = self.selected_node();
        self.rebuild_tree(selected);
    }

    pub(super) fn selected_node(&self) -> Option<NodeKind> {
        self.selection
            .selected_item()
            .and_then(|o| o.downcast::<TreeListRow>().ok())
            .and_then(|row| row.item())
            .and_then(|o| NodeKind::from_object(&o))
    }

    /// The visible row of a node (rows under a folded row are not visible).
    fn find_row(&self, target: &NodeKind) -> Option<(u32, TreeListRow)> {
        (0..self.selection.n_items()).find_map(|i| {
            let row = self
                .selection
                .item(i)
                .and_then(|o| o.downcast::<TreeListRow>().ok())?;
            let kind = row.item().and_then(|o| NodeKind::from_object(&o))?;
            (&kind == target).then_some((i, row))
        })
    }

    pub(super) fn select_node(&self, target: &NodeKind) {
        if self.find_row(target).is_none() {
            // Under a folded row: open the way down to it.
            for crumb in self.breadcrumb_path(target) {
                if let Some((_, row)) = self.find_row(&crumb) {
                    row.set_expanded(true);
                }
            }
        }
        let Some((i, _)) = self.find_row(target) else {
            return;
        };
        if self.selection.selected() == i {
            // Same position: GTK fires no notify, drive the page ourselves.
            self.selection.set_selected(gtk4::INVALID_LIST_POSITION);
        }
        self.selection.set_selected(i);
    }

    /// Unfold a node's row (an item shows its children once selected).
    pub(super) fn expand_node(&self, target: &NodeKind) {
        if let Some((_, row)) = self.find_row(target) {
            row.set_expanded(true);
        }
    }

    /// Refresh the labels of the bound rows (summaries changed) without
    /// rebuilding the tree.
    pub(super) fn refresh_row_texts(&self) {
        let rows: Vec<(NodeKind, glib::WeakRef<Label>, glib::WeakRef<Label>)> = self
            .row_labels
            .borrow()
            .values()
            .map(|(k, t, s)| (k.clone(), t.clone(), s.clone()))
            .collect();
        for (kind, title, summary) in rows {
            let (t, s) = self.row_text(&kind);
            if let Some(l) = title.upgrade() {
                l.set_text(&t);
            }
            if let Some(l) = summary.upgrade() {
                l.set_text(&s);
            }
        }
    }

    /// Title and dim summary of a row.
    pub(super) fn row_text(&self, kind: &NodeKind) -> (String, String) {
        let applied = self.applied.borrow();
        match kind {
            NodeKind::Root => (
                displayer_display_name(applied.displayer_config.displayer_type()),
                source_display_name(applied.source_config.source_type()),
            ),
            NodeKind::Geometry => (
                "Position & Size".into(),
                format!(
                    "{}×{} at {},{}",
                    applied.geometry.width,
                    applied.geometry.height,
                    applied.geometry.x,
                    applied.geometry.y
                ),
            ),
            NodeKind::Theme => ("Theme".into(), self.theme_ref.borrow().describe()),
            NodeKind::Source => (
                "Source".into(),
                source_display_name(applied.source_config.source_type()),
            ),
            NodeKind::Display => (
                "Display".into(),
                displayer_display_name(applied.displayer_config.displayer_type()),
            ),
            NodeKind::Section(path) => (
                self.display_editor
                    .borrow()
                    .as_ref()
                    .map(|e| e.page_title(path))
                    .unwrap_or_else(|| path.last().cloned().unwrap_or_default()),
                String::new(),
            ),
            NodeKind::Content => {
                let (groups, items) = self
                    .combo_source
                    .borrow()
                    .as_ref()
                    .map(|cs| (cs.groups.len(), cs.total_item_count()))
                    .unwrap_or((0, 0));
                (
                    "Content".into(),
                    format!("{} groups · {} items", groups, items),
                )
            }
            NodeKind::ContentGroup(g) => {
                let count = self
                    .combo_source
                    .borrow()
                    .as_ref()
                    .and_then(|cs| cs.groups.get(*g).map(|gr| gr.item_count))
                    .unwrap_or(0);
                (format!("Group {}", g + 1), format!("{} items", count))
            }
            NodeKind::ContentItem(slot) => {
                let (_, m) = NodeKind::parse_slot(slot).unwrap_or((0, 0));
                let source = self
                    .combo_source
                    .borrow()
                    .as_ref()
                    .and_then(|cs| cs.slots.get(slot).cloned())
                    .map(|s| {
                        if s.source_id.is_empty() || s.source_id == "none" {
                            "(none)".to_string()
                        } else if !s.caption_override.is_empty() {
                            format!(
                                "{} \"{}\"",
                                source_display_name(&s.source_id),
                                s.caption_override
                            )
                        } else {
                            source_display_name(&s.source_id)
                        }
                    })
                    .unwrap_or_else(|| "(none)".to_string());
                let display_as = applied
                    .displayer_config
                    .content_items()
                    .and_then(|items| items.get(slot))
                    .map(|i| super::combo_items::display_type_label(i.display_as))
                    .unwrap_or_default();
                (
                    format!("Item {}", m + 1),
                    if display_as.is_empty() {
                        source
                    } else {
                        format!("{} → {}", source, display_as)
                    },
                )
            }
            NodeKind::ItemSource(slot) => (
                "Source".into(),
                self.combo_source
                    .borrow()
                    .as_ref()
                    .and_then(|cs| cs.slots.get(slot))
                    .map(|s| source_display_name(&s.source_id))
                    .unwrap_or_default(),
            ),
            NodeKind::ItemSection(slot, path) => (
                self.item_tree_pages(slot)
                    .into_iter()
                    .find(|p| &p.path == path)
                    .map(|p| p.title)
                    .unwrap_or_else(|| path.last().cloned().unwrap_or_default()),
                String::new(),
            ),
            NodeKind::Background => ("Background".into(), String::new()),
            NodeKind::Appearance => ("Border & Corners".into(), String::new()),
        }
    }

    /// Crumb path for a node.
    pub(super) fn breadcrumb_path(&self, kind: &NodeKind) -> Vec<NodeKind> {
        match kind {
            NodeKind::Root => vec![NodeKind::Root],
            NodeKind::Section(path) => {
                // Up through the ancestor pages to Display.
                let mut chain = vec![kind.clone()];
                let mut parent = self.page_node(&path[..path.len() - 1]);
                while let NodeKind::Section(p) = &parent {
                    chain.push(parent.clone());
                    parent = self.page_node(&p[..p.len() - 1]);
                }
                chain.push(NodeKind::Display);
                chain.push(NodeKind::Root);
                chain.reverse();
                chain
            }
            NodeKind::ContentGroup(_) => vec![NodeKind::Root, NodeKind::Content, kind.clone()],
            NodeKind::ContentItem(slot) => item_chain(slot),
            NodeKind::ItemSource(slot) => {
                let mut chain = item_chain(slot);
                chain.push(kind.clone());
                chain
            }
            NodeKind::ItemSection(slot, path) => {
                // Up through the ancestor pages to the item.
                let mut above = vec![kind.clone()];
                let mut parent = self.item_page_node(slot, &path[..path.len() - 1]);
                while let NodeKind::ItemSection(_, p) = &parent {
                    above.push(parent.clone());
                    parent = self.item_page_node(slot, &p[..p.len() - 1]);
                }
                above.reverse();
                let mut chain = item_chain(slot);
                chain.extend(above);
                chain
            }
            other => vec![NodeKind::Root, other.clone()],
        }
    }

    pub(super) fn alert(&self, message: &str, detail: &str) {
        // Never raise a modal alert without a live parent: it would block
        // every window while staying invisible.
        let Some(window) = self.window().filter(|w| w.is_visible()) else {
            log::warn!("{}: {}", message, detail);
            return;
        };
        let dialog = gtk4::AlertDialog::builder()
            .modal(true)
            .message(message)
            .detail(detail)
            .buttons(["OK"])
            .build();
        dialog.show(Some(&window));
    }
}

/// Default config for a displayer: the saved "default style" for that id if
/// there is one, else the type default.
pub(super) fn default_displayer_config(id: &str) -> DisplayerConfig {
    let defaults = crate::config::DefaultsConfig::load();
    defaults
        .get_displayer_default(id)
        .and_then(|v| DisplayerConfig::from_value_for_type(id, v.clone()))
        .or_else(|| DisplayerConfig::default_for_type(id))
        .unwrap_or_default()
}

pub(super) fn flat_button(label: &str) -> Button {
    let b = Button::with_label(label);
    b.add_css_class("flat");
    b
}

pub(super) fn start_polling(state: &Rc<State>) {
    let weak = Rc::downgrade(state);
    let alive = state.alive.clone();
    glib::timeout_add_local(POLL_PERIOD, move || {
        if !alive.get() {
            return glib::ControlFlow::Break;
        }
        match weak.upgrade() {
            Some(s) => {
                s.poll();
                glib::ControlFlow::Continue
            }
            None => glib::ControlFlow::Break,
        }
    });
}

/// Root › Content › Group › Item, for a slot.
fn item_chain(slot: &str) -> Vec<NodeKind> {
    let g = NodeKind::parse_slot(slot).map(|(g, _)| g).unwrap_or(0);
    vec![
        NodeKind::Root,
        NodeKind::Content,
        NodeKind::ContentGroup(g),
        NodeKind::ContentItem(slot.to_string()),
    ]
}

/// Whether a slot's source has settings to show (a Source child node).
fn slot_has_source_editor(source_id: &str) -> bool {
    !source_id.is_empty() && source_id != "none" && editor_registry::has_source_editor(source_id)
}

/// The longest prefix of `path` that is a page: the page itself when it
/// exists, else its nearest existing ancestor page. Pure, so the shapes the
/// tree meets are tested below.
pub(super) fn resolve_page_prefix(path: &[String], pages: &[Vec<String>]) -> Option<Vec<String>> {
    (1..=path.len())
        .rev()
        .map(|k| &path[..k])
        .find(|prefix| pages.iter().any(|p| p.as_slice() == *prefix))
        .map(<[String]>::to_vec)
}

/// The node for a display editor page path, else Display.
pub(super) fn resolve_page_node(path: &[String], pages: &[Vec<String>]) -> NodeKind {
    resolve_page_prefix(path, pages)
        .map(NodeKind::Section)
        .unwrap_or(NodeKind::Display)
}

/// The node for an item display editor page path, else the item.
pub(super) fn item_node_for(slot: &str, path: &[String], pages: &[Vec<String>]) -> NodeKind {
    match resolve_page_prefix(path, pages) {
        Some(prefix) => NodeKind::ItemSection(slot.to_string(), prefix),
        None => NodeKind::ContentItem(slot.to_string()),
    }
}

#[cfg(test)]
mod page_node_tests {
    use super::*;

    fn v(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn a_section_is_its_own_node_and_the_empty_path_is_display() {
        let pages = vec![v(&["Face"]), v(&["Hands"])];
        assert_eq!(
            resolve_page_node(&v(&["Hands"]), &pages),
            NodeKind::Section(v(&["Hands"]))
        );
        assert_eq!(resolve_page_node(&[], &pages), NodeKind::Display);
        assert_eq!(resolve_page_node(&v(&["Icon"]), &pages), NodeKind::Display);
    }

    #[test]
    fn item_under_display_when_the_editor_has_no_sections() {
        let pages = vec![v(&["lines", "0"])];
        assert_eq!(
            resolve_page_node(&v(&["lines", "0"]), &pages),
            NodeKind::Section(v(&["lines", "0"]))
        );
        // Its parent (the list) has no page: Display.
        assert_eq!(resolve_page_node(&v(&["lines"]), &pages), NodeKind::Display);
    }

    #[test]
    fn item_under_its_section() {
        let pages = vec![
            v(&["Style"]),
            v(&["Text overlay"]),
            v(&["Text overlay", "lines", "0"]),
        ];
        assert_eq!(
            resolve_page_node(&v(&["Text overlay", "lines"]), &pages),
            NodeKind::Section(v(&["Text overlay"]))
        );
        assert_eq!(
            resolve_page_node(&v(&["Text overlay", "lines", "0"]), &pages),
            NodeKind::Section(v(&["Text overlay", "lines", "0"]))
        );
        assert_eq!(
            resolve_page_node(&v(&["Colours"]), &pages),
            NodeKind::Display
        );
    }

    #[test]
    fn a_removed_item_falls_back_to_its_parent() {
        let pages = vec![v(&["Text overlay"]), v(&["Text overlay", "lines", "0"])];
        assert_eq!(
            resolve_page_node(&v(&["Text overlay", "lines", "1"]), &pages),
            NodeKind::Section(v(&["Text overlay"]))
        );
    }

    #[test]
    fn item_pages_resolve_to_item_sections_or_the_item() {
        let slot = "group2_3";
        let sectioned = vec![v(&["Style"]), v(&["Text overlay", "lines", "0"])];
        assert_eq!(
            item_node_for(slot, &v(&["Text overlay", "lines", "0"]), &sectioned),
            NodeKind::ItemSection(slot.into(), v(&["Text overlay", "lines", "0"]))
        );
        // Its parent (the list) has no page, nor has its section here: the item.
        assert_eq!(
            item_node_for(slot, &v(&["Text overlay", "lines"]), &sectioned),
            NodeKind::ContentItem(slot.into())
        );
        assert_eq!(
            item_node_for(slot, &[], &sectioned),
            NodeKind::ContentItem(slot.into())
        );
        // A text item: lines straight under the item.
        let flat = vec![v(&["lines", "0"]), v(&["lines", "1"])];
        assert_eq!(
            item_node_for(slot, &v(&["lines", "1"]), &flat),
            NodeKind::ItemSection(slot.into(), v(&["lines", "1"]))
        );
        assert_eq!(
            item_node_for(slot, &v(&["lines", "2"]), &flat),
            NodeKind::ContentItem(slot.into())
        );
    }
}
