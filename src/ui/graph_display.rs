//! Graph display rendering module

use anyhow::Result;
use cairo::{Context, LineCap, LineJoin};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

use super::background::Color;
use crate::displayers::TextLineConfig;

/// Graph type
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum GraphType {
    Line,
    Bar,
    Area,
    SteppedLine,
}

impl Default for GraphType {
    fn default() -> Self {
        Self::Line
    }
}

/// Graph line style
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum LineStyle {
    Solid,
    Dashed,
    Dotted,
}

impl Default for LineStyle {
    fn default() -> Self {
        Self::Solid
    }
}

/// Graph fill mode
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum FillMode {
    None,
    Solid,
    Gradient,
}

impl Default for FillMode {
    fn default() -> Self {
        Self::None
    }
}

/// Axis configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AxisConfig {
    pub show: bool,
    pub color: Color,
    pub width: f64,
    pub show_labels: bool,
    pub label_color: Color,
    pub label_font_size: f64,
    pub show_grid: bool,
    pub grid_color: Color,
    pub grid_width: f64,
    pub grid_line_style: LineStyle,
}

impl Default for AxisConfig {
    fn default() -> Self {
        Self {
            show: true,
            color: Color { r: 0.7, g: 0.7, b: 0.7, a: 1.0 },
            width: 1.0,
            show_labels: true,
            label_color: Color { r: 0.8, g: 0.8, b: 0.8, a: 1.0 },
            label_font_size: 10.0,
            show_grid: true,
            grid_color: Color { r: 0.3, g: 0.3, b: 0.3, a: 0.5 },
            grid_width: 0.5,
            grid_line_style: LineStyle::Dotted,
        }
    }
}

/// Graph margin/padding
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Margin {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}

impl Default for Margin {
    fn default() -> Self {
        Self {
            top: 10.0,
            right: 10.0,
            bottom: 30.0,
            left: 50.0,
        }
    }
}

/// Graph display configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphDisplayConfig {
    // Graph type and style
    pub graph_type: GraphType,
    pub line_style: LineStyle,
    pub line_width: f64,
    pub line_color: Color,

    // Fill configuration
    pub fill_mode: FillMode,
    pub fill_color: Color,
    pub fill_gradient_start: Color,
    pub fill_gradient_end: Color,
    pub fill_opacity: f64,

    // Data points
    pub max_data_points: usize,
    pub point_radius: f64,
    pub show_points: bool,
    pub point_color: Color,

    // Value range
    pub auto_scale: bool,
    pub min_value: f64,
    pub max_value: f64,
    pub value_padding: f64, // Percentage padding when auto-scaling

    // Axes
    pub x_axis: AxisConfig,
    pub y_axis: AxisConfig,

    // Graph area
    pub margin: Margin,
    pub background_color: Color,
    pub plot_background_color: Color,

    // Animation/smoothing
    pub smooth_lines: bool,
    pub animate_new_points: bool,

    // Text overlay
    pub text_overlay: Vec<TextLineConfig>,
}

impl Default for GraphDisplayConfig {
    fn default() -> Self {
        Self {
            graph_type: GraphType::Line,
            line_style: LineStyle::Solid,
            line_width: 2.0,
            line_color: Color { r: 0.2, g: 0.8, b: 0.4, a: 1.0 },

            fill_mode: FillMode::Gradient,
            fill_color: Color { r: 0.2, g: 0.8, b: 0.4, a: 0.3 },
            fill_gradient_start: Color { r: 0.2, g: 0.8, b: 0.4, a: 0.6 },
            fill_gradient_end: Color { r: 0.2, g: 0.8, b: 0.4, a: 0.0 },
            fill_opacity: 0.3,

            max_data_points: 60,
            point_radius: 3.0,
            show_points: false,
            point_color: Color { r: 0.2, g: 0.8, b: 0.4, a: 1.0 },

            auto_scale: true,
            min_value: 0.0,
            max_value: 100.0,
            value_padding: 10.0,

            x_axis: AxisConfig::default(),
            y_axis: AxisConfig::default(),

            margin: Margin::default(),
            background_color: Color { r: 0.0, g: 0.0, b: 0.0, a: 0.0 },
            plot_background_color: Color { r: 0.1, g: 0.1, b: 0.1, a: 0.5 },

            smooth_lines: true,
            animate_new_points: false,

            text_overlay: Vec::new(),
        }
    }
}

