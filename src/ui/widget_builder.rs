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

/// Builder for creating config widgets with automatic callback wiring.
///
/// This builder provides methods to create common widget patterns (spin buttons,
/// dropdowns, color pickers, etc.) with automatic connection to config updates
/// and preview redraws. All methods return the widget for storage in *Widgets structs.
///
/// # Example
/// ```ignore
/// let builder = ConfigWidgetBuilder::new(&config, &preview, &on_change);
///
/// // Create a spin button, connect it, append to page, and get the widget back
/// let corner_spin = builder.spin_row(
///     &page,
///     "Corner Radius:",
///     0.0, 32.0, 1.0,
///     config.borrow().frame.corner_radius,
///     |cfg, val| cfg.frame.corner_radius = val,
/// );
/// ```
pub struct ConfigWidgetBuilder<T: 'static> {
    config: Rc<RefCell<T>>,
    preview: DrawingArea,
    on_change: OnChangeCallback,
    theme_refreshers: Option<Rc<RefCell<Vec<Rc<dyn Fn()>>>>>,
}

impl<T: 'static> ConfigWidgetBuilder<T> {
    /// Create a new builder without theme refreshers.
    pub fn new(
        config: &Rc<RefCell<T>>,
        preview: &DrawingArea,
        on_change: &OnChangeCallback,
    ) -> Self {
        Self {
            config: config.clone(),
            preview: preview.clone(),
            on_change: on_change.clone(),
            theme_refreshers: None,
        }
    }

    /// Create a new builder with theme refreshers support.
    pub fn with_theme_refreshers(
        config: &Rc<RefCell<T>>,
        preview: &DrawingArea,
        on_change: &OnChangeCallback,
        theme_refreshers: &Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
    ) -> Self {
        Self {
            config: config.clone(),
            preview: preview.clone(),
            on_change: on_change.clone(),
            theme_refreshers: Some(theme_refreshers.clone()),
        }
    }

    /// Creates a spin button row, connects it, appends to container, and returns the SpinButton.
    pub fn spin_row<F>(
        &self,
        container: &GtkBox,
        label: &str,
        min: f64,
        max: f64,
        step: f64,
        initial: f64,
        update_fn: F,
    ) -> SpinButton
    where
        F: Fn(&mut T, f64) + 'static,
    {
        let (row, spin) = create_spin_row_with_value(label, min, max, step, initial);

        let config = self.config.clone();
        let preview = self.preview.clone();
        let on_change = self.on_change.clone();

        spin.connect_value_changed(move |spin| {
            update_fn(&mut config.borrow_mut(), spin.value());
            preview.queue_draw();
            if let Some(cb) = on_change.borrow().as_ref() {
                cb();
            }
        });

        container.append(&row);
        spin
    }

    /// Creates an integer spin button row.
    pub fn int_spin_row<F>(
        &self,
        container: &GtkBox,
        label: &str,
        min: i32,
        max: i32,
        initial: i32,
        update_fn: F,
    ) -> SpinButton
    where
        F: Fn(&mut T, i32) + 'static,
    {
        let (row, spin) = create_int_spin_row_with_value(label, min, max, initial);

        let config = self.config.clone();
        let preview = self.preview.clone();
        let on_change = self.on_change.clone();

        spin.connect_value_changed(move |spin| {
            update_fn(&mut config.borrow_mut(), spin.value() as i32);
            preview.queue_draw();
            if let Some(cb) = on_change.borrow().as_ref() {
                cb();
            }
        });

        container.append(&row);
        spin
    }

    /// Creates a dropdown row, connects it, appends to container, and returns the DropDown.
    ///
    /// The `update_fn` receives the selected index (validated, not INVALID_LIST_POSITION).
    pub fn dropdown_row<F>(
        &self,
        container: &GtkBox,
        label: &str,
        options: &[&str],
        initial: u32,
        update_fn: F,
    ) -> DropDown
    where
        F: Fn(&mut T, u32) + 'static,
    {
        let (row, dropdown) = create_dropdown_row_with_selected(label, options, initial);

        let config = self.config.clone();
        let preview = self.preview.clone();
        let on_change = self.on_change.clone();

        dropdown.connect_selected_notify(move |dropdown| {
            let selected = dropdown.selected();
            if selected == gtk4::INVALID_LIST_POSITION {
                return;
            }
            update_fn(&mut config.borrow_mut(), selected);
            preview.queue_draw();
            if let Some(cb) = on_change.borrow().as_ref() {
                cb();
            }
        });

        container.append(&row);
        dropdown
    }

    /// Creates a check button, connects it, appends to container, and returns the CheckButton.
    pub fn check_button<F>(
        &self,
        container: &GtkBox,
        label: &str,
        initial: bool,
        update_fn: F,
    ) -> CheckButton
    where
        F: Fn(&mut T, bool) + 'static,
    {
        let check = create_check_button(label, initial);

        let config = self.config.clone();
        let preview = self.preview.clone();
        let on_change = self.on_change.clone();

        check.connect_toggled(move |check| {
            update_fn(&mut config.borrow_mut(), check.is_active());
            preview.queue_draw();
            if let Some(cb) = on_change.borrow().as_ref() {
                cb();
            }
        });

        container.append(&check);
        check
    }

    /// Creates an entry row, connects it, appends to container, and returns the Entry.
    pub fn entry_row<F>(
        &self,
        container: &GtkBox,
        label: &str,
        initial: &str,
        update_fn: F,
    ) -> gtk4::Entry
    where
        F: Fn(&mut T, String) + 'static,
    {
        let entry = gtk4::Entry::new();
        entry.set_text(initial);
        entry.set_hexpand(true);

        let row = create_labeled_row(label, &entry);

        let config = self.config.clone();
        let preview = self.preview.clone();
        let on_change = self.on_change.clone();

        entry.connect_changed(move |entry| {
            update_fn(&mut config.borrow_mut(), entry.text().to_string());
            preview.queue_draw();
            if let Some(cb) = on_change.borrow().as_ref() {
                cb();
            }
        });

        container.append(&row);
        entry
    }

    /// Creates a scale (slider) row, connects it, appends to container, and returns the Scale.
    pub fn scale_row<F>(
        &self,
        container: &GtkBox,
        label: &str,
        min: f64,
        max: f64,
        step: f64,
        initial: f64,
        update_fn: F,
    ) -> gtk4::Scale
    where
        F: Fn(&mut T, f64) + 'static,
    {
        let scale = gtk4::Scale::with_range(Orientation::Horizontal, min, max, step);
        scale.set_value(initial);
        scale.set_hexpand(true);
        scale.set_draw_value(true);

        let row = create_labeled_row(label, &scale);

        let config = self.config.clone();
        let preview = self.preview.clone();
        let on_change = self.on_change.clone();

        scale.connect_value_changed(move |scale| {
            update_fn(&mut config.borrow_mut(), scale.value());
            preview.queue_draw();
            if let Some(cb) = on_change.borrow().as_ref() {
                cb();
            }
        });

        container.append(&row);
        scale
    }
}

