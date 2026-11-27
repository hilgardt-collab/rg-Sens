//! Page builders: one editor page per node kind, built on first selection.

use super::node::NodeKind;
use super::schema_editor::SchemaEditor;
use super::state::{
    flat_button, theme_choice_index, theme_mode_hint, AppearanceControls, GeometryControls, State,
    ThemeChoice, ThemeControls,
};
use super::util::*;
use crate::core::{global_registry, DisplayerConfig};
use crate::ui::theme::{ComboThemeConfig, ThemeRef};
use crate::ui::{ColorButtonWidget, ComboReorder, GlobalThemeWidget, PanelStyle, CLIPBOARD};
use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, CheckButton, DropDown, Entry, Label, Orientation, ScrolledWindow,
    SpinButton, StringList, Widget,
};
use rg_sens_types::combo::GroupTitleConfig;
use rg_sens_types::schema::Schema;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

fn page_box() -> GtkBox {
    let b = GtkBox::new(Orientation::Vertical, 10);
    b.set_margin_start(12);
    b.set_margin_end(12);
    b.set_margin_top(12);
    b.set_margin_bottom(12);
    b
}

fn heading(text: &str) -> Label {
    let l = Label::new(Some(text));
    l.add_css_class("heading");
    l.set_halign(gtk4::Align::Start);
    l
}

/// Take a widget out of the box it sits in, so it can be placed on a page
/// (an editor's widget outlives the page that showed it).
fn detach(widget: &Widget) {
    if let Some(parent) = widget.parent().and_then(|p| p.downcast::<GtkBox>().ok()) {
        parent.remove(widget);
    }
}

fn hint(text: &str) -> Label {
    let l = Label::new(Some(text));
    l.add_css_class("dim-label");
    l.set_halign(gtk4::Align::Start);
    l.set_wrap(true);
    l.set_xalign(0.0);
    l
}

fn spin_row(
    container: &GtkBox,
    label: &str,
    min: f64,
    max: f64,
    step: f64,
    value: f64,
) -> SpinButton {
    let row = GtkBox::new(Orientation::Horizontal, 8);
    let l = Label::new(Some(label));
    l.set_width_chars(14);
    l.set_xalign(0.0);
    let spin = SpinButton::with_range(min, max, step);
    spin.set_value(value);
    spin.set_hexpand(true);
    row.append(&l);
    row.append(&spin);
    container.append(&row);
    spin
}

fn dropdown_row(container: &GtkBox, label: &str, names: &[String], selected: usize) -> DropDown {
    let row = GtkBox::new(Orientation::Horizontal, 8);
    let l = Label::new(Some(label));
    l.set_width_chars(14);
    l.set_xalign(0.0);
    let refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
    let dd = DropDown::new(Some(StringList::new(&refs)), None::<gtk4::Expression>);
    dd.set_selected(selected as u32);
    dd.set_hexpand(true);
    row.append(&l);
    row.append(&dd);
    container.append(&row);
    dd
}

/// Which page a CSS template's custom layout rows are for.
enum CssRows {
    Group(usize),
    Item(String),
}

impl State {
    /// Make sure the page for `kind` exists in the stack; returns its name.
    pub(super) fn ensure_page(self: &Rc<Self>, kind: &NodeKind) -> String {
        let id = kind.id();
        if !self.pages.borrow().contains(&id) {
            let page = self.build_page(kind);
            self.stack.add_named(&page, Some(&id));
            self.pages.borrow_mut().insert(id.clone());
        }
        id
    }

