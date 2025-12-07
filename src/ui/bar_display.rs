//! Bar display widget for visualizing values with various styles

use gtk4::cairo;
use serde::{Deserialize, Serialize};

use crate::ui::background::{Color, ColorStop};
use crate::displayers::TextDisplayerConfig;

/// Bar display style
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum BarStyle {
    #[serde(rename = "full")]
    Full,         // Fill entire panel
    #[serde(rename = "rectangle")]
    Rectangle,    // Rectangular bar with rounded corners
    #[serde(rename = "segmented")]
    Segmented,    // Multiple segments with spacing
}

impl Default for BarStyle {
    fn default() -> Self {
        Self::Full
    }
}

/// Bar orientation
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum BarOrientation {
    #[serde(rename = "horizontal")]
    Horizontal,
    #[serde(rename = "vertical")]
    Vertical,
}

impl Default for BarOrientation {
    fn default() -> Self {
        Self::Horizontal
    }
}

/// Bar fill direction
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum BarFillDirection {
    #[serde(rename = "left_to_right")]
    LeftToRight,
    #[serde(rename = "right_to_left")]
    RightToLeft,
    #[serde(rename = "bottom_to_top")]
    BottomToTop,
    #[serde(rename = "top_to_bottom")]
    TopToBottom,
}

impl Default for BarFillDirection {
    fn default() -> Self {
        Self::LeftToRight
    }
}

/// Foreground fill type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum BarFillType {
    #[serde(rename = "solid")]
    Solid { color: Color },
    #[serde(rename = "gradient")]
    Gradient { stops: Vec<ColorStop> },
}

impl Default for BarFillType {
    fn default() -> Self {
        Self::Solid {
            color: Color::new(0.2, 0.6, 1.0, 1.0), // Blue
        }
    }
}

/// Background fill type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum BarBackgroundType {
    #[serde(rename = "solid")]
    Solid { color: Color },
    #[serde(rename = "gradient")]
    Gradient { stops: Vec<ColorStop> },
    #[serde(rename = "transparent")]
    Transparent,
}

impl Default for BarBackgroundType {
    fn default() -> Self {
        Self::Solid {
            color: Color::new(0.15, 0.15, 0.15, 0.8),
        }
    }
}

/// Text overlay configuration - uses full TextDisplayer config
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TextOverlayConfig {
    pub enabled: bool,
    #[serde(default)]
    pub text_config: TextDisplayerConfig,
}

impl Default for TextOverlayConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            text_config: TextDisplayerConfig::default(),
        }
    }
}

/// Border configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BorderConfig {
    pub enabled: bool,
    pub color: Color,
    pub width: f64,
}

impl Default for BorderConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            color: Color::new(0.5, 0.5, 0.5, 1.0),
            width: 1.0,
        }
    }
}

/// Bar display configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BarDisplayConfig {
    pub style: BarStyle,
    pub orientation: BarOrientation,
    pub fill_direction: BarFillDirection,

    pub foreground: BarFillType,
    pub background: BarBackgroundType,

    // Rectangle style options
    #[serde(default = "default_corner_radius")]
    pub corner_radius: f64,
    #[serde(default = "default_padding")]
    pub padding: f64,
    #[serde(default = "default_rectangle_width")]
    pub rectangle_width: f64, // Percentage of panel width (0.0 to 1.0)
    #[serde(default = "default_rectangle_height")]
    pub rectangle_height: f64, // Percentage of panel height (0.0 to 1.0)

    // Segmented style options
    #[serde(default = "default_segment_count")]
    pub segment_count: u32,
    #[serde(default = "default_segment_spacing")]
    pub segment_spacing: f64,
    #[serde(default = "default_segment_width")]
    pub segment_width: f64, // Percentage of panel width (0.0 to 1.0)
    #[serde(default = "default_segment_height")]
    pub segment_height: f64, // Percentage of panel height (0.0 to 1.0)

    // Border
    #[serde(default)]
    pub border: BorderConfig,

    // Text overlay
    #[serde(default)]
    pub text_overlay: TextOverlayConfig,

    // Animation
    #[serde(default = "default_true")]
    pub smooth_animation: bool,
    #[serde(default = "default_animation_speed")]
    pub animation_speed: f64, // 0.0 to 1.0
}

fn default_corner_radius() -> f64 {
    5.0
}

fn default_padding() -> f64 {
    4.0
}

fn default_rectangle_width() -> f64 {
    0.8  // 80% of panel width
}

fn default_rectangle_height() -> f64 {
    0.6  // 60% of panel height
}

fn default_segment_count() -> u32 {
    10
}

fn default_segment_spacing() -> f64 {
    2.0
}

fn default_segment_width() -> f64 {
    0.9  // 90% of panel width
}

