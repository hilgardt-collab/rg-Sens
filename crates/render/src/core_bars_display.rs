//! Core bars display - renders multiple CPU core usage bars
//!
//! This module provides a standalone rendering function for displaying
//! multiple CPU core usage values as animated bars. Designed to be reusable
//! by combo displays like LCARS.

use gtk4::cairo;
use serde_json::Value;
use std::collections::HashMap;

use crate::background::{Color, ColorStop};
use crate::bar_display::{
    BarBackgroundType, BarFillDirection, BarFillType, BarOrientation, BarStyle,
};
use crate::pango_text::{pango_show_text, pango_text_extents};
use crate::text_renderer::render_text_lines_with_theme;
use rg_sens_types::theme::{resolve_stops, ComboThemeConfig};

// Re-export core bars config types from rg-sens-types
pub use rg_sens_types::display_configs::core_bars::{CoreBarsConfig, LabelPosition};

/// Render multiple CPU core bars
///
/// # Arguments
/// * `cr` - Cairo context
/// * `config` - Core bars configuration
/// * `core_values` - Slice of normalized values (0.0-1.0) for each core
/// * `width` - Available width
/// * `height` - Available height
pub fn render_core_bars(
    cr: &cairo::Context,
    config: &CoreBarsConfig,
    theme: &ComboThemeConfig,
    core_values: &[f64],
    width: f64,
    height: f64,
) -> Result<(), cairo::Error> {
    render_core_bars_with_values(
        cr,
        config,
        theme,
        core_values,
        width,
        height,
        &HashMap::new(),
    )
}

/// Render multiple CPU core bars with source values for text overlay
///
/// # Arguments
/// * `cr` - Cairo context
/// * `config` - Core bars configuration
/// * `theme` - Theme configuration for color resolution
/// * `core_values` - Slice of normalized values (0.0-1.0) for each core
/// * `width` - Available width
/// * `height` - Available height
/// * `source_values` - Source values for text overlay
pub fn render_core_bars_with_values(
    cr: &cairo::Context,
    config: &CoreBarsConfig,
    theme: &ComboThemeConfig,
    core_values: &[f64],
    width: f64,
    height: f64,
    source_values: &HashMap<String, Value>,
) -> Result<(), cairo::Error> {
    let num_bars = core_values.len();
    if num_bars == 0 {
        return Ok(());
    }

    // Apply padding
    let padded_x = config.padding_left;
    let padded_y = config.padding_top;
    let padded_width = (width - config.padding_left - config.padding_right).max(1.0);
    let padded_height = (height - config.padding_top - config.padding_bottom).max(1.0);

    // Translate to padded origin
    cr.save()?;
    cr.translate(padded_x, padded_y);

    // Resolve the label font once (theme or legacy family/size) and size the
    // label band from it
    let label_font = config.resolved_label_font(theme);
    let label_space = if config.show_labels {
        calculate_label_space(cr, config, &label_font, num_bars)
    } else {
        0.0
    };

    // For horizontal bars (stacked vertically), we divide height
    // For vertical bars (arranged horizontally), we divide width
    match config.orientation {
        BarOrientation::Horizontal => {
            render_horizontal_bars(
                cr,
                config,
                theme,
                core_values,
                padded_width,
                padded_height,
                label_space,
                &label_font,
            )?;
        }
        BarOrientation::Vertical => {
            render_vertical_bars(
                cr,
                config,
                theme,
                core_values,
                padded_width,
                padded_height,
                label_space,
                &label_font,
            )?;
        }
    }

    cr.restore()?;

    // Render text overlay if enabled
    if config.text_overlay.enabled && !config.text_overlay.text_config.lines.is_empty() {
        render_text_lines_with_theme(
            cr,
            width,
            height,
            &config.text_overlay.text_config,
            source_values,
            Some(theme),
        );
    }

    Ok(())
}

