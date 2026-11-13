//! CSS Template configuration widget
//!
//! Template file selection, the Bindings page (which sensor each thing the
//! template shows reads) and the animation settings.

use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, CheckButton, DropDown, Entry, FileDialog, Image, Label, Notebook,
    Orientation, ScrolledWindow, Separator, SpinButton, StringList,
};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

use crate::ui::css_template_display::{
    auto_map, bindings, detect_placeholders, extract_placeholder_defaults,
    extract_placeholder_hints, mapped_value, source_matches, split_hint, wanted_slots, Binding,
    CssTemplateDisplayConfig, PlaceholderDefault, PlaceholderMapping, SlotKind,
};
use crate::ui::editor_registry::SlotHost;
use crate::ui::widget_builder::{create_page_container, create_section_header};

/// Debounce delay for the HTML path entry (reads + parses the template file)
const HTML_PATH_DEBOUNCE_MS: u64 = 300;

type ChangeHook = Rc<RefCell<Option<Box<dyn Fn()>>>>;
type SlotRequestHook = Rc<RefCell<Option<Box<dyn Fn(Vec<(String, String)>)>>>>;

/// CSS Template configuration widget
pub struct CssTemplateConfigWidget {
    container: GtkBox,
    config: Rc<RefCell<CssTemplateDisplayConfig>>,
    on_change: ChangeHook,
    /// Asks the dialog to create source slots (Scan Placeholders).
    on_request_slots: SlotRequestHook,
    // Template widgets
    html_path_entry: Entry,
    css_path_entry: Entry,
    hot_reload_check: CheckButton,
    scan_btn: Button,
    bindings: Rc<BindingsPage>,
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
        let on_change: ChangeHook = Rc::new(RefCell::new(None));
        let on_request_slots: SlotRequestHook = Rc::new(RefCell::new(None));

        let notebook = Notebook::new();
        notebook.set_vexpand(true);

        let (template_page, html_path_entry, css_path_entry, hot_reload_check, scan_btn) =
            Self::create_template_tab(config.clone(), on_change.clone());
        notebook.append_page(&template_page, Some(&Label::new(Some("Template"))));

        let bindings = BindingsPage::new(config.clone(), on_change.clone());
        notebook.append_page(&bindings.page, Some(&Label::new(Some("Bindings"))));

        let (display_page, animation_check, animation_speed_spin) =
            Self::create_display_tab(config.clone(), on_change.clone());
        notebook.append_page(&display_page, Some(&Label::new(Some("Display"))));

        container.append(&notebook);

        let widget = Self {
            container,
            config,
            on_change,
            on_request_slots,
            html_path_entry,
            css_path_entry,
            hot_reload_check,
            scan_btn,
            bindings,
            animation_check,
            animation_speed_spin,
        };
        widget.connect_scan_handler();
        widget.connect_html_path_handler();
        widget.scan_btn.set_sensitive(false);
        widget
    }

    /// A new template path: once typing pauses, read it and show its
    /// bindings (reading and parsing per keystroke is too slow).
    fn connect_html_path_handler(&self) {
        let scan_btn = self.scan_btn.clone();
        let bindings = Rc::downgrade(&self.bindings);
        let debounce_id: Rc<Cell<u32>> = Rc::new(Cell::new(0));

        self.html_path_entry.connect_changed(move |_| {
            let current_id = debounce_id.get().wrapping_add(1);
            debounce_id.set(current_id);
            let scan_btn = scan_btn.clone();
            let bindings = bindings.clone();
            let debounce_check = debounce_id.clone();
            glib::timeout_add_local_once(
                std::time::Duration::from_millis(HTML_PATH_DEBOUNCE_MS),
                move || {
                    if debounce_check.get() != current_id {
                        return;
                    }
                    if let Some(bindings) = bindings.upgrade() {
                        bindings.rebuild();
                        scan_btn.set_sensitive(bindings.template.borrow().readable);
                    }
                },
            );
        });
    }

    /// Scan Placeholders sets the panel up: first the source slots the
    /// template asks for, then every binding from scratch.
    fn connect_scan_handler(&self) {
        let config = self.config.clone();
        let on_request_slots = self.on_request_slots.clone();
        let bindings = Rc::downgrade(&self.bindings);
        self.scan_btn.connect_clicked(move |_| {
            // No borrow is held across the hook: the dialog answers by
            // pushing the new slot list back into this widget.
            let wanted = read_template(&config.borrow())
                .map(|html| {
                    wanted_slots(
                        &detect_placeholders(&html),
                        &extract_placeholder_defaults(&html),
                    )
                })
                .unwrap_or_default();
            if !wanted.is_empty() {
                if let Some(hook) = on_request_slots.borrow().as_ref() {
                    hook(wanted);
                }
            }
            if let Some(bindings) = bindings.upgrade() {
                bindings.auto_map(true);
            }
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
            "Set the panel up for this template: add the source slots it asks for\n\
             (under Content; existing ones are kept) and bind every placeholder to\n\
             its sensor. This replaces the existing bindings.",
        ));
        scan_btn.set_sensitive(false); // Enabled when HTML path is valid
        scan_row.append(&scan_btn);

        let scan_help = Label::new(Some("Adds the sources the template needs and maps them"));
        scan_help.add_css_class("dim-label");
        scan_row.append(&scan_help);
        page.append(&scan_row);

        // Help text
        let help_label = Label::new(Some(
            "Use {{0}}, {{1}}, {{2}}, etc. as placeholders in your HTML template.\n\
             \"Scan Placeholders\" adds the sources a template asks for under Content\n\
             and maps each placeholder to its sensor; adjust them on the Bindings tab.",
        ));
        help_label.set_wrap(true);
        help_label.set_xalign(0.0);
        help_label.add_css_class("dim-label");
        help_label.set_margin_top(12);
        page.append(&help_label);

        (page, html_entry, css_entry, hot_reload_check, scan_btn)
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

        self.bindings.rebuild();
        self.scan_btn
            .set_sensitive(self.bindings.template.borrow().readable);
    }

    /// The dialog's handler for "create these source slots".
    pub fn set_on_request_slots<F: Fn(Vec<(String, String)>) + 'static>(&self, f: F) {
        *self.on_request_slots.borrow_mut() = Some(Box::new(f));
    }

    /// The dialog's services for the Bindings page: create a sensor, show a
    /// sensor's settings, read the live values.
    pub fn set_slot_host(&self, host: SlotHost) {
        *self.bindings.host.borrow_mut() = Some(host);
        self.bindings.rebuild();
    }

    /// The panel's sensors (from the tree dialog): what the picker offers.
    pub fn set_slot_kinds(&self, kinds: Vec<SlotKind>) {
        self.bindings.set_kinds(kinds);
    }

    /// Kept for the editor interface; the picker is built from
    /// [`Self::set_slot_kinds`].
    pub fn set_source_summaries(&self, _summaries: Vec<(String, String, usize, u32)>) {}
}