fn default_segment_height() -> f64 {
    0.8  // 80% of panel height
}

fn default_true() -> bool {
    true
}

fn default_animation_speed() -> f64 {
    0.5
}

impl Default for BarDisplayConfig {
    fn default() -> Self {
        Self {
            style: BarStyle::default(),
            orientation: BarOrientation::default(),
            fill_direction: BarFillDirection::default(),
            foreground: BarFillType::default(),
            background: BarBackgroundType::default(),
            corner_radius: default_corner_radius(),
            padding: default_padding(),
            rectangle_width: default_rectangle_width(),
            rectangle_height: default_rectangle_height(),
            segment_count: default_segment_count(),
            segment_spacing: default_segment_spacing(),
            segment_width: default_segment_width(),
            segment_height: default_segment_height(),
            border: BorderConfig::default(),
            text_overlay: TextOverlayConfig::default(),
            smooth_animation: default_true(),
            animation_speed: default_animation_speed(),
        }
    }
}

/// Render a bar display
pub fn render_bar(
    cr: &cairo::Context,
    config: &BarDisplayConfig,
    value: f64, // 0.0 to 1.0
    values: &std::collections::HashMap<String, serde_json::Value>, // All source data for text overlay
    width: f64,
    height: f64,
) -> Result<(), cairo::Error> {
    // Clamp value
    let value = value.clamp(0.0, 1.0);

    match config.style {
        BarStyle::Full => render_full_bar(cr, config, value, width, height)?,
        BarStyle::Rectangle => render_rectangle_bar(cr, config, value, width, height)?,
        BarStyle::Segmented => render_segmented_bar(cr, config, value, width, height)?,
    }

    // Render text overlay if enabled
    if config.text_overlay.enabled {
        render_text_overlay(cr, config, value, values, width, height)?;
    }

    Ok(())
}

/// Render full panel style bar
fn render_full_bar(
    cr: &cairo::Context,
    config: &BarDisplayConfig,
    value: f64,
    width: f64,
    height: f64,
) -> Result<(), cairo::Error> {
    // Render background
    render_background(cr, &config.background, width, height)?;

    // Render foreground based on value
    cr.save()?;

    let (fill_width, fill_height, fill_x, fill_y) = match config.fill_direction {
        BarFillDirection::LeftToRight => (width * value, height, 0.0, 0.0),
        BarFillDirection::RightToLeft => (width * value, height, width * (1.0 - value), 0.0),
        BarFillDirection::BottomToTop => (width, height * value, 0.0, height * (1.0 - value)),
        BarFillDirection::TopToBottom => (width, height * value, 0.0, 0.0),
    };

    cr.rectangle(fill_x, fill_y, fill_width, fill_height);
    cr.clip();

    render_foreground(cr, &config.foreground, config.fill_direction, width, height)?;

    cr.restore()?;

    // Render border
    if config.border.enabled {
        render_border(cr, &config.border, 0.0, 0.0, width, height, 0.0)?;
    }

    Ok(())
}

/// Render rectangle style bar
fn render_rectangle_bar(
    cr: &cairo::Context,
    config: &BarDisplayConfig,
    value: f64,
    width: f64,
    height: f64,
) -> Result<(), cairo::Error> {
    let padding = config.padding;
    let radius = config.corner_radius;

    // Calculate bar dimensions based on configured percentages
    let bar_width = width * config.rectangle_width;
    let bar_height = height * config.rectangle_height;

    // Center the bar
    let bar_x = (width - bar_width) / 2.0;
    let bar_y = (height - bar_height) / 2.0;

    // Render background
    cr.save()?;
    rounded_rectangle(cr, bar_x, bar_y, bar_width, bar_height, radius);
    cr.clip();
    render_background(cr, &config.background, width, height)?;
    cr.restore()?;

    // Render foreground based on value
    cr.save()?;

    let (fill_width, fill_height, fill_x, fill_y) = match config.fill_direction {
        BarFillDirection::LeftToRight => (bar_width * value, bar_height, bar_x, bar_y),
        BarFillDirection::RightToLeft => (bar_width * value, bar_height, bar_x + bar_width * (1.0 - value), bar_y),
        BarFillDirection::BottomToTop => (bar_width, bar_height * value, bar_x, bar_y + bar_height * (1.0 - value)),
        BarFillDirection::TopToBottom => (bar_width, bar_height * value, bar_x, bar_y),
    };

    rounded_rectangle(cr, fill_x, fill_y, fill_width, fill_height, radius);
    cr.clip();

    render_foreground(cr, &config.foreground, config.fill_direction, width, height)?;

    cr.restore()?;

    // Render border
    if config.border.enabled {
        rounded_rectangle(cr, bar_x, bar_y, bar_width, bar_height, radius);
        render_border(cr, &config.border, bar_x, bar_y, bar_width, bar_height, radius)?;
    }

    Ok(())
}

