//! Gradient editor widget for configuring linear and radial gradients

use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, DrawingArea, Label, ListBox, ListBoxRow, Orientation, Scale, SpinButton};
use std::cell::RefCell;
use std::rc::Rc;

use crate::ui::background::{Color, ColorStop, LinearGradientConfig};
use crate::ui::color_picker::ColorPickerDialog;

/// Gradient editor widget
pub struct GradientEditor {
    container: GtkBox,
    stops: Rc<RefCell<Vec<ColorStop>>>,
    angle: Rc<RefCell<f64>>,
    on_change: Rc<RefCell<Option<std::boxed::Box<dyn Fn()>>>>,
    preview: DrawingArea,
    stops_listbox: ListBox,
    angle_scale: Scale,
    angle_spin: SpinButton,
}

impl GradientEditor {
    pub fn new() -> Self {
        let container = GtkBox::new(Orientation::Vertical, 12);
        container.set_margin_start(12);
        container.set_margin_end(12);
        container.set_margin_top(12);
        container.set_margin_bottom(12);
        container.set_vexpand(true);

        let stops = Rc::new(RefCell::new(Vec::new()));
        let angle = Rc::new(RefCell::new(90.0));
        let on_change: Rc<RefCell<Option<std::boxed::Box<dyn Fn()>>>> = Rc::new(RefCell::new(None));

        // Preview area (created early so angle handlers can reference it)
        let preview = DrawingArea::new();
        preview.set_content_height(100);
        preview.set_vexpand(false);

        let stops_clone = stops.clone();
        let angle_clone = angle.clone();
        preview.set_draw_func(move |_, cr, width, height| {
            use crate::ui::background::render_background;
            use crate::ui::background::{BackgroundConfig, BackgroundType, LinearGradientConfig};

            // Render checkerboard pattern to show transparency
            Self::render_checkerboard(cr, width as f64, height as f64);

            let stops = stops_clone.borrow();
            let angle = *angle_clone.borrow();

            let config = BackgroundConfig {
                background: BackgroundType::LinearGradient(LinearGradientConfig {
                    angle,
                    stops: stops.clone(),
                }),
            };

            let _ = render_background(cr, &config, width as f64, height as f64);
        });

        // Angle control
        let angle_box = GtkBox::new(Orientation::Horizontal, 6);
        angle_box.append(&Label::new(Some("Angle:")));

        let angle_scale = Scale::with_range(Orientation::Horizontal, 0.0, 360.0, 1.0);
        angle_scale.set_hexpand(true);
        angle_scale.set_value(90.0);

        let angle_spin = SpinButton::with_range(0.0, 360.0, 1.0);
        angle_spin.set_value(90.0);
        angle_spin.set_digits(0);

        // Sync scale and spin button
        let angle_clone = angle.clone();
        let angle_spin_clone = angle_spin.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        angle_scale.connect_value_changed(move |scale| {
            let value = scale.value();
            angle_spin_clone.set_value(value);
            *angle_clone.borrow_mut() = value;
            preview_clone.queue_draw();
            if let Some(callback) = on_change_clone.borrow().as_ref() {
                callback();
            }
        });

        let angle_scale_clone = angle_scale.clone();
        let angle_clone2 = angle.clone();
        let on_change_clone2 = on_change.clone();
        let preview_clone2 = preview.clone();
        angle_spin.connect_value_changed(move |spin| {
            let value = spin.value();
            angle_scale_clone.set_value(value);
            *angle_clone2.borrow_mut() = value;
            preview_clone2.queue_draw();
            if let Some(callback) = on_change_clone2.borrow().as_ref() {
                callback();
            }
        });

        angle_box.append(&angle_scale);
        angle_box.append(&angle_spin);
        container.append(&angle_box);

        container.append(&preview);

        // Color stops header with Add button
        let header_box = GtkBox::new(Orientation::Horizontal, 6);
        let stops_label = Label::new(Some("Color Stops:"));
        stops_label.set_halign(gtk4::Align::Start);
        stops_label.set_hexpand(true);
        header_box.append(&stops_label);

        let add_button = Button::with_label("Add Stop");
        header_box.append(&add_button);
        container.append(&header_box);

        // Stops list
        let stops_listbox = ListBox::new();
        stops_listbox.set_selection_mode(gtk4::SelectionMode::None);
        stops_listbox.add_css_class("boxed-list");

        let scroll = gtk4::ScrolledWindow::new();
        scroll.set_child(Some(&stops_listbox));
        scroll.set_vexpand(true);
        scroll.set_propagate_natural_height(false);
        container.append(&scroll);

        // Add stop button handler
        let stops_clone = stops.clone();
        let stops_listbox_clone = stops_listbox.clone();
        let preview_clone = preview.clone();
        let on_change_clone = on_change.clone();

        add_button.connect_clicked(move |_| {
            let mut stops_list = stops_clone.borrow_mut();

            // Find a good position for the new stop
            let position = if stops_list.is_empty() {
                0.5
            } else {
                let mut positions: Vec<f64> = stops_list.iter().map(|s| s.position).collect();
                positions.sort_by(|a, b| a.partial_cmp(b).unwrap());

                let mut max_gap = positions[0];
                let mut max_gap_pos = positions[0] / 2.0;

                for i in 0..positions.len() - 1 {
                    let gap = positions[i + 1] - positions[i];
                    if gap > max_gap {
                        max_gap = gap;
                        max_gap_pos = (positions[i] + positions[i + 1]) / 2.0;
                    }
                }

                if 1.0 - positions.last().unwrap() > max_gap {
                    (1.0 + positions.last().unwrap()) / 2.0
                } else {
                    max_gap_pos
                }
            };

            let new_stop = ColorStop::new(position, Color::new(0.5, 0.5, 0.5, 1.0));
            stops_list.push(new_stop);
            stops_list.sort_by(|a, b| a.position.partial_cmp(&b.position).unwrap());

            drop(stops_list);

            Self::rebuild_stops_list(
                &stops_listbox_clone,
                &stops_clone,
                &preview_clone,
                &on_change_clone,
            );

            preview_clone.queue_draw();

            if let Some(callback) = on_change_clone.borrow().as_ref() {
                callback();
            }
        });

        let editor = Self {
            container,
            stops,
            angle,
            on_change,
            preview,
            stops_listbox,
            angle_scale,
            angle_spin,
        };

        editor
    }