/// Graph data point
#[derive(Debug, Clone, Copy)]
pub struct DataPoint {
    pub value: f64,
    pub timestamp: f64, // Relative time in seconds
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
pub fn render_graph(
    cr: &Context,
    config: &GraphDisplayConfig,
    data: &VecDeque<DataPoint>,
    source_values: &std::collections::HashMap<String, serde_json::Value>,
    width: f64,
    height: f64,
) -> Result<()> {
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
    let (min_val, max_val) = if config.auto_scale && !data.is_empty() {
        let data_min = data.iter().map(|p| p.value).fold(f64::INFINITY, f64::min);
        let data_max = data.iter().map(|p| p.value).fold(f64::NEG_INFINITY, f64::max);
        let range = data_max - data_min;
        let padding = range * (config.value_padding / 100.0);
        (data_min - padding, data_max + padding)
    } else {
        (config.min_value, config.max_value)
    };

    let value_range = max_val - min_val;
    if value_range == 0.0 {
        return Ok(());
    }

    // Draw grid
    if config.y_axis.show_grid {
        cr.save()?;
        cr.set_source_rgba(
            config.y_axis.grid_color.r,
            config.y_axis.grid_color.g,
            config.y_axis.grid_color.b,
            config.y_axis.grid_color.a,
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
            config.x_axis.grid_color.r,
            config.x_axis.grid_color.g,
            config.x_axis.grid_color.b,
            config.x_axis.grid_color.a,
        );
        cr.set_line_width(config.x_axis.grid_width);
        apply_line_style(cr, config.x_axis.grid_line_style, config.x_axis.grid_width);

        // Draw vertical grid lines
        let num_lines = 5.min(data.len() - 1);
        for i in 0..=num_lines {
            let x = plot_x + (i as f64 / num_lines as f64) * plot_width;
            cr.move_to(x, plot_y);
            cr.line_to(x, plot_y + plot_height);
            cr.stroke()?;
        }
        cr.restore()?;
    }

    // Draw data
    if !data.is_empty() {
        let points: Vec<(f64, f64)> = data
            .iter()
            .enumerate()
            .map(|(i, point)| {
                let x = plot_x + (i as f64 / (config.max_data_points - 1).max(1) as f64) * plot_width;
                let normalized = ((point.value - min_val) / value_range).clamp(0.0, 1.0);
                let y = plot_y + plot_height - (normalized * plot_height);
                (x, y)
            })
            .collect();

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
                                config.fill_color.r,
                                config.fill_color.g,
                                config.fill_color.b,
                                config.fill_color.a * config.fill_opacity,
                            );
                            cr.fill()?;
                        }
                        FillMode::Gradient => {
                            let gradient = cairo::LinearGradient::new(
                                0.0,
                                plot_y,
                                0.0,
                                plot_y + plot_height,
                            );
                            gradient.add_color_stop_rgba(
                                0.0,
                                config.fill_gradient_start.r,
                                config.fill_gradient_start.g,
                                config.fill_gradient_start.b,
                                config.fill_gradient_start.a * config.fill_opacity,
                            );
                            gradient.add_color_stop_rgba(
                                1.0,
                                config.fill_gradient_end.r,
                                config.fill_gradient_end.g,
                                config.fill_gradient_end.b,
                                config.fill_gradient_end.a * config.fill_opacity,
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
                    cr.set_source_rgba(
                        config.line_color.r,
                        config.line_color.g,
                        config.line_color.b,
                        config.line_color.a,
                    );
                    cr.set_line_width(config.line_width);
                    apply_line_style(cr, config.line_style, config.line_width);
                    cr.set_line_cap(LineCap::Round);
                    cr.set_line_join(LineJoin::Round);

                    cr.move_to(points[0].0, points[0].1);

                    if config.smooth_lines && config.graph_type != GraphType::SteppedLine && points.len() > 2 {
                        // Draw smooth Bezier curves
                        for i in 0..points.len() - 1 {
                            let p0 = points[i];
                            let p3 = points[i + 1];

                            // Calculate control points for smooth curve
                            let tension = 0.3; // Adjust this for more/less smoothing

                            // Get surrounding points for better curve calculation
                            let p_prev = if i > 0 { points[i - 1] } else { p0 };
                            let p_next = if i + 2 < points.len() { points[i + 2] } else { p3 };

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
                    cr.set_source_rgba(
                        config.point_color.r,
                        config.point_color.g,
                        config.point_color.b,
                        config.point_color.a,
                    );
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
                cr.set_source_rgba(
                    config.line_color.r,
                    config.line_color.g,
                    config.line_color.b,
                    config.line_color.a,
                );

                for &(x, y) in &points {
                    let bar_height = plot_y + plot_height - y;
                    cr.rectangle(x - bar_width / 2.0, y, bar_width, bar_height);
                    cr.fill()?;
                }
                cr.restore()?;
            }
        }
    }

    // Draw axes
    if config.y_axis.show {
        cr.save()?;
        cr.set_source_rgba(
            config.y_axis.color.r,
            config.y_axis.color.g,
            config.y_axis.color.b,
            config.y_axis.color.a,
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
                config.y_axis.label_color.r,
                config.y_axis.label_color.g,
                config.y_axis.label_color.b,
                config.y_axis.label_color.a,
            );
            cr.select_font_face("Sans", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
            cr.set_font_size(config.y_axis.label_font_size);

            let num_labels = 5;
            for i in 0..=num_labels {
                let value = max_val - (i as f64 / num_labels as f64) * value_range;
                let label = format!("{:.1}", value);
                let y = plot_y + (i as f64 / num_labels as f64) * plot_height;

                let extents = cr.text_extents(&label)?;
                cr.move_to(plot_x - extents.width() - 5.0, y + extents.height() / 2.0);
                cr.show_text(&label)?;
            }
            cr.restore()?;
        }
    }

    if config.x_axis.show {
        cr.save()?;
        cr.set_source_rgba(
            config.x_axis.color.r,
            config.x_axis.color.g,
            config.x_axis.color.b,
            config.x_axis.color.a,
        );
        cr.set_line_width(config.x_axis.width);
        cr.move_to(plot_x, plot_y + plot_height);
        cr.line_to(plot_x + plot_width, plot_y + plot_height);
        cr.stroke()?;
        cr.restore()?;
    }

    // Draw text overlay
    if !config.text_overlay.is_empty() {
        let text_config = crate::displayers::TextDisplayerConfig {
            lines: config.text_overlay.clone(),
        };
        crate::ui::text_renderer::render_text_lines(
            cr,
            width,
            height,
            &text_config,
            source_values,
        );
    }

    Ok(())
}
