//! Configuration widget for the Combination source
//!
//! Provides UI for configuring multiple data sources in a combo/combination panel.
//! Each slot can have its own source type and source-specific configuration.

use crate::core::global_registry;
use crate::sources::ComboSourceConfig;
use crate::ui::{
    ClockSourceConfigWidget, CpuSourceConfigWidget, DiskSourceConfigWidget, FanSpeedConfigWidget,
    GpuSourceConfigWidget, MemorySourceConfigWidget, NetworkSourceConfigWidget,
    StaticTextConfigWidget, SystemTempConfigWidget, TestSourceConfigWidget,
};
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, DropDown, Entry, Frame, Label, Notebook, Orientation, ScrolledWindow,
    SpinButton, StringList, Widget,
};
use serde_json::Value;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

/// Debounce delay for spinner changes (milliseconds)
const SPINNER_DEBOUNCE_MS: u64 = 150;

/// Enum to hold different source config widget types
enum SourceConfigWidgetType {
    Cpu(CpuSourceConfigWidget),
    Gpu(GpuSourceConfigWidget),
    Memory(MemorySourceConfigWidget),
    SystemTemp(SystemTempConfigWidget),
    FanSpeed(FanSpeedConfigWidget),
    Disk(DiskSourceConfigWidget),
    Network(NetworkSourceConfigWidget),
    Clock(ClockSourceConfigWidget),
    StaticText(StaticTextConfigWidget),
    Test(TestSourceConfigWidget),
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
            SourceConfigWidgetType::Network(w) => w.widget().clone().upcast(),
            SourceConfigWidgetType::Clock(w) => w.widget().clone().upcast(),
            SourceConfigWidgetType::StaticText(w) => w.widget().clone().upcast(),
            SourceConfigWidgetType::Test(w) => w.widget().clone().upcast(),
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
            SourceConfigWidgetType::Network(w) => serde_json::to_value(w.get_config()).ok(),
            SourceConfigWidgetType::Clock(w) => serde_json::to_value(w.get_config()).ok(),
            SourceConfigWidgetType::StaticText(w) => serde_json::to_value(w.get_config()).ok(),
            SourceConfigWidgetType::Test(w) => serde_json::to_value(w.get_config()).ok(),
        }
    }

    fn set_config_from_json(&self, config: &HashMap<String, Value>) {
        if config.is_empty() {
            log::info!("set_config_from_json: empty config, skipping");
            return;
        }

        let json_value = serde_json::to_value(config).unwrap_or_default();
        log::info!(
            "set_config_from_json: loading config with {} keys",
            config.len()
        );

        match self {
            SourceConfigWidgetType::Cpu(w) => match serde_json::from_value(json_value) {
                Ok(cfg) => {
                    log::info!("Successfully loaded CPU config");
                    w.set_config(cfg);
                }
                Err(e) => log::warn!("Failed to deserialize CPU config: {}", e),
            },
            SourceConfigWidgetType::Gpu(w) => match serde_json::from_value(json_value) {
                Ok(cfg) => {
                    log::info!("Successfully loaded GPU config");
                    w.set_config(cfg);
                }
                Err(e) => log::warn!("Failed to deserialize GPU config: {}", e),
            },
            SourceConfigWidgetType::Memory(w) => match serde_json::from_value(json_value) {
                Ok(cfg) => {
                    log::info!("Successfully loaded Memory config");
                    w.set_config(cfg);
                }
                Err(e) => log::warn!("Failed to deserialize Memory config: {}", e),
            },
            SourceConfigWidgetType::SystemTemp(w) => match serde_json::from_value(json_value) {
                Ok(cfg) => {
                    log::info!("Successfully loaded SystemTemp config");
                    w.set_config(cfg);
                }
                Err(e) => log::warn!("Failed to deserialize SystemTemp config: {}", e),
            },
            SourceConfigWidgetType::FanSpeed(w) => match serde_json::from_value(json_value) {
                Ok(cfg) => {
                    log::info!("Successfully loaded FanSpeed config");
                    w.set_config(&cfg);
                }
                Err(e) => log::warn!("Failed to deserialize FanSpeed config: {}", e),
            },
            SourceConfigWidgetType::Disk(w) => match serde_json::from_value(json_value) {
                Ok(cfg) => {
                    log::info!("Successfully loaded Disk config");
                    w.set_config(cfg);
                }
                Err(e) => log::warn!("Failed to deserialize Disk config: {}", e),
            },
            SourceConfigWidgetType::Network(w) => match serde_json::from_value(json_value) {
                Ok(cfg) => {
                    log::info!("Successfully loaded Network config");
                    w.set_config(cfg);
                }
                Err(e) => log::warn!("Failed to deserialize Network config: {}", e),
            },
            SourceConfigWidgetType::Clock(w) => match serde_json::from_value(json_value) {
                Ok(cfg) => {
                    log::info!("Successfully loaded Clock config");
                    w.set_config(&cfg);
                }
                Err(e) => log::warn!("Failed to deserialize Clock config: {}", e),
            },
            SourceConfigWidgetType::StaticText(w) => match serde_json::from_value(json_value) {
                Ok(cfg) => {
                    log::info!("Successfully loaded StaticText config");
                    w.set_config(&cfg);
                }
                Err(e) => log::warn!("Failed to deserialize StaticText config: {}", e),
            },
            SourceConfigWidgetType::Test(w) => match serde_json::from_value(json_value) {
                Ok(cfg) => {
                    log::info!("Successfully loaded Test config");
                    w.set_config(&cfg);
                }
                Err(e) => log::warn!("Failed to deserialize Test config: {}", e),
            },
        }
    }
}

