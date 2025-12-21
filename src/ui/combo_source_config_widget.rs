//! Configuration widget for the Combination source
//!
//! Provides UI for configuring multiple data sources in a combo/combination panel.
//! Each slot can have its own source type and source-specific configuration.

use crate::core::global_registry;
use crate::sources::ComboSourceConfig;
use crate::ui::{
    CpuSourceConfigWidget, GpuSourceConfigWidget, MemorySourceConfigWidget,
    SystemTempConfigWidget, FanSpeedConfigWidget, DiskSourceConfigWidget,
    ClockSourceConfigWidget, StaticTextConfigWidget,
};
use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, DropDown, Entry, Frame, Label, Notebook, Orientation, ScrolledWindow,
    SpinButton, StringList, Widget,
};
use serde_json::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// Enum to hold different source config widget types
enum SourceConfigWidgetType {
    Cpu(CpuSourceConfigWidget),
    Gpu(GpuSourceConfigWidget),
    Memory(MemorySourceConfigWidget),
    SystemTemp(SystemTempConfigWidget),
    FanSpeed(FanSpeedConfigWidget),
    Disk(DiskSourceConfigWidget),
    Clock(ClockSourceConfigWidget),
    StaticText(StaticTextConfigWidget),
}

impl SourceConfigWidgetType {
    fn widget(&self) -> Widget {
        match self {
            SourceConfigWidgetType::Cpu(w) => w.widget().clone().upcast(),
            SourceConfigWidgetType::Gpu(w) => w.widget().clone().upcast(),
            SourceConfigWidgetType::Memory(w) => w.widget().clone().upcast(),
            SourceConfigWidgetType::SystemTemp(w) => w.widget().clone().upcast(),
            SourceConfigWidgetType::FanSpeed(w) => w.widget().clone().upcast(),
            SourceConfigWidgetType::Disk(w) => w.widget().clone().upcast(),
            SourceConfigWidgetType::Clock(w) => w.widget().clone().upcast(),
            SourceConfigWidgetType::StaticText(w) => w.widget().clone().upcast(),
        }
    }

    fn get_config_json(&self) -> Option<Value> {
        match self {
            SourceConfigWidgetType::Cpu(w) => serde_json::to_value(w.get_config()).ok(),
            SourceConfigWidgetType::Gpu(w) => serde_json::to_value(w.get_config()).ok(),
            SourceConfigWidgetType::Memory(w) => serde_json::to_value(w.get_config()).ok(),
            SourceConfigWidgetType::SystemTemp(w) => serde_json::to_value(w.get_config()).ok(),
            SourceConfigWidgetType::FanSpeed(w) => serde_json::to_value(w.get_config()).ok(),
            SourceConfigWidgetType::Disk(w) => serde_json::to_value(w.get_config()).ok(),
            SourceConfigWidgetType::Clock(w) => serde_json::to_value(w.get_config()).ok(),
            SourceConfigWidgetType::StaticText(w) => serde_json::to_value(w.get_config()).ok(),
        }
    }

    fn set_config_from_json(&self, config: &HashMap<String, Value>) {
        if config.is_empty() {
            log::info!("set_config_from_json: empty config, skipping");
            return;
        }

        let json_value = serde_json::to_value(config).unwrap_or_default();
        log::info!("set_config_from_json: loading config with {} keys", config.len());

        match self {
            SourceConfigWidgetType::Cpu(w) => {
                match serde_json::from_value(json_value) {
                    Ok(cfg) => {
                        log::info!("Successfully loaded CPU config");
                        w.set_config(cfg);
                    }
                    Err(e) => log::warn!("Failed to deserialize CPU config: {}", e),
                }
            }
            SourceConfigWidgetType::Gpu(w) => {
                match serde_json::from_value(json_value) {
                    Ok(cfg) => {
                        log::info!("Successfully loaded GPU config");
                        w.set_config(cfg);
                    }
                    Err(e) => log::warn!("Failed to deserialize GPU config: {}", e),
                }
            }
            SourceConfigWidgetType::Memory(w) => {
                match serde_json::from_value(json_value) {
                    Ok(cfg) => {
                        log::info!("Successfully loaded Memory config");
                        w.set_config(cfg);
                    }
                    Err(e) => log::warn!("Failed to deserialize Memory config: {}", e),
                }
            }
            SourceConfigWidgetType::SystemTemp(w) => {
                match serde_json::from_value(json_value) {
                    Ok(cfg) => {
                        log::info!("Successfully loaded SystemTemp config");
                        w.set_config(cfg);
                    }
                    Err(e) => log::warn!("Failed to deserialize SystemTemp config: {}", e),
                }
            }
            SourceConfigWidgetType::FanSpeed(w) => {
                match serde_json::from_value(json_value) {
                    Ok(cfg) => {
                        log::info!("Successfully loaded FanSpeed config");
                        w.set_config(&cfg);
                    }
                    Err(e) => log::warn!("Failed to deserialize FanSpeed config: {}", e),
                }
            }
            SourceConfigWidgetType::Disk(w) => {
                match serde_json::from_value(json_value) {
                    Ok(cfg) => {
                        log::info!("Successfully loaded Disk config");
                        w.set_config(cfg);
                    }
                    Err(e) => log::warn!("Failed to deserialize Disk config: {}", e),
                }
            }
            SourceConfigWidgetType::Clock(w) => {
                match serde_json::from_value(json_value) {
                    Ok(cfg) => {
                        log::info!("Successfully loaded Clock config");
                        w.set_config(&cfg);
                    }
                    Err(e) => log::warn!("Failed to deserialize Clock config: {}", e),
                }
            }
            SourceConfigWidgetType::StaticText(w) => {
                match serde_json::from_value(json_value) {
                    Ok(cfg) => {
                        log::info!("Successfully loaded StaticText config");
                        w.set_config(&cfg);
                    }
                    Err(e) => log::warn!("Failed to deserialize StaticText config: {}", e),
                }
            }
        }
    }
}

