//! Gradient editor widget for configuring linear and radial gradients

use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, DrawingArea, Grid, Label, Orientation, Scale, SpinButton};
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
}

impl GradientEditor {
    pub fn new() -> Self {
        let container = GtkBox::new(Orientation::Vertical, 12);
        container.set_margin_start(12);
        container.set_margin_end(12);
        container.set_margin_top(12);
        container.set_margin_bottom(12);

        let stops = Rc::new(RefCell::new(Vec::new()));
        let angle = Rc::new(RefCell::new(90.0));
        let on_change = Rc::new(RefCell::new(None));

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
        angle_scale.connect_value_changed(move |scale| {
            let value = scale.value();
            angle_spin_clone.set_value(value);
            *angle_clone.borrow_mut() = value;
            if let Some(callback) = on_change_clone.borrow().as_ref() {
                callback();
            }
        });

        let angle_scale_clone = angle_scale.clone();
        let angle_clone2 = angle.clone();
        let on_change_clone2 = on_change.clone();
        angle_spin.connect_value_changed(move |spin| {
            let value = spin.value();
            angle_scale_clone.set_value(value);
            *angle_clone2.borrow_mut() = value;
            if let Some(callback) = on_change_clone2.borrow().as_ref() {
                callback();
            }
        });

        angle_box.append(&angle_scale);
        angle_box.append(&angle_spin);
        container.append(&angle_box);

        // Preview area
        let preview = DrawingArea::new();
        preview.set_content_height(100);
        preview.set_vexpand(false);

        let stops_clone = stops.clone();
        let angle_clone = angle.clone();
        preview.set_draw_func(move |_, cr, width, height| {
            use crate::ui::background::render_background;
            use crate::ui::background::{BackgroundConfig, BackgroundType, LinearGradientConfig};

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

        container.append(&preview);

        // Color stops list
        let stops_label = Label::new(Some("Color Stops:"));
        stops_label.set_halign(gtk4::Align::Start);
        container.append(&stops_label);

        let stops_container = GtkBox::new(Orientation::Vertical, 6);
        container.append(&stops_container);

        // Add stop button
        let add_button = Button::with_label("Add Color Stop");
        let stops_clone = stops.clone();
        let stops_container_clone = stops_container.clone();
        let preview_clone = preview.clone();
        let on_change_clone = on_change.clone();

        add_button.connect_clicked(move |_| {
            let mut stops_list = stops_clone.borrow_mut();

            // Find a good position for the new stop (middle of largest gap)
            let position = if stops_list.is_empty() {
                0.5
            } else if stops_list.len() == 1 {
                if stops_list[0].position < 0.5 {
                    1.0
                } else {
                    0.0
                }
            } else {
                // Find largest gap
                let mut sorted_stops: Vec<_> = stops_list.iter().map(|s| s.position).collect();
                sorted_stops.sort_by(|a, b| a.partial_cmp(b).unwrap());

                let mut max_gap = 0.0;
                let mut best_pos = 0.5;

                for i in 0..sorted_stops.len() - 1 {
                    let gap = sorted_stops[i + 1] - sorted_stops[i];
                    if gap > max_gap {
                        max_gap = gap;
                        best_pos = (sorted_stops[i] + sorted_stops[i + 1]) / 2.0;
                    }
                }

                best_pos
            };

            let new_stop = ColorStop::new(position, Color::new(0.5, 0.5, 0.5, 1.0));
            stops_list.push(new_stop);

            drop(stops_list);

            preview_clone.queue_draw();

            if let Some(callback) = on_change_clone.borrow().as_ref() {
                callback();
            }

            // Refresh stops list UI would go here
            // For now, we'll need to rebuild the stops container
        });

        container.append(&add_button);

        let editor = Self {
            container,
            stops,
            angle,
            on_change,
            preview,
        };

        editor
    }

    /// Set the gradient configuration
    pub fn set_gradient(&self, config: &LinearGradientConfig) {
        *self.stops.borrow_mut() = config.stops.clone();
        *self.angle.borrow_mut() = config.angle;
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
