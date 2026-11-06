//! Dialog state: the working model, lazily built editors, live commit and
//! revert.
//!
//! The live panel is the truth. `applied` mirrors what the grid currently
//! shows; editors hold working values; `commit_now` assembles a `PanelData`
//! from them and hands it to `GridLayout::apply_panel_data`. `snapshot` is
//! the state at open, for Revert.

use super::node::NodeKind;
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
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::{Rc, Weak};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// Debounce for live commits: coalesces spin-button ticks and slider drags.
const COMMIT_DEBOUNCE: Duration = Duration::from_millis(50);
/// Poll period for editors without a change signal (the source widgets and
/// two clock widgets); only the visible page is polled.
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

/// Everything built for one content item page.
pub(super) struct ItemEditor {
    pub source_editor: RefCell<Option<Box<dyn SourceEditor>>>,
    /// The editor has no change signal and must be polled.
    pub needs_poll: Cell<bool>,
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
    /// Detached themed-editor pages other than Theme and Content.
    pub(super) display_pages: RefCell<Vec<(String, Widget)>>,
    /// Deeper pages of a schema display editor (list items), filed under
    /// their nearest ancestor page or Display; paths have two or more parts.
    pub(super) display_subpages: RefCell<Vec<EditorPage>>,
    pub(super) theme_page: RefCell<Option<Widget>>,
    pub(super) background_editor: RefCell<Option<Rc<BackgroundConfigWidget>>>,
    pub(super) inline_theme_widget: RefCell<Option<Rc<GlobalThemeWidget>>>,
    pub(super) item_editors: RefCell<HashMap<String, Rc<ItemEditor>>>,
    pub(super) item_theme_refreshers: Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
    pub(super) item_cleanup: Rc<RefCell<Vec<Box<dyn Fn()>>>>,
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
        if self.is_combo() {
            self.display_editor
                .borrow()
                .as_ref()
                .and_then(|e| e.store())
                .and_then(|c| c.theme().cloned())
        } else {
            self.inline_theme_widget
                .borrow()
                .as_ref()
                .map(|w| w.get_config())
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
        self.loading.set(false);
        self.rebuild_tree(Some(NodeKind::Root));
    }

