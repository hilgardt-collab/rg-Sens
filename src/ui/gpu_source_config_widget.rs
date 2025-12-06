//! GPU source configuration widget

use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, CheckButton, DropDown, Entry, Label, Orientation, SpinButton, StringList,
};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::rc::Rc;

/// GPU source field types
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum GpuField {
    Temperature,
    Utilization,
    MemoryUsed,
    MemoryPercent,
    PowerUsage,
    FanSpeed,
}

use crate::ui::TemperatureUnit;

/// GPU source configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuSourceConfig {
    pub field: GpuField,
    pub temp_unit: TemperatureUnit,
    pub gpu_index: u32,
    #[serde(default)]
    pub custom_caption: Option<String>,
    #[serde(default = "default_update_interval")]
    pub update_interval_ms: u64,
    #[serde(default)]
    pub min_limit: Option<f64>,
    #[serde(default)]
    pub max_limit: Option<f64>,
    #[serde(default = "default_auto_detect_limits")]
    pub auto_detect_limits: bool,
}

fn default_update_interval() -> u64 {
    1000 // 1 second default
}

fn default_auto_detect_limits() -> bool {
    true
}

impl Default for GpuSourceConfig {
    fn default() -> Self {
        Self {
            field: GpuField::Temperature,
            temp_unit: TemperatureUnit::Celsius,
            gpu_index: 0,
            custom_caption: None,
            update_interval_ms: default_update_interval(),
            min_limit: None,
            max_limit: None,
            auto_detect_limits: default_auto_detect_limits(),
        }
    }
}

/// Widget for configuring GPU source
pub struct GpuSourceConfigWidget {
    widget: GtkBox,
    config: Rc<RefCell<GpuSourceConfig>>,
    caption_entry: Entry,
    field_combo: DropDown,
    unit_combo: DropDown,
    unit_box: GtkBox,
    gpu_combo: DropDown,
    update_interval_spin: SpinButton,
    min_limit_spin: SpinButton,
    max_limit_spin: SpinButton,
    auto_detect_check: CheckButton,
}

