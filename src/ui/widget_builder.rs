//! Widget builder helpers for consistent UI construction
//!
//! This module provides helper functions to reduce boilerplate when creating
//! common GTK4 widget patterns used throughout the config widgets.

use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, CheckButton, DrawingArea, DropDown, Label, Orientation, SpinButton, StringList,
    Widget,
};
use std::cell::RefCell;
use std::rc::Rc;

/// Type alias for the common on_change callback pattern
pub type OnChangeCallback = Rc<RefCell<Option<Box<dyn Fn()>>>>;

/// Queue preview redraw and invoke change callback.
///
/// This is the standard pattern used after config changes in widgets with a preview:
/// ```ignore
/// preview.queue_draw();
/// if let Some(cb) = on_change.borrow().as_ref() {
///     cb();
/// }
/// ```
pub fn queue_redraw(preview: &DrawingArea, on_change: &OnChangeCallback) {
    preview.queue_draw();
    if let Some(cb) = on_change.borrow().as_ref() {
        cb();
    }
}

/// Invoke change callback without preview redraw.
///
/// Use this for config widgets that don't have a preview (e.g., data source configs).
pub fn notify_change(on_change: &OnChangeCallback) {
    if let Some(cb) = on_change.borrow().as_ref() {
        cb();
    }
}

/// Default margin used for page containers
pub const DEFAULT_MARGIN: i32 = 12;

/// Default spacing used for vertical containers
pub const DEFAULT_SPACING: i32 = 12;

/// Default spacing used for horizontal label+control rows
pub const ROW_SPACING: i32 = 6;

/// Creates a vertical box configured as a notebook page container with standard margins.
///
/// This is the common pattern for creating notebook page content:
/// ```ignore
/// let page = GtkBox::new(Orientation::Vertical, 12);
/// page.set_margin_start(12);
/// page.set_margin_end(12);
/// page.set_margin_top(12);
/// page.set_margin_bottom(12);
/// ```
pub fn create_page_container() -> GtkBox {
    create_page_container_with_spacing(DEFAULT_SPACING)
}

/// Creates a vertical box configured as a notebook page container with custom spacing.
pub fn create_page_container_with_spacing(spacing: i32) -> GtkBox {
    let page = GtkBox::new(Orientation::Vertical, spacing);
    page.set_margin_start(DEFAULT_MARGIN);
    page.set_margin_end(DEFAULT_MARGIN);
    page.set_margin_top(DEFAULT_MARGIN);
    page.set_margin_bottom(DEFAULT_MARGIN);
    page
}

/// Creates a vertical box with standard margins (no notebook page styling).
pub fn create_padded_box(orientation: Orientation, spacing: i32) -> GtkBox {
    let container = GtkBox::new(orientation, spacing);
    container.set_margin_start(DEFAULT_MARGIN);
    container.set_margin_end(DEFAULT_MARGIN);
    container.set_margin_top(DEFAULT_MARGIN);
    container.set_margin_bottom(DEFAULT_MARGIN);
    container
}

/// Creates a horizontal box containing a label and a widget.
///
/// This is the common pattern for labeled controls:
/// ```ignore
/// let row = GtkBox::new(Orientation::Horizontal, 6);
/// row.append(&Label::new(Some("Label:")));
/// widget.set_hexpand(true);
/// row.append(&widget);
/// ```
pub fn create_labeled_row<W: IsA<Widget>>(label_text: &str, widget: &W) -> GtkBox {
    let row = GtkBox::new(Orientation::Horizontal, ROW_SPACING);
    row.append(&Label::new(Some(label_text)));
    widget.set_hexpand(true);
    row.append(widget);
    row
}

/// Creates a horizontal box containing a label and widget without hexpand on the widget.
pub fn create_labeled_row_compact<W: IsA<Widget>>(label_text: &str, widget: &W) -> GtkBox {
    let row = GtkBox::new(Orientation::Horizontal, ROW_SPACING);
    row.append(&Label::new(Some(label_text)));
    row.append(widget);
    row
}

