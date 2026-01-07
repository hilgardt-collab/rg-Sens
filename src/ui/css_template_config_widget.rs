//! CSS Template configuration widget
//!
//! Provides an interface for configuring CSS Template combo panels,
//! including template file selection and placeholder mappings.

use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, CheckButton, DropDown, Entry, FileDialog, Label,
    Notebook, Orientation, ScrolledWindow, SpinButton, StringList,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

use crate::core::FieldMetadata;
use crate::ui::css_template_display::{extract_placeholder_hints, CssTemplateDisplayConfig, PlaceholderMapping};
use crate::ui::widget_builder::{create_page_container, create_section_header};

/// CSS Template configuration widget
pub struct CssTemplateConfigWidget {
    container: GtkBox,
    config: Rc<RefCell<CssTemplateDisplayConfig>>,
    on_change: Rc<RefCell<Option<Box<dyn Fn()>>>>,
    source_summaries: Rc<RefCell<Vec<(String, String, usize, u32)>>>,
    placeholder_hints: Rc<RefCell<HashMap<u32, String>>>,
    // Template widgets
    html_path_entry: Entry,
    css_path_entry: Entry,
    hot_reload_check: CheckButton,
    // Mappings widgets
    mappings_container: GtkBox,
    // Display widgets
    animation_check: CheckButton,
    animation_speed_spin: SpinButton,
}

impl CssTemplateConfigWidget {
    pub fn new() -> Self {
        let container = GtkBox::new(Orientation::Vertical, 6);
        container.set_margin_start(12);
        container.set_margin_end(12);
        container.set_margin_top(12);
        container.set_margin_bottom(12);

        let config = Rc::new(RefCell::new(CssTemplateDisplayConfig::default()));
        let on_change: Rc<RefCell<Option<Box<dyn Fn()>>>> = Rc::new(RefCell::new(None));
        let source_summaries: Rc<RefCell<Vec<(String, String, usize, u32)>>> =
            Rc::new(RefCell::new(Vec::new()));
        let placeholder_hints: Rc<RefCell<HashMap<u32, String>>> =
            Rc::new(RefCell::new(HashMap::new()));

        // Create notebook for tabs
        let notebook = Notebook::new();
        notebook.set_vexpand(true);

        // Template Tab
        let (template_page, html_path_entry, css_path_entry, hot_reload_check) =
            Self::create_template_tab(config.clone(), on_change.clone());
        notebook.append_page(&template_page, Some(&Label::new(Some("Template"))));

        // Mappings Tab
        let (mappings_page, mappings_container) =
            Self::create_mappings_tab(config.clone(), on_change.clone(), source_summaries.clone(), placeholder_hints.clone());
        notebook.append_page(&mappings_page, Some(&Label::new(Some("Mappings"))));

        // Display Tab
        let (display_page, animation_check, animation_speed_spin) =
            Self::create_display_tab(config.clone(), on_change.clone());
        notebook.append_page(&display_page, Some(&Label::new(Some("Display"))));

        container.append(&notebook);

        Self {
            container,
            config,
            on_change,
            source_summaries,
            placeholder_hints,
            html_path_entry,
            css_path_entry,
            hot_reload_check,
            mappings_container,
            animation_check,
            animation_speed_spin,
        }
    }

