//! Core bars display - renders multiple CPU core usage bars
//!
//! This module provides a standalone rendering function for displaying
//! multiple CPU core usage values as animated bars. Designed to be reusable
//! by combo displays like LCARS.

use gtk4::cairo;
use serde::{Deserialize, Serialize};

use crate::ui::background::{Color, ColorStop};
use crate::ui::bar_display::{BarBackgroundType, BarFillDirection, BarFillType, BarOrientation, BarStyle, BorderConfig};

/// Label position relative to the bar
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum LabelPosition {
    #[serde(rename = "start")]
    #[default]
    Start,  // Left for horizontal, Top for vertical
    #[serde(rename = "end")]
    End,    // Right for horizontal, Bottom for vertical
    #[serde(rename = "inside")]
    Inside, // Inside the bar
}

/// Core bars display configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CoreBarsConfig {
    // Core selection
    #[serde(default)]
    pub start_core: usize,
    #[serde(default = "default_end_core")]
    pub end_core: usize,

    // Padding
    #[serde(default)]
    pub padding_top: f64,
    #[serde(default)]
    pub padding_bottom: f64,
    #[serde(default)]
    pub padding_left: f64,
    #[serde(default)]
    pub padding_right: f64,

    // Bar styling (unified for all bars)
    #[serde(default)]
    pub bar_style: BarStyle,
    #[serde(default)]
    pub orientation: BarOrientation,
    #[serde(default)]
    pub fill_direction: BarFillDirection,
    #[serde(default)]
    pub foreground: BarFillType,
    #[serde(default)]
    pub background: BarBackgroundType,
    #[serde(default = "default_corner_radius")]
    pub corner_radius: f64,
    #[serde(default = "default_bar_spacing")]
    pub bar_spacing: f64,

    // Segmented bar options
    #[serde(default = "default_segment_count")]
    pub segment_count: u32,
    #[serde(default = "default_segment_spacing")]
    pub segment_spacing: f64,

    // Border
    #[serde(default)]
    pub border: BorderConfig,

    // Labels
    #[serde(default = "default_true")]
    pub show_labels: bool,
    #[serde(default = "default_label_prefix")]
    pub label_prefix: String,
    #[serde(default)]
    pub label_position: LabelPosition,
    #[serde(default = "default_label_font")]
    pub label_font: String,
    #[serde(default = "default_label_size")]
    pub label_size: f64,
    #[serde(default = "default_label_color")]
    pub label_color: Color,
    #[serde(default)]
    pub label_bold: bool,

    // Animation
    #[serde(default = "default_true")]
    pub animate: bool,
    #[serde(default = "default_animation_speed")]
    pub animation_speed: f64,
}

fn default_end_core() -> usize {
    15 // Default to 16 cores (0-15)
}

fn default_corner_radius() -> f64 {
    3.0
}

fn default_bar_spacing() -> f64 {
    4.0
}

fn default_segment_count() -> u32 {
    10
}

fn default_segment_spacing() -> f64 {
    1.0
}

fn default_true() -> bool {
    true
}

fn default_label_prefix() -> String {
    "".to_string()
}

fn default_label_font() -> String {
    "Sans".to_string()
}

fn default_label_size() -> f64 {
    10.0
}

fn default_label_color() -> Color {
    Color::new(1.0, 1.0, 1.0, 0.9)
}

fn default_animation_speed() -> f64 {
    8.0
}

impl Default for CoreBarsConfig {
    fn default() -> Self {
        Self {
            start_core: 0,
            end_core: default_end_core(),
            padding_top: 0.0,
            padding_bottom: 0.0,
            padding_left: 0.0,
            padding_right: 0.0,
            bar_style: BarStyle::default(),
            orientation: BarOrientation::default(),
            fill_direction: BarFillDirection::default(),
            foreground: BarFillType::default(),
            background: BarBackgroundType::default(),
            corner_radius: default_corner_radius(),
            bar_spacing: default_bar_spacing(),
            segment_count: default_segment_count(),
            segment_spacing: default_segment_spacing(),
            border: BorderConfig::default(),
            show_labels: default_true(),
            label_prefix: default_label_prefix(),
            label_position: LabelPosition::default(),
            label_font: default_label_font(),
            label_size: default_label_size(),
            label_color: default_label_color(),
            label_bold: false,
            animate: default_true(),
            animation_speed: default_animation_speed(),
        }
    }
}

