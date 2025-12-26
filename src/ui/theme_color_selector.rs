//! Theme color selector widget
//!
//! Provides a row of theme color toggle buttons (T1-T4) plus a custom color picker.
//! Used throughout combo panel config dialogs to select either a theme color or custom color.

use crate::ui::background::Color;
use crate::ui::clipboard::CLIPBOARD;
use crate::ui::color_picker::ColorPickerDialog;
use crate::ui::theme::{ColorSource, ComboThemeConfig};
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, DrawingArea, Orientation, ToggleButton};
use std::cell::RefCell;
use std::rc::Rc;

/// A widget for selecting either a theme color (T1-T4) or a custom color.
///
/// Layout: [T1][T2][T3][T4] [Color Swatch][Copy][Paste]
///
/// When a theme button is active, the custom color picker is dimmed.
/// When "Custom" mode is active (no theme button selected), the color picker is enabled.
pub struct ThemeColorSelector {
    container: GtkBox,
    theme_buttons: [ToggleButton; 4],
    color_button: Button,
    color_drawing_area: DrawingArea,
    #[allow(dead_code)]
    copy_button: Button,
    #[allow(dead_code)]
    paste_button: Button,
    source: Rc<RefCell<ColorSource>>,
    custom_color: Rc<RefCell<Color>>,
    theme_config: Rc<RefCell<Option<ComboThemeConfig>>>,
    on_change: Rc<RefCell<Option<Box<dyn Fn(ColorSource)>>>>,
}

