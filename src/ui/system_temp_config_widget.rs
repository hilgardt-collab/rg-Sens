//! Configuration widget for System Temperature source

use gtk4::prelude::*;
use gtk4::{
    Adjustment, Box as GtkBox, CheckButton, DropDown, Entry, Label, Orientation,
    SpinButton, StringList,
};
use std::cell::RefCell;
use std::rc::Rc;

use crate::sources::{SystemTempConfig, SystemTempSource, SystemTempUnit, SensorCategory};

/// Widget for configuring System Temperature source
pub struct SystemTempConfigWidget {
    widget: GtkBox,
    config: Rc<RefCell<SystemTempConfig>>,
    sensor_combo: DropDown,
    unit_combo: DropDown,
    caption_entry: Entry,
    update_interval_spin: SpinButton,
    min_limit_spin: SpinButton,
    max_limit_spin: SpinButton,
    auto_detect_check: CheckButton,
    min_unit_label: Label,
    max_unit_label: Label,
}

impl Default for SystemTempConfigWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemTempConfigWidget {
    pub fn new() -> Self {
        let widget = GtkBox::new(Orientation::Vertical, 12);
        widget.set_margin_start(12);
        widget.set_margin_end(12);
        widget.set_margin_top(12);
        widget.set_margin_bottom(12);

        let config = Rc::new(RefCell::new(SystemTempConfig::default()));

        // Sensor selection
        let sensor_box = GtkBox::new(Orientation::Horizontal, 6);
        sensor_box.append(&Label::new(Some("Temperature Sensor:")));

        // Get available sensors
        let sensors = SystemTempSource::available_sensors();
        let sensor_labels: Vec<String> = sensors
            .iter()
            .map(|s| format!("[{:?}] {}", s.category, s.label))
            .collect();

        let sensor_options = StringList::new(&sensor_labels.iter().map(|s| s.as_str()).collect::<Vec<_>>());
        let sensor_combo = DropDown::new(Some(sensor_options), Option::<gtk4::Expression>::None);
        sensor_combo.set_selected(0);
        sensor_combo.set_hexpand(true);

        sensor_box.append(&sensor_combo);
        widget.append(&sensor_box);

        // Add info label showing sensor count
        let info_label = Label::new(Some(&format!(
            "Found {} temperature sensors ({} CPU, {} GPU, {} MB, {} Storage, {} Other)",
            sensors.len(),
            sensors.iter().filter(|s| matches!(s.category, SensorCategory::CPU)).count(),
            sensors.iter().filter(|s| matches!(s.category, SensorCategory::GPU)).count(),
            sensors.iter().filter(|s| matches!(s.category, SensorCategory::Motherboard)).count(),
            sensors.iter().filter(|s| matches!(s.category, SensorCategory::Storage)).count(),
            sensors.iter().filter(|s| matches!(s.category, SensorCategory::Other)).count(),
        )));
        info_label.set_halign(gtk4::Align::Start);
        info_label.add_css_class("dim-label");
        widget.append(&info_label);

        // Custom caption
        let caption_box = GtkBox::new(Orientation::Horizontal, 6);
        caption_box.append(&Label::new(Some("Custom Caption:")));

        let caption_entry = Entry::new();
        caption_entry.set_placeholder_text(Some("Auto-generated if empty"));
        caption_entry.set_hexpand(true);

        caption_box.append(&caption_entry);
        widget.append(&caption_box);

        // Unit selection
        let unit_box = GtkBox::new(Orientation::Horizontal, 6);
        unit_box.append(&Label::new(Some("Temperature Unit:")));

        let unit_options = StringList::new(&["Celsius (°C)", "Fahrenheit (°F)", "Kelvin (K)"]);
        let unit_combo = DropDown::new(Some(unit_options), Option::<gtk4::Expression>::None);
        unit_combo.set_selected(0); // Default to Celsius

        unit_box.append(&unit_combo);
        widget.append(&unit_box);

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
        let min_adjustment = Adjustment::new(0.0, -50.0, 200.0, 1.0, 10.0, 0.0);
        let min_limit_spin = SpinButton::new(Some(&min_adjustment), 0.1, 2);
        min_limit_spin.set_hexpand(true);
        min_limit_spin.set_sensitive(false); // Disabled when auto-detect is on
        limits_box.append(&min_limit_spin);

        let min_unit_label = Label::new(Some("°C"));
        limits_box.append(&min_unit_label);

        limits_box.append(&Label::new(Some("Max:")));
        let max_adjustment = Adjustment::new(100.0, -50.0, 200.0, 1.0, 10.0, 0.0);
        let max_limit_spin = SpinButton::new(Some(&max_adjustment), 0.1, 2);
        max_limit_spin.set_hexpand(true);
        max_limit_spin.set_sensitive(false); // Disabled when auto-detect is on
        limits_box.append(&max_limit_spin);

        let max_unit_label = Label::new(Some("°C"));
        limits_box.append(&max_unit_label);

        widget.append(&limits_box);

        // Wire up handlers
        let config_clone = config.clone();
        sensor_combo.connect_selected_notify(move |combo| {
            let index = combo.selected() as usize;
            // Use set_sensor_by_index to store both index and label for stability
            config_clone.borrow_mut().set_sensor_by_index(index);
        });

        let config_clone = config.clone();
        caption_entry.connect_changed(move |entry| {
            let text = entry.text().to_string();
            config_clone.borrow_mut().custom_caption = if text.is_empty() {
                None
            } else {
                Some(text)
            };
        });

        let config_clone = config.clone();
        let min_unit_label_clone = min_unit_label.clone();
        let max_unit_label_clone = max_unit_label.clone();
        unit_combo.connect_selected_notify(move |combo| {
            let selected = combo.selected();
            let unit = match selected {
                0 => SystemTempUnit::Celsius,
                1 => SystemTempUnit::Fahrenheit,
                2 => SystemTempUnit::Kelvin,
                _ => SystemTempUnit::Celsius,
            };

            config_clone.borrow_mut().temp_unit = unit;

            // Update limit unit labels
            let unit_text = match selected {
                0 => "°C",
                1 => "°F",
                2 => "K",
                _ => "°C",
            };
            min_unit_label_clone.set_text(unit_text);
            max_unit_label_clone.set_text(unit_text);
        });

        let config_clone = config.clone();
        update_interval_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().update_interval_ms = spin.value() as u64;
        });

        let config_clone = config.clone();
        let min_spin_clone = min_limit_spin.clone();
        let max_spin_clone = max_limit_spin.clone();
        auto_detect_check.connect_toggled(move |check| {
            let enabled = check.is_active();
            config_clone.borrow_mut().auto_detect_limits = enabled;
            min_spin_clone.set_sensitive(!enabled);
            max_spin_clone.set_sensitive(!enabled);
        });

        let config_clone = config.clone();
        min_limit_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().min_limit = Some(spin.value());
        });

        let config_clone = config.clone();
        max_limit_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().max_limit = Some(spin.value());
        });

        Self {
            widget,
            config,
            sensor_combo,
            unit_combo,
            caption_entry,
            update_interval_spin,
            min_limit_spin,
            max_limit_spin,
            auto_detect_check,
            min_unit_label,
            max_unit_label,
        }
    }

    pub fn widget(&self) -> &GtkBox {
        &self.widget
    }

    pub fn set_config(&self, config: SystemTempConfig) {
        // Resolve sensor_label to get the correct index (handles sensor order changes)
        let mut config = config;
        let sensor_index = config.resolve_sensor_index();

        // Update UI widgets based on config
        self.sensor_combo.set_selected(sensor_index as u32);

        self.unit_combo.set_selected(match config.temp_unit {
            SystemTempUnit::Celsius => 0,
            SystemTempUnit::Fahrenheit => 1,
            SystemTempUnit::Kelvin => 2,
        });

        // Update limit unit labels
        let unit_text = match config.temp_unit {
            SystemTempUnit::Celsius => "°C",
            SystemTempUnit::Fahrenheit => "°F",
            SystemTempUnit::Kelvin => "K",
        };
        self.min_unit_label.set_text(unit_text);
        self.max_unit_label.set_text(unit_text);

        // Set custom caption if provided
        if let Some(ref caption) = config.custom_caption {
            self.caption_entry.set_text(caption);
        } else {
            self.caption_entry.set_text("");
        }

        self.update_interval_spin.set_value(config.update_interval_ms as f64);
        self.auto_detect_check.set_active(config.auto_detect_limits);

        if let Some(min) = config.min_limit {
            self.min_limit_spin.set_value(min);
        }

        if let Some(max) = config.max_limit {
            self.max_limit_spin.set_value(max);
        }

        self.min_limit_spin.set_sensitive(!config.auto_detect_limits);
        self.max_limit_spin.set_sensitive(!config.auto_detect_limits);

        // Store config
        *self.config.borrow_mut() = config;
    }

    pub fn get_config(&self) -> SystemTempConfig {
        let mut config = self.config.borrow().clone();

        // Ensure sensor_label is set based on current index for stability across restarts
        let index = self.sensor_combo.selected() as usize;
        config.set_sensor_by_index(index);

        // When auto_detect is disabled, ensure we use the spinbutton values
        // This handles the case where set_config was called with None limits
        // but the spinbuttons have values the user may have entered
        if !config.auto_detect_limits {
            config.min_limit = Some(self.min_limit_spin.value());
            config.max_limit = Some(self.max_limit_spin.value());
        }

        config
    }
}
