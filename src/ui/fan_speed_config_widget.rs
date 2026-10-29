//! Configuration widget for Fan Speed source

use gtk4::prelude::*;
use gtk4::{
    Adjustment, Box as GtkBox, CheckButton, DropDown, Entry, Label, Orientation, SpinButton,
    StringList,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::sources::{
    describe_fan_selection, resolve_fan_index, set_fan_by_index, FanCategory, FanSpeedConfig,
    FanSpeedSource,
};
use crate::ui::widget_builder::create_page_container;

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
    /// Warning shown when the saved fan is not present on this system
    missing_label: Label,
    /// False while the dropdown does not reflect the stored config (the saved
    /// fan is missing, so nothing could be selected for it). `get_config` then
    /// keeps the stored selection instead of syncing from the dropdown. GTK's
    /// DropDown cannot be set to "no selection" (`set_selected` with
    /// `INVALID_LIST_POSITION` is a no-op), hence the flag.
    selection_valid: Rc<Cell<bool>>,
}

impl FanSpeedConfigWidget {
    pub fn new() -> Self {
        let widget = create_page_container();

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

        let sensor_options =
            StringList::new(&sensor_labels.iter().map(|s| s.as_str()).collect::<Vec<_>>());
        let sensor_combo = DropDown::new(Some(sensor_options), Option::<gtk4::Expression>::None);
        sensor_combo.set_selected(0);
        sensor_combo.set_hexpand(true);

        sensor_box.append(&sensor_combo);
        widget.append(&sensor_box);

        // Shown when the saved fan is not present on this system
        let missing_label = Label::new(None);
        missing_label.set_halign(gtk4::Align::Start);
        missing_label.set_wrap(true);
        missing_label.add_css_class("warning");
        missing_label.set_visible(false);
        widget.append(&missing_label);

        // Add info label showing sensor count
        let info_label = Label::new(Some(&format!(
            "Found {} fan sensors ({} CPU, {} GPU, {} Chassis, {} PSU, {} Other)",
            sensors.len(),
            sensors
                .iter()
                .filter(|s| matches!(s.category, FanCategory::CPU))
                .count(),
            sensors
                .iter()
                .filter(|s| matches!(s.category, FanCategory::GPU))
                .count(),
            sensors
                .iter()
                .filter(|s| matches!(s.category, FanCategory::Chassis))
                .count(),
            sensors
                .iter()
                .filter(|s| matches!(s.category, FanCategory::PSU))
                .count(),
            sensors
                .iter()
                .filter(|s| matches!(s.category, FanCategory::Other))
                .count(),
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
        let selection_valid = Rc::new(Cell::new(true));
        let config_clone = config.clone();
        let sensor_count = sensors.len();
        let missing_label_clone = missing_label.clone();
        let selection_valid_clone = selection_valid.clone();
        sensor_combo.connect_selected_notify(move |combo| {
            let selected = combo.selected();
            // GTK returns u32::MAX (GTK_INVALID_LIST_POSITION) when nothing is selected
            if selected != gtk4::INVALID_LIST_POSITION && (selected as usize) < sensor_count {
                // Store both index and label: the label is the stable identity
                // that survives hwmon reordering across boots
                set_fan_by_index(&mut config_clone.borrow_mut(), selected as usize);
                selection_valid_clone.set(true);
                missing_label_clone.set_visible(false);
            }
        });

        let config_clone = config.clone();
        caption_entry.connect_changed(move |entry| {
            let text = entry.text().to_string();
            config_clone.borrow_mut().custom_caption =
                if text.is_empty() { None } else { Some(text) };
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
            missing_label,
            selection_valid,
        }
    }

    pub fn widget(&self) -> &GtkBox {
        &self.widget
    }

    pub fn get_config(&self) -> FanSpeedConfig {
        let mut config = self.config.borrow().clone();

        // Sync index + label from the dropdown, unless the dropdown does not
        // reflect the stored config (saved fan not present): then the stored
        // selection is kept as-is rather than replaced by whatever is showing.
        let selected = self.sensor_combo.selected();
        if self.selection_valid.get() && selected != gtk4::INVALID_LIST_POSITION {
            set_fan_by_index(&mut config, selected as usize);
        }

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
        // Resolve the saved label to this run's index (hwmon order changes
        // across boots); this also normalizes index/label in the stored config
        let mut config = config.clone();
        let resolved = resolve_fan_index(&mut config);
        *self.config.borrow_mut() = config.clone();

        // Update UI
        match resolved {
            Some(index) => {
                self.missing_label.set_visible(false);
                self.sensor_combo.set_selected(index as u32);
                self.selection_valid.set(true);
            }
            None => {
                // The dropdown keeps showing some fan, but it is not the
                // configured one; ignore it until the user picks explicitly
                self.selection_valid.set(false);
                self.missing_label.set_text(&format!(
                    "Saved fan {} was not found on this system, so the panel shows N/A. \
                     Pick a fan above to replace it.",
                    describe_fan_selection(&config)
                ));
                self.missing_label.set_visible(true);
            }
        }
        self.caption_entry
            .set_text(config.custom_caption.as_deref().unwrap_or(""));
        self.update_interval_spin
            .set_value(config.update_interval_ms as f64);
        self.auto_detect_check.set_active(config.auto_detect_limits);

        if let Some(min) = config.min_limit {
            self.min_limit_spin.set_value(min);
        }
        if let Some(max) = config.max_limit {
            self.max_limit_spin.set_value(max);
        }

        self.min_limit_spin
            .set_sensitive(!config.auto_detect_limits);
        self.max_limit_spin
            .set_sensitive(!config.auto_detect_limits);
    }
}

impl Default for FanSpeedConfigWidget {
    fn default() -> Self {
        Self::new()
    }
}