    pub(super) fn cleanup_editors(&self) {
        for cb in self.item_cleanup.borrow_mut().drain(..) {
            cb();
        }
        self.item_theme_refreshers.borrow_mut().clear();
        for (_, item) in self.item_editors.borrow_mut().drain() {
            if let Some(e) = item.source_editor.borrow().as_ref() {
                e.cleanup();
            }
        }
        if let Some(e) = self.source_editor.borrow_mut().take() {
            e.cleanup();
        }
        if let Some(e) = self.display_editor.borrow_mut().take() {
            e.cleanup();
        }
        self.display_pages.borrow_mut().clear();
        self.display_subpages.borrow_mut().clear();
        *self.theme_page.borrow_mut() = None;
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
                editor.set_source_summaries(combo_summaries(cs));
            }
        }
        let weak = Rc::downgrade(self);
        let has_signal = editor.set_on_change(Rc::new(move || {
            if let Some(s) = weak.upgrade() {
                s.changed();
            }
        }));
        self.display_needs_poll.set(!has_signal);

        if let Some(combo) = editor.combo() {
            let mut sections = Vec::new();
            for (name, page) in combo.take_pages() {
                match name.as_str() {
                    "Theme" => *self.theme_page.borrow_mut() = Some(page),
                    _ => sections.push((name, page)),
                }
            }
            *self.display_pages.borrow_mut() = sections;
        } else {
            // Schema-driven displayers: one child node per section, and one
            // per detached list item (text lines).
            self.adopt_editor_pages(&**editor);
            let weak = Rc::downgrade(self);
            editor.set_on_sections_changed(Rc::new(move |select| {
                if let Some(s) = weak.upgrade() {
                    s.refresh_display_pages(select);
                }
            }));
        }
        *self.display_editor.borrow_mut() = Some(editor);
        *self.theme_baseline.borrow_mut() = self.inline_payload();
    }

    /// Split an editor's pages into top-level sections and deeper pages.
    fn adopt_editor_pages(&self, editor: &dyn DisplayEditor) {
        let mut sections = Vec::new();
        let mut subpages = Vec::new();
        for page in editor.sections() {
            match page.path.as_slice() {
                [name] => sections.push((name.clone(), page.widget)),
                [_, _, ..] => subpages.push(page),
                [] => {}
            }
        }
        *self.display_pages.borrow_mut() = sections;
        *self.display_subpages.borrow_mut() = subpages;
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
        let current_pages: Vec<EditorPage> = self
            .display_pages
            .borrow()
            .iter()
            .map(|(n, w)| EditorPage {
                path: vec![n.clone()],
                widget: w.clone(),
            })
            .chain(self.display_subpages.borrow().iter().cloned())
            .collect();
        let new_pages = editor.sections();
        let unchanged = current_pages.len() == new_pages.len()
            && current_pages
                .iter()
                .zip(new_pages.iter())
                .all(|(a, b)| a.path == b.path && a.widget == b.widget);
        if unchanged {
            if let Some(path) = select {
                self.select_node(&self.page_node(&path));
            }
            return;
        }
        let stale: Vec<String> = self
            .display_subpages
            .borrow()
            .iter()
            .map(|p| NodeKind::Section(p.path.clone()).id())
            .collect();
        for id in stale {
            if let Some(page) = self.stack.child_by_name(&id) {
                self.stack.remove(&page);
            }
            self.pages.borrow_mut().remove(&id);
            self.poll_last.borrow_mut().remove(&id);
        }
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
        let sections: Vec<String> = self
            .display_pages
            .borrow()
            .iter()
            .map(|(n, _)| n.clone())
            .collect();
        let subpages: Vec<Vec<String>> = self
            .display_subpages
            .borrow()
            .iter()
            .map(|p| p.path.clone())
            .collect();
        resolve_page_node(path, &sections, &subpages)
    }

    fn node_exists(&self, kind: &NodeKind) -> bool {
        match kind {
            NodeKind::Section(path) => self
                .display_subpages
                .borrow()
                .iter()
                .any(|p| &p.path == path),
            NodeKind::DisplaySection(name) => {
                self.display_pages.borrow().iter().any(|(n, _)| n == name)
            }
            _ => true,
        }
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
        for r in self.item_theme_refreshers.borrow().iter() {
            r();
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
        let (summaries, fields) = match self.combo_source.borrow().as_ref() {
            Some(cs) => (combo_summaries(cs), combo_fields(cs)),
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
                e.set_source_summaries(summaries);
            }
            self.loading.set(false);
        }
        if let Some(bg) = self.background_editor.borrow().as_ref() {
            bg.set_source_fields(fields);
        }
        self.rebuild_tree(Some(reselect));
        self.changed();
    }

    fn drop_item_pages(&self) {
        let slots: Vec<String> = self.item_editors.borrow().keys().cloned().collect();
        for slot in slots {
            let id = NodeKind::ContentItem(slot.clone()).id();
            if let Some(page) = self.stack.child_by_name(&id) {
                self.stack.remove(&page);
            }
            self.pages.borrow_mut().remove(&id);
            self.poll_last.borrow_mut().remove(&id);
        }
        for (_, item) in self.item_editors.borrow_mut().drain() {
            if let Some(e) = item.source_editor.borrow().as_ref() {
                e.cleanup();
            }
        }
        for id in ["content".to_string()]
            .into_iter()
            .chain((0..64).map(|g| NodeKind::ContentGroup(g).id()))
        {
            if let Some(page) = self.stack.child_by_name(&id) {
                self.stack.remove(&page);
                self.pages.borrow_mut().remove(&id);
            }
        }
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
            let slot = slot.to_string();
            self.defer(move |state| state.structure_changed(NodeKind::ContentItem(slot)));
        }
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

        let sections: Vec<NodeKind> = self
            .display_pages
            .borrow()
            .iter()
            .map(|(n, _)| NodeKind::DisplaySection(n.clone()))
            .collect();
        spec.insert(NodeKind::Display.id(), sections);
        for page in self.display_subpages.borrow().iter() {
            let parent = self.page_node(&page.path[..page.path.len() - 1]);
            spec.entry(parent.id())
                .or_default()
                .push(NodeKind::Section(page.path.clone()));
        }

        if let Some(cs) = self.combo_source.borrow().as_ref() {
            let groups: Vec<NodeKind> = (0..cs.groups.len()).map(NodeKind::ContentGroup).collect();
            spec.insert(NodeKind::Content.id(), groups);
            for (g, group) in cs.groups.iter().enumerate() {
                let items = (1..=group.item_count)
                    .map(|m| NodeKind::ContentItem(format!("group{}_{}", g + 1, m)))
                    .collect();
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
        if let Some(target) = reselect {
            self.select_node(&target);
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
            NodeKind::ContentItem(slot) => self
                .item_editors
                .borrow()
                .get(slot)
                .and_then(|i| i.source_editor.borrow().as_ref().map(|e| e.search_terms()))
                .unwrap_or_default(),
            NodeKind::DisplaySection(name) => self
                .display_editor
                .borrow()
                .as_ref()
                .map(|e| e.page_search_terms(std::slice::from_ref(name)))
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

    pub(super) fn select_node(&self, target: &NodeKind) {
        let n = self.selection.n_items();
        for i in 0..n {
            let kind = self
                .selection
                .item(i)
                .and_then(|o| o.downcast::<TreeListRow>().ok())
                .and_then(|row| row.item())
                .and_then(|o| NodeKind::from_object(&o));
            if kind.as_ref() == Some(target) {
                if self.selection.selected() == i {
                    // Same position: GTK fires no notify, drive the page ourselves.
                    self.selection.set_selected(gtk4::INVALID_LIST_POSITION);
                }
                self.selection.set_selected(i);
                return;
            }
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
            NodeKind::DisplaySection(n) => (n.clone(), String::new()),
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
                    .map(|i| format!("{:?}", i.display_as))
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
            NodeKind::Background => ("Background".into(), String::new()),
            NodeKind::Appearance => ("Border & Corners".into(), String::new()),
        }
    }

    /// Crumb path for a node.
    pub(super) fn breadcrumb_path(&self, kind: &NodeKind) -> Vec<NodeKind> {
        match kind {
            NodeKind::Root => vec![NodeKind::Root],
            NodeKind::DisplaySection(_) => vec![NodeKind::Root, NodeKind::Display, kind.clone()],
            NodeKind::Section(path) => {
                let mut chain = vec![kind.clone()];
                let mut parent = self.page_node(&path[..path.len() - 1]);
                loop {
                    match &parent {
                        NodeKind::Section(p) => {
                            chain.push(parent.clone());
                            parent = self.page_node(&p[..p.len() - 1]);
                        }
                        NodeKind::DisplaySection(_) => {
                            chain.push(parent.clone());
                            break;
                        }
                        _ => break,
                    }
                }
                chain.push(NodeKind::Display);
                chain.push(NodeKind::Root);
                chain.reverse();
                chain
            }
            NodeKind::ContentGroup(_) => vec![NodeKind::Root, NodeKind::Content, kind.clone()],
            NodeKind::ContentItem(slot) => {
                let g = NodeKind::parse_slot(slot).map(|(g, _)| g).unwrap_or(0);
                vec![
                    NodeKind::Root,
                    NodeKind::Content,
                    NodeKind::ContentGroup(g),
                    kind.clone(),
                ]
            }
            other => vec![NodeKind::Root, other.clone()],
        }
    }

    pub(super) fn alert(&self, message: &str, detail: &str) {
        let dialog = gtk4::AlertDialog::builder()
            .modal(true)
            .message(message)
            .detail(detail)
            .buttons(["OK"])
            .build();
        dialog.show(self.window().as_ref());
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

/// The node for an editor page path: the page itself when it exists (a
/// one-part path is a section, longer paths are list items), else its
/// nearest existing ancestor, else Display. Pure, so the three shapes the
/// tree meets are tested below.
pub(super) fn resolve_page_node(
    path: &[String],
    sections: &[String],
    subpages: &[Vec<String>],
) -> NodeKind {
    for k in (1..=path.len()).rev() {
        let prefix = &path[..k];
        if k == 1 {
            if sections.iter().any(|s| s == &prefix[0]) {
                return NodeKind::DisplaySection(prefix[0].clone());
            }
        } else if subpages.iter().any(|p| p.as_slice() == prefix) {
            return NodeKind::Section(prefix.to_vec());
        }
    }
    NodeKind::Display
}

#[cfg(test)]
mod page_node_tests {
    use super::*;

    fn v(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn item_under_display_when_the_editor_has_no_sections() {
        let subpages = vec![v(&["lines", "0"])];
        assert_eq!(
            resolve_page_node(&v(&["lines", "0"]), &[], &subpages),
            NodeKind::Section(v(&["lines", "0"]))
        );
        // Its parent (the list) has no page: Display.
        assert_eq!(
            resolve_page_node(&v(&["lines"]), &[], &subpages),
            NodeKind::Display
        );
    }

    #[test]
    fn item_under_its_section() {
        let sections = v(&["Style", "Text overlay"]);
        let subpages = vec![v(&["Text overlay", "lines", "0"])];
        assert_eq!(
            resolve_page_node(&v(&["Text overlay", "lines"]), &sections, &subpages),
            NodeKind::DisplaySection("Text overlay".into())
        );
        assert_eq!(
            resolve_page_node(&v(&["Colours"]), &sections, &subpages),
            NodeKind::Display
        );
    }

    #[test]
    fn a_removed_item_falls_back_to_its_parent() {
        let sections = v(&["Text overlay"]);
        let subpages = vec![v(&["Text overlay", "lines", "0"])];
        assert_eq!(
            resolve_page_node(&v(&["Text overlay", "lines", "1"]), &sections, &subpages),
            NodeKind::DisplaySection("Text overlay".into())
        );
    }
}
