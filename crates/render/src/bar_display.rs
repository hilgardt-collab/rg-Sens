//! Bar display widget for visualizing values with various styles

use gtk4::cairo;

use crate::background::{Color, ColorStop};
use rg_sens_types::theme::ComboThemeConfig;

// Re-export bar display config types from rg-sens-types
pub use rg_sens_types::display_configs::bar::{
    BarBackgroundType, BarDisplayConfig, BarFillDirection, BarFillType, BarOrientation, BarStyle,
    BarTaperAlignment, BarTaperStyle, BorderConfig, ResolvedBarBackground, ResolvedBarFill,
};

/// Render a bar display
pub fn render_bar(
    cr: &cairo::Context,
    config: &BarDisplayConfig,
    theme: &ComboThemeConfig,
    value: f64,                                                    // 0.0 to 1.0
    values: &std::collections::HashMap<String, serde_json::Value>, // All source data for text overlay
    width: f64,
    height: f64,
) -> Result<(), cairo::Error> {
    // Resolve theme references to actual colors
    let foreground = config.foreground.resolve(theme);
    let background = config.background.resolve(theme);
    let border_color = config.border.resolve_color(theme);

    // Clamp value
    let value = value.clamp(0.0, 1.0);

    match config.style {
        BarStyle::Full => render_full_bar(
            cr,
            config,
            &foreground,
            &background,
            border_color,
            value,
            width,
            height,
        )?,
        BarStyle::Rectangle => render_rectangle_bar(
            cr,
            config,
            &foreground,
            &background,
            border_color,
            value,
            width,
            height,
        )?,
        BarStyle::Segmented => render_segmented_bar(
            cr,
            config,
            &foreground,
            &background,
            border_color,
            value,
            width,
            height,
        )?,
    }

    // Render text overlay if enabled
    if config.text_overlay.enabled {
        render_text_overlay(cr, config, theme, value, values, width, height)?;
    }

    Ok(())
}

/// Calculate tapered dimension at position t (0.0 to 1.0)
fn calculate_tapered_dimension(base_dim: f64, t: f64, style: BarTaperStyle, amount: f64) -> f64 {
    match style {
        BarTaperStyle::None => base_dim,
        BarTaperStyle::Start => {
            // Narrower at start (t=0)
            let factor = 1.0 - amount * (1.0 - t);
            base_dim * factor
        }
        BarTaperStyle::End => {
            // Narrower at end (t=1)
            let factor = 1.0 - amount * t;
            base_dim * factor
        }
        BarTaperStyle::Both => {
            // Narrower at both ends
            let factor = 1.0 - amount * (2.0 * (t - 0.5)).abs();
            base_dim * factor
        }
    }
}

/// Calculate the offset for a tapered segment based on alignment
/// base_dim: the full dimension (height for horizontal bars, width for vertical)
/// tapered_dim: the tapered dimension at this position
/// alignment: where to anchor the taper
fn calculate_taper_offset(base_dim: f64, tapered_dim: f64, alignment: BarTaperAlignment) -> f64 {
    match alignment {
        BarTaperAlignment::Start => 0.0, // Top/Left aligned
        BarTaperAlignment::Center => (base_dim - tapered_dim) / 2.0, // Centered
        BarTaperAlignment::End => base_dim - tapered_dim, // Bottom/Right aligned
    }
}

/// Render full panel style bar
fn render_full_bar(
    cr: &cairo::Context,
    config: &BarDisplayConfig,
    foreground: &ResolvedBarFill,
    background: &ResolvedBarBackground,
    border_color: Color,
    value: f64,
    width: f64,
    height: f64,
) -> Result<(), cairo::Error> {
    let has_taper = config.taper_style != BarTaperStyle::None;

    // Render background
    if has_taper {
        render_tapered_bar_background(cr, config, background, width, height)?;
    } else {
        cr.save()?;
        rounded_rectangle(cr, 0.0, 0.0, width, height, config.corner_radius);
        cr.clip();
        render_background_resolved(cr, background, width, height)?;
        cr.restore()?;
    }

    // Render foreground based on value
    if has_taper {
        render_tapered_bar_foreground(cr, config, foreground, value, 0.0, 0.0, width, height)?;
    } else {
        cr.save()?;

        let (fill_width, fill_height, fill_x, fill_y) = match config.fill_direction {
            BarFillDirection::LeftToRight => (width * value, height, 0.0, 0.0),
            BarFillDirection::RightToLeft => (width * value, height, width * (1.0 - value), 0.0),
            BarFillDirection::BottomToTop => (width, height * value, 0.0, height * (1.0 - value)),
            BarFillDirection::TopToBottom => (width, height * value, 0.0, 0.0),
        };

        // Use rounded rectangle for clipping to apply corner radius
        rounded_rectangle(
            cr,
            fill_x,
            fill_y,
            fill_width,
            fill_height,
            config.corner_radius,
        );
        cr.clip();

        render_foreground_resolved(cr, foreground, config.fill_direction, width, height)?;

        cr.restore()?;
    }

    // Render border with corner radius (only for non-tapered)
    if config.border.enabled && !has_taper {
        render_border_resolved(
            cr,
            border_color,
            config.border.width,
            0.0,
            0.0,
            width,
            height,
            config.corner_radius,
        )?;
    }

    Ok(())
}

