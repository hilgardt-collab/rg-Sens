//! CSS Template configuration widget
//!
//! Provides an interface for configuring CSS Template combo panels,
//! including template file selection and placeholder mappings.

use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, CheckButton, DropDown, Entry, FileDialog, Label, Notebook, Orientation,
    ScrolledWindow, SpinButton, StringList,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

use crate::core::FieldMetadata;
use crate::ui::css_template_display::{
    detect_placeholders, extract_placeholder_defaults, extract_placeholder_hints,
    CssTemplateDisplayConfig, PlaceholderDefault, PlaceholderMapping,
};
use crate::ui::widget_builder::{create_page_container, create_section_header};

/// CSS Template configuration widget
pub struct CssTemplateConfigWidget {
    container: GtkBox,
    config: Rc<RefCell<CssTemplateDisplayConfig>>,
    on_change: Rc<RefCell<Option<Box<dyn Fn()>>>>,
    source_summaries: Rc<RefCell<Vec<(String, String, usize, u32)>>>,
    placeholder_hints: Rc<RefCell<HashMap<u32, String>>>,
    placeholder_defaults: Rc<RefCell<HashMap<u32, PlaceholderDefault>>>,
    // Template widgets
    html_path_entry: Entry,
    css_path_entry: Entry,
    hot_reload_check: CheckButton,
    scan_btn: Button,
    // Mappings widgets
    mappings_container: GtkBox,
    auto_config_btn: Button,
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
        let placeholder_defaults: Rc<RefCell<HashMap<u32, PlaceholderDefault>>> =
            Rc::new(RefCell::new(HashMap::new()));

        // Create notebook for tabs
        let notebook = Notebook::new();
        notebook.set_vexpand(true);

        // Template Tab
        let (template_page, html_path_entry, css_path_entry, hot_reload_check, scan_btn) =
            Self::create_template_tab(config.clone(), on_change.clone());
        notebook.append_page(&template_page, Some(&Label::new(Some("Template"))));

        // Mappings Tab
        let (mappings_page, mappings_container, auto_config_btn) = Self::create_mappings_tab(
            config.clone(),
            on_change.clone(),
            source_summaries.clone(),
            placeholder_hints.clone(),
            placeholder_defaults.clone(),
        );
        notebook.append_page(&mappings_page, Some(&Label::new(Some("Mappings"))));

        // Display Tab
        let (display_page, animation_check, animation_speed_spin) =
            Self::create_display_tab(config.clone(), on_change.clone());
        notebook.append_page(&display_page, Some(&Label::new(Some("Display"))));

        container.append(&notebook);

        let widget = Self {
            container,
            config,
            on_change,
            source_summaries,
            placeholder_hints,
            placeholder_defaults,
            html_path_entry,
            css_path_entry,
            hot_reload_check,
            scan_btn,
            mappings_container,
            auto_config_btn,
            animation_check,
            animation_speed_spin,
        };

        // Connect button handlers
        widget.connect_auto_config_handler();
        widget.connect_scan_handler();
        widget.connect_html_path_handler();

        // Disable buttons by default until a template is loaded
        widget.auto_config_btn.set_sensitive(false);
        widget.scan_btn.set_sensitive(false);