    fn build_page(self: &Rc<Self>, kind: &NodeKind) -> Widget {
        self.loading.set(true);
        let inner: Widget = match kind {
            NodeKind::Root => self.build_root_page().upcast(),
            NodeKind::Geometry => self.build_geometry_page().upcast(),
            NodeKind::Theme => self.build_theme_page().upcast(),
            NodeKind::Source => self.build_source_page().upcast(),
            NodeKind::Display => self.build_display_page().upcast(),
            NodeKind::Content => self.build_content_page().upcast(),
            NodeKind::ContentGroup(g) => self.build_group_page(*g).upcast(),
            NodeKind::ContentItem(slot) => self.build_item_page(slot).upcast(),
            NodeKind::Background => self.build_background_page().upcast(),
            NodeKind::Appearance => self.build_appearance_page().upcast(),
            NodeKind::Section(path) => self.build_subpage(path).upcast(),
            NodeKind::ItemSource(slot) => self.build_item_source_page(slot).upcast(),
            NodeKind::ItemSection(slot, path) => self.build_item_subpage(slot, path).upcast(),
        };
        self.loading.set(false);
        let scroll = ScrolledWindow::new();
        scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);
        scroll.set_vexpand(true);
        scroll.set_hexpand(true);
        scroll.set_child(Some(&inner));
        scroll.upcast()
    }

    // ---- Root --------------------------------------------------------------

    fn build_root_page(self: &Rc<Self>) -> GtkBox {
        let page = page_box();
        let (title, sub) = self.row_text(&NodeKind::Root);
        page.append(&heading(&format!("{} panel", title)));
        page.append(&hint(&format!("Source: {}", sub)));
        let group_text = self
            .group_id()
            .and_then(|gid| self.app_config.borrow().group(&gid).map(|g| g.name.clone()))
            .map(|n| format!("Group: {}", n))
            .unwrap_or_else(|| "Not in a group".to_string());
        page.append(&hint(&group_text));
        page.append(&hint(
            "Select a node on the left to edit it. Changes apply to the panel \
             immediately; Revert restores the state from when this window opened.",
        ));
        page
    }

    // ---- Geometry ----------------------------------------------------------

    fn build_geometry_page(self: &Rc<Self>) -> GtkBox {
        let page = page_box();
        let applied = self.applied.borrow().clone();
        page.append(&heading("Panel Size"));
        let width = spin_row(
            &page,
            "Width (cells):",
            1.0,
            512.0,
            1.0,
            applied.geometry.width as f64,
        );
        let height = spin_row(
            &page,
            "Height (cells):",
            1.0,
            512.0,
            1.0,
            applied.geometry.height as f64,
        );
        page.append(&heading("Content Transform"));
        let scale = spin_row(&page, "Scale:", 0.1, 5.0, 0.1, applied.appearance.scale);
        scale.set_digits(2);
        let translate_x = spin_row(
            &page,
            "Offset X:",
            -500.0,
            500.0,
            1.0,
            applied.appearance.translate_x,
        );
        let translate_y = spin_row(
            &page,
            "Offset Y:",
            -500.0,
            500.0,
            1.0,
            applied.appearance.translate_y,
        );
        page.append(&heading("Layering"));
        let z_index = spin_row(
            &page,
            "Z-index:",
            -100.0,
            100.0,
            1.0,
            applied.appearance.z_index as f64,
        );
        z_index.set_tooltip_text(Some("Higher values bring the panel in front of others"));
        let ignore_collision = CheckButton::with_label("Ignore collision (allow overlap)");
        ignore_collision.set_active(applied.appearance.ignore_collision);
        page.append(&ignore_collision);

        for spin in [
            &width,
            &height,
            &scale,
            &translate_x,
            &translate_y,
            &z_index,
        ] {
            let weak = Rc::downgrade(self);
            spin.connect_value_changed(move |_| {
                if let Some(s) = weak.upgrade() {
                    s.changed();
                }
            });
        }
        {
            let weak = Rc::downgrade(self);
            ignore_collision.connect_toggled(move |_| {
                if let Some(s) = weak.upgrade() {
                    s.changed();
                }
            });
        }
        *self.geometry.borrow_mut() = Some(GeometryControls {
            width,
            height,
            scale,
            translate_x,
            translate_y,
            z_index,
            ignore_collision,
        });
        page
    }

    // ---- Theme -------------------------------------------------------------

    fn build_theme_page(self: &Rc<Self>) -> GtkBox {
        let page = page_box();
        page.append(&heading("Theme"));
        page.append(&hint(
            "Colours and fonts marked as theme references resolve against this theme. \
             Inherit uses the panel group's theme, or the global theme when the panel is \
             not in a group. Named themes and groups are managed in the Themes & Groups \
             window (dashboard context menu).",
        ));

        let mut choices = vec![ThemeChoice::Inherit, ThemeChoice::Global];
        let mut names = vec![
            "Inherit (group, else global)".to_string(),
            "Global".to_string(),
        ];
        {
            let cfg = self.app_config.borrow();
            for name in cfg.themes.keys() {
                choices.push(ThemeChoice::Named(name.clone()));
                names.push(name.clone());
            }
        }
        for (name, _) in ComboThemeConfig::builtin_presets() {
            choices.push(ThemeChoice::Named(name.to_string()));
            names.push(format!("{} (built-in)", name));
        }
        choices.push(ThemeChoice::Inline);
        names.push("Inline (private to this panel)".to_string());

        let current = self.theme_ref.borrow().clone();
        let dropdown = dropdown_row(
            &page,
            "Use theme:",
            &names,
            theme_choice_index(&choices, &current),
        );
        let mode_hint = hint(theme_mode_hint(&current));
        page.append(&mode_hint);

        // The theme editor is always shown: the resolved theme for a shared
        // reference, the private theme for Inline. Editing under a shared
        // reference detaches the panel (`State::detach_theme_if_edited`).
        // Preset dropdown: the displayer's own palettes, then the
        // built-ins. Applies to the editor below (an edit, so a shared
        // reference detaches to Inline as any other edit would).
        let displayer = self.displayer_type();
        let presets: Vec<(String, ComboThemeConfig)> =
            ComboThemeConfig::palette_presets(&displayer)
                .into_iter()
                .map(|(n, t)| (n.to_string(), t))
                .chain(
                    ComboThemeConfig::builtin_presets()
                        .into_iter()
                        .map(|(n, t)| (format!("{} (built-in)", n), t)),
                )
                .collect();
        let mut preset_names = vec!["Apply a preset…".to_string()];
        preset_names.extend(presets.iter().map(|(n, _)| n.clone()));
        let preset_dropdown = dropdown_row(&page, "Preset:", &preset_names, 0);

        let widget = Rc::new(GlobalThemeWidget::new());
        // Inline: the reference's payload (no editor exists yet to ask);
        // shared: the resolved theme.
        widget.set_config(self.resolved_theme());
        {
            let weak = Rc::downgrade(self);
            widget.set_on_change(move || {
                if let Some(s) = weak.upgrade() {
                    s.theme_editor_changed();
                }
            });
        }
        {
            let weak = Rc::downgrade(self);
            let widget = widget.clone();
            let presets = Rc::new(presets);
            preset_dropdown.connect_selected_notify(move |dd| {
                let Some(s) = weak.upgrade() else { return };
                if s.loading.get() {
                    return;
                }
                let sel = dd.selected() as usize;
                if sel == 0 {
                    return;
                }
                if let Some((_, theme)) = presets.get(sel - 1) {
                    let was = s.loading.replace(true);
                    widget.set_config(theme.clone());
                    dd.set_selected(0);
                    s.loading.set(was);
                    s.theme_editor_changed();
                }
            });
        }
        page.append(widget.widget());
        *self.inline_theme_widget.borrow_mut() = Some(widget);
        *self.theme_baseline.borrow_mut() = self.inline_payload();
        {
            let weak = Rc::downgrade(self);
            dropdown.connect_selected_notify(move |dd| {
                let Some(s) = weak.upgrade() else { return };
                if s.loading.get() {
                    return;
                }
                let choice = s
                    .theme_controls
                    .borrow()
                    .as_ref()
                    .and_then(|t| t.choices.get(dd.selected() as usize).cloned());
                let theme_ref = match choice {
                    Some(ThemeChoice::Inherit) => ThemeRef::Inherit,
                    Some(ThemeChoice::Global) => ThemeRef::Global,
                    Some(ThemeChoice::Named(n)) => ThemeRef::named(n),
                    Some(ThemeChoice::Inline) => ThemeRef::inline(s.resolved_theme()),
                    None => return,
                };
                s.apply_theme_ref(theme_ref);
            });
        }
        *self.theme_controls.borrow_mut() = Some(ThemeControls {
            choices,
            dropdown,
            mode_hint,
        });
        page
    }

    // ---- Source (standalone) -------------------------------------------------

    fn build_source_page(self: &Rc<Self>) -> GtkBox {
        let page = page_box();
        page.append(&heading("Data Source"));
        let infos = global_registry().list_sources_with_info();
        let ids: Vec<String> = infos.iter().map(|s| s.id.clone()).collect();
        let names: Vec<String> = infos.iter().map(|s| s.display_name.clone()).collect();
        let current = self.source_type();
        let selected = ids.iter().position(|i| *i == current).unwrap_or(0);
        let dropdown = dropdown_row(&page, "Source:", &names, selected);
        {
            let weak = Rc::downgrade(self);
            let ids = ids.clone();
            dropdown.connect_selected_notify(move |dd| {
                let Some(s) = weak.upgrade() else { return };
                if s.loading.get() {
                    return;
                }
                if let Some(id) = ids.get(dd.selected() as usize) {
                    s.switch_source_type(id);
                }
            });
        }

        let btn_row = GtkBox::new(Orientation::Horizontal, 6);
        let copy_btn = Button::with_label("Copy Source Config");
        let paste_btn = Button::with_label("Paste Source Config");
        btn_row.append(&copy_btn);
        btn_row.append(&paste_btn);
        page.append(&btn_row);
        {
            let weak = Rc::downgrade(self);
            copy_btn.connect_clicked(move |_| {
                let Some(s) = weak.upgrade() else { return };
                let cfg = s.source_editor.borrow().as_ref().and_then(|e| e.store());
                if let Some(cfg) = cfg {
                    if let Ok(mut cb) = CLIPBOARD.lock() {
                        let json =
                            serde_json::to_value(flatten_source_config(&cfg)).unwrap_or_default();
                        cb.copy_source_config(cfg.source_type().to_string(), json);
                    }
                }
            });
        }
        {
            let weak = Rc::downgrade(self);
            paste_btn.connect_clicked(move |_| {
                let Some(s) = weak.upgrade() else { return };
                let pasted = CLIPBOARD
                    .lock()
                    .ok()
                    .and_then(|cb| cb.paste_source_config());
                let Some((source_type, value)) = pasted else {
                    return;
                };
                if source_type != s.source_type() {
                    log::info!(
                        "Source config paste skipped: clipboard has '{}'",
                        source_type
                    );
                    return;
                }
                let flat: std::collections::HashMap<String, serde_json::Value> = value
                    .as_object()
                    .map(|o| o.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                    .unwrap_or_default();
                if let Some(cfg) = source_config_from_flat(&source_type, &flat) {
                    if let Some(e) = s.source_editor.borrow().as_ref() {
                        e.load(&cfg);
                    }
                    s.changed();
                }
            });
        }

        match self.ensure_source_editor() {
            Some(widget) => page.append(&widget),
            None => page.append(&hint("This source has no settings.")),
        }
        page
    }

    // ---- Display -----------------------------------------------------------

    fn build_display_page(self: &Rc<Self>) -> GtkBox {
        let page = page_box();
        page.append(&heading("Display Type"));
        let infos = global_registry().get_compatible_displayers(&self.source_type());
        let ids: Vec<String> = infos.iter().map(|d| d.id.clone()).collect();
        let names: Vec<String> = infos.iter().map(|d| d.display_name.clone()).collect();
        let current = self.displayer_type();
        let selected = ids.iter().position(|i| *i == current).unwrap_or(0);
        let dropdown = dropdown_row(&page, "Displayer:", &names, selected);
        {
            let weak = Rc::downgrade(self);
            dropdown.connect_selected_notify(move |dd| {
                let Some(s) = weak.upgrade() else { return };
                if s.loading.get() {
                    return;
                }
                if let Some(id) = ids.get(dd.selected() as usize) {
                    s.switch_displayer_type(id);
                }
            });
        }

        let default_btn = Button::with_label("Save as Default Style");
        default_btn.set_halign(gtk4::Align::Start);
        default_btn.set_tooltip_text(Some(
            "New panels with this displayer start from this configuration",
        ));
        {
            let weak = Rc::downgrade(self);
            default_btn.connect_clicked(move |_| {
                let Some(s) = weak.upgrade() else { return };
                let cfg = s.display_editor.borrow().as_ref().and_then(|e| e.store());
                let Some(cfg) = cfg else { return };
                if let Some(value) = cfg.to_inner_value() {
                    let mut defaults = crate::config::DefaultsConfig::load();
                    defaults.set_displayer_default(cfg.displayer_type(), value);
                    if let Err(e) = defaults.save() {
                        s.alert("Could not save default style", &e.to_string());
                    }
                }
            });
        }
        page.append(&default_btn);

        // Whole-config copy/paste, tagged by displayer type (replaces the
        // per-widget Copy/Paste buttons the bar and arc editors had).
        let btn_row = GtkBox::new(Orientation::Horizontal, 6);
        let copy_btn = Button::with_label("Copy Display Config");
        let paste_btn = Button::with_label("Paste Display Config");
        btn_row.append(&copy_btn);
        btn_row.append(&paste_btn);
        page.append(&btn_row);
        {
            let weak = Rc::downgrade(self);
            copy_btn.connect_clicked(move |_| {
                let Some(s) = weak.upgrade() else { return };
                let cfg = s.display_editor.borrow().as_ref().and_then(|e| e.store());
                let Some(cfg) = cfg else { return };
                if let (Some(value), Ok(mut cb)) = (cfg.to_inner_value(), CLIPBOARD.lock()) {
                    cb.copy_display_config(cfg.displayer_type().to_string(), value);
                }
            });
        }
        {
            let weak = Rc::downgrade(self);
            paste_btn.connect_clicked(move |_| {
                let Some(s) = weak.upgrade() else { return };
                let pasted = CLIPBOARD
                    .lock()
                    .ok()
                    .and_then(|cb| cb.paste_display_config());
                let Some((displayer_type, value)) = pasted else {
                    return;
                };
                if displayer_type != s.displayer_type() {
                    log::info!(
                        "Display config paste skipped: clipboard has '{}'",
                        displayer_type
                    );
                    return;
                }
                let Some(cfg) = DisplayerConfig::from_value_for_type(&displayer_type, value) else {
                    return;
                };
                s.load_display_config(cfg);
                s.changed();
            });
        }

        let is_combo_editor = self
            .display_editor
            .borrow()
            .as_ref()
            .map(|e| e.combo().is_some())
            .unwrap_or(false);
        if is_combo_editor {
            page.append(&hint(
                "The sections of this displayer (frame, headers, layout, …) are the \
                 child nodes of Display. Per-item display settings are under Content.",
            ));
        } else if self.has_sections() {
            page.append(&hint(
                "The settings of this displayer are grouped under the child nodes of Display.",
            ));
        } else if let Some(editor) = self.display_editor.borrow().as_ref() {
            page.append(&editor.widget());
        } else {
            page.append(&hint("No editor is available for this displayer."));
        }
        page
    }

    /// A page of the display editor: a section or a detached list item,
    /// under a heading with the node's live title.
    fn build_subpage(self: &Rc<Self>, path: &[String]) -> GtkBox {
        let page = page_box();
        let (title, _) = self.row_text(&NodeKind::Section(path.to_vec()));
        page.append(&heading(&title));
        let widget = self
            .display_pages
            .borrow()
            .iter()
            .find(|p| p.path == path)
            .map(|p| p.widget.clone());
        match widget {
            Some(w) => {
                detach(&w);
                page.append(&w);
            }
            None => page.append(&hint("Page not available.")),
        }
        page
    }

    // ---- Content (combo) -----------------------------------------------------

    fn build_content_page(self: &Rc<Self>) -> GtkBox {
        let page = page_box();
        page.append(&heading("Content"));
        let (groups, interval) = self
            .combo_source
            .borrow()
            .as_ref()
            .map(|cs| (cs.groups.len(), cs.update_interval_ms))
            .unwrap_or((1, 1000));
        let group_spin = spin_row(&page, "Groups:", 1.0, 16.0, 1.0, groups as f64);
        {
            let weak = Rc::downgrade(self);
            group_spin.connect_value_changed(move |spin| {
                if let Some(s) = weak.upgrade() {
                    if !s.loading.get() {
                        s.set_group_count(spin.value() as usize);
                    }
                }
            });
        }
        let interval_spin = spin_row(
            &page,
            "Update (ms):",
            100.0,
            60000.0,
            100.0,
            interval as f64,
        );
        {
            let weak = Rc::downgrade(self);
            interval_spin.connect_value_changed(move |spin| {
                if let Some(s) = weak.upgrade() {
                    if !s.loading.get() {
                        s.set_update_interval(spin.value() as u64);
                    }
                }
            });
        }
        page.append(&hint(
            "Each group is a child node; each item is a node under its group, with \
             its source settings and display sections as child nodes of its own. \
             An item whose Source page sets its own update interval keeps it; the \
             panel polls at the fastest interval any item asks for.",
        ));
        page.append(&heading("Group sizes"));
        page.append(&self.group_weight_board());
        page
    }

    /// The groups as blocks on the panel's own shape, laid out as the panel
    /// lays them out (a frame's stack, a CSS page's columns), each spanning
    /// cells by its size weight: drag a block's far edge across the cells
    /// to make it bigger or smaller, drag a block onto another to swap the
    /// two groups.
    fn group_weight_board(self: &Rc<Self>) -> GtkBox {
        use crate::ui::group_board::{arrange, span_of, weight_of, Board, Flow, MAX_SPAN};
        use std::cell::Cell;
        const PAD: f64 = 6.0;
        /// The least room it takes; it grows into whatever the page has.
        const MIN_HEIGHT: i32 = 260;
        /// How near the far edge, in pixels, a press resizes instead of moves.
        const EDGE: f64 = 8.0;
        #[derive(Clone, Copy, PartialEq)]
        enum Drag {
            Idle,
            /// `per_cell`: pixels a cell had when the drag began.
            Resize {
                group: usize,
                from: u32,
                per_cell: f64,
                span: u32,
            },
            Move {
                group: usize,
                to: usize,
            },
        }
        struct Shape {
            px: (f64, f64),
            flow: Flow,
            titles: Vec<String>,
        }
        let shape = Rc::new(RefCell::new(Shape {
            px: (1.0, 1.0),
            flow: Flow::Down,
            titles: Vec::new(),
        }));
        let refresh = {
            let (weak, shape) = (Rc::downgrade(self), shape.clone());
            Rc::new(move || {
                let Some(s) = weak.upgrade() else { return };
                let (px, flow, titles) = s.group_board_shape();
                *shape.borrow_mut() = Shape { px, flow, titles };
            })
        };
        let drag = Rc::new(Cell::new(Drag::Idle));
        // (spans, item counts) as the source has them, the drag's span in
        // place of the source's while one is on.
        let groups = {
            let (weak, drag) = (Rc::downgrade(self), drag.clone());
            Rc::new(move || -> (Vec<u32>, Vec<u32>) {
                let Some(s) = weak.upgrade() else {
                    return (Vec::new(), Vec::new());
                };
                let cs = s.combo_source.borrow();
                let Some(cs) = cs.as_ref() else {
                    return (Vec::new(), Vec::new());
                };
                let mut spans: Vec<u32> =
                    cs.groups.iter().map(|g| span_of(g.size_weight)).collect();
                if let Drag::Resize { group, span, .. } = drag.get() {
                    if let Some(at) = spans.get_mut(group) {
                        *at = span;
                    }
                }
                (spans, cs.groups.iter().map(|g| g.item_count).collect())
            })
        };
        // The panel's rectangle on the area, and the board in it.
        let frame_of = {
            let shape = shape.clone();
            move |width: f64, height: f64| -> (f64, f64, f64, f64) {
                let (pw, ph) = shape.borrow().px;
                let (aw, ah) = (width - 2.0 * PAD, height - 2.0 * PAD);
                let k = (aw / pw).min(ah / ph);
                let (w, h) = (pw * k, ph * k);
                ((width - w) / 2.0, (height - h) / 2.0, w, h)
            }
        };
        let frame_of = Rc::new(frame_of);
        // A block's rectangle in pixels.
        fn rect(board: &Board, frame: (f64, f64, f64, f64), i: usize) -> (f64, f64, f64, f64) {
            let (fx, fy, fw, fh) = frame;
            let (cw, ch) = (fw / board.cols as f64, fh / board.rows as f64);
            let b = board.blocks[i];
            (
                fx + b.col as f64 * cw,
                fy + b.row as f64 * ch,
                b.wide as f64 * cw,
                b.high as f64 * ch,
            )
        }
        let board_now = {
            let (shape, groups) = (shape.clone(), groups.clone());
            Rc::new(move || arrange(&groups().0, shape.borrow().flow))
        };

        let area = gtk4::DrawingArea::new();
        area.set_content_height(MIN_HEIGHT);
        area.set_hexpand(true);
        area.set_vexpand(true);
        {
            let (shape, groups, drag, frame_of, board_now) = (
                shape.clone(),
                groups.clone(),
                drag.clone(),
                frame_of.clone(),
                board_now.clone(),
            );
            area.set_draw_func(move |area, cr, width, height| {
                let (spans, counts) = groups();
                if spans.is_empty() {
                    return;
                }
                let board = board_now();
                let frame = frame_of(width as f64, height as f64);
                let fg = area.color();
                let (r, g, b) = (fg.red() as f64, fg.green() as f64, fg.blue() as f64);
                // the panel and its cells
                let (fx, fy, fw, fh) = frame;
                cr.rectangle(fx + 0.5, fy + 0.5, fw - 1.0, fh - 1.0);
                cr.set_source_rgba(r, g, b, 0.05);
                cr.fill_preserve().ok();
                cr.set_source_rgba(r, g, b, 0.35);
                cr.set_line_width(1.0);
                cr.stroke().ok();
                cr.set_source_rgba(r, g, b, 0.1);
                let (cw, ch) = (fw / board.cols as f64, fh / board.rows as f64);
                if cw >= 4.0 {
                    for c in 1..board.cols {
                        cr.move_to((fx + c as f64 * cw).round() + 0.5, fy);
                        cr.line_to((fx + c as f64 * cw).round() + 0.5, fy + fh);
                    }
                }
                if ch >= 4.0 {
                    for row in 1..board.rows {
                        cr.move_to(fx, (fy + row as f64 * ch).round() + 0.5);
                        cr.line_to(fx + fw, (fy + row as f64 * ch).round() + 0.5);
                    }
                }
                cr.stroke().ok();

                let layout = pangocairo::functions::create_layout(cr);
                let mut font = pango::FontDescription::new();
                font.set_size(8 * pango::SCALE);
                layout.set_font_description(Some(&font));
                layout.set_ellipsize(pango::EllipsizeMode::End);
                let (active, target) = match drag.get() {
                    Drag::Move { group, to } => (Some(group), Some(to)),
                    Drag::Resize { group, .. } => (Some(group), None),
                    Drag::Idle => (None, None),
                };
                let shape = shape.borrow();
                for (i, span) in spans.iter().enumerate().take(board.blocks.len()) {
                    let (x, y, w, h) = rect(&board, frame, i);
                    let (x, y, w, h) = (x + 2.0, y + 2.0, (w - 4.0).max(1.0), (h - 4.0).max(1.0));
                    let lifted = active == Some(i);
                    let aimed = target == Some(i) && active != Some(i);
                    cr.save().ok();
                    cr.rectangle(x, y, w, h);
                    cr.set_source_rgba(r, g, b, if lifted || aimed { 0.25 } else { 0.1 });
                    cr.fill_preserve().ok();
                    cr.set_source_rgba(r, g, b, if lifted || aimed { 0.95 } else { 0.5 });
                    cr.set_line_width(if lifted || aimed { 2.0 } else { 1.0 });
                    cr.stroke().ok();
                    // the edge that resizes
                    cr.set_source_rgba(r, g, b, 0.6);
                    if shape.flow == Flow::Across {
                        cr.rectangle(x + w - 3.0, y + h / 2.0 - 8.0, 2.0, 16.0);
                    } else {
                        cr.rectangle(x + w / 2.0 - 8.0, y + h - 3.0, 16.0, 2.0);
                    }
                    cr.fill().ok();
                    cr.rectangle(x, y, w, h);
                    cr.clip();
                    let title = shape
                        .titles
                        .get(i)
                        .filter(|t| !t.is_empty())
                        .cloned()
                        .unwrap_or_else(|| format!("Group {}", i + 1));
                    layout.set_width(((w - 8.0).max(1.0) * pango::SCALE as f64) as i32);
                    cr.set_source_rgba(r, g, b, 0.95);
                    layout.set_text(&title);
                    cr.move_to(x + 4.0, y + 2.0);
                    pangocairo::functions::show_layout(cr, &layout);
                    if h > 28.0 {
                        let n = counts.get(i).copied().unwrap_or(0);
                        cr.set_source_rgba(r, g, b, 0.6);
                        layout.set_text(&format!(
                            "×{} · {n} item{}",
                            weight_of(*span),
                            if n == 1 { "" } else { "s" }
                        ));
                        cr.move_to(x + 4.0, y + 15.0);
                        pangocairo::functions::show_layout(cr, &layout);
                    }
                    cr.restore().ok();
                }
            });
        }

        let gesture = gtk4::GestureDrag::new();
        {
            let (shape, drag, frame_of, board_now, refresh) = (
                shape.clone(),
                drag.clone(),
                frame_of.clone(),
                board_now.clone(),
                refresh.clone(),
            );
            let weak_area = area.downgrade();
            gesture.connect_drag_begin(move |_, px, py| {
                let Some(area) = weak_area.upgrade() else {
                    return;
                };
                refresh();
                drag.set(Drag::Idle);
                let board = board_now();
                let frame = frame_of(area.width() as f64, area.height() as f64);
                let across = shape.borrow().flow == Flow::Across;
                for i in 0..board.blocks.len() {
                    let (x, y, w, h) = rect(&board, frame, i);
                    if px < x || px > x + w || py < y || py > y + h + EDGE / 2.0 {
                        continue;
                    }
                    let b = board.blocks[i];
                    let near = if across {
                        x + w - px <= EDGE
                    } else {
                        y + h - py <= EDGE
                    };
                    drag.set(if near {
                        let (from, per_cell) = if across {
                            (b.wide, w / b.wide as f64)
                        } else {
                            (b.high, h / b.high as f64)
                        };
                        Drag::Resize {
                            group: i,
                            from,
                            per_cell,
                            span: from,
                        }
                    } else {
                        Drag::Move { group: i, to: i }
                    });
                    break;
                }
                area.queue_draw();
            });
        }
        {
            let (shape, drag, frame_of, board_now) = (
                shape.clone(),
                drag.clone(),
                frame_of.clone(),
                board_now.clone(),
            );
            let weak_area = area.downgrade();
            gesture.connect_drag_update(move |gesture, dx, dy| {
                let (Some(area), Some((sx, sy))) = (weak_area.upgrade(), gesture.start_point())
                else {
                    return;
                };
                match drag.get() {
                    Drag::Resize {
                        group,
                        from,
                        per_cell,
                        ..
                    } => {
                        let delta = if shape.borrow().flow == Flow::Across {
                            dx
                        } else {
                            dy
                        };
                        let span = (from as f64 + (delta / per_cell.max(1.0)).round())
                            .clamp(1.0, MAX_SPAN as f64) as u32;
                        drag.set(Drag::Resize {
                            group,
                            from,
                            per_cell,
                            span,
                        });
                    }
                    Drag::Move { group, .. } => {
                        let (x, y) = (sx + dx, sy + dy);
                        let board = board_now();
                        let frame = frame_of(area.width() as f64, area.height() as f64);
                        let to = (0..board.blocks.len())
                            .find(|&i| {
                                let (bx, by, bw, bh) = rect(&board, frame, i);
                                x >= bx && x < bx + bw && y >= by && y < by + bh
                            })
                            .unwrap_or(group);
                        drag.set(Drag::Move { group, to });
                    }
                    Drag::Idle => return,
                }
                area.queue_draw();
            });
        }
        {
            let weak = Rc::downgrade(self);
            let weak_area = area.downgrade();
            gesture.connect_drag_end(move |_, _, _| {
                let was = drag.replace(Drag::Idle);
                let (Some(s), Some(area)) = (weak.upgrade(), weak_area.upgrade()) else {
                    return;
                };
                match was {
                    Drag::Resize {
                        group, from, span, ..
                    } if span != from => {
                        s.set_group_weight(group, weight_of(span));
                    }
                    Drag::Move { group, to } if to != group => {
                        s.reorder(ComboReorder::Group { a: group, b: to });
                    }
                    _ => {}
                }
                area.queue_draw();
            });
        }
        area.add_controller(gesture);
        // The panel's size, the groups and their titles may change while the
        // page is hidden.
        area.connect_map(move |area| {
            refresh();
            area.queue_draw();
        });

        let board = GtkBox::new(Orientation::Vertical, 4);
        board.append(&area);
        board.append(&hint(
            "The panel, with each group as a block laid out as the panel lays them \
             out. Drag the marked edge of a group across the cells to give it more \
             or less room (each cell is a quarter of a size weight); drag a group \
             onto another to swap them.",
        ));
        board
    }

    fn build_group_page(self: &Rc<Self>, group: usize) -> GtkBox {
        let page = page_box();
        page.append(&heading(&format!("Group {}", group + 1)));
        let (count, total) = self
            .combo_source
            .borrow()
            .as_ref()
            .map(|cs| {
                let g = cs.groups.get(group);
                (g.map(|g| g.item_count).unwrap_or(1), cs.groups.len())
            })
            .unwrap_or((1, 1));
        let count_spin = spin_row(&page, "Items:", 1.0, 16.0, 1.0, count as f64);
        {
            let weak = Rc::downgrade(self);
            count_spin.connect_value_changed(move |spin| {
                if let Some(s) = weak.upgrade() {
                    if !s.loading.get() {
                        s.set_item_count(group, spin.value() as u32);
                    }
                }
            });
        }
        // How the group lays out its items: stored in the frame beside the
        // group's weight, so only a themed frame has it.
        let orientation = self
            .display_editor
            .borrow()
            .as_ref()
            .and_then(|e| e.combo().map(|c| c.group_orientation(group)));
        if let Some(orientation) = orientation {
            let names = ["Vertical".to_string(), "Horizontal".to_string()];
            let selected = usize::from(orientation == "horizontal");
            let dropdown = dropdown_row(&page, "Item layout:", &names, selected);
            dropdown.set_tooltip_text(Some(
                "Stack this group's items vertically or place them side by side",
            ));
            {
                let weak = Rc::downgrade(self);
                dropdown.connect_selected_notify(move |d| {
                    if let Some(s) = weak.upgrade() {
                        if !s.loading.get() {
                            let tag = if d.selected() == 1 {
                                "horizontal"
                            } else {
                                "vertical"
                            };
                            s.set_group_orientation(group, tag);
                        }
                    }
                });
            }
            {
                // A group without its own entry follows the group layout
                // under Display › Layout, which may change while this page
                // is hidden: re-read whenever the page is shown.
                let weak = Rc::downgrade(self);
                dropdown.connect_map(move |d| {
                    if let Some(s) = weak.upgrade() {
                        let current = s
                            .display_editor
                            .borrow()
                            .as_ref()
                            .and_then(|e| e.combo().map(|c| c.group_orientation(group)));
                        if let Some(current) = current {
                            let was = s.loading.replace(true);
                            d.set_selected(u32::from(current == "horizontal"));
                            s.loading.set(was);
                        }
                    }
                });
            }
        }
        let order = GtkBox::new(Orientation::Horizontal, 6);
        order.append(&Label::new(Some("Order:")));
        let up = Button::from_icon_name("go-up-symbolic");
        up.set_tooltip_text(Some("Move this group earlier"));
        up.set_sensitive(group > 0);
        let down = Button::from_icon_name("go-down-symbolic");
        down.set_tooltip_text(Some("Move this group later"));
        down.set_sensitive(group + 1 < total);
        order.append(&up);
        order.append(&down);
        page.append(&order);
        {
            let weak = Rc::downgrade(self);
            up.connect_clicked(move |_| {
                if let Some(s) = weak.upgrade() {
                    s.reorder(ComboReorder::Group {
                        a: group,
                        b: group - 1,
                    });
                }
            });
        }
        {
            let weak = Rc::downgrade(self);
            down.connect_clicked(move |_| {
                if let Some(s) = weak.upgrade() {
                    s.reorder(ComboReorder::Group {
                        a: group,
                        b: group + 1,
                    });
                }
            });
        }
        // Title bar: stored in the frame (its font and colours resolve
        // against the frame's theme), edited beside the group's structure.
        // Only a themed frame draws one (css_template has no frame model).
        let current = self
            .display_editor
            .borrow()
            .as_ref()
            .and_then(|e| e.combo().map(|c| c.group_title(group)))
            .and_then(|v| v.as_object().cloned());
        if let Some(current) = current {
            page.append(&heading("Title bar"));
            let title_editor = Rc::new(SchemaEditor::with_fields(
                GroupTitleConfig::props(),
                Vec::new(),
            ));
            title_editor.set_theme(&self.resolved_theme());
            title_editor.load(current);
            {
                let weak = Rc::downgrade(self);
                let editor = title_editor.clone();
                title_editor.set_on_change(Rc::new(move || {
                    if let Some(s) = weak.upgrade() {
                        s.set_group_title(group, serde_json::Value::Object(editor.model()));
                    }
                }));
            }
            page.append(title_editor.widget());
            self.group_editors.borrow_mut().insert(group, title_editor);
        }
        page.append(&self.css_layout_rows(CssRows::Group(group)));
        page
    }

    /// The rows a CSS template's custom layout adds to a Group or Item
    /// page. The pages are built once and kept, while the layout can be
    /// switched on, the template changed or a source swapped elsewhere in
    /// the dialog, so the rows are rebuilt each time the page is shown.
    fn css_layout_rows(self: &Rc<Self>, target: CssRows) -> GtkBox {
        let holder = GtkBox::new(Orientation::Vertical, 10);
        let weak = Rc::downgrade(self);
        holder.connect_map(move |holder| {
            if let Some(s) = weak.upgrade() {
                s.fill_css_layout_rows(holder, &target);
            }
        });
        holder
    }

    fn fill_css_layout_rows(self: &Rc<Self>, holder: &GtkBox, target: &CssRows) {
        use crate::ui::css_template_layout::SIZES;
        use rg_sens_types::display_configs::{CssWidgetChoice, CSS_LAYOUT_MAX_ITEMS};

        while let Some(child) = holder.first_child() {
            holder.remove(&child);
        }
        let Some((cfg, spec)) = self.css_layout() else {
            return;
        };
        if !spec.usable() {
            return;
        }
        holder.append(&heading("Layout"));
        if !cfg.layout.custom {
            holder.append(&hint(
                "The template arranges its own groups. Choose \"My own groups and widgets\" \
                 under Display to set titles and place widgets here.",
            ));
            return;
        }
        match target {
            CssRows::Group(group) => {
                let group = *group;
                let current = cfg.layout.groups.get(group).cloned().unwrap_or_default();
                let row = GtkBox::new(Orientation::Horizontal, 8);
                let label = Label::new(Some("Title:"));
                label.set_width_chars(14);
                label.set_xalign(0.0);
                let entry = Entry::new();
                entry.set_placeholder_text(Some(&format!("Group {}", group + 1)));
                entry.set_text(&current.title);
                entry.set_hexpand(true);
                row.append(&label);
                row.append(&entry);
                holder.append(&row);
                {
                    // Applied once typing pauses: an edit reloads the
                    // display editor and the page.
                    let weak = Rc::downgrade(self);
                    let typed = Rc::new(std::cell::Cell::new(0u32));
                    entry.connect_changed(move |entry| {
                        typed.set(typed.get().wrapping_add(1));
                        let (weak, typed, at) = (weak.clone(), typed.clone(), typed.get());
                        let entry = entry.downgrade();
                        gtk4::glib::timeout_add_local_once(
                            std::time::Duration::from_millis(350),
                            move || {
                                let (Some(s), Some(entry)) = (weak.upgrade(), entry.upgrade())
                                else {
                                    return;
                                };
                                if typed.get() != at {
                                    return;
                                }
                                let title = entry.text().to_string();
                                s.edit_css_layout(move |layout| {
                                    if let Some(g) = layout.groups.get_mut(group) {
                                        g.title = title;
                                    }
                                });
                            },
                        );
                    });
                }

                if spec.groups.len() > 1 {
                    let mut ids = vec![String::new()];
                    let mut names = vec!["Automatic (follows the first widget)".to_string()];
                    for proto in &spec.groups {
                        ids.push(proto.id.clone());
                        names.push(proto.label.clone());
                    }
                    let selected = ids.iter().position(|id| *id == current.style).unwrap_or(0);
                    let style = dropdown_row(holder, "Style:", &names, selected);
                    let weak = Rc::downgrade(self);
                    style.connect_selected_notify(move |d| {
                        let Some(s) = weak.upgrade() else { return };
                        let id = ids.get(d.selected() as usize).cloned().unwrap_or_default();
                        s.edit_css_layout(move |layout| {
                            if let Some(g) = layout.groups.get_mut(group) {
                                g.style = id;
                            }
                        });
                    });
                }
                holder.append(&self.css_layout_board(holder, group, &cfg, &spec));
                let count = cfg.layout.item_counts.get(group).copied().unwrap_or(0) as usize;
                let drawn = (1..=count)
                    .filter(|i| {
                        cfg.layout
                            .widgets
                            .get(&format!("group{}_{}", group + 1, i))
                            .is_none_or(|w| w.kind != crate::ui::css_template_layout::HIDDEN)
                    })
                    .count();
                holder.append(&hint(&if drawn > CSS_LAYOUT_MAX_ITEMS {
                    format!(
                        "A group shows at most {CSS_LAYOUT_MAX_ITEMS} widgets; this one has \
                         {drawn}, so the last {} are not drawn. Move them to another group \
                         or set their widget to \"Not shown\".",
                        drawn - CSS_LAYOUT_MAX_ITEMS
                    )
                } else if drawn == 0 {
                    "Every item of this group is \"Not shown\", so the group is not drawn."
                        .to_string()
                } else {
                    format!("Showing {drawn} of at most {CSS_LAYOUT_MAX_ITEMS} widgets.")
                }));
            }
            CssRows::Item(slot) => {
                let source = self
                    .combo_source
                    .borrow()
                    .as_ref()
                    .and_then(|cs| cs.slots.get(slot))
                    .map(|s| s.source_id.clone())
                    .unwrap_or_default();
                let current = cfg.layout.widgets.get(slot).cloned().unwrap_or_default();
                // Another widget or size offers other sizes and colours.
                let refill: Rc<dyn Fn(&Rc<State>)> = {
                    let (holder, slot) = (holder.downgrade(), slot.clone());
                    Rc::new(move |s| {
                        let (holder, slot) = (holder.clone(), slot.clone());
                        s.defer(move |s| {
                            if let Some(holder) = holder.upgrade() {
                                s.fill_css_layout_rows(&holder, &CssRows::Item(slot));
                            }
                        });
                    })
                };
                let kinds = spec.kinds_for(&source);
                let names: Vec<String> = kinds.iter().map(|(_, l)| l.clone()).collect();
                let selected = kinds
                    .iter()
                    .position(|(k, _)| *k == current.kind)
                    .unwrap_or(0);
                let widget = dropdown_row(holder, "Widget:", &names, selected);
                widget.set_tooltip_text(Some(
                    "How the template draws this item; the list follows the item's source",
                ));
                {
                    let weak = Rc::downgrade(self);
                    let (slot, refill) = (slot.clone(), refill.clone());
                    widget.connect_selected_notify(move |d| {
                        let Some(s) = weak.upgrade() else { return };
                        let Some((kind, _)) = kinds.get(d.selected() as usize).cloned() else {
                            return;
                        };
                        let key = slot.clone();
                        s.edit_css_layout(move |layout| {
                            layout.widgets.insert(
                                key,
                                CssWidgetChoice {
                                    kind,
                                    ..Default::default()
                                },
                            );
                        });
                        refill(&s);
                    });
                }

                // Every widget scales to the cells it is given.
                if current.kind != crate::ui::css_template_layout::HIDDEN {
                    let mut names: Vec<String> = SIZES.iter().map(|(_, l)| l.to_string()).collect();
                    let shown = spec.size_of(&current);
                    let mut selected = SIZES.iter().position(|(id, _)| *id == shown).unwrap_or(0);
                    // A block shaped on the board is its own entry; picking
                    // a size above goes back to that size's square.
                    if current.span.is_some() {
                        let (w, h) = spec.block_of(&current);
                        selected = names.len();
                        names.push(format!("Custom ({w} x {h})"));
                    }
                    let size = dropdown_row(holder, "Size:", &names, selected);
                    size.set_tooltip_text(Some(
                        "The cells the widget takes on its group's grid. Place it, or drag \
                         its edges to any other shape (a bar at 4 x 1), on the board on the \
                         group's page",
                    ));
                    let weak = Rc::downgrade(self);
                    let (slot, refill) = (slot.clone(), refill.clone());
                    size.connect_selected_notify(move |d| {
                        let Some(s) = weak.upgrade() else { return };
                        let Some((id, _)) = SIZES.get(d.selected() as usize).copied() else {
                            return;
                        };
                        let key = slot.clone();
                        s.edit_css_layout(move |layout| {
                            let widget = layout.widgets.entry(key).or_default();
                            widget.size = id.to_string();
                            widget.span = None;
                        });
                        refill(&s);
                    });
                }

                // The template's colours for this widget, if it has any.
                let colors = spec.colors_of(&current).to_vec();
                if !colors.is_empty() {
                    let mut names = vec!["Automatic (the colours take turns)".to_string()];
                    names.extend(colors.iter().map(|c| {
                        let mut chars = c.chars();
                        chars
                            .next()
                            .map(|f| f.to_uppercase().chain(chars).collect::<String>())
                            .unwrap_or_default()
                            .replace(['-', '_'], " ")
                    }));
                    let selected = colors
                        .iter()
                        .position(|c| *c == current.color)
                        .map_or(0, |at| at + 1);
                    let color = dropdown_row(holder, "Colour:", &names, selected);
                    let weak = Rc::downgrade(self);
                    let slot = slot.clone();
                    color.connect_selected_notify(move |d| {
                        let Some(s) = weak.upgrade() else { return };
                        let picked = match d.selected() as usize {
                            0 => String::new(),
                            at => colors.get(at - 1).cloned().unwrap_or_default(),
                        };
                        let key = slot.clone();
                        s.edit_css_layout(move |layout| {
                            layout.widgets.entry(key).or_default().color = picked;
                        });
                    });
                }
            }
        }
    }

    /// The group's widgets on its grid of square cells: drag a box to put
    /// it anywhere free, drag its right or bottom edge (or that corner) to
    /// give it any block of cells. The page scales the grid, and each
    /// widget, to the room the group has.
    fn css_layout_board(
        self: &Rc<Self>,
        holder: &GtkBox,
        group: usize,
        cfg: &rg_sens_types::display_configs::CssTemplateDisplayConfig,
        spec: &crate::ui::css_template_layout::LayoutSpec,
    ) -> GtkBox {
        use crate::ui::css_template_layout::{overlap, place, size_for, HIDDEN, MAX_SPAN};
        use rg_sens_types::display_configs::CSS_LAYOUT_MAX_ITEMS;
        use std::cell::Cell;

        const PAD: f64 = 6.0;
        const MAX_UNIT: f64 = 38.0;
        /// How near an edge, in pixels, a press resizes instead of moves.
        const EDGE: f64 = 9.0;

        struct BoardItem {
            slot: String,
            label: String,
            kind: String,
            /// (column, row, columns, rows) as the page places it now.
            cell: (u32, u32, u32, u32),
        }
        #[derive(Clone, Copy, PartialEq)]
        enum Drag {
            Idle,
            /// `grab` is the cell of the box the pointer took hold of;
            /// `to` where the box would land.
            Move {
                item: usize,
                grab: (u32, u32),
                to: (u32, u32),
            },
            /// `wide` / `high`: which of the two the pointer changes.
            Resize {
                item: usize,
                wide: bool,
                high: bool,
                to: (u32, u32),
            },
        }

        let labels: HashMap<String, String> = self
            .combo_source
            .borrow()
            .as_ref()
            .map(|cs| {
                combo_slot_kinds(cs)
                    .into_iter()
                    .map(|k| {
                        let caption = cs
                            .slots
                            .get(&k.prefix)
                            .map(|s| s.caption_override.clone())
                            .filter(|c| !c.is_empty());
                        (k.prefix, caption.unwrap_or(k.label))
                    })
                    .collect()
            })
            .unwrap_or_default();
        let kind_labels: HashMap<String, String> = spec.kinds().into_iter().collect();
        let count = cfg.layout.item_counts.get(group).copied().unwrap_or(0) as usize;
        let mut drawn: Vec<(String, rg_sens_types::display_configs::CssWidgetChoice)> = Vec::new();
        for index in 0..count {
            let slot = format!("group{}_{}", group + 1, index + 1);
            let choice = cfg.layout.widgets.get(&slot).cloned().unwrap_or_default();
            if choice.kind != HIDDEN && drawn.len() < CSS_LAYOUT_MAX_ITEMS {
                drawn.push((slot, choice));
            }
        }
        let placement = place(
            &drawn
                .iter()
                .map(|(_, c)| (spec.block_of(c), c.pos))
                .collect::<Vec<_>>(),
        );
        let items: Vec<BoardItem> = drawn
            .into_iter()
            .zip(&placement.cells)
            .map(|((slot, choice), cell)| BoardItem {
                label: labels.get(&slot).cloned().unwrap_or_else(|| slot.clone()),
                kind: kind_labels.get(&choice.kind).cloned().unwrap_or_default(),
                slot,
                cell: *cell,
            })
            .collect();
        // Room to grow: a large widget's worth to the right, a row below.
        let (cols, rows) = (
            (placement.cols + 4).clamp(8, 16),
            (placement.rows + 2).clamp(4, 16),
        );
        let items = Rc::new(items);
        let drag = Rc::new(Cell::new(Drag::Idle));

        let unit = move |width: f64| ((width - 2.0 * PAD) / cols as f64).min(MAX_UNIT);
        // The cell of every box as the drag would leave it, and whether
        // that is a place the dragged box can be (on the board, on nothing).
        let outcome = {
            let (items, drag) = (items.clone(), drag.clone());
            Rc::new(move || -> (Vec<(u32, u32, u32, u32)>, bool) {
                let mut cells: Vec<(u32, u32, u32, u32)> = items.iter().map(|i| i.cell).collect();
                let moved = match drag.get() {
                    Drag::Move { item, to, .. } => {
                        (cells[item].0, cells[item].1) = to;
                        item
                    }
                    Drag::Resize { item, to, .. } => {
                        (cells[item].2, cells[item].3) = to;
                        item
                    }
                    Drag::Idle => return (cells, true),
                };
                let block = cells[moved];
                let clear = cells
                    .iter()
                    .enumerate()
                    .all(|(i, other)| i == moved || !overlap(block, *other));
                (
                    cells,
                    clear && block.0 + block.2 <= cols && block.1 + block.3 <= rows,
                )
            })
        };

        let area = gtk4::DrawingArea::new();
        area.set_content_height((rows as f64 * MAX_UNIT + 2.0 * PAD) as i32);
        area.set_hexpand(true);
        {
            let (items, outcome, drag) = (items.clone(), outcome.clone(), drag.clone());
            area.set_draw_func(move |area, cr, width, _| {
                let fg = area.color();
                let (r, g, b) = (fg.red() as f64, fg.green() as f64, fg.blue() as f64);
                let u = unit(width as f64);
                // the cells
                cr.set_source_rgba(r, g, b, 0.12);
                cr.set_line_width(1.0);
                for c in 0..=cols {
                    cr.move_to(PAD + c as f64 * u + 0.5, PAD);
                    cr.line_to(PAD + c as f64 * u + 0.5, PAD + rows as f64 * u);
                }
                for row in 0..=rows {
                    cr.move_to(PAD, PAD + row as f64 * u + 0.5);
                    cr.line_to(PAD + cols as f64 * u, PAD + row as f64 * u + 0.5);
                }
                cr.stroke().ok();

                let layout = pangocairo::functions::create_layout(cr);
                let mut font = pango::FontDescription::new();
                font.set_size(8 * pango::SCALE);
                layout.set_font_description(Some(&font));
                layout.set_ellipsize(pango::EllipsizeMode::End);
                let active = match drag.get() {
                    Drag::Move { item, .. } | Drag::Resize { item, .. } => Some(item),
                    Drag::Idle => None,
                };
                let (cells, valid) = outcome();
                for (i, (item, (c, row, wide, high))) in items.iter().zip(&cells).enumerate() {
                    let (x, y) = (PAD + *c as f64 * u + 2.0, PAD + *row as f64 * u + 2.0);
                    let (bw, bh) = (*wide as f64 * u - 4.0, *high as f64 * u - 4.0);
                    let lifted = active == Some(i);
                    cr.save().ok();
                    cr.rectangle(x, y, bw, bh);
                    if lifted && !valid {
                        cr.set_source_rgba(0.85, 0.2, 0.2, 0.3);
                    } else {
                        cr.set_source_rgba(r, g, b, if lifted { 0.25 } else { 0.1 });
                    }
                    cr.fill_preserve().ok();
                    cr.set_source_rgba(r, g, b, if lifted { 0.95 } else { 0.5 });
                    cr.set_line_width(if lifted { 2.0 } else { 1.0 });
                    cr.stroke().ok();
                    // the corner that resizes
                    cr.set_source_rgba(r, g, b, 0.5);
                    cr.move_to(x + bw - 1.0, y + bh - 9.0);
                    cr.line_to(x + bw - 1.0, y + bh - 1.0);
                    cr.line_to(x + bw - 9.0, y + bh - 1.0);
                    cr.close_path();
                    cr.fill().ok();
                    cr.rectangle(x, y, bw, bh);
                    cr.clip();
                    cr.set_source_rgba(r, g, b, 0.95);
                    layout.set_width(((bw - 10.0).max(1.0) * pango::SCALE as f64) as i32);
                    layout.set_text(&item.label);
                    cr.move_to(x + 4.0, y + 3.0);
                    pangocairo::functions::show_layout(cr, &layout);
                    if *high > 1 {
                        cr.set_source_rgba(r, g, b, 0.6);
                        layout.set_text(&item.kind);
                        cr.move_to(x + 4.0, y + 17.0);
                        pangocairo::functions::show_layout(cr, &layout);
                    }
                    cr.restore().ok();
                }
            });
        }

        let gesture = gtk4::GestureDrag::new();
        {
            let (drag, items) = (drag.clone(), items.clone());
            let weak_area = area.downgrade();
            gesture.connect_drag_begin(move |_, x, y| {
                let Some(area) = weak_area.upgrade() else {
                    return;
                };
                let u = unit(area.width() as f64);
                let at = (
                    ((x - PAD) / u).floor().max(0.0) as u32,
                    ((y - PAD) / u).floor().max(0.0) as u32,
                );
                let hit = items
                    .iter()
                    .position(|i| overlap(i.cell, (at.0, at.1, 1, 1)));
                drag.set(match hit {
                    Some(item) => {
                        let (c, r, w, h) = items[item].cell;
                        let wide = x >= PAD + (c + w) as f64 * u - EDGE;
                        let high = y >= PAD + (r + h) as f64 * u - EDGE;
                        if wide || high {
                            Drag::Resize {
                                item,
                                wide,
                                high,
                                to: (w, h),
                            }
                        } else {
                            Drag::Move {
                                item,
                                grab: (at.0 - c, at.1 - r),
                                to: (c, r),
                            }
                        }
                    }
                    None => Drag::Idle,
                });
                area.queue_draw();
            });
        }
        {
            let (drag, items) = (drag.clone(), items.clone());
            let weak_area = area.downgrade();
            gesture.connect_drag_update(move |gesture, dx, dy| {
                let (Some(area), Some((sx, sy))) = (weak_area.upgrade(), gesture.start_point())
                else {
                    return;
                };
                let u = unit(area.width() as f64);
                let (x, y) = ((sx + dx - PAD) / u, (sy + dy - PAD) / u);
                match drag.get() {
                    Drag::Move { item, grab, .. } => {
                        let to = (
                            (x.floor() - grab.0 as f64).max(0.0) as u32,
                            (y.floor() - grab.1 as f64).max(0.0) as u32,
                        );
                        drag.set(Drag::Move { item, grab, to });
                    }
                    Drag::Resize {
                        item, wide, high, ..
                    } => {
                        // Each edge snaps to the cell line nearest the pointer.
                        let (c, r, w, h) = items[item].cell;
                        let snap = |edge: f64, from: u32| {
                            (edge - from as f64).round().clamp(1.0, MAX_SPAN as f64) as u32
                        };
                        let to = (
                            if wide { snap(x, c) } else { w },
                            if high { snap(y, r) } else { h },
                        );
                        drag.set(Drag::Resize {
                            item,
                            wide,
                            high,
                            to,
                        });
                    }
                    Drag::Idle => return,
                }
                area.queue_draw();
            });
        }
        {
            let weak = Rc::downgrade(self);
            let (weak_area, weak_holder) = (area.downgrade(), holder.downgrade());
            let items = items.clone();
            gesture.connect_drag_end(move |_, _, _| {
                let (cells, valid) = outcome();
                let was = drag.replace(Drag::Idle);
                let (Some(s), Some(area)) = (weak.upgrade(), weak_area.upgrade()) else {
                    return;
                };
                area.queue_draw();
                let changed = items.iter().zip(&cells).any(|(i, c)| i.cell != *c);
                if was == Drag::Idle || !valid || !changed {
                    return;
                }
                // Every box gets the cell it is in now, so the ones that
                // were placed automatically stay where the user saw them.
                let resized = match was {
                    Drag::Resize { item, .. } => Some(item),
                    _ => None,
                };
                let placed: Vec<(String, (u32, u32, u32, u32), bool)> = items
                    .iter()
                    .zip(cells)
                    .enumerate()
                    .map(|(i, (item, cell))| (item.slot.clone(), cell, resized == Some(i)))
                    .collect();
                s.edit_css_layout(move |layout| {
                    for (slot, (c, r, w, h), resized) in placed {
                        let widget = layout.widgets.entry(slot).or_default();
                        widget.pos = Some((c, r));
                        if !resized {
                            continue;
                        }
                        // A square the sizes cover is that size again.
                        match size_for(w).filter(|_| w == h) {
                            Some(id) => {
                                widget.size = id.to_string();
                                widget.span = None;
                            }
                            None => widget.span = Some((w, h)),
                        }
                    }
                });
                let holder = weak_holder.clone();
                s.defer(move |s| {
                    if let Some(holder) = holder.upgrade() {
                        s.fill_css_layout_rows(&holder, &CssRows::Group(group));
                    }
                });
            });
        }
        area.add_controller(gesture);

        let board = GtkBox::new(Orientation::Vertical, 4);
        let frame = gtk4::Frame::new(None);
        frame.set_child(Some(&area));
        board.append(&frame);
        board.append(&hint(
            "Drag a box to any free cells. Drag its right or bottom edge, or the marked \
             corner, to give it any block of cells (a bar at 4 x 1). The page fits the \
             grid, and each widget, to the room the group has.",
        ));
        board
    }

    /// The item itself: caption and order, which source feeds it and how it
    /// is shown. The source's settings are the item's Source child node; the
    /// display type's sections are its other children (a single-section type
    /// such as Text keeps its rows here, with its lines as children).
    fn build_item_page(self: &Rc<Self>, slot: &str) -> GtkBox {
        let page = page_box();
        let (group, item) = NodeKind::parse_slot(slot).unwrap_or((0, 0));
        page.append(&heading(&format!(
            "Item {} of Group {}",
            item + 1,
            group + 1
        )));

        // The whole item travels: its source (type and settings), caption
        // and display settings, onto any slot of any combo panel.
        let btn_row = GtkBox::new(Orientation::Horizontal, 6);
        let copy_btn = Button::with_label("Copy Item");
        copy_btn.set_tooltip_text(Some("Copy this item's source and display settings"));
        let paste_btn = Button::with_label("Paste Item");
        paste_btn.set_tooltip_text(Some("Replace this item with the copied one"));
        btn_row.append(&copy_btn);
        btn_row.append(&paste_btn);
        page.append(&btn_row);
        {
            let weak = Rc::downgrade(self);
            let slot = slot.to_string();
            copy_btn.connect_clicked(move |_| {
                if let Some(s) = weak.upgrade() {
                    s.copy_item(&slot);
                }
            });
        }
        {
            let weak = Rc::downgrade(self);
            let slot = slot.to_string();
            paste_btn.connect_clicked(move |_| {
                if let Some(s) = weak.upgrade() {
                    s.paste_item(&slot);
                }
            });
        }

        let (source_id, caption, item_count) = self
            .combo_source
            .borrow()
            .as_ref()
            .map(|cs| {
                let s = cs.slots.get(slot).cloned().unwrap_or_default();
                (
                    s.source_id,
                    s.caption_override,
                    cs.groups
                        .get(group)
                        .map(|g| g.item_count as usize)
                        .unwrap_or(1),
                )
            })
            .unwrap_or_default();

        // Caption + order
        let caption_row = GtkBox::new(Orientation::Horizontal, 8);
        let caption_label = Label::new(Some("Caption:"));
        caption_label.set_width_chars(14);
        caption_label.set_xalign(0.0);
        let caption_entry = Entry::new();
        caption_entry.set_placeholder_text(Some("(use the source's default)"));
        caption_entry.set_text(&caption);
        caption_entry.set_hexpand(true);
        caption_row.append(&caption_label);
        caption_row.append(&caption_entry);
        let up = Button::from_icon_name("go-up-symbolic");
        up.set_tooltip_text(Some("Move this item earlier"));
        up.set_sensitive(item > 0);
        let down = Button::from_icon_name("go-down-symbolic");
        down.set_tooltip_text(Some("Move this item later"));
        down.set_sensitive(item + 1 < item_count);
        caption_row.append(&up);
        caption_row.append(&down);
        page.append(&caption_row);
        {
            let weak = Rc::downgrade(self);
            let slot = slot.to_string();
            caption_entry.connect_changed(move |entry| {
                if let Some(s) = weak.upgrade() {
                    if !s.loading.get() {
                        s.set_item_caption(&slot, entry.text().to_string());
                    }
                }
            });
        }
        {
            let weak = Rc::downgrade(self);
            up.connect_clicked(move |_| {
                if let Some(s) = weak.upgrade() {
                    s.reorder(ComboReorder::Item {
                        group,
                        i: item,
                        j: item - 1,
                    });
                }
            });
        }
        {
            let weak = Rc::downgrade(self);
            down.connect_clicked(move |_| {
                if let Some(s) = weak.upgrade() {
                    s.reorder(ComboReorder::Item {
                        group,
                        i: item,
                        j: item + 1,
                    });
                }
            });
        }

        // Source type; its settings live on the Source child node.
        let infos = global_registry().list_sources_with_info();
        let mut source_ids = vec!["none".to_string()];
        let mut source_names = vec!["None".to_string()];
        for info in infos.iter().filter(|s| s.id != "combination") {
            source_ids.push(info.id.clone());
            source_names.push(info.display_name.clone());
        }
        let selected = source_ids.iter().position(|i| *i == source_id).unwrap_or(0);
        let source_dropdown = dropdown_row(&page, "Source:", &source_names, selected);
        {
            let weak = Rc::downgrade(self);
            let slot = slot.to_string();
            source_dropdown.connect_selected_notify(move |dd| {
                let Some(s) = weak.upgrade() else { return };
                if s.loading.get() {
                    return;
                }
                if let Some(id) = source_ids.get(dd.selected() as usize) {
                    s.set_item_source(&slot, id);
                }
            });
        }
        let editors = self.ensure_item_editor(slot);
        page.append(&hint(if source_id.is_empty() || source_id == "none" {
            "Select a data source for this item."
        } else if editors.source_editor.borrow().is_some() {
            "The source's settings are under the item's Source node."
        } else {
            "This source has no settings."
        }));

        // Display type and height; the type's sections are child nodes.
        match editors.display.as_ref() {
            Some(d) => {
                let w = d.widget();
                detach(&w);
                page.append(&w);
            }
            // A CSS template's item is shown through its bindings, or
            // through the widget chosen in the Layout rows below.
            None if self.displayer_type() == "css_template" => {}
            None => page.append(&hint(
                "This displayer has no per-item display settings; its mappings are under Display.",
            )),
        }
        page.append(&self.css_layout_rows(CssRows::Item(slot.to_string())));
        page
    }

    /// The source settings of one item: the source's editor (Copy/Paste
    /// Item on the item page carries these along).
    fn build_item_source_page(self: &Rc<Self>, slot: &str) -> GtkBox {
        let page = page_box();
        let source_id = self
            .combo_source
            .borrow()
            .as_ref()
            .and_then(|cs| cs.slots.get(slot))
            .map(|s| s.source_id.clone())
            .unwrap_or_default();
        page.append(&heading(&format!(
            "{} settings",
            source_display_name(&source_id)
        )));

        let editors = self.ensure_item_editor(slot);
        let widget = editors.source_editor.borrow().as_ref().map(|e| e.widget());
        match widget {
            Some(w) => {
                detach(&w);
                page.append(&w);
            }
            None => page.append(&hint("This source has no settings.")),
        }
        page
    }

    /// A page of an item's display editor: a section or a list item, under
    /// a heading with the node's live title.
    fn build_item_subpage(self: &Rc<Self>, slot: &str, path: &[String]) -> GtkBox {
        let page = page_box();
        let (title, _) = self.row_text(&NodeKind::ItemSection(slot.to_string(), path.to_vec()));
        page.append(&heading(&title));
        let editors = self.ensure_item_editor(slot);
        let widget = editors.display.as_ref().and_then(|d| d.page_widget(path));
        match widget {
            Some(w) => {
                detach(&w);
                page.append(&w);
            }
            None => page.append(&hint("Page not available.")),
        }
        page
    }

    // ---- Background ----------------------------------------------------------

    fn build_background_page(self: &Rc<Self>) -> GtkBox {
        let page = page_box();
        page.append(&heading("Background"));
        let bg = self.ensure_background_editor();
        page.append(bg.widget());
        page
    }

    // ---- Appearance ----------------------------------------------------------

    fn build_appearance_page(self: &Rc<Self>) -> GtkBox {
        let page = page_box();
        let applied = self.applied.borrow().clone();

        page.append(&heading("Panel Style"));
        let btn_row = GtkBox::new(Orientation::Horizontal, 6);
        let copy_btn = Button::with_label("Copy Style");
        let paste_btn = Button::with_label("Paste Style");
        btn_row.append(&copy_btn);
        btn_row.append(&paste_btn);
        page.append(&btn_row);

        page.append(&heading("Corners"));
        let radius = spin_row(
            &page,
            "Corner radius:",
            0.0,
            50.0,
            1.0,
            applied.appearance.corner_radius,
        );
        page.append(&heading("Border"));
        let border_enabled = CheckButton::with_label("Show border");
        border_enabled.set_active(applied.appearance.border.enabled);
        page.append(&border_enabled);
        let border_width = spin_row(
            &page,
            "Width:",
            0.5,
            10.0,
            0.5,
            applied.appearance.border.width,
        );
        let color = Rc::new(RefCell::new(applied.appearance.border.color));
        let color_row = GtkBox::new(Orientation::Horizontal, 8);
        let color_label = Label::new(Some("Colour:"));
        color_label.set_width_chars(14);
        color_label.set_xalign(0.0);
        let border_color = Rc::new(ColorButtonWidget::new(*color.borrow()));
        color_row.append(&color_label);
        color_row.append(border_color.widget());
        page.append(&color_row);

        for spin in [&radius, &border_width] {
            let weak = Rc::downgrade(self);
            spin.connect_value_changed(move |_| {
                if let Some(s) = weak.upgrade() {
                    s.changed();
                }
            });
        }
        {
            let weak = Rc::downgrade(self);
            border_enabled.connect_toggled(move |_| {
                if let Some(s) = weak.upgrade() {
                    s.changed();
                }
            });
        }
        {
            let weak = Rc::downgrade(self);
            let color = color.clone();
            border_color.set_on_change(move |c| {
                *color.borrow_mut() = c;
                if let Some(s) = weak.upgrade() {
                    s.changed();
                }
            });
        }
        {
            let weak = Rc::downgrade(self);
            copy_btn.connect_clicked(move |_| {
                let Some(s) = weak.upgrade() else { return };
                let data = s.build_data();
                let mut displayer_config = data.displayer_config.to_hashmap();
                super::super::grid_layout::filter_source_config_keys(&mut displayer_config);
                let style = PanelStyle {
                    background: data.appearance.background.clone(),
                    corner_radius: data.appearance.corner_radius,
                    border: data.appearance.border.clone(),
                    displayer_config,
                };
                if let Ok(mut cb) = CLIPBOARD.lock() {
                    cb.copy_panel_style(style);
                }
            });
        }
        {
            let weak = Rc::downgrade(self);
            paste_btn.connect_clicked(move |_| {
                let Some(s) = weak.upgrade() else { return };
                let style = CLIPBOARD.lock().ok().and_then(|cb| cb.paste_panel_style());
                let Some(style) = style else { return };
                s.loading.set(true);
                if let Some(a) = s.appearance.borrow().as_ref() {
                    a.radius.set_value(style.corner_radius);
                    a.border_enabled.set_active(style.border.enabled);
                    a.border_width.set_value(style.border.width);
                    *a.color.borrow_mut() = style.border.color;
                    a.border_color.set_color(style.border.color);
                }
                let bg = s.ensure_background_editor();
                bg.set_config(style.background.clone());
                let displayer_type = s.displayer_type();
                let pasted =
                    DisplayerConfig::extract_from_hashmap(&style.displayer_config, &displayer_type);
                s.loading.set(false);
                if let Some(cfg) = pasted {
                    s.load_display_config(cfg);
                }
                s.changed();
            });
        }
        *self.appearance.borrow_mut() = Some(AppearanceControls {
            radius,
            border_enabled,
            border_width,
            border_color,
            color,
        });
        page
    }

    /// Rebuild the breadcrumb row for `kind`.
    pub(super) fn update_breadcrumb(self: &Rc<Self>, kind: &NodeKind) {
        while let Some(child) = self.breadcrumb.first_child() {
            self.breadcrumb.remove(&child);
        }
        let path = self.breadcrumb_path(kind);
        let last = path.len().saturating_sub(1);
        for (i, crumb) in path.into_iter().enumerate() {
            if i > 0 {
                let sep = Label::new(Some("›"));
                sep.add_css_class("dim-label");
                self.breadcrumb.append(&sep);
            }
            let (title, _) = self.row_text(&crumb);
            let button = flat_button(&title);
            if i == last {
                button.set_sensitive(false);
            } else {
                let weak = Rc::downgrade(self);
                button.connect_clicked(move |_| {
                    if let Some(s) = weak.upgrade() {
                        s.select_node(&crumb);
                    }
                });
            }
            self.breadcrumb.append(&button);
        }
    }
}
