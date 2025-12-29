//! Speedometer gauge display for visualizing values like traditional gauges

use gtk4::cairo;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::ui::background::{Color, ColorStop};
use crate::ui::theme::{deserialize_color_or_source, ColorSource, ColorStopSource, ComboThemeConfig};
use crate::displayers::TextDisplayerConfig;

/// Needle style
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[derive(Default)]
pub enum NeedleStyle {
    #[serde(rename = "arrow")]
    #[default]
    Arrow,        // Traditional pointed arrow
    #[serde(rename = "line")]
    Line,         // Simple line
    #[serde(rename = "tapered")]
    Tapered,      // Tapered line (wider at base)
    #[serde(rename = "triangle")]
    Triangle,     // Solid triangle
}


/// Needle tail style
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[derive(Default)]
pub enum NeedleTailStyle {
    #[serde(rename = "none")]
    None,         // No tail
    #[serde(rename = "short")]
    #[default]
    Short,        // Short tail opposite direction
    #[serde(rename = "balanced")]
    Balanced,     // Balanced tail for counterweight
}


/// Tick mark style
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[derive(Default)]
pub enum TickStyle {
    #[serde(rename = "line")]
    #[default]
    Line,         // Simple lines
    #[serde(rename = "wedge")]
    Wedge,        // Wedge-shaped ticks
    #[serde(rename = "dot")]
    Dot,          // Dots at tick positions
}


/// Bezel style
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[derive(Default)]
pub enum BezelStyle {
    #[serde(rename = "none")]
    None,         // No bezel
    #[serde(rename = "simple")]
    #[default]
    Simple,       // Simple ring
    #[serde(rename = "3d")]
    ThreeD,       // 3D effect with gradient
    #[serde(rename = "chrome")]
    Chrome,       // Metallic chrome effect
}


/// Tick label configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TickLabelConfig {
    pub font_family: String,
    pub font_size: f64,
    #[serde(deserialize_with = "deserialize_color_or_source")]
    pub color: ColorSource,
    pub bold: bool,
    pub italic: bool,
    pub use_percentage: bool, // Show as 0-100% instead of actual values
}

impl Default for TickLabelConfig {
    fn default() -> Self {
        Self {
            font_family: "Sans".to_string(),
            font_size: 12.0,
            color: ColorSource::Custom { color: Color { r: 0.9, g: 0.9, b: 0.9, a: 1.0 } },
            bold: false,
            italic: false,
            use_percentage: false,
        }
    }
}

