//! Shared text rendering utilities for displayers

use cairo::Context;
use std::collections::HashMap;
use serde_json::Value;

use crate::displayers::{TextDisplayerConfig, TextLineConfig, HorizontalPosition, VerticalPosition};
use crate::ui::render_cache::TEXT_EXTENTS_CACHE;

/// Render text lines using a TextDisplayerConfig
pub fn render_text_lines(
    cr: &Context,
    width: f64,
    height: f64,
    config: &TextDisplayerConfig,
    values: &HashMap<String, Value>,
) {
    // Group lines by group_id for combined rendering
    let mut grouped_lines: HashMap<Option<String>, Vec<&TextLineConfig>> = HashMap::new();
    let mut standalone_lines: Vec<&TextLineConfig> = Vec::new();

    for line in &config.lines {
        if line.is_combined {
            grouped_lines.entry(line.group_id.clone())
                .or_default()
                .push(line);
        } else {
            standalone_lines.push(line);
        }
    }

    // Render grouped lines
    for (_, group) in grouped_lines {
        render_line_group(cr, width, height, &group, values);
    }

    // Render standalone lines
    for line in standalone_lines {
        render_single_line(cr, width, height, line, values);
    }
}

fn render_line_group(
    cr: &Context,
    width: f64,
    height: f64,
    lines: &[&TextLineConfig],
    values: &HashMap<String, Value>,
) {
    if lines.is_empty() {
        return;
    }

    // All lines in a group share the same vertical position and rotation from the first line
    let first_line = lines[0];
    let shared_v_pos = &first_line.vertical_position;
    let shared_rotation = first_line.rotation_angle;
    let shared_offset_x = first_line.offset_x;
    let shared_offset_y = first_line.offset_y;

    // Group lines by horizontal position
    let mut left_parts: Vec<(&TextLineConfig, String)> = Vec::new();
    let mut center_parts: Vec<(&TextLineConfig, String)> = Vec::new();
    let mut right_parts: Vec<(&TextLineConfig, String)> = Vec::new();

    for line in lines {
        if let Some(text) = get_field_value(&line.field_id, values) {
            match line.horizontal_position {
                HorizontalPosition::Left => left_parts.push((line, text)),
                HorizontalPosition::Center => center_parts.push((line, text)),
                HorizontalPosition::Right => right_parts.push((line, text)),
            }
        }
    }

    // Render each group of parts with shared rotation and offset
    if !left_parts.is_empty() {
        render_combined_parts(cr, width, height, &left_parts, shared_v_pos, &HorizontalPosition::Left, shared_rotation, shared_offset_x, shared_offset_y);
    }
    if !center_parts.is_empty() {
        render_combined_parts(cr, width, height, &center_parts, shared_v_pos, &HorizontalPosition::Center, shared_rotation, shared_offset_x, shared_offset_y);
    }
    if !right_parts.is_empty() {
        render_combined_parts(cr, width, height, &right_parts, shared_v_pos, &HorizontalPosition::Right, shared_rotation, shared_offset_x, shared_offset_y);
    }
}