    fn create_template_tab(
        config: Rc<RefCell<CssTemplateDisplayConfig>>,
        on_change: Rc<RefCell<Option<Box<dyn Fn()>>>>,
    ) -> (GtkBox, Entry, Entry, CheckButton) {
        let page = create_page_container();

        // HTML Template section
        let html_section = create_section_header("HTML Template");
        page.append(&html_section);

        let html_row = GtkBox::new(Orientation::Horizontal, 6);
        let html_entry = Entry::new();
        html_entry.set_hexpand(true);
        html_entry.set_placeholder_text(Some("Path to HTML template file..."));

        let html_browse = Button::with_label("Browse...");
        html_row.append(&html_entry);
        html_row.append(&html_browse);
        page.append(&html_row);

        // Connect HTML browse button
        let html_entry_clone = html_entry.clone();
        let config_for_html = config.clone();
        let on_change_for_html = on_change.clone();
        html_browse.connect_clicked(move |btn| {
            let entry = html_entry_clone.clone();
            let config = config_for_html.clone();
            let on_change = on_change_for_html.clone();

            if let Some(root) = btn.root() {
                if let Some(window) = root.downcast_ref::<gtk4::Window>() {
                    let window_clone = window.clone();

                    gtk4::glib::MainContext::default().spawn_local(async move {
                        let filters = gtk4::gio::ListStore::new::<gtk4::FileFilter>();

                        let html_filter = gtk4::FileFilter::new();
                        html_filter.set_name(Some("HTML files"));
                        html_filter.add_pattern("*.html");
                        html_filter.add_pattern("*.htm");
                        filters.append(&html_filter);

                        let all_filter = gtk4::FileFilter::new();
                        all_filter.set_name(Some("All files"));
                        all_filter.add_pattern("*");
                        filters.append(&all_filter);

                        let file_dialog = FileDialog::builder()
                            .title("Select HTML Template")
                            .modal(true)
                            .filters(&filters)
                            .build();

                        if let Ok(file) = file_dialog.open_future(Some(&window_clone)).await {
                            if let Some(path) = file.path() {
                                entry.set_text(&path.to_string_lossy());
                                config.borrow_mut().html_path = path;
                                if let Some(ref cb) = *on_change.borrow() {
                                    cb();
                                }
                            }
                        }
                    });
                }
            }
        });

        // Connect HTML entry changes
        let config_for_entry = config.clone();
        let on_change_for_entry = on_change.clone();
        html_entry.connect_changed(move |entry| {
            let text = entry.text();
            config_for_entry.borrow_mut().html_path = PathBuf::from(text.as_str());
            if let Some(ref cb) = *on_change_for_entry.borrow() {
                cb();
            }
        });

        // CSS File section (optional)
        let css_section = create_section_header("CSS File (Optional)");
        page.append(&css_section);

        let css_row = GtkBox::new(Orientation::Horizontal, 6);
        let css_entry = Entry::new();
        css_entry.set_hexpand(true);
        css_entry.set_placeholder_text(Some("Path to external CSS file..."));

        let css_browse = Button::with_label("Browse...");
        let css_clear = Button::with_label("Clear");
        css_row.append(&css_entry);
        css_row.append(&css_browse);
        css_row.append(&css_clear);
        page.append(&css_row);

        // Connect CSS browse button
        let css_entry_clone = css_entry.clone();
        let config_for_css = config.clone();
        let on_change_for_css = on_change.clone();
        css_browse.connect_clicked(move |btn| {
            let entry = css_entry_clone.clone();
            let config = config_for_css.clone();
            let on_change = on_change_for_css.clone();

            if let Some(root) = btn.root() {
                if let Some(window) = root.downcast_ref::<gtk4::Window>() {
                    let window_clone = window.clone();

                    gtk4::glib::MainContext::default().spawn_local(async move {
                        let filters = gtk4::gio::ListStore::new::<gtk4::FileFilter>();

                        let css_filter = gtk4::FileFilter::new();
                        css_filter.set_name(Some("CSS files"));
                        css_filter.add_pattern("*.css");
                        filters.append(&css_filter);

                        let all_filter = gtk4::FileFilter::new();
                        all_filter.set_name(Some("All files"));
                        all_filter.add_pattern("*");
                        filters.append(&all_filter);

                        let file_dialog = FileDialog::builder()
                            .title("Select CSS File")
                            .modal(true)
                            .filters(&filters)
                            .build();

                        if let Ok(file) = file_dialog.open_future(Some(&window_clone)).await {
                            if let Some(path) = file.path() {
                                entry.set_text(&path.to_string_lossy());
                                config.borrow_mut().css_path = Some(path);
                                if let Some(ref cb) = *on_change.borrow() {
                                    cb();
                                }
                            }
                        }
                    });
                }
            }
        });

        // Connect CSS clear button
        let css_entry_for_clear = css_entry.clone();
        let config_for_clear = config.clone();
        let on_change_for_clear = on_change.clone();
        css_clear.connect_clicked(move |_| {
            css_entry_for_clear.set_text("");
            config_for_clear.borrow_mut().css_path = None;
            if let Some(ref cb) = *on_change_for_clear.borrow() {
                cb();
            }
        });

        // Connect CSS entry changes
        let config_for_css_entry = config.clone();
        let on_change_for_css_entry = on_change.clone();
        css_entry.connect_changed(move |entry| {
            let text = entry.text();
            if text.is_empty() {
                config_for_css_entry.borrow_mut().css_path = None;
            } else {
                config_for_css_entry.borrow_mut().css_path = Some(PathBuf::from(text.as_str()));
            }
            if let Some(ref cb) = *on_change_for_css_entry.borrow() {
                cb();
            }
        });

        // Hot Reload section
        let options_section = create_section_header("Options");
        page.append(&options_section);

        let hot_reload_check = CheckButton::with_label("Hot Reload (auto-refresh when files change)");
        hot_reload_check.set_active(true);
        page.append(&hot_reload_check);

        let config_for_hot = config.clone();
        let on_change_for_hot = on_change.clone();
        hot_reload_check.connect_toggled(move |check| {
            config_for_hot.borrow_mut().hot_reload = check.is_active();
            if let Some(ref cb) = *on_change_for_hot.borrow() {
                cb();
            }
        });

        // Help text
        let help_label = Label::new(Some(
            "Use {{0}}, {{1}}, {{2}}, etc. as placeholders in your HTML template.\n\
             Map these placeholders to data sources in the Mappings tab.",
        ));
        help_label.set_wrap(true);
        help_label.set_xalign(0.0);
        help_label.add_css_class("dim-label");
        help_label.set_margin_top(12);
        page.append(&help_label);

        (page, html_entry, css_entry, hot_reload_check)
    }

