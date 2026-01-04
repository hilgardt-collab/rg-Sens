//! Shared text rendering utilities for displayers

use cairo::Context;
use std::collections::HashMap;
use serde_json::Value;

use crate::displayers::{
    CombineDirection, HorizontalPosition, TextBackgroundConfig, TextBackgroundType,
    TextDisplayerConfig, TextFillType, TextLineConfig, TextPosition, VerticalPosition,
};
use crate::ui::render_cache::TEXT_EXTENTS_CACHE;
use crate::ui::theme::ComboThemeConfig;

/// Render text lines using a TextDisplayerConfig
pub fn render_text_lines(
    cr: &Context,
    width: f64,
    height: f64,
    config: &TextDisplayerConfig,
    values: &HashMap<String, Value>,
) {
    render_text_lines_with_theme(cr, width, height, config, values, None);
}

/// Render text lines with theme support for resolving theme colors
pub fn render_text_lines_with_theme(
    cr: &Context,
    width: f64,
    height: f64,
    config: &TextDisplayerConfig,
    values: &HashMap<String, Value>,
    theme: Option<&ComboThemeConfig>,
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
        render_line_group(cr, width, height, &group, values, theme);
    }

    // Render standalone lines
    for line in standalone_lines {
        render_single_line(cr, width, height, line, values, theme);
    }
}

fn render_line_group(
    cr: &Context,
    width: f64,
    height: f64,
    lines: &[&TextLineConfig],
    values: &HashMap<String, Value>,
    theme: Option<&ComboThemeConfig>,
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
            shared_rotation, shared_offset_x, shared_offset_y, shared_direction, shared_alignment, theme,
        );
    }
    if !center_parts.is_empty() {
        render_combined_parts(
            cr, width, height, &center_parts, &shared_v_pos, &HorizontalPosition::Center,
            shared_rotation, shared_offset_x, shared_offset_y, shared_direction, shared_alignment, theme,
        );
    }
    if !right_parts.is_empty() {
        render_combined_parts(
            cr, width, height, &right_parts, &shared_v_pos, &HorizontalPosition::Right,
            shared_rotation, shared_offset_x, shared_offset_y, shared_direction, shared_alignment, theme,
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
    theme: Option<&ComboThemeConfig>,
) {
    if parts.is_empty() {
        return;
    }

    const SPACING: f64 = 5.0;

    // Calculate dimensions for each part
    let mut part_extents: Vec<cairo::TextExtents> = Vec::new();
    for (config, text) in parts {
        // Resolve font using theme if available
        let (font_family, font_size) = config.resolved_font(theme);
        let extents = TEXT_EXTENTS_CACHE
            .lock()
            .ok()
            .and_then(|mut cache| {
                cache.get_or_compute(
                    cr,
                    &font_family,
                    font_size,
                    config.bold,
                    config.italic,
                    text,
                )
            })
            .unwrap_or_else(|| cairo::TextExtents::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0));
        part_extents.push(extents);
    }

    // Calculate combined dimensions based on direction
    let (combined_width, combined_height, _min_y_bearing) = match direction {
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
        cr.translate(-combined_width / 2.0, -combined_height / 2.0);
    } else {
        // Translate to top-left of bounding box (y=0 is at top of combined area)
        // Each part will add its own baseline offset via baseline_y calculation
        cr.translate(base_x + offset_x, base_y + offset_y);
    }

    // Check if any part has a background configured - use first non-None background for group
    // For grouped text, we render a single background for the entire bounding box
    let group_bg_config = parts.iter()
        .map(|(config, _)| &config.text_background)
        .find(|bg| !bg.background.is_none());

    // Render group background if any part has one configured
    if let Some(bg_config) = group_bg_config {
        // Render background for entire combined bounding box
        render_text_background(cr, bg_config, 0.0, 0.0, combined_width, combined_height, theme);
    }

    // Extract alignment components from TextPosition
    let (align_v, align_h) = alignment.to_positions();

    // Render each part based on direction and alignment
    // Note: y_offset is calculated as position of text TOP, but render_text_part
    // expects baseline position. We convert by subtracting y_bearing (which is negative).
    // Skip individual backgrounds since we rendered the group background above
    let skip_individual_bg = group_bg_config.is_some();

    match direction {
        CombineDirection::Horizontal => {
            let mut current_x = 0.0;
            for (i, ((config, text), ext)) in parts.iter().zip(&part_extents).enumerate() {
                // Calculate y offset for top of text based on vertical alignment
                let part_height = ext.height();
                let top_offset = match align_v {
                    VerticalPosition::Top => 0.0,
                    VerticalPosition::Center => (combined_height - part_height) / 2.0,
                    VerticalPosition::Bottom => combined_height - part_height,
                };

                // Convert top offset to baseline position: baseline = top - y_bearing
                // (y_bearing is negative, so we subtract it to go down to baseline)
                let baseline_y = top_offset - ext.y_bearing();

                render_text_part(cr, config, text, current_x, baseline_y, ext, theme, skip_individual_bg);

                current_x += ext.width();
                if i < parts.len() - 1 {
                    current_x += SPACING;
                }
            }
        }
        CombineDirection::Vertical => {
            let mut current_top = 0.0;
            for (i, ((config, text), ext)) in parts.iter().zip(&part_extents).enumerate() {
                // Calculate x offset based on horizontal alignment
                let part_width = ext.width();
                let x_offset = match align_h {
                    HorizontalPosition::Left => 0.0,
                    HorizontalPosition::Center => (combined_width - part_width) / 2.0,
                    HorizontalPosition::Right => combined_width - part_width,
                };

                // Convert top position to baseline position
                let baseline_y = current_top - ext.y_bearing();

                render_text_part(cr, config, text, x_offset, baseline_y, ext, theme, skip_individual_bg);

                current_top += ext.height();
                if i < parts.len() - 1 {
                    current_top += SPACING;
                }
            }
        }
    }

    cr.restore().ok();
}