/// Calculate number of segments for tapered bars based on dimension.
/// Scales with panel size: ~1 segment per 8 pixels, clamped to 5-25 range.
/// Lower max reduces CPU from Cairo save/restore overhead (was 50, now 25).
#[inline]
fn calculate_tapered_segments(dimension: f64) -> i32 {
    ((dimension / 8.0).round() as i32).clamp(5, 25)
}

/// Render tapered bar background
fn render_tapered_bar_background(
    cr: &cairo::Context,
    config: &BarDisplayConfig,
    background: &ResolvedBarBackground,
    width: f64,
    height: f64,
) -> Result<(), cairo::Error> {
    // Pre-create the fill source ONCE (avoids recreating gradient 50x)
    let source = FillSource::from_resolved_background(background, width, height);
    if source.is_none() {
        return Ok(());
    }

    let is_horizontal = matches!(
        config.fill_direction,
        BarFillDirection::LeftToRight | BarFillDirection::RightToLeft
    );
    let num_segments = calculate_tapered_segments(if is_horizontal { width } else { height });

    cr.save()?;

    if is_horizontal {
        let segment_width = width / num_segments as f64;
        for i in 0..num_segments {
            let t = (i as f64 + 0.5) / num_segments as f64;
            let seg_height =
                calculate_tapered_dimension(height, t, config.taper_style, config.taper_amount);
            let seg_x = i as f64 * segment_width;
            let seg_y = calculate_taper_offset(height, seg_height, config.taper_alignment);

            cr.save()?;
            cr.rectangle(seg_x, seg_y, segment_width + 0.5, seg_height); // +0.5 to avoid gaps
            cr.clip();
            source.apply(cr)?;
            cr.restore()?;
        }
    } else {
        let segment_height = height / num_segments as f64;
        for i in 0..num_segments {
            let t = (i as f64 + 0.5) / num_segments as f64;
            let seg_width =
                calculate_tapered_dimension(width, t, config.taper_style, config.taper_amount);
            let seg_x = calculate_taper_offset(width, seg_width, config.taper_alignment);
            let seg_y = i as f64 * segment_height;

            cr.save()?;
            cr.rectangle(seg_x, seg_y, seg_width, segment_height + 0.5);
            cr.clip();
            source.apply(cr)?;
            cr.restore()?;
        }
    }

    cr.restore()?;
    Ok(())
}

