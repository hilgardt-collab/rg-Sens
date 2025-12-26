//! Shared text rendering utilities for displayers

use cairo::Context;
use std::collections::HashMap;
use serde_json::Value;

use crate::displayers::{
    CombineDirection, HorizontalPosition, TextBackgroundConfig, TextBackgroundType,
    TextDisplayerConfig, TextFillType, TextLineConfig, TextPosition, VerticalPosition,
};
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

    // All lines in a group share settings from the first line
    let first_line = lines[0];
    let shared_v_pos = first_line.vertical_position();
    let shared_rotation = first_line.rotation_angle;
    let shared_offset_x = first_line.offset_x;
    let shared_offset_y = first_line.offset_y;
    let shared_direction = first_line.combine_direction;
    let shared_alignment = first_line.combine_alignment;

    // Group lines by horizontal position
    let mut left_parts: Vec<(&TextLineConfig, String)> = Vec::new();
    let mut center_parts: Vec<(&TextLineConfig, String)> = Vec::new();
    let mut right_parts: Vec<(&TextLineConfig, String)> = Vec::new();

    for line in lines {
        if let Some(text) = get_field_value(&line.field_id, values) {
            match line.horizontal_position() {
                HorizontalPosition::Left => left_parts.push((line, text)),
                HorizontalPosition::Center => center_parts.push((line, text)),
                HorizontalPosition::Right => right_parts.push((line, text)),
            }
        }
    }

    // Render each group of parts with shared settings
    if !left_parts.is_empty() {
        render_combined_parts(
            cr, width, height, &left_parts, &shared_v_pos, &HorizontalPosition::Left,
            shared_rotation, shared_offset_x, shared_offset_y, shared_direction, shared_alignment,
        );
    }
    if !center_parts.is_empty() {
        render_combined_parts(
            cr, width, height, &center_parts, &shared_v_pos, &HorizontalPosition::Center,
            shared_rotation, shared_offset_x, shared_offset_y, shared_direction, shared_alignment,
        );
    }
    if !right_parts.is_empty() {
        render_combined_parts(
            cr, width, height, &right_parts, &shared_v_pos, &HorizontalPosition::Right,
            shared_rotation, shared_offset_x, shared_offset_y, shared_direction, shared_alignment,
        );
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
    direction: CombineDirection,
    alignment: TextPosition,
) {
    if parts.is_empty() {
        return;
    }

    const SPACING: f64 = 5.0;

    // Calculate dimensions for each part
    let mut part_extents: Vec<cairo::TextExtents> = Vec::new();
    for (config, text) in parts {
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
        part_extents.push(extents);
    }

    // Calculate combined dimensions based on direction
    let (combined_width, combined_height, min_y_bearing) = match direction {
        CombineDirection::Horizontal => {
            let mut total_width = 0.0;
            let mut min_y_bearing: f64 = 0.0;
            let mut max_descent: f64 = 0.0;
            for ext in &part_extents {
                total_width += ext.width();
                min_y_bearing = min_y_bearing.min(ext.y_bearing());
                max_descent = max_descent.max(ext.y_bearing() + ext.height());
            }
            if parts.len() > 1 {
                total_width += SPACING * (parts.len() - 1) as f64;
            }
            (total_width, max_descent - min_y_bearing, min_y_bearing)
        }
        CombineDirection::Vertical => {
            let mut max_width: f64 = 0.0;
            let mut total_height = 0.0;
            let mut min_y_bearing: f64 = 0.0;
            for ext in &part_extents {
                max_width = max_width.max(ext.width());
                total_height += ext.height();
                min_y_bearing = min_y_bearing.min(ext.y_bearing());
            }
            if parts.len() > 1 {
                total_height += SPACING * (parts.len() - 1) as f64;
            }
            (max_width, total_height, min_y_bearing)
        }
    };

    // For rotated text, calculate the rotated bounding box dimensions
    let angle_rad = rotation_angle.to_radians();
    let cos_a = angle_rad.cos().abs();
    let sin_a = angle_rad.sin().abs();
    let rotated_w = combined_width * cos_a + combined_height * sin_a;
    let rotated_h = combined_width * sin_a + combined_height * cos_a;

    let effective_w = if rotation_angle != 0.0 { rotated_w } else { combined_width };
    let effective_h = if rotation_angle != 0.0 { rotated_h } else { combined_height };

    // Calculate starting position
    let base_x = match h_pos {
        HorizontalPosition::Left => 10.0,
        HorizontalPosition::Center => (width - effective_w) / 2.0,
        HorizontalPosition::Right => width - effective_w - 10.0,
    };

    let base_y = match v_pos {
        VerticalPosition::Top => 10.0,
        VerticalPosition::Center => (height - effective_h) / 2.0,
        VerticalPosition::Bottom => height - 10.0 - effective_h,
    };

    cr.save().ok();
    if rotation_angle != 0.0 {
        let center_x = base_x + effective_w / 2.0 + offset_x;
        let center_y = base_y + effective_h / 2.0 + offset_y;
        cr.translate(center_x, center_y);
        cr.rotate(angle_rad);
        cr.translate(-combined_width / 2.0, -min_y_bearing - combined_height / 2.0);
    } else {
        cr.translate(base_x + offset_x, base_y - min_y_bearing + offset_y);
    }

    // Extract alignment components from TextPosition
    let (align_v, align_h) = alignment.to_positions();

    // Render each part based on direction and alignment
    match direction {
        CombineDirection::Horizontal => {
            let mut current_x = 0.0;
            for (i, ((config, text), ext)) in parts.iter().zip(&part_extents).enumerate() {
                // Calculate y offset based on vertical alignment
                let part_height = ext.height();
                let y_offset = match align_v {
                    VerticalPosition::Top => 0.0,
                    VerticalPosition::Center => (combined_height - part_height) / 2.0,
                    VerticalPosition::Bottom => combined_height - part_height,
                };

                render_text_part(cr, config, text, current_x, y_offset, ext);

                current_x += ext.width();
                if i < parts.len() - 1 {
                    current_x += SPACING;
                }
            }
        }
        CombineDirection::Vertical => {
            let mut current_y = 0.0;
            for (i, ((config, text), ext)) in parts.iter().zip(&part_extents).enumerate() {
                // Calculate x offset based on horizontal alignment
                let part_width = ext.width();
                let x_offset = match align_h {
                    HorizontalPosition::Left => 0.0,
                    HorizontalPosition::Center => (combined_width - part_width) / 2.0,
                    HorizontalPosition::Right => combined_width - part_width,
                };

                render_text_part(cr, config, text, x_offset, current_y, ext);

                current_y += ext.height();
                if i < parts.len() - 1 {
                    current_y += SPACING;
                }
            }
        }
    }

    cr.restore().ok();
}