    /// Render a checkerboard pattern to show transparency
    fn render_checkerboard(cr: &gtk4::cairo::Context, width: f64, height: f64) {
        let square_size = 10.0;
        let light_gray = 0.8;
        let dark_gray = 0.6;

        for y in 0..((height / square_size).ceil() as i32) {
            for x in 0..((width / square_size).ceil() as i32) {
                let is_light = (x + y) % 2 == 0;
                let gray = if is_light { light_gray } else { dark_gray };

                cr.set_source_rgb(gray, gray, gray);
                cr.rectangle(
                    x as f64 * square_size,
                    y as f64 * square_size,
                    square_size,
                    square_size,
                );
                let _ = cr.fill();
            }
        }
    }

    /// Rebuild the stops list UI
    fn rebuild_stops_list(
        listbox: &ListBox,
        stops: &Rc<RefCell<Vec<ColorStop>>>,
        preview: &DrawingArea,
        on_change: &Rc<RefCell<Option<std::boxed::Box<dyn Fn()>>>>,
    ) {
        // Clear existing rows
        while let Some(child) = listbox.first_child() {
            listbox.remove(&child);
        }

        let stops_ref = stops.borrow();
        let stop_count = stops_ref.len();

        for (index, stop) in stops_ref.iter().enumerate() {
            let row = Self::create_stop_row(
                index,
                stop,
                stop_count,
                stops,
                listbox,
                preview,
                on_change,
            );
            listbox.append(&row);
        }
    }