/// Render tapered bar foreground
fn render_tapered_bar_foreground(
    cr: &cairo::Context,
    config: &BarDisplayConfig,
    foreground: &ResolvedBarFill,
    value: f64,
    bar_x: f64,
    bar_y: f64,
    bar_width: f64,
    bar_height: f64,
) -> Result<(), cairo::Error> {
    if value <= 0.0 {
        return Ok(());
    }

    // Pre-create the fill source ONCE (avoids recreating gradient 50x)
    let source = FillSource::from_resolved_foreground(foreground, bar_width, bar_height);

    let is_horizontal = matches!(
        config.fill_direction,
        BarFillDirection::LeftToRight | BarFillDirection::RightToLeft
    );
    let num_segments =
        calculate_tapered_segments(if is_horizontal { bar_width } else { bar_height });

    cr.save()?;

    if is_horizontal {
        let segment_width = bar_width / num_segments as f64;
        // Clamp filled_segments to valid range to prevent index out of bounds
        let filled_segments =
            ((value.clamp(0.0, 1.0) * num_segments as f64).ceil() as i32).min(num_segments);

        for i in 0..filled_segments {
            let draw_index = match config.fill_direction {
                BarFillDirection::RightToLeft => num_segments - 1 - i,
                _ => i,
            };

            let t = (draw_index as f64 + 0.5) / num_segments as f64;
            let seg_height =
                calculate_tapered_dimension(bar_height, t, config.taper_style, config.taper_amount);
            let seg_x = bar_x + draw_index as f64 * segment_width;
            let seg_y =
                bar_y + calculate_taper_offset(bar_height, seg_height, config.taper_alignment);

            cr.save()?;
            cr.rectangle(seg_x, seg_y, segment_width + 0.5, seg_height);
            cr.clip();
            source.apply(cr)?;
            cr.restore()?;
        }
    } else {
        let segment_height = bar_height / num_segments as f64;
        // Clamp filled_segments to valid range to prevent index out of bounds
        let filled_segments =
            ((value.clamp(0.0, 1.0) * num_segments as f64).ceil() as i32).min(num_segments);

        for i in 0..filled_segments {
            let draw_index = match config.fill_direction {
                BarFillDirection::TopToBottom => i,
                _ => num_segments - 1 - i, // BottomToTop
            };

            let t = (draw_index as f64 + 0.5) / num_segments as f64;
            let seg_width =
                calculate_tapered_dimension(bar_width, t, config.taper_style, config.taper_amount);
            let seg_x =
                bar_x + calculate_taper_offset(bar_width, seg_width, config.taper_alignment);
            let seg_y = bar_y + draw_index as f64 * segment_height;

            cr.save()?;
            cr.rectangle(seg_x, seg_y, seg_width, segment_height + 0.5);
            cr.clip();
            source.apply(cr)?;
            cr.restore()?;
        }
    }

    cr.restore()?;
    Ok(())
}

/// Render rectangle style bar
fn render_rectangle_bar(
    cr: &cairo::Context,
    config: &BarDisplayConfig,
    foreground: &ResolvedBarFill,
    background: &ResolvedBarBackground,
    border_color: Color,
    value: f64,
    width: f64,
    height: f64,
) -> Result<(), cairo::Error> {
    let radius = config.corner_radius;
    let has_taper = config.taper_style != BarTaperStyle::None;

    // Calculate bar dimensions based on configured percentages
    let bar_width = width * config.rectangle_width;
    let bar_height = height * config.rectangle_height;

    // Center the bar
    let bar_x = (width - bar_width) / 2.0;
    let bar_y = (height - bar_height) / 2.0;

    // Render background
    if has_taper {
        render_tapered_rectangle_background(
            cr, config, background, bar_x, bar_y, bar_width, bar_height,
        )?;
    } else {
        cr.save()?;
        rounded_rectangle(cr, bar_x, bar_y, bar_width, bar_height, radius);
        cr.clip();
        render_background_resolved(cr, background, width, height)?;
        cr.restore()?;
    }

    // Render foreground based on value
    if has_taper {
        render_tapered_bar_foreground(
            cr, config, foreground, value, bar_x, bar_y, bar_width, bar_height,
        )?;
    } else {
        cr.save()?;

        let (fill_width, fill_height, fill_x, fill_y) = match config.fill_direction {
            BarFillDirection::LeftToRight => (bar_width * value, bar_height, bar_x, bar_y),
            BarFillDirection::RightToLeft => (
                bar_width * value,
                bar_height,
                bar_x + bar_width * (1.0 - value),
                bar_y,
            ),
            BarFillDirection::BottomToTop => (
                bar_width,
                bar_height * value,
                bar_x,
                bar_y + bar_height * (1.0 - value),
            ),
            BarFillDirection::TopToBottom => (bar_width, bar_height * value, bar_x, bar_y),
        };

        rounded_rectangle(cr, fill_x, fill_y, fill_width, fill_height, radius);
        cr.clip();

        render_foreground_resolved(cr, foreground, config.fill_direction, width, height)?;

        cr.restore()?;
    }

    // Render border (only for non-tapered)
    if config.border.enabled && !has_taper {
        rounded_rectangle(cr, bar_x, bar_y, bar_width, bar_height, radius);
        render_border_resolved(
            cr,
            border_color,
            config.border.width,
            bar_x,
            bar_y,
            bar_width,
            bar_height,
            radius,
        )?;
    }

    Ok(())
}