impl Default for CssTemplateConfigWidget {
    fn default() -> Self {
        Self::new()
    }
}

/// The template as it is on disk now, else the HTML embedded in the config.
fn read_template(config: &CssTemplateDisplayConfig) -> Option<String> {
    if !config.html_path.as_os_str().is_empty() {
        if let Ok(html) = std::fs::read_to_string(&config.html_path) {
            return Some(html);
        }
    }
    config.embedded_html.clone()
}

/// What a template asks for, as last read.
#[derive(Default)]
struct Template {
    readable: bool,
    placeholders: Vec<u32>,
    defaults: HashMap<u32, PlaceholderDefault>,
    hints: HashMap<u32, String>,
}

#[derive(Clone, Copy, PartialEq)]
enum RowState {
    Bound,
    Unbound,
    /// Bound to a sensor the panel no longer has.
    Missing,
}

/// One binding's widgets, kept for the filter, the live values and label
/// updates.
struct Row {
    outer: GtkBox,
    state: RowState,
    /// The open row stays listed whatever the filter says: binding it must
    /// not make it vanish from under the user.
    expanded: bool,
    picker: DropDown,
    model: StringList,
    /// The row's value label and the placeholders it shows (value, unit).
    summary: (Label, Option<u32>, Option<u32>),
    /// Value labels of the expanded row's placeholders.
    details: Vec<(u32, Label)>,
}

const FILTER_ALL: u32 = 0;
const BASE_FIELDS: [&str; 5] = ["caption", "value", "unit", "min", "max"];

/// The Bindings page: one row per thing the template shows (a gauge, the
/// clock), each bound to one of the panel's sensors.
///
/// The rows come from the template, never from the user: a binding is found
/// by its placeholder index, so nothing here can renumber or misplace one.
/// Every closure holds the page weakly; the widgets it builds own those
/// closures, so a strong reference would be a cycle.
struct BindingsPage {
    page: GtkBox,
    config: Rc<RefCell<CssTemplateDisplayConfig>>,
    on_change: ChangeHook,
    kinds: RefCell<Vec<SlotKind>>,
    host: RefCell<Option<SlotHost>>,
    template: RefCell<Template>,
    summary: Label,
    filter: DropDown,
    match_btn: Button,
    scroll: ScrolledWindow,
    list: GtkBox,
    rows: RefCell<Vec<Row>>,
    /// Section headers with the rows under them.
    sections: RefCell<Vec<(Label, Vec<usize>)>>,
    /// The expanded binding, by its first placeholder (one at a time: the
    /// dialog lends one sensor settings editor).
    expanded: Cell<Option<u32>>,
    /// Set while this code changes a picker; its handler then stays quiet.
    updating: Cell<bool>,
    rebuild_pending: Cell<bool>,
}

impl BindingsPage {
    fn new(config: Rc<RefCell<CssTemplateDisplayConfig>>, on_change: ChangeHook) -> Rc<Self> {
        let page = create_page_container();

        let top = GtkBox::new(Orientation::Horizontal, 6);
        let summary = Label::new(None);
        summary.set_xalign(0.0);
        summary.set_hexpand(true);
        top.append(&summary);
        let filter = DropDown::from_strings(&["Show all", "Unbound only"]);
        top.append(&filter);
        let match_btn = Button::with_label("Match automatically");
        match_btn.set_tooltip_text(Some(
            "Bind what is still unbound to a matching sensor.\n\
             Bindings you already made are kept.",
        ));
        top.append(&match_btn);
        page.append(&top);

        let scroll = ScrolledWindow::new();
        scroll.set_vexpand(true);
        scroll.set_min_content_height(260);
        scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);
        let list = GtkBox::new(Orientation::Vertical, 2);
        list.set_margin_end(8);
        scroll.set_child(Some(&list));
        page.append(&scroll);

