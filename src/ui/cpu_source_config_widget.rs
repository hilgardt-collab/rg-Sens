//! CPU source configuration widget

use gtk4::prelude::*;
use gtk4::{
    Adjustment, Box as GtkBox, CheckButton, DropDown, Entry, Label, Orientation, SpinButton,
    StringList,
};
use std::cell::RefCell;
use std::rc::Rc;

use crate::ui::widget_builder::create_page_container;

// Re-export CPU source config types from rg-sens-types
pub use rg_sens_types::source_configs::cpu::{
    CoreSelection, CpuField, CpuSourceConfig, FrequencyUnit, TemperatureUnit,
};

/// Widget for configuring CPU source
pub struct CpuSourceConfigWidget {
    widget: GtkBox,
    config: Rc<RefCell<CpuSourceConfig>>,
    caption_entry: Entry,
    field_combo: DropDown,
    unit_combo: DropDown,
    unit_box: GtkBox,
    freq_combo: DropDown,
    freq_box: GtkBox,
    sensor_combo: DropDown,
    sensor_box: GtkBox,
    core_combo: DropDown,
    per_core_check: CheckButton,
    update_interval_spin: SpinButton,
    min_limit_spin: SpinButton,
    max_limit_spin: SpinButton,
    auto_detect_check: CheckButton,
    min_unit_label: Label,
    max_unit_label: Label,
}

