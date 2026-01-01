//! Theme color selector widget
//!
//! Provides a row of theme color buttons (showing actual colors) plus a custom color picker.
//! Used throughout combo panel config dialogs to select either a theme color or custom color.

use crate::ui::background::Color;
use crate::ui::clipboard::CLIPBOARD;
use crate::ui::color_picker::ColorPickerDialog;
use crate::ui::theme::{ColorSource, ComboThemeConfig};
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, DrawingArea, Orientation};
use std::cell::RefCell;
use std::rc::Rc;

/// A widget for selecting either a theme color (1-4) or a custom color.
///
/// Layout: [Color1][Color2][Color3][Color4] | [Custom Color Swatch][Copy][Paste]
///
/// Theme buttons show the actual theme colors. Clicking a selected theme button
/// switches to custom mode. The custom color swatch shows the current custom color.
pub struct ThemeColorSelector {
    container: GtkBox,
    #[allow(dead_code)]
    theme_buttons: [Button; 4],
    theme_drawings: [DrawingArea; 4],
    color_button: Button,
    color_drawing_area: DrawingArea,
    #[allow(dead_code)]
    copy_button: Button,
    #[allow(dead_code)]
    paste_button: Button,
    source: Rc<RefCell<ColorSource>>,
    custom_color: Rc<RefCell<Color>>,
    theme_config: Rc<RefCell<ComboThemeConfig>>,
    on_change: Rc<RefCell<Option<Box<dyn Fn(ColorSource)>>>>,
    selected_index: Rc<RefCell<Option<u8>>>, // None = custom, Some(1-4) = theme
}