        let this = Rc::new(Self {
            page,
            config,
            on_change,
            kinds: RefCell::new(Vec::new()),
            host: RefCell::new(None),
            template: RefCell::new(Template::default()),
            summary,
            filter,
            match_btn,
            scroll,
            list,
            rows: RefCell::new(Vec::new()),
            sections: RefCell::new(Vec::new()),
            expanded: Cell::new(None),
            updating: Cell::new(false),
            rebuild_pending: Cell::new(false),
        });

        let weak = Rc::downgrade(&this);
        this.filter.connect_selected_notify(move |_| {
            if let Some(page) = weak.upgrade() {
                page.apply_filter();
            }
        });
        let weak = Rc::downgrade(&this);
        this.match_btn.connect_clicked(move |_| {
            if let Some(page) = weak.upgrade() {
                page.auto_map(false);
            }
        });
        // Live values, once a second while the page is on screen. The timer
        // ends with the page.
        let weak = Rc::downgrade(&this);
        glib::timeout_add_seconds_local(1, move || match weak.upgrade() {
            Some(page) => {
                if page.page.is_mapped() {
                    page.refresh_values();
                }
                glib::ControlFlow::Continue
            }
            None => glib::ControlFlow::Break,
        });
        this
    }

    fn emit_change(&self) {
        if let Some(cb) = self.on_change.borrow().as_ref() {
            cb();
        }
    }

    // ---- model ---------------------------------------------------------

    /// The mapping a placeholder has, or the one the template suggests
    /// (unbound) when the config has none yet.
    fn mapping_for(&self, index: u32) -> PlaceholderMapping {
        if let Some(m) = self
            .config
            .borrow()
            .mappings
            .iter()
            .find(|m| m.index == index)
        {
            return m.clone();
        }
        let template = self.template.borrow();
        let default = template.defaults.get(&index);
        PlaceholderMapping {
            index,
            slot_prefix: String::new(),
            field: default
                .map(|d| d.field.clone())
                .filter(|f| !f.is_empty())
                .unwrap_or_else(|| "value".to_string()),
            format: default.and_then(|d| d.format.clone()),
        }
    }

    /// Change one placeholder's mapping, creating it on first edit. Found by
    /// placeholder index, never by position.
    fn edit_mapping(&self, index: u32, edit: impl FnOnce(&mut PlaceholderMapping)) {
        let mut mapping = self.mapping_for(index);
        edit(&mut mapping);
        let mut config = self.config.borrow_mut();
        match config.mappings.iter_mut().find(|m| m.index == index) {
            Some(existing) => *existing = mapping,
            None => {
                config.mappings.push(mapping);
                config.mappings.sort_by_key(|m| m.index);
            }
        }
    }

    /// The sensor a binding reads: its first placeholder that has one.
    fn binding_slot(&self, binding: &Binding) -> String {
        let config = self.config.borrow();
        binding
            .indices
            .iter()
            .filter_map(|i| config.mappings.iter().find(|m| m.index == *i))
            .map(|m| m.slot_prefix.clone())
            .find(|p| !p.is_empty())
            .unwrap_or_default()
    }

    fn set_binding_slot(&self, binding: &Binding, slot: &str) {
        for index in &binding.indices {
            self.edit_mapping(*index, |m| m.slot_prefix = slot.to_string());
        }
    }

    /// Bind the template's placeholders to the panel's sensors. `replace`
    /// starts from scratch (Scan Placeholders); otherwise what is already
    /// bound is kept (Match automatically).
    fn auto_map(self: &Rc<Self>, replace: bool) {
        self.read();
        let mappings = {
            let template = self.template.borrow();
            if template.placeholders.is_empty() && template.defaults.is_empty() {
                log::warn!("CSS template: no placeholders found in the template");
                return;
            }
            let keep: HashMap<u32, PlaceholderMapping> = if replace {
                HashMap::new()
            } else {
                self.config
                    .borrow()
                    .mappings
                    .iter()
                    .map(|m| (m.index, m.clone()))
                    .collect()
            };
            auto_map(
                &template.placeholders,
                &template.defaults,
                &keep,
                &self.kinds.borrow(),
            )
        };
        self.config.borrow_mut().mappings = mappings;
        self.rebuild();
        self.emit_change();
    }

    /// Read the template as it is now (it may have changed on disk).
    fn read(&self) {
        let html = read_template(&self.config.borrow());
        *self.template.borrow_mut() = match html {
            Some(html) => Template {
                readable: true,
                placeholders: detect_placeholders(&html),
                defaults: extract_placeholder_defaults(&html),
                hints: extract_placeholder_hints(&html),
            },
            None => Template::default(),
        };
    }

    // ---- sensors -------------------------------------------------------

    /// The panel's sensors changed. Same sensors under new names (a metric
    /// edited inline) only relabels the pickers, so the row being edited
    /// survives; anything else rebuilds.
    fn set_kinds(self: &Rc<Self>, kinds: Vec<SlotKind>) {
        let same_sensors = {
            let old = self.kinds.borrow();
            old.len() == kinds.len()
                && old
                    .iter()
                    .zip(&kinds)
                    .all(|(a, b)| a.prefix == b.prefix && a.source == b.source)
        };
        *self.kinds.borrow_mut() = kinds;
        if !same_sensors || self.rows.borrow().is_empty() {
            self.rebuild();
            return;
        }
        let labels = self.picker_labels();
        self.updating.set(true);
        for row in self.rows.borrow().iter() {
            let selected = row.picker.selected();
            for (i, label) in labels.iter().enumerate() {
                let at = i as u32 + 1;
                if row.model.string(at).as_deref() != Some(label.as_str()) {
                    row.model.splice(at, 1, &[label.as_str()]);
                }
            }
            row.picker.set_selected(selected);
        }
        self.updating.set(false);
    }

    /// One picker entry per sensor; sensors that read alike are numbered.
    fn picker_labels(&self) -> Vec<String> {
        let kinds = self.kinds.borrow();
        let mut seen: HashMap<&str, u32> = HashMap::new();
        kinds
            .iter()
            .map(|kind| {
                let n = seen.entry(kind.label.as_str()).or_insert(0);
                *n += 1;
                let twin = kinds.iter().filter(|k| k.label == kind.label).count() > 1;
                if twin && !kind.source.is_empty() {
                    format!("{} #{}", kind.label, n)
                } else {
                    kind.label.clone()
                }
            })
            .collect()
    }

    // ---- rows ----------------------------------------------------------

    /// Rebuild on idle, from inside a handler of a widget the rebuild will
    /// destroy. Keeps the scroll position.
    fn schedule_rebuild(self: &Rc<Self>) {
        if self.rebuild_pending.replace(true) {
            return;
        }
        let weak = Rc::downgrade(self);
        glib::idle_add_local_once(move || {
            if let Some(page) = weak.upgrade() {
                page.rebuild_pending.set(false);
                page.rebuild();
            }
        });
    }

    fn rebuild(self: &Rc<Self>) {
        let position = self.scroll.vadjustment().value();
        self.read();
        while let Some(child) = self.list.first_child() {
            self.list.remove(&child);
        }
        self.rows.borrow_mut().clear();
        self.sections.borrow_mut().clear();

        let all = {
            let template = self.template.borrow();
            bindings(&template.placeholders, &template.defaults, &template.hints)
        };
        self.match_btn.set_sensitive(!all.is_empty());
        if all.is_empty() {
            let saved = self.config.borrow().mappings.len();
            let text = if self.template.borrow().readable {
                "This template has no {{0}}, {{1}}, … placeholders to bind.".to_string()
            } else if saved > 0 {
                format!(
                    "The template cannot be read. Its {saved} saved bindings are kept; \
                     choose the template on the Template tab to edit them."
                )
            } else {
                "Choose a template on the Template tab. What it shows appears here, \
                 ready to bind to your sensors."
                    .to_string()
            };
            let empty = Label::new(Some(&text));
            empty.set_wrap(true);
            empty.set_xalign(0.0);
            empty.add_css_class("dim-label");
            empty.set_margin_top(12);
            self.list.append(&empty);
            self.summary.set_text("");
            return;
        }

        let mut section = None::<String>;
        for binding in &all {
            let title = source_title(&binding.source);
            if section.as_deref() != Some(title.as_str()) {
                let header = create_section_header(&title);
                self.list.append(&header);
                self.sections.borrow_mut().push((header, Vec::new()));
                section = Some(title);
            }
            let row = self.build_row(binding);
            self.list.append(&row.outer);
            let at = self.rows.borrow().len();
            if let Some((_, rows)) = self.sections.borrow_mut().last_mut() {
                rows.push(at);
            }
            self.rows.borrow_mut().push(row);
        }

        self.update_summary();
        self.apply_filter();
        self.refresh_values();
        // The list was emptied and refilled; put the view back once it has
        // its new height.
        let scroll = self.scroll.clone();
        glib::idle_add_local_once(move || scroll.vadjustment().set_value(position));
    }

    fn build_row(self: &Rc<Self>, binding: &Binding) -> Row {
        let slot = self.binding_slot(binding);
        let kinds = self.kinds.borrow().clone();
        let bound_at = kinds.iter().position(|k| k.prefix == slot);
        let state = match (slot.is_empty(), bound_at) {
            (true, _) => RowState::Unbound,
            (false, Some(_)) => RowState::Bound,
            (false, None) => RowState::Missing,
        };
        let first = binding.indices[0];
        let expanded = self.expanded.get() == Some(first);

        let outer = GtkBox::new(Orientation::Vertical, 4);
        let head = GtkBox::new(Orientation::Horizontal, 8);
        head.set_margin_top(2);
        head.set_margin_bottom(2);

        let toggle = Button::from_icon_name(if expanded {
            "pan-down-symbolic"
        } else {
            "pan-end-symbolic"
        });
        toggle.add_css_class("flat");
        toggle.set_tooltip_text(Some("Placeholders and sensor settings"));
        let weak = Rc::downgrade(self);
        toggle.connect_clicked(move |_| {
            if let Some(page) = weak.upgrade() {
                page.expanded
                    .set((page.expanded.get() != Some(first)).then_some(first));
                page.schedule_rebuild();
            }
        });
        head.append(&toggle);

        let (icon, note) = match state {
            RowState::Bound => ("object-select-symbolic", None),
            RowState::Unbound => ("dialog-warning-symbolic", Some("not bound")),
            RowState::Missing => ("dialog-warning-symbolic", Some("its sensor was removed")),
        };
        let status = Image::from_icon_name(icon);
        if state != RowState::Bound {
            status.add_css_class("warning");
        }
        head.append(&status);

        let names = GtkBox::new(Orientation::Vertical, 0);
        names.set_hexpand(true);
        let title = Label::new(Some(&binding.title));
        title.set_xalign(0.0);
        title.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        names.append(&title);
        let parts: Vec<String> = binding
            .indices
            .iter()
            .map(|i| self.part_name(*i).to_lowercase())
            .collect();
        let subtitle = match note {
            Some(note) => format!("{} — {}", parts.join(", "), note),
            None => parts.join(", "),
        };
        let subtitle = Label::new(Some(&subtitle));
        subtitle.set_xalign(0.0);
        subtitle.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        subtitle.add_css_class("dim-label");
        subtitle.add_css_class("caption");
        names.append(&subtitle);
        head.append(&names);

        // Picker: nothing, the panel's sensors, the missing one, new ones.
        let model = StringList::new(&["Choose a sensor…"]);
        for label in self.picker_labels() {
            model.append(&label);
        }
        if state == RowState::Missing {
            model.append("Removed sensor");
        }
        let new_sources = self.new_sources(binding);
        for (_, name) in &new_sources {
            model.append(&format!("New {name} sensor"));
        }
        let picker = DropDown::new(Some(model.clone()), None::<gtk4::Expression>);
        picker.set_size_request(240, -1);
        picker.set_valign(gtk4::Align::Center);
        picker.set_selected(match (state, bound_at) {
            (RowState::Bound, Some(at)) => at as u32 + 1,
            (RowState::Missing, _) => kinds.len() as u32 + 1,
            _ => 0,
        });
        let weak = Rc::downgrade(self);
        let for_pick = binding.clone();
        let missing = state == RowState::Missing;
        picker.connect_selected_notify(move |picker| {
            let Some(page) = weak.upgrade() else { return };
            if page.updating.get() {
                return;
            }
            let selected = picker.selected() as usize;
            let sensors = page.kinds.borrow().len();
            let slot = if selected == 0 {
                String::new()
            } else if selected <= sensors {
                page.kinds.borrow()[selected - 1].prefix.clone()
            } else if missing && selected == sensors + 1 {
                return;
            } else {
                let Some((source, _)) = new_sources.get(selected - sensors - 1 - missing as usize)
                else {
                    return;
                };
                // The template's metric only fits the source it names.
                let metric = if source_matches(source, &for_pick.source) {
                    for_pick.metric.as_str()
                } else {
                    ""
                };
                let host = page.host.borrow().clone();
                match host.and_then(|h| (h.create)(source, metric)) {
                    Some(slot) => slot,
                    None => {
                        page.schedule_rebuild();
                        return;
                    }
                }
            };
            page.set_binding_slot(&for_pick, &slot);
            // A picked sensor shows its settings at once.
            page.expanded
                .set((!slot.is_empty()).then_some(for_pick.indices[0]));
            page.emit_change();
            page.schedule_rebuild();
        });
        head.append(&picker);

        let value = Label::new(Some("--"));
        value.set_width_chars(10);
        value.set_xalign(1.0);
        value.add_css_class("monospace");
        head.append(&value);
        outer.append(&head);

        let field_of = |index: &u32| self.mapping_for(*index).field;
        let value_index = binding
            .indices
            .iter()
            .copied()
            .find(|i| field_of(i) == "value")
            .or(Some(first));
        let unit_index = binding
            .indices
            .iter()
            .copied()
            .find(|i| field_of(i) == "unit");

        let mut details = Vec::new();
        if expanded {
            let source = bound_at.map(|at| kinds[at].source.clone());
            outer.append(&self.build_details(binding, state, &slot, source, &mut details));
        }
        outer.append(&Separator::new(Orientation::Horizontal));

        Row {
            outer,
            state,
            expanded,
            picker,
            model,
            summary: (value, value_index, unit_index),
            details,
        }
    }

    /// The expanded part of a row: each placeholder's field and format, then
    /// the bound sensor's own settings.
    fn build_details(
        self: &Rc<Self>,
        binding: &Binding,
        state: RowState,
        slot: &str,
        source: Option<String>,
        values: &mut Vec<(u32, Label)>,
    ) -> GtkBox {
        let details = GtkBox::new(Orientation::Vertical, 4);
        details.set_margin_start(36);
        details.set_margin_bottom(8);

        let mut fields: Vec<String> = BASE_FIELDS.iter().map(|f| f.to_string()).collect();
        if let Some(source) = source.as_deref().filter(|s| !s.is_empty()) {
            for field in crate::core::global_registry().get_source_fields_cached(source) {
                if !fields.contains(&field.id) {
                    fields.push(field.id);
                }
            }
        }

        let grid = gtk4::Grid::new();
        grid.set_column_spacing(8);
        grid.set_row_spacing(4);
        for (line, index) in binding.indices.iter().copied().enumerate() {
            let mapping = self.mapping_for(index);
            let line = line as i32;

            let number = Label::new(Some(&format!("{{{{{index}}}}}")));
            number.set_xalign(0.0);
            number.add_css_class("monospace");
            number.add_css_class("dim-label");
            grid.attach(&number, 0, line, 1, 1);

            let part = Label::new(Some(&self.part_name(index)));
            part.set_xalign(0.0);
            part.set_hexpand(true);
            part.set_ellipsize(gtk4::pango::EllipsizeMode::End);
            grid.attach(&part, 1, line, 1, 1);

            // The sensor's fields, plus this placeholder's own when it reads
            // one the list lacks: a row must never misreport what it reads.
            let mut options = fields.clone();
            if !options.contains(&mapping.field) {
                options.push(mapping.field.clone());
            }
            let refs: Vec<&str> = options.iter().map(String::as_str).collect();
            let field = DropDown::from_strings(&refs);
            field.set_selected(
                options
                    .iter()
                    .position(|f| *f == mapping.field)
                    .unwrap_or(0) as u32,
            );
            field.set_tooltip_text(Some("Which of the sensor's values this placeholder shows"));
            let weak = Rc::downgrade(self);
            field.connect_selected_notify(move |field| {
                let (Some(page), Some(name)) =
                    (weak.upgrade(), options.get(field.selected() as usize))
                else {
                    return;
                };
                page.edit_mapping(index, |m| m.field = name.clone());
                page.emit_change();
                page.refresh_values();
            });
            grid.attach(&field, 2, line, 1, 1);

            let format = Entry::new();
            format.set_placeholder_text(Some("{:.1}"));
            format.set_tooltip_text(Some(
                "Number format: {:.0} for no decimals, {:.1}% for one and a % sign",
            ));
            format.set_width_chars(8);
            format.set_text(mapping.format.as_deref().unwrap_or(""));
            let weak = Rc::downgrade(self);
            format.connect_changed(move |format| {
                let Some(page) = weak.upgrade() else { return };
                let text = format.text();
                page.edit_mapping(index, |m| {
                    m.format = (!text.is_empty()).then(|| text.to_string());
                });
                page.emit_change();
                page.refresh_values();
            });
            grid.attach(&format, 3, line, 1, 1);

            let value = Label::new(Some("--"));
            value.set_width_chars(10);
            value.set_xalign(1.0);
            value.add_css_class("monospace");
            grid.attach(&value, 4, line, 1, 1);
            values.push((index, value));
        }
        details.append(&grid);

        let settings = match state {
            RowState::Bound => {
                let host = self.host.borrow().clone();
                host.and_then(|h| (h.settings)(slot))
            }
            _ => None,
        };
        match settings {
            Some(widget) => {
                let header = Label::new(Some("Sensor settings"));
                header.set_xalign(0.0);
                header.add_css_class("dim-label");
                header.set_margin_top(8);
                details.append(&header);
                details.append(&widget);
            }
            None => {
                let text = match state {
                    RowState::Bound => "This sensor has no settings.",
                    RowState::Unbound => "Pick a sensor to see its settings here.",
                    RowState::Missing => {
                        "The sensor this was bound to is gone from Content. Pick another; \
                         the binding is kept in case the sensor comes back."
                    }
                };
                let hint = Label::new(Some(text));
                hint.set_xalign(0.0);
                hint.set_wrap(true);
                hint.add_css_class("dim-label");
                hint.set_margin_top(6);
                details.append(&hint);
            }
        }
        details
    }

    /// What a placeholder is within its binding: the part its hint names
    /// (`Value`), else the field it reads.
    fn part_name(&self, index: u32) -> String {
        let template = self.template.borrow();
        let part = template
            .hints
            .get(&index)
            .map(|hint| split_hint(hint).1.to_string())
            .unwrap_or_default();
        if part.is_empty() {
            drop(template);
            self.mapping_for(index).field
        } else {
            part
        }
    }

    /// The sources "New … sensor" offers: the one the template names, or
    /// every source when it names none.
    fn new_sources(&self, binding: &Binding) -> Vec<(String, String)> {
        if self.host.borrow().is_none() {
            return Vec::new();
        }
        let all = crate::ui::config_tree::util::template_source_choices();
        if binding.source.is_empty() {
            return all;
        }
        let wanted: Vec<_> = all
            .iter()
            .filter(|(id, _)| source_matches(id, &binding.source))
            .cloned()
            .collect();
        if wanted.is_empty() {
            all
        } else {
            wanted
        }
    }

    // ---- status, filter, live values -----------------------------------

    fn update_summary(&self) {
        let rows = self.rows.borrow();
        let count = |state: RowState| rows.iter().filter(|r| r.state == state).count();
        let mut text = format!("{} bound", count(RowState::Bound));
        if count(RowState::Unbound) > 0 {
            text.push_str(&format!(" · {} not bound", count(RowState::Unbound)));
        }
        if count(RowState::Missing) > 0 {
            text.push_str(&format!(" · {} sensor removed", count(RowState::Missing)));
        }
        self.summary.set_text(&text);
    }

    fn apply_filter(&self) {
        let all = self.filter.selected() == FILTER_ALL;
        let rows = self.rows.borrow();
        for row in rows.iter() {
            row.outer
                .set_visible(all || row.expanded || row.state != RowState::Bound);
        }
        for (header, members) in self.sections.borrow().iter() {
            header.set_visible(members.iter().any(|at| rows[*at].outer.is_visible()));
        }
    }

    fn refresh_values(&self) {
        let host = self.host.borrow().clone();
        let Some(values) = host.and_then(|h| (h.values)()) else {
            return;
        };
        let show = |index: u32| mapped_value(&values, &self.mapping_for(index));
        for row in self.rows.borrow().iter() {
            let (label, value, unit) = &row.summary;
            label.set_text(&summary_text(value.map(show), unit.map(show)));
            for (index, label) in &row.details {
                label.set_text(&show(*index));
            }
        }
    }
}