    /// Create a row for a color stop
    fn create_stop_row(
        index: usize,
        stop: &ColorStop,
        stop_count: usize,
        stops: &Rc<RefCell<Vec<ColorStop>>>,
        listbox: &ListBox,
        preview: &DrawingArea,
        on_change: &Rc<RefCell<Option<std::boxed::Box<dyn Fn()>>>>,
    ) -> ListBoxRow {
        let row = ListBoxRow::new();
        let hbox = GtkBox::new(Orientation::Horizontal, 12);
        hbox.set_margin_start(12);
        hbox.set_margin_end(12);
        hbox.set_margin_top(6);
        hbox.set_margin_bottom(6);

        // Position spinner
        let position_box = GtkBox::new(Orientation::Horizontal, 6);
        let position_label = Label::new(Some("Position:"));
        position_label.set_halign(gtk4::Align::Start);

        let position_spin = SpinButton::with_range(0.0, 100.0, 1.0);
        position_spin.set_value(stop.position * 100.0); // Convert to percentage
        position_spin.set_digits(0);
        position_spin.set_width_request(80);

        let percent_label = Label::new(Some("%"));

        position_box.append(&position_label);
        position_box.append(&position_spin);
        position_box.append(&percent_label);
        hbox.append(&position_box);

        // Color button with swatch
        let color_button = Button::new();
        let swatch_box = GtkBox::new(Orientation::Horizontal, 6);

        let swatch = DrawingArea::new();
        swatch.set_size_request(32, 32);

        let color = stop.color;
        swatch.set_draw_func(move |_, cr, width, height| {
            color.apply_to_cairo(cr);
            let _ = cr.rectangle(0.0, 0.0, width as f64, height as f64);
            let _ = cr.fill();

            // Border
            cr.set_source_rgb(0.5, 0.5, 0.5);
            cr.set_line_width(1.0);
            let _ = cr.rectangle(0.5, 0.5, width as f64 - 1.0, height as f64 - 1.0);
            let _ = cr.stroke();
        });

        swatch_box.append(&swatch);
        swatch_box.append(&Label::new(Some("Color")));
        color_button.set_child(Some(&swatch_box));

        hbox.append(&color_button);

        // Remove button (only if more than 2 stops)
        if stop_count > 2 {
            let remove_button = Button::from_icon_name("user-trash-symbolic");
            remove_button.set_tooltip_text(Some("Remove stop"));

            let stops_clone = stops.clone();
            let listbox_clone = listbox.clone();
            let preview_clone = preview.clone();
            let on_change_clone = on_change.clone();

            remove_button.connect_clicked(move |_| {
                let mut stops = stops_clone.borrow_mut();
                if stops.len() > 2 {
                    stops.remove(index);
                    drop(stops);

                    Self::rebuild_stops_list(
                        &listbox_clone,
                        &stops_clone,
                        &preview_clone,
                        &on_change_clone,
                    );

                    preview_clone.queue_draw();

                    if let Some(callback) = on_change_clone.borrow().as_ref() {
                        callback();
                    }
                }
            });

            hbox.append(&remove_button);
        }

        row.set_child(Some(&hbox));

        // Position change handler
        let stops_clone = stops.clone();
        let listbox_clone = listbox.clone();
        let preview_clone = preview.clone();
        let on_change_clone = on_change.clone();

        position_spin.connect_value_changed(move |spin| {
            let mut new_position = spin.value() / 100.0; // Convert from percentage to 0.0-1.0

            // Validate: ensure minimum spacing of 0.01 (1%) between adjacent stops
            const MIN_SPACING: f64 = 0.01;

            let needs_rebuild;
            {
                let stops = stops_clone.borrow();
                // Check if this position would be too close to another stop
                for (i, other_stop) in stops.iter().enumerate() {
                    if i != index {
                        let distance = (new_position - other_stop.position).abs();
                        if distance < MIN_SPACING && distance > 0.0 {
                            // Adjust position to maintain minimum spacing
                            if new_position < other_stop.position {
                                new_position = (other_stop.position - MIN_SPACING).max(0.0);
                            } else {
                                new_position = (other_stop.position + MIN_SPACING).min(1.0);
                            }
                        }
                    }
                }

                // Check if order would change (needs rebuild)
                let old_index = index;
                let would_be_index = stops.iter()
                    .enumerate()
                    .filter(|(i, _)| *i != index)
                    .filter(|(_, s)| s.position < new_position)
                    .count();
                needs_rebuild = would_be_index != old_index.min(stops.len().saturating_sub(1));
            }

            {
                let mut stops = stops_clone.borrow_mut();
                if let Some(stop) = stops.get_mut(index) {
                    stop.position = new_position;
                }
                stops.sort_by(|a, b| a.position.partial_cmp(&b.position).unwrap());
            }

            // Only rebuild the list if the order changed - defer to idle to avoid
            // GTK adjustment issues when the SpinButton is still being interacted with
            if needs_rebuild {
                let listbox_clone2 = listbox_clone.clone();
                let stops_clone2 = stops_clone.clone();
                let preview_clone2 = preview_clone.clone();
                let on_change_clone2 = on_change_clone.clone();
                gtk4::glib::idle_add_local_once(move || {
                    Self::rebuild_stops_list(
                        &listbox_clone2,
                        &stops_clone2,
                        &preview_clone2,
                        &on_change_clone2,
                    );
                });
            }

            preview_clone.queue_draw();

            if let Some(callback) = on_change_clone.borrow().as_ref() {
                callback();
            }
        });

        // Color button handler
        let stops_clone = stops.clone();
        let listbox_clone = listbox.clone();
        let preview_clone = preview.clone();
        let on_change_clone = on_change.clone();
        let current_color = stop.color;

        color_button.connect_clicked(move |btn| {
            let window = btn.root().and_then(|root| root.downcast::<gtk4::Window>().ok());
            let stops_clone2 = stops_clone.clone();
            let listbox_clone2 = listbox_clone.clone();
            let preview_clone2 = preview_clone.clone();
            let on_change_clone2 = on_change_clone.clone();

            gtk4::glib::MainContext::default().spawn_local(async move {
                if let Some(new_color) = ColorPickerDialog::pick_color(window.as_ref(), current_color).await {
                    let mut stops = stops_clone2.borrow_mut();
                    if let Some(stop) = stops.get_mut(index) {
                        stop.color = new_color;
                    }
                    drop(stops);

                    Self::rebuild_stops_list(
                        &listbox_clone2,
                        &stops_clone2,
                        &preview_clone2,
                        &on_change_clone2,
                    );

                    preview_clone2.queue_draw();

                    if let Some(callback) = on_change_clone2.borrow().as_ref() {
                        callback();
                    }
                }
            });
        });

        row
    }

    /// Set the gradient configuration
    pub fn set_gradient(&self, config: &LinearGradientConfig) {
        *self.stops.borrow_mut() = config.stops.clone();
        *self.angle.borrow_mut() = config.angle;

        // Update the angle UI widgets
        self.angle_scale.set_value(config.angle);
        self.angle_spin.set_value(config.angle);

        Self::rebuild_stops_list(
            &self.stops_listbox,
            &self.stops,
            &self.preview,
            &self.on_change,
        );
        self.preview.queue_draw();
    }

    /// Get the current gradient configuration
    pub fn get_gradient(&self) -> LinearGradientConfig {
        LinearGradientConfig {
            angle: *self.angle.borrow(),
            stops: self.stops.borrow().clone(),
        }
    }

    /// Set callback for when gradient changes
    pub fn set_on_change<F: Fn() + 'static>(&self, callback: F) {
        *self.on_change.borrow_mut() = Some(std::boxed::Box::new(callback));
    }

    /// Get the container widget
    pub fn widget(&self) -> &GtkBox {
        &self.container
    }

    /// Update preview
    pub fn update_preview(&self) {
        self.preview.queue_draw();
    }
}

impl Default for GradientEditor {
    fn default() -> Self {
        Self::new()
    }
}