impl CoreBarsConfig {
    /// Get the number of cores to display based on config
    pub fn core_count(&self) -> usize {
        if self.end_core >= self.start_core {
            self.end_core - self.start_core + 1
        } else {
            0
        }
    }
}

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
    core_values: &[f64],
    width: f64,
    height: f64,
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

    // Calculate label space if labels are shown
    let label_space = if config.show_labels {
        calculate_label_space(cr, config, num_bars)
    } else {
        0.0
    };

    // For horizontal bars (stacked vertically), we divide height
    // For vertical bars (arranged horizontally), we divide width
    match config.orientation {
        BarOrientation::Horizontal => {
            render_horizontal_bars(cr, config, core_values, padded_width, padded_height, label_space)?;
        }
        BarOrientation::Vertical => {
            render_vertical_bars(cr, config, core_values, padded_width, padded_height, label_space)?;
        }
    }

    cr.restore()?;
    Ok(())
}

/// Calculate space needed for labels
fn calculate_label_space(cr: &cairo::Context, config: &CoreBarsConfig, num_bars: usize) -> f64 {
    let font_weight = if config.label_bold {
        cairo::FontWeight::Bold
    } else {
        cairo::FontWeight::Normal
    };

    cr.select_font_face(&config.label_font, cairo::FontSlant::Normal, font_weight);
    cr.set_font_size(config.label_size);

    // Calculate max label width/height
    let max_index = config.start_core + num_bars - 1;
    let test_label = format!("{}{}", config.label_prefix, max_index);

    if let Ok(extents) = cr.text_extents(&test_label) {
        match config.orientation {
            BarOrientation::Horizontal => extents.width() + 8.0,
            BarOrientation::Vertical => extents.height() + 8.0,
        }
    } else {
        config.label_size + 8.0
    }
}

/// Render horizontal bars stacked vertically
fn render_horizontal_bars(
    cr: &cairo::Context,
    config: &CoreBarsConfig,
    core_values: &[f64],
    width: f64,
    height: f64,
    label_space: f64,
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
            cr,
            config,
            value,
            bar_x,
            bar_y,
            bar_width,
            bar_height,
            true, // horizontal
        )?;
        cr.restore()?;

        // Render label if enabled
        if config.show_labels {
            render_label(
                cr,
                config,
                config.start_core + i,
                bar_y,
                bar_height,
                width,
                height,
                label_space,
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
    core_values: &[f64],
    width: f64,
    height: f64,
    label_space: f64,
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
            cr,
            config,
            value,
            bar_x,
            bar_y,
            bar_width,
            bar_height,
            false, // vertical
        )?;
        cr.restore()?;

        // Render label if enabled
        if config.show_labels {
            render_label(
                cr,
                config,
                config.start_core + i,
                bar_x,
                bar_width,
                width,
                height,
                label_space,
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
    value: f64,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    horizontal: bool,
) -> Result<(), cairo::Error> {
    let radius = config.corner_radius.min(width / 2.0).min(height / 2.0);

    match config.bar_style {
        BarStyle::Full | BarStyle::Rectangle => {
            // Render background
            cr.save()?;
            rounded_rectangle(cr, x, y, width, height, radius);
            cr.clip();
            render_background(cr, &config.background, x, y, width, height)?;
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
                render_foreground(cr, &config.foreground, x, y, width, height)?;
            }
            cr.restore()?;

            // Render border
            if config.border.enabled {
                rounded_rectangle(cr, x, y, width, height, radius);
                config.border.color.apply_to_cairo(cr);
                cr.set_line_width(config.border.width);
                cr.stroke()?;
            }
        }
        BarStyle::Segmented => {
            render_segmented_single_bar(cr, config, value, x, y, width, height, horizontal)?;
        }
    }

    Ok(())
}

/// Render a segmented bar
fn render_segmented_single_bar(
    cr: &cairo::Context,
    config: &CoreBarsConfig,
    value: f64,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    horizontal: bool,
) -> Result<(), cairo::Error> {
    let segment_count = config.segment_count.max(1);
    let spacing = config.segment_spacing;
    let filled_segments = (value * segment_count as f64).ceil() as u32;

    if horizontal {
        let total_spacing = spacing * (segment_count - 1) as f64;
        let segment_width = (width - total_spacing) / segment_count as f64;
        let radius = config.corner_radius.min(segment_width / 2.0).min(height / 2.0);

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

            render_segment(
                cr,
                config,
                is_filled,
                seg_x,
                y,
                segment_width,
                height,
                radius,
                x,
                y,
                width,
                height,
            )?;
        }
    } else {
        let total_spacing = spacing * (segment_count - 1) as f64;
        let segment_height = (height - total_spacing) / segment_count as f64;
        let radius = config.corner_radius.min(width / 2.0).min(segment_height / 2.0);

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

            render_segment(
                cr,
                config,
                is_filled,
                x,
                seg_y,
                width,
                segment_height,
                radius,
                x,
                y,
                width,
                height,
            )?;
        }
    }

    Ok(())
}