/// Calculate space needed for labels
fn calculate_label_space(
    cr: &cairo::Context,
    config: &CoreBarsConfig,
    (font_family, font_size): &(String, f64),
    num_bars: usize,
) -> f64 {
    let font_weight = if config.label_bold {
        cairo::FontWeight::Bold
    } else {
        cairo::FontWeight::Normal
    };

    // Calculate max label width/height
    let max_index = config.start_core + num_bars - 1;
    let test_label = format!("{}{}", config.label_prefix, max_index);

    let extents = pango_text_extents(
        cr,
        &test_label,
        font_family,
        cairo::FontSlant::Normal,
        font_weight,
        *font_size,
    );
    match config.orientation {
        BarOrientation::Horizontal => extents.width() + 8.0,
        BarOrientation::Vertical => extents.height() + 8.0,
    }
}

/// Render horizontal bars stacked vertically
fn render_horizontal_bars(
    cr: &cairo::Context,
    config: &CoreBarsConfig,
    theme: &ComboThemeConfig,
    core_values: &[f64],
    width: f64,
    height: f64,
    label_space: f64,
    label_font: &(String, f64),
) -> Result<(), cairo::Error> {
    let num_bars = core_values.len();
    let total_spacing = config.bar_spacing * (num_bars - 1) as f64;
    let bar_height = (height - total_spacing) / num_bars as f64;

    // Determine bar area based on label position
    let (bar_x, bar_width) = match config.label_position {
        LabelPosition::Start => (label_space, width - label_space),
        LabelPosition::End => (0.0, width - label_space),
        LabelPosition::Inside => (0.0, width),
    };

    for (i, &value) in core_values.iter().enumerate() {
        let bar_y = i as f64 * (bar_height + config.bar_spacing);
        let value = value.clamp(0.0, 1.0);

        // Render the bar
        cr.save()?;
        render_single_bar(
            cr, config, theme, value, bar_x, bar_y, bar_width, bar_height, true, // horizontal
            i, num_bars,
        )?;
        cr.restore()?;

        // Render label if enabled
        if config.show_labels {
            render_label(
                cr,
                config,
                theme,
                config.start_core + i,
                bar_y,
                bar_height,
                width,
                height,
                label_space,
                label_font,
                true, // horizontal
            )?;
        }
    }

    Ok(())
}

/// Render vertical bars arranged horizontally
fn render_vertical_bars(
    cr: &cairo::Context,
    config: &CoreBarsConfig,
    theme: &ComboThemeConfig,
    core_values: &[f64],
    width: f64,
    height: f64,
    label_space: f64,
    label_font: &(String, f64),
) -> Result<(), cairo::Error> {
    let num_bars = core_values.len();
    let total_spacing = config.bar_spacing * (num_bars - 1) as f64;
    let bar_width = (width - total_spacing) / num_bars as f64;

    // Determine bar area based on label position
    let (bar_y, bar_height) = match config.label_position {
        LabelPosition::Start => (label_space, height - label_space),
        LabelPosition::End => (0.0, height - label_space),
        LabelPosition::Inside => (0.0, height),
    };

    for (i, &value) in core_values.iter().enumerate() {
        let bar_x = i as f64 * (bar_width + config.bar_spacing);
        let value = value.clamp(0.0, 1.0);

        // Render the bar
        cr.save()?;
        render_single_bar(
            cr, config, theme, value, bar_x, bar_y, bar_width, bar_height, false, // vertical
            i, num_bars,
        )?;
        cr.restore()?;

        // Render label if enabled
        if config.show_labels {
            render_label(
                cr,
                config,
                theme,
                config.start_core + i,
                bar_x,
                bar_width,
                width,
                height,
                label_space,
                label_font,
                false, // vertical
            )?;
        }
    }

    Ok(())
}