/// Render a single text part with background and fill support
/// If `skip_background` is true, doesn't render individual background (used for grouped text
/// where the group background is rendered separately)
fn render_text_part(
    cr: &Context,
    config: &TextLineConfig,
    text: &str,
    x: f64,
    y: f64,
    extents: &cairo::TextExtents,
    theme: Option<&ComboThemeConfig>,
    skip_background: bool,
) {
    cr.save().ok();

    // Resolve font using theme if available
    let (font_family, font_size) = config.resolved_font(theme);

    // Debug log font resolution
    if let Some(ref source) = config.font_source {
        if let Some(t) = theme {
            log::trace!(
                "render_single_text: font_source={:?}, theme T1='{}' T2='{}', resolved='{}'",
                source, t.font1_family, t.font2_family, font_family
            );
        }
    }

    // Set font using cached ScaledFont to prevent memory leaks
    let font_slant = if config.italic { cairo::FontSlant::Italic } else { cairo::FontSlant::Normal };
    let font_weight = if config.bold { cairo::FontWeight::Bold } else { cairo::FontWeight::Normal };
    crate::ui::render_cache::apply_cached_font(cr, &font_family, font_slant, font_weight, font_size);

    // Render background if configured (skip when rendering grouped text - group renders its own background)
    if !skip_background {
        render_text_background(cr, &config.text_background, x, y + extents.y_bearing(), extents.width(), extents.height(), theme);
    }

    // Position for text
    cr.move_to(x, y);

    // Render text with fill
    render_text_fill(cr, config, text, x, y, extents, theme);

    cr.restore().ok();
}