/// Speedometer configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpeedometerConfig {
    // Arc geometry
    #[serde(default = "default_start_angle")]
    pub start_angle: f64,  // Degrees (0 = right, 90 = down, 180 = left, 270 = up)
    #[serde(default = "default_end_angle")]
    pub end_angle: f64,    // Degrees
    #[serde(default = "default_arc_width")]
    pub arc_width: f64,    // Percentage of radius (0.0 to 1.0)
    #[serde(default = "default_radius_percent")]
    pub radius_percent: f64, // Percentage of available space (0.0 to 1.0)

    // Arc/Track display
    #[serde(default = "default_true")]
    pub show_track: bool,
    #[serde(default = "default_track_color_source")]
    pub track_color: ColorSource,
    #[serde(default = "default_color_stops")]
    pub track_color_stops: Vec<ColorStopSource>, // Color zones for track (theme-aware)

    // Ticks
    #[serde(default = "default_true")]
    pub show_major_ticks: bool,
    #[serde(default = "default_major_tick_count")]
    pub major_tick_count: u32,
    #[serde(default = "default_major_tick_length")]
    pub major_tick_length: f64, // Percentage of arc width
    #[serde(default = "default_major_tick_width")]
    pub major_tick_width: f64,
    #[serde(default = "default_tick_color_source")]
    pub major_tick_color: ColorSource,
    #[serde(default)]
    pub major_tick_style: TickStyle,

    #[serde(default = "default_true")]
    pub show_minor_ticks: bool,
    #[serde(default = "default_minor_tick_count")]
    pub minor_ticks_per_major: u32,
    #[serde(default = "default_minor_tick_length")]
    pub minor_tick_length: f64,
    #[serde(default = "default_minor_tick_width")]
    pub minor_tick_width: f64,
    #[serde(default = "default_tick_color_source")]
    pub minor_tick_color: ColorSource,
    #[serde(default)]
    pub minor_tick_style: TickStyle,

    // Tick labels (10, 20, 30, etc) - configurable text display
    #[serde(default = "default_true")]
    pub show_tick_labels: bool,
    #[serde(default)]
    pub tick_label_config: TickLabelConfig,

    // Needle
    #[serde(default = "default_true")]
    pub show_needle: bool,
    #[serde(default)]
    pub needle_style: NeedleStyle,
    #[serde(default)]
    pub needle_tail_style: NeedleTailStyle,
    #[serde(default = "default_needle_length")]
    pub needle_length: f64, // Percentage of radius
    #[serde(default = "default_needle_width")]
    pub needle_width: f64,
    #[serde(default = "default_needle_color_source")]
    pub needle_color: ColorSource,
    #[serde(default = "default_false")]
    pub needle_shadow: bool,

    // Center hub (pivot point)
    #[serde(default = "default_true")]
    pub show_center_hub: bool,
    #[serde(default = "default_hub_radius")]
    pub center_hub_radius: f64,
    #[serde(default = "default_hub_color_source")]
    pub center_hub_color: ColorSource,
    #[serde(default = "default_false")]
    pub center_hub_3d: bool, // Add 3D highlight effect

    // Bezel (uses full background configuration)
    #[serde(default = "default_true")]
    pub show_bezel: bool,
    #[serde(default = "default_bezel_width")]
    pub bezel_width: f64, // 0.0 to 1.0 (percentage of radius)
    #[serde(default)]
    pub bezel_background: crate::ui::BackgroundConfig,
    /// Theme-aware solid color for bezel (used when bezel_background is solid)
    #[serde(default = "default_bezel_solid_color", deserialize_with = "deserialize_color_or_source")]
    pub bezel_solid_color: ColorSource,

    // Value zones/regions (danger zone, warning zone, etc.)
    #[serde(default)]
    pub zones: Vec<ValueZone>,

    // Animation
    #[serde(default = "default_true")]
    pub animate: bool,
    #[serde(default = "default_animation_duration")]
    pub animation_duration: f64, // Duration in seconds
    #[serde(default = "default_false")]
    pub bounce_animation: bool, // Bounce at end of animation

    // Text overlay
    #[serde(default)]
    pub text_overlay: TextOverlayConfig,
}

/// Value zone configuration (colored regions on gauge)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ValueZone {
    pub start_percent: f64,  // 0.0 to 1.0
    pub end_percent: f64,    // 0.0 to 1.0
    pub color: Color,
    #[serde(default = "default_zone_alpha")]
    pub alpha: f64,          // Transparency
}

fn default_zone_alpha() -> f64 {
    0.3
}

/// Text overlay configuration
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

// Default values
fn default_start_angle() -> f64 {
    135.0 // Bottom-left
}

fn default_end_angle() -> f64 {
    45.0 // Bottom-right (clockwise)
}

fn default_arc_width() -> f64 {
    0.15
}

fn default_radius_percent() -> f64 {
    0.85
}

fn default_true() -> bool {
    true
}

fn default_false() -> bool {
    false
}

fn default_track_color_source() -> ColorSource {
    ColorSource::Custom { color: Color { r: 0.2, g: 0.2, b: 0.2, a: 1.0 } }
}

fn default_color_stops() -> Vec<ColorStopSource> {
    vec![
        ColorStopSource::custom(0.0, Color { r: 0.0, g: 0.8, b: 0.0, a: 1.0 }),
        ColorStopSource::custom(0.7, Color { r: 1.0, g: 0.8, b: 0.0, a: 1.0 }),
        ColorStopSource::custom(0.9, Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 }),
    ]
}

fn default_major_tick_count() -> u32 {
    10
}

fn default_major_tick_length() -> f64 {
    0.15
}

fn default_major_tick_width() -> f64 {
    2.0
}