/// Render tapered rectangle background
fn render_tapered_rectangle_background(
    cr: &cairo::Context,
    config: &BarDisplayConfig,
    background: &ResolvedBarBackground,
    bar_x: f64,
    bar_y: f64,
    bar_width: f64,
    bar_height: f64,
) -> Result<(), cairo::Error> {
    // Pre-create the fill source ONCE (avoids recreating gradient 50x)
    let source = FillSource::from_resolved_background(background, bar_width, bar_height);
    if source.is_none() {
        return Ok(());
    }

    let num_segments = 50;
    let is_horizontal = matches!(
        config.fill_direction,
        BarFillDirection::LeftToRight | BarFillDirection::RightToLeft
    );

    cr.save()?;

    if is_horizontal {
        let segment_width = bar_width / num_segments as f64;
        for i in 0..num_segments {
            let t = (i as f64 + 0.5) / num_segments as f64;
            let seg_height =
                calculate_tapered_dimension(bar_height, t, config.taper_style, config.taper_amount);
            let seg_x = bar_x + i as f64 * segment_width;
            let seg_y =
                bar_y + calculate_taper_offset(bar_height, seg_height, config.taper_alignment);

            cr.save()?;
            cr.rectangle(seg_x, seg_y, segment_width + 0.5, seg_height);
            cr.clip();
            source.apply(cr)?;
            cr.restore()?;
        }
    } else {
        let segment_height = bar_height / num_segments as f64;
        for i in 0..num_segments {
            let t = (i as f64 + 0.5) / num_segments as f64;
            let seg_width =
                calculate_tapered_dimension(bar_width, t, config.taper_style, config.taper_amount);
            let seg_x =
                bar_x + calculate_taper_offset(bar_width, seg_width, config.taper_alignment);
            let seg_y = bar_y + i as f64 * segment_height;

            cr.save()?;
            cr.rectangle(seg_x, seg_y, seg_width, segment_height + 0.5);
            cr.clip();
            source.apply(cr)?;
            cr.restore()?;
        }
    }

    cr.restore()?;
    Ok(())
}

/// Render segmented style bar
fn render_segmented_bar(
    cr: &cairo::Context,
    config: &BarDisplayConfig,
    foreground: &ResolvedBarFill,
    background: &ResolvedBarBackground,
    border_color: Color,
    value: f64,
    width: f64,
    height: f64,
) -> Result<(), cairo::Error> {
    let segment_count = config.segment_count.max(1);
    let spacing = config.segment_spacing;

    // Apply segment width/height percentages
    let bar_width = width * config.segment_width;
    let bar_height = height * config.segment_height;

    // Center the segmented bar in the panel
    let bar_x = (width - bar_width) / 2.0;
    let bar_y = (height - bar_height) / 2.0;

    let filled_segments = (value * segment_count as f64).ceil() as u32;

    let is_horizontal = matches!(
        config.fill_direction,
        BarFillDirection::LeftToRight | BarFillDirection::RightToLeft
    );
    let has_taper = config.taper_style != BarTaperStyle::None;

    if is_horizontal {
        let total_spacing = spacing * (segment_count - 1) as f64;
        let segment_width = (bar_width - total_spacing) / segment_count as f64;

        for i in 0..segment_count {
            let reverse = matches!(config.fill_direction, BarFillDirection::RightToLeft);
            let seg_index = if reverse { segment_count - 1 - i } else { i };

            // Calculate taper position (t) based on segment position
            let t = (seg_index as f64 + 0.5) / segment_count as f64;
            let seg_height = if has_taper {
                calculate_tapered_dimension(bar_height, t, config.taper_style, config.taper_amount)
            } else {
                bar_height
            };

            let seg_x = bar_x + seg_index as f64 * (segment_width + spacing);
            let seg_y =
                bar_y + calculate_taper_offset(bar_height, seg_height, config.taper_alignment);

            let is_filled = if reverse {
                i < filled_segments
            } else {
                seg_index < filled_segments
            };

            render_segment(
                cr,
                config,
                foreground,
                background,
                border_color,
                is_filled,
                seg_x,
                seg_y,
                segment_width,
                seg_height,
                bar_x,
                bar_y,
                bar_width,
                bar_height,
            )?;
        }
    } else {
        let total_spacing = spacing * (segment_count - 1) as f64;
        let segment_height = (bar_height - total_spacing) / segment_count as f64;

        for i in 0..segment_count {
            let reverse = matches!(config.fill_direction, BarFillDirection::TopToBottom);
            let seg_index = if reverse { i } else { segment_count - 1 - i };

            // Calculate taper position (t) based on segment position
            let t = (seg_index as f64 + 0.5) / segment_count as f64;
            let seg_width = if has_taper {
                calculate_tapered_dimension(bar_width, t, config.taper_style, config.taper_amount)
            } else {
                bar_width
            };

            let seg_x =
                bar_x + calculate_taper_offset(bar_width, seg_width, config.taper_alignment);
            let seg_y = bar_y + seg_index as f64 * (segment_height + spacing);

            let is_filled = if reverse {
                i < filled_segments
            } else {
                segment_count - 1 - seg_index < filled_segments
            };

            render_segment(
                cr,
                config,
                foreground,
                background,
                border_color,
                is_filled,
                seg_x,
                seg_y,
                seg_width,
                segment_height,
                bar_x,
                bar_y,
                bar_width,
                bar_height,
            )?;
        }
    }

    Ok(())
}