/// Render a single text part with background and fill support
fn render_text_part(
    cr: &Context,
    config: &TextLineConfig,
    text: &str,
    x: f64,
    y: f64,
    extents: &cairo::TextExtents,
) {
    cr.save().ok();

    // Set font
    let font_slant = if config.italic { cairo::FontSlant::Italic } else { cairo::FontSlant::Normal };
    let font_weight = if config.bold { cairo::FontWeight::Bold } else { cairo::FontWeight::Normal };
    cr.select_font_face(&config.font_family, font_slant, font_weight);
    cr.set_font_size(config.font_size);

    // Render background if configured
    render_text_background(cr, &config.text_background, x, y + extents.y_bearing(), extents.width(), extents.height());

    // Position for text
    cr.move_to(x, y);

    // Render text with fill
    render_text_fill(cr, config, text, x, y, extents);

    cr.restore().ok();
}

/// Render text background (solid or gradient)
fn render_text_background(
    cr: &Context,
    bg_config: &TextBackgroundConfig,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) {
    match &bg_config.background {
        TextBackgroundType::None => {}
        TextBackgroundType::Solid { color } => {
            cr.save().ok();
            let padding = bg_config.padding;
            let radius = bg_config.corner_radius;

            // Draw rounded rectangle
            draw_rounded_rect(cr, x - padding, y - padding, width + padding * 2.0, height + padding * 2.0, radius);
            cr.set_source_rgba(color.r, color.g, color.b, color.a);
            cr.fill().ok();
            cr.restore().ok();
        }
        TextBackgroundType::LinearGradient { stops, angle } => {
            cr.save().ok();
            let padding = bg_config.padding;
            let radius = bg_config.corner_radius;
            let bg_x = x - padding;
            let bg_y = y - padding;
            let bg_w = width + padding * 2.0;
            let bg_h = height + padding * 2.0;

            // Create gradient
            let angle_rad = angle.to_radians();
            let cx = bg_x + bg_w / 2.0;
            let cy = bg_y + bg_h / 2.0;
            let length = (bg_w * bg_w + bg_h * bg_h).sqrt() / 2.0;
            let dx = angle_rad.cos() * length;
            let dy = angle_rad.sin() * length;

            let gradient = cairo::LinearGradient::new(cx - dx, cy - dy, cx + dx, cy + dy);
            for stop in stops {
                gradient.add_color_stop_rgba(stop.position, stop.color.r, stop.color.g, stop.color.b, stop.color.a);
            }

            draw_rounded_rect(cr, bg_x, bg_y, bg_w, bg_h, radius);
            cr.set_source(&gradient).ok();
            cr.fill().ok();
            cr.restore().ok();
        }
    }
}