/// Cached source information to avoid repeated registry lookups
struct CachedSources {
    source_ids: Vec<String>,
    source_names: Vec<String>,
}

impl CachedSources {
    fn new() -> Self {
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

        Self {
            source_ids,
            source_names,
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
    /// Cached source registry info (computed once)
    cached_sources: Rc<CachedSources>,
    /// Debounce counter for group count spinner
    group_debounce_id: Rc<Cell<u32>>,
    /// Generation counter to cancel stale async operations
    rebuild_generation: Rc<Cell<u32>>,
    /// Cached fields (updated asynchronously when sources change)
    cached_fields: Rc<RefCell<Vec<crate::core::FieldMetadata>>>,
    /// Generation counter for field updates (to cancel stale updates)
    fields_generation: Rc<Cell<u32>>,
    /// Debounce counter for field updates (to coalesce rapid updates)
    fields_debounce_id: Rc<Cell<u32>>,
    /// Callback to invoke when fields are updated asynchronously
    on_fields_updated: Rc<RefCell<Option<Box<dyn Fn(Vec<crate::core::FieldMetadata>)>>>>,
    /// Flag to indicate widget is destroyed - checked by async callbacks to abort early
    /// This prevents memory leaks from async callbacks holding Rc references after dialog closes
    destroyed: Rc<Cell<bool>>,
}

/// Widgets for a single slot configuration
#[allow(dead_code)]
struct SlotWidgets {
    source_dropdown: DropDown,
    caption_entry: Entry,
    source_ids: Vec<String>,
    config_container: GtkBox,
    source_config_widget: Rc<RefCell<Option<SourceConfigWidgetType>>>,
    /// ScrolledWindow for this slot tab (needed for disconnecting map handler)
    tab_scrolled: ScrolledWindow,
    /// Signal handler ID for the lazy initialization connect_map callback
    map_handler_id: Option<glib::SignalHandlerId>,
}

impl ComboSourceConfigWidget {
    /// Create a new widget with default config (builds tabs once)
    pub fn new() -> Self {
        Self::create_internal(ComboSourceConfig::default())
    }

    /// Create a new widget with the given config (builds tabs once, avoiding double-build)
    /// Use this when you already have a config to load - it's more efficient than
    /// calling new() followed by set_config().
    pub fn with_config(config: ComboSourceConfig) -> Self {
        Self::create_internal(config)
    }

    /// Internal constructor - builds the widget with the given config exactly once
    fn create_internal(mut initial_config: ComboSourceConfig) -> Self {
        // Migrate legacy format if needed
        initial_config.migrate_legacy();

        let config = Rc::new(RefCell::new(initial_config));
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

        // Cache source registry info once (avoid repeated lookups)
        let cached_sources = Rc::new(CachedSources::new());

        // Debounce counter for group count spinner
        let group_debounce_id = Rc::new(Cell::new(0u32));

        // Generation counter to cancel stale async operations
        let rebuild_generation = Rc::new(Cell::new(0u32));

        // Cached fields and generation counter
        let cached_fields = Rc::new(RefCell::new(Vec::new()));
        let fields_generation = Rc::new(Cell::new(0u32));
        let fields_debounce_id = Rc::new(Cell::new(0u32));
        let on_fields_updated: Rc<RefCell<Option<Box<dyn Fn(Vec<crate::core::FieldMetadata>)>>>> =
            Rc::new(RefCell::new(None));

        // Destroyed flag - set when widget is destroyed to cancel async callbacks
        let destroyed = Rc::new(Cell::new(false));

        let widget = Self {
            container,
            config,
            group_count_spin,
            update_interval_spin,
            notebook,
            slot_widgets,
            on_change,
            cached_sources,
            group_debounce_id,
            rebuild_generation,
            cached_fields,
            fields_generation,
            fields_debounce_id,
            on_fields_updated,
            destroyed,
        };

        // Connect unrealize signal to cancel async callbacks and prevent memory leaks.
        // In GTK4, connect_destroy doesn't fire reliably when a dialog is closed.
        // connect_unrealize fires when the widget's GDK resources are released,
        // which happens when the dialog is closed and its widget tree is torn down.
        {
            let destroyed_clone = widget.destroyed.clone();
            let rebuild_gen = widget.rebuild_generation.clone();
            let fields_gen = widget.fields_generation.clone();
            let fields_debounce = widget.fields_debounce_id.clone();
            widget.container.connect_unrealize(move |_| {
                log::debug!("ComboSourceConfigWidget unrealized - cancelling async operations");
                destroyed_clone.set(true);
                // Increment generation and debounce counters to cancel any pending async operations
                rebuild_gen.set(rebuild_gen.get().wrapping_add(1));
                fields_gen.set(fields_gen.get().wrapping_add(1));
                fields_debounce.set(fields_debounce.get().wrapping_add(1));
            });
        }

        // Build tabs once with the initial config
        widget.rebuild_tabs();

        // Connect group count spinner change with debouncing
        {
            let config_clone = widget.config.clone();
            let notebook_clone = widget.notebook.clone();
            let slot_widgets_clone = widget.slot_widgets.clone();
            let on_change_clone = widget.on_change.clone();
            let cached_sources_clone = widget.cached_sources.clone();
            let debounce_id = widget.group_debounce_id.clone();
            let rebuild_generation_clone = widget.rebuild_generation.clone();

            widget.group_count_spin.connect_value_changed(move |spin| {
                let new_count = spin.value() as usize;

                // Update config immediately (cheap operation)
                {
                    let mut cfg = config_clone.borrow_mut();
                    while cfg.groups.len() < new_count {
                        cfg.groups.push(crate::sources::GroupConfig {
                            item_count: 2,
                            ..Default::default()
                        });
                    }
                    while cfg.groups.len() > new_count {
                        cfg.groups.pop();
                    }
                }

                // Debounce the expensive rebuild operation
                let current_id = debounce_id.get().wrapping_add(1);
                debounce_id.set(current_id);

                let config_for_rebuild = config_clone.clone();
                let notebook_for_rebuild = notebook_clone.clone();
                let slot_widgets_for_rebuild = slot_widgets_clone.clone();
                let on_change_for_rebuild = on_change_clone.clone();
                let cached_for_rebuild = cached_sources_clone.clone();
                let debounce_check = debounce_id.clone();
                let rebuild_gen_for_rebuild = rebuild_generation_clone.clone();

                glib::timeout_add_local_once(
                    Duration::from_millis(SPINNER_DEBOUNCE_MS),
                    move || {
                        // Only rebuild if this is still the latest change
                        if debounce_check.get() == current_id {
                            // Increment generation to cancel any pending async operations
                            let generation = rebuild_gen_for_rebuild.get().wrapping_add(1);
                            rebuild_gen_for_rebuild.set(generation);

                            Self::rebuild_tabs_internal(
                                &config_for_rebuild,
                                &notebook_for_rebuild,
                                &slot_widgets_for_rebuild,
                                &on_change_for_rebuild,
                                &cached_for_rebuild,
                                &rebuild_gen_for_rebuild,
                                generation,
                            );
                            if let Some(cb) = on_change_for_rebuild.borrow().as_ref() {
                                cb();
                            }
                        }
                    },
                );
            });
        }

        // Connect update interval spin button
        {
            let config_for_interval = widget.config.clone();
            let on_change_for_interval = widget.on_change.clone();
            widget
                .update_interval_spin
                .connect_value_changed(move |spin| {
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

        log::info!(
            "ComboSourceConfigWidget::set_config called with {} groups, {} slots",
            config.groups.len(),
            config.slots.len()
        );
        for (slot_name, slot_cfg) in &config.slots {
            log::info!(
                "  Slot '{}': source='{}', source_config keys: {:?}",
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
        self.update_interval_spin
            .set_value(update_interval_ms as f64);

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
                        slot_config.source_config =
                            obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
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
        // Increment generation to cancel any pending async operations
        let generation = self.rebuild_generation.get().wrapping_add(1);
        self.rebuild_generation.set(generation);

        Self::rebuild_tabs_internal(
            &self.config,
            &self.notebook,
            &self.slot_widgets,
            &self.on_change,
            &self.cached_sources,
            &self.rebuild_generation,
            generation,
        );
    }

    fn rebuild_tabs_internal(
        config: &Rc<RefCell<ComboSourceConfig>>,
        notebook: &Notebook,
        slot_widgets: &Rc<RefCell<HashMap<String, SlotWidgets>>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        cached_sources: &Rc<CachedSources>,
        rebuild_generation: &Rc<Cell<u32>>,
        current_generation: u32,
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
                            slot_config.source_config =
                                obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
                        }
                    }
                }
            }
        }

        // Clear existing tabs - remove all pages at once
        while notebook.n_pages() > 0 {
            notebook.remove_page(Some(0));
        }
        slot_widgets.borrow_mut().clear();

        // Collect group info before releasing borrow
        let groups_info: Vec<(usize, u32)> = {
            let cfg = config.borrow();
            cfg.groups
                .iter()
                .enumerate()
                .map(|(idx, g)| (idx + 1, g.item_count))
                .collect()
        };

        // If no groups, nothing more to do
        if groups_info.is_empty() {
            return;
        }

        // Show loading indicator while creating tabs asynchronously
        let loading_box = GtkBox::new(Orientation::Vertical, 12);
        loading_box.set_halign(gtk4::Align::Center);
        loading_box.set_valign(gtk4::Align::Center);
        loading_box.set_vexpand(true);

        let spinner = gtk4::Spinner::new();
        spinner.set_size_request(32, 32);
        spinner.start();
        loading_box.append(&spinner);

        let loading_label = Label::new(Some("Loading groups..."));
        loading_label.add_css_class("dim-label");
        loading_box.append(&loading_label);

        // Add loading indicator as a temporary page
        let loading_tab_label = Label::new(Some("Loading..."));
        notebook.append_page(&loading_box, Some(&loading_tab_label));

        // Create tabs incrementally using idle callbacks to keep UI responsive
        let groups_queue = Rc::new(RefCell::new(
            groups_info
                .into_iter()
                .collect::<std::collections::VecDeque<_>>(),
        ));
        let config_clone = config.clone();
        let notebook_clone = notebook.clone();
        let slot_widgets_clone = slot_widgets.clone();
        let on_change_clone = on_change.clone();
        let cached_sources_clone = cached_sources.clone();
        let loading_box_clone = loading_box.clone();
        let is_first = Rc::new(Cell::new(true));
        let generation_ref = rebuild_generation.clone();

        glib::source::idle_add_local_full(glib::Priority::DEFAULT_IDLE, move || {
            // Check if this operation has been superseded by a newer rebuild
            if generation_ref.get() != current_generation {
                return glib::ControlFlow::Break;
            }

            // Get next group to create
            let next_group = groups_queue.borrow_mut().pop_front();

            if let Some((group_num, item_count)) = next_group {
                // Remove loading indicator on first actual tab creation
                if is_first.get() {
                    is_first.set(false);
                    // Remove the loading page
                    if let Some(page_num) = notebook_clone.page_num(&loading_box_clone) {
                        notebook_clone.remove_page(Some(page_num));
                    }
                }

                Self::create_group_tab(
                    &notebook_clone,
                    &slot_widgets_clone,
                    &on_change_clone,
                    &config_clone,
                    group_num,
                    item_count,
                    &cached_sources_clone.source_ids,
                    &cached_sources_clone.source_names,
                    &cached_sources_clone,
                );

                glib::ControlFlow::Continue
            } else {
                // All groups created - remove loading indicator if still present
                if let Some(page_num) = notebook_clone.page_num(&loading_box_clone) {
                    notebook_clone.remove_page(Some(page_num));
                }
                glib::ControlFlow::Break
            }
        });
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
        cached_sources: &Rc<CachedSources>,
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

        // Debounce counter for this group's item count spinner
        let item_debounce_id = Rc::new(Cell::new(0u32));
        // Generation counter for cancelling stale async operations
        let item_rebuild_generation = Rc::new(Cell::new(0u32));

        // Connect item count spinner change with debouncing
        {
            let config_clone = config.clone();
            let items_notebook_clone = items_notebook.clone();
            let slot_widgets_clone = slot_widgets.clone();
            let on_change_clone = on_change.clone();
            let cached_sources_clone = cached_sources.clone();
            let group_num_copy = group_num;
            let debounce_id = item_debounce_id.clone();
            let rebuild_gen = item_rebuild_generation.clone();

            item_count_spin.connect_value_changed(move |spin| {
                let new_item_count = spin.value() as u32;
                let group_idx = group_num_copy - 1;

                // Update config immediately (cheap operation)
                {
                    let mut cfg = config_clone.borrow_mut();
                    if group_idx < cfg.groups.len() {
                        cfg.groups[group_idx].item_count = new_item_count;
                    }
                }

                // Debounce the expensive rebuild operation
                let current_id = debounce_id.get().wrapping_add(1);
                debounce_id.set(current_id);

                let config_for_rebuild = config_clone.clone();
                let notebook_for_rebuild = items_notebook_clone.clone();
                let slot_widgets_for_rebuild = slot_widgets_clone.clone();
                let on_change_for_rebuild = on_change_clone.clone();
                let cached_for_rebuild = cached_sources_clone.clone();
                let debounce_check = debounce_id.clone();
                let rebuild_gen_for_rebuild = rebuild_gen.clone();

                glib::timeout_add_local_once(
                    Duration::from_millis(SPINNER_DEBOUNCE_MS),
                    move || {
                        // Only rebuild if this is still the latest change
                        if debounce_check.get() == current_id {
                            // Increment generation to cancel any pending async operations
                            let generation = rebuild_gen_for_rebuild.get().wrapping_add(1);
                            rebuild_gen_for_rebuild.set(generation);

                            Self::rebuild_items_notebook(
                                &notebook_for_rebuild,
                                &slot_widgets_for_rebuild,
                                &on_change_for_rebuild,
                                &config_for_rebuild,
                                group_num_copy,
                                new_item_count,
                                &cached_for_rebuild.source_ids,
                                &cached_for_rebuild.source_names,
                                &rebuild_gen_for_rebuild,
                                generation,
                            );

                            if let Some(cb) = on_change_for_rebuild.borrow().as_ref() {
                                cb();
                            }
                        }
                    },
                );
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
        rebuild_generation: &Rc<Cell<u32>>,
        current_generation: u32,
    ) {
        // Save current values for this group's slots before clearing
        // Only iterate over slots that actually exist in slot_widgets (not hardcoded 1-8)
        {
            let mut cfg = config.borrow_mut();
            let slot_widgets_ref = slot_widgets.borrow();
            let group_prefix = format!("group{}_", group_num);

            for (slot_name, widgets) in slot_widgets_ref.iter() {
                if slot_name.starts_with(&group_prefix) {
                    let slot_config = cfg.slots.entry(slot_name.clone()).or_default();

                    let selected_idx = widgets.source_dropdown.selected() as usize;
                    if selected_idx < widgets.source_ids.len() {
                        slot_config.source_id = widgets.source_ids[selected_idx].clone();
                    }
                    slot_config.caption_override = widgets.caption_entry.text().to_string();

                    if let Some(ref source_widget) = *widgets.source_config_widget.borrow() {
                        if let Some(config_value) = source_widget.get_config_json() {
                            if let Some(obj) = config_value.as_object() {
                                slot_config.source_config =
                                    obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
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
            let keys_to_remove: Vec<String> = sw
                .keys()
                .filter(|k| k.starts_with(&format!("group{}_", group_num)))
                .cloned()
                .collect();
            for key in keys_to_remove {
                sw.remove(&key);
            }
        }

        // If no items, nothing more to do
        if item_count == 0 {
            return;
        }

        // Show loading indicator while creating tabs asynchronously
        let loading_box = GtkBox::new(Orientation::Vertical, 12);
        loading_box.set_halign(gtk4::Align::Center);
        loading_box.set_valign(gtk4::Align::Center);
        loading_box.set_vexpand(true);

        let spinner = gtk4::Spinner::new();
        spinner.set_size_request(24, 24);
        spinner.start();
        loading_box.append(&spinner);

        let loading_label = Label::new(Some("Loading items..."));
        loading_label.add_css_class("dim-label");
        loading_box.append(&loading_label);

        // Add loading indicator as a temporary page
        let loading_tab_label = Label::new(Some("..."));
        notebook.append_page(&loading_box, Some(&loading_tab_label));

        // Create tabs incrementally using idle callbacks to keep UI responsive
        let items_queue = Rc::new(RefCell::new(
            (1..=item_count).collect::<std::collections::VecDeque<_>>(),
        ));
        let config_clone = config.clone();
        let notebook_clone = notebook.clone();
        let slot_widgets_clone = slot_widgets.clone();
        let on_change_clone = on_change.clone();
        let source_ids_clone = source_ids.to_vec();
        let source_names_clone = source_names.to_vec();
        let loading_box_clone = loading_box.clone();
        let is_first = Rc::new(Cell::new(true));
        let generation_ref = rebuild_generation.clone();

        glib::source::idle_add_local_full(glib::Priority::DEFAULT_IDLE, move || {
            // Check if this operation has been superseded by a newer rebuild
            if generation_ref.get() != current_generation {
                return glib::ControlFlow::Break;
            }

            // Get next item to create
            let next_item = items_queue.borrow_mut().pop_front();

            if let Some(item_idx) = next_item {
                // Remove loading indicator on first actual tab creation
                if is_first.get() {
                    is_first.set(false);
                    if let Some(page_num) = notebook_clone.page_num(&loading_box_clone) {
                        notebook_clone.remove_page(Some(page_num));
                    }
                }

                let slot_name = format!("group{}_{}", group_num, item_idx);
                let tab_label = format!("Item {}", item_idx);
                Self::create_slot_tab(
                    &notebook_clone,
                    &slot_widgets_clone,
                    &on_change_clone,
                    &config_clone,
                    &slot_name,
                    &tab_label,
                    &source_ids_clone,
                    &source_names_clone,
                );

                glib::ControlFlow::Continue
            } else {
                // All items created - remove loading indicator if still present
                if let Some(page_num) = notebook_clone.page_num(&loading_box_clone) {
                    notebook_clone.remove_page(Some(page_num));
                }
                glib::ControlFlow::Break
            }
        });
    }

    /// Create the appropriate source config widget for a source type
    fn create_source_config_widget(source_id: &str) -> Option<SourceConfigWidgetType> {
        match source_id {
            "cpu" => Some(SourceConfigWidgetType::Cpu(CpuSourceConfigWidget::new())),
            "gpu" => {
                let gpu_widget = GpuSourceConfigWidget::new();
                // Populate with detected GPUs
                let gpu_names: Vec<String> =
                    crate::sources::GpuSource::get_cached_gpu_names().to_vec();
                gpu_widget.set_available_gpus(&gpu_names);
                Some(SourceConfigWidgetType::Gpu(gpu_widget))
            }
            "memory" => Some(SourceConfigWidgetType::Memory(
                MemorySourceConfigWidget::new(),
            )),
            "system_temp" => Some(SourceConfigWidgetType::SystemTemp(
                SystemTempConfigWidget::new(),
            )),
            "fan_speed" => Some(SourceConfigWidgetType::FanSpeed(FanSpeedConfigWidget::new())),
            "disk" => {
                let disk_widget = DiskSourceConfigWidget::new();
                // Populate with available disks
                let disks = crate::sources::DiskSource::get_available_disks();
                disk_widget.set_available_disks(&disks);
                Some(SourceConfigWidgetType::Disk(disk_widget))
            }
            "network" => {
                let network_widget = NetworkSourceConfigWidget::new();
                // Populate with available network interfaces
                let interfaces = crate::sources::NetworkSource::get_available_interfaces();
                network_widget.set_available_interfaces(&interfaces);
                Some(SourceConfigWidgetType::Network(network_widget))
            }
            "clock" => Some(SourceConfigWidgetType::Clock(ClockSourceConfigWidget::new())),
            "static_text" => Some(SourceConfigWidgetType::StaticText(
                StaticTextConfigWidget::new(),
            )),
            "test" => Some(SourceConfigWidgetType::Test(TestSourceConfigWidget::new())),
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
        let current_source_id = cfg
            .slots
            .get(slot_name)
            .map(|s| s.source_id.clone())
            .unwrap_or_default();
        let current_source_config = cfg
            .slots
            .get(slot_name)
            .map(|s| s.source_config.clone())
            .unwrap_or_default();
        log::info!(
            "create_slot_tab '{}': source_id='{}', source_config keys={:?}",
            slot_name,
            current_source_id,
            current_source_config.keys().collect::<Vec<_>>()
        );

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

        // Create initial source config widget LAZILY - only show placeholder initially
        // Widget will be created when tab is mapped (becomes visible)
        let source_config_widget: Rc<RefCell<Option<SourceConfigWidgetType>>> =
            Rc::new(RefCell::new(None));

        // Track whether widget has been initialized
        let widget_initialized = Rc::new(RefCell::new(false));

        // Show placeholder initially
        let placeholder_label = Label::new(Some("Loading source configuration..."));
        placeholder_label.set_halign(gtk4::Align::Start);
        placeholder_label.set_margin_start(12);
        placeholder_label.set_margin_top(8);
        placeholder_label.set_margin_bottom(8);
        placeholder_label.add_css_class("dim-label");
        config_container.append(&placeholder_label);

        // Set up lazy initialization when tab becomes visible
        // Store the handler ID so we can disconnect it during cleanup to prevent memory leaks
        let map_handler_id = {
            let config_container_clone = config_container.clone();
            let source_config_widget_clone = source_config_widget.clone();
            let widget_initialized_clone = widget_initialized.clone();
            let current_source_id_clone = current_source_id.clone();
            let current_source_config_clone = current_source_config.clone();

            tab_scrolled.connect_map(move |_| {
                // Only initialize once
                if *widget_initialized_clone.borrow() {
                    return;
                }
                *widget_initialized_clone.borrow_mut() = true;

                // Remove placeholder
                while let Some(child) = config_container_clone.first_child() {
                    config_container_clone.remove(&child);
                }

                // Create the actual source config widget
                if !current_source_id_clone.is_empty() && current_source_id_clone != "none" {
                    if let Some(source_widget) =
                        Self::create_source_config_widget(&current_source_id_clone)
                    {
                        source_widget.set_config_from_json(&current_source_config_clone);
                        let widget_gtk = source_widget.widget();
                        config_container_clone.append(&widget_gtk);
                        *source_config_widget_clone.borrow_mut() = Some(source_widget);
                    } else {
                        let no_config_label = Label::new(Some(
                            "No additional configuration available for this source.",
                        ));
                        no_config_label.set_halign(gtk4::Align::Start);
                        no_config_label.set_margin_start(12);
                        no_config_label.set_margin_top(8);
                        no_config_label.set_margin_bottom(8);
                        no_config_label.add_css_class("dim-label");
                        config_container_clone.append(&no_config_label);
                    }
                } else {
                    let select_source_label = Label::new(Some(
                        "Select a data source above to see configuration options.",
                    ));
                    select_source_label.set_halign(gtk4::Align::Start);
                    select_source_label.set_margin_start(12);
                    select_source_label.set_margin_top(8);
                    select_source_label.set_margin_bottom(8);
                    select_source_label.add_css_class("dim-label");
                    config_container_clone.append(&select_source_label);
                }
            })
        };

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
                                    let config_map: HashMap<String, Value> =
                                        obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
                                    widget.set_config_from_json(&config_map);
                                    // Trigger on_change
                                    if let Some(ref callback) = *on_change_clone.borrow() {
                                        callback();
                                    }
                                }
                            }
                        } else {
                            log::info!(
                                "Source config paste skipped: clipboard has '{}' but slot has '{}'",
                                pasted_source_type,
                                current_source_id
                            );
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
            let widget_initialized_clone = widget_initialized.clone();

            source_dropdown.connect_selected_notify(move |dropdown| {
                // Mark as initialized since user is now interacting
                *widget_initialized_clone.borrow_mut() = true;
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
                            let no_config_label = Label::new(Some(
                                "No additional configuration available for this source.",
                            ));
                            no_config_label.set_halign(gtk4::Align::Start);
                            no_config_label.set_margin_start(12);
                            no_config_label.set_margin_top(8);
                            no_config_label.set_margin_bottom(8);
                            no_config_label.add_css_class("dim-label");
                            config_container_clone.append(&no_config_label);
                            *source_config_widget_clone.borrow_mut() = None;
                        }
                    } else {
                        let select_source_label = Label::new(Some(
                            "Select a data source above to see configuration options.",
                        ));
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
        // NOTE: We update config but do NOT trigger on_change to avoid expensive rebuilds on every keystroke.
        // The caption value is read when needed (get_source_summaries, get_config, save).
        {
            let config_clone = config.clone();
            let slot_name_clone = slot_name.to_string();

            caption_entry.connect_changed(move |entry| {
                let text = entry.text().to_string();
                let mut cfg = config_clone.borrow_mut();
                let slot_config = cfg.slots.entry(slot_name_clone.clone()).or_default();
                slot_config.caption_override = text;
            });
        }

        // Store widgets for later access (including tab_scrolled and map_handler_id for cleanup)
        slot_widgets.borrow_mut().insert(
            slot_name.to_string(),
            SlotWidgets {
                source_dropdown: source_dropdown.clone(),
                caption_entry: caption_entry.clone(),
                source_ids: source_ids.to_vec(),
                config_container,
                source_config_widget,
                tab_scrolled: tab_scrolled.clone(),
                map_handler_id: Some(map_handler_id),
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
        config
            .groups
            .iter()
            .enumerate()
            .map(|(idx, g)| (idx + 1, g.item_count))
            .collect()
    }

    /// Get the available fields from the cached fields
    /// Returns cached fields immediately (non-blocking). Call `update_fields_cache_async()`
    /// to refresh the cache when source configuration changes.
    pub fn get_available_fields(&self) -> Vec<crate::core::FieldMetadata> {
        self.cached_fields.borrow().clone()
    }

    /// Set a callback to be called when fields are updated asynchronously.
    /// This allows displayer config widgets to update their UI when field data arrives.
    pub fn set_on_fields_updated<F: Fn(Vec<crate::core::FieldMetadata>) + 'static>(
        &self,
        callback: F,
    ) {
        *self.on_fields_updated.borrow_mut() = Some(Box::new(callback));
    }

    /// Update the cached fields using the registry's field cache (lightweight).
    ///
    /// Instead of creating a full ComboSource with all child sources (which is expensive
    /// and uses 50-150MB), this method constructs the field list directly from the
    /// registry's cached field metadata. Each source type's fields are cached on first
    /// access, so subsequent calls are nearly instant.
    ///
    /// This method is debounced - rapid calls are coalesced into a single update.
    /// This prevents excessive UI updates when multiple dropdowns change at once
    /// (e.g., during initialization or when pasting configurations).
    pub fn update_fields_cache_async(&self) {
        log::info!("=== update_fields_cache_async CALLED (using cached fields) ===");
        // Debounce: increment counter and schedule delayed execution
        // If called again before delay, the counter changes and previous call is cancelled
        let debounce_id = self.fields_debounce_id.get().wrapping_add(1);
        self.fields_debounce_id.set(debounce_id);

        let config = self.config.clone();
        let cached_fields = self.cached_fields.clone();
        let fields_generation = self.fields_generation.clone();
        let fields_debounce_id = self.fields_debounce_id.clone();
        let on_fields_updated = self.on_fields_updated.clone();
        let destroyed = self.destroyed.clone();

        // Wait for debounce period before computing fields
        // This coalesces rapid calls (e.g., setting up 20 slots) into one
        glib::timeout_add_local_once(Duration::from_millis(100), move || {
            // Check if this is still the latest request
            if fields_debounce_id.get() != debounce_id {
                log::debug!(
                    "Skipping debounced fields update (id {} vs current {})",
                    debounce_id,
                    fields_debounce_id.get()
                );
                return;
            }

            // Check if widget is destroyed
            if destroyed.get() {
                log::debug!("Skipping fields update - widget destroyed");
                return;
            }

            // Increment generation for this actual update
            let generation = fields_generation.get().wrapping_add(1);
            fields_generation.set(generation);

            let config_snapshot = config.borrow().clone();
            let start = std::time::Instant::now();

            // Compute fields using cached field metadata from the registry
            // This is MUCH cheaper than creating full source instances
            let fields = Self::compute_fields_from_cache(&config_snapshot);

            log::info!(
                "=== Fields computed from cache in {:?} with {} fields ===",
                start.elapsed(),
                fields.len()
            );

            // Check if this update is still valid (not superseded by newer request)
            if fields_generation.get() != generation {
                log::debug!(
                    "Skipping stale fields update (gen {} vs current {})",
                    generation,
                    fields_generation.get()
                );
                return;
            }

            // Update the cache
            *cached_fields.borrow_mut() = fields.clone();

            // Notify listeners
            if let Some(ref callback) = *on_fields_updated.borrow() {
                callback(fields);
            }
        });
    }

    /// Compute combo source fields using the registry's cached field metadata.
    ///
    /// This is MUCH cheaper than creating a full ComboSource with all child sources,
    /// which would require creating sysinfo::System objects (50-100MB each).
    /// Instead, we use the registry's field cache which only creates each source
    /// type once and caches its field metadata.
    fn compute_fields_from_cache(
        config: &crate::sources::ComboSourceConfig,
    ) -> Vec<crate::core::FieldMetadata> {
        use crate::core::global_registry;

        let mut fields = Vec::new();
        let registry = global_registry();

        // Get slot names based on mode (matching ComboSource::get_slot_names())
        let slot_names: Vec<String> = match config.mode.as_str() {
            "lcars" => {
                let mut names = Vec::new();
                for (group_idx, group) in config.groups.iter().enumerate() {
                    let group_num = group_idx + 1;
                    for item_idx in 1..=group.item_count {
                        names.push(format!("group{}_{}", group_num, item_idx));
                    }
                }
                names
            }
            "arc" => {
                let mut names = vec!["center".to_string()];
                let arc_count = config.groups.first().map(|g| g.item_count).unwrap_or(4);
                for i in 1..=arc_count {
                    names.push(format!("arc{}", i));
                }
                names
            }
            "level_bar" => {
                let count = config.groups.first().map(|g| g.item_count).unwrap_or(4);
                (1..=count).map(|i| format!("bar{}", i)).collect()
            }
            _ => {
                // Default: groups with items
                let mut names = Vec::new();
                for (group_idx, group) in config.groups.iter().enumerate() {
                    let group_num = group_idx + 1;
                    for item_idx in 1..=group.item_count {
                        names.push(format!("group{}_{}", group_num, item_idx));
                    }
                }
                names
            }
        };

        // For each slot, get cached fields from the registry
        for slot_name in slot_names {
            if let Some(slot_config) = config.slots.get(&slot_name) {
                if !slot_config.source_id.is_empty() && slot_config.source_id != "none" {
                    let slot_fields = registry
                        .get_source_fields_for_combo_slot(&slot_name, &slot_config.source_id);
                    fields.extend(slot_fields);
                }
            }
        }

        fields
    }

    /// Compute available fields synchronously using cached field metadata.
    ///
    /// This is now cheap because it uses the registry's field cache instead of
    /// creating full source instances.
    pub fn compute_fields_sync(&self) -> Vec<crate::core::FieldMetadata> {
        let config = self.config.borrow().clone();
        Self::compute_fields_from_cache(&config)
    }

    /// Explicitly cancel all async operations and mark this widget as destroyed.
    /// Call this before dropping the widget to ensure async callbacks exit cleanly.
    /// This is important for preventing memory leaks when the parent dialog closes.
    pub fn cleanup(&self) {
        log::debug!("ComboSourceConfigWidget::cleanup() - cancelling async operations and disconnecting signal handlers");
        self.destroyed.set(true);
        // Increment generation and debounce counters to cancel any pending async operations
        self.rebuild_generation
            .set(self.rebuild_generation.get().wrapping_add(1));
        self.fields_generation
            .set(self.fields_generation.get().wrapping_add(1));
        self.fields_debounce_id
            .set(self.fields_debounce_id.get().wrapping_add(1));

        // Clear the on_fields_updated callback to prevent stale execution
        // if a debounced timer fires after cleanup
        *self.on_fields_updated.borrow_mut() = None;

        // Disconnect all map signal handlers to break reference cycles
        // This is critical for preventing memory leaks - the connect_map closures
        // capture Rc references that would otherwise keep the widget alive indefinitely
        for (slot_name, slot) in self.slot_widgets.borrow_mut().iter_mut() {
            if let Some(handler_id) = slot.map_handler_id.take() {
                log::debug!(
                    "ComboSourceConfigWidget::cleanup() - disconnecting map handler for slot '{}'",
                    slot_name
                );
                slot.tab_scrolled.disconnect(handler_id);
            }
        }
    }
}

impl Drop for ComboSourceConfigWidget {
    fn drop(&mut self) {
        log::info!("=== ComboSourceConfigWidget DROPPED ===");
    }
}

impl Default for ComboSourceConfigWidget {
    fn default() -> Self {
        Self::new()
    }
}
