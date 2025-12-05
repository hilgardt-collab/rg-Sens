//! Text displayer implementation

use crate::core::{ConfigOption, ConfigSchema, Displayer};
use crate::displayers::{TextDisplayerConfig, TextLineConfig, VerticalPosition, HorizontalPosition};
use anyhow::Result;
use cairo::Context;
use gtk4::{glib, prelude::*, DrawingArea, Widget};
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
    /// Current data values from the source
    values: HashMap<String, Value>,
    /// Text display configuration
    config: TextDisplayerConfig,
}

impl TextDisplayer {
    pub fn new() -> Self {
        let data = Arc::new(Mutex::new(DisplayData {
            values: HashMap::new(),
            config: TextDisplayerConfig::default(),
        }));

        Self {
            id: "text".to_string(),
            name: "Text Display".to_string(),
            data,
        }
    }

    fn draw_internal(cr: &Context, width: i32, height: i32, data: &DisplayData) {
        // Don't clear background - let the custom panel background show through

        // Group lines by group_id for combined rendering
        let mut grouped_lines: HashMap<Option<String>, Vec<&TextLineConfig>> = HashMap::new();
        let mut standalone_lines: Vec<&TextLineConfig> = Vec::new();

        for line in &data.config.lines {
            if line.is_combined {
                grouped_lines.entry(line.group_id.clone())
                    .or_insert_with(Vec::new)
                    .push(line);
            } else {
                standalone_lines.push(line);
            }
        }

        // Render grouped lines
        for (_, group) in grouped_lines {
            Self::render_line_group(cr, width, height, &group, &data.values);
        }

        // Render standalone lines
        for line in standalone_lines {
            Self::render_single_line(cr, width, height, line, &data.values);
        }
    }

    fn render_line_group(cr: &Context, width: i32, height: i32, lines: &[&TextLineConfig], values: &HashMap<String, Value>) {
        if lines.is_empty() {
            return;
        }

        // Use the position of the first line in the group
        let primary_line = lines[0];

        // Build combined text with left/center/right positioning
        let mut left_parts = Vec::new();
        let mut center_parts = Vec::new();
        let mut right_parts = Vec::new();

        for line in lines {
            if let Some(text) = Self::get_field_value(&line.field_id, values) {
                match line.horizontal_position {
                    HorizontalPosition::Left => left_parts.push(text),
                    HorizontalPosition::Center => center_parts.push(text),
                    HorizontalPosition::Right => right_parts.push(text),
                }
            }
        }

        let combined_text = format!(
            "{}{}{}{}{}",
            left_parts.join(" "),
            if !left_parts.is_empty() && !center_parts.is_empty() { " " } else { "" },
            center_parts.join(" "),
            if (!left_parts.is_empty() || !center_parts.is_empty()) && !right_parts.is_empty() { " " } else { "" },
            right_parts.join(" ")
        );

        Self::render_text_at_position(cr, width, height, &combined_text, primary_line);
    }

    fn render_single_line(cr: &Context, width: i32, height: i32, line: &TextLineConfig, values: &HashMap<String, Value>) {
        if let Some(text) = Self::get_field_value(&line.field_id, values) {
            Self::render_text_at_position(cr, width, height, &text, line);
        }
    }

    fn render_text_at_position(cr: &Context, width: i32, height: i32, text: &str, config: &TextLineConfig) {
        cr.save().ok();

        // Set font
        cr.select_font_face(&config.font_family, cairo::FontSlant::Normal, cairo::FontWeight::Normal);
        cr.set_font_size(config.font_size);

        // Set color
        cr.set_source_rgba(config.color.0, config.color.1, config.color.2, config.color.3);

        // Get text dimensions
        let extents = cr.text_extents(text).unwrap_or_else(|_| {
            cairo::TextExtents::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0)
        });

        // Calculate position based on vertical and horizontal alignment
        let (base_x, base_y) = Self::calculate_position(
            width,
            height,
            extents.width(),
            extents.height(),
            &config.vertical_position,
            &config.horizontal_position,
        );

        // Apply rotation if needed
        if config.rotation_angle != 0.0 {
            cr.translate(base_x, base_y);
            cr.rotate(config.rotation_angle.to_radians());
            cr.move_to(-extents.x_bearing(), -extents.y_bearing());
        } else {
            cr.move_to(base_x - extents.x_bearing(), base_y - extents.y_bearing());
        }

        cr.show_text(text).ok();
        cr.restore().ok();
    }

    fn calculate_position(
        width: i32,
        height: i32,
        text_width: f64,
        text_height: f64,
        v_pos: &VerticalPosition,
        h_pos: &HorizontalPosition,
    ) -> (f64, f64) {
        let x = match h_pos {
            HorizontalPosition::Left => text_width / 2.0 + 10.0,
            HorizontalPosition::Center => width as f64 / 2.0,
            HorizontalPosition::Right => width as f64 - text_width / 2.0 - 10.0,
        };

        let y = match v_pos {
            VerticalPosition::Top => text_height / 2.0 + 10.0,
            VerticalPosition::Center => height as f64 / 2.0,
            VerticalPosition::Bottom => height as f64 - text_height / 2.0 - 10.0,
        };

        (x, y)
    }

    fn get_field_value(field_id: &str, values: &HashMap<String, Value>) -> Option<String> {
        values.get(field_id).and_then(|v| {
            match v {
                Value::String(s) => Some(s.clone()),
                Value::Number(n) => {
                    if let Some(f) = n.as_f64() {
                        Some(format!("{:.1}", f))
                    } else {
                        Some(n.to_string())
                    }
                }
                Value::Bool(b) => Some(b.to_string()),
                _ => Some(format!("{}", v)),
            }
        })
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
        drawing_area.set_draw_func(move |_widget, cr, width, height| {
            if let Ok(data) = data_clone.lock() {
                Self::draw_internal(cr, width, height, &data);
            }
        });

        // Set up periodic redraw using timeout
        // This requests a redraw every 500ms to update the display without creating an infinite loop
        glib::timeout_add_local(std::time::Duration::from_millis(500), {
            let drawing_area = drawing_area.clone();
            move || {
                drawing_area.queue_draw();
                glib::ControlFlow::Continue
            }
        });

        drawing_area.upcast()
    }

    fn update_data(&mut self, data: &HashMap<String, Value>) {
        // Store the data values
        if let Ok(mut display_data) = self.data.lock() {
            display_data.values = data.clone();
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
        // Try to deserialize the config as TextDisplayerConfig
        if let Some(lines_value) = config.get("lines") {
            if let Ok(text_config) = serde_json::from_value::<TextDisplayerConfig>(
                serde_json::json!({ "lines": lines_value })
            ) {
                if let Ok(mut data) = self.data.lock() {
                    data.config = text_config;
                }
            }
        }

        Ok(())
    }

    fn needs_redraw(&self) -> bool {
        true
    }
}