impl Default for CpuSourceConfigWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl CpuSourceConfigWidget {
    pub fn new() -> Self {
        let widget = create_page_container();

        let config = Rc::new(RefCell::new(CpuSourceConfig::default()));

        // Field selection
        let field_box = GtkBox::new(Orientation::Horizontal, 6);
        field_box.append(&Label::new(Some("Data Field:")));

        let field_options = StringList::new(&["Temperature", "Usage", "Frequency"]);
        let field_combo = DropDown::new(Some(field_options), Option::<gtk4::Expression>::None);
        field_combo.set_selected(1); // Default to Usage

        field_box.append(&field_combo);
        widget.append(&field_box);

        // Custom caption
        let caption_box = GtkBox::new(Orientation::Horizontal, 6);
        caption_box.append(&Label::new(Some("Custom Caption:")));

        let caption_entry = Entry::new();
        caption_entry.set_placeholder_text(Some("Auto-generated if empty"));
        caption_entry.set_hexpand(true);

        caption_box.append(&caption_entry);
        widget.append(&caption_box);

        // Unit selection (only for temperature)
        let unit_box = GtkBox::new(Orientation::Horizontal, 6);
        unit_box.append(&Label::new(Some("Temperature Unit:")));

        let unit_options = StringList::new(&["Celsius (°C)", "Fahrenheit (°F)", "Kelvin (K)"]);
        let unit_combo = DropDown::new(Some(unit_options), Option::<gtk4::Expression>::None);
        unit_combo.set_selected(0); // Default to Celsius

        unit_box.append(&unit_combo);
        unit_box.set_visible(false); // Hidden by default (show only for temperature)
        widget.append(&unit_box);

        // Frequency unit selection (only for frequency)
        let freq_box = GtkBox::new(Orientation::Horizontal, 6);
        freq_box.append(&Label::new(Some("Frequency Unit:")));

        let freq_options = StringList::new(&["MHz", "GHz"]);
        let freq_combo = DropDown::new(Some(freq_options), Option::<gtk4::Expression>::None);
        freq_combo.set_selected(0); // Default to MHz

        freq_box.append(&freq_combo);
        freq_box.set_visible(false); // Hidden by default (show only for frequency)
        widget.append(&freq_box);

        // Sensor selection
        let sensor_box = GtkBox::new(Orientation::Horizontal, 6);
        sensor_box.append(&Label::new(Some("Sensor:")));

        let sensor_options = StringList::new(&["CPU Sensor 1"]); // Will be populated dynamically
        let sensor_combo = DropDown::new(Some(sensor_options), Option::<gtk4::Expression>::None);
        sensor_combo.set_selected(0);

        sensor_box.append(&sensor_combo);
        sensor_box.set_visible(false); // Hidden by default (show only for temperature)
        widget.append(&sensor_box);

        // Per-core selection
        let core_box = GtkBox::new(Orientation::Vertical, 6);

        let per_core_check = CheckButton::with_label("Show per-core data");
        per_core_check.set_active(false);
        core_box.append(&per_core_check);

        let core_select_box = GtkBox::new(Orientation::Horizontal, 6);
        core_select_box.append(&Label::new(Some("Core:")));

        let core_options = StringList::new(&["Overall", "Core 0", "Core 1", "Core 2", "Core 3"]);
        let core_combo = DropDown::new(Some(core_options), Option::<gtk4::Expression>::None);
        core_combo.set_selected(0);
        core_combo.set_sensitive(false); // Disabled until per-core is checked

        core_select_box.append(&core_combo);
        core_box.append(&core_select_box);

        widget.append(&core_box);

        // Update interval
        let interval_box = GtkBox::new(Orientation::Horizontal, 6);
        interval_box.append(&Label::new(Some("Update Interval (ms):")));

        let interval_adjustment = Adjustment::new(1000.0, 100.0, 60000.0, 100.0, 1000.0, 0.0);
        let update_interval_spin = SpinButton::new(Some(&interval_adjustment), 100.0, 0);
        update_interval_spin.set_hexpand(true);

        interval_box.append(&update_interval_spin);
        widget.append(&interval_box);

        // Value limits
        let limits_label = Label::new(Some("Value Limits (for displayers):"));
        limits_label.set_halign(gtk4::Align::Start);
        widget.append(&limits_label);

        let auto_detect_check = CheckButton::with_label("Auto-detect limits");
        auto_detect_check.set_active(true);
        widget.append(&auto_detect_check);

        let limits_box = GtkBox::new(Orientation::Horizontal, 6);

        limits_box.append(&Label::new(Some("Min:")));
        let min_adjustment = Adjustment::new(0.0, -1000.0, 10000.0, 1.0, 10.0, 0.0);
        let min_limit_spin = SpinButton::new(Some(&min_adjustment), 0.1, 2);
        min_limit_spin.set_hexpand(true);
        min_limit_spin.set_sensitive(false); // Disabled when auto-detect is on
        limits_box.append(&min_limit_spin);

        // Unit label (shows current unit for limits)
        let min_unit_label = Label::new(Some("%"));
        limits_box.append(&min_unit_label);

        limits_box.append(&Label::new(Some("Max:")));
        let max_adjustment = Adjustment::new(100.0, -1000.0, 10000.0, 1.0, 10.0, 0.0);
        let max_limit_spin = SpinButton::new(Some(&max_adjustment), 0.1, 2);
        max_limit_spin.set_hexpand(true);
        max_limit_spin.set_sensitive(false); // Disabled when auto-detect is on
        limits_box.append(&max_limit_spin);

        // Unit label (shows current unit for limits)
        let max_unit_label = Label::new(Some("%"));
        limits_box.append(&max_unit_label);

        widget.append(&limits_box);

        // Wire up handlers
        let config_clone = config.clone();
        let unit_box_clone = unit_box.clone();
        let freq_box_clone = freq_box.clone();
        let sensor_box_clone = sensor_box.clone();
        let min_unit_label_clone = min_unit_label.clone();
        let max_unit_label_clone = max_unit_label.clone();
        let freq_combo_clone = freq_combo.clone();
        let unit_combo_clone = unit_combo.clone();
        field_combo.connect_selected_notify(move |combo| {
            let selected = combo.selected();
            let field = match selected {
                0 => CpuField::Temperature,
                1 => CpuField::Usage,
                2 => CpuField::Frequency,
                _ => CpuField::Usage,
            };

            config_clone.borrow_mut().field = field;

            // Show/hide unit and sensor selectors based on field
            let is_temp = field == CpuField::Temperature;
            let is_freq = field == CpuField::Frequency;
            unit_box_clone.set_visible(is_temp);
            freq_box_clone.set_visible(is_freq);
            sensor_box_clone.set_visible(is_temp);

            // Update limit unit labels
            let unit_text = match field {
                CpuField::Temperature => {
                    let temp_unit = unit_combo_clone.selected();
                    match temp_unit {
                        0 => "°C",
                        1 => "°F",
                        2 => "K",
                        _ => "°C",
                    }
                }
                CpuField::Usage => "%",
                CpuField::Frequency => {
                    let freq_unit = freq_combo_clone.selected();
                    match freq_unit {
                        0 => "MHz",
                        1 => "GHz",
                        _ => "MHz",
                    }
                }
            };
            min_unit_label_clone.set_text(unit_text);
            max_unit_label_clone.set_text(unit_text);
        });

        let config_clone = config.clone();
        caption_entry.connect_changed(move |entry| {
            let text = entry.text().to_string();
            config_clone.borrow_mut().custom_caption =
                if text.is_empty() { None } else { Some(text) };
        });

        let config_clone = config.clone();
        let min_unit_label_clone2 = min_unit_label.clone();
        let max_unit_label_clone2 = max_unit_label.clone();
        unit_combo.connect_selected_notify(move |combo| {
            let selected = combo.selected();
            let unit = match selected {
                0 => TemperatureUnit::Celsius,
                1 => TemperatureUnit::Fahrenheit,
                2 => TemperatureUnit::Kelvin,
                _ => TemperatureUnit::Celsius,
            };

            config_clone.borrow_mut().temp_unit = unit;

            // Update limit unit labels
            let unit_text = match selected {
                0 => "°C",
                1 => "°F",
                2 => "K",
                _ => "°C",
            };
            min_unit_label_clone2.set_text(unit_text);
            max_unit_label_clone2.set_text(unit_text);
        });

        let config_clone = config.clone();
        let min_unit_label_clone3 = min_unit_label.clone();
        let max_unit_label_clone3 = max_unit_label.clone();
        freq_combo.connect_selected_notify(move |combo| {
            let selected = combo.selected();
            let unit = match selected {
                0 => FrequencyUnit::MHz,
                1 => FrequencyUnit::GHz,
                _ => FrequencyUnit::MHz,
            };

            config_clone.borrow_mut().freq_unit = unit;

            // Update limit unit labels
            let unit_text = match selected {
                0 => "MHz",
                1 => "GHz",
                _ => "MHz",
            };
            min_unit_label_clone3.set_text(unit_text);
            max_unit_label_clone3.set_text(unit_text);
        });

        let config_clone = config.clone();
        sensor_combo.connect_selected_notify(move |combo| {
            let selected = combo.selected();
            // GTK returns u32::MAX (GTK_INVALID_LIST_POSITION) when nothing is selected
            if selected != gtk4::INVALID_LIST_POSITION {
                // Validate against the model size
                if let Some(model) = combo.model() {
                    if (selected as usize) < model.n_items() as usize {
                        config_clone.borrow_mut().sensor_index = selected as usize;
                    }
                }
            }
        });

        let config_clone = config.clone();
        let core_combo_clone = core_combo.clone();
        per_core_check.connect_toggled(move |check| {
            let active = check.is_active();
            core_combo_clone.set_sensitive(active);

            if !active {
                config_clone.borrow_mut().core_selection = CoreSelection::Overall;
            }
        });

        let config_clone = config.clone();
        core_combo.connect_selected_notify(move |combo| {
            let selected = combo.selected();
            // GTK returns u32::MAX (GTK_INVALID_LIST_POSITION) when nothing is selected
            if selected == gtk4::INVALID_LIST_POSITION {
                return;
            }
            let selection = if selected == 0 {
                CoreSelection::Overall
            } else {
                CoreSelection::Core(selected as usize - 1)
            };

            config_clone.borrow_mut().core_selection = selection;
        });

        let config_clone = config.clone();
        update_interval_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().update_interval_ms = spin.value() as u64;
        });

        let config_clone = config.clone();
        let min_spin_clone = min_limit_spin.clone();
        let max_spin_clone = max_limit_spin.clone();
        auto_detect_check.connect_toggled(move |check| {
            let active = check.is_active();
            config_clone.borrow_mut().auto_detect_limits = active;

            // Enable/disable manual limit inputs
            min_spin_clone.set_sensitive(!active);
            max_spin_clone.set_sensitive(!active);

            // Clear limits if auto-detect is enabled
            if active {
                config_clone.borrow_mut().min_limit = None;
                config_clone.borrow_mut().max_limit = None;
            }
        });

        let config_clone = config.clone();
        min_limit_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().min_limit = Some(spin.value());
        });

        let config_clone = config.clone();
        max_limit_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().max_limit = Some(spin.value());
        });

        let widget_instance = Self {
            widget,
            config,
            caption_entry,
            field_combo,
            unit_combo,
            unit_box,
            freq_combo,
            freq_box,
            sensor_combo,
            sensor_box,
            core_combo,
            per_core_check,
            update_interval_spin,
            min_limit_spin,
            max_limit_spin,
            auto_detect_check,
            min_unit_label,
            max_unit_label,
        };

        // Initialize sensors and core count from cached data
        widget_instance.set_available_sensors(crate::sources::CpuSource::get_cached_sensors());
        widget_instance.set_cpu_core_count(crate::sources::CpuSource::get_cached_core_count());

        widget_instance
    }

    pub fn widget(&self) -> &GtkBox {
        &self.widget
    }

    pub fn set_config(&self, config: CpuSourceConfig) {
        // Update UI widgets based on config
        self.field_combo.set_selected(match config.field {
            CpuField::Temperature => 0,
            CpuField::Usage => 1,
            CpuField::Frequency => 2,
        });

        self.unit_combo.set_selected(match config.temp_unit {
            TemperatureUnit::Celsius => 0,
            TemperatureUnit::Fahrenheit => 1,
            TemperatureUnit::Kelvin => 2,
        });

        self.freq_combo.set_selected(match config.freq_unit {
            FrequencyUnit::MHz => 0,
            FrequencyUnit::GHz => 1,
        });

        let is_temp = config.field == CpuField::Temperature;
        let is_freq = config.field == CpuField::Frequency;
        self.unit_box.set_visible(is_temp);
        self.freq_box.set_visible(is_freq);
        self.sensor_box.set_visible(is_temp);

        // Update limit unit labels
        let unit_text = match config.field {
            CpuField::Temperature => match config.temp_unit {
                TemperatureUnit::Celsius => "°C",
                TemperatureUnit::Fahrenheit => "°F",
                TemperatureUnit::Kelvin => "K",
            },
            CpuField::Usage => "%",
            CpuField::Frequency => match config.freq_unit {
                FrequencyUnit::MHz => "MHz",
                FrequencyUnit::GHz => "GHz",
            },
        };
        self.min_unit_label.set_text(unit_text);
        self.max_unit_label.set_text(unit_text);

        // Set custom caption if provided
        if let Some(ref caption) = config.custom_caption {
            self.caption_entry.set_text(caption);
        } else {
            self.caption_entry.set_text("");
        }

        self.sensor_combo.set_selected(config.sensor_index as u32);

        match &config.core_selection {
            CoreSelection::Overall => {
                self.per_core_check.set_active(false);
                self.core_combo.set_selected(0);
                self.core_combo.set_sensitive(false);
            }
            CoreSelection::Core(core_idx) => {
                self.per_core_check.set_active(true);
                self.core_combo.set_selected((*core_idx + 1) as u32);
                self.core_combo.set_sensitive(true);
            }
        }

        // Set update interval
        self.update_interval_spin
            .set_value(config.update_interval_ms as f64);

        // Set auto-detect checkbox and limits
        self.auto_detect_check.set_active(config.auto_detect_limits);
        self.min_limit_spin
            .set_sensitive(!config.auto_detect_limits);
        self.max_limit_spin
            .set_sensitive(!config.auto_detect_limits);

        if let Some(min) = config.min_limit {
            self.min_limit_spin.set_value(min);
        }

        if let Some(max) = config.max_limit {
            self.max_limit_spin.set_value(max);
        }

        *self.config.borrow_mut() = config;
    }

    pub fn get_config(&self) -> CpuSourceConfig {
        let mut config = self.config.borrow().clone();

        // When auto_detect is disabled, ensure we use the spinbutton values
        // This handles the case where set_config was called with None limits
        // but the spinbuttons have values the user may have entered
        if !config.auto_detect_limits {
            config.min_limit = Some(self.min_limit_spin.value());
            config.max_limit = Some(self.max_limit_spin.value());
        }

        config
    }

    /// Populate sensor dropdown with available CPU sensors
    pub fn set_available_sensors(&self, sensors: &[crate::sources::CpuSensor]) {
        let sensor_names: Vec<String> = if sensors.is_empty() {
            vec!["No sensors detected".to_string()]
        } else {
            sensors.iter().map(|s| s.label.clone()).collect()
        };

        let sensor_list =
            StringList::new(&sensor_names.iter().map(|s| s.as_str()).collect::<Vec<_>>());
        self.sensor_combo.set_model(Some(&sensor_list));

        // Set to first sensor by default
        if !sensors.is_empty() {
            self.sensor_combo.set_selected(0);
        }

        // Disable if no sensors
        self.sensor_combo.set_sensitive(!sensors.is_empty());
    }

    /// Populate core dropdown with actual number of CPU cores
    pub fn set_cpu_core_count(&self, num_cores: usize) {
        let mut core_names = vec!["Overall".to_string()];
        for i in 0..num_cores {
            core_names.push(format!("Core {}", i));
        }

        let core_list = StringList::new(&core_names.iter().map(|s| s.as_str()).collect::<Vec<_>>());
        self.core_combo.set_model(Some(&core_list));
        self.core_combo.set_selected(0);
    }
}