/// Creates a dropdown with the given options and returns both the row and dropdown.
///
/// Returns a tuple of (row_box, dropdown) so the dropdown can be stored for later use.
pub fn create_dropdown_row(label_text: &str, options: &[&str]) -> (GtkBox, DropDown) {
    let string_list = StringList::new(options);
    let dropdown = DropDown::new(Some(string_list), Option::<gtk4::Expression>::None);
    dropdown.set_selected(0);
    dropdown.set_hexpand(true);
    let row = create_labeled_row(label_text, &dropdown);
    (row, dropdown)
}

/// Creates a dropdown with the given options and initial selection.
pub fn create_dropdown_row_with_selected(
    label_text: &str,
    options: &[&str],
    selected: u32,
) -> (GtkBox, DropDown) {
    let (row, dropdown) = create_dropdown_row(label_text, options);
    dropdown.set_selected(selected);
    (row, dropdown)
}

/// Creates a spin button with the given range and returns both the row and spin button.
///
/// Returns a tuple of (row_box, spin_button) so the spin button can be stored for later use.
pub fn create_spin_row(label_text: &str, min: f64, max: f64, step: f64) -> (GtkBox, SpinButton) {
    let spin = SpinButton::with_range(min, max, step);
    spin.set_hexpand(true);
    let row = create_labeled_row(label_text, &spin);
    (row, spin)
}

/// Creates a spin button with the given range and initial value.
pub fn create_spin_row_with_value(
    label_text: &str,
    min: f64,
    max: f64,
    step: f64,
    value: f64,
) -> (GtkBox, SpinButton) {
    let (row, spin) = create_spin_row(label_text, min, max, step);
    spin.set_value(value);
    (row, spin)
}

/// Creates a spin button configured for integer values.
pub fn create_int_spin_row(label_text: &str, min: i32, max: i32) -> (GtkBox, SpinButton) {
    create_spin_row(label_text, min as f64, max as f64, 1.0)
}

/// Creates a spin button configured for integer values with an initial value.
pub fn create_int_spin_row_with_value(
    label_text: &str,
    min: i32,
    max: i32,
    value: i32,
) -> (GtkBox, SpinButton) {
    let (row, spin) = create_int_spin_row(label_text, min, max);
    spin.set_value(value as f64);
    (row, spin)
}

/// Creates a spin button configured for percentage values (0-100).
pub fn create_percent_spin_row(label_text: &str) -> (GtkBox, SpinButton) {
    create_spin_row(label_text, 0.0, 100.0, 1.0)
}

/// Creates a spin button configured for percentage values with an initial value.
pub fn create_percent_spin_row_with_value(label_text: &str, value: f64) -> (GtkBox, SpinButton) {
    let (row, spin) = create_percent_spin_row(label_text);
    spin.set_value(value);
    (row, spin)
}

/// Creates a check button with the given label.
pub fn create_check_button(label_text: &str, active: bool) -> CheckButton {
    let check = CheckButton::with_label(label_text);
    check.set_active(active);
    check
}

/// Creates a horizontal box with radio-style check buttons (mutually exclusive).
///
/// The first option is active by default.
pub fn create_radio_row(options: &[&str]) -> (GtkBox, Vec<CheckButton>) {
    let row = GtkBox::new(Orientation::Horizontal, DEFAULT_SPACING);
    let mut buttons = Vec::with_capacity(options.len());

    for (i, &label) in options.iter().enumerate() {
        let radio = CheckButton::with_label(label);
        if i == 0 {
            radio.set_active(true);
        } else if let Some(first) = buttons.first() {
            radio.set_group(Some(first));
        }
        row.append(&radio);
        buttons.push(radio);
    }

    (row, buttons)
}

/// Creates a horizontal box with radio-style check buttons with a specific one active.
pub fn create_radio_row_with_active(options: &[&str], active_index: usize) -> (GtkBox, Vec<CheckButton>) {
    let (row, buttons) = create_radio_row(options);
    if let Some(button) = buttons.get(active_index) {
        button.set_active(true);
    }
    (row, buttons)
}