fn default_minor_tick_count() -> u32 {
    5
}

fn default_minor_tick_length() -> f64 {
    0.08
}

fn default_minor_tick_width() -> f64 {
    1.0
}

fn default_tick_color_source() -> ColorSource {
    ColorSource::Custom { color: Color { r: 0.9, g: 0.9, b: 0.9, a: 1.0 } }
}

fn default_needle_length() -> f64 {
    0.75
}

fn default_needle_width() -> f64 {
    3.0
}

fn default_needle_color_source() -> ColorSource {
    ColorSource::Custom { color: Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 } }
}

fn default_hub_radius() -> f64 {
    0.08
}

fn default_hub_color_source() -> ColorSource {
    ColorSource::Custom { color: Color { r: 0.3, g: 0.3, b: 0.3, a: 1.0 } }
}

fn default_bezel_width() -> f64 {
    0.05
}

fn default_bezel_solid_color() -> ColorSource {
    ColorSource::Custom { color: Color::new(0.3, 0.3, 0.3, 1.0) }
}

fn default_animation_duration() -> f64 {
    0.3
}

impl Default for SpeedometerConfig {
    fn default() -> Self {
        Self {
            start_angle: default_start_angle(),
            end_angle: default_end_angle(),
            arc_width: default_arc_width(),
            radius_percent: default_radius_percent(),
            show_track: default_true(),
            track_color: default_track_color_source(),
            track_color_stops: default_color_stops(),
            show_major_ticks: default_true(),
            major_tick_count: default_major_tick_count(),
            major_tick_length: default_major_tick_length(),
            major_tick_width: default_major_tick_width(),
            major_tick_color: default_tick_color_source(),
            major_tick_style: TickStyle::default(),
            show_minor_ticks: default_true(),
            minor_ticks_per_major: default_minor_tick_count(),
            minor_tick_length: default_minor_tick_length(),
            minor_tick_width: default_minor_tick_width(),
            minor_tick_color: default_tick_color_source(),
            minor_tick_style: TickStyle::default(),
            show_tick_labels: default_true(),
            tick_label_config: TickLabelConfig::default(),
            show_needle: default_true(),
            needle_style: NeedleStyle::default(),
            needle_tail_style: NeedleTailStyle::default(),
            needle_length: default_needle_length(),
            needle_width: default_needle_width(),
            needle_color: default_needle_color_source(),
            needle_shadow: default_false(),
            show_center_hub: default_true(),
            center_hub_radius: default_hub_radius(),
            center_hub_color: default_hub_color_source(),
            center_hub_3d: default_false(),
            show_bezel: default_true(),
            bezel_width: default_bezel_width(),
            bezel_background: crate::ui::BackgroundConfig::default(),
            bezel_solid_color: default_bezel_solid_color(),
            zones: Vec::new(),
            animate: default_true(),
            animation_duration: default_animation_duration(),
            bounce_animation: default_false(),
            text_overlay: TextOverlayConfig::default(),
        }
    }
}

/// Render a speedometer gauge
pub fn render_speedometer(
    cr: &cairo::Context,
    config: &SpeedometerConfig,
    value: f64, // 0.0 to 1.0
    values: &HashMap<String, Value>,
    width: f64,
    height: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    // Use default theme for standalone rendering (e.g., from non-combo panels)
    render_speedometer_with_theme(cr, config, value, values, width, height, &ComboThemeConfig::default())
}

