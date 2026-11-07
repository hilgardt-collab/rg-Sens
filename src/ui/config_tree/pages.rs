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
             its source settings and display sections as child nodes of its own.",
        ));
        page
    }

    fn build_group_page(self: &Rc<Self>, group: usize) -> GtkBox {
        let page = page_box();
        page.append(&heading(&format!("Group {}", group + 1)));
        let (count, weight, total) = self
            .combo_source
            .borrow()
            .as_ref()
            .map(|cs| {
                let g = cs.groups.get(group);
                (
                    g.map(|g| g.item_count).unwrap_or(1),
                    g.map(|g| g.size_weight).unwrap_or(1.0),
                    cs.groups.len(),
                )
            })
            .unwrap_or((1, 1.0, 1));
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
        let weight_spin = spin_row(&page, "Size weight:", 0.1, 10.0, 0.1, weight);
        weight_spin.set_digits(1);
        weight_spin.set_tooltip_text(Some(
            "Share of the content area this group gets, relative to the other groups",
        ));
        {
            let weak = Rc::downgrade(self);
            weight_spin.connect_value_changed(move |spin| {
                if let Some(s) = weak.upgrade() {
                    if !s.loading.get() {
                        s.set_group_weight(group, spin.value());
                    }
                }
            });
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
        page.append(&hint(
            "Item orientation within each group is under Display › Layout.",
        ));
        page
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
            None => page.append(&hint(
                "This displayer has no per-item display settings; its mappings are under Display.",
            )),
        }
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