fn render_combined_parts(
    cr: &Context,
    width: f64,
    height: f64,
    parts: &[(&TextLineConfig, String)],
    v_pos: &VerticalPosition,
    h_pos: &HorizontalPosition,
    rotation_angle: f64,
    offset_x: f64,
    offset_y: f64,
) {
    if parts.is_empty() {
        return;
    }

    // Calculate total width and combined bounding box of all parts
    let mut total_width = 0.0;
    let mut part_widths = Vec::new();
    let mut min_y_bearing: f64 = 0.0;  // Most negative (highest point above baseline)
    let mut max_descent: f64 = 0.0;    // Lowest point below baseline

    for (config, text) in parts {
        // Use cached text extents to avoid expensive font metric calculations every frame
        let extents = TEXT_EXTENTS_CACHE
            .lock()
            .ok()
            .and_then(|mut cache| {
                cache.get_or_compute(
                    cr,
                    &config.font_family,
                    config.font_size,
                    config.bold,
                    config.italic,
                    text,
                )
            })
            .unwrap_or_else(|| cairo::TextExtents::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0));

        part_widths.push(extents.width());
        total_width += extents.width();

        // Track the combined vertical extent (y_bearing is negative for text above baseline)
        min_y_bearing = min_y_bearing.min(extents.y_bearing());
        max_descent = max_descent.max(extents.y_bearing() + extents.height());
    }

    // Add spacing between parts
    if parts.len() > 1 {
        total_width += 5.0 * (parts.len() - 1) as f64;
    }

    // Calculate combined text height from extents
    let combined_height = max_descent - min_y_bearing;

    // For rotated text, calculate the rotated bounding box dimensions
    let angle_rad = rotation_angle.to_radians();
    let cos_a = angle_rad.cos().abs();
    let sin_a = angle_rad.sin().abs();
    let rotated_w = total_width * cos_a + combined_height * sin_a;
    let rotated_h = total_width * sin_a + combined_height * cos_a;

    // Use rotated dimensions for positioning when text is rotated
    let effective_w = if rotation_angle != 0.0 { rotated_w } else { total_width };
    let effective_h = if rotation_angle != 0.0 { rotated_h } else { combined_height };

    // Calculate starting X position based on horizontal alignment using effective dimensions
    let base_x = match h_pos {
        HorizontalPosition::Left => 10.0,
        HorizontalPosition::Center => (width - effective_w) / 2.0,
        HorizontalPosition::Right => width - effective_w - 10.0,
    };

    // Calculate Y position using effective dimensions for proper centering
    let base_y = match v_pos {
        VerticalPosition::Top => 10.0,
        VerticalPosition::Center => (height - effective_h) / 2.0,
        VerticalPosition::Bottom => height - 10.0 - effective_h,
    };

    // Apply rotation if needed
    cr.save().ok();
    if rotation_angle != 0.0 {
        // Position the center of the rotated bounding box
        let center_x = base_x + effective_w / 2.0 + offset_x;
        let center_y = base_y + effective_h / 2.0 + offset_y;
        cr.translate(center_x, center_y);
        cr.rotate(angle_rad);
        // Move to draw position relative to center (use original dimensions)
        cr.translate(-total_width / 2.0, -min_y_bearing - combined_height / 2.0);
    } else {
        // Just apply offset without rotation, with baseline adjustment
        cr.translate(base_x + offset_x, base_y - min_y_bearing + offset_y);
    }

    // Render each part sequentially
    let mut current_x = 0.0;
    for (i, (config, text)) in parts.iter().enumerate() {
        cr.save().ok();

        // Set font and color for this part
        let font_slant = if config.italic { cairo::FontSlant::Italic } else { cairo::FontSlant::Normal };
        let font_weight = if config.bold { cairo::FontWeight::Bold } else { cairo::FontWeight::Normal };
        cr.select_font_face(&config.font_family, font_slant, font_weight);
        cr.set_font_size(config.font_size);
        cr.set_source_rgba(config.color.0, config.color.1, config.color.2, config.color.3);

        // Position and draw this part
        cr.move_to(current_x, 0.0);
        cr.show_text(text).ok();

        cr.restore().ok();

        // Move to next position (add part width + spacing)
        current_x += part_widths[i];
        if i < parts.len() - 1 {
            current_x += 5.0; // spacing between parts
        }
    }

    cr.restore().ok();
}

fn render_single_line(
    cr: &Context,
    width: f64,
    height: f64,
    line: &TextLineConfig,
    values: &HashMap<String, Value>,
) {
    if let Some(text) = get_field_value(&line.field_id, values) {
        render_text_with_alignment(
            cr,
            width,
            height,
            &text,
            &line.font_family,
            line.font_size,
            line.bold,
            line.italic,
            &line.color,
            &line.vertical_position,
            &line.horizontal_position,
            line.rotation_angle,
            line.offset_x,
            line.offset_y,
        );
    }
}