/// Render a speedometer gauge with theme support
pub fn render_speedometer_with_theme(
    cr: &cairo::Context,
    config: &SpeedometerConfig,
    value: f64, // 0.0 to 1.0
    values: &HashMap<String, Value>,
    width: f64,
    height: f64,
    theme: &ComboThemeConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let center_x = width / 2.0;
    let center_y = height / 2.0;
    let max_radius = center_x.min(center_y);
    let radius = max_radius * config.radius_percent;

    // Convert angles to radians
    let start_rad = config.start_angle.to_radians();
    let end_rad = config.end_angle.to_radians();

    // Calculate actual sweep accounting for wrap-around
    let mut sweep = end_rad - start_rad;
    if sweep < 0.0 {
        sweep += 2.0 * std::f64::consts::PI;
    }

    // Draw bezel
    if config.show_bezel {
        draw_bezel(cr, center_x, center_y, radius, config, theme, width, height)?;
    }

    // Draw value zones
    for zone in &config.zones {
        draw_zone(cr, center_x, center_y, radius, start_rad, sweep, config, zone)?;
    }

    // Draw track
    if config.show_track {
        draw_track(cr, center_x, center_y, radius, start_rad, end_rad, sweep, config, theme)?;
    }

    // Draw ticks and labels
    if config.show_major_ticks || config.show_minor_ticks || config.show_tick_labels {
        draw_ticks(cr, center_x, center_y, radius, start_rad, sweep, config, values, theme)?;
    }

    // Draw needle
    if config.show_needle {
        let needle_angle = start_rad + sweep * value.clamp(0.0, 1.0);
        draw_needle(cr, center_x, center_y, radius, needle_angle, config, theme)?;
    }

    // Draw center hub
    if config.show_center_hub {
        draw_center_hub(cr, center_x, center_y, radius, config, theme)?;
    }

    // Draw text overlay
    if config.text_overlay.enabled {
        crate::ui::text_renderer::render_text_lines_with_theme(
            cr,
            width,
            height,
            &config.text_overlay.text_config,
            values,
            Some(theme),
        );
    }

    Ok(())
}

fn draw_bezel(
    cr: &cairo::Context,
    center_x: f64,
    center_y: f64,
    radius: f64,
    config: &SpeedometerConfig,
    theme: &ComboThemeConfig,
    width: f64,
    height: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    if !config.show_bezel || config.bezel_width <= 0.0 {
        return Ok(());
    }

    let max_radius = center_x.min(center_y);
    let bezel_inner = radius;
    let bezel_outer = bezel_inner + (max_radius - bezel_inner) * config.bezel_width;

    cr.save()?;

    // Create clipping path for the bezel ring (donut shape)
    // Use new_sub_path() to prevent Cairo from drawing a line between the arcs
    cr.new_path();
    cr.arc(center_x, center_y, bezel_outer, 0.0, 2.0 * std::f64::consts::PI);
    cr.close_path();
    cr.new_sub_path();
    cr.arc(center_x, center_y, bezel_inner, 0.0, 2.0 * std::f64::consts::PI);
    cr.close_path();
    cr.set_fill_rule(cairo::FillRule::EvenOdd);
    cr.clip();

    // Render background within the clipped region
    // Use theme-aware color for solid backgrounds
    match &config.bezel_background.background {
        crate::ui::background::BackgroundType::Solid { .. } => {
            // Use theme-aware bezel_solid_color instead of the raw color
            let color = config.bezel_solid_color.resolve(theme);
            color.apply_to_cairo(cr);
            cr.rectangle(0.0, 0.0, width, height);
            cr.fill()?;
        }
        _ => {
            // For gradients and images, use the standard background rendering
            crate::ui::background::render_background(cr, &config.bezel_background, width, height)?;
        }
    }

    cr.restore()?;
    Ok(())
}

fn draw_zone(
    cr: &cairo::Context,
    center_x: f64,
    center_y: f64,
    radius: f64,
    start_rad: f64,
    sweep: f64,
    config: &SpeedometerConfig,
    zone: &ValueZone,
) -> Result<(), Box<dyn std::error::Error>> {
    cr.save()?;
    cr.new_path();  // Clear any existing path to prevent spurious lines

    let zone_start = start_rad + sweep * zone.start_percent;
    let zone_end = start_rad + sweep * zone.end_percent;
    let _zone_sweep = zone_end - zone_start;

    let arc_width_pixels = radius * config.arc_width;
    let _inner_radius = radius - arc_width_pixels / 2.0;
    let _outer_radius = radius + arc_width_pixels / 2.0;

    cr.set_source_rgba(zone.color.r, zone.color.g, zone.color.b, zone.alpha);
    cr.set_line_width(arc_width_pixels);
    cr.arc(center_x, center_y, radius, zone_start, zone_end);
    cr.stroke()?;

    cr.restore()?;
    Ok(())
}