impl ThemeColorSelector {
    /// Create a new ThemeColorSelector with the given initial source.
    pub fn new(initial_source: ColorSource) -> Self {
        let container = GtkBox::new(Orientation::Horizontal, 2);
        let source = Rc::new(RefCell::new(initial_source.clone()));
        let on_change: Rc<RefCell<Option<Box<dyn Fn(ColorSource)>>>> = Rc::new(RefCell::new(None));
        let theme_config = Rc::new(RefCell::new(ComboThemeConfig::default()));

        // Extract custom color from source, or default
        let initial_custom = match &initial_source {
            ColorSource::Custom { color } => *color,
            ColorSource::Theme { .. } => Color::new(0.5, 0.5, 0.5, 1.0),
        };
        let custom_color = Rc::new(RefCell::new(initial_custom));

        // Track which theme button is selected (None = custom mode)
        let selected_index: Rc<RefCell<Option<u8>>> = Rc::new(RefCell::new(
            match &initial_source {
                ColorSource::Theme { index } => Some(*index),
                ColorSource::Custom { .. } => None,
            }
        ));

        // Create theme buttons with drawing areas inside
        let theme_buttons: [Button; 4] = [
            Button::new(),
            Button::new(),
            Button::new(),
            Button::new(),
        ];

        let theme_drawings: [DrawingArea; 4] = [
            DrawingArea::new(),
            DrawingArea::new(),
            DrawingArea::new(),
            DrawingArea::new(),
        ];

        // Set up each theme button
        for i in 0..4 {
            let drawing = &theme_drawings[i];
            let button = &theme_buttons[i];

            drawing.set_size_request(24, 20);
            button.set_child(Some(drawing));

            // Set tooltips
            let tooltip = match i {
                0 => "Theme Color 1 (Primary) - Click again to use custom",
                1 => "Theme Color 2 (Secondary) - Click again to use custom",
                2 => "Theme Color 3 (Accent) - Click again to use custom",
                3 => "Theme Color 4 (Highlight) - Click again to use custom",
                _ => "",
            };
            button.set_tooltip_text(Some(tooltip));

            // Set up draw function for theme color
            let theme_config_clone = theme_config.clone();
            let selected_clone = selected_index.clone();
            let idx = (i + 1) as u8;
            drawing.set_draw_func(move |_, cr, width, height| {
                let cfg = theme_config_clone.borrow();
                let color = cfg.get_color(idx);
                let is_selected = *selected_clone.borrow() == Some(idx);
                draw_theme_button(cr, width, height, color, is_selected);
            });

            container.append(button);
        }

        // Connect theme button click handlers
        for (i, theme_button) in theme_buttons.iter().enumerate() {
            let source_clone = source.clone();
            let on_change_clone = on_change.clone();
            let selected_clone = selected_index.clone();
            let custom_color_clone = custom_color.clone();
            let theme_drawings_clone: Vec<DrawingArea> = theme_drawings.to_vec();
            let color_drawing_ref = Rc::new(RefCell::new(None::<DrawingArea>));
            let color_drawing_ref_clone = color_drawing_ref.clone();
            let idx = (i + 1) as u8;

            theme_button.connect_clicked(move |_| {
                let currently_selected = *selected_clone.borrow();

                if currently_selected == Some(idx) {
                    // Already selected - switch to custom mode
                    *selected_clone.borrow_mut() = None;
                    let custom = *custom_color_clone.borrow();
                    let new_source = ColorSource::Custom { color: custom };
                    *source_clone.borrow_mut() = new_source.clone();

                    if let Some(ref callback) = *on_change_clone.borrow() {
                        callback(new_source);
                    }
                } else {
                    // Select this theme color
                    *selected_clone.borrow_mut() = Some(idx);
                    let new_source = ColorSource::Theme { index: idx };
                    *source_clone.borrow_mut() = new_source.clone();

                    if let Some(ref callback) = *on_change_clone.borrow() {
                        callback(new_source);
                    }
                }

                // Redraw all theme buttons to update selection indicator
                for drawing in &theme_drawings_clone {
                    drawing.queue_draw();
                }
                // Redraw custom color swatch
                if let Some(ref da) = *color_drawing_ref_clone.borrow() {
                    da.queue_draw();
                }
            });
        }

        // Add separator
        let separator = gtk4::Separator::new(Orientation::Vertical);
        separator.set_margin_start(4);
        separator.set_margin_end(4);
        container.append(&separator);

        // Create the custom color swatch button
        let color_button = Button::new();
        color_button.set_tooltip_text(Some("Custom color (click to change)"));

        // Create the drawing area for the custom color swatch
        let color_drawing_area = DrawingArea::new();
        color_drawing_area.set_size_request(32, 20);

        // Set up the draw function for custom color
        let custom_color_for_draw = custom_color.clone();
        let selected_for_draw = selected_index.clone();
        color_drawing_area.set_draw_func(move |_, cr, width, height| {
            let color = *custom_color_for_draw.borrow();
            let is_custom_mode = selected_for_draw.borrow().is_none();
            draw_custom_swatch(cr, width, height, color, is_custom_mode);
        });

        color_button.set_child(Some(&color_drawing_area));

        // Now update the button references in the theme button handlers
        // We need to reconnect the handlers with the actual button references
        // This is a bit awkward but necessary since we created the handlers before the button

        // Create copy button with icon
        let copy_button = Button::from_icon_name("edit-copy-symbolic");
        copy_button.set_tooltip_text(Some("Copy color"));

        // Create paste button with icon
        let paste_button = Button::from_icon_name("edit-paste-symbolic");
        paste_button.set_tooltip_text(Some("Paste color"));

        // Add to container
        container.append(&color_button);
        container.append(&copy_button);
        container.append(&paste_button);

        // Connect custom color button click handler
        let custom_color_clone = custom_color.clone();
        let source_for_click = source.clone();
        let on_change_for_click = on_change.clone();
        let selected_for_click = selected_index.clone();
        let color_drawing_for_click = color_drawing_area.clone();
        let theme_drawings_for_click: Vec<DrawingArea> = theme_drawings.to_vec();

        color_button.connect_clicked(move |btn| {
            let current_custom = *custom_color_clone.borrow();
            let window = btn
                .root()
                .and_then(|root| root.downcast::<gtk4::Window>().ok());

            let custom_color_clone2 = custom_color_clone.clone();
            let source_clone2 = source_for_click.clone();
            let on_change_clone2 = on_change_for_click.clone();
            let selected_clone2 = selected_for_click.clone();
            let color_drawing_clone2 = color_drawing_for_click.clone();
            let theme_drawings_clone2 = theme_drawings_for_click.clone();

            gtk4::glib::MainContext::default().spawn_local(async move {
                if let Some(new_color) =
                    ColorPickerDialog::pick_color(window.as_ref(), current_custom).await
                {
                    *custom_color_clone2.borrow_mut() = new_color;
                    let new_source = ColorSource::Custom { color: new_color };
                    *source_clone2.borrow_mut() = new_source.clone();

                    // Switch to custom mode
                    *selected_clone2.borrow_mut() = None;

                    // Redraw all
                    color_drawing_clone2.queue_draw();
                    for da in &theme_drawings_clone2 {
                        da.queue_draw();
                    }

                    if let Some(ref callback) = *on_change_clone2.borrow() {
                        callback(new_source);
                    }
                }
            });
        });

        // Set up copy button handler
        let source_for_copy = source.clone();
        let theme_config_for_copy = theme_config.clone();
        copy_button.connect_clicked(move |_| {
            let current_source = source_for_copy.borrow().clone();
            let c = match &current_source {
                ColorSource::Custom { color } => *color,
                ColorSource::Theme { index } => {
                    theme_config_for_copy.borrow().get_color(*index)
                }
            };
            if let Ok(mut clipboard) = CLIPBOARD.lock() {
                clipboard.copy_color(c.r, c.g, c.b, c.a);
                log::info!("Color copied to clipboard");
            }
        });

        // Set up paste button handler
        let custom_color_for_paste = custom_color.clone();
        let source_for_paste = source.clone();
        let on_change_for_paste = on_change.clone();
        let selected_for_paste = selected_index.clone();
        let color_drawing_for_paste = color_drawing_area.clone();
        let theme_drawings_for_paste: Vec<DrawingArea> = theme_drawings.to_vec();

        paste_button.connect_clicked(move |_| {
            if let Ok(clipboard) = CLIPBOARD.lock() {
                if let Some((r, g, b, a)) = clipboard.paste_color() {
                    let new_color = Color::new(r, g, b, a);
                    *custom_color_for_paste.borrow_mut() = new_color;
                    let new_source = ColorSource::Custom { color: new_color };
                    *source_for_paste.borrow_mut() = new_source.clone();

                    // Switch to custom mode
                    *selected_for_paste.borrow_mut() = None;

                    // Redraw all
                    color_drawing_for_paste.queue_draw();
                    for da in &theme_drawings_for_paste {
                        da.queue_draw();
                    }
                    log::info!("Color pasted from clipboard");

                    if let Some(ref callback) = *on_change_for_paste.borrow() {
                        callback(new_source);
                    }
                }
            }
        });

        Self {
            container,
            theme_buttons,
            theme_drawings,
            color_button,
            color_drawing_area,
            copy_button,
            paste_button,
            source,
            custom_color,
            theme_config,
            on_change,
            selected_index,
        }
    }

