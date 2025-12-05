//! CPU source configuration widget

use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, CheckButton, DropDown, Label, Orientation, StringList,
};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::rc::Rc;

/// CPU source field types
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum CpuField {
    Temperature,
    Usage,
    Frequency,
}

/// Temperature units
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TemperatureUnit {
    Celsius,
    Fahrenheit,
    Kelvin,
}

/// CPU core selection
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CoreSelection {
    Overall,
    Core(usize),
}

/// CPU source configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuSourceConfig {
    pub field: CpuField,
    pub temp_unit: TemperatureUnit,
    pub sensor_index: usize,
    pub core_selection: CoreSelection,
}

impl Default for CpuSourceConfig {
    fn default() -> Self {
        Self {
            field: CpuField::Usage,
            temp_unit: TemperatureUnit::Celsius,
            sensor_index: 0,
            core_selection: CoreSelection::Overall,
        }
    }
}

/// Widget for configuring CPU source
pub struct CpuSourceConfigWidget {
    widget: GtkBox,
    config: Rc<RefCell<CpuSourceConfig>>,
    field_combo: DropDown,
    unit_combo: DropDown,
    unit_box: GtkBox,
    sensor_combo: DropDown,
    core_combo: DropDown,
    per_core_check: CheckButton,
}

impl CpuSourceConfigWidget {
    pub fn new() -> Self {
        let widget = GtkBox::new(Orientation::Vertical, 12);
        widget.set_margin_start(12);
        widget.set_margin_end(12);
        widget.set_margin_top(12);
        widget.set_margin_bottom(12);

        let config = Rc::new(RefCell::new(CpuSourceConfig::default()));

        // Field selection
        let field_box = GtkBox::new(Orientation::Horizontal, 6);
        field_box.append(&Label::new(Some("Data Field:")));

        let field_options = StringList::new(&["Temperature", "Usage", "Frequency"]);
        let field_combo = DropDown::new(Some(field_options), Option::<gtk4::Expression>::None);
        field_combo.set_selected(1); // Default to Usage

        field_box.append(&field_combo);
        widget.append(&field_box);

        // Unit selection (only for temperature)
        let unit_box = GtkBox::new(Orientation::Horizontal, 6);
        unit_box.append(&Label::new(Some("Temperature Unit:")));

        let unit_options = StringList::new(&["Celsius (°C)", "Fahrenheit (°F)", "Kelvin (K)"]);
        let unit_combo = DropDown::new(Some(unit_options), Option::<gtk4::Expression>::None);
        unit_combo.set_selected(0); // Default to Celsius

        unit_box.append(&unit_combo);
        unit_box.set_visible(false); // Hidden by default (show only for temperature)
        widget.append(&unit_box);

        // Sensor selection
        let sensor_box = GtkBox::new(Orientation::Horizontal, 6);
        sensor_box.append(&Label::new(Some("Sensor:")));

        let sensor_options = StringList::new(&["CPU Sensor 1"]); // Will be populated dynamically
        let sensor_combo = DropDown::new(Some(sensor_options), Option::<gtk4::Expression>::None);
        sensor_combo.set_selected(0);

        sensor_box.append(&sensor_combo);
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

        // Wire up handlers
        let config_clone = config.clone();
        let unit_box_clone = unit_box.clone();
        field_combo.connect_selected_notify(move |combo| {
            let selected = combo.selected();
            let field = match selected {
                0 => CpuField::Temperature,
                1 => CpuField::Usage,
                2 => CpuField::Frequency,
                _ => CpuField::Usage,
            };

            config_clone.borrow_mut().field = field;

            // Show/hide unit selector based on field
            unit_box_clone.set_visible(field == CpuField::Temperature);
        });

        let config_clone = config.clone();
        unit_combo.connect_selected_notify(move |combo| {
            let selected = combo.selected();
            let unit = match selected {
                0 => TemperatureUnit::Celsius,
                1 => TemperatureUnit::Fahrenheit,
                2 => TemperatureUnit::Kelvin,
                _ => TemperatureUnit::Celsius,
            };

            config_clone.borrow_mut().temp_unit = unit;
        });

        let config_clone = config.clone();
        sensor_combo.connect_selected_notify(move |combo| {
            config_clone.borrow_mut().sensor_index = combo.selected() as usize;
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
            let selection = if selected == 0 {
                CoreSelection::Overall
            } else {
                CoreSelection::Core(selected as usize - 1)
            };

            config_clone.borrow_mut().core_selection = selection;
        });

        Self {
            widget,
            config,
            field_combo,
            unit_combo,
            unit_box,
            sensor_combo,
            core_combo,
            per_core_check,
        }
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

        self.unit_box.set_visible(config.field == CpuField::Temperature);

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

        *self.config.borrow_mut() = config;
    }

    pub fn get_config(&self) -> CpuSourceConfig {
        self.config.borrow().clone()
    }

    /// Populate sensor dropdown with available CPU sensors
    pub fn set_available_sensors(&self, sensors: &[crate::sources::CpuSensor]) {
        let sensor_names: Vec<String> = if sensors.is_empty() {
            vec!["No sensors detected".to_string()]
        } else {
            sensors.iter().map(|s| s.label.clone()).collect()
        };

        let sensor_list = StringList::new(&sensor_names.iter().map(|s| s.as_str()).collect::<Vec<_>>());
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