/// Render a single bar
fn render_single_bar(
    cr: &cairo::Context,
    config: &CoreBarsConfig,
    theme: &ComboThemeConfig,
    value: f64,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    horizontal: bool,
    bar_index: usize,
    total_bars: usize,
) -> Result<(), cairo::Error> {
    let radius = config.corner_radius.min(width / 2.0).min(height / 2.0);

    match config.bar_style {
        BarStyle::Full | BarStyle::Rectangle => {
            // Render background
            cr.save()?;
            rounded_rectangle(cr, x, y, width, height, radius);
            cr.clip();
            render_background(cr, &config.background, theme, x, y, width, height)?;
            cr.restore()?;

            // Render foreground based on value and fill direction
            cr.save()?;
            let (fill_x, fill_y, fill_w, fill_h) = match config.fill_direction {
                BarFillDirection::LeftToRight => (x, y, width * value, height),
                BarFillDirection::RightToLeft => {
                    let fill_width = width * value;
                    (x + width - fill_width, y, fill_width, height)
                }
                BarFillDirection::BottomToTop => {
                    let fill_height = height * value;
                    (x, y + height - fill_height, width, fill_height)
                }
                BarFillDirection::TopToBottom => (x, y, width, height * value),
            };

            if fill_w > 0.0 && fill_h > 0.0 {
                rounded_rectangle(cr, fill_x, fill_y, fill_w, fill_h, radius);
                cr.clip();

                // When gradient_spans_bars is true, sample a single color from gradient
                if config.gradient_spans_bars {
                    if let BarFillType::Gradient {
                        stops,
                        theme_gradient,
                        ..
                    } = &config.foreground
                    {
                        let position = if total_bars > 1 {
                            bar_index as f64 / (total_bars - 1) as f64
                        } else {
                            0.5
                        };
                        // Resolve color stops and sample
                        let resolved_stops = resolve_stops(*theme_gradient, stops, theme);
                        let color = sample_gradient_color(&resolved_stops, position);
                        color.apply_to_cairo(cr);
                        cr.paint()?;
                    } else {
                        render_foreground(cr, &config.foreground, theme, x, y, width, height)?;
                    }
                } else {
                    render_foreground(cr, &config.foreground, theme, x, y, width, height)?;
                }
            }
            cr.restore()?;

            // Render border
            if config.border.enabled {
                rounded_rectangle(cr, x, y, width, height, radius);
                let border_color = config.border.color.resolve(theme);
                border_color.apply_to_cairo(cr);
                cr.set_line_width(config.border.width);
                cr.stroke()?;
            }
        }
        BarStyle::Segmented => {
            render_segmented_single_bar(
                cr, config, theme, value, x, y, width, height, horizontal, bar_index, total_bars,
            )?;
        }
    }

    Ok(())
}

/// Render a segmented bar
fn render_segmented_single_bar(
    cr: &cairo::Context,
    config: &CoreBarsConfig,
    theme: &ComboThemeConfig,
    value: f64,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    horizontal: bool,
    bar_index: usize,
    total_bars: usize,
) -> Result<(), cairo::Error> {
    let (segments, radius) = bar_segments(config, value, x, y, width, height, horizontal);

    // Segments with a gap between them and no border are drawn in two
    // batches, the lit ones and the unlit ones: a strip of 32 bars of 50
    // segments is two fills a bar instead of fifty clips and paints, and it
    // is drawn again on every frame of a glide. The picture is the
    // same, because all lit segments of a bar share one paint. With a border,
    // or with segments that touch, each segment is drawn after the one
    // before it and the order shows, so those keep the one-by-one path.
    if config.border.enabled || config.segment_spacing <= 0.0 {
        for &(is_filled, seg_x, seg_y, seg_w, seg_h) in &segments {
            render_segment(
                cr, config, theme, is_filled, seg_x, seg_y, seg_w, seg_h, radius, x, y, width,
                height, bar_index, total_bars,
            )?;
        }
        return Ok(());
    }

    for lit in [true, false] {
        cr.save()?;
        cr.new_path();
        let has_fill = set_segment_source(
            cr, config, theme, lit, x, y, width, height, bar_index, total_bars,
        );
        if matches!(has_fill, Ok(true)) {
            for &(is_filled, seg_x, seg_y, seg_w, seg_h) in &segments {
                if is_filled == lit {
                    rounded_rectangle_subpath(cr, seg_x, seg_y, seg_w, seg_h, radius);
                }
            }
            // filling the segments with the bar's fill is what clipping to
            // them and painting gives, without a mask for the clip
            cr.fill()?;
        }
        cr.restore()?;
        has_fill?;
    }

    Ok(())
}