/// Widget for configuring a Combination data source
pub struct ComboSourceConfigWidget {
    container: GtkBox,
    config: Rc<RefCell<ComboSourceConfig>>,
    /// Spinner for number of groups (1-8)
    group_count_spin: SpinButton,
    update_interval_spin: SpinButton,
    /// Main notebook for groups
    notebook: Notebook,
    /// Maps slot name (e.g., "group1_1") to its widgets
    slot_widgets: Rc<RefCell<HashMap<String, SlotWidgets>>>,
    on_change: Rc<RefCell<Option<Box<dyn Fn()>>>>,
}

/// Widgets for a single slot configuration
#[allow(dead_code)]
struct SlotWidgets {
    source_dropdown: DropDown,
    caption_entry: Entry,
    source_ids: Vec<String>,
    config_container: GtkBox,
    source_config_widget: Rc<RefCell<Option<SourceConfigWidgetType>>>,
}

impl ComboSourceConfigWidget {
    pub fn new() -> Self {
        let config = Rc::new(RefCell::new(ComboSourceConfig::default()));
        let on_change: Rc<RefCell<Option<Box<dyn Fn()>>>> = Rc::new(RefCell::new(None));

        let container = GtkBox::new(Orientation::Vertical, 12);
        container.set_margin_start(12);
        container.set_margin_end(12);
        container.set_margin_top(12);
        container.set_margin_bottom(12);

        // Title
        let title = Label::new(Some("Combination Source Configuration"));
        title.add_css_class("heading");
        title.set_halign(gtk4::Align::Start);
        container.append(&title);

        // Settings row: Group Count and Update Interval
        let settings_box = GtkBox::new(Orientation::Horizontal, 24);

        // Group count spinner
        let group_count_box = GtkBox::new(Orientation::Horizontal, 8);
        let group_count_label = Label::new(Some("Number of Groups:"));
        let group_count_spin = SpinButton::with_range(1.0, 8.0, 1.0);
        group_count_spin.set_value(config.borrow().groups.len() as f64);
        group_count_box.append(&group_count_label);
        group_count_box.append(&group_count_spin);
        settings_box.append(&group_count_box);

        // Update interval spinner
        let interval_box = GtkBox::new(Orientation::Horizontal, 8);
        let interval_label = Label::new(Some("Update Interval (ms):"));
        let update_interval_spin = SpinButton::with_range(50.0, 10000.0, 50.0);
        update_interval_spin.set_value(config.borrow().update_interval_ms as f64);
        interval_box.append(&interval_label);
        interval_box.append(&update_interval_spin);
        settings_box.append(&interval_box);

        container.append(&settings_box);

        // Main notebook for groups
        let notebook = Notebook::new();
        notebook.set_scrollable(true);
        notebook.set_vexpand(true);

        // Wrap notebook in scrolled window for better UX with many tabs
        let scrolled = ScrolledWindow::new();
        scrolled.set_child(Some(&notebook));
        scrolled.set_vexpand(true);
        scrolled.set_min_content_height(400);
        container.append(&scrolled);

        let slot_widgets: Rc<RefCell<HashMap<String, SlotWidgets>>> =
            Rc::new(RefCell::new(HashMap::new()));

        let widget = Self {
            container,
            config,
            group_count_spin,
            update_interval_spin,
            notebook,
            slot_widgets,
            on_change,
        };

        // Build initial tabs
        widget.rebuild_tabs();

        // Connect group count spinner change
        {
            let config_clone = widget.config.clone();
            let notebook_clone = widget.notebook.clone();
            let slot_widgets_clone = widget.slot_widgets.clone();
            let on_change_clone = widget.on_change.clone();

            widget.group_count_spin.connect_value_changed(move |spin| {
                let new_count = spin.value() as usize;
                {
                    let mut cfg = config_clone.borrow_mut();
                    // Adjust groups vector
                    while cfg.groups.len() < new_count {
                        cfg.groups.push(crate::sources::GroupConfig { item_count: 2, ..Default::default() });
                    }
                    while cfg.groups.len() > new_count {
                        cfg.groups.pop();
                    }
                }
                Self::rebuild_tabs_internal(&config_clone, &notebook_clone, &slot_widgets_clone, &on_change_clone);
                if let Some(cb) = on_change_clone.borrow().as_ref() {
                    cb();
                }
            });
        }

        // Connect update interval spin button
        {
            let config_for_interval = widget.config.clone();
            let on_change_for_interval = widget.on_change.clone();
            widget.update_interval_spin.connect_value_changed(move |spin| {
                config_for_interval.borrow_mut().update_interval_ms = spin.value() as u64;
                if let Some(cb) = on_change_for_interval.borrow().as_ref() {
                    cb();
                }
            });
        }

        widget
    }