/// Render text with fill (solid or gradient)
fn render_text_fill(
    cr: &Context,
    config: &TextLineConfig,
    text: &str,
    x: f64,
    y: f64,
    extents: &cairo::TextExtents,
) {
    match &config.fill {
        TextFillType::Solid { color } => {
            cr.set_source_rgba(color.r, color.g, color.b, color.a);
            cr.move_to(x, y);
            cr.show_text(text).ok();
        }
        TextFillType::LinearGradient { stops, angle } => {
            // Create text path
            cr.move_to(x, y);
            cr.text_path(text);

            // Create gradient covering the text bounds
            let angle_rad = angle.to_radians();
            let cx = x + extents.width() / 2.0;
            let cy = y + extents.y_bearing() + extents.height() / 2.0;
            let length = (extents.width() * extents.width() + extents.height() * extents.height()).sqrt() / 2.0;
            let dx = angle_rad.cos() * length;
            let dy = angle_rad.sin() * length;

            let gradient = cairo::LinearGradient::new(cx - dx, cy - dy, cx + dx, cy + dy);
            for stop in stops {
                gradient.add_color_stop_rgba(stop.position, stop.color.r, stop.color.g, stop.color.b, stop.color.a);
            }

            cr.set_source(&gradient).ok();
            cr.fill().ok();
        }
    }
}

/// Draw a rounded rectangle path
fn draw_rounded_rect(cr: &Context, x: f64, y: f64, width: f64, height: f64, radius: f64) {
    if radius <= 0.0 {
        cr.rectangle(x, y, width, height);
        return;
    }

    let r = radius.min(width / 2.0).min(height / 2.0);
    cr.new_sub_path();
    cr.arc(x + width - r, y + r, r, -std::f64::consts::FRAC_PI_2, 0.0);
    cr.arc(x + width - r, y + height - r, r, 0.0, std::f64::consts::FRAC_PI_2);
    cr.arc(x + r, y + height - r, r, std::f64::consts::FRAC_PI_2, std::f64::consts::PI);
    cr.arc(x + r, y + r, r, std::f64::consts::PI, 3.0 * std::f64::consts::FRAC_PI_2);
    cr.close_path();
}

fn render_single_line(
    cr: &Context,
    width: f64,
    height: f64,
    line: &TextLineConfig,
    values: &HashMap<String, Value>,
) {
    if let Some(text) = get_field_value(&line.field_id, values) {
        render_text_with_config(cr, width, height, &text, line);
    }
}

/// Render a single text line with full config support (background, gradient fill, rotation)
fn render_text_with_config(
    cr: &Context,
    width: f64,
    height: f64,
    text: &str,
    config: &TextLineConfig,
) {
    cr.save().ok();

    let v_pos = config.vertical_position();
    let h_pos = config.horizontal_position();
    let rotation_angle = config.rotation_angle;
    let offset_x = config.offset_x;
    let offset_y = config.offset_y;

    // Set font
    let font_slant = if config.italic { cairo::FontSlant::Italic } else { cairo::FontSlant::Normal };
    let font_weight = if config.bold { cairo::FontWeight::Bold } else { cairo::FontWeight::Normal };
    cr.select_font_face(&config.font_family, font_slant, font_weight);
    cr.set_font_size(config.font_size);

    // Get text dimensions
    let extents = TEXT_EXTENTS_CACHE
        .lock()
        .ok()
        .and_then(|mut cache| {
            cache.get_or_compute(cr, &config.font_family, config.font_size, config.bold, config.italic, text)
        })
        .unwrap_or_else(|| cairo::TextExtents::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0));

    let text_w = extents.width();
    let text_h = extents.height();

    // Calculate rotated bounding box dimensions
    let angle_rad = rotation_angle.to_radians();
    let cos_a = angle_rad.cos().abs();
    let sin_a = angle_rad.sin().abs();
    let rotated_w = text_w * cos_a + text_h * sin_a;
    let rotated_h = text_w * sin_a + text_h * cos_a;

    let effective_w = if rotation_angle != 0.0 { rotated_w } else { text_w };
    let effective_h = if rotation_angle != 0.0 { rotated_h } else { text_h };

    // Calculate position
    let text_x = match h_pos {
        HorizontalPosition::Left => 10.0,
        HorizontalPosition::Center => (width - effective_w) / 2.0,
        HorizontalPosition::Right => width - effective_w - 10.0,
    };

    let text_y = match v_pos {
        VerticalPosition::Top => 10.0,
        VerticalPosition::Center => (height - effective_h) / 2.0,
        VerticalPosition::Bottom => height - 10.0 - effective_h,
    };

    // Apply rotation and offset
    if rotation_angle != 0.0 {
        let center_x = text_x + effective_w / 2.0 + offset_x;
        let center_y = text_y + effective_h / 2.0 + offset_y;
        cr.translate(center_x, center_y);
        cr.rotate(angle_rad);
        // Position for drawing (relative to center)
        let draw_x = -text_w / 2.0;
        let draw_y = -extents.y_bearing() - text_h / 2.0;

        // Render background
        render_text_background(cr, &config.text_background, draw_x, draw_y + extents.y_bearing(), text_w, text_h);

        // Render text with fill
        render_text_fill(cr, config, text, draw_x, draw_y, &extents);
    } else {
        let adjusted_y = text_y - extents.y_bearing();
        let draw_x = text_x + offset_x;
        let draw_y = adjusted_y + offset_y;

        // Render background
        render_text_background(cr, &config.text_background, draw_x, draw_y + extents.y_bearing(), text_w, text_h);

        // Render text with fill
        render_text_fill(cr, config, text, draw_x, draw_y, &extents);
    }

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
