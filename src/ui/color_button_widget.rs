//! Reusable color button widget with color swatch, copy and paste buttons.
//!
//! Provides a consistent appearance for color selection across all config dialogs.

use crate::ui::background::Color;
use crate::ui::clipboard::CLIPBOARD;
use crate::ui::color_picker::ColorPickerDialog;
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, DrawingArea, Orientation};
use std::cell::RefCell;
use std::rc::Rc;

/// A reusable color button widget that provides:
/// - A color swatch button (40x40) with checkerboard for transparency
/// - Copy button with stock icon
/// - Paste button with stock icon
pub struct ColorButtonWidget {
    container: GtkBox,
    color_button: Button,
    drawing_area: DrawingArea,
    #[allow(dead_code)]
    copy_button: Button,
    #[allow(dead_code)]
    paste_button: Button,
    color: Rc<RefCell<Color>>,
    on_change: Rc<RefCell<Option<Box<dyn Fn(Color)>>>>,
}

impl ColorButtonWidget {
    /// Create a new ColorButtonWidget with the given initial color.
    pub fn new(initial_color: Color) -> Self {
        let container = GtkBox::new(Orientation::Horizontal, 4);
        let color = Rc::new(RefCell::new(initial_color));
        let on_change: Rc<RefCell<Option<Box<dyn Fn(Color)>>>> = Rc::new(RefCell::new(None));

        // Create the color swatch button
        let color_button = Button::new();
        color_button.set_tooltip_text(Some("Click to change color"));

        // Create the drawing area for the color swatch
        let drawing_area = DrawingArea::new();
        drawing_area.set_size_request(40, 40);

        // Set up the draw function
        let color_for_draw = color.clone();
        drawing_area.set_draw_func(move |_, cr, width, height| {
            let current_color = *color_for_draw.borrow();
            draw_color_swatch(cr, width, height, current_color);
        });

        color_button.set_child(Some(&drawing_area));

        // Create copy button with icon
        let copy_button = Button::from_icon_name("edit-copy-symbolic");
        copy_button.set_tooltip_text(Some("Copy color"));

        // Create paste button with icon
        let paste_button = Button::from_icon_name("edit-paste-symbolic");
        paste_button.set_tooltip_text(Some("Paste color"));

        // Add widgets to container
        container.append(&color_button);
        container.append(&copy_button);
        container.append(&paste_button);

        // Set up color button click handler
        let color_clone = color.clone();
        let on_change_clone = on_change.clone();
        let drawing_area_clone = drawing_area.clone();
        color_button.connect_clicked(move |btn| {
            let current_color = *color_clone.borrow();
            let window = btn
                .root()
                .and_then(|root| root.downcast::<gtk4::Window>().ok());

            let color_clone2 = color_clone.clone();
            let on_change_clone2 = on_change_clone.clone();
            let drawing_area_clone2 = drawing_area_clone.clone();

            gtk4::glib::MainContext::default().spawn_local(async move {
                if let Some(new_color) =
                    ColorPickerDialog::pick_color(window.as_ref(), current_color).await
                {
                    *color_clone2.borrow_mut() = new_color;
                    drawing_area_clone2.queue_draw();

                    // Notify callback
                    if let Some(ref callback) = *on_change_clone2.borrow() {
                        callback(new_color);
                    }
                }
            });
        });

        // Set up copy button handler
        let color_for_copy = color.clone();
        copy_button.connect_clicked(move |_| {
            let c = *color_for_copy.borrow();
            if let Ok(mut clipboard) = CLIPBOARD.lock() {
                clipboard.copy_color(c.r, c.g, c.b, c.a);
                log::info!("Color copied to clipboard");
            }
        });

        // Set up paste button handler
        let color_for_paste = color.clone();
        let on_change_for_paste = on_change.clone();
        let drawing_area_for_paste = drawing_area.clone();
        paste_button.connect_clicked(move |_| {
            if let Ok(clipboard) = CLIPBOARD.lock() {
                if let Some((r, g, b, a)) = clipboard.paste_color() {
                    let new_color = Color::new(r, g, b, a);
                    *color_for_paste.borrow_mut() = new_color;
                    drawing_area_for_paste.queue_draw();
                    log::info!("Color pasted from clipboard");

                    // Notify callback
                    if let Some(ref callback) = *on_change_for_paste.borrow() {
                        callback(new_color);
                    }
                }
            }
        });

        Self {
            container,
            color_button,
            drawing_area,
            copy_button,
            paste_button,
            color,
            on_change,
        }
    }

    /// Get the container widget (for adding to layouts).
    pub fn widget(&self) -> &GtkBox {
        &self.container
    }

    /// Get the current color.
    pub fn color(&self) -> Color {
        *self.color.borrow()
    }

    /// Set the color (updates the swatch display).
    pub fn set_color(&self, color: Color) {
        *self.color.borrow_mut() = color;
        self.drawing_area.queue_draw();
    }

    /// Set a callback to be called when the color changes.
    pub fn set_on_change<F: Fn(Color) + 'static>(&self, callback: F) {
        *self.on_change.borrow_mut() = Some(Box::new(callback));
    }

    /// Get the main color button (for keyboard focus, etc.).
    #[allow(dead_code)]
    pub fn color_button(&self) -> &Button {
        &self.color_button
    }
}

/// Draw a color swatch with checkerboard background for transparency.
fn draw_color_swatch(cr: &gtk4::cairo::Context, width: i32, height: i32, color: Color) {
    // Draw checkerboard pattern for transparency visualization
    let checker_size = 8.0;
    for y in 0..(height as f64 / checker_size).ceil() as i32 {
        for x in 0..(width as f64 / checker_size).ceil() as i32 {
            if (x + y) % 2 == 0 {
                cr.set_source_rgb(0.8, 0.8, 0.8);
            } else {
                cr.set_source_rgb(0.6, 0.6, 0.6);
            }
            cr.rectangle(
                x as f64 * checker_size,
                y as f64 * checker_size,
                checker_size,
                checker_size,
            );
            let _ = cr.fill();
        }
    }

    // Draw the color with alpha on top
    cr.set_source_rgba(color.r, color.g, color.b, color.a);
    cr.rectangle(0.0, 0.0, width as f64, height as f64);
    let _ = cr.fill();
}
