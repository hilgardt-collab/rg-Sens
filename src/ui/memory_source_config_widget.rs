//! Memory source configuration widget

use gtk4::prelude::*;
use gtk4::{Box as GtkBox, DropDown, Entry, Label, Orientation, SpinButton, StringList};
use std::cell::RefCell;
use std::rc::Rc;

use crate::ui::widget_builder::create_page_container;

// Re-export memory source config types from rg-sens-types
pub use rg_sens_types::source_configs::gpu::MemoryUnit;
pub use rg_sens_types::source_configs::memory::{MemoryField, MemorySourceConfig};

/// Widget for configuring memory source
pub struct MemorySourceConfigWidget {
    widget: GtkBox,
    config: Rc<RefCell<MemorySourceConfig>>,
    caption_entry: Entry,
    field_combo: DropDown,
    unit_combo: DropDown,
    unit_box: GtkBox,
    update_interval_spin: SpinButton,
}

impl MemorySourceConfigWidget {
    pub fn new() -> Self {
        let config = Rc::new(RefCell::new(MemorySourceConfig::default()));

        let widget = create_page_container();

        // Caption
        let caption_label = Label::new(Some("Custom Caption (optional):"));
        caption_label.set_halign(gtk4::Align::Start);
        widget.append(&caption_label);

        let caption_entry = Entry::new();
        caption_entry.set_placeholder_text(Some("Leave empty for auto-generated"));
        widget.append(&caption_entry);

        // Field selection
        let field_label = Label::new(Some("Display Field:"));
        field_label.set_halign(gtk4::Align::Start);
        field_label.set_margin_top(12);
        widget.append(&field_label);

        let field_options = StringList::new(&[
            "RAM Used",
            "RAM Free",
            "RAM Available",
            "RAM Percent",
            "Swap Used",
            "Swap Percent",
        ]);
        let field_combo = DropDown::new(Some(field_options), Option::<gtk4::Expression>::None);
        field_combo.set_selected(3); // Default to Percent
        widget.append(&field_combo);

        // Memory unit selection
        let unit_label = Label::new(Some("Memory Unit:"));
        unit_label.set_halign(gtk4::Align::Start);
        unit_label.set_margin_top(12);

        let unit_options = StringList::new(&["MB", "GB"]);
        let unit_combo = DropDown::new(Some(unit_options), Option::<gtk4::Expression>::None);
        unit_combo.set_selected(1); // Default to GB

        let unit_box = GtkBox::new(Orientation::Vertical, 6);
        unit_box.append(&unit_label);
        unit_box.append(&unit_combo);
        widget.append(&unit_box);

        // Update interval
        let interval_label = Label::new(Some("Update Interval (ms):"));
        interval_label.set_halign(gtk4::Align::Start);
        interval_label.set_margin_top(12);
        widget.append(&interval_label);

        let update_interval_spin = SpinButton::with_range(100.0, 10000.0, 100.0);
        update_interval_spin.set_value(1000.0);
        widget.append(&update_interval_spin);

        // Setup change handlers
        let config_clone = config.clone();
        let unit_box_clone = unit_box.clone();
        field_combo.connect_selected_notify(move |combo| {
            let mut cfg = config_clone.borrow_mut();
            cfg.field = MemoryField::from_index(combo.selected());

            // Show/hide unit selector based on field
            let field = cfg.field;
            let show_unit = matches!(
                field,
                MemoryField::Used
                    | MemoryField::Free
                    | MemoryField::Available
                    | MemoryField::SwapUsed
            );
            unit_box_clone.set_visible(show_unit);
        });

        let config_clone = config.clone();
        unit_combo.connect_selected_notify(move |combo| {
            let mut cfg = config_clone.borrow_mut();
            cfg.memory_unit = if combo.selected() == 0 {
                MemoryUnit::MB
            } else {
                MemoryUnit::GB
            };
        });

        let config_clone = config.clone();
        caption_entry.connect_changed(move |entry| {
            let mut cfg = config_clone.borrow_mut();
            let text = entry.text().to_string();
            cfg.custom_caption = if text.is_empty() { None } else { Some(text) };
        });

        let config_clone = config.clone();
        update_interval_spin.connect_value_changed(move |spin| {
            let mut cfg = config_clone.borrow_mut();
            cfg.update_interval_ms = spin.value() as u64;
        });

        Self {
            widget,
            config,
            caption_entry,
            field_combo,
            unit_combo,
            unit_box,
            update_interval_spin,
        }
    }

    pub fn widget(&self) -> &GtkBox {
        &self.widget
    }

    pub fn get_config(&self) -> MemorySourceConfig {
        self.config.borrow().clone()
    }

    pub fn set_config(&self, config: MemorySourceConfig) {
        // Update UI to reflect config
        self.field_combo.set_selected(config.field.to_index());

        let unit_index = match config.memory_unit {
            MemoryUnit::MB => 0,
            MemoryUnit::GB => 1,
        };
        self.unit_combo.set_selected(unit_index);

        if let Some(ref caption) = config.custom_caption {
            self.caption_entry.set_text(caption);
        } else {
            self.caption_entry.set_text("");
        }

        self.update_interval_spin
            .set_value(config.update_interval_ms as f64);

        // Show/hide unit selector based on field
        let show_unit = matches!(
            config.field,
            MemoryField::Used | MemoryField::Free | MemoryField::Available | MemoryField::SwapUsed
        );
        self.unit_box.set_visible(show_unit);

        // Update internal config
        *self.config.borrow_mut() = config;
    }
}

impl Default for MemorySourceConfigWidget {
    fn default() -> Self {
        Self::new()
    }
}
