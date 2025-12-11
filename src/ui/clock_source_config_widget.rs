//! Configuration widget for Clock source
//!
//! Note: Alarm and Timer settings are now accessed via the alarm/timer dialog
//! which can be opened by clicking the bell icon on the clock panel.

use gtk4::prelude::*;
use gtk4::{
    Adjustment, Box as GtkBox, Button, CheckButton, DropDown, Label, Orientation,
    SpinButton, StringList,
};
use std::cell::RefCell;
use std::rc::Rc;

use crate::sources::{ClockSourceConfig, DateFormat, TimeFormat};
use crate::ui::TimezoneDialog;

/// Widget for configuring Clock source
pub struct ClockSourceConfigWidget {
    widget: GtkBox,
    config: Rc<RefCell<ClockSourceConfig>>,
    tz_button: Button,
    time_format_dropdown: DropDown,
    date_format_dropdown: DropDown,
    show_seconds_check: CheckButton,
    interval_spin: SpinButton,
}

impl ClockSourceConfigWidget {
    pub fn new() -> Self {
        let widget = GtkBox::new(Orientation::Vertical, 12);
        widget.set_margin_start(12);
        widget.set_margin_end(12);
        widget.set_margin_top(12);
        widget.set_margin_bottom(12);

        let config = Rc::new(RefCell::new(ClockSourceConfig::default()));

        // Time Format Section
        let time_section = create_section_label("Time Settings");
        widget.append(&time_section);

        // Time format dropdown
        let time_format_box = GtkBox::new(Orientation::Horizontal, 6);
        time_format_box.append(&Label::new(Some("Time Format:")));

        let time_format_options = StringList::new(&["24 Hour", "12 Hour"]);
        let time_format_dropdown = DropDown::new(Some(time_format_options), Option::<gtk4::Expression>::None);
        time_format_dropdown.set_selected(0);
        time_format_dropdown.set_hexpand(true);
        time_format_box.append(&time_format_dropdown);
        widget.append(&time_format_box);

        // Date format dropdown
        let date_format_box = GtkBox::new(Orientation::Horizontal, 6);
        date_format_box.append(&Label::new(Some("Date Format:")));

        let date_format_options = StringList::new(&[
            "YYYY-MM-DD",
            "DD/MM/YYYY",
            "MM/DD/YYYY",
            "Day, Month DD, YYYY",
        ]);
        let date_format_dropdown = DropDown::new(Some(date_format_options), Option::<gtk4::Expression>::None);
        date_format_dropdown.set_selected(0);
        date_format_dropdown.set_hexpand(true);
        date_format_box.append(&date_format_dropdown);
        widget.append(&date_format_box);

        // Show seconds checkbox
        let show_seconds_check = CheckButton::with_label("Show Seconds");
        show_seconds_check.set_active(true);
        widget.append(&show_seconds_check);

        // Timezone selection
        let tz_box = GtkBox::new(Orientation::Horizontal, 6);
        tz_box.append(&Label::new(Some("Timezone:")));

        let tz_button = Button::with_label("Local");
        tz_button.set_hexpand(true);
        tz_box.append(&tz_button);
        widget.append(&tz_box);

        // Update interval
        let interval_box = GtkBox::new(Orientation::Horizontal, 6);
        interval_box.append(&Label::new(Some("Update Interval (ms):")));

        let interval_adjustment = Adjustment::new(100.0, 16.0, 1000.0, 10.0, 100.0, 0.0);
        let interval_spin = SpinButton::new(Some(&interval_adjustment), 10.0, 0);
        interval_spin.set_hexpand(true);
        interval_box.append(&interval_spin);
        widget.append(&interval_box);

        // Note about alarm/timer
        let note_label = Label::new(Some("Tip: Click the bell icon on the clock panel to access alarm and timer controls."));
        note_label.set_halign(gtk4::Align::Start);
        note_label.set_wrap(true);
        note_label.set_margin_top(12);
        note_label.add_css_class("dim-label");
        widget.append(&note_label);

        // Connect signals
        let config_clone = config.clone();
        time_format_dropdown.connect_selected_notify(move |dropdown| {
            let mut cfg = config_clone.borrow_mut();
            cfg.time_format = match dropdown.selected() {
                0 => TimeFormat::Hour24,
                _ => TimeFormat::Hour12,
            };
        });

        let config_clone = config.clone();
        date_format_dropdown.connect_selected_notify(move |dropdown| {
            let mut cfg = config_clone.borrow_mut();
            cfg.date_format = match dropdown.selected() {
                0 => DateFormat::YearMonthDay,
                1 => DateFormat::DayMonthYear,
                2 => DateFormat::MonthDayYear,
                _ => DateFormat::LongFormat,
            };
        });

        let config_clone = config.clone();
        show_seconds_check.connect_toggled(move |check| {
            config_clone.borrow_mut().show_seconds = check.is_active();
        });

        // Timezone button click handler
        let config_clone = config.clone();
        let tz_button_clone = tz_button.clone();
        tz_button.connect_clicked(move |_btn| {
            let config_for_dialog = config_clone.clone();
            let btn_for_dialog = tz_button_clone.clone();
            let current_tz = config_for_dialog.borrow().timezone.clone();

            gtk4::glib::MainContext::default().spawn_local(async move {
                // Get the window from the button's ancestor
                let window = btn_for_dialog
                    .root()
                    .and_then(|r| r.downcast::<gtk4::Window>().ok());

                if let Some(tz) = TimezoneDialog::pick_timezone(window.as_ref(), &current_tz).await
                {
                    config_for_dialog.borrow_mut().timezone = tz.clone();
                    btn_for_dialog.set_label(&tz);
                }
            });
        });

        let config_clone = config.clone();
        interval_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().update_interval_ms = spin.value() as u64;
        });

        Self {
            widget,
            config,
            tz_button,
            time_format_dropdown,
            date_format_dropdown,
            show_seconds_check,
            interval_spin,
        }
    }

    pub fn widget(&self) -> &GtkBox {
        &self.widget
    }

    pub fn get_config(&self) -> ClockSourceConfig {
        self.config.borrow().clone()
    }

    pub fn set_config(&self, config: &ClockSourceConfig) {
        *self.config.borrow_mut() = config.clone();

        // Update UI widgets from config
        self.tz_button.set_label(&config.timezone);

        self.time_format_dropdown.set_selected(match config.time_format {
            TimeFormat::Hour24 => 0,
            TimeFormat::Hour12 => 1,
        });

        self.date_format_dropdown.set_selected(match config.date_format {
            DateFormat::YearMonthDay => 0,
            DateFormat::DayMonthYear => 1,
            DateFormat::MonthDayYear => 2,
            DateFormat::LongFormat => 3,
        });

        self.show_seconds_check.set_active(config.show_seconds);
        self.interval_spin.set_value(config.update_interval_ms as f64);
    }
}

impl Default for ClockSourceConfigWidget {
    fn default() -> Self {
        Self::new()
    }
}

fn create_section_label(text: &str) -> Label {
    let label = Label::new(Some(text));
    label.set_halign(gtk4::Align::Start);
    label.add_css_class("heading");
    label.set_margin_top(8);
    label
}