/// What a row shows as its live value: the binding's value with its unit.
/// `--` when there is no value yet (unbound, or the sensor has not reported).
fn summary_text(value: Option<String>, unit: Option<String>) -> String {
    match (value, unit) {
        (Some(value), Some(unit)) if value != "--" && unit != "--" => format!("{value} {unit}"),
        (Some(value), _) => value,
        (None, _) => "--".to_string(),
    }
}

/// The section a binding is listed under: the source its template names.
fn source_title(source: &str) -> String {
    if source.is_empty() {
        return "Other".to_string();
    }
    crate::ui::config_tree::util::template_source_choices()
        .into_iter()
        .find(|(id, _)| source_matches(id, source))
        .map(|(_, name)| name)
        .unwrap_or_else(|| source.to_string())
}

/// Renders the Bindings page to a PNG, for looking at it without the app:
///
/// ```bash
/// gtk4-broadwayd :7 &
/// GDK_BACKEND=broadway BROADWAY_DISPLAY=:7 GSK_RENDERER=cairo BINDINGS_PNG=/tmp/bindings.png \
///     cargo test --lib bindings_page_preview -- --ignored
/// ```
///
/// `bindings_page_interactions` runs the same way, on its own: GTK belongs
/// to the thread that initialised it, so the two cannot share a test run.
///
/// ```bash
/// GDK_BACKEND=broadway BROADWAY_DISPLAY=:7 GSK_RENDERER=cairo \
///     cargo test --lib bindings_page_interactions -- --ignored
/// ```
#[cfg(test)]
mod preview {
    use super::*;
    use crate::ui::config_tree::util::{add_wanted_slots, combo_slot_kinds};
    use crate::ui::editor_registry;
    use std::sync::Arc;