/// Render a single segment
fn render_segment(
    cr: &cairo::Context,
    config: &BarDisplayConfig,
    foreground: &ResolvedBarFill,
    background: &ResolvedBarBackground,
    border_color: Color,
    is_filled: bool,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    full_bar_x: f64,
    full_bar_y: f64,
    full_bar_width: f64,
    full_bar_height: f64,
) -> Result<(), cairo::Error> {
    let radius = config.corner_radius.min(width / 2.0).min(height / 2.0);

    cr.save()?;
    rounded_rectangle(cr, x, y, width, height, radius);

    if is_filled {
        cr.clip();
        // Translate to align gradient with full bar
        cr.translate(-full_bar_x, -full_bar_y);
        render_foreground_resolved(
            cr,
            foreground,
            config.fill_direction,
            full_bar_width,
            full_bar_height,
        )?;
    } else {
        cr.clip();
        // Translate to align gradient with full bar
        cr.translate(-full_bar_x, -full_bar_y);
        render_background_resolved(cr, background, full_bar_width, full_bar_height)?;
    }

    cr.restore()?;

    if config.border.enabled {
        rounded_rectangle(cr, x, y, width, height, radius);
        border_color.apply_to_cairo(cr);
        cr.set_line_width(config.border.width);
        cr.stroke()?;
    }

    Ok(())
}

/// Draw a rounded rectangle path
fn rounded_rectangle(cr: &cairo::Context, x: f64, y: f64, width: f64, height: f64, radius: f64) {
    let radius = radius.min(width / 2.0).min(height / 2.0);

    cr.new_path();
    cr.arc(
        x + radius,
        y + radius,
        radius,
        std::f64::consts::PI,
        3.0 * std::f64::consts::PI / 2.0,
    );
    cr.arc(
        x + width - radius,
        y + radius,
        radius,
        3.0 * std::f64::consts::PI / 2.0,
        0.0,
    );
    cr.arc(
        x + width - radius,
        y + height - radius,
        radius,
        0.0,
        std::f64::consts::PI / 2.0,
    );
    cr.arc(
        x + radius,
        y + height - radius,
        radius,
        std::f64::consts::PI / 2.0,
        std::f64::consts::PI,
    );
    cr.close_path();
}

/// Render background (resolved)
fn render_background_resolved(
    cr: &cairo::Context,
    background: &ResolvedBarBackground,
    width: f64,
    height: f64,
) -> Result<(), cairo::Error> {
    match background {
        ResolvedBarBackground::Solid { color } => {
            color.apply_to_cairo(cr);
            cr.paint()?;
        }
        ResolvedBarBackground::Gradient { stops, angle } => {
            render_gradient(cr, stops, *angle, width, height)?;
        }
        ResolvedBarBackground::Transparent => {
            // Do nothing
        }
    }
    Ok(())
}

/// Render foreground (resolved)
fn render_foreground_resolved(
    cr: &cairo::Context,
    foreground: &ResolvedBarFill,
    _direction: BarFillDirection,
    width: f64,
    height: f64,
) -> Result<(), cairo::Error> {
    match foreground {
        ResolvedBarFill::Solid { color } => {
            color.apply_to_cairo(cr);
            cr.paint()?;
        }
        ResolvedBarFill::Gradient { stops, angle } => {
            render_gradient(cr, stops, *angle, width, height)?;
        }
    }
    Ok(())
}

