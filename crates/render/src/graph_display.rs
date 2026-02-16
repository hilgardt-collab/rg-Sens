//! Graph display rendering module

use anyhow::Result;
use cairo::{Context, LineCap, LineJoin};
use std::cell::RefCell;
use std::collections::VecDeque;

use super::pango_text::{pango_show_text, pango_text_extents};
use rg_sens_types::theme::ComboThemeConfig;

// Re-export graph display config types from rg-sens-types
pub use rg_sens_types::display_configs::graph::{
    AxisConfig, DataPoint, FillMode, GraphDisplayConfig, GraphType, LineStyle, Margin,
};

// Thread-local buffer for graph points to avoid allocation per frame
// Pre-sized for 128 points which covers most use cases (typical max is 60-120)
thread_local! {
    static POINTS_BUFFER: RefCell<Vec<(f64, f64)>> = RefCell::new(Vec::with_capacity(128));
}

/// Apply line style to Cairo context
fn apply_line_style(cr: &Context, style: LineStyle, width: f64) {
    match style {
        LineStyle::Solid => {
            cr.set_dash(&[], 0.0);
        }
        LineStyle::Dashed => {
            let pattern = [width * 4.0, width * 2.0];
            cr.set_dash(&pattern, 0.0);
        }
        LineStyle::Dotted => {
            let pattern = [width, width];
            cr.set_dash(&pattern, 0.0);
        }
    }
}

/// Render graph display
/// scroll_offset: 0.0 to 1.0, represents smooth scroll progress toward next point position
pub fn render_graph(
    cr: &Context,
    config: &GraphDisplayConfig,
    data: &VecDeque<DataPoint>,
    source_values: &std::collections::HashMap<String, serde_json::Value>,
    width: f64,
    height: f64,
    scroll_offset: f64,
) -> Result<()> {
    // Use theme from config for color/font resolution
    render_graph_with_theme(
        cr,
        config,
        data,
        source_values,
        width,
        height,
        scroll_offset,
        Some(&config.theme),
    )
}