/// Render a single segment
fn render_segment(
    cr: &cairo::Context,
    config: &CoreBarsConfig,
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
) -> Result<(), cairo::Error> {
    cr.save()?;
    rounded_rectangle(cr, seg_x, seg_y, seg_width, seg_height, radius);

    if is_filled {
        cr.clip();
        render_foreground(cr, &config.foreground, full_x, full_y, full_width, full_height)?;
    } else {
        cr.clip();
        render_background(cr, &config.background, full_x, full_y, full_width, full_height)?;
    }

    cr.restore()?;

    if config.border.enabled {
        rounded_rectangle(cr, seg_x, seg_y, seg_width, seg_height, radius);
        config.border.color.apply_to_cairo(cr);
        cr.set_line_width(config.border.width);
        cr.stroke()?;
    }

    Ok(())
}

/// Render a label for a core
fn render_label(
    cr: &cairo::Context,
    config: &CoreBarsConfig,
    core_index: usize,
    bar_pos: f64,     // y for horizontal, x for vertical
    bar_size: f64,    // height for horizontal, width for vertical
    _width: f64,
    height: f64,
    label_space: f64,
    horizontal: bool,
) -> Result<(), cairo::Error> {
    let label = format!("{}{}", config.label_prefix, core_index);

    let font_weight = if config.label_bold {
        cairo::FontWeight::Bold
    } else {
        cairo::FontWeight::Normal
    };

    cr.select_font_face(&config.label_font, cairo::FontSlant::Normal, font_weight);
    cr.set_font_size(config.label_size);
    config.label_color.apply_to_cairo(cr);

    let (text_width, text_height) = if let Ok(extents) = cr.text_extents(&label) {
        (extents.width(), extents.height())
    } else {
        (config.label_size, config.label_size)
    };

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

    cr.show_text(&label)?;

    Ok(())
}

/// Draw a rounded rectangle path
fn rounded_rectangle(cr: &cairo::Context, x: f64, y: f64, width: f64, height: f64, radius: f64) {
    if width <= 0.0 || height <= 0.0 {
        return;
    }

    let radius = radius.min(width / 2.0).min(height / 2.0).max(0.0);

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

/// Render background
fn render_background(
    cr: &cairo::Context,
    background: &BarBackgroundType,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), cairo::Error> {
    match background {
        BarBackgroundType::Solid { color } => {
            color.apply_to_cairo(cr);
            cr.paint()?;
        }
        BarBackgroundType::Gradient { stops, angle } => {
            render_gradient(cr, stops, *angle, x, y, width, height)?;
        }
        BarBackgroundType::Transparent => {
            // Do nothing
        }
    }
    Ok(())
}

/// Render foreground
fn render_foreground(
    cr: &cairo::Context,
    foreground: &BarFillType,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), cairo::Error> {
    match foreground {
        BarFillType::Solid { color } => {
            color.apply_to_cairo(cr);
            cr.paint()?;
        }
        BarFillType::Gradient { stops, angle } => {
            render_gradient(cr, stops, *angle, x, y, width, height)?;
        }
    }
    Ok(())
}

/// Render a gradient
fn render_gradient(
    cr: &cairo::Context,
    stops: &[ColorStop],
    angle: f64,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), cairo::Error> {
    if stops.is_empty() || width <= 0.0 || height <= 0.0 {
        return Ok(());
    }

    let angle_rad = angle.to_radians();
    let diagonal = (width * width + height * height).sqrt();

    let cx = x + width / 2.0;
    let cy = y + height / 2.0;

    let x1 = cx - diagonal * angle_rad.cos() / 2.0;
    let y1 = cy - diagonal * angle_rad.sin() / 2.0;
    let x2 = cx + diagonal * angle_rad.cos() / 2.0;
    let y2 = cy + diagonal * angle_rad.sin() / 2.0;

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
    cr.paint()?;

    Ok(())
}
