//! Color picker dialog with alpha channel support

use gtk4::prelude::*;
use gtk4::{ColorDialog, Window};
use crate::ui::background::Color;

/// Color picker dialog
pub struct ColorPickerDialog;

impl ColorPickerDialog {
    /// Show color picker and return selected color
    pub async fn pick_color(parent: Option<&Window>, initial_color: Color) -> Option<Color> {
        let dialog = ColorDialog::builder()
            .title("Select Color")
            .modal(true)
            .with_alpha(true)
            .build();

        // Set initial color
        let initial_rgba = initial_color.to_gdk_rgba();

        match dialog.choose_rgba_future(parent, Some(&initial_rgba)).await {
            Ok(rgba) => Some(Color::from_gdk_rgba(&rgba)),
            Err(_) => None,
        }
    }

    /// Show color picker synchronously (blocking)
    /// Note: This should be used carefully as it blocks the UI thread
    pub fn pick_color_sync(parent: Option<&Window>, initial_color: Color) -> Option<Color> {
        use gtk4::glib;

        let main_context = glib::MainContext::default();
        main_context.block_on(Self::pick_color(parent, initial_color))
    }
}
