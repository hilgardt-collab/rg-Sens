//! Disk usage source configuration widget

use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, CheckButton, DropDown, Entry, Label, Orientation, SpinButton, StringList,
};
use std::cell::RefCell;
use std::rc::Rc;

use crate::ui::widget_builder::create_page_container;

// Re-export disk source config types from rg-sens-types
pub use rg_sens_types::source_configs::disk::{DiskField, DiskSourceConfig, DiskUnit};

/// Widget for configuring disk source
#[allow(dead_code)]
pub struct DiskSourceConfigWidget {
    widget: GtkBox,
    config: Rc<RefCell<DiskSourceConfig>>,
    caption_entry: Entry,
    field_combo: DropDown,
    unit_combo: DropDown,
    unit_box: GtkBox,
    unit_label: Label,
    disk_combo: DropDown,
    update_interval_spin: SpinButton,
    min_limit_spin: SpinButton,
    max_limit_spin: SpinButton,
    auto_detect_check: CheckButton,
}

impl DiskSourceConfigWidget {
    pub fn new() -> Self {
        let widget = create_page_container();

        let config = Rc::new(RefCell::new(DiskSourceConfig::default()));

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

        let field_options =
            StringList::new(&["Used Space", "Free Space", "Total Space", "Usage Percent"]);
        let field_combo = DropDown::new(Some(field_options), Option::<gtk4::Expression>::None);
        field_combo.set_selected(3); // Percent by default
        field_box.append(&field_combo);
        widget.append(&field_box);

        // Unit selection (for space fields)
        let unit_box = GtkBox::new(Orientation::Horizontal, 6);
        let unit_label = Label::new(Some("Space Unit:"));
        unit_box.append(&unit_label);

        let unit_options = StringList::new(&["MB", "GB", "TB"]);
        let unit_combo = DropDown::new(Some(unit_options), Option::<gtk4::Expression>::None);
        unit_combo.set_selected(1); // GB by default
        unit_box.append(&unit_combo);
        unit_box.set_visible(false); // Hidden by default (percent doesn't need unit)
        widget.append(&unit_box);

        // Disk selection
        let disk_box = GtkBox::new(Orientation::Horizontal, 6);
        disk_box.append(&Label::new(Some("Disk:")));

        let disk_options = StringList::new(&["/ (Root)"]);
        let disk_combo = DropDown::new(Some(disk_options), Option::<gtk4::Expression>::None);
        disk_combo.set_selected(0);
        disk_box.append(&disk_combo);
        widget.append(&disk_box);

        // Update interval
        let interval_box = GtkBox::new(Orientation::Horizontal, 6);
        interval_box.append(&Label::new(Some("Update Interval (ms):")));

        let interval_adjustment = gtk4::Adjustment::new(2000.0, 500.0, 60000.0, 100.0, 1000.0, 0.0);
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
        let min_adjustment = gtk4::Adjustment::new(0.0, -1000.0, 10000.0, 1.0, 10.0, 0.0);
        let min_limit_spin = SpinButton::new(Some(&min_adjustment), 0.1, 2);
        min_limit_spin.set_hexpand(true);
        min_limit_spin.set_sensitive(false);
        limits_box.append(&min_limit_spin);

        limits_box.append(&Label::new(Some("Max:")));
        let max_adjustment = gtk4::Adjustment::new(100.0, -1000.0, 50000.0, 1.0, 10.0, 0.0);
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
        let unit_box_clone = unit_box.clone();
        field_combo.connect_selected_notify(move |combo| {
            let selected = combo.selected();
            let field = match selected {
                0 => DiskField::Used,
                1 => DiskField::Free,
                2 => DiskField::Total,
                3 => DiskField::Percent,
                _ => DiskField::Percent,
            };
            config_clone.borrow_mut().field = field;

            // Show/hide unit box based on field
            match field {
                DiskField::Used | DiskField::Free | DiskField::Total => {
                    unit_box_clone.set_visible(true);
                }
                DiskField::Percent => {
                    unit_box_clone.set_visible(false);
                }
            }
        });

        let config_clone = config.clone();
        unit_combo.connect_selected_notify(move |combo| {
            let selected = combo.selected();
            let unit = match selected {
                0 => DiskUnit::MB,
                1 => DiskUnit::GB,
                2 => DiskUnit::TB,
                _ => DiskUnit::GB,
            };
            config_clone.borrow_mut().disk_unit = unit;
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
            unit_label,
            disk_combo,
            update_interval_spin,
            min_limit_spin,
            max_limit_spin,
            auto_detect_check,
        }
    }

    pub fn widget(&self) -> &GtkBox {
        &self.widget
    }

    pub fn set_config(&self, config: DiskSourceConfig) {
        // Update UI widgets based on config
        self.field_combo.set_selected(match config.field {
            DiskField::Used => 0,
            DiskField::Free => 1,
            DiskField::Total => 2,
            DiskField::Percent => 3,
        });

        // Update unit box visibility based on field
        match config.field {
            DiskField::Used | DiskField::Free | DiskField::Total => {
                self.unit_combo.set_selected(match config.disk_unit {
                    DiskUnit::MB => 0,
                    DiskUnit::GB => 1,
                    DiskUnit::TB => 2,
                });
                self.unit_box.set_visible(true);
            }
            DiskField::Percent => {
                self.unit_box.set_visible(false);
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

    pub fn get_config(&self) -> DiskSourceConfig {
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

    /// Set available disks
    pub fn set_available_disks(&self, disks: &[(String, String)]) {
        let names: Vec<String> = if disks.is_empty() {
            vec!["No disks detected".to_string()]
        } else {
            disks
                .iter()
                .map(|(mount, name)| {
                    if name.is_empty() {
                        mount.clone()
                    } else {
                        format!("{} ({})", mount, name)
                    }
                })
                .collect()
        };

        let disk_list = StringList::new(&names.iter().map(|s| s.as_str()).collect::<Vec<_>>());
        self.disk_combo.set_model(Some(&disk_list));

        // Set up disk selection handler
        let config = self.config.clone();
        let disks_clone: Vec<(String, String)> = disks.to_vec();
        self.disk_combo.connect_selected_notify(move |combo| {
            let idx = combo.selected() as usize;
            if idx < disks_clone.len() {
                config.borrow_mut().disk_path = disks_clone[idx].0.clone();
            }
        });

        if !disks.is_empty() {
            // Find the index of the current disk_path
            let current_path = self.config.borrow().disk_path.clone();
            if let Some(idx) = disks.iter().position(|(path, _)| path == &current_path) {
                self.disk_combo.set_selected(idx as u32);
            } else {
                self.disk_combo.set_selected(0);
                self.config.borrow_mut().disk_path = disks[0].0.clone();
            }
        }
    }
}

impl Default for DiskSourceConfigWidget {
    fn default() -> Self {
        Self::new()
    }
}