/// Extension trait for ConfigWidgetBuilder to add ColorButtonWidget support.
///
/// This is separate because ColorButtonWidget is in a different module and
/// we want to avoid circular dependencies.
pub trait ConfigWidgetBuilderColorExt<T: 'static> {
    /// Creates a color button row using ColorButtonWidget.
    fn color_row<F>(
        &self,
        container: &GtkBox,
        label: &str,
        initial: crate::ui::background::Color,
        update_fn: F,
    ) -> Rc<crate::ui::color_button_widget::ColorButtonWidget>
    where
        F: Fn(&mut T, crate::ui::background::Color) + 'static;

    /// Creates a color button row that also triggers theme refreshers.
    fn theme_color_row<F>(
        &self,
        container: &GtkBox,
        label: &str,
        initial: crate::ui::background::Color,
        update_fn: F,
    ) -> Rc<crate::ui::color_button_widget::ColorButtonWidget>
    where
        F: Fn(&mut T, crate::ui::background::Color) + 'static;
}

impl<T: 'static> ConfigWidgetBuilderColorExt<T> for ConfigWidgetBuilder<T> {
    fn color_row<F>(
        &self,
        container: &GtkBox,
        label: &str,
        initial: crate::ui::background::Color,
        update_fn: F,
    ) -> Rc<crate::ui::color_button_widget::ColorButtonWidget>
    where
        F: Fn(&mut T, crate::ui::background::Color) + 'static,
    {
        use crate::ui::color_button_widget::ColorButtonWidget;

        let row = GtkBox::new(Orientation::Horizontal, ROW_SPACING);
        row.append(&Label::new(Some(label)));

        let color_widget = Rc::new(ColorButtonWidget::new(initial));
        row.append(color_widget.widget());
        color_widget.widget().set_hexpand(true);

        let config = self.config.clone();
        let preview = self.preview.clone();
        let on_change = self.on_change.clone();

        color_widget.set_on_change(move |color| {
            update_fn(&mut config.borrow_mut(), color);
            preview.queue_draw();
            if let Some(cb) = on_change.borrow().as_ref() {
                cb();
            }
        });

        container.append(&row);
        color_widget
    }

    fn theme_color_row<F>(
        &self,
        container: &GtkBox,
        label: &str,
        initial: crate::ui::background::Color,
        update_fn: F,
    ) -> Rc<crate::ui::color_button_widget::ColorButtonWidget>
    where
        F: Fn(&mut T, crate::ui::background::Color) + 'static,
    {
        use crate::ui::color_button_widget::ColorButtonWidget;
        use crate::ui::combo_config_base;

        let row = GtkBox::new(Orientation::Horizontal, ROW_SPACING);
        row.append(&Label::new(Some(label)));

        let color_widget = Rc::new(ColorButtonWidget::new(initial));
        row.append(color_widget.widget());
        color_widget.widget().set_hexpand(true);

        let config = self.config.clone();
        let preview = self.preview.clone();
        let on_change = self.on_change.clone();
        let refreshers = self.theme_refreshers.clone();

        color_widget.set_on_change(move |color| {
            update_fn(&mut config.borrow_mut(), color);
            if let Some(ref r) = refreshers {
                combo_config_base::refresh_theme_refs(r);
            }
            preview.queue_draw();
            if let Some(cb) = on_change.borrow().as_ref() {
                cb();
            }
        });

        container.append(&row);
        color_widget
    }
}