    fn create_mappings_tab(
        config: Rc<RefCell<CssTemplateDisplayConfig>>,
        on_change: Rc<RefCell<Option<Box<dyn Fn()>>>>,
        source_summaries: Rc<RefCell<Vec<(String, String, usize, u32)>>>,
        placeholder_hints: Rc<RefCell<HashMap<u32, String>>>,
    ) -> (GtkBox, GtkBox) {
        let page = create_page_container();

        // Header
        let header = create_section_header("Placeholder Mappings");
        page.append(&header);

        let help = Label::new(Some(
            "Map each placeholder ({{0}}, {{1}}, etc.) to a data source.\n\
             Hints from the template are shown in italics.",
        ));
        help.set_xalign(0.0);
        help.add_css_class("dim-label");
        page.append(&help);

        // Scrolled container for mappings
        let scroll = ScrolledWindow::new();
        scroll.set_vexpand(true);
        scroll.set_min_content_height(200);

        let mappings_container = GtkBox::new(Orientation::Vertical, 6);
        mappings_container.set_margin_start(6);
        mappings_container.set_margin_end(6);
        mappings_container.set_margin_top(6);
        mappings_container.set_margin_bottom(6);

        scroll.set_child(Some(&mappings_container));
        page.append(&scroll);

        // Add mapping button
        let add_btn = Button::with_label("Add Mapping");
        let mappings_container_clone = mappings_container.clone();
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let source_summaries_clone = source_summaries.clone();
        let placeholder_hints_clone = placeholder_hints.clone();
        add_btn.connect_clicked(move |_| {
            let cfg = config_clone.borrow();
            let next_idx = cfg.mappings.len() as u32;
            drop(cfg);

            let new_mapping = PlaceholderMapping {
                index: next_idx,
                slot_prefix: String::new(),
                field: "value".to_string(),
                format: None,
            };

            config_clone.borrow_mut().mappings.push(new_mapping.clone());

            Self::add_mapping_row(
                &mappings_container_clone,
                next_idx as usize,
                &new_mapping,
                config_clone.clone(),
                on_change_clone.clone(),
                source_summaries_clone.clone(),
                placeholder_hints_clone.clone(),
            );

            if let Some(ref cb) = *on_change_clone.borrow() {
                cb();
            }
        });
        page.append(&add_btn);

        (page, mappings_container)
    }