        widget
    }

    /// Connect handler to update button states when HTML path changes
    fn connect_html_path_handler(&self) {
        let scan_btn = self.scan_btn.clone();
        let auto_config_btn = self.auto_config_btn.clone();
        let placeholder_hints = self.placeholder_hints.clone();
        let placeholder_defaults = self.placeholder_defaults.clone();

        self.html_path_entry.connect_changed(move |entry| {
            let path = PathBuf::from(entry.text().as_str());

            if !path.as_os_str().is_empty() && path.exists() {
                if let Ok(html) = std::fs::read_to_string(&path) {
                    // Extract hints and defaults
                    let hints = crate::ui::css_template_display::extract_placeholder_hints(&html);
                    *placeholder_hints.borrow_mut() = hints;

                    let defaults =
                        crate::ui::css_template_display::extract_placeholder_defaults(&html);
                    let has_defaults = defaults.values().any(|d| !d.source.is_empty());
                    *placeholder_defaults.borrow_mut() = defaults;

                    // Enable buttons
                    scan_btn.set_sensitive(true);
                    auto_config_btn.set_sensitive(has_defaults);
                    return;
                }
            }

            // Disable buttons if path is invalid
            scan_btn.set_sensitive(false);
            auto_config_btn.set_sensitive(false);
        });
    }

    fn create_template_tab(
        config: Rc<RefCell<CssTemplateDisplayConfig>>,
        on_change: Rc<RefCell<Option<Box<dyn Fn()>>>>,
    ) -> (GtkBox, Entry, Entry, CheckButton, Button) {
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

        let hot_reload_check =
            CheckButton::with_label("Hot Reload (auto-refresh when files change)");
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

        // Scan Placeholders section
        let scan_section = create_section_header("Placeholder Scanning");
        page.append(&scan_section);

        let scan_row = GtkBox::new(Orientation::Horizontal, 6);
        let scan_btn = Button::with_label("Scan Placeholders");
        scan_btn.set_tooltip_text(Some(
            "Scan the HTML template for placeholders and create mappings.\n\
             This will replace any existing mappings.",
        ));
        scan_btn.set_sensitive(false); // Enabled when HTML path is valid
        scan_row.append(&scan_btn);

        let scan_help = Label::new(Some("Reads template and creates mapping entries"));
        scan_help.add_css_class("dim-label");
        scan_row.append(&scan_help);
        page.append(&scan_row);

        // Help text
        let help_label = Label::new(Some(
            "Use {{0}}, {{1}}, {{2}}, etc. as placeholders in your HTML template.\n\
             Click \"Scan Placeholders\" to detect them, then use the Mappings tab\n\
             to assign data sources. \"Auto-configure\" will try to match sources.",
        ));
        help_label.set_wrap(true);
        help_label.set_xalign(0.0);
        help_label.add_css_class("dim-label");
        help_label.set_margin_top(12);
        page.append(&help_label);

        (page, html_entry, css_entry, hot_reload_check, scan_btn)
    }

    fn create_mappings_tab(
        config: Rc<RefCell<CssTemplateDisplayConfig>>,
        on_change: Rc<RefCell<Option<Box<dyn Fn()>>>>,
        source_summaries: Rc<RefCell<Vec<(String, String, usize, u32)>>>,
        placeholder_hints: Rc<RefCell<HashMap<u32, String>>>,
        _placeholder_defaults: Rc<RefCell<HashMap<u32, PlaceholderDefault>>>,
    ) -> (GtkBox, GtkBox, Button) {
        let page = create_page_container();

        // Header
        let header = create_section_header("Placeholder Mappings");
        page.append(&header);

        let help = Label::new(Some(
            "Map each placeholder ({{0}}, {{1}}, etc.) to a data source.\n\
             Hints from the template are shown in italics.\n\
             Use \"Auto-configure\" to apply template defaults.",
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

        // Button row
        let btn_row = GtkBox::new(Orientation::Horizontal, 6);

        // Auto-configure button
        let auto_config_btn = Button::with_label("Auto-configure");
        auto_config_btn.set_tooltip_text(Some(
            "Automatically configure mappings based on template defaults",
        ));
        // Note: The actual click handler is connected later via connect_auto_config
        btn_row.append(&auto_config_btn);

        // Add source group button (4 fields: caption, value, unit, max)
        let add_group_btn = Button::with_label("Add Source Group");
        add_group_btn.set_tooltip_text(Some(
            "Add a group of 4 mappings (caption, value, unit, max) for one source",
        ));
        let mappings_container_for_group = mappings_container.clone();
        let config_for_group = config.clone();
        let on_change_for_group = on_change.clone();
        let source_summaries_for_group = source_summaries.clone();
        let placeholder_hints_for_group = placeholder_hints.clone();
        add_group_btn.connect_clicked(move |_| {
            let cfg = config_for_group.borrow();
            let next_idx = cfg.mappings.len() as u32;
            drop(cfg);

            // Create 4 mappings for the group
            let fields = ["caption", "value", "unit", "max"];
            let mut new_mappings = Vec::new();
            for (i, field) in fields.iter().enumerate() {
                new_mappings.push(PlaceholderMapping {
                    index: next_idx + i as u32,
                    slot_prefix: String::new(),
                    field: field.to_string(),
                    format: if *field == "value" {
                        Some("{:.1}".to_string())
                    } else {
                        None
                    },
                });
            }

            // Add to config
            {
                let mut cfg = config_for_group.borrow_mut();
                for mapping in &new_mappings {
                    cfg.mappings.push(mapping.clone());
                }
            }

            // Add the grouped UI
            Self::add_source_group_row(
                &mappings_container_for_group,
                next_idx as usize,
                &new_mappings,
                config_for_group.clone(),
                on_change_for_group.clone(),
                source_summaries_for_group.clone(),
                placeholder_hints_for_group.clone(),
            );

            if let Some(ref cb) = *on_change_for_group.borrow() {
                cb();
            }
        });
        btn_row.append(&add_group_btn);

        // Add single mapping button
        let add_btn = Button::with_label("Add Single");
        add_btn.set_tooltip_text(Some("Add a single placeholder mapping"));
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
        btn_row.append(&add_btn);

        page.append(&btn_row);

        (page, mappings_container, auto_config_btn)
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

    /// Add a grouped source mapping row (4 fields: caption, value, unit, max)
    ///
    /// This creates a visual group with a single source dropdown that controls
    /// all 4 field mappings, making it easier to configure common source patterns.
    fn add_source_group_row(
        container: &GtkBox,
        start_idx: usize,
        mappings: &[PlaceholderMapping],
        config: Rc<RefCell<CssTemplateDisplayConfig>>,
        on_change: Rc<RefCell<Option<Box<dyn Fn()>>>>,
        source_summaries: Rc<RefCell<Vec<(String, String, usize, u32)>>>,
        placeholder_hints: Rc<RefCell<HashMap<u32, String>>>,
    ) {
        // Create a frame to visually group the 4 fields
        let frame = gtk4::Frame::new(None);
        frame.set_margin_bottom(8);
        frame.add_css_class("card");

        let group_box = GtkBox::new(Orientation::Vertical, 4);
        group_box.set_margin_start(8);
        group_box.set_margin_end(8);
        group_box.set_margin_top(8);
        group_box.set_margin_bottom(8);

        // Header row with source dropdown and remove button
        let header_row = GtkBox::new(Orientation::Horizontal, 6);

        // Placeholder indices label
        let indices: Vec<String> = mappings
            .iter()
            .map(|m| format!("{{{{{}}}}}", m.index))
            .collect();
        let indices_label = Label::new(Some(&indices.join(" ")));
        indices_label.add_css_class("dim-label");
        indices_label.set_width_chars(20);
        header_row.append(&indices_label);

        // Source dropdown (shared for all 4 fields)
        let summaries = source_summaries.borrow();
        let source_list = StringList::new(&[] as &[&str]);
        source_list.append("(none)");
        for (prefix, label, _, _) in summaries.iter() {
            source_list.append(&format!("{}: {}", prefix, label));
        }
        drop(summaries);

        let source_dropdown = DropDown::new(Some(source_list), None::<gtk4::Expression>);
        source_dropdown.set_hexpand(true);

        // Find current selection (from first mapping)
        if !mappings.is_empty() && !mappings[0].slot_prefix.is_empty() {
            let summaries = source_summaries.borrow();
            for (i, (prefix, _, _, _)) in summaries.iter().enumerate() {
                if prefix == &mappings[0].slot_prefix {
                    source_dropdown.set_selected((i + 1) as u32);
                    break;
                }
            }
        }

        // Connect source dropdown to update all 4 mappings
        let config_for_source = config.clone();
        let on_change_for_source = on_change.clone();
        let source_summaries_for_cb = source_summaries.clone();
        let mapping_indices: Vec<usize> = (start_idx..start_idx + mappings.len()).collect();
        source_dropdown.connect_selected_notify(move |dropdown| {
            let selected = dropdown.selected();
            let mut cfg = config_for_source.borrow_mut();

            let new_prefix = if selected == 0 {
                String::new()
            } else {
                let summaries = source_summaries_for_cb.borrow();
                summaries
                    .get((selected - 1) as usize)
                    .map(|(prefix, _, _, _)| prefix.clone())
                    .unwrap_or_default()
            };

            // Update all mappings in this group
            for &idx in &mapping_indices {
                if let Some(mapping) = cfg.mappings.get_mut(idx) {
                    mapping.slot_prefix = new_prefix.clone();
                }
            }
            drop(cfg);

            if let Some(ref cb) = *on_change_for_source.borrow() {
                cb();
            }
        });

        header_row.append(&source_dropdown);

        // Remove group button
        let remove_btn = Button::with_label("Remove");
        let container_for_remove = container.clone();
        let frame_for_remove = frame.clone();
        let config_for_remove = config.clone();
        let on_change_for_remove = on_change.clone();
        let num_mappings = mappings.len();
        remove_btn.connect_clicked(move |_| {
            container_for_remove.remove(&frame_for_remove);
            let mut cfg = config_for_remove.borrow_mut();

            // Remove all mappings in this group (in reverse order to maintain indices)
            for i in (0..num_mappings).rev() {
                let idx = start_idx + i;
                if idx < cfg.mappings.len() {
                    cfg.mappings.remove(idx);
                }
            }

            // Re-index remaining mappings
            for (i, m) in cfg.mappings.iter_mut().enumerate() {
                m.index = i as u32;
            }
            drop(cfg);

            if let Some(ref cb) = *on_change_for_remove.borrow() {
                cb();
            }
        });
        header_row.append(&remove_btn);

        group_box.append(&header_row);

        // Add a row for each field in the group
        let hints = placeholder_hints.borrow();
        for (i, mapping) in mappings.iter().enumerate() {
            let field_row = GtkBox::new(Orientation::Horizontal, 6);

            // Field label
            let field_label = Label::new(Some(&format!(
                "{{{{{}}}}} {}",
                mapping.index, mapping.field
            )));
            field_label.set_width_chars(16);
            field_label.set_xalign(0.0);
            field_row.append(&field_label);

            // Hint (if available)
            if let Some(hint) = hints.get(&mapping.index) {
                let hint_label = Label::new(None);
                hint_label.set_markup(&format!("<i>{}</i>", glib::markup_escape_text(hint)));
                hint_label.set_xalign(0.0);
                hint_label.set_hexpand(true);
                hint_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
                hint_label.add_css_class("dim-label");
                hint_label.set_tooltip_text(Some(hint));
                field_row.append(&hint_label);
            } else {
                let spacer = GtkBox::new(Orientation::Horizontal, 0);
                spacer.set_hexpand(true);
                field_row.append(&spacer);
            }

            // Format entry (mainly useful for value field)
            let format_entry = Entry::new();
            format_entry.set_placeholder_text(Some("Format"));
            format_entry.set_width_chars(10);
            if let Some(ref fmt) = mapping.format {
                format_entry.set_text(fmt);
            }

            let config_for_format = config.clone();
            let on_change_for_format = on_change.clone();
            let mapping_idx = start_idx + i;
            format_entry.connect_changed(move |entry| {
                let text = entry.text();
                let mut cfg = config_for_format.borrow_mut();
                if let Some(mapping) = cfg.mappings.get_mut(mapping_idx) {
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
            field_row.append(&format_entry);

            group_box.append(&field_row);
        }
        drop(hints);

        frame.set_child(Some(&group_box));
        container.append(&frame);
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

    /// Update placeholder hints and defaults from HTML file
    fn update_hints_from_html(&self, html_path: &std::path::Path) {
        if !html_path.as_os_str().is_empty() && html_path.exists() {
            if let Ok(html) = std::fs::read_to_string(html_path) {
                // Extract hints (which also tries config format first)
                let hints = extract_placeholder_hints(&html);
                *self.placeholder_hints.borrow_mut() = hints;

                // Extract defaults for auto-configure
                let defaults = extract_placeholder_defaults(&html);
                let has_defaults = defaults.values().any(|d| !d.source.is_empty());
                *self.placeholder_defaults.borrow_mut() = defaults;

                // Enable scan button when HTML file is valid
                self.scan_btn.set_sensitive(true);

                // Enable/disable auto-configure button based on whether template has defaults
                self.auto_config_btn.set_sensitive(has_defaults);
            } else {
                *self.placeholder_hints.borrow_mut() = HashMap::new();
                *self.placeholder_defaults.borrow_mut() = HashMap::new();
                self.scan_btn.set_sensitive(false);
                self.auto_config_btn.set_sensitive(false);
            }
        } else {
            *self.placeholder_hints.borrow_mut() = HashMap::new();
            *self.placeholder_defaults.borrow_mut() = HashMap::new();
            self.scan_btn.set_sensitive(false);
            self.auto_config_btn.set_sensitive(false);
        }
    }

    /// Apply auto-configuration based on template defaults and available sources
    ///
    /// This matches placeholder defaults to available source slots and creates/updates mappings.
    /// Existing mappings with a configured source (non-empty slot_prefix) are preserved.
    /// Groups (4 consecutive placeholders with same source/instance) are processed together,
    /// and if any member has an existing configured source, it's used for all members.
    pub fn apply_auto_config(&self) {
        let defaults = self.placeholder_defaults.borrow();
        let summaries = self.source_summaries.borrow();

        if defaults.is_empty() || summaries.is_empty() {
            log::warn!("Cannot auto-configure: no defaults or no sources available");
            return;
        }

        // Build a map of existing mappings by index for quick lookup
        let existing_mappings: HashMap<u32, PlaceholderMapping> = self
            .config
            .borrow()
            .mappings
            .iter()
            .map(|m| (m.index, m.clone()))
            .collect();

        // Group source summaries by source type for easier lookup
        let mut source_slots: HashMap<String, Vec<(String, usize)>> = HashMap::new();

        for (prefix, label, slot_idx, _group_idx) in summaries.iter() {
            let source_type = label.split_whitespace().next().unwrap_or("").to_lowercase();
            source_slots
                .entry(source_type)
                .or_default()
                .push((prefix.clone(), *slot_idx));
        }

        // Track which slots we've used for each source type
        let mut used_instances: HashMap<String, usize> = HashMap::new();

        // Build new mappings
        let mut new_mappings: Vec<PlaceholderMapping> = Vec::new();

        // Sort defaults by index for predictable order
        let mut sorted_defaults: Vec<_> = defaults.iter().collect();
        sorted_defaults.sort_by_key(|(idx, _)| *idx);

        // Process defaults, detecting groups (4 consecutive with same source/instance)
        let mut i = 0;
        while i < sorted_defaults.len() {
            let (idx, default) = sorted_defaults[i];

            // Check if this is the start of a group (4 consecutive with same source/instance)
            let is_group = if i + 3 < sorted_defaults.len() && !default.source.is_empty() {
                let group_indices: Vec<u32> = (0..4).map(|j| *sorted_defaults[i + j].0).collect();
                let consecutive = group_indices.windows(2).all(|w| w[1] == w[0] + 1);

                if consecutive {
                    let group_defaults: Vec<&PlaceholderDefault> =
                        (0..4).map(|j| sorted_defaults[i + j].1).collect();

                    // Check same source and instance
                    let same_source_instance = group_defaults
                        .iter()
                        .all(|d| d.source == default.source && d.instance == default.instance);

                    // Check fields are caption/value/unit/max
                    let fields: Vec<&str> =
                        group_defaults.iter().map(|d| d.field.as_str()).collect();
                    let standard_fields = fields == ["caption", "value", "unit", "max"];

                    same_source_instance && standard_fields
                } else {
                    false
                }
            } else {
                false
            };

            if is_group {
                // Process as a group - check if ANY member has an existing configured source
                let group_indices: Vec<u32> = (0..4).map(|j| *sorted_defaults[i + j].0).collect();
                let existing_prefix = group_indices
                    .iter()
                    .filter_map(|idx| existing_mappings.get(idx))
                    .find(|m| !m.slot_prefix.is_empty())
                    .map(|m| m.slot_prefix.clone());

                let slot_prefix = if let Some(prefix) = existing_prefix {
                    // Use the existing configured prefix for all group members
                    prefix
                } else {
                    // Auto-configure: find a slot for this source type
                    let source_type = default.source.to_lowercase();
                    if let Some(slots) = source_slots.get(&source_type) {
                        let instance = default.instance as usize;
                        let used = used_instances.entry(source_type.clone()).or_insert(0);

                        let slot_to_use = if instance > 0 {
                            slots.iter().find(|(_, slot_idx)| *slot_idx == instance)
                        } else {
                            let target = *used;
                            slots.get(target)
                        };

                        if let Some((prefix, _)) = slot_to_use {
                            if instance == 0 {
                                *used += 1;
                            }
                            prefix.clone()
                        } else {
                            String::new()
                        }
                    } else {
                        String::new()
                    }
                };

                // Add all 4 group members with the same prefix
                for j in 0..4 {
                    let (group_idx, group_default) = sorted_defaults[i + j];
                    new_mappings.push(PlaceholderMapping {
                        index: *group_idx,
                        slot_prefix: slot_prefix.clone(),
                        field: group_default.field.clone(),
                        format: group_default.format.clone(),
                    });
                }
                i += 4;
            } else {
                // Process as individual mapping
                // Check if there's an existing mapping with a configured source
                if let Some(existing) = existing_mappings.get(idx) {
                    if !existing.slot_prefix.is_empty() {
                        // Preserve the existing mapping
                        new_mappings.push(existing.clone());
                        i += 1;
                        continue;
                    }
                }

                // No existing configured mapping, try to auto-configure
                if default.source.is_empty() {
                    new_mappings.push(PlaceholderMapping {
                        index: *idx,
                        slot_prefix: String::new(),
                        field: default.field.clone(),
                        format: default.format.clone(),
                    });
                    i += 1;
                    continue;
                }

                let source_type = default.source.to_lowercase();

                if let Some(slots) = source_slots.get(&source_type) {
                    let instance = default.instance as usize;
                    let used = used_instances.entry(source_type.clone()).or_insert(0);

                    let slot_to_use = if instance > 0 {
                        slots.iter().find(|(_, slot_idx)| *slot_idx == instance)
                    } else {
                        let target = *used;
                        slots.get(target)
                    };

                    if let Some((prefix, _)) = slot_to_use {
                        new_mappings.push(PlaceholderMapping {
                            index: *idx,
                            slot_prefix: prefix.clone(),
                            field: default.field.clone(),
                            format: default.format.clone(),
                        });
                        if instance == 0 {
                            *used += 1;
                        }
                    } else {
                        new_mappings.push(PlaceholderMapping {
                            index: *idx,
                            slot_prefix: String::new(),
                            field: default.field.clone(),
                            format: default.format.clone(),
                        });
                    }
                } else {
                    new_mappings.push(PlaceholderMapping {
                        index: *idx,
                        slot_prefix: String::new(),
                        field: default.field.clone(),
                        format: default.format.clone(),
                    });
                }
                i += 1;
            }
        }

        drop(defaults);
        drop(summaries);

        // Apply new mappings
        self.config.borrow_mut().mappings = new_mappings;

        // Rebuild UI
        self.rebuild_mappings();

        // Trigger change callback
        if let Some(ref cb) = *self.on_change.borrow() {
            cb();
        }
    }

    /// Connect the auto-configure button handler
    /// This must be called after the widget is fully constructed
    pub fn connect_auto_config_handler(&self) {
        let self_config = self.config.clone();
        let self_on_change = self.on_change.clone();
        let self_source_summaries = self.source_summaries.clone();
        let self_placeholder_defaults = self.placeholder_defaults.clone();
        let self_placeholder_hints = self.placeholder_hints.clone();
        let self_mappings_container = self.mappings_container.clone();

        self.auto_config_btn.connect_clicked(move |_| {
            let defaults = self_placeholder_defaults.borrow();
            let summaries = self_source_summaries.borrow();

            if defaults.is_empty() || summaries.is_empty() {
                log::warn!("Cannot auto-configure: no defaults or no sources available");
                return;
            }

            // Build a map of existing mappings by index for quick lookup
            let existing_mappings: HashMap<u32, PlaceholderMapping> = self_config
                .borrow()
                .mappings
                .iter()
                .map(|m| (m.index, m.clone()))
                .collect();

            // Group source summaries by source type
            let mut source_slots: HashMap<String, Vec<(String, usize)>> = HashMap::new();
            for (prefix, label, slot_idx, _group_idx) in summaries.iter() {
                let source_type = label.split_whitespace().next().unwrap_or("").to_lowercase();
                source_slots
                    .entry(source_type)
                    .or_default()
                    .push((prefix.clone(), *slot_idx));
            }

            let mut used_instances: HashMap<String, usize> = HashMap::new();
            let mut new_mappings: Vec<PlaceholderMapping> = Vec::new();

            // Sort defaults by index for predictable order
            let mut sorted_defaults: Vec<_> = defaults.iter().collect();
            sorted_defaults.sort_by_key(|(idx, _)| *idx);

            // Process defaults, detecting groups (4 consecutive with same source/instance)
            let mut i = 0;
            while i < sorted_defaults.len() {
                let (idx, default) = sorted_defaults[i];

                // Check if this is the start of a group (4 consecutive with same source/instance)
                let is_group = if i + 3 < sorted_defaults.len() && !default.source.is_empty() {
                    let group_indices: Vec<u32> = (0..4).map(|j| *sorted_defaults[i + j].0).collect();
                    let consecutive = group_indices.windows(2).all(|w| w[1] == w[0] + 1);

                    if consecutive {
                        let group_defaults: Vec<&PlaceholderDefault> =
                            (0..4).map(|j| sorted_defaults[i + j].1).collect();

                        // Check same source and instance
                        let same_source_instance = group_defaults.iter().all(|d| {
                            d.source == default.source && d.instance == default.instance
                        });

                        // Check fields are caption/value/unit/max
                        let fields: Vec<&str> = group_defaults.iter().map(|d| d.field.as_str()).collect();
                        let standard_fields = fields == ["caption", "value", "unit", "max"];

                        same_source_instance && standard_fields
                    } else {
                        false
                    }
                } else {
                    false
                };

                if is_group {
                    // Process as a group - check if ANY member has an existing configured source
                    let group_indices: Vec<u32> = (0..4).map(|j| *sorted_defaults[i + j].0).collect();
                    let existing_prefix = group_indices
                        .iter()
                        .filter_map(|idx| existing_mappings.get(idx))
                        .find(|m| !m.slot_prefix.is_empty())
                        .map(|m| m.slot_prefix.clone());

                    let slot_prefix = if let Some(prefix) = existing_prefix {
                        // Use the existing configured prefix for all group members
                        prefix
                    } else {
                        // Auto-configure: find a slot for this source type
                        let source_type = default.source.to_lowercase();
                        if let Some(slots) = source_slots.get(&source_type) {
                            let instance = default.instance as usize;
                            let used = used_instances.entry(source_type.clone()).or_insert(0);

                            let slot_to_use = if instance > 0 {
                                slots.iter().find(|(_, slot_idx)| *slot_idx == instance)
                            } else {
                                let target = *used;
                                slots.get(target)
                            };

                            if let Some((prefix, _)) = slot_to_use {
                                if instance == 0 {
                                    *used += 1;
                                }
                                prefix.clone()
                            } else {
                                String::new()
                            }
                        } else {
                            String::new()
                        }
                    };

                    // Add all 4 group members with the same prefix
                    for j in 0..4 {
                        let (group_idx, group_default) = sorted_defaults[i + j];
                        new_mappings.push(PlaceholderMapping {
                            index: *group_idx,
                            slot_prefix: slot_prefix.clone(),
                            field: group_default.field.clone(),
                            format: group_default.format.clone(),
                        });
                    }
                    i += 4;
                } else {
                    // Process as individual mapping
                    // Check if there's an existing mapping with a configured source
                    if let Some(existing) = existing_mappings.get(idx) {
                        if !existing.slot_prefix.is_empty() {
                            // Preserve the existing mapping
                            new_mappings.push(existing.clone());
                            i += 1;
                            continue;
                        }
                    }

                    // No existing configured mapping, try to auto-configure
                    if default.source.is_empty() {
                        new_mappings.push(PlaceholderMapping {
                            index: *idx,
                            slot_prefix: String::new(),
                            field: default.field.clone(),
                            format: default.format.clone(),
                        });
                        i += 1;
                        continue;
                    }

                    let source_type = default.source.to_lowercase();

                    if let Some(slots) = source_slots.get(&source_type) {
                        let instance = default.instance as usize;
                        let used = used_instances.entry(source_type.clone()).or_insert(0);

                        let slot_to_use = if instance > 0 {
                            slots.iter().find(|(_, slot_idx)| *slot_idx == instance)
                        } else {
                            let target = *used;
                            slots.get(target)
                        };

                        if let Some((prefix, _)) = slot_to_use {
                            new_mappings.push(PlaceholderMapping {
                                index: *idx,
                                slot_prefix: prefix.clone(),
                                field: default.field.clone(),
                                format: default.format.clone(),
                            });
                            if instance == 0 {
                                *used += 1;
                            }
                        } else {
                            new_mappings.push(PlaceholderMapping {
                                index: *idx,
                                slot_prefix: String::new(),
                                field: default.field.clone(),
                                format: default.format.clone(),
                            });
                        }
                    } else {
                        new_mappings.push(PlaceholderMapping {
                            index: *idx,
                            slot_prefix: String::new(),
                            field: default.field.clone(),
                            format: default.format.clone(),
                        });
                    }
                    i += 1;
                }
            }

            drop(defaults);
            drop(summaries);

            // Apply new mappings
            self_config.borrow_mut().mappings = new_mappings;

            // Rebuild UI - clear existing rows
            while let Some(child) = self_mappings_container.first_child() {
                self_mappings_container.remove(&child);
            }

            // Add rows for each mapping, detecting groups
            let config = self_config.borrow();
            let mappings = &config.mappings;
            let mut map_idx = 0;

            while map_idx < mappings.len() {
                // Check if this could be the start of a group (4 consecutive with same prefix and standard fields)
                let is_group_ui = if map_idx + 3 < mappings.len() {
                    let prefix = &mappings[map_idx].slot_prefix;
                    let fields: Vec<&str> = mappings[map_idx..map_idx + 4]
                        .iter()
                        .map(|m| m.field.as_str())
                        .collect();

                    // Check if all 4 have the same prefix and are caption/value/unit/max
                    let same_prefix = mappings[map_idx..map_idx + 4]
                        .iter()
                        .all(|m| &m.slot_prefix == prefix);

                    same_prefix && fields == ["caption", "value", "unit", "max"]
                } else {
                    false
                };

                if is_group_ui {
                    let group_mappings: Vec<PlaceholderMapping> =
                        mappings[map_idx..map_idx + 4].to_vec();
                    CssTemplateConfigWidget::add_source_group_row(
                        &self_mappings_container,
                        map_idx,
                        &group_mappings,
                        self_config.clone(),
                        self_on_change.clone(),
                        self_source_summaries.clone(),
                        self_placeholder_hints.clone(),
                    );
                    map_idx += 4;
                } else {
                    CssTemplateConfigWidget::add_mapping_row(
                        &self_mappings_container,
                        map_idx,
                        &mappings[map_idx],
                        self_config.clone(),
                        self_on_change.clone(),
                        self_source_summaries.clone(),
                        self_placeholder_hints.clone(),
                    );
                    map_idx += 1;
                }
            }
            drop(config);

            // Trigger change callback
            if let Some(ref cb) = *self_on_change.borrow() {
                cb();
            }
        });
    }

    /// Connect the scan placeholders button handler
    ///
    /// This scans the HTML template for placeholders and creates mappings based on:
    /// 1. The rg-placeholder-config JSON block (if present) for hints and groupings
    /// 2. Direct detection of {{N}} patterns in the HTML
    ///
    /// Existing mappings are cleared and replaced with the scanned results.
    pub fn connect_scan_handler(&self) {
        let self_config = self.config.clone();
        let self_on_change = self.on_change.clone();
        let self_source_summaries = self.source_summaries.clone();
        let self_placeholder_hints = self.placeholder_hints.clone();
        let self_placeholder_defaults = self.placeholder_defaults.clone();
        let self_mappings_container = self.mappings_container.clone();

        self.scan_btn.connect_clicked(move |_| {
            let config = self_config.borrow();
            let html_path = &config.html_path;

            // Read the HTML file
            let html = match std::fs::read_to_string(html_path) {
                Ok(content) => content,
                Err(e) => {
                    log::error!("Failed to read HTML template: {}", e);
                    return;
                }
            };
            drop(config);

            // Detect all placeholders in the HTML
            let placeholder_indices = detect_placeholders(&html);
            if placeholder_indices.is_empty() {
                log::warn!("No placeholders found in HTML template");
                return;
            }

            // Extract defaults/hints from the config block
            let defaults = extract_placeholder_defaults(&html);
            let hints = extract_placeholder_hints(&html);

            // Update the shared state
            *self_placeholder_hints.borrow_mut() = hints.clone();
            *self_placeholder_defaults.borrow_mut() = defaults.clone();

            // Build new mappings based on detected placeholders
            let mut new_mappings: Vec<PlaceholderMapping> = Vec::new();

            // Process placeholders, detecting groups
            let mut idx = 0;
            while idx < placeholder_indices.len() {
                let placeholder_idx = placeholder_indices[idx];

                // Check if this could be the start of a group (4 consecutive with same source/instance)
                let is_group = if idx + 3 < placeholder_indices.len() {
                    // Check if we have defaults for all 4 and they're from the same source/instance
                    let indices: Vec<u32> = (0..4)
                        .map(|i| placeholder_indices[idx + i])
                        .collect();

                    // Check if all 4 have defaults with same source and instance
                    let group_defaults: Vec<Option<&PlaceholderDefault>> = indices
                        .iter()
                        .map(|i| defaults.get(i))
                        .collect();

                    if group_defaults.iter().all(|d| d.is_some()) {
                        let defs: Vec<&PlaceholderDefault> =
                            group_defaults.into_iter().flatten().collect();

                        // Check same source and instance
                        let first_source = &defs[0].source;
                        let first_instance = defs[0].instance;
                        let same_source_instance = defs.iter().all(|d| {
                            &d.source == first_source && d.instance == first_instance
                        });

                        // Check if fields are caption/value/unit/max
                        let fields: Vec<&str> = defs.iter().map(|d| d.field.as_str()).collect();
                        let standard_fields = fields == ["caption", "value", "unit", "max"];

                        same_source_instance && standard_fields && !first_source.is_empty()
                    } else {
                        false
                    }
                } else {
                    false
                };

                if is_group {
                    // Create 4 mappings for the group
                    let fields = ["caption", "value", "unit", "max"];
                    for (i, field) in fields.iter().enumerate() {
                        let pidx = placeholder_indices[idx + i];
                        let default = defaults.get(&pidx);
                        new_mappings.push(PlaceholderMapping {
                            index: pidx,
                            slot_prefix: String::new(), // Will be set by auto-configure
                            field: field.to_string(),
                            format: default.and_then(|d| d.format.clone()),
                        });
                    }
                    idx += 4;
                } else {
                    // Create single mapping
                    let default = defaults.get(&placeholder_idx);
                    new_mappings.push(PlaceholderMapping {
                        index: placeholder_idx,
                        slot_prefix: String::new(),
                        field: default
                            .map(|d| d.field.clone())
                            .unwrap_or_else(|| "value".to_string()),
                        format: default.and_then(|d| d.format.clone()),
                    });
                    idx += 1;
                }
            }

            // Apply new mappings
            self_config.borrow_mut().mappings = new_mappings;

            // Rebuild UI - clear existing rows
            while let Some(child) = self_mappings_container.first_child() {
                self_mappings_container.remove(&child);
            }

            // Add rows for each mapping, detecting groups
            let config = self_config.borrow();
            let mappings = &config.mappings;
            let mut map_idx = 0;

            while map_idx < mappings.len() {
                // Check if this could be the start of a group
                let is_group_ui = if map_idx + 3 < mappings.len() {
                    let fields: Vec<&str> = mappings[map_idx..map_idx + 4]
                        .iter()
                        .map(|m| m.field.as_str())
                        .collect();
                    fields == ["caption", "value", "unit", "max"]
                } else {
                    false
                };

                if is_group_ui {
                    let group_mappings: Vec<PlaceholderMapping> =
                        mappings[map_idx..map_idx + 4].to_vec();
                    CssTemplateConfigWidget::add_source_group_row(
                        &self_mappings_container,
                        map_idx,
                        &group_mappings,
                        self_config.clone(),
                        self_on_change.clone(),
                        self_source_summaries.clone(),
                        self_placeholder_hints.clone(),
                    );
                    map_idx += 4;
                } else {
                    CssTemplateConfigWidget::add_mapping_row(
                        &self_mappings_container,
                        map_idx,
                        &mappings[map_idx],
                        self_config.clone(),
                        self_on_change.clone(),
                        self_source_summaries.clone(),
                        self_placeholder_hints.clone(),
                    );
                    map_idx += 1;
                }
            }
            drop(config);

            // Trigger change callback
            if let Some(ref cb) = *self_on_change.borrow() {
                cb();
            }

            log::info!(
                "Scanned template: found {} placeholders",
                placeholder_indices.len()
            );
        });
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
    ///
    /// This detects groups of 4 consecutive mappings with the same slot_prefix
    /// (caption, value, unit, max) and displays them as grouped source rows.
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

        // Add rows for each mapping, detecting groups
        let config = self.config.borrow();
        let mappings = &config.mappings;
        let mut idx = 0;

        while idx < mappings.len() {
            // Check if this could be the start of a group (4 consecutive with same prefix)
            let is_group = if idx + 3 < mappings.len() {
                let prefix = &mappings[idx].slot_prefix;
                let fields: Vec<&str> = mappings[idx..idx + 4]
                    .iter()
                    .map(|m| m.field.as_str())
                    .collect();

                // Check if all 4 have the same prefix (including empty) and are caption/value/unit/max
                let same_prefix = mappings[idx..idx + 4]
                    .iter()
                    .all(|m| &m.slot_prefix == prefix);

                let standard_fields = fields == ["caption", "value", "unit", "max"];

                same_prefix && standard_fields
            } else {
                false
            };

            if is_group {
                // Display as a group
                let group_mappings: Vec<PlaceholderMapping> = mappings[idx..idx + 4].to_vec();

                Self::add_source_group_row(
                    &self.mappings_container,
                    idx,
                    &group_mappings,
                    self.config.clone(),
                    self.on_change.clone(),
                    self.source_summaries.clone(),
                    self.placeholder_hints.clone(),
                );
                idx += 4;
            } else {
                // Display as individual mapping
                Self::add_mapping_row(
                    &self.mappings_container,
                    idx,
                    &mappings[idx],
                    self.config.clone(),
                    self.on_change.clone(),
                    self.source_summaries.clone(),
                    self.placeholder_hints.clone(),
                );
                idx += 1;
            }
        }
    }
}

impl Default for CssTemplateConfigWidget {
    fn default() -> Self {
        Self::new()
    }
}