/// Extension trait for ConfigWidgetBuilder to add ThemeColorSelector support.
pub trait ConfigWidgetBuilderThemeSelectorExt<T: 'static> {
    /// Creates a theme color selector row.
    fn theme_color_selector_row<F>(
        &self,
        container: &GtkBox,
        label: &str,
        initial: crate::ui::theme::ColorSource,
        theme: crate::ui::theme::ComboThemeConfig,
        update_fn: F,
    ) -> Rc<crate::ui::theme_color_selector::ThemeColorSelector>
    where
        F: Fn(&mut T, crate::ui::theme::ColorSource) + 'static;

    /// Creates a theme font selector row.
    fn theme_font_selector_row<F>(
        &self,
        container: &GtkBox,
        label: &str,
        initial: crate::ui::theme::FontSource,
        theme: crate::ui::theme::ComboThemeConfig,
        update_fn: F,
    ) -> Rc<crate::ui::theme_font_selector::ThemeFontSelector>
    where
        F: Fn(&mut T, crate::ui::theme::FontSource) + 'static;
}

impl<T: 'static> ConfigWidgetBuilderThemeSelectorExt<T> for ConfigWidgetBuilder<T> {
    fn theme_color_selector_row<F>(
        &self,
        container: &GtkBox,
        label: &str,
        initial: crate::ui::theme::ColorSource,
        theme: crate::ui::theme::ComboThemeConfig,
        update_fn: F,
    ) -> Rc<crate::ui::theme_color_selector::ThemeColorSelector>
    where
        F: Fn(&mut T, crate::ui::theme::ColorSource) + 'static,
    {
        use crate::ui::theme_color_selector::ThemeColorSelector;

        let row = GtkBox::new(Orientation::Horizontal, ROW_SPACING);
        row.append(&Label::new(Some(label)));

        let selector = Rc::new(ThemeColorSelector::new(initial));
        selector.set_theme_config(theme);
        row.append(selector.widget());
        selector.widget().set_hexpand(true);

        let config = self.config.clone();
        let preview = self.preview.clone();
        let on_change = self.on_change.clone();

        selector.set_on_change(move |source| {
            update_fn(&mut config.borrow_mut(), source);
            preview.queue_draw();
            if let Some(cb) = on_change.borrow().as_ref() {
                cb();
            }
        });

        container.append(&row);
        selector
    }

    fn theme_font_selector_row<F>(
        &self,
        container: &GtkBox,
        label: &str,
        initial: crate::ui::theme::FontSource,
        theme: crate::ui::theme::ComboThemeConfig,
        update_fn: F,
    ) -> Rc<crate::ui::theme_font_selector::ThemeFontSelector>
    where
        F: Fn(&mut T, crate::ui::theme::FontSource) + 'static,
    {
        use crate::ui::theme_font_selector::ThemeFontSelector;

        let row = GtkBox::new(Orientation::Horizontal, ROW_SPACING);
        row.append(&Label::new(Some(label)));

        let selector = Rc::new(ThemeFontSelector::new(initial));
        selector.set_theme_config(theme);
        row.append(selector.widget());
        selector.widget().set_hexpand(true);

        let config = self.config.clone();
        let preview = self.preview.clone();
        let on_change = self.on_change.clone();

        selector.set_on_change(move |source| {
            update_fn(&mut config.borrow_mut(), source);
            preview.queue_draw();
            if let Some(cb) = on_change.borrow().as_ref() {
                cb();
            }
        });

        container.append(&row);
        selector
    }
}