/// The value a core's bar draws for `value`: a segmented bar changes only
/// when another segment lights, so every value on one step draws as that
/// step (see [`segment_step`](crate::render_utils::segment_step)); the other
/// styles follow the value itself.
pub fn shown_value(config: &CoreBarsConfig, value: f64) -> f64 {
    match config.bar_style {
        BarStyle::Segmented => crate::render_utils::segment_step(value, config.segment_count),
        _ => value,
    }
}

/// One segment of a bar as `(lit, x, y, width, height)`.
type Segment = (bool, f64, f64, f64, f64);

/// The segments of one bar in the order they are drawn, and their corner
/// radius.
fn bar_segments(
    config: &CoreBarsConfig,
    value: f64,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    horizontal: bool,
) -> (Vec<Segment>, f64) {
    let segment_count = config.segment_count.max(1);
    let spacing = config.segment_spacing;
    let filled_segments = (value * segment_count as f64).ceil() as u32;
    let mut segments = Vec::with_capacity(segment_count as usize);

    if horizontal {
        let total_spacing = spacing * (segment_count - 1) as f64;
        let segment_width = (width - total_spacing) / segment_count as f64;
        let radius = config
            .corner_radius
            .min(segment_width / 2.0)
            .min(height / 2.0);

        for i in 0..segment_count {
            // Determine segment index based on fill direction
            let seg_index = match config.fill_direction {
                BarFillDirection::LeftToRight => i,
                BarFillDirection::RightToLeft => segment_count - 1 - i,
                _ => i, // For vertical fill directions, use left-to-right layout
            };

            let seg_x = x + seg_index as f64 * (segment_width + spacing);

            // Determine if filled based on direction
            let is_filled = match config.fill_direction {
                BarFillDirection::RightToLeft => i < filled_segments,
                _ => seg_index < filled_segments,
            };

            segments.push((is_filled, seg_x, y, segment_width, height));
        }
        (segments, radius)
    } else {
        let total_spacing = spacing * (segment_count - 1) as f64;
        let segment_height = (height - total_spacing) / segment_count as f64;
        let radius = config
            .corner_radius
            .min(width / 2.0)
            .min(segment_height / 2.0);

        for i in 0..segment_count {
            // Determine segment index based on fill direction
            let seg_index = match config.fill_direction {
                BarFillDirection::BottomToTop => segment_count - 1 - i,
                BarFillDirection::TopToBottom => i,
                _ => segment_count - 1 - i, // For horizontal fill directions, use bottom-to-top layout
            };

            let seg_y = y + seg_index as f64 * (segment_height + spacing);

            // Determine if filled based on direction
            let is_filled = match config.fill_direction {
                BarFillDirection::TopToBottom => i < filled_segments,
                _ => (segment_count - 1 - seg_index) < filled_segments,
            };

            segments.push((is_filled, x, seg_y, width, segment_height));
        }
        (segments, radius)
    }
}

/// Render a single segment
fn render_segment(
    cr: &cairo::Context,
    config: &CoreBarsConfig,
    theme: &ComboThemeConfig,
    is_filled: bool,
    seg_x: f64,
    seg_y: f64,
    seg_width: f64,
    seg_height: f64,
    radius: f64,
    full_x: f64,
    full_y: f64,
    full_width: f64,
    full_height: f64,
    bar_index: usize,
    total_bars: usize,
) -> Result<(), cairo::Error> {
    cr.save()?;
    rounded_rectangle(cr, seg_x, seg_y, seg_width, seg_height, radius);
    cr.clip();
    let painted = paint_segment(
        cr,
        config,
        theme,
        is_filled,
        full_x,
        full_y,
        full_width,
        full_height,
        bar_index,
        total_bars,
    );
    cr.restore()?;
    painted?;

    if config.border.enabled {
        rounded_rectangle(cr, seg_x, seg_y, seg_width, seg_height, radius);
        let border_color = config.border.color.resolve(theme);
        border_color.apply_to_cairo(cr);
        cr.set_line_width(config.border.width);
        cr.stroke()?;
    }

    Ok(())
}