    fn add_mapping_row(
        container: &GtkBox,
        row_idx: usize,
        mapping: &PlaceholderMapping,
        config: Rc<RefCell<CssTemplateDisplayConfig>>,
        on_change: Rc<RefCell<Option<Box<dyn Fn()>>>>,
        source_summaries: Rc<RefCell<Vec<(String, String, usize, u32)>>>,
        placeholder_hints: Rc<RefCell<HashMap<u32, String>>>,
    ) {
        // Create outer vertical box to hold the main row and hint
        let outer_box = GtkBox::new(Orientation::Vertical, 2);
        outer_box.set_margin_bottom(6);

        let row = GtkBox::new(Orientation::Horizontal, 6);

        // Index label
        let idx_label = Label::new(Some(&format!("{{{{{}}}}}", mapping.index)));
        idx_label.set_width_chars(6);
        row.append(&idx_label);

        // Add hint label if available
        let hints = placeholder_hints.borrow();
        if let Some(hint) = hints.get(&mapping.index) {
            let hint_label = Label::new(Some(hint));
            hint_label.set_xalign(0.0);
            hint_label.add_css_class("dim-label");
            hint_label.set_markup(&format!("<i>{}</i>", glib::markup_escape_text(hint)));
            hint_label.set_hexpand(true);
            hint_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
            hint_label.set_tooltip_text(Some(hint));
            outer_box.append(&hint_label);
        }
        drop(hints);

        // Source dropdown
        let summaries = source_summaries.borrow();
        let source_list = StringList::new(&[] as &[&str]);
        source_list.append("(none)");
        for (prefix, label, _, _) in summaries.iter() {
            source_list.append(&format!("{}: {}", prefix, label));
        }
        drop(summaries);

        let source_dropdown = DropDown::new(Some(source_list), None::<gtk4::Expression>);
        source_dropdown.set_hexpand(true);

        // Find current selection
        if !mapping.slot_prefix.is_empty() {
            let summaries = source_summaries.borrow();
            for (i, (prefix, _, _, _)) in summaries.iter().enumerate() {
                if prefix == &mapping.slot_prefix {
                    source_dropdown.set_selected((i + 1) as u32);
                    break;
                }
            }
        }

        let config_for_source = config.clone();
        let on_change_for_source = on_change.clone();
        let source_summaries_for_cb = source_summaries.clone();
        source_dropdown.connect_selected_notify(move |dropdown| {
            let selected = dropdown.selected();
            let mut cfg = config_for_source.borrow_mut();
            if let Some(mapping) = cfg.mappings.get_mut(row_idx) {
                if selected == 0 {
                    mapping.slot_prefix = String::new();
                } else {
                    let summaries = source_summaries_for_cb.borrow();
                    if let Some((prefix, _, _, _)) = summaries.get((selected - 1) as usize) {
                        mapping.slot_prefix = prefix.clone();
                    }
                }
            }
            drop(cfg);

            if let Some(ref cb) = *on_change_for_source.borrow() {
                cb();
            }
        });

        row.append(&source_dropdown);

        // Field dropdown
        let field_list = StringList::new(&["value", "caption", "unit", "percent", "min", "max"]);
        let field_dropdown = DropDown::new(Some(field_list), None::<gtk4::Expression>);

        // Find current field selection
        let field_idx = match mapping.field.as_str() {
            "value" => 0,
            "caption" => 1,
            "unit" => 2,
            "percent" => 3,
            "min" => 4,
            "max" => 5,
            _ => 0,
        };
        field_dropdown.set_selected(field_idx);

        let config_for_field = config.clone();
        let on_change_for_field = on_change.clone();
        field_dropdown.connect_selected_notify(move |dropdown| {
            let selected = dropdown.selected();
            let field = match selected {
                0 => "value",
                1 => "caption",
                2 => "unit",
                3 => "percent",
                4 => "min",
                5 => "max",
                _ => "value",
            };

            let mut cfg = config_for_field.borrow_mut();
            if let Some(mapping) = cfg.mappings.get_mut(row_idx) {
                mapping.field = field.to_string();
            }
            drop(cfg);

            if let Some(ref cb) = *on_change_for_field.borrow() {
                cb();
            }
        });

        row.append(&field_dropdown);

        // Format entry
        let format_entry = Entry::new();
        format_entry.set_placeholder_text(Some("Format (e.g., {:.1}%)"));
        format_entry.set_width_chars(12);
        if let Some(ref fmt) = mapping.format {
            format_entry.set_text(fmt);
        }

        let config_for_format = config.clone();
        let on_change_for_format = on_change.clone();
        format_entry.connect_changed(move |entry| {
            let text = entry.text();
            let mut cfg = config_for_format.borrow_mut();
            if let Some(mapping) = cfg.mappings.get_mut(row_idx) {
                if text.is_empty() {
                    mapping.format = None;
                } else {
                    mapping.format = Some(text.to_string());
                }
            }
            drop(cfg);

            if let Some(ref cb) = *on_change_for_format.borrow() {
                cb();
            }
        });

        row.append(&format_entry);

        // Remove button
        let remove_btn = Button::with_label("X");
        let container_for_remove = container.clone();
        let outer_box_for_remove = outer_box.clone();
        let config_for_remove = config.clone();
        let on_change_for_remove = on_change.clone();
        remove_btn.connect_clicked(move |_| {
            container_for_remove.remove(&outer_box_for_remove);
            let mut cfg = config_for_remove.borrow_mut();
            if row_idx < cfg.mappings.len() {
                cfg.mappings.remove(row_idx);
                // Re-index remaining mappings
                for (i, m) in cfg.mappings.iter_mut().enumerate() {
                    m.index = i as u32;
                }
            }
            drop(cfg);

            if let Some(ref cb) = *on_change_for_remove.borrow() {
                cb();
            }
        });

        row.append(&remove_btn);

        // Add row to outer box and outer box to container
        outer_box.append(&row);
        container.append(&outer_box);
    }