/// Render text background (solid or gradient) with theme support
fn render_text_background(
    cr: &Context,
    bg_config: &TextBackgroundConfig,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    theme: Option<&ComboThemeConfig>,
) {
    match &bg_config.background {
        TextBackgroundType::None => {}
        TextBackgroundType::Solid { color } => {
            cr.save().ok();
            let padding = bg_config.padding;
            let radius = bg_config.corner_radius;

            // Resolve color against theme
            let resolved = if let Some(theme) = theme {
                color.resolve(theme)
            } else {
                match color {
                    crate::ui::theme::ColorSource::Custom { color } => *color,
                    crate::ui::theme::ColorSource::Theme { .. } => {
                        // Fallback to semi-transparent gray when no theme
                        crate::ui::background::Color::new(0.3, 0.3, 0.3, 0.5)
                    }
                }
            };

            // Draw rounded rectangle
            draw_rounded_rect(cr, x - padding, y - padding, width + padding * 2.0, height + padding * 2.0, radius);
            cr.set_source_rgba(resolved.r, resolved.g, resolved.b, resolved.a);
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
    theme: Option<&ComboThemeConfig>,
) {
    // Resolve the primary color using theme if available
    let color = config.fill.primary_color(theme);

    match &config.fill {
        TextFillType::Solid { .. } => {
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
    theme: Option<&ComboThemeConfig>,
) {
    if let Some(text) = get_field_value(&line.field_id, values) {
        render_text_with_config(cr, width, height, &text, line, theme);
    }
}

/// Render a single text line with full config support (background, gradient fill, rotation)
fn render_text_with_config(
    cr: &Context,
    width: f64,
    height: f64,
    text: &str,
    config: &TextLineConfig,
    theme: Option<&ComboThemeConfig>,
) {
    cr.save().ok();

    let v_pos = config.vertical_position();
    let h_pos = config.horizontal_position();
    let rotation_angle = config.rotation_angle;
    let offset_x = config.offset_x;
    let offset_y = config.offset_y;

    // Resolve font using theme if available
    let (font_family, font_size) = config.resolved_font(theme);

    // Set font using cached ScaledFont to prevent memory leaks
    let font_slant = if config.italic { cairo::FontSlant::Italic } else { cairo::FontSlant::Normal };
    let font_weight = if config.bold { cairo::FontWeight::Bold } else { cairo::FontWeight::Normal };
    crate::ui::render_cache::apply_cached_font(cr, &font_family, font_slant, font_weight, font_size);

    // Get text dimensions
    let extents = TEXT_EXTENTS_CACHE
        .lock()
        .ok()
        .and_then(|mut cache| {
            cache.get_or_compute(cr, &font_family, font_size, config.bold, config.italic, text)
        })
        .unwrap_or_else(|| cairo::TextExtents::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0));

    let text_w = extents.width();
    let text_h = extents.height();

    // Get font metrics for consistent vertical positioning
    // Using font ascent ensures all text at same font size aligns consistently,
    // regardless of whether specific characters have descenders
    let (font_height, font_ascent) = if let Ok(font_extents) = cr.font_extents() {
        (font_extents.ascent() + font_extents.descent(), font_extents.ascent())
    } else {
        // Fallback to text extents if font_extents fails
        (text_h, -extents.y_bearing())
    };

    // Calculate rotated bounding box dimensions
    let angle_rad = rotation_angle.to_radians();
    let cos_a = angle_rad.cos().abs();
    let sin_a = angle_rad.sin().abs();
    let rotated_w = text_w * cos_a + text_h * sin_a;
    let rotated_h = text_w * sin_a + text_h * cos_a;

    let effective_w = if rotation_angle != 0.0 { rotated_w } else { text_w };
    // Use font_height for vertical positioning to ensure consistent alignment
    let effective_h = if rotation_angle != 0.0 { rotated_h } else { font_height };

    // Calculate position
    let text_x = match h_pos {
        HorizontalPosition::Left => 10.0,
        HorizontalPosition::Center => (width - effective_w) / 2.0,
        HorizontalPosition::Right => width - effective_w - 10.0,
    };

    // Use font metrics for vertical positioning (consistent across all text at same font size)
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
        render_text_background(cr, &config.text_background, draw_x, draw_y + extents.y_bearing(), text_w, text_h, theme);

        // Render text with fill
        render_text_fill(cr, config, text, draw_x, draw_y, &extents, theme);
    } else {
        // Position baseline: text_y is top of font box, add ascent to get baseline
        let baseline_y = text_y + font_ascent;
        let draw_x = text_x + offset_x;
        let draw_y = baseline_y + offset_y;

        // Render background (use actual text extents for background sizing)
        render_text_background(cr, &config.text_background, draw_x, draw_y + extents.y_bearing(), text_w, text_h, theme);

        // Render text with fill
        render_text_fill(cr, config, text, draw_x, draw_y, &extents, theme);
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
