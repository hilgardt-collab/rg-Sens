//! Network source configuration widget

use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, CheckButton, DropDown, Entry, Label, Orientation, SpinButton, StringList,
};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::rc::Rc;

use crate::ui::widget_builder::create_page_container;

/// Network source field types
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub enum NetworkField {
    #[default]
    DownloadSpeed,
    UploadSpeed,
    TotalDownload,
    TotalUpload,
}

/// Network speed unit types
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub enum NetworkSpeedUnit {
    BytesPerSec,
    #[default]
    KBPerSec,
    MBPerSec,
    GBPerSec,
}

/// Network total data unit types
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub enum NetworkTotalUnit {
    Bytes,
    KB,
    #[default]
    MB,
    GB,
}

/// Network source configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSourceConfig {
    pub field: NetworkField,
    #[serde(default)]
    pub speed_unit: NetworkSpeedUnit,
    #[serde(default)]
    pub total_unit: NetworkTotalUnit,
    pub interface: String,
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
    1000 // 1 second default for network stats
}

fn default_auto_detect_limits() -> bool {
    true
}

impl Default for NetworkSourceConfig {
    fn default() -> Self {
        Self {
            field: NetworkField::DownloadSpeed,
            speed_unit: NetworkSpeedUnit::KBPerSec,
            total_unit: NetworkTotalUnit::MB,
            interface: "".to_string(), // Will be set to first available interface
            custom_caption: None,
            update_interval_ms: default_update_interval(),
            min_limit: None,
            max_limit: Some(100.0), // 100 KB/s default max
            auto_detect_limits: default_auto_detect_limits(),
        }
    }
}

/// Widget for configuring network source
#[allow(dead_code)]
pub struct NetworkSourceConfigWidget {
    widget: GtkBox,
    config: Rc<RefCell<NetworkSourceConfig>>,
    caption_entry: Entry,
    field_combo: DropDown,
    speed_unit_combo: DropDown,
    speed_unit_box: GtkBox,
    total_unit_combo: DropDown,
    total_unit_box: GtkBox,
    interface_combo: DropDown,
    update_interval_spin: SpinButton,
    min_limit_spin: SpinButton,
    max_limit_spin: SpinButton,
    auto_detect_check: CheckButton,
}

