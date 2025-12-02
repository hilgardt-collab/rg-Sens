//! Text displayer implementation

use crate::core::{ConfigOption, ConfigSchema, Displayer};
use anyhow::Result;
use cairo::Context;
use gtk4::{prelude::*, DrawingArea, Widget};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Text displayer
///
/// Displays data values as text using Cairo and Pango.
pub struct TextDisplayer {
    id: String,
    name: String,
    data: Arc<Mutex<DisplayData>>,
}

#[derive(Clone)]
struct DisplayData {
    text: String,
    font_size: f64,
    color: (f64, f64, f64), // RGB
}

impl TextDisplayer {
    pub fn new() -> Self {
        let data = Arc::new(Mutex::new(DisplayData {
            text: "No data".to_string(),
            font_size: 24.0,
            color: (1.0, 1.0, 1.0), // White
        }));

        Self {
            id: "text".to_string(),
            name: "Text Display".to_string(),
            data,
        }
    }

    fn draw_internal(cr: &Context, width: i32, height: i32, data: &DisplayData) {
        // Clear background
        cr.set_source_rgb(0.0, 0.0, 0.0);
        cr.paint().ok();

        // Set text color
        cr.set_source_rgb(data.color.0, data.color.1, data.color.2);

        // Set font
        cr.select_font_face("Sans", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
        cr.set_font_size(data.font_size);

        // Get text dimensions and center text
        if let Ok(extents) = cr.text_extents(&data.text) {
            let x = (width as f64 - extents.width()) / 2.0 - extents.x_bearing();
            let y = (height as f64 - extents.height()) / 2.0 - extents.y_bearing();
            cr.move_to(x, y);
        } else {
            // Fallback: place text at center without measuring
            cr.move_to(width as f64 / 2.0, height as f64 / 2.0);
        }

        cr.show_text(&data.text).ok();
    }
}

impl Default for TextDisplayer {
    fn default() -> Self {
        Self::new()
    }
}

impl Displayer for TextDisplayer {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn create_widget(&self) -> Widget {
        let drawing_area = DrawingArea::new();

        // Set minimum size
        drawing_area.set_size_request(200, 100);

        // Set up draw function
        let data_clone = self.data.clone();
        drawing_area.set_draw_func(move |widget, cr, width, height| {
            if let Ok(data) = data_clone.lock() {
                Self::draw_internal(cr, width, height, &data);
            }
            // Schedule another redraw to keep updating
            widget.queue_draw();
        });

        drawing_area.upcast()
    }

    fn update_data(&mut self, data: &HashMap<String, Value>) {
        // Format the data as text
        let mut text = String::new();

        // Look for "usage" key (from CPU source)
        if let Some(usage) = data.get("usage") {
            if let Some(usage_val) = usage.as_f64() {
                text = format!("CPU: {:.1}%", usage_val);
            }
        }

        // If no specific key found, show all data
        if text.is_empty() {
            for (key, value) in data {
                text.push_str(&format!("{}: {:?}\n", key, value));
            }
        }

        // Update display data
        // The widget will redraw automatically on its next draw cycle
        if let Ok(mut display_data) = self.data.lock() {
            display_data.text = text;
        }
    }

    fn draw(&self, cr: &Context, width: f64, height: f64) -> Result<()> {
        if let Ok(data) = self.data.lock() {
            Self::draw_internal(cr, width as i32, height as i32, &data);
        }
        Ok(())
    }

    fn config_schema(&self) -> ConfigSchema {
        ConfigSchema {
            options: vec![
                ConfigOption {
                    key: "font_size".to_string(),
                    name: "Font Size".to_string(),
                    description: "Size of the text in pixels".to_string(),
                    value_type: "number".to_string(),
                    default: Value::from(24.0),
                },
                ConfigOption {
                    key: "color".to_string(),
                    name: "Text Color".to_string(),
                    description: "RGB color for the text".to_string(),
                    value_type: "color".to_string(),
                    default: Value::from("#FFFFFF"),
                },
            ],
        }
    }

    fn apply_config(&mut self, config: &HashMap<String, Value>) -> Result<()> {
        if let Ok(mut data) = self.data.lock() {
            if let Some(font_size) = config.get("font_size") {
                if let Some(size) = font_size.as_f64() {
                    data.font_size = size;
                }
            }

            if let Some(color) = config.get("color") {
                if let Some(color_str) = color.as_str() {
                    // Parse hex color (e.g., "#FFFFFF")
                    if let Some(rgb) = Self::parse_hex_color(color_str) {
                        data.color = rgb;
                    }
                }
            }

        }

        Ok(())
    }

    fn needs_redraw(&self) -> bool {
        true
    }
}

impl TextDisplayer {
    fn parse_hex_color(hex: &str) -> Option<(f64, f64, f64)> {
        let hex = hex.trim_start_matches('#');
        if hex.len() != 6 {
            return None;
        }

        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;

        Some((r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0))
    }
}