    /// Get the container widget (for adding to layouts).
    pub fn widget(&self) -> &GtkBox {
        &self.container
    }

    /// Get the current color source.
    pub fn source(&self) -> ColorSource {
        self.source.borrow().clone()
    }

    /// Set the color source (updates the UI).
    pub fn set_source(&self, source: ColorSource) {
        *self.source.borrow_mut() = source.clone();

        match &source {
            ColorSource::Theme { index } => {
                *self.selected_index.borrow_mut() = Some(*index);
            }
            ColorSource::Custom { color } => {
                *self.selected_index.borrow_mut() = None;
                *self.custom_color.borrow_mut() = *color;
            }
        }

        // Redraw all
        self.color_drawing_area.queue_draw();
        for da in &self.theme_drawings {
            da.queue_draw();
        }
    }

    /// Set the theme config (used to resolve theme colors for display).
    pub fn set_theme_config(&self, config: ComboThemeConfig) {
        *self.theme_config.borrow_mut() = config;
        // Redraw theme buttons to show new colors
        for da in &self.theme_drawings {
            da.queue_draw();
        }
    }

    /// Set a callback to be called when the source changes.
    pub fn set_on_change<F: Fn(ColorSource) + 'static>(&self, callback: F) {
        *self.on_change.borrow_mut() = Some(Box::new(callback));
    }

    /// Resolve the current source to an actual color.
    pub fn resolve_color(&self) -> Color {
        let source = self.source.borrow().clone();
        match &source {
            ColorSource::Custom { color } => *color,
            ColorSource::Theme { index } => {
                self.theme_config.borrow().get_color(*index)
            }
        }
    }
}

/// Draw a theme color button with selection indicator.
fn draw_theme_button(cr: &gtk4::cairo::Context, width: i32, height: i32, color: Color, is_selected: bool) {
    let w = width as f64;
    let h = height as f64;

    // Draw checkerboard for transparency
    let checker_size = 4.0;
    for y in 0..(h / checker_size).ceil() as i32 {
        for x in 0..(w / checker_size).ceil() as i32 {
            if (x + y) % 2 == 0 {
                cr.set_source_rgb(0.7, 0.7, 0.7);
            } else {
                cr.set_source_rgb(0.5, 0.5, 0.5);
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

    // Draw the color
    cr.set_source_rgba(color.r, color.g, color.b, color.a);
    cr.rectangle(0.0, 0.0, w, h);
    let _ = cr.fill();

    // Draw selection indicator (border)
    if is_selected {
        cr.set_source_rgb(1.0, 1.0, 1.0);
        cr.set_line_width(2.0);
        cr.rectangle(1.0, 1.0, w - 2.0, h - 2.0);
        let _ = cr.stroke();

        cr.set_source_rgb(0.0, 0.0, 0.0);
        cr.set_line_width(1.0);
        cr.rectangle(2.5, 2.5, w - 5.0, h - 5.0);
        let _ = cr.stroke();
    }
}

/// Draw the custom color swatch.
fn draw_custom_swatch(cr: &gtk4::cairo::Context, width: i32, height: i32, color: Color, is_custom_mode: bool) {
    let w = width as f64;
    let h = height as f64;

    // Draw checkerboard for transparency
    let checker_size = 4.0;
    for y in 0..(h / checker_size).ceil() as i32 {
        for x in 0..(w / checker_size).ceil() as i32 {
            if (x + y) % 2 == 0 {
                cr.set_source_rgb(0.7, 0.7, 0.7);
            } else {
                cr.set_source_rgb(0.5, 0.5, 0.5);
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

    // Draw the color
    cr.set_source_rgba(color.r, color.g, color.b, color.a);
    cr.rectangle(0.0, 0.0, w, h);
    let _ = cr.fill();

    // Draw selection indicator if in custom mode
    if is_custom_mode {
        cr.set_source_rgb(1.0, 1.0, 1.0);
        cr.set_line_width(2.0);
        cr.rectangle(1.0, 1.0, w - 2.0, h - 2.0);
        let _ = cr.stroke();

        cr.set_source_rgb(0.0, 0.0, 0.0);
        cr.set_line_width(1.0);
        cr.rectangle(2.5, 2.5, w - 5.0, h - 5.0);
        let _ = cr.stroke();
    }
}