    fn create_display_tab(
        config: Rc<RefCell<CssTemplateDisplayConfig>>,
        on_change: Rc<RefCell<Option<Box<dyn Fn()>>>>,
    ) -> (GtkBox, CheckButton, SpinButton) {
        let page = create_page_container();

        // Animation section
        let anim_section = create_section_header("Animation");
        page.append(&anim_section);

        let anim_check = CheckButton::with_label("Enable CSS animations");
        anim_check.set_active(config.borrow().animation_enabled);
        page.append(&anim_check);

        let speed_row = GtkBox::new(Orientation::Horizontal, 6);
        let speed_label = Label::new(Some("Animation speed:"));
        let speed_spin = SpinButton::with_range(0.1, 10.0, 0.1);
        speed_spin.set_value(config.borrow().animation_speed);
        speed_row.append(&speed_label);
        speed_row.append(&speed_spin);
        page.append(&speed_row);

        // Connect animation controls
        let config_for_anim = config.clone();
        let on_change_for_anim = on_change.clone();
        anim_check.connect_toggled(move |check| {
            config_for_anim.borrow_mut().animation_enabled = check.is_active();
            if let Some(ref cb) = *on_change_for_anim.borrow() {
                cb();
            }
        });

        let config_for_speed = config.clone();
        let on_change_for_speed = on_change.clone();
        speed_spin.connect_value_changed(move |spin| {
            config_for_speed.borrow_mut().animation_speed = spin.value();
            if let Some(ref cb) = *on_change_for_speed.borrow() {
                cb();
            }
        });

        (page, anim_check, speed_spin)
    }

