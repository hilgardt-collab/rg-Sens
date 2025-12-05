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

        // All lines in a group share the same vertical position
        let shared_v_pos = &lines[0].vertical_position;

        // Group lines by horizontal position
        let mut left_parts: Vec<(&TextLineConfig, String)> = Vec::new();
        let mut center_parts: Vec<(&TextLineConfig, String)> = Vec::new();
        let mut right_parts: Vec<(&TextLineConfig, String)> = Vec::new();

        for line in lines {
            if let Some(text) = Self::get_field_value(&line.field_id, values) {
                match line.horizontal_position {
                    HorizontalPosition::Left => left_parts.push((line, text)),
                    HorizontalPosition::Center => center_parts.push((line, text)),
                    HorizontalPosition::Right => right_parts.push((line, text)),
                }
            }
        }

        // Render each group of parts
        if !left_parts.is_empty() {
            Self::render_combined_parts(cr, width, height, &left_parts, shared_v_pos, &HorizontalPosition::Left);
        }
        if !center_parts.is_empty() {
            Self::render_combined_parts(cr, width, height, &center_parts, shared_v_pos, &HorizontalPosition::Center);
        }
        if !right_parts.is_empty() {
            Self::render_combined_parts(cr, width, height, &right_parts, shared_v_pos, &HorizontalPosition::Right);
        }
    }

    fn render_combined_parts(
        cr: &Context,
        width: i32,
        height: i32,
        parts: &[(&TextLineConfig, String)],
        v_pos: &VerticalPosition,
        h_pos: &HorizontalPosition,
    ) {
        if parts.is_empty() {
            return;
        }

        // Calculate total width of all parts combined
        let mut total_width = 0.0;
        let mut part_widths = Vec::new();
        let max_font_size = parts.iter().map(|(cfg, _)| cfg.font_size).fold(0.0, f64::max);

        for (config, text) in parts {
            cr.save().ok();
            cr.select_font_face(&config.font_family, cairo::FontSlant::Normal, cairo::FontWeight::Normal);
            cr.set_font_size(config.font_size);

            let extents = cr.text_extents(text).unwrap_or_else(|_| {
                cairo::TextExtents::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0)
            });

            part_widths.push(extents.width());
            total_width += extents.width();
            cr.restore().ok();
        }

        // Add spacing between parts
        if parts.len() > 1 {
            total_width += 5.0 * (parts.len() - 1) as f64;
        }

        // Calculate starting X position based on horizontal alignment
        let start_x = match h_pos {
            HorizontalPosition::Left => 10.0,
            HorizontalPosition::Center => (width as f64 - total_width) / 2.0,
            HorizontalPosition::Right => width as f64 - total_width - 10.0,
        };

        // Calculate Y position
        let y = match v_pos {
            VerticalPosition::Top => 10.0 + max_font_size,
            VerticalPosition::Center => (height as f64 + max_font_size) / 2.0,
            VerticalPosition::Bottom => height as f64 - 10.0,
        };

        // Render each part sequentially
        let mut current_x = start_x;
        for (i, (config, text)) in parts.iter().enumerate() {
            cr.save().ok();

            // Set font and color for this part
            cr.select_font_face(&config.font_family, cairo::FontSlant::Normal, cairo::FontWeight::Normal);
            cr.set_font_size(config.font_size);
            cr.set_source_rgba(config.color.0, config.color.1, config.color.2, config.color.3);

            // Position and draw this part
            cr.move_to(current_x, y);
            cr.show_text(text).ok();

            cr.restore().ok();

            // Move to next position (add part width + spacing)
            current_x += part_widths[i];
            if i < parts.len() - 1 {
                current_x += 5.0; // spacing between parts
            }
        }
    }

    fn render_single_line(cr: &Context, width: i32, height: i32, line: &TextLineConfig, values: &HashMap<String, Value>) {
        if let Some(text) = Self::get_field_value(&line.field_id, values) {
            Self::render_text_with_alignment(
                cr,
                width,
                height,
                &text,
                &line.font_family,
                line.font_size,
                &line.color,
                &line.vertical_position,
                &line.horizontal_position,
                line.rotation_angle,
            );
        }
    }

    fn render_text_with_alignment(
        cr: &Context,
        width: i32,
        height: i32,
        text: &str,
        font_family: &str,
        font_size: f64,
        color: &(f64, f64, f64, f64),
        v_pos: &VerticalPosition,
        h_pos: &HorizontalPosition,
        rotation_angle: f64,
    ) {
        cr.save().ok();

        // Set font
        cr.select_font_face(font_family, cairo::FontSlant::Normal, cairo::FontWeight::Normal);
        cr.set_font_size(font_size);

        // Set color
        cr.set_source_rgba(color.0, color.1, color.2, color.3);

        // Get text dimensions
        let extents = cr.text_extents(text).unwrap_or_else(|_| {
            cairo::TextExtents::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0)
        });

        // Calculate text origin position for proper alignment
        let text_x = match h_pos {
            HorizontalPosition::Left => 10.0,
            HorizontalPosition::Center => (width as f64 - extents.width()) / 2.0,
            HorizontalPosition::Right => width as f64 - extents.width() - 10.0,
        };

        let text_y = match v_pos {
            VerticalPosition::Top => 10.0 + font_size,
            VerticalPosition::Center => (height as f64 + font_size) / 2.0,
            VerticalPosition::Bottom => height as f64 - 10.0,
        };

        // Apply rotation if needed
        if rotation_angle != 0.0 {
            // For rotation, translate to the desired position, rotate, then draw at origin
            let center_x = text_x + extents.width() / 2.0;
            let center_y = text_y - font_size / 2.0;
            cr.translate(center_x, center_y);
            cr.rotate(rotation_angle.to_radians());
            cr.move_to(-extents.width() / 2.0, font_size / 2.0);
        } else {
            cr.move_to(text_x, text_y);
        }

        cr.show_text(text).ok();
        cr.restore().ok();
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