    fn pump(ms: u64) {
        let context = glib::MainContext::default();
        let until = std::time::Instant::now() + std::time::Duration::from_millis(ms);
        while std::time::Instant::now() < until {
            context.iteration(false);
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
    }

    /// Drives the pickers the way a user does, against a stand-in for the
    /// dialog: bind, unbind, create a sensor, relabel a sensor in place.
    #[test]
    #[ignore = "needs a display"]
    fn bindings_page_interactions() {
        gtk4::init().expect("a display");
        crate::sources::register_all();
        editor_registry::register_all_editors();

        let html_path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/art_deco_panel.html");
        let html = std::fs::read_to_string(&html_path).unwrap();
        let (placeholders, defaults) = (
            detect_placeholders(&html),
            extract_placeholder_defaults(&html),
        );
        let source = Rc::new(RefCell::new(crate::sources::ComboSourceConfig::default()));
        add_wanted_slots(
            &mut source.borrow_mut(),
            &wanted_slots(&placeholders, &defaults),
        );
        let kinds = combo_slot_kinds(&source.borrow());
        let mappings = auto_map(&placeholders, &defaults, &HashMap::new(), &kinds);

        let widget = Rc::new(CssTemplateConfigWidget::new());
        let changes = Rc::new(Cell::new(0));
        let counter = changes.clone();
        widget.set_on_change(move || counter.set(counter.get() + 1));
        widget.set_slot_kinds(kinds.clone());
        let (for_create, weak) = (source.clone(), Rc::downgrade(&widget));
        widget.set_slot_host(SlotHost {
            // Like the dialog: the slot exists at once, the new sensor list
            // reaches the page on idle.
            create: Rc::new(move |src, metric| {
                let slot = crate::ui::config_tree::util::add_slot(
                    &mut for_create.borrow_mut(),
                    src,
                    metric,
                )?;
                let (source, weak) = (for_create.clone(), weak.clone());
                glib::idle_add_local_once(move || {
                    if let Some(w) = weak.upgrade() {
                        w.set_slot_kinds(combo_slot_kinds(&source.borrow()));
                    }
                });
                Some(slot)
            }),
            settings: Rc::new(|_| Some(Label::new(Some("settings")).upcast())),
            values: Rc::new(|| None),
        });
        widget.set_config(&CssTemplateDisplayConfig {
            html_path,
            mappings,
            ..Default::default()
        });
        let window = gtk4::Window::new();
        window.set_child(Some(widget.widget()));
        window.present();
        pump(300);
        changes.set(0);

        let page = &widget.bindings;
        let slot_of = |index: u32| {
            widget
                .get_config()
                .mappings
                .iter()
                .find(|m| m.index == index)
                .map(|m| m.slot_prefix.clone())
                .unwrap_or_default()
        };
        let pick = |row: usize, item: u32| {
            let picker = page.rows.borrow()[row].picker.clone();
            picker.set_selected(item);
            pump(200);
        };
        // rows: 0 clock, 1 CPU temperature (4..=7), 2 CPU usage (8..=11),
        // 4 CPU power (16..=19, names no source)
        let usage = slot_of(8);
        assert_ne!(slot_of(4), usage);

        // bind the gauge to the usage sensor: all four placeholders follow
        let usage_item = kinds.iter().position(|k| k.prefix == usage).unwrap() as u32 + 1;
        pick(1, usage_item);
        assert!((4..=7).all(|i| slot_of(i) == usage));
        assert_eq!(
            page.expanded.get(),
            Some(4),
            "a picked sensor opens its row"
        );
        assert!(changes.get() > 0);
        assert_eq!(slot_of(8), usage, "other bindings are untouched");

        // unbind it; field and format survive
        pick(1, 0);
        assert!((4..=7).all(|i| slot_of(i).is_empty()));
        let value = widget.get_config().mappings[5].clone();
        assert_eq!(
            (value.field.as_str(), value.format.as_deref()),
            ("value", Some("{:.0}"))
        );
        assert!(page.rows.borrow()[1].state == RowState::Unbound);

        // CPU power names no source: a new GPU sensor is made and bound
        let before = combo_slot_kinds(&source.borrow()).len();
        let model = page.rows.borrow()[4].model.clone();
        let new_gpu = (0..model.n_items())
            .find(|i| model.string(*i).as_deref() == Some("New Gpu sensor"))
            .expect("unsourced bindings offer every source");
        pick(4, new_gpu);
        let kinds_now = combo_slot_kinds(&source.borrow());
        assert_eq!(kinds_now.len(), before + 1);
        let made = slot_of(16);
        assert!(kinds_now
            .iter()
            .any(|k| k.prefix == made && k.source == "gpu"));
        assert!(page.rows.borrow()[4].state == RowState::Bound);

        // a sensor renamed in place (metric edited inline) keeps the rows
        let mut renamed = kinds_now.clone();
        let at = renamed.iter().position(|k| k.prefix == made).unwrap();
        renamed[at].label = "Gpu · Fan speed".to_string();
        let changes_before = changes.get();
        widget.set_slot_kinds(renamed);
        pump(100);
        {
            let rows = page.rows.borrow();
            let row = &rows[4];
            assert_eq!(
                row.model.string(row.picker.selected()).as_deref(),
                Some("Gpu · Fan speed")
            );
        }
        assert_eq!(slot_of(16), made, "relabelling binds nothing");
        assert_eq!(changes.get(), changes_before);

        // a binding whose sensor is gone is kept and flagged
        let without: Vec<_> = kinds_now
            .iter()
            .filter(|k| k.prefix != made)
            .cloned()
            .collect();
        widget.set_slot_kinds(without);
        pump(100);
        assert!(page.rows.borrow()[4].state == RowState::Missing);
        assert_eq!(slot_of(16), made);
    }

    #[test]
    #[ignore = "needs a display; writes $BINDINGS_PNG"]
    fn bindings_page_preview() {
        let Ok(path) = std::env::var("BINDINGS_PNG") else {
            return;
        };
        gtk4::init().expect("a display");
        crate::sources::register_all();
        editor_registry::register_all_editors();

        // The Art Deco example set up by Scan Placeholders, then one binding
        // pointed at a sensor that is gone.
        let html_path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/art_deco_panel.html");
        let html = std::fs::read_to_string(&html_path).unwrap();
        let (placeholders, defaults) = (
            detect_placeholders(&html),
            extract_placeholder_defaults(&html),
        );
        let mut source = crate::sources::ComboSourceConfig::default();
        add_wanted_slots(&mut source, &wanted_slots(&placeholders, &defaults));
        let kinds = combo_slot_kinds(&source);
        let mut mappings = auto_map(&placeholders, &defaults, &HashMap::new(), &kinds);
        for m in mappings.iter_mut().filter(|m| (8..12).contains(&m.index)) {
            m.slot_prefix = "group9_9".to_string();
        }
        let gauge = mappings[5].slot_prefix.clone();

        let widget = CssTemplateConfigWidget::new();
        widget.set_slot_kinds(kinds);
        let mut values: HashMap<String, serde_json::Value> = HashMap::new();
        for (key, value) in [("caption", "CPU Temp"), ("unit", "°C")] {
            values.insert(format!("{gauge}_{key}"), value.into());
        }
        values.insert(format!("{gauge}_value"), 54.3.into());
        values.insert(format!("{gauge}_max_limit"), 95.0.into());
        values.insert("group1_1_time".to_string(), "14:32".into());
        let values = Arc::new(values);
        let editors: Rc<RefCell<Vec<Box<dyn editor_registry::SourceEditor>>>> = Rc::default();
        widget.set_slot_host(SlotHost {
            create: Rc::new(|_, _| None),
            settings: Rc::new(move |_| {
                let editor = editor_registry::create_source_editor("cpu")?;
                let w = editor.widget();
                editors.borrow_mut().push(editor);
                Some(w)
            }),
            values: Rc::new(move || Some(values.clone())),
        });
        widget.set_config(&CssTemplateDisplayConfig {
            html_path,
            mappings,
            ..Default::default()
        });
        widget.bindings.expanded.set(Some(4));
        widget.bindings.rebuild();

        let notebook = widget
            .widget()
            .first_child()
            .and_downcast::<Notebook>()
            .unwrap();
        notebook.set_current_page(Some(1));
        let window = gtk4::Window::new();
        window.set_default_size(820, 1000);
        window.set_child(Some(widget.widget()));
        window.present();

        let context = glib::MainContext::default();
        let until = std::time::Instant::now() + std::time::Duration::from_millis(1500);
        while std::time::Instant::now() < until {
            context.iteration(false);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let (w, h) = (window.width() as f64, window.height() as f64);
        let paintable = gtk4::WidgetPaintable::new(Some(&window));
        let snapshot = gtk4::Snapshot::new();
        gtk4::gdk::prelude::PaintableExt::snapshot(&paintable, &snapshot, w, h);
        let node = snapshot.to_node().expect("something drawn");
        let renderer = gtk4::prelude::NativeExt::renderer(&window).expect("a renderer");
        renderer
            .render_texture(&node, None)
            .save_to_png(&path)
            .unwrap();
    }
}