impl GpuSourceConfigWidget {
    pub fn new() -> Self {
        let widget = GtkBox::new(Orientation::Vertical, 12);
        widget.set_margin_start(12);
        widget.set_margin_end(12);

        let config = Rc::new(RefCell::new(GpuSourceConfig::default()));

        // Custom caption
        let caption_box = GtkBox::new(Orientation::Horizontal, 6);
        caption_box.append(&Label::new(Some("Custom Caption:")));
        let caption_entry = Entry::new();
        caption_entry.set_placeholder_text(Some("Auto-generated if empty"));
        caption_entry.set_hexpand(true);
        caption_box.append(&caption_entry);
        widget.append(&caption_box);

        // Field selection
        let field_box = GtkBox::new(Orientation::Horizontal, 6);
        field_box.append(&Label::new(Some("Field:")));

        let field_options = StringList::new(&[
            "Temperature",
            "Utilization",
            "Memory Used",
            "Memory Percent",
            "Power Usage",
            "Fan Speed",
        ]);
        let field_combo = DropDown::new(Some(field_options), Option::<gtk4::Expression>::None);
        field_combo.set_selected(0); // Temperature by default
        field_box.append(&field_combo);
        widget.append(&field_box);

        // Temperature unit (visible only for temperature field)
        let unit_box = GtkBox::new(Orientation::Horizontal, 6);
        unit_box.append(&Label::new(Some("Temperature Unit:")));

        let unit_options = StringList::new(&["Celsius", "Fahrenheit", "Kelvin"]);
        let unit_combo = DropDown::new(Some(unit_options), Option::<gtk4::Expression>::None);
        unit_combo.set_selected(0);
        unit_box.append(&unit_combo);
        unit_box.set_visible(true); // Visible by default (temperature is default field)
        widget.append(&unit_box);

        // GPU selection
        let gpu_box = GtkBox::new(Orientation::Horizontal, 6);
        gpu_box.append(&Label::new(Some("GPU:")));

        let gpu_options = StringList::new(&["GPU 0"]);
        let gpu_combo = DropDown::new(Some(gpu_options), Option::<gtk4::Expression>::None);
        gpu_combo.set_selected(0);
        gpu_box.append(&gpu_combo);
        widget.append(&gpu_box);

        // Update interval
        let interval_box = GtkBox::new(Orientation::Horizontal, 6);
        interval_box.append(&Label::new(Some("Update Interval (ms):")));

        let interval_adjustment = gtk4::Adjustment::new(1000.0, 100.0, 60000.0, 100.0, 1000.0, 0.0);
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
        let min_adjustment = gtk4::Adjustment::new(0.0, -1000.0, 1000.0, 1.0, 10.0, 0.0);
        let min_limit_spin = SpinButton::new(Some(&min_adjustment), 0.1, 2);
        min_limit_spin.set_hexpand(true);
        min_limit_spin.set_sensitive(false);
        limits_box.append(&min_limit_spin);

        limits_box.append(&Label::new(Some("Max:")));
        let max_adjustment = gtk4::Adjustment::new(100.0, -1000.0, 10000.0, 1.0, 10.0, 0.0);
        let max_limit_spin = SpinButton::new(Some(&max_adjustment), 0.1, 2);
        max_limit_spin.set_hexpand(true);
        max_limit_spin.set_sensitive(false);
        limits_box.append(&max_limit_spin);

        widget.append(&limits_box);

        // Wire up handlers
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
        let unit_box_clone = unit_box.clone();
        field_combo.connect_selected_notify(move |combo| {
            let selected = combo.selected();
            let field = match selected {
                0 => GpuField::Temperature,
                1 => GpuField::Utilization,
                2 => GpuField::MemoryUsed,
                3 => GpuField::MemoryPercent,
                4 => GpuField::PowerUsage,
                5 => GpuField::FanSpeed,
                _ => GpuField::Temperature,
            };
            config_clone.borrow_mut().field = field;

            // Show/hide temperature unit box
            unit_box_clone.set_visible(field == GpuField::Temperature);
        });

        let config_clone = config.clone();
        unit_combo.connect_selected_notify(move |combo| {
            let unit = match combo.selected() {
                0 => TemperatureUnit::Celsius,
                1 => TemperatureUnit::Fahrenheit,
                2 => TemperatureUnit::Kelvin,
                _ => TemperatureUnit::Celsius,
            };
            config_clone.borrow_mut().temp_unit = unit;
        });

        let config_clone = config.clone();
        gpu_combo.connect_selected_notify(move |combo| {
            config_clone.borrow_mut().gpu_index = combo.selected();
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

            min_spin_clone.set_sensitive(!active);
            max_spin_clone.set_sensitive(!active);

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

        Self {
            widget,
            config,
            caption_entry,
            field_combo,
            unit_combo,
            unit_box,
            gpu_combo,
            update_interval_spin,
            min_limit_spin,
            max_limit_spin,
            auto_detect_check,
        }
    }

    pub fn widget(&self) -> &GtkBox {
        &self.widget
    }

    pub fn set_config(&self, config: GpuSourceConfig) {
        // Update UI widgets based on config
        self.field_combo.set_selected(match config.field {
            GpuField::Temperature => 0,
            GpuField::Utilization => 1,
            GpuField::MemoryUsed => 2,
            GpuField::MemoryPercent => 3,
            GpuField::PowerUsage => 4,
            GpuField::FanSpeed => 5,
        });

        self.unit_combo.set_selected(match config.temp_unit {
            TemperatureUnit::Celsius => 0,
            TemperatureUnit::Fahrenheit => 1,
            TemperatureUnit::Kelvin => 2,
        });

        self.unit_box.set_visible(config.field == GpuField::Temperature);

        if let Some(ref caption) = config.custom_caption {
            self.caption_entry.set_text(caption);
        } else {
            self.caption_entry.set_text("");
        }

        self.gpu_combo.set_selected(config.gpu_index);
        self.update_interval_spin.set_value(config.update_interval_ms as f64);

        self.auto_detect_check.set_active(config.auto_detect_limits);
        self.min_limit_spin.set_sensitive(!config.auto_detect_limits);
        self.max_limit_spin.set_sensitive(!config.auto_detect_limits);

        if let Some(min) = config.min_limit {
            self.min_limit_spin.set_value(min);
        }

        if let Some(max) = config.max_limit {
            self.max_limit_spin.set_value(max);
        }

        *self.config.borrow_mut() = config;
    }

    pub fn get_config(&self) -> GpuSourceConfig {
        self.config.borrow().clone()
    }

    /// Set available GPUs
    pub fn set_available_gpus(&self, gpu_names: &[String]) {
        let names: Vec<&str> = if gpu_names.is_empty() {
            vec!["No GPUs detected"]
        } else {
            gpu_names.iter().map(|s| s.as_str()).collect()
        };

        let gpu_list = StringList::new(&names);
        self.gpu_combo.set_model(Some(&gpu_list));

        if !gpu_names.is_empty() {
            self.gpu_combo.set_selected(0);
        }
    }
}

impl Default for GpuSourceConfigWidget {
    fn default() -> Self {
        Self::new()
    }
}