    /// Get the container widget
    pub fn widget(&self) -> &GtkBox {
        &self.container
    }

    /// Set callback for config changes
    pub fn set_on_change<F: Fn() + 'static>(&self, callback: F) {
        *self.on_change.borrow_mut() = Some(Box::new(callback));
    }

    /// Get current configuration
    pub fn get_config(&self) -> CssTemplateDisplayConfig {
        self.config.borrow().clone()
    }

    /// Set configuration
    pub fn set_config(&self, config: &CssTemplateDisplayConfig) {
        *self.config.borrow_mut() = config.clone();

        // Update widgets
        self.html_path_entry
            .set_text(&config.html_path.to_string_lossy());
        self.css_path_entry.set_text(
            config
                .css_path
                .as_ref()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default()
                .as_str(),
        );
        self.hot_reload_check.set_active(config.hot_reload);
        self.animation_check.set_active(config.animation_enabled);
        self.animation_speed_spin.set_value(config.animation_speed);

        // Rebuild mappings (also updates hints from HTML)
        self.rebuild_mappings();
    }

    /// Update placeholder hints from HTML file
    fn update_hints_from_html(&self, html_path: &std::path::Path) {
        let hints = if !html_path.as_os_str().is_empty() && html_path.exists() {
            if let Ok(html) = std::fs::read_to_string(html_path) {
                extract_placeholder_hints(&html)
            } else {
                HashMap::new()
            }
        } else {
            HashMap::new()
        };
        *self.placeholder_hints.borrow_mut() = hints;
    }

    /// Set available sources
    pub fn set_available_sources(&self, sources: Vec<(String, String, Vec<FieldMetadata>)>) {
        // Convert to summaries format: (prefix, label, slot_index, group_index)
        let mut summaries = Vec::new();
        for (group_idx, (_source_id, source_name, _fields)) in sources.iter().enumerate() {
            // Create summaries for typical combo source prefixes
            for slot_idx in 1..=10 {
                let prefix = format!("group{}_{}", group_idx + 1, slot_idx);
                let label = format!("{} Slot {}", source_name, slot_idx);
                summaries.push((prefix, label, slot_idx, group_idx as u32));
            }
        }
        *self.source_summaries.borrow_mut() = summaries;

        // Rebuild mappings UI
        self.rebuild_mappings();
    }

    /// Set source summaries directly (from combo widget)
    pub fn set_source_summaries(&self, summaries: Vec<(String, String, usize, u32)>) {
        *self.source_summaries.borrow_mut() = summaries;
        // Rebuild mappings UI to update dropdowns
        self.rebuild_mappings();
    }

    /// Set placeholder hints from template
    pub fn set_placeholder_hints(&self, hints: HashMap<u32, String>) {
        *self.placeholder_hints.borrow_mut() = hints;
        // Rebuild mappings UI to show hints
        self.rebuild_mappings();
    }

    /// Rebuild the mappings UI
    fn rebuild_mappings(&self) {
        // Update hints from the current HTML path
        {
            let config = self.config.borrow();
            self.update_hints_from_html(&config.html_path);
        }

        // Clear existing rows
        while let Some(child) = self.mappings_container.first_child() {
            self.mappings_container.remove(&child);
        }

        // Add rows for each mapping
        let config = self.config.borrow();
        for (idx, mapping) in config.mappings.iter().enumerate() {
            Self::add_mapping_row(
                &self.mappings_container,
                idx,
                mapping,
                self.config.clone(),
                self.on_change.clone(),
                self.source_summaries.clone(),
                self.placeholder_hints.clone(),
            );
        }
    }
}

impl Default for CssTemplateConfigWidget {
    fn default() -> Self {
        Self::new()
    }
}