/// Render a gradient with angle support
fn render_gradient(
    cr: &cairo::Context,
    stops: &[ColorStop],
    angle: f64,
    width: f64,
    height: f64,
) -> Result<(), cairo::Error> {
    if let Some(pattern) = create_gradient_pattern(stops, angle, width, height) {
        cr.set_source(&pattern)?;
        cr.paint()?;
    }
    Ok(())
}

/// Create a reusable gradient pattern (avoids recreating for each segment)
fn create_gradient_pattern(
    stops: &[ColorStop],
    angle: f64,
    width: f64,
    height: f64,
) -> Option<cairo::LinearGradient> {
    if stops.is_empty() {
        return None;
    }

    // Convert angle to radians (same convention as background.rs)
    let angle_rad = angle.to_radians();

    // Calculate gradient vector - same formula as background.rs
    let diagonal = (width * width + height * height).sqrt();
    let x1 = width / 2.0 - diagonal * angle_rad.cos() / 2.0;
    let y1 = height / 2.0 - diagonal * angle_rad.sin() / 2.0;
    let x2 = width / 2.0 + diagonal * angle_rad.cos() / 2.0;
    let y2 = height / 2.0 + diagonal * angle_rad.sin() / 2.0;

    let pattern = cairo::LinearGradient::new(x1, y1, x2, y2);

    for stop in stops {
        pattern.add_color_stop_rgba(
            stop.position,
            stop.color.r,
            stop.color.g,
            stop.color.b,
            stop.color.a,
        );
    }

    Some(pattern)
}

/// Pre-created fill source for efficient tapered bar rendering
enum FillSource {
    Solid(Color),
    Gradient(cairo::LinearGradient),
    None,
}

impl FillSource {
    fn from_resolved_background(
        background: &ResolvedBarBackground,
        width: f64,
        height: f64,
    ) -> Self {
        match background {
            ResolvedBarBackground::Solid { color } => FillSource::Solid(*color),
            ResolvedBarBackground::Gradient { stops, angle } => {
                match create_gradient_pattern(stops, *angle, width, height) {
                    Some(pattern) => FillSource::Gradient(pattern),
                    None => FillSource::None,
                }
            }
            ResolvedBarBackground::Transparent => FillSource::None,
        }
    }

    fn from_resolved_foreground(foreground: &ResolvedBarFill, width: f64, height: f64) -> Self {
        match foreground {
            ResolvedBarFill::Solid { color } => FillSource::Solid(*color),
            ResolvedBarFill::Gradient { stops, angle } => {
                match create_gradient_pattern(stops, *angle, width, height) {
                    Some(pattern) => FillSource::Gradient(pattern),
                    None => FillSource::None,
                }
            }
        }
    }

    fn apply(&self, cr: &cairo::Context) -> Result<(), cairo::Error> {
        match self {
            FillSource::Solid(color) => {
                color.apply_to_cairo(cr);
                cr.paint()?;
            }
            FillSource::Gradient(pattern) => {
                cr.set_source(pattern)?;
                cr.paint()?;
            }
            FillSource::None => {}
        }
        Ok(())
    }

    fn is_none(&self) -> bool {
        matches!(self, FillSource::None)
    }
}

/// Render border with resolved color
fn render_border_resolved(
    cr: &cairo::Context,
    border_color: Color,
    border_width: f64,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    radius: f64,
) -> Result<(), cairo::Error> {
    if radius > 0.0 {
        rounded_rectangle(cr, x, y, width, height, radius);
    } else {
        cr.rectangle(x, y, width, height);
    }

    border_color.apply_to_cairo(cr);
    cr.set_line_width(border_width);
    cr.stroke()?;

    Ok(())
}

/// Render text overlay using shared text renderer
fn render_text_overlay(
    cr: &cairo::Context,
    config: &BarDisplayConfig,
    theme: &ComboThemeConfig,
    _value: f64,
    values: &std::collections::HashMap<String, serde_json::Value>,
    width: f64,
    height: f64,
) -> Result<(), cairo::Error> {
    // Use shared text renderer for proper combined field handling
    crate::text_renderer::render_text_lines_with_theme(
        cr,
        width,
        height,
        &config.text_overlay.text_config,
        values,
        Some(theme),
    );

    Ok(())
}