/// Paint what the clip lets through with a segment's fill (see
/// [`set_segment_source`]).
#[allow(clippy::too_many_arguments)]
fn paint_segment(
    cr: &cairo::Context,
    config: &CoreBarsConfig,
    theme: &ComboThemeConfig,
    is_filled: bool,
    full_x: f64,
    full_y: f64,
    full_width: f64,
    full_height: f64,
    bar_index: usize,
    total_bars: usize,
) -> Result<(), cairo::Error> {
    if set_segment_source(
        cr,
        config,
        theme,
        is_filled,
        full_x,
        full_y,
        full_width,
        full_height,
        bar_index,
        total_bars,
    )? {
        cr.paint()?;
    }
    Ok(())
}

/// Make a segment's fill the context's source: the bar's foreground for lit
/// segments (one colour of the gradient when it spans the bars), its
/// background for the others. The fill is laid out over the whole bar, so
/// every segment of a bar takes its part of the same one. `false`: there is
/// nothing to paint.
#[allow(clippy::too_many_arguments)]
fn set_segment_source(
    cr: &cairo::Context,
    config: &CoreBarsConfig,
    theme: &ComboThemeConfig,
    is_filled: bool,
    full_x: f64,
    full_y: f64,
    full_width: f64,
    full_height: f64,
    bar_index: usize,
    total_bars: usize,
) -> Result<bool, cairo::Error> {
    if !is_filled {
        return set_background_source(
            cr,
            &config.background,
            theme,
            full_x,
            full_y,
            full_width,
            full_height,
        );
    }
    // When gradient_spans_bars is true, sample a single color from gradient
    if config.gradient_spans_bars {
        if let BarFillType::Gradient {
            stops,
            theme_gradient,
            ..
        } = &config.foreground
        {
            let position = if total_bars > 1 {
                bar_index as f64 / (total_bars - 1) as f64
            } else {
                0.5
            };
            // Resolve color stops and sample
            let resolved_stops = resolve_stops(*theme_gradient, stops, theme);
            let color = sample_gradient_color(&resolved_stops, position);
            color.apply_to_cairo(cr);
            return Ok(true);
        }
    }
    set_foreground_source(
        cr,
        &config.foreground,
        theme,
        full_x,
        full_y,
        full_width,
        full_height,
    )
}

/// Render a label for a core
fn render_label(
    cr: &cairo::Context,
    config: &CoreBarsConfig,
    theme: &ComboThemeConfig,
    core_index: usize,
    bar_pos: f64,  // y for horizontal, x for vertical
    bar_size: f64, // height for horizontal, width for vertical
    _width: f64,
    height: f64,
    label_space: f64,
    (font_family, font_size): &(String, f64),
    horizontal: bool,
) -> Result<(), cairo::Error> {
    let label = format!("{}{}", config.label_prefix, core_index);

    let font_weight = if config.label_bold {
        cairo::FontWeight::Bold
    } else {
        cairo::FontWeight::Normal
    };

    let label_color = config.label_color.resolve(theme);
    label_color.apply_to_cairo(cr);

    let extents = pango_text_extents(
        cr,
        &label,
        font_family,
        cairo::FontSlant::Normal,
        font_weight,
        *font_size,
    );
    let text_width = extents.width();
    let text_height = extents.height();

    if horizontal {
        // Label for horizontal bar (on left or right)
        let text_y = bar_pos + (bar_size + text_height) / 2.0;
        let text_x = match config.label_position {
            LabelPosition::Start => (label_space - text_width) / 2.0,
            LabelPosition::End => _width - label_space + (label_space - text_width) / 2.0,
            LabelPosition::Inside => 4.0,
        };
        cr.move_to(text_x, text_y);
    } else {
        // Label for vertical bar (on top or bottom)
        let text_x = bar_pos + (bar_size - text_width) / 2.0;
        let text_y = match config.label_position {
            LabelPosition::Start => label_space / 2.0 + text_height / 2.0,
            LabelPosition::End => height - label_space / 2.0 + text_height / 2.0,
            LabelPosition::Inside => height - 4.0,
        };
        cr.move_to(text_x, text_y);
    }

    pango_show_text(
        cr,
        &label,
        font_family,
        cairo::FontSlant::Normal,
        font_weight,
        *font_size,
    );

    Ok(())
}

