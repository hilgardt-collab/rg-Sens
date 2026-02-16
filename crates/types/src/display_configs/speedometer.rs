//! Speedometer gauge display configuration types

use serde::{Deserialize, Serialize};

use crate::background::BackgroundConfig;
use crate::color::Color;
use crate::text::TextOverlayConfig;
use crate::theme::{
    deserialize_color_or_source, ColorSource, ColorStopSource,
};

/// Needle style
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum NeedleStyle {
    #[serde(rename = "arrow")]
    #[default]
    Arrow, // Traditional pointed arrow
    #[serde(rename = "line")]
    Line, // Simple line
    #[serde(rename = "tapered")]
    Tapered, // Tapered line (wider at base)
    #[serde(rename = "triangle")]
    Triangle, // Solid triangle
}

/// Needle tail style
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum NeedleTailStyle {
    #[serde(rename = "none")]
    None, // No tail
    #[serde(rename = "short")]
    #[default]
    Short, // Short tail opposite direction
    #[serde(rename = "balanced")]
    Balanced, // Balanced tail for counterweight
}

/// Tick mark style
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum TickStyle {
    #[serde(rename = "line")]
    #[default]
    Line, // Simple lines
    #[serde(rename = "wedge")]
    Wedge, // Wedge-shaped ticks
    #[serde(rename = "dot")]
    Dot, // Dots at tick positions
}

/// Bezel style
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum BezelStyle {
    #[serde(rename = "none")]
    None, // No bezel
    #[serde(rename = "simple")]
    #[default]
    Simple, // Simple ring
    #[serde(rename = "3d")]
    ThreeD, // 3D effect with gradient
    #[serde(rename = "chrome")]
    Chrome, // Metallic chrome effect
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
            color: ColorSource::Custom {
                color: Color {
                    r: 0.9,
                    g: 0.9,
                    b: 0.9,
                    a: 1.0,
                },
            },
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
    pub start_angle: f64, // Degrees (0 = right, 90 = down, 180 = left, 270 = up)
    #[serde(default = "default_end_angle")]
    pub end_angle: f64, // Degrees
    #[serde(default = "default_arc_width")]
    pub arc_width: f64, // Percentage of radius (0.0 to 1.0)
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
    pub bezel_background: BackgroundConfig,
    /// Theme-aware solid color for bezel (used when bezel_background is solid)
    #[serde(
        default = "default_bezel_solid_color",
        deserialize_with = "deserialize_color_or_source"
    )]
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
    pub start_percent: f64, // 0.0 to 1.0
    pub end_percent: f64,   // 0.0 to 1.0
    pub color: Color,
    #[serde(default = "default_zone_alpha")]
    pub alpha: f64, // Transparency
}

fn default_zone_alpha() -> f64 {
    0.3
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
    ColorSource::Custom {
        color: Color {
            r: 0.2,
            g: 0.2,
            b: 0.2,
            a: 1.0,
        },
    }
}

fn default_color_stops() -> Vec<ColorStopSource> {
    vec![
        ColorStopSource::custom(
            0.0,
            Color {
                r: 0.0,
                g: 0.8,
                b: 0.0,
                a: 1.0,
            },
        ),
        ColorStopSource::custom(
            0.7,
            Color {
                r: 1.0,
                g: 0.8,
                b: 0.0,
                a: 1.0,
            },
        ),
        ColorStopSource::custom(
            0.9,
            Color {
                r: 1.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            },
        ),
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
    ColorSource::Custom {
        color: Color {
            r: 0.9,
            g: 0.9,
            b: 0.9,
            a: 1.0,
        },
    }
}

fn default_needle_length() -> f64 {
    0.75
}

fn default_needle_width() -> f64 {
    3.0
}

fn default_needle_color_source() -> ColorSource {
    ColorSource::Custom {
        color: Color {
            r: 1.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        },
    }
}

fn default_hub_radius() -> f64 {
    0.08
}

fn default_hub_color_source() -> ColorSource {
    ColorSource::Custom {
        color: Color {
            r: 0.3,
            g: 0.3,
            b: 0.3,
            a: 1.0,
        },
    }
}

fn default_bezel_width() -> f64 {
    0.05
}

fn default_bezel_solid_color() -> ColorSource {
    ColorSource::Custom {
        color: Color::new(0.3, 0.3, 0.3, 1.0),
    }
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
            bezel_background: BackgroundConfig::default(),
            bezel_solid_color: default_bezel_solid_color(),
            zones: Vec::new(),
            animate: default_true(),
            animation_duration: default_animation_duration(),
            bounce_animation: default_false(),
            text_overlay: TextOverlayConfig::default(),
        }
    }
}