fn draw_track(
    cr: &cairo::Context,
    center_x: f64,
    center_y: f64,
    radius: f64,
    start_rad: f64,
    end_rad: f64,
    sweep: f64,
    config: &SpeedometerConfig,
    theme: &ComboThemeConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    cr.save()?;
    cr.new_path();  // Clear any existing path to prevent spurious lines

    let arc_width_pixels = radius * config.arc_width;

    // If we have color stops, create gradient
    if config.track_color_stops.len() > 1 {
        // Resolve theme-aware color stops to concrete colors
        let resolved_stops: Vec<ColorStop> = config.track_color_stops
            .iter()
            .map(|stop| stop.resolve(theme))
            .collect();

        // Draw track with gradient by drawing many small segments
        let segments = 100;
        for i in 0..segments {
            let t1 = i as f64 / segments as f64;
            let t2 = (i + 1) as f64 / segments as f64;

            let angle1 = start_rad + sweep * t1;
            let angle2 = start_rad + sweep * t2;

            // Interpolate color from resolved stops
            let color = interpolate_color_stops(&resolved_stops, t1);

            cr.set_source_rgba(color.r, color.g, color.b, color.a);
            cr.set_line_width(arc_width_pixels);
            cr.arc(center_x, center_y, radius, angle1, angle2);
            cr.stroke()?;
        }
    } else {
        // Simple solid color track
        let track_color = config.track_color.resolve(theme);
        cr.set_source_rgba(track_color.r, track_color.g, track_color.b, track_color.a);
        cr.set_line_width(arc_width_pixels);
        cr.arc(center_x, center_y, radius, start_rad, end_rad);
        cr.stroke()?;
    }

    cr.restore()?;
    Ok(())
}

fn draw_ticks(
    cr: &cairo::Context,
    center_x: f64,
    center_y: f64,
    radius: f64,
    start_rad: f64,
    sweep: f64,
    config: &SpeedometerConfig,
    values: &HashMap<String, Value>,
    theme: &ComboThemeConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    cr.save()?;
    cr.new_path();  // Clear any existing path

    let arc_width_pixels = radius * config.arc_width;
    let tick_base_radius = radius + arc_width_pixels / 2.0;

    // Resolve colors once
    let major_tick_color = config.major_tick_color.resolve(theme);
    let minor_tick_color = config.minor_tick_color.resolve(theme);
    let tick_label_color = config.tick_label_config.color.resolve(theme);

    // Get min/max for label calculation
    let min_val = values.get("min_limit").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let max_val = values.get("max_limit").and_then(|v| v.as_f64()).unwrap_or(100.0);

    // Draw major ticks and labels
    for i in 0..=config.major_tick_count {
        let t = i as f64 / config.major_tick_count as f64;
        let angle = start_rad + sweep * t;

        // Draw major tick
        if config.show_major_ticks {
            draw_single_tick(
                cr,
                center_x,
                center_y,
                tick_base_radius,
                angle,
                config.major_tick_length * arc_width_pixels,
                config.major_tick_width,
                &major_tick_color,
                config.major_tick_style,
            )?;
        }

        // Draw label
        if config.show_tick_labels {
            let label_value = if config.tick_label_config.use_percentage {
                t * 100.0
            } else {
                min_val + (max_val - min_val) * t
            };

            let label_text = if config.tick_label_config.use_percentage {
                format!("{:.0}", label_value)
            } else if (max_val - min_val).abs() < 10.0 {
                format!("{:.1}", label_value)
            } else {
                format!("{:.0}", label_value)
            };

            draw_tick_label(
                cr,
                center_x,
                center_y,
                tick_base_radius + config.major_tick_length * arc_width_pixels + 5.0,
                angle,
                &label_text,
                &config.tick_label_config,
                &tick_label_color,
            )?;
        }

        // Draw minor ticks between major ticks
        if config.show_minor_ticks && i < config.major_tick_count {
            let major_span = 1.0 / config.major_tick_count as f64;
            let minor_span = major_span / (config.minor_ticks_per_major + 1) as f64;

            for j in 1..=config.minor_ticks_per_major {
                let minor_t = t + minor_span * j as f64;
                if minor_t <= 1.0 {
                    let minor_angle = start_rad + sweep * minor_t;
                    draw_single_tick(
                        cr,
                        center_x,
                        center_y,
                        tick_base_radius,
                        minor_angle,
                        config.minor_tick_length * arc_width_pixels,
                        config.minor_tick_width,
                        &minor_tick_color,
                        config.minor_tick_style,
                    )?;
                }
            }
        }
    }

    cr.restore()?;
    Ok(())
}