    /// Get the widget container
    pub fn widget(&self) -> &Widget {
        self.container.upcast_ref()
    }

    /// Set the configuration
    pub fn set_config(&self, mut config: ComboSourceConfig) {
        // Migrate legacy format if needed
        config.migrate_legacy();

        log::info!("ComboSourceConfigWidget::set_config called with {} groups, {} slots",
            config.groups.len(), config.slots.len());
        for (slot_name, slot_cfg) in &config.slots {
            log::info!("  Slot '{}': source='{}', source_config keys: {:?}",
                slot_name,
                slot_cfg.source_id,
                slot_cfg.source_config.keys().collect::<Vec<_>>()
            );
        }

        // Save the group count and interval before moving config
        let group_count = config.groups.len();
        let update_interval_ms = config.update_interval_ms;

        // CRITICAL: Clear slot_widgets BEFORE setting spin buttons!
        // This prevents rebuild_tabs_internal from saving old widget values
        // back to the config, which would overwrite our new slot data.
        self.slot_widgets.borrow_mut().clear();

        // Set the full config
        *self.config.borrow_mut() = config;

        // Update spin buttons - callbacks will trigger but slot_widgets is empty
        // so no save-back will occur.
        self.group_count_spin.set_value(group_count as f64);
        self.update_interval_spin.set_value(update_interval_ms as f64);

        // Final rebuild with the correct config
        self.rebuild_tabs();
    }

    /// Get the current configuration
    pub fn get_config(&self) -> ComboSourceConfig {
        // Update config from widgets before returning
        let mut config = self.config.borrow().clone();

        // Sync update interval from spin button
        config.update_interval_ms = self.update_interval_spin.value() as u64;

        for (slot_name, widgets) in self.slot_widgets.borrow().iter() {
            let slot_config = config.slots.entry(slot_name.clone()).or_default();

            // Get selected source
            let selected_idx = widgets.source_dropdown.selected() as usize;
            if selected_idx < widgets.source_ids.len() {
                slot_config.source_id = widgets.source_ids[selected_idx].clone();
            }

            // Get caption override
            slot_config.caption_override = widgets.caption_entry.text().to_string();

            // Get source-specific config from embedded widget
            if let Some(ref source_widget) = *widgets.source_config_widget.borrow() {
                if let Some(config_value) = source_widget.get_config_json() {
                    // Convert Value to HashMap<String, Value>
                    if let Some(obj) = config_value.as_object() {
                        slot_config.source_config = obj.iter()
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect();
                    }
                }
            }
        }

        config
    }