/// Helper struct for connecting spin button changes with preview updates.
pub struct SpinChangeHandler<T: 'static> {
    pub config: Rc<RefCell<T>>,
    pub preview: gtk4::DrawingArea,
    pub on_change: Rc<RefCell<Option<Box<dyn Fn()>>>>,
}

impl<T: 'static> SpinChangeHandler<T> {
    pub fn new(
        config: Rc<RefCell<T>>,
        preview: gtk4::DrawingArea,
        on_change: Rc<RefCell<Option<Box<dyn Fn()>>>>,
    ) -> Self {
        Self {
            config,
            preview,
            on_change,
        }
    }

    /// Connects a spin button value change to update config and trigger preview/callback.
    pub fn connect_spin<F>(&self, spin: &SpinButton, update_fn: F)
    where
        F: Fn(&mut T, f64) + 'static,
    {
        let config = self.config.clone();
        let preview = self.preview.clone();
        let on_change = self.on_change.clone();

        spin.connect_value_changed(move |spin| {
            update_fn(&mut config.borrow_mut(), spin.value());
            preview.queue_draw();
            if let Some(callback) = on_change.borrow().as_ref() {
                callback();
            }
        });
    }

    /// Connects a spin button that stores an integer value.
    pub fn connect_spin_int<F>(&self, spin: &SpinButton, update_fn: F)
    where
        F: Fn(&mut T, i32) + 'static,
    {
        let config = self.config.clone();
        let preview = self.preview.clone();
        let on_change = self.on_change.clone();

        spin.connect_value_changed(move |spin| {
            update_fn(&mut config.borrow_mut(), spin.value() as i32);
            preview.queue_draw();
            if let Some(callback) = on_change.borrow().as_ref() {
                callback();
            }
        });
    }

    /// Connects a spin button that stores a percentage (0-1) from a 0-100 range spin.
    pub fn connect_spin_percent<F>(&self, spin: &SpinButton, update_fn: F)
    where
        F: Fn(&mut T, f64) + 'static,
    {
        let config = self.config.clone();
        let preview = self.preview.clone();
        let on_change = self.on_change.clone();

        spin.connect_value_changed(move |spin| {
            update_fn(&mut config.borrow_mut(), spin.value() / 100.0);
            preview.queue_draw();
            if let Some(callback) = on_change.borrow().as_ref() {
                callback();
            }
        });
    }

    /// Connects a dropdown selection change.
    pub fn connect_dropdown<F>(&self, dropdown: &DropDown, update_fn: F)
    where
        F: Fn(&mut T, u32) + 'static,
    {
        let config = self.config.clone();
        let preview = self.preview.clone();
        let on_change = self.on_change.clone();

        dropdown.connect_selected_notify(move |dropdown| {
            update_fn(&mut config.borrow_mut(), dropdown.selected());
            preview.queue_draw();
            if let Some(callback) = on_change.borrow().as_ref() {
                callback();
            }
        });
    }

    /// Connects a check button toggle.
    pub fn connect_check<F>(&self, check: &CheckButton, update_fn: F)
    where
        F: Fn(&mut T, bool) + 'static,
    {
        let config = self.config.clone();
        let preview = self.preview.clone();
        let on_change = self.on_change.clone();

        check.connect_toggled(move |check| {
            update_fn(&mut config.borrow_mut(), check.is_active());
            preview.queue_draw();
            if let Some(callback) = on_change.borrow().as_ref() {
                callback();
            }
        });
    }
}

/// Creates a section header label with bold styling.
pub fn create_section_header(text: &str) -> Label {
    let label = Label::new(Some(text));
    label.set_halign(gtk4::Align::Start);
    label.add_css_class("heading");
    label
}

/// Creates a vertical separator (horizontal line).
pub fn create_separator() -> gtk4::Separator {
    gtk4::Separator::new(Orientation::Horizontal)
}