impl NetworkSourceConfigWidget {
    pub fn new() -> Self {
        let widget = create_page_container();

        let config = Rc::new(RefCell::new(NetworkSourceConfig::default()));

        // Custom caption
        let caption_box = GtkBox::new(Orientation::Horizontal, 6);
        caption_box.append(&Label::new(Some("Custom Caption:")));
        let caption_entry = Entry::new();
        caption_entry.set_placeholder_text(Some("Auto-generated if empty"));
        caption_entry.set_hexpand(true);
        caption_box.append(&caption_entry);
        widget.append(&caption_box);

        // Interface selection
        let interface_box = GtkBox::new(Orientation::Horizontal, 6);
        interface_box.append(&Label::new(Some("Interface:")));

        let interface_options = StringList::new(&["No interfaces detected"]);
        let interface_combo =
            DropDown::new(Some(interface_options), Option::<gtk4::Expression>::None);
        interface_combo.set_selected(0);
        interface_box.append(&interface_combo);
        widget.append(&interface_box);

        // Field selection
        let field_box = GtkBox::new(Orientation::Horizontal, 6);
        field_box.append(&Label::new(Some("Field:")));

        let field_options = StringList::new(&[
            "Download Speed",
            "Upload Speed",
            "Total Downloaded",
            "Total Uploaded",
        ]);
        let field_combo = DropDown::new(Some(field_options), Option::<gtk4::Expression>::None);
        field_combo.set_selected(0); // Download Speed by default
        field_box.append(&field_combo);
        widget.append(&field_box);

        // Speed unit selection (for speed fields)
        let speed_unit_box = GtkBox::new(Orientation::Horizontal, 6);
        speed_unit_box.append(&Label::new(Some("Speed Unit:")));

        let speed_unit_options = StringList::new(&["B/s", "KB/s", "MB/s", "GB/s"]);
        let speed_unit_combo =
            DropDown::new(Some(speed_unit_options), Option::<gtk4::Expression>::None);
        speed_unit_combo.set_selected(1); // KB/s by default
        speed_unit_box.append(&speed_unit_combo);
        widget.append(&speed_unit_box);

        // Total unit selection (for total fields)
        let total_unit_box = GtkBox::new(Orientation::Horizontal, 6);
        total_unit_box.append(&Label::new(Some("Data Unit:")));

        let total_unit_options = StringList::new(&["Bytes", "KB", "MB", "GB"]);
        let total_unit_combo =
            DropDown::new(Some(total_unit_options), Option::<gtk4::Expression>::None);
        total_unit_combo.set_selected(2); // MB by default
        total_unit_box.append(&total_unit_combo);
        total_unit_box.set_visible(false); // Hidden by default (speed is default)
        widget.append(&total_unit_box);

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
        let min_adjustment = gtk4::Adjustment::new(0.0, 0.0, 1000000.0, 1.0, 10.0, 0.0);
        let min_limit_spin = SpinButton::new(Some(&min_adjustment), 0.1, 2);
        min_limit_spin.set_hexpand(true);
        min_limit_spin.set_sensitive(false);
        limits_box.append(&min_limit_spin);

        limits_box.append(&Label::new(Some("Max:")));
        let max_adjustment = gtk4::Adjustment::new(100.0, 0.0, 1000000.0, 1.0, 10.0, 0.0);
        let max_limit_spin = SpinButton::new(Some(&max_adjustment), 0.1, 2);
        max_limit_spin.set_hexpand(true);
        max_limit_spin.set_sensitive(false);
        limits_box.append(&max_limit_spin);

        widget.append(&limits_box);

        // Wire up handlers
        let config_clone = config.clone();
        caption_entry.connect_changed(move |entry| {
            let text = entry.text().to_string();
            config_clone.borrow_mut().custom_caption =
                if text.is_empty() { None } else { Some(text) };
        });

        let config_clone = config.clone();
        let speed_unit_box_clone = speed_unit_box.clone();
        let total_unit_box_clone = total_unit_box.clone();
        field_combo.connect_selected_notify(move |combo| {
            let selected = combo.selected();
            let field = match selected {
                0 => NetworkField::DownloadSpeed,
                1 => NetworkField::UploadSpeed,
                2 => NetworkField::TotalDownload,
                3 => NetworkField::TotalUpload,
                _ => NetworkField::DownloadSpeed,
            };
            config_clone.borrow_mut().field = field;

            // Show/hide unit boxes based on field
            match field {
                NetworkField::DownloadSpeed | NetworkField::UploadSpeed => {
                    speed_unit_box_clone.set_visible(true);
                    total_unit_box_clone.set_visible(false);
                }
                NetworkField::TotalDownload | NetworkField::TotalUpload => {
                    speed_unit_box_clone.set_visible(false);
                    total_unit_box_clone.set_visible(true);
                }
            }
        });

        let config_clone = config.clone();
        speed_unit_combo.connect_selected_notify(move |combo| {
            let selected = combo.selected();
            let unit = match selected {
                0 => NetworkSpeedUnit::BytesPerSec,
                1 => NetworkSpeedUnit::KBPerSec,
                2 => NetworkSpeedUnit::MBPerSec,
                3 => NetworkSpeedUnit::GBPerSec,
                _ => NetworkSpeedUnit::KBPerSec,
            };
            config_clone.borrow_mut().speed_unit = unit;
        });

        let config_clone = config.clone();
        total_unit_combo.connect_selected_notify(move |combo| {
            let selected = combo.selected();
            let unit = match selected {
                0 => NetworkTotalUnit::Bytes,
                1 => NetworkTotalUnit::KB,
                2 => NetworkTotalUnit::MB,
                3 => NetworkTotalUnit::GB,
                _ => NetworkTotalUnit::MB,
            };
            config_clone.borrow_mut().total_unit = unit;
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
            speed_unit_combo,
            speed_unit_box,
            total_unit_combo,
            total_unit_box,
            interface_combo,
            update_interval_spin,
            min_limit_spin,
            max_limit_spin,
            auto_detect_check,
        }
    }

    pub fn widget(&self) -> &GtkBox {
        &self.widget
    }

    pub fn set_config(&self, config: NetworkSourceConfig) {
        // Update UI widgets based on config
        self.field_combo.set_selected(match config.field {
            NetworkField::DownloadSpeed => 0,
            NetworkField::UploadSpeed => 1,
            NetworkField::TotalDownload => 2,
            NetworkField::TotalUpload => 3,
        });

        // Update unit boxes visibility and selection based on field
        match config.field {
            NetworkField::DownloadSpeed | NetworkField::UploadSpeed => {
                self.speed_unit_combo.set_selected(match config.speed_unit {
                    NetworkSpeedUnit::BytesPerSec => 0,
                    NetworkSpeedUnit::KBPerSec => 1,
                    NetworkSpeedUnit::MBPerSec => 2,
                    NetworkSpeedUnit::GBPerSec => 3,
                });
                self.speed_unit_box.set_visible(true);
                self.total_unit_box.set_visible(false);
            }
            NetworkField::TotalDownload | NetworkField::TotalUpload => {
                self.total_unit_combo.set_selected(match config.total_unit {
                    NetworkTotalUnit::Bytes => 0,
                    NetworkTotalUnit::KB => 1,
                    NetworkTotalUnit::MB => 2,
                    NetworkTotalUnit::GB => 3,
                });
                self.speed_unit_box.set_visible(false);
                self.total_unit_box.set_visible(true);
            }
        }

        if let Some(ref caption) = config.custom_caption {
            self.caption_entry.set_text(caption);
        } else {
            self.caption_entry.set_text("");
        }

        self.update_interval_spin
            .set_value(config.update_interval_ms as f64);

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

    pub fn get_config(&self) -> NetworkSourceConfig {
        let mut config = self.config.borrow().clone();

        // When auto_detect is disabled, ensure we use the spinbutton values
        if !config.auto_detect_limits {
            config.min_limit = Some(self.min_limit_spin.value());
            config.max_limit = Some(self.max_limit_spin.value());
        }

        config
    }

    /// Set available network interfaces
    pub fn set_available_interfaces(&self, interfaces: &[String]) {
        let names: Vec<String> = if interfaces.is_empty() {
            vec!["No interfaces detected".to_string()]
        } else {
            interfaces.to_vec()
        };

        let interface_list = StringList::new(&names.iter().map(|s| s.as_str()).collect::<Vec<_>>());
        self.interface_combo.set_model(Some(&interface_list));

        // Set up interface selection handler
        let config = self.config.clone();
        let interfaces_clone: Vec<String> = interfaces.to_vec();
        self.interface_combo.connect_selected_notify(move |combo| {
            let idx = combo.selected() as usize;
            if idx < interfaces_clone.len() {
                config.borrow_mut().interface = interfaces_clone[idx].clone();
            }
        });

        if !interfaces.is_empty() {
            // Find the index of the current interface
            let current_interface = self.config.borrow().interface.clone();
            if let Some(idx) = interfaces
                .iter()
                .position(|iface| iface == &current_interface)
            {
                self.interface_combo.set_selected(idx as u32);
            } else {
                self.interface_combo.set_selected(0);
                self.config.borrow_mut().interface = interfaces[0].clone();
            }
        }
    }
}

impl Default for NetworkSourceConfigWidget {
    fn default() -> Self {
        Self::new()
    }
}