/// Render segmented style bar
fn render_segmented_bar(
    cr: &cairo::Context,
    config: &BarDisplayConfig,
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

    let is_horizontal = matches!(config.fill_direction, BarFillDirection::LeftToRight | BarFillDirection::RightToLeft);

    if is_horizontal {
        let total_spacing = spacing * (segment_count - 1) as f64;
        let segment_width = (bar_width - total_spacing) / segment_count as f64;

        for i in 0..segment_count {
            let reverse = matches!(config.fill_direction, BarFillDirection::RightToLeft);
            let seg_index = if reverse { segment_count - 1 - i } else { i };

            let seg_x = bar_x + seg_index as f64 * (segment_width + spacing);
            let seg_y = bar_y;

            let is_filled = if reverse {
                i < filled_segments
            } else {
                seg_index < filled_segments
            };

            render_segment(
                cr,
                config,
                is_filled,
                seg_x,
                seg_y,
                segment_width,
                bar_height,
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

            let seg_x = bar_x;
            let seg_y = bar_y + seg_index as f64 * (segment_height + spacing);

            let is_filled = if reverse {
                i < filled_segments
            } else {
                segment_count - 1 - seg_index < filled_segments
            };

            render_segment(
                cr,
                config,
                is_filled,
                seg_x,
                seg_y,
                bar_width,
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
        render_foreground(cr, &config.foreground, config.fill_direction, full_bar_width, full_bar_height)?;
    } else {
        cr.clip();
        // Translate to align gradient with full bar
        cr.translate(-full_bar_x, -full_bar_y);
        render_background(cr, &config.background, full_bar_width, full_bar_height)?;
    }

    cr.restore()?;

    if config.border.enabled {
        rounded_rectangle(cr, x, y, width, height, radius);
        config.border.color.apply_to_cairo(cr);
        cr.set_line_width(config.border.width);
        cr.stroke()?;
    }

    Ok(())
}

/// Draw a rounded rectangle path
fn rounded_rectangle(cr: &cairo::Context, x: f64, y: f64, width: f64, height: f64, radius: f64) {
    let radius = radius.min(width / 2.0).min(height / 2.0);

    cr.new_path();
    cr.arc(x + radius, y + radius, radius, std::f64::consts::PI, 3.0 * std::f64::consts::PI / 2.0);
    cr.arc(x + width - radius, y + radius, radius, 3.0 * std::f64::consts::PI / 2.0, 0.0);
    cr.arc(x + width - radius, y + height - radius, radius, 0.0, std::f64::consts::PI / 2.0);
    cr.arc(x + radius, y + height - radius, radius, std::f64::consts::PI / 2.0, std::f64::consts::PI);
    cr.close_path();
}

/// Render background
fn render_background(
    cr: &cairo::Context,
    background: &BarBackgroundType,
    width: f64,
    height: f64,
) -> Result<(), cairo::Error> {
    match background {
        BarBackgroundType::Solid { color } => {
            color.apply_to_cairo(cr);
            cr.paint()?;
        }
        BarBackgroundType::Gradient { stops } => {
            render_gradient(cr, stops, width, height)?;
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
    direction: BarFillDirection,
    width: f64,
    height: f64,
) -> Result<(), cairo::Error> {
    match foreground {
        BarFillType::Solid { color } => {
            color.apply_to_cairo(cr);
            cr.paint()?;
        }
        BarFillType::Gradient { stops } => {
            render_gradient(cr, stops, width, height)?;
        }
    }
    Ok(())
}

/// Render a gradient
fn render_gradient(
    cr: &cairo::Context,
    stops: &[ColorStop],
    width: f64,
    height: f64,
) -> Result<(), cairo::Error> {
    if stops.is_empty() {
        return Ok(());
    }

    // Horizontal gradient
    let pattern = cairo::LinearGradient::new(0.0, 0.0, width, 0.0);

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

/// Render border
fn render_border(
    cr: &cairo::Context,
    border: &BorderConfig,
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

    border.color.apply_to_cairo(cr);
    cr.set_line_width(border.width);
    cr.stroke()?;

    Ok(())
}

/// Render text overlay using shared text renderer
fn render_text_overlay(
    cr: &cairo::Context,
    config: &BarDisplayConfig,
    _value: f64,
    values: &std::collections::HashMap<String, serde_json::Value>,
    width: f64,
    height: f64,
) -> Result<(), cairo::Error> {
    // Use shared text renderer for proper combined field handling
    crate::ui::text_renderer::render_text_lines(
        cr,
        width,
        height,
        &config.text_overlay.text_config,
        values,
    );

    Ok(())
}