fn render_text_with_alignment(
    cr: &Context,
    width: f64,
    height: f64,
    text: &str,
    font_family: &str,
    font_size: f64,
    bold: bool,
    italic: bool,
    color: &(f64, f64, f64, f64),
    v_pos: &VerticalPosition,
    h_pos: &HorizontalPosition,
    rotation_angle: f64,
    offset_x: f64,
    offset_y: f64,
) {
    cr.save().ok();

    // Set font with bold/italic support
    let font_slant = if italic { cairo::FontSlant::Italic } else { cairo::FontSlant::Normal };
    let font_weight = if bold { cairo::FontWeight::Bold } else { cairo::FontWeight::Normal };
    cr.select_font_face(font_family, font_slant, font_weight);
    cr.set_font_size(font_size);

    // Set color
    cr.set_source_rgba(color.0, color.1, color.2, color.3);

    // Get text dimensions using cached extents
    let extents = TEXT_EXTENTS_CACHE
        .lock()
        .ok()
        .and_then(|mut cache| {
            cache.get_or_compute(cr, font_family, font_size, bold, italic, text)
        })
        .unwrap_or_else(|| cairo::TextExtents::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0));

    // For rotated text, we need to account for the rotated bounding box dimensions
    let text_w = extents.width();
    let text_h = extents.height();

    // Calculate rotated bounding box dimensions
    let angle_rad = rotation_angle.to_radians();
    let cos_a = angle_rad.cos().abs();
    let sin_a = angle_rad.sin().abs();
    let rotated_w = text_w * cos_a + text_h * sin_a;
    let rotated_h = text_w * sin_a + text_h * cos_a;

    // Use rotated dimensions for positioning when text is rotated
    let effective_w = if rotation_angle != 0.0 { rotated_w } else { text_w };
    let effective_h = if rotation_angle != 0.0 { rotated_h } else { text_h };

    // Calculate text origin position for proper alignment (before offsets)
    // Use effective (rotated) dimensions for left/right/center alignment
    let text_x = match h_pos {
        HorizontalPosition::Left => 10.0,
        HorizontalPosition::Center => (width - effective_w) / 2.0,
        HorizontalPosition::Right => width - effective_w - 10.0,
    };

    // Calculate Y position using effective (rotated) dimensions for proper centering
    let text_y = match v_pos {
        VerticalPosition::Top => 10.0,
        VerticalPosition::Center => (height - effective_h) / 2.0,
        VerticalPosition::Bottom => height - 10.0 - effective_h,
    };

    // Apply offset and rotation
    if rotation_angle != 0.0 {
        // Position the center of the rotated bounding box
        let center_x = text_x + effective_w / 2.0 + offset_x;
        let center_y = text_y + effective_h / 2.0 + offset_y;
        cr.translate(center_x, center_y);
        cr.rotate(angle_rad);
        // Move to draw position relative to center (use original text dimensions)
        cr.move_to(-text_w / 2.0, -extents.y_bearing() - text_h / 2.0);
    } else {
        // No rotation - use original y calculation with baseline adjustment
        let adjusted_y = text_y - extents.y_bearing();
        cr.move_to(text_x + offset_x, adjusted_y + offset_y);
    }

    cr.show_text(text).ok();
    cr.restore().ok();
}

fn get_field_value(field_id: &str, values: &HashMap<String, Value>) -> Option<String> {
    let result = values.get(field_id).map(|value| {
        match value {
            Value::Number(n) => {
                if let Some(f) = n.as_f64() {
                    format!("{:.1}", f)
                } else {
                    n.to_string()
                }
            }
            Value::String(s) => s.clone(),
            Value::Bool(b) => b.to_string(),
            _ => format!("{}", value),
        }
    });
    if result.is_none() {
        log::debug!("Text overlay field '{}' not found in values", field_id);
    }
    result
}