/// Draw a rounded rectangle path
fn rounded_rectangle(cr: &cairo::Context, x: f64, y: f64, width: f64, height: f64, radius: f64) {
    if width <= 0.0 || height <= 0.0 {
        return;
    }
    cr.new_path();
    rounded_rectangle_subpath(cr, x, y, width, height, radius);
}

/// Add a rounded rectangle to the current path as a sub-path of its own.
fn rounded_rectangle_subpath(
    cr: &cairo::Context,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    radius: f64,
) {
    if width <= 0.0 || height <= 0.0 {
        return;
    }

    let radius = radius.min(width / 2.0).min(height / 2.0).max(0.0);
    if radius <= 0.0 {
        cr.rectangle(x, y, width, height);
        return;
    }

    // Each corner is one Bézier curve, the quarter circle Cairo's `arc`
    // makes of a right angle, written out: a strip of segmented bars has
    // thousands of corners and is drawn on every frame of a glide, and
    // `arc` works each one out with trigonometry.
    const KAPPA: f64 = 0.552_284_749_830_793_6;
    let k = radius * (1.0 - KAPPA);
    let (left, top, right, bottom) = (x, y, x + width, y + height);
    cr.new_sub_path();
    cr.move_to(left, top + radius);
    cr.curve_to(left, top + k, left + k, top, left + radius, top);
    cr.line_to(right - radius, top);
    cr.curve_to(right - k, top, right, top + k, right, top + radius);
    cr.line_to(right, bottom - radius);
    cr.curve_to(right, bottom - k, right - k, bottom, right - radius, bottom);
    cr.line_to(left + radius, bottom);
    cr.curve_to(left + k, bottom, left, bottom - k, left, bottom - radius);
    cr.close_path();
}

/// Render background
fn render_background(
    cr: &cairo::Context,
    background: &BarBackgroundType,
    theme: &ComboThemeConfig,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), cairo::Error> {
    if set_background_source(cr, background, theme, x, y, width, height)? {
        cr.paint()?;
    }
    Ok(())
}