    /// Set the on_change callback
    pub fn set_on_change<F: Fn() + 'static>(&self, callback: F) {
        *self.on_change.borrow_mut() = Some(Box::new(callback));
    }

    /// Rebuild the notebook tabs based on current configuration
    fn rebuild_tabs(&self) {
        Self::rebuild_tabs_internal(
            &self.config,
            &self.notebook,
            &self.slot_widgets,
            &self.on_change,
        );
    }

    fn rebuild_tabs_internal(
        config: &Rc<RefCell<ComboSourceConfig>>,
        notebook: &Notebook,
        slot_widgets: &Rc<RefCell<HashMap<String, SlotWidgets>>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
    ) {
        // IMPORTANT: Before clearing widgets, save their current values back to config
        // This preserves settings when group/item counts change
        {
            let mut cfg = config.borrow_mut();
            for (slot_name, widgets) in slot_widgets.borrow().iter() {
                let slot_config = cfg.slots.entry(slot_name.clone()).or_default();

                // Get selected source
                let selected_idx = widgets.source_dropdown.selected() as usize;
                if selected_idx < widgets.source_ids.len() {
                    slot_config.source_id = widgets.source_ids[selected_idx].clone();
                }

                // Get caption override
                slot_config.caption_override = widgets.caption_entry.text().to_string();

                // Get source-specific config from embedded widget
                if let Some(ref source_widget) = *widgets.source_config_widget.borrow() {
                    if let Some(config_value) = source_widget.get_config_json() {
                        // Convert Value to HashMap<String, Value>
                        if let Some(obj) = config_value.as_object() {
                            slot_config.source_config = obj.iter()
                                .map(|(k, v)| (k.clone(), v.clone()))
                                .collect();
                        }
                    }
                }
            }
        }

        // Clear existing tabs
        while notebook.n_pages() > 0 {
            notebook.remove_page(Some(0));
        }
        slot_widgets.borrow_mut().clear();

        let cfg = config.borrow();

        // Get available sources (excluding combination - it would cause recursion)
        let registry = global_registry();
        let all_sources = registry.list_sources_with_info();
        let available_sources: Vec<_> = all_sources
            .into_iter()
            .filter(|s| s.id != "combination")
            .collect();

        // Add "None" option at the start
        let mut source_ids = vec!["none".to_string()];
        let mut source_names = vec!["None".to_string()];
        for source in &available_sources {
            source_ids.push(source.id.clone());
            source_names.push(source.display_name.clone());
        }

        // Collect group info before releasing borrow
        let groups_info: Vec<(usize, u32)> = cfg.groups.iter()
            .enumerate()
            .map(|(idx, g)| (idx + 1, g.item_count))
            .collect();
        drop(cfg);

        // Create a tab for each group with nested items notebook
        for (group_num, item_count) in groups_info {
            Self::create_group_tab(
                notebook,
                slot_widgets,
                on_change,
                config,
                group_num,
                item_count,
                &source_ids,
                &source_names,
            );
        }
    }

    /// Create a group tab containing an item count spinner and nested items notebook
    fn create_group_tab(
        parent_notebook: &Notebook,
        slot_widgets: &Rc<RefCell<HashMap<String, SlotWidgets>>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        config: &Rc<RefCell<ComboSourceConfig>>,
        group_num: usize,
        item_count: u32,
        source_ids: &[String],
        source_names: &[String],
    ) {
        let group_box = GtkBox::new(Orientation::Vertical, 8);
        group_box.set_margin_start(8);
        group_box.set_margin_end(8);
        group_box.set_margin_top(8);
        group_box.set_margin_bottom(8);

        // Item count spinner row
        let item_count_box = GtkBox::new(Orientation::Horizontal, 8);
        let item_count_label = Label::new(Some("Items in this group:"));
        let item_count_spin = SpinButton::with_range(1.0, 8.0, 1.0);
        item_count_spin.set_value(item_count as f64);
        item_count_box.append(&item_count_label);
        item_count_box.append(&item_count_spin);
        group_box.append(&item_count_box);

        // Nested notebook for items
        let items_notebook = Notebook::new();
        items_notebook.set_scrollable(true);
        items_notebook.set_vexpand(true);

        // Create tabs for each item in this group
        for item_idx in 1..=item_count {
            let slot_name = format!("group{}_{}", group_num, item_idx);
            let tab_label = format!("Item {}", item_idx);
            Self::create_slot_tab(
                &items_notebook,
                slot_widgets,
                on_change,
                config,
                &slot_name,
                &tab_label,
                source_ids,
                source_names,
            );
        }

        group_box.append(&items_notebook);

        // Connect item count spinner change
        {
            let config_clone = config.clone();
            let items_notebook_clone = items_notebook.clone();
            let slot_widgets_clone = slot_widgets.clone();
            let on_change_clone = on_change.clone();
            let source_ids_clone = source_ids.to_vec();
            let source_names_clone = source_names.to_vec();
            let group_num_copy = group_num;

            item_count_spin.connect_value_changed(move |spin| {
                let new_item_count = spin.value() as u32;
                let group_idx = group_num_copy - 1;

                // Update config
                {
                    let mut cfg = config_clone.borrow_mut();
                    if group_idx < cfg.groups.len() {
                        cfg.groups[group_idx].item_count = new_item_count;
                    }
                }

                // Rebuild items in this group's notebook
                Self::rebuild_items_notebook(
                    &items_notebook_clone,
                    &slot_widgets_clone,
                    &on_change_clone,
                    &config_clone,
                    group_num_copy,
                    new_item_count,
                    &source_ids_clone,
                    &source_names_clone,
                );

                if let Some(cb) = on_change_clone.borrow().as_ref() {
                    cb();
                }
            });
        }

        // Add group tab to parent notebook
        let tab_label = Label::new(Some(&format!("Group {}", group_num)));
        parent_notebook.append_page(&group_box, Some(&tab_label));
    }

    /// Rebuild items notebook when item count changes
    fn rebuild_items_notebook(
        notebook: &Notebook,
        slot_widgets: &Rc<RefCell<HashMap<String, SlotWidgets>>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        config: &Rc<RefCell<ComboSourceConfig>>,
        group_num: usize,
        item_count: u32,
        source_ids: &[String],
        source_names: &[String],
    ) {
        // Save current values for this group's slots before clearing
        {
            let mut cfg = config.borrow_mut();
            let slot_widgets_ref = slot_widgets.borrow();
            for item_idx in 1..=8 {
                let slot_name = format!("group{}_{}", group_num, item_idx);
                if let Some(widgets) = slot_widgets_ref.get(&slot_name) {
                    let slot_config = cfg.slots.entry(slot_name.clone()).or_default();

                    let selected_idx = widgets.source_dropdown.selected() as usize;
                    if selected_idx < widgets.source_ids.len() {
                        slot_config.source_id = widgets.source_ids[selected_idx].clone();
                    }
                    slot_config.caption_override = widgets.caption_entry.text().to_string();

                    if let Some(ref source_widget) = *widgets.source_config_widget.borrow() {
                        if let Some(config_value) = source_widget.get_config_json() {
                            if let Some(obj) = config_value.as_object() {
                                slot_config.source_config = obj.iter()
                                    .map(|(k, v)| (k.clone(), v.clone()))
                                    .collect();
                            }
                        }
                    }
                }
            }
        }

        // Clear existing tabs
        while notebook.n_pages() > 0 {
            notebook.remove_page(Some(0));
        }

        // Remove this group's slot widgets
        {
            let mut sw = slot_widgets.borrow_mut();
            let keys_to_remove: Vec<String> = sw.keys()
                .filter(|k| k.starts_with(&format!("group{}_", group_num)))
                .cloned()
                .collect();
            for key in keys_to_remove {
                sw.remove(&key);
            }
        }

        // Create tabs for new item count
        for item_idx in 1..=item_count {
            let slot_name = format!("group{}_{}", group_num, item_idx);
            let tab_label = format!("Item {}", item_idx);
            Self::create_slot_tab(
                notebook,
                slot_widgets,
                on_change,
                config,
                &slot_name,
                &tab_label,
                source_ids,
                source_names,
            );
        }
    }

    /// Create the appropriate source config widget for a source type
    fn create_source_config_widget(source_id: &str) -> Option<SourceConfigWidgetType> {
        match source_id {
            "cpu" => Some(SourceConfigWidgetType::Cpu(CpuSourceConfigWidget::new())),
            "gpu" => {
                let gpu_widget = GpuSourceConfigWidget::new();
                // Populate with detected GPUs
                let gpu_names: Vec<String> = crate::sources::GpuSource::get_cached_gpu_names().to_vec();
                gpu_widget.set_available_gpus(&gpu_names);
                Some(SourceConfigWidgetType::Gpu(gpu_widget))
            }
            "memory" => Some(SourceConfigWidgetType::Memory(MemorySourceConfigWidget::new())),
            "system_temp" => Some(SourceConfigWidgetType::SystemTemp(SystemTempConfigWidget::new())),
            "fan_speed" => Some(SourceConfigWidgetType::FanSpeed(FanSpeedConfigWidget::new())),
            "disk" => Some(SourceConfigWidgetType::Disk(DiskSourceConfigWidget::new())),
            "clock" => Some(SourceConfigWidgetType::Clock(ClockSourceConfigWidget::new())),
            "static_text" => Some(SourceConfigWidgetType::StaticText(StaticTextConfigWidget::new())),
            _ => None,
        }
    }

    fn create_slot_tab(
        notebook: &Notebook,
        slot_widgets: &Rc<RefCell<HashMap<String, SlotWidgets>>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        config: &Rc<RefCell<ComboSourceConfig>>,
        slot_name: &str,
        tab_label: &str,
        source_ids: &[String],
        source_names: &[String],
    ) {
        // Create scrolled window for each tab to handle long config widgets
        let tab_scrolled = ScrolledWindow::new();
        tab_scrolled.set_vexpand(true);
        tab_scrolled.set_hexpand(true);

        let tab_box = GtkBox::new(Orientation::Vertical, 8);
        tab_box.set_margin_start(12);
        tab_box.set_margin_end(12);
        tab_box.set_margin_top(12);
        tab_box.set_margin_bottom(12);

        // Source selection row
        let source_row = GtkBox::new(Orientation::Horizontal, 8);
        let source_label = Label::new(Some("Data Source:"));
        source_label.set_width_chars(12);
        source_label.set_xalign(0.0);

        let source_strings: Vec<&str> = source_names.iter().map(|s| s.as_str()).collect();
        let source_list = StringList::new(&source_strings);
        let source_dropdown = DropDown::new(Some(source_list), Option::<gtk4::Expression>::None);
        source_dropdown.set_hexpand(true);

        // Get current slot config
        let cfg = config.borrow();
        let current_source_id = cfg.slots.get(slot_name)
            .map(|s| s.source_id.clone())
            .unwrap_or_default();
        let current_source_config = cfg.slots.get(slot_name)
            .map(|s| s.source_config.clone())
            .unwrap_or_default();
        log::info!("create_slot_tab '{}': source_id='{}', source_config keys={:?}",
            slot_name, current_source_id, current_source_config.keys().collect::<Vec<_>>());

        let selected_idx = source_ids
            .iter()
            .position(|id| id == &current_source_id)
            .unwrap_or(0);
        source_dropdown.set_selected(selected_idx as u32);
        drop(cfg);

        source_row.append(&source_label);
        source_row.append(&source_dropdown);
        tab_box.append(&source_row);

        // Caption override row
        let caption_row = GtkBox::new(Orientation::Horizontal, 8);
        let caption_label = Label::new(Some("Caption:"));
        caption_label.set_width_chars(12);
        caption_label.set_xalign(0.0);

        let caption_entry = Entry::new();
        caption_entry.set_placeholder_text(Some("(Use default from source)"));
        caption_entry.set_hexpand(true);

        // Set current caption if configured
        let cfg = config.borrow();
        if let Some(slot_config) = cfg.slots.get(slot_name) {
            caption_entry.set_text(&slot_config.caption_override);
        }
        drop(cfg);

        caption_row.append(&caption_label);
        caption_row.append(&caption_entry);
        tab_box.append(&caption_row);

        // Source configuration frame with copy/paste buttons
        let config_header_box = GtkBox::new(Orientation::Horizontal, 6);
        config_header_box.set_margin_top(12);
        let config_header_label = Label::new(Some("Source Configuration"));
        config_header_label.set_hexpand(true);
        config_header_label.set_halign(gtk4::Align::Start);
        let copy_source_btn = gtk4::Button::with_label("Copy");
        let paste_source_btn = gtk4::Button::with_label("Paste");
        config_header_box.append(&config_header_label);
        config_header_box.append(&copy_source_btn);
        config_header_box.append(&paste_source_btn);
        tab_box.append(&config_header_box);

        let config_frame = Frame::new(None::<&str>);

        let config_container = GtkBox::new(Orientation::Vertical, 0);
        config_frame.set_child(Some(&config_container));
        tab_box.append(&config_frame);

        // Create initial source config widget if source is selected
        let source_config_widget: Rc<RefCell<Option<SourceConfigWidgetType>>> =
            Rc::new(RefCell::new(None));

        if !current_source_id.is_empty() && current_source_id != "none" {
            if let Some(source_widget) = Self::create_source_config_widget(&current_source_id) {
                // Load existing config
                source_widget.set_config_from_json(&current_source_config);

                let widget_gtk = source_widget.widget();
                config_container.append(&widget_gtk);
                *source_config_widget.borrow_mut() = Some(source_widget);
            } else {
                let no_config_label = Label::new(Some("No additional configuration available for this source."));
                no_config_label.set_halign(gtk4::Align::Start);
                no_config_label.set_margin_start(12);
                no_config_label.set_margin_top(8);
                no_config_label.set_margin_bottom(8);
                no_config_label.add_css_class("dim-label");
                config_container.append(&no_config_label);
            }
        } else {
            let select_source_label = Label::new(Some("Select a data source above to see configuration options."));
            select_source_label.set_halign(gtk4::Align::Start);
            select_source_label.set_margin_start(12);
            select_source_label.set_margin_top(8);
            select_source_label.set_margin_bottom(8);
            select_source_label.add_css_class("dim-label");
            config_container.append(&select_source_label);
        }

        // Copy source config handler
        {
            let source_config_widget_clone = source_config_widget.clone();
            let source_dropdown_clone = source_dropdown.clone();
            let source_ids_clone = source_ids.to_vec();
            copy_source_btn.connect_clicked(move |_| {
                if let Some(ref widget) = *source_config_widget_clone.borrow() {
                    let selected_idx = source_dropdown_clone.selected() as usize;
                    if selected_idx < source_ids_clone.len() {
                        let source_id = source_ids_clone[selected_idx].clone();
                        if let Some(config_json) = widget.get_config_json() {
                            if let Ok(mut clipboard) = crate::ui::clipboard::CLIPBOARD.lock() {
                                clipboard.copy_source_config(source_id, config_json);
                            }
                        }
                    }
                }
            });
        }

        // Paste source config handler
        {
            let source_config_widget_clone = source_config_widget.clone();
            let source_dropdown_clone = source_dropdown.clone();
            let source_ids_clone = source_ids.to_vec();
            let on_change_clone = on_change.clone();
            paste_source_btn.connect_clicked(move |_| {
                let pasted = if let Ok(clipboard) = crate::ui::clipboard::CLIPBOARD.lock() {
                    clipboard.paste_source_config()
                } else {
                    None
                };

                if let Some((pasted_source_type, pasted_config)) = pasted {
                    let selected_idx = source_dropdown_clone.selected() as usize;
                    if selected_idx < source_ids_clone.len() {
                        let current_source_id = &source_ids_clone[selected_idx];
                        // Only paste if source types match
                        if current_source_id == &pasted_source_type {
                            if let Some(ref widget) = *source_config_widget_clone.borrow() {
                                // Convert Value to HashMap<String, Value>
                                if let Some(obj) = pasted_config.as_object() {
                                    let config_map: HashMap<String, Value> = obj
                                        .iter()
                                        .map(|(k, v)| (k.clone(), v.clone()))
                                        .collect();
                                    widget.set_config_from_json(&config_map);
                                    // Trigger on_change
                                    if let Some(ref callback) = *on_change_clone.borrow() {
                                        callback();
                                    }
                                }
                            }
                        } else {
                            log::info!("Source config paste skipped: clipboard has '{}' but slot has '{}'",
                                pasted_source_type, current_source_id);
                        }
                    }
                }
            });
        }

        // Connect source dropdown change handler
        {
            let on_change_clone = on_change.clone();
            let config_clone = config.clone();
            let slot_name_clone = slot_name.to_string();
            let source_ids_clone = source_ids.to_vec();
            let config_container_clone = config_container.clone();
            let source_config_widget_clone = source_config_widget.clone();

            source_dropdown.connect_selected_notify(move |dropdown| {
                let selected_idx = dropdown.selected() as usize;
                if selected_idx < source_ids_clone.len() {
                    let source_id = &source_ids_clone[selected_idx];

                    // Update config
                    {
                        let mut cfg = config_clone.borrow_mut();
                        let slot_config = cfg.slots.entry(slot_name_clone.clone()).or_default();
                        slot_config.source_id = source_id.clone();
                        // Clear old source config when source changes
                        slot_config.source_config.clear();
                    }

                    // Clear config container
                    while let Some(child) = config_container_clone.first_child() {
                        config_container_clone.remove(&child);
                    }

                    // Create new source config widget
                    if source_id != "none" && !source_id.is_empty() {
                        if let Some(source_widget) = Self::create_source_config_widget(source_id) {
                            let widget_gtk = source_widget.widget();
                            config_container_clone.append(&widget_gtk);
                            *source_config_widget_clone.borrow_mut() = Some(source_widget);
                        } else {
                            let no_config_label = Label::new(Some("No additional configuration available for this source."));
                            no_config_label.set_halign(gtk4::Align::Start);
                            no_config_label.set_margin_start(12);
                            no_config_label.set_margin_top(8);
                            no_config_label.set_margin_bottom(8);
                            no_config_label.add_css_class("dim-label");
                            config_container_clone.append(&no_config_label);
                            *source_config_widget_clone.borrow_mut() = None;
                        }
                    } else {
                        let select_source_label = Label::new(Some("Select a data source above to see configuration options."));
                        select_source_label.set_halign(gtk4::Align::Start);
                        select_source_label.set_margin_start(12);
                        select_source_label.set_margin_top(8);
                        select_source_label.set_margin_bottom(8);
                        select_source_label.add_css_class("dim-label");
                        config_container_clone.append(&select_source_label);
                        *source_config_widget_clone.borrow_mut() = None;
                    }

                    if let Some(cb) = on_change_clone.borrow().as_ref() {
                        cb();
                    }
                }
            });
        }

        // Connect caption entry change handler
        {
            let on_change_clone = on_change.clone();
            let config_clone = config.clone();
            let slot_name_clone = slot_name.to_string();

            caption_entry.connect_changed(move |entry| {
                let text = entry.text().to_string();
                {
                    let mut cfg = config_clone.borrow_mut();
                    let slot_config = cfg.slots.entry(slot_name_clone.clone()).or_default();
                    slot_config.caption_override = text;
                }
                if let Some(cb) = on_change_clone.borrow().as_ref() {
                    cb();
                }
            });
        }

        // Store widgets for later access
        slot_widgets.borrow_mut().insert(
            slot_name.to_string(),
            SlotWidgets {
                source_dropdown: source_dropdown.clone(),
                caption_entry: caption_entry.clone(),
                source_ids: source_ids.to_vec(),
                config_container,
                source_config_widget,
            },
        );

        tab_scrolled.set_child(Some(&tab_box));

        // Add tab to notebook
        let label = Label::new(Some(tab_label));
        notebook.append_page(&tab_scrolled, Some(&label));
    }

    /// Get a summary of configured sources for use in display type tabs
    /// Returns Vec of (slot_name, source_summary, group_num, item_idx)
    pub fn get_source_summaries(&self) -> Vec<(String, String, usize, u32)> {
        let config = self.config.borrow();
        let mut summaries = Vec::new();

        log::debug!(
            "ComboSourceConfigWidget::get_source_summaries - config has {} groups",
            config.groups.len()
        );

        // Iterate through groups and items
        for (group_idx, group) in config.groups.iter().enumerate() {
            let group_num = group_idx + 1;
            for item_idx in 1..=group.item_count {
                let slot_name = format!("group{}_{}", group_num, item_idx);
                let summary = if let Some(slot_config) = config.slots.get(&slot_name) {
                    if slot_config.source_id.is_empty() || slot_config.source_id == "none" {
                        "(Not configured)".to_string()
                    } else if !slot_config.caption_override.is_empty() {
                        slot_config.caption_override.clone()
                    } else {
                        // Get display name from registry
                        global_registry()
                            .get_source_info(&slot_config.source_id)
                            .map(|s| s.display_name)
                            .unwrap_or_else(|| slot_config.source_id.clone())
                    }
                } else {
                    "(Not configured)".to_string()
                };
                summaries.push((slot_name, summary, group_num, item_idx));
            }
        }

        summaries
    }

    /// Get the group configuration (group count and item counts per group)
    pub fn get_groups_info(&self) -> Vec<(usize, u32)> {
        let config = self.config.borrow();
        config.groups.iter()
            .enumerate()
            .map(|(idx, g)| (idx + 1, g.item_count))
            .collect()
    }

    /// Get the available fields from the configured combo source
    /// Creates a temporary source instance to get the actual fields from child sources
    pub fn get_available_fields(&self) -> Vec<crate::core::FieldMetadata> {
        use crate::core::DataSource;
        use crate::sources::ComboSource;
        use serde_json::Value;
        use std::collections::HashMap;

        let config = self.config.borrow().clone();
        let mut source = ComboSource::new();

        // Configure the source
        let combo_config_value = serde_json::to_value(&config).unwrap_or(Value::Null);
        let mut config_map: HashMap<String, Value> = HashMap::new();
        config_map.insert("combo_config".to_string(), combo_config_value);
        if source.configure(&config_map).is_ok() {
            source.fields()
        } else {
            Vec::new()
        }
    }
}

impl Default for ComboSourceConfigWidget {
    fn default() -> Self {
        Self::new()
    }
}