/// Render graph display with optional theme for color resolution
pub fn render_graph_with_theme(
    cr: &Context,
    config: &GraphDisplayConfig,
    data: &VecDeque<DataPoint>,
    source_values: &std::collections::HashMap<String, serde_json::Value>,
    width: f64,
    height: f64,
    scroll_offset: f64,
    theme: Option<&ComboThemeConfig>,
) -> Result<()> {
    // Resolve theme colors
    let default_theme = ComboThemeConfig::default();
    let theme = theme.unwrap_or(&default_theme);

    let line_color = config.line_color.resolve(theme);
    let fill_color = config.fill_color.resolve(theme);
    let fill_gradient_start = config.fill_gradient_start.resolve(theme);
    let fill_gradient_end = config.fill_gradient_end.resolve(theme);
    let point_color = config.point_color.resolve(theme);

    // Resolve axis colors
    let y_axis_color = config.y_axis.color.resolve(theme);
    let y_axis_label_color = config.y_axis.label_color.resolve(theme);
    let y_axis_grid_color = config.y_axis.grid_color.resolve(theme);
    let x_axis_color = config.x_axis.color.resolve(theme);
    let _x_axis_label_color = config.x_axis.label_color.resolve(theme);
    let x_axis_grid_color = config.x_axis.grid_color.resolve(theme);

    // Clear background
    cr.save()?;
    cr.set_source_rgba(
        config.background_color.r,
        config.background_color.g,
        config.background_color.b,
        config.background_color.a,
    );
    cr.rectangle(0.0, 0.0, width, height);
    cr.fill()?;
    cr.restore()?;

    // Calculate plot area
    let plot_x = config.margin.left;
    let plot_y = config.margin.top;
    let plot_width = width - config.margin.left - config.margin.right;
    let plot_height = height - config.margin.top - config.margin.bottom;

    if plot_width <= 0.0 || plot_height <= 0.0 {
        return Ok(());
    }

    // Draw plot background
    cr.save()?;
    cr.set_source_rgba(
        config.plot_background_color.r,
        config.plot_background_color.g,
        config.plot_background_color.b,
        config.plot_background_color.a,
    );
    cr.rectangle(plot_x, plot_y, plot_width, plot_height);
    cr.fill()?;
    cr.restore()?;

    // Determine value range
    // Priority: 1) auto_scale from data, 2) source limits, 3) config values
    let (min_val, max_val) = if config.auto_scale && !data.is_empty() {
        let data_min = data.iter().map(|p| p.value).fold(f64::INFINITY, f64::min);
        let data_max = data
            .iter()
            .map(|p| p.value)
            .fold(f64::NEG_INFINITY, f64::max);
        let range = data_max - data_min;
        let padding = range * (config.value_padding / 100.0);
        (data_min - padding, data_max + padding)
    } else {
        // Use source limits if available, otherwise fall back to config values
        let source_min = source_values.get("min_limit").and_then(|v| v.as_f64());
        let source_max = source_values.get("max_limit").and_then(|v| v.as_f64());
        (
            source_min.unwrap_or(config.min_value),
            source_max.unwrap_or(config.max_value),
        )
    };

    let value_range = max_val - min_val;
    // Handle case where all values are the same - use a minimum range to avoid division by zero
    // and still draw a horizontal line at the data value
    let (min_val, max_val, value_range) = if value_range == 0.0 || value_range.abs() < 0.001 {
        // Use the data value as center with a range of 10% of its absolute value, or 1.0 if near zero
        let center = if !data.is_empty() { data[0].value } else { 0.0 };
        let half_range = if center.abs() > 0.001 {
            center.abs() * 0.1
        } else {
            0.5
        };
        (center - half_range, center + half_range, half_range * 2.0)
    } else {
        (min_val, max_val, value_range)
    };

    // Draw grid
    if config.y_axis.show_grid {
        cr.save()?;
        cr.set_source_rgba(
            y_axis_grid_color.r,
            y_axis_grid_color.g,
            y_axis_grid_color.b,
            y_axis_grid_color.a,
        );
        cr.set_line_width(config.y_axis.grid_width);
        apply_line_style(cr, config.y_axis.grid_line_style, config.y_axis.grid_width);

        // Draw horizontal grid lines
        let num_lines = 5;
        for i in 0..=num_lines {
            let y = plot_y + (i as f64 / num_lines as f64) * plot_height;
            cr.move_to(plot_x, y);
            cr.line_to(plot_x + plot_width, y);
            cr.stroke()?;
        }
        cr.restore()?;
    }

    if config.x_axis.show_grid && data.len() > 1 {
        cr.save()?;
        cr.set_source_rgba(
            x_axis_grid_color.r,
            x_axis_grid_color.g,
            x_axis_grid_color.b,
            x_axis_grid_color.a,
        );
        cr.set_line_width(config.x_axis.grid_width);
        apply_line_style(cr, config.x_axis.grid_line_style, config.x_axis.grid_width);

        // Draw vertical grid lines (ensure at least 1 to avoid division by zero)
        let num_lines = 5.min(data.len().saturating_sub(1)).max(1);
        for i in 0..=num_lines {
            let x = plot_x + (i as f64 / num_lines as f64) * plot_width;
            cr.move_to(x, plot_y);
            cr.line_to(x, plot_y + plot_height);
            cr.stroke()?;
        }
        cr.restore()?;
    }

    // Draw data (clip to plot area to prevent drawing outside)
    cr.save()?;
    cr.rectangle(plot_x, plot_y, plot_width, plot_height);
    cr.clip();

    if !data.is_empty() {
        // Calculate point spacing and scroll offset in pixels
        let point_spacing = plot_width / (config.max_data_points - 1).max(1) as f64;
        let scroll_pixels = if config.animate_new_points {
            scroll_offset * point_spacing
        } else {
            0.0
        };

        // Use thread-local buffer to avoid allocation per frame
        // Take ownership temporarily, populate, use, then return
        let points = POINTS_BUFFER.with(|buf| {
            let mut points = buf.borrow_mut();
            points.clear();

            // Pre-compute divisor to avoid repeated division
            let max_points_divisor = (config.max_data_points - 1).max(1) as f64;

            for (i, point) in data.iter().enumerate() {
                let base_x = plot_x + (i as f64 / max_points_divisor) * plot_width;
                let x = base_x - scroll_pixels;
                let normalized = ((point.value - min_val) / value_range).clamp(0.0, 1.0);
                let y = plot_y + plot_height - (normalized * plot_height);
                points.push((x, y));
            }

            // Return a clone of the points for use outside the closure
            // This is still faster than allocating fresh each time as the buffer
            // capacity is maintained across frames
            points.clone()
        });

        match config.graph_type {
            GraphType::Line | GraphType::SteppedLine | GraphType::Area => {
                // Draw fill if enabled
                if config.fill_mode != FillMode::None && points.len() > 1 {
                    cr.save()?;

                    // Create path
                    cr.move_to(points[0].0, plot_y + plot_height);
                    for (i, &(x, y)) in points.iter().enumerate() {
                        if config.graph_type == GraphType::SteppedLine && i > 0 {
                            cr.line_to(x, points[i - 1].1);
                        }
                        cr.line_to(x, y);
                    }
                    cr.line_to(points[points.len() - 1].0, plot_y + plot_height);
                    cr.close_path();

                    // Apply fill
                    match config.fill_mode {
                        FillMode::Solid => {
                            cr.set_source_rgba(
                                fill_color.r,
                                fill_color.g,
                                fill_color.b,
                                fill_color.a * config.fill_opacity,
                            );
                            cr.fill()?;
                        }
                        FillMode::Gradient => {
                            let gradient =
                                cairo::LinearGradient::new(0.0, plot_y, 0.0, plot_y + plot_height);
                            gradient.add_color_stop_rgba(
                                0.0,
                                fill_gradient_start.r,
                                fill_gradient_start.g,
                                fill_gradient_start.b,
                                fill_gradient_start.a * config.fill_opacity,
                            );
                            gradient.add_color_stop_rgba(
                                1.0,
                                fill_gradient_end.r,
                                fill_gradient_end.g,
                                fill_gradient_end.b,
                                fill_gradient_end.a * config.fill_opacity,
                            );
                            cr.set_source(&gradient)?;
                            cr.fill()?;
                        }
                        FillMode::None => {}
                    }

                    cr.restore()?;
                }

                // Draw line
                if points.len() > 1 {
                    cr.save()?;
                    cr.set_source_rgba(line_color.r, line_color.g, line_color.b, line_color.a);
                    cr.set_line_width(config.line_width);
                    apply_line_style(cr, config.line_style, config.line_width);
                    cr.set_line_cap(LineCap::Round);
                    cr.set_line_join(LineJoin::Round);

                    cr.move_to(points[0].0, points[0].1);

                    if config.smooth_lines
                        && config.graph_type != GraphType::SteppedLine
                        && points.len() > 2
                    {
                        // Draw smooth Bezier curves
                        for i in 0..points.len() - 1 {
                            let p0 = points[i];
                            let p3 = points[i + 1];

                            // Calculate control points for smooth curve
                            let tension = 0.3; // Adjust this for more/less smoothing

                            // Get surrounding points for better curve calculation
                            let p_prev = if i > 0 { points[i - 1] } else { p0 };
                            let p_next = if i + 2 < points.len() {
                                points[i + 2]
                            } else {
                                p3
                            };

                            // Control point 1 (near p0)
                            let cp1_x = p0.0 + (p3.0 - p_prev.0) * tension;
                            let cp1_y = p0.1 + (p3.1 - p_prev.1) * tension;

                            // Control point 2 (near p3)
                            let cp2_x = p3.0 - (p_next.0 - p0.0) * tension;
                            let cp2_y = p3.1 - (p_next.1 - p0.1) * tension;

                            cr.curve_to(cp1_x, cp1_y, cp2_x, cp2_y, p3.0, p3.1);
                        }
                    } else {
                        // Draw straight lines
                        for (i, &(x, y)) in points.iter().enumerate().skip(1) {
                            if config.graph_type == GraphType::SteppedLine {
                                cr.line_to(x, points[i - 1].1);
                            }
                            cr.line_to(x, y);
                        }
                    }
                    cr.stroke()?;
                    cr.restore()?;
                }

                // Draw points if enabled
                if config.show_points {
                    cr.save()?;
                    cr.set_source_rgba(point_color.r, point_color.g, point_color.b, point_color.a);
                    for &(x, y) in &points {
                        cr.arc(x, y, config.point_radius, 0.0, 2.0 * std::f64::consts::PI);
                        cr.fill()?;
                    }
                    cr.restore()?;
                }
            }
            GraphType::Bar => {
                let bar_width = (plot_width / config.max_data_points as f64) * 0.8;
                cr.save()?;
                cr.set_source_rgba(line_color.r, line_color.g, line_color.b, line_color.a);

                for &(x, y) in &points {
                    let bar_height = plot_y + plot_height - y;
                    cr.rectangle(x - bar_width / 2.0, y, bar_width, bar_height);
                    cr.fill()?;
                }
                cr.restore()?;
            }
        }
    }

    // Restore from clip region
    cr.restore()?;

    // Draw axes
    if config.y_axis.show {
        cr.save()?;
        cr.set_source_rgba(
            y_axis_color.r,
            y_axis_color.g,
            y_axis_color.b,
            y_axis_color.a,
        );
        cr.set_line_width(config.y_axis.width);
        cr.move_to(plot_x, plot_y);
        cr.line_to(plot_x, plot_y + plot_height);
        cr.stroke()?;
        cr.restore()?;

        // Y-axis labels
        if config.y_axis.show_labels {
            cr.save()?;
            cr.set_source_rgba(
                y_axis_label_color.r,
                y_axis_label_color.g,
                y_axis_label_color.b,
                y_axis_label_color.a,
            );
            let font_slant = if config.y_axis.label_italic {
                cairo::FontSlant::Italic
            } else {
                cairo::FontSlant::Normal
            };
            let font_weight = if config.y_axis.label_bold {
                cairo::FontWeight::Bold
            } else {
                cairo::FontWeight::Normal
            };

            let num_labels = 5;
            for i in 0..=num_labels {
                let value = max_val - (i as f64 / num_labels as f64) * value_range;
                // Use adaptive formatting based on value magnitude
                let label = if value.abs() >= 1000.0 {
                    format!("{:.0}", value)
                } else if value.abs() >= 100.0 {
                    format!("{:.0}", value)
                } else if value.abs() >= 10.0 {
                    format!("{:.1}", value)
                } else {
                    format!("{:.1}", value)
                };
                let y = plot_y + (i as f64 / num_labels as f64) * plot_height;

                let extents = pango_text_extents(
                    cr,
                    &label,
                    &config.y_axis.label_font_family,
                    font_slant,
                    font_weight,
                    config.y_axis.label_font_size,
                );
                // Ensure label stays within panel bounds
                let label_x = (plot_x - extents.width() - 5.0).max(2.0);
                cr.move_to(label_x, y + extents.height() / 2.0);
                pango_show_text(
                    cr,
                    &label,
                    &config.y_axis.label_font_family,
                    font_slant,
                    font_weight,
                    config.y_axis.label_font_size,
                );
            }
            cr.restore()?;
        }
    }

    if config.x_axis.show {
        cr.save()?;
        cr.set_source_rgba(
            x_axis_color.r,
            x_axis_color.g,
            x_axis_color.b,
            x_axis_color.a,
        );
        cr.set_line_width(config.x_axis.width);
        cr.move_to(plot_x, plot_y + plot_height);
        cr.line_to(plot_x + plot_width, plot_y + plot_height);
        cr.stroke()?;
        cr.restore()?;
    }

    // Draw text overlay if enabled
    if config.text_overlay.enabled {
        log::debug!(
            "Graph text overlay: {} lines, source_values keys: {:?}",
            config.text_overlay.text_config.lines.len(),
            source_values.keys().collect::<Vec<_>>()
        );
        crate::text_renderer::render_text_lines_with_theme(
            cr,
            width,
            height,
            &config.text_overlay.text_config,
            source_values,
            Some(theme),
        );
    } else {
        log::debug!(
            "Graph text overlay is disabled, source_values keys: {:?}",
            source_values.keys().collect::<Vec<_>>()
        );
    }

    Ok(())
}