/// Make the background the context's source without painting. `false`: the
/// background has nothing to paint.
fn set_background_source(
    cr: &cairo::Context,
    background: &BarBackgroundType,
    theme: &ComboThemeConfig,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<bool, cairo::Error> {
    match background {
        BarBackgroundType::Solid { color } => {
            let resolved = color.resolve(theme);
            resolved.apply_to_cairo(cr);
            Ok(true)
        }
        BarBackgroundType::Gradient {
            stops,
            angle,
            theme_gradient,
        } => {
            let resolved_stops = resolve_stops(*theme_gradient, stops, theme);
            set_gradient_source(cr, &resolved_stops, *angle, x, y, width, height)
        }
        BarBackgroundType::Transparent => Ok(false),
    }
}

/// Render foreground
fn render_foreground(
    cr: &cairo::Context,
    foreground: &BarFillType,
    theme: &ComboThemeConfig,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), cairo::Error> {
    if set_foreground_source(cr, foreground, theme, x, y, width, height)? {
        cr.paint()?;
    }
    Ok(())
}

/// Make the foreground the context's source without painting. `false`: the
/// foreground has nothing to paint.
fn set_foreground_source(
    cr: &cairo::Context,
    foreground: &BarFillType,
    theme: &ComboThemeConfig,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<bool, cairo::Error> {
    match foreground {
        BarFillType::Solid { color } => {
            let resolved = color.resolve(theme);
            resolved.apply_to_cairo(cr);
            Ok(true)
        }
        BarFillType::Gradient {
            stops,
            angle,
            theme_gradient,
        } => {
            let resolved_stops = resolve_stops(*theme_gradient, stops, theme);
            set_gradient_source(cr, &resolved_stops, *angle, x, y, width, height)
        }
    }
}

/// Sample a color from gradient stops at a given position (0.0 to 1.0)
fn sample_gradient_color(stops: &[ColorStop], position: f64) -> Color {
    if stops.is_empty() {
        return Color::new(1.0, 1.0, 1.0, 1.0);
    }

    if stops.len() == 1 {
        return stops[0].color;
    }

    let position = position.clamp(0.0, 1.0);

    // Find the two stops to interpolate between
    let mut sorted_stops: Vec<ColorStop> = stops.to_vec();
    sorted_stops.sort_by(|a, b| {
        a.position
            .partial_cmp(&b.position)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Find the stops surrounding our position
    let mut lower_idx = 0;
    let mut upper_idx = sorted_stops.len() - 1;

    for i in 0..sorted_stops.len() - 1 {
        if sorted_stops[i].position <= position && sorted_stops[i + 1].position >= position {
            lower_idx = i;
            upper_idx = i + 1;
            break;
        }
    }

    let lower_stop = &sorted_stops[lower_idx];
    let upper_stop = &sorted_stops[upper_idx];

    // Handle edge cases
    if position <= lower_stop.position {
        return lower_stop.color;
    }
    if position >= upper_stop.position {
        return upper_stop.color;
    }

    // Interpolate between the two stops
    let range = upper_stop.position - lower_stop.position;
    if range <= 0.0 {
        return lower_stop.color;
    }

    let t = (position - lower_stop.position) / range;

    Color::new(
        lower_stop.color.r + (upper_stop.color.r - lower_stop.color.r) * t,
        lower_stop.color.g + (upper_stop.color.g - lower_stop.color.g) * t,
        lower_stop.color.b + (upper_stop.color.b - lower_stop.color.b) * t,
        lower_stop.color.a + (upper_stop.color.a - lower_stop.color.a) * t,
    )
}

/// Make a gradient over the given box the context's source without
/// painting. `false`: there is nothing to paint.
fn set_gradient_source(
    cr: &cairo::Context,
    stops: &[ColorStop],
    angle: f64,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<bool, cairo::Error> {
    if stops.is_empty() || width <= 0.0 || height <= 0.0 {
        return Ok(false);
    }

    let (x1, y1, x2, y2) = crate::background::linear_gradient_endpoints(angle, x, y, width, height);

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

    cr.set_source(&pattern)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pixels(draw: impl Fn(&cairo::Context)) -> Vec<u8> {
        let surface = cairo::ImageSurface::create(cairo::Format::ARgb32, 240, 160).unwrap();
        {
            let cr = cairo::Context::new(&surface).unwrap();
            draw(&cr);
        }
        surface.flush();
        let mut out = Vec::new();
        surface
            .with_data(|data| out.extend_from_slice(data))
            .unwrap();
        out
    }

    /// The corners written out as curves are the corners `arc` draws.
    #[test]
    fn rounded_corners_match_cairo_arcs() {
        use std::f64::consts::PI;
        for (x, y, w, h, r) in [
            (10.5, 20.25, 200.0, 17.0, 3.0),
            (30.0, 8.0, 13.0, 3.2, 1.5),
            (20.0, 20.0, 180.0, 120.0, 24.0),
        ] {
            let curves = pixels(|cr| {
                rounded_rectangle(cr, x, y, w, h, r);
                cr.set_source_rgb(1.0, 1.0, 1.0);
                cr.fill().unwrap();
            });
            let arcs = pixels(|cr| {
                let r = r.min(w / 2.0).min(h / 2.0);
                cr.new_path();
                cr.arc(x + r, y + r, r, PI, 1.5 * PI);
                cr.arc(x + w - r, y + r, r, 1.5 * PI, 0.0);
                cr.arc(x + w - r, y + h - r, r, 0.0, 0.5 * PI);
                cr.arc(x + r, y + h - r, r, 0.5 * PI, PI);
                cr.close_path();
                cr.set_source_rgb(1.0, 1.0, 1.0);
                cr.fill().unwrap();
            });
            let worst = curves
                .iter()
                .zip(&arcs)
                .map(|(a, b)| a.abs_diff(*b))
                .max()
                .unwrap();
            assert!(worst <= 2, "radius {r}: differs by {worst}");
        }
    }

    /// Segmented core bars drawn with the steps their values are on are the
    /// bars drawn with the values.
    #[test]
    fn segmented_core_bars_draw_their_steps() {
        let theme = ComboThemeConfig::default();
        let source = HashMap::new();
        for (count, direction) in [
            (10, BarFillDirection::BottomToTop),
            (50, BarFillDirection::BottomToTop),
            (7, BarFillDirection::LeftToRight),
        ] {
            let config = CoreBarsConfig {
                bar_style: BarStyle::Segmented,
                segment_count: count,
                fill_direction: direction,
                ..Default::default()
            };
            for round in 0..40 {
                let values: Vec<f64> = (0..8)
                    .map(|core| ((core * 37 + round * 113) % 1001) as f64 / 1000.0)
                    .collect();
                let shown: Vec<f64> = values.iter().map(|&v| shown_value(&config, v)).collect();
                let draw = |values: &[f64]| {
                    pixels(|cr| {
                        render_core_bars_with_values(
                            cr, &config, &theme, values, 240.0, 160.0, &source,
                        )
                        .unwrap()
                    })
                };
                assert!(
                    draw(&values) == draw(&shown),
                    "{count} segments, round {round}"
                );
            }
        }
        let plain = CoreBarsConfig {
            bar_style: BarStyle::Full,
            ..Default::default()
        };
        assert_eq!(shown_value(&plain, 0.4321), 0.4321);
    }

    /// Drawing a bar's lit segments in one go, and its unlit ones in
    /// another, gives the picture that drawing them one by one gives.
    #[test]
    fn batched_segments_look_like_segments_drawn_one_by_one() {
        let theme = ComboThemeConfig::default();
        for (horizontal, spans, radius) in [
            (false, false, 2.0),
            (true, false, 0.0),
            (false, true, 1.5),
            (true, true, 3.0),
        ] {
            let mut config = CoreBarsConfig {
                bar_style: BarStyle::Segmented,
                segment_count: 12,
                segment_spacing: 1.5,
                corner_radius: radius,
                gradient_spans_bars: spans,
                ..Default::default()
            };
            config.border.enabled = false;
            let (x, y, w, h) = if horizontal {
                (10.5, 20.25, 200.0, 17.0)
            } else {
                (30.25, 8.5, 13.0, 140.0)
            };
            for value in [0.0, 0.37, 1.0] {
                let batched = pixels(|cr| {
                    render_segmented_single_bar(
                        cr, &config, &theme, value, x, y, w, h, horizontal, 3, 8,
                    )
                    .unwrap();
                });
                let one_by_one = pixels(|cr| {
                    let (segments, radius) = bar_segments(&config, value, x, y, w, h, horizontal);
                    for (lit, sx, sy, sw, sh) in segments {
                        render_segment(
                            cr, &config, &theme, lit, sx, sy, sw, sh, radius, x, y, w, h, 3, 8,
                        )
                        .unwrap();
                    }
                });
                // (an empty bar over a transparent background draws nothing)
                if value > 0.0 {
                    assert!(batched.iter().any(|&b| b != 0), "something is drawn");
                }
                let worst = batched
                    .iter()
                    .zip(&one_by_one)
                    .map(|(a, b)| a.abs_diff(*b))
                    .max()
                    .unwrap();
                assert!(
                    worst <= 1,
                    "horizontal {horizontal}, spans {spans}, radius {radius}, value {value}: differs by {worst}"
                );
            }
        }
    }
}