fn draw_single_tick(
    cr: &cairo::Context,
    center_x: f64,
    center_y: f64,
    base_radius: f64,
    angle: f64,
    length: f64,
    width: f64,
    color: &Color,
    style: TickStyle,
) -> Result<(), Box<dyn std::error::Error>> {
    cr.save()?;

    let x_base = center_x + base_radius * angle.cos();
    let y_base = center_y + base_radius * angle.sin();
    let x_tip = center_x + (base_radius + length) * angle.cos();
    let y_tip = center_y + (base_radius + length) * angle.sin();

    cr.set_source_rgba(color.r, color.g, color.b, color.a);
    cr.set_line_width(width);

    match style {
        TickStyle::Line => {
            cr.move_to(x_base, y_base);
            cr.line_to(x_tip, y_tip);
            cr.stroke()?;
        },
        TickStyle::Wedge => {
            // Draw wedge shape
            let perp_x = -angle.sin();
            let perp_y = angle.cos();
            let half_width = width / 2.0;

            cr.move_to(x_base + perp_x * half_width, y_base + perp_y * half_width);
            cr.line_to(x_base - perp_x * half_width, y_base - perp_y * half_width);
            cr.line_to(x_tip, y_tip);
            cr.close_path();
            cr.fill()?;
        },
        TickStyle::Dot => {
            cr.arc(x_tip, y_tip, width, 0.0, 2.0 * std::f64::consts::PI);
            cr.fill()?;
        },
    }

    cr.restore()?;
    Ok(())
}

fn draw_tick_label(
    cr: &cairo::Context,
    center_x: f64,
    center_y: f64,
    label_radius: f64,
    angle: f64,
    text: &str,
    label_config: &TickLabelConfig,
    resolved_color: &Color,
) -> Result<(), Box<dyn std::error::Error>> {
    cr.save()?;

    let font_slant = if label_config.italic {
        cairo::FontSlant::Italic
    } else {
        cairo::FontSlant::Normal
    };
    let font_weight = if label_config.bold {
        cairo::FontWeight::Bold
    } else {
        cairo::FontWeight::Normal
    };

    cr.select_font_face(&label_config.font_family, font_slant, font_weight);
    cr.set_font_size(label_config.font_size);
    cr.set_source_rgba(resolved_color.r, resolved_color.g, resolved_color.b, resolved_color.a);

    let extents = cr.text_extents(text)?;
    let x = center_x + label_radius * angle.cos() - extents.width() / 2.0;
    let y = center_y + label_radius * angle.sin() + extents.height() / 2.0;

    cr.move_to(x, y);
    cr.show_text(text)?;

    cr.restore()?;
    Ok(())
}

fn draw_needle(
    cr: &cairo::Context,
    center_x: f64,
    center_y: f64,
    radius: f64,
    angle: f64,
    config: &SpeedometerConfig,
    theme: &ComboThemeConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    cr.save()?;

    let needle_length = radius * config.needle_length;
    let tail_length = match config.needle_tail_style {
        NeedleTailStyle::None => 0.0,
        NeedleTailStyle::Short => needle_length * 0.1,
        NeedleTailStyle::Balanced => needle_length * 0.2,
    };

    // Draw shadow if enabled
    if config.needle_shadow {
        cr.save()?;
        cr.set_source_rgba(0.0, 0.0, 0.0, 0.3);
        draw_needle_shape(cr, center_x + 2.0, center_y + 2.0, angle, needle_length, tail_length, config)?;
        cr.restore()?;
    }

    // Draw needle
    let needle_color = config.needle_color.resolve(theme);
    cr.set_source_rgba(needle_color.r, needle_color.g, needle_color.b, needle_color.a);
    draw_needle_shape(cr, center_x, center_y, angle, needle_length, tail_length, config)?;

    cr.restore()?;
    Ok(())
}