impl ThemeColorSelector {
    /// Create a new ThemeColorSelector with the given initial source.
    pub fn new(initial_source: ColorSource) -> Self {
        let container = GtkBox::new(Orientation::Horizontal, 4);
        let source = Rc::new(RefCell::new(initial_source.clone()));
        let on_change: Rc<RefCell<Option<Box<dyn Fn(ColorSource)>>>> = Rc::new(RefCell::new(None));
        let theme_config: Rc<RefCell<Option<ComboThemeConfig>>> = Rc::new(RefCell::new(None));

        // Extract custom color from source, or default
        let initial_custom = match &initial_source {
            ColorSource::Custom { color } => *color,
            ColorSource::Theme { .. } => Color::new(0.5, 0.5, 0.5, 1.0),
        };
        let custom_color = Rc::new(RefCell::new(initial_custom));

        // Create theme toggle buttons
        let theme_buttons: [ToggleButton; 4] = [
            ToggleButton::with_label("T1"),
            ToggleButton::with_label("T2"),
            ToggleButton::with_label("T3"),
            ToggleButton::with_label("T4"),
        ];

        // Set tooltips
        theme_buttons[0].set_tooltip_text(Some("Theme Color 1 (Primary)"));
        theme_buttons[1].set_tooltip_text(Some("Theme Color 2 (Secondary)"));
        theme_buttons[2].set_tooltip_text(Some("Theme Color 3 (Accent)"));
        theme_buttons[3].set_tooltip_text(Some("Theme Color 4 (Highlight)"));

        // Group the toggle buttons so only one can be active at a time
        // (or none, for custom mode)
        for i in 1..4 {
            theme_buttons[i].set_group(Some(&theme_buttons[0]));
        }

        // Set initial active state based on source
        if let ColorSource::Theme { index } = &initial_source {
            let idx = (*index as usize).saturating_sub(1).min(3);
            theme_buttons[idx].set_active(true);
        }

        // Add theme buttons to container
        for btn in &theme_buttons {
            container.append(btn);
        }

        // Add separator
        let separator = gtk4::Separator::new(Orientation::Vertical);
        separator.set_margin_start(4);
        separator.set_margin_end(4);
        container.append(&separator);

        // Create the color swatch button
        let color_button = Button::new();
        color_button.set_tooltip_text(Some("Custom color (click to change)"));

        // Create the drawing area for the color swatch
        let color_drawing_area = DrawingArea::new();
        color_drawing_area.set_size_request(32, 24);

        // Set up the draw function
        let custom_color_for_draw = custom_color.clone();
        let source_for_draw = source.clone();
        let theme_config_for_draw = theme_config.clone();
        color_drawing_area.set_draw_func(move |_, cr, width, height| {
            let current_source = source_for_draw.borrow().clone();
            let display_color = match &current_source {
                ColorSource::Custom { color } => *color,
                ColorSource::Theme { index } => {
                    if let Some(ref cfg) = *theme_config_for_draw.borrow() {
                        cfg.get_color(*index)
                    } else {
                        *custom_color_for_draw.borrow()
                    }
                }
            };
            draw_color_swatch(cr, width, height, display_color);
        });

        color_button.set_child(Some(&color_drawing_area));

        // Dim color button if theme is selected
        if initial_source.is_theme() {
            color_button.set_sensitive(false);
        }

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

        // Connect theme button toggled handlers
        for (i, btn) in theme_buttons.iter().enumerate() {
            let source_clone = source.clone();
            let on_change_clone = on_change.clone();
            let color_button_clone = color_button.clone();
            let drawing_area_clone = color_drawing_area.clone();

            btn.connect_toggled(move |toggle_btn| {
                if toggle_btn.is_active() {
                    let new_source = ColorSource::Theme { index: (i + 1) as u8 };
                    *source_clone.borrow_mut() = new_source.clone();
                    color_button_clone.set_sensitive(false);
                    drawing_area_clone.queue_draw();

                    if let Some(ref callback) = *on_change_clone.borrow() {
                        callback(new_source);
                    }
                }
            });
        }

        // Connect color button click handler (for custom color)
        let custom_color_clone = custom_color.clone();
        let source_for_click = source.clone();
        let on_change_for_click = on_change.clone();
        let drawing_area_for_click = color_drawing_area.clone();
        let theme_buttons_clone: Vec<ToggleButton> = theme_buttons.iter().cloned().collect();
        let color_button_for_click = color_button.clone();

        color_button.connect_clicked(move |btn| {
            let current_custom = *custom_color_clone.borrow();
            let window = btn
                .root()
                .and_then(|root| root.downcast::<gtk4::Window>().ok());

            let custom_color_clone2 = custom_color_clone.clone();
            let source_clone2 = source_for_click.clone();
            let on_change_clone2 = on_change_for_click.clone();
            let drawing_area_clone2 = drawing_area_for_click.clone();
            let theme_buttons_clone2 = theme_buttons_clone.clone();
            let color_button_clone2 = color_button_for_click.clone();

            gtk4::glib::MainContext::default().spawn_local(async move {
                if let Some(new_color) =
                    ColorPickerDialog::pick_color(window.as_ref(), current_custom).await
                {
                    *custom_color_clone2.borrow_mut() = new_color;
                    let new_source = ColorSource::Custom { color: new_color };
                    *source_clone2.borrow_mut() = new_source.clone();

                    // Deselect all theme buttons
                    for btn in &theme_buttons_clone2 {
                        btn.set_active(false);
                    }
                    color_button_clone2.set_sensitive(true);
                    drawing_area_clone2.queue_draw();

                    if let Some(ref callback) = *on_change_clone2.borrow() {
                        callback(new_source);
                    }
                }
            });
        });

        // Set up copy button handler
        let source_for_copy = source.clone();
        let custom_color_for_copy = custom_color.clone();
        let theme_config_for_copy = theme_config.clone();
        copy_button.connect_clicked(move |_| {
            let current_source = source_for_copy.borrow().clone();
            let c = match &current_source {
                ColorSource::Custom { color } => *color,
                ColorSource::Theme { index } => {
                    if let Some(ref cfg) = *theme_config_for_copy.borrow() {
                        cfg.get_color(*index)
                    } else {
                        *custom_color_for_copy.borrow()
                    }
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
        let drawing_area_for_paste = color_drawing_area.clone();
        let theme_buttons_for_paste: Vec<ToggleButton> = theme_buttons.iter().cloned().collect();
        let color_button_for_paste = color_button.clone();

        paste_button.connect_clicked(move |_| {
            if let Ok(clipboard) = CLIPBOARD.lock() {
                if let Some((r, g, b, a)) = clipboard.paste_color() {
                    let new_color = Color::new(r, g, b, a);
                    *custom_color_for_paste.borrow_mut() = new_color;
                    let new_source = ColorSource::Custom { color: new_color };
                    *source_for_paste.borrow_mut() = new_source.clone();

                    // Deselect all theme buttons and enable custom picker
                    for btn in &theme_buttons_for_paste {
                        btn.set_active(false);
                    }
                    color_button_for_paste.set_sensitive(true);
                    drawing_area_for_paste.queue_draw();
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
            color_button,
            color_drawing_area,
            copy_button,
            paste_button,
            source,
            custom_color,
            theme_config,
            on_change,
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
                let idx = (*index as usize).saturating_sub(1).min(3);
                self.theme_buttons[idx].set_active(true);
                self.color_button.set_sensitive(false);
            }
            ColorSource::Custom { color } => {
                // Deselect all theme buttons
                for btn in &self.theme_buttons {
                    btn.set_active(false);
                }
                *self.custom_color.borrow_mut() = *color;
                self.color_button.set_sensitive(true);
            }
        }
        self.color_drawing_area.queue_draw();
    }

    /// Set the theme config (used to resolve theme colors for display).
    pub fn set_theme_config(&self, config: ComboThemeConfig) {
        *self.theme_config.borrow_mut() = Some(config);
        self.color_drawing_area.queue_draw();
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
                if let Some(ref cfg) = *self.theme_config.borrow() {
                    cfg.get_color(*index)
                } else {
                    *self.custom_color.borrow()
                }
            }
        }
    }
}

/// Draw a color swatch with checkerboard background for transparency.
fn draw_color_swatch(cr: &gtk4::cairo::Context, width: i32, height: i32, color: Color) {
    // Draw checkerboard pattern for transparency visualization
    let checker_size = 6.0;
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
