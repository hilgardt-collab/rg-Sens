//! Configuration widget for Fan Speed source

use gtk4::prelude::*;
use gtk4::{
    Adjustment, Box as GtkBox, CheckButton, DropDown, Entry, Label, Orientation,
    SpinButton, StringList,
};
use std::cell::RefCell;
use std::rc::Rc;

use crate::sources::{FanSpeedConfig, FanSpeedSource, FanCategory};

/// Widget for configuring Fan Speed source
pub struct FanSpeedConfigWidget {
    widget: GtkBox,
    config: Rc<RefCell<FanSpeedConfig>>,
    sensor_combo: DropDown,
    caption_entry: Entry,
    update_interval_spin: SpinButton,
    min_limit_spin: SpinButton,
    max_limit_spin: SpinButton,
    auto_detect_check: CheckButton,
}

impl FanSpeedConfigWidget {
    pub fn new() -> Self {
        let widget = GtkBox::new(Orientation::Vertical, 12);
        widget.set_margin_start(12);
        widget.set_margin_end(12);
        widget.set_margin_top(12);
        widget.set_margin_bottom(12);

        let config = Rc::new(RefCell::new(FanSpeedConfig::default()));

        // Sensor selection
        let sensor_box = GtkBox::new(Orientation::Horizontal, 6);
        sensor_box.append(&Label::new(Some("Fan Sensor:")));

        // Get available sensors
        let sensors = FanSpeedSource::available_sensors();
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
            "Found {} fan sensors ({} CPU, {} GPU, {} Chassis, {} PSU, {} Other)",
            sensors.len(),
            sensors.iter().filter(|s| matches!(s.category, FanCategory::CPU)).count(),
            sensors.iter().filter(|s| matches!(s.category, FanCategory::GPU)).count(),
            sensors.iter().filter(|s| matches!(s.category, FanCategory::Chassis)).count(),
            sensors.iter().filter(|s| matches!(s.category, FanCategory::PSU)).count(),
            sensors.iter().filter(|s| matches!(s.category, FanCategory::Other)).count(),
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

        // Update interval
        let interval_box = GtkBox::new(Orientation::Horizontal, 6);
        interval_box.append(&Label::new(Some("Update Interval (ms):")));

        let interval_adjustment = Adjustment::new(1000.0, 100.0, 60000.0, 100.0, 1000.0, 0.0);
        let update_interval_spin = SpinButton::new(Some(&interval_adjustment), 100.0, 0);
        update_interval_spin.set_hexpand(true);

        interval_box.append(&update_interval_spin);
        widget.append(&interval_box);

        // Value limits
        let limits_label = Label::new(Some("RPM Limits (for displayers):"));
        limits_label.set_halign(gtk4::Align::Start);
        widget.append(&limits_label);

        let auto_detect_check = CheckButton::with_label("Auto-detect limits");
        auto_detect_check.set_active(true);
        widget.append(&auto_detect_check);

        let limits_box = GtkBox::new(Orientation::Horizontal, 6);

        limits_box.append(&Label::new(Some("Min:")));
        let min_adjustment = Adjustment::new(0.0, 0.0, 10000.0, 50.0, 100.0, 0.0);
        let min_limit_spin = SpinButton::new(Some(&min_adjustment), 1.0, 0);
        min_limit_spin.set_hexpand(true);
        min_limit_spin.set_sensitive(false); // Disabled when auto-detect is on
        limits_box.append(&min_limit_spin);

        limits_box.append(&Label::new(Some("RPM")));

        limits_box.append(&Label::new(Some("Max:")));
        let max_adjustment = Adjustment::new(3000.0, 0.0, 10000.0, 50.0, 100.0, 0.0);
        let max_limit_spin = SpinButton::new(Some(&max_adjustment), 1.0, 0);
        max_limit_spin.set_hexpand(true);
        max_limit_spin.set_sensitive(false); // Disabled when auto-detect is on
        limits_box.append(&max_limit_spin);

        limits_box.append(&Label::new(Some("RPM")));

        widget.append(&limits_box);

        // Wire up handlers
        let config_clone = config.clone();
        sensor_combo.connect_selected_notify(move |combo| {
            config_clone.borrow_mut().sensor_index = combo.selected() as usize;
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
            caption_entry,
            update_interval_spin,
            min_limit_spin,
            max_limit_spin,
            auto_detect_check,
        }
    }

    pub fn widget(&self) -> &GtkBox {
        &self.widget
    }

    pub fn get_config(&self) -> FanSpeedConfig {
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

    pub fn set_config(&self, config: &FanSpeedConfig) {
        *self.config.borrow_mut() = config.clone();

        // Update UI
        self.sensor_combo.set_selected(config.sensor_index as u32);
        self.caption_entry.set_text(config.custom_caption.as_deref().unwrap_or(""));
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
    }
}

impl Default for FanSpeedConfigWidget {
    fn default() -> Self {
        Self::new()
    }
}