fn draw_needle_shape(
    cr: &cairo::Context,
    center_x: f64,
    center_y: f64,
    angle: f64,
    length: f64,
    tail_length: f64,
    config: &SpeedometerConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let tip_x = center_x + length * angle.cos();
    let tip_y = center_y + length * angle.sin();
    let tail_x = center_x - tail_length * angle.cos();
    let tail_y = center_y - tail_length * angle.sin();

    let perp_x = -angle.sin();
    let perp_y = angle.cos();

    match config.needle_style {
        NeedleStyle::Line => {
            cr.set_line_width(config.needle_width);
            cr.move_to(tail_x, tail_y);
            cr.line_to(tip_x, tip_y);
            cr.stroke()?;
        },
        NeedleStyle::Arrow => {
            let half_width = config.needle_width / 2.0;
            cr.move_to(tail_x + perp_x * half_width, tail_y + perp_y * half_width);
            cr.line_to(tail_x - perp_x * half_width, tail_y - perp_y * half_width);
            cr.line_to(center_x - perp_x * half_width, center_y - perp_y * half_width);
            cr.line_to(tip_x, tip_y);
            cr.line_to(center_x + perp_x * half_width, center_y + perp_y * half_width);
            cr.close_path();
            cr.fill()?;
        },
        NeedleStyle::Tapered => {
            let base_width = config.needle_width;
            let tip_width = config.needle_width * 0.3;
            cr.move_to(tail_x + perp_x * base_width, tail_y + perp_y * base_width);
            cr.line_to(tail_x - perp_x * base_width, tail_y - perp_y * base_width);
            cr.line_to(tip_x - perp_x * tip_width, tip_y - perp_y * tip_width);
            cr.line_to(tip_x + perp_x * tip_width, tip_y + perp_y * tip_width);
            cr.close_path();
            cr.fill()?;
        },
        NeedleStyle::Triangle => {
            let half_width = config.needle_width / 2.0;
            cr.move_to(center_x + perp_x * half_width, center_y + perp_y * half_width);
            cr.line_to(center_x - perp_x * half_width, center_y - perp_y * half_width);
            cr.line_to(tip_x, tip_y);
            cr.close_path();
            cr.fill()?;
        },
    }

    Ok(())
}

fn draw_center_hub(
    cr: &cairo::Context,
    center_x: f64,
    center_y: f64,
    radius: f64,
    config: &SpeedometerConfig,
    theme: &ComboThemeConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    cr.save()?;

    let hub_radius = radius * config.center_hub_radius;
    let hub_color = config.center_hub_color.resolve(theme);

    if config.center_hub_3d {
        // 3D effect with radial gradient
        let gradient = cairo::RadialGradient::new(
            center_x - hub_radius * 0.3,
            center_y - hub_radius * 0.3,
            hub_radius * 0.2,
            center_x,
            center_y,
            hub_radius,
        );
        gradient.add_color_stop_rgb(0.0,
            (hub_color.r * 1.5).min(1.0),
            (hub_color.g * 1.5).min(1.0),
            (hub_color.b * 1.5).min(1.0)
        );
        gradient.add_color_stop_rgb(1.0,
            hub_color.r * 0.5,
            hub_color.g * 0.5,
            hub_color.b * 0.5
        );
        cr.set_source(&gradient)?;
    } else {
        cr.set_source_rgba(hub_color.r, hub_color.g, hub_color.b, hub_color.a);
    }

    cr.arc(center_x, center_y, hub_radius, 0.0, 2.0 * std::f64::consts::PI);
    cr.fill()?;

    cr.restore()?;
    Ok(())
}

fn interpolate_color_stops(stops: &[ColorStop], t: f64) -> Color {
    use crate::ui::render_cache::get_cached_color_at;

    if stops.is_empty() {
        return Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
    }

    get_cached_color_at(stops, t)
}
