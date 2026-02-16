//! Arc gauge display configuration types

use serde::{Deserialize, Serialize};

use crate::color::Color;
use crate::text::TextOverlayConfig;
use crate::theme::{
    deserialize_color_or_source, deserialize_color_stops_vec, ColorSource, ColorStopSource,
    ComboThemeConfig,
};

/// Arc end cap style
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum ArcCapStyle {
    #[serde(rename = "butt")]
    Butt, // Square/flat end
    #[serde(rename = "round")]
    #[default]
    Round, // Rounded end
    #[serde(rename = "pointed")]
    Pointed, // Pointed/triangular end
}

/// Arc tapering style
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum ArcTaperStyle {
    #[serde(rename = "none")]
    #[default]
    None, // No tapering, constant width
    #[serde(rename = "start")]
    Start, // Narrower at start
    #[serde(rename = "end")]
    End, // Narrower at end
    #[serde(rename = "both")]
    Both, // Narrower at both ends (elliptical)
}

/// Color transition style
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum ColorTransitionStyle {
    #[serde(rename = "smooth")]
    Smooth, // Smooth gradient fade between colors
    #[serde(rename = "abrupt")]
    #[default]
    Abrupt, // Abrupt change at threshold
}

/// Color application mode
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum ColorApplicationMode {
    #[serde(rename = "progressive")]
    #[default]
    Progressive, // Whole arc changes color based on value
    #[serde(rename = "segments")]
    Segments, // Individual segments have fixed colors
}

/// Arc gauge configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ArcDisplayConfig {
    // Arc geometry
    #[serde(default = "default_start_angle")]
    pub start_angle: f64, // Degrees (0 = right, 90 = down, 180 = left, 270 = up)
    #[serde(default = "default_end_angle")]
    pub end_angle: f64, // Degrees
    #[serde(default = "default_arc_width")]
    pub arc_width: f64, // Percentage of radius (0.0 to 1.0)
    #[serde(default = "default_radius_percent")]
    pub radius_percent: f64, // Percentage of available space (0.0 to 1.0)

    // Segmentation
    #[serde(default = "default_false")]
    pub segmented: bool,
    #[serde(default = "default_segment_count")]
    pub segment_count: u32,
    #[serde(default = "default_segment_spacing")]
    pub segment_spacing: f64, // Degrees

    // Style
    #[serde(default)]
    pub cap_style: ArcCapStyle,
    #[serde(default)]
    pub taper_style: ArcTaperStyle,
    #[serde(default = "default_taper_amount")]
    pub taper_amount: f64, // 0.0 to 1.0 (how much to taper)

    // Colors
    #[serde(
        default = "default_color_stops",
        deserialize_with = "deserialize_color_stops_vec"
    )]
    pub color_stops: Vec<ColorStopSource>,
    #[serde(default)]
    pub color_transition: ColorTransitionStyle,
    #[serde(default)]
    pub color_mode: ColorApplicationMode,

    // Background arc (unfilled portion)
    #[serde(default = "default_show_background_arc")]
    pub show_background_arc: bool,
    #[serde(
        default = "default_background_color",
        deserialize_with = "deserialize_color_or_source"
    )]
    pub background_color: ColorSource,
    #[serde(default = "default_false")]
    pub overlay_background: bool, // If true, draw full arc then overlay with background color

    // Animation
    #[serde(default = "default_false")]
    pub animate: bool,
    #[serde(default = "default_animation_duration")]
    pub animation_duration: f64, // Duration in seconds

    // Text overlay
    #[serde(default)]
    pub text_overlay: TextOverlayConfig,

    // Theme configuration for resolving theme color/font references
    #[serde(default)]
    pub theme: ComboThemeConfig,
}

fn default_start_angle() -> f64 {
    135.0 // Bottom-left
}

fn default_end_angle() -> f64 {
    45.0 // Bottom-right (goes through top)
}

fn default_arc_width() -> f64 {
    0.15 // 15% of radius
}

fn default_radius_percent() -> f64 {
    0.85 // 85% of available space
}

fn default_false() -> bool {
    false
}

fn default_segment_count() -> u32 {
    20
}

fn default_segment_spacing() -> f64 {
    2.0
}

fn default_taper_amount() -> f64 {
    0.5
}

fn default_color_stops() -> Vec<ColorStopSource> {
    vec![
        ColorStopSource::custom(0.0, Color::new(0.0, 0.8, 0.0, 1.0)), // Green 0-60%
        ColorStopSource::custom(0.6, Color::new(0.0, 0.8, 0.0, 1.0)), // Green at 60%
        ColorStopSource::custom(0.6, Color::new(1.0, 0.8, 0.0, 1.0)), // Yellow at 60%
        ColorStopSource::custom(0.8, Color::new(1.0, 0.8, 0.0, 1.0)), // Yellow at 80%
        ColorStopSource::custom(0.8, Color::new(1.0, 0.0, 0.0, 1.0)), // Red at 80%
        ColorStopSource::custom(1.0, Color::new(1.0, 0.0, 0.0, 1.0)), // Red at 100%
    ]
}

fn default_show_background_arc() -> bool {
    true
}

fn default_background_color() -> ColorSource {
    ColorSource::Custom {
        color: Color::new(0.2, 0.2, 0.2, 0.3),
    }
}

fn default_animation_duration() -> f64 {
    0.3 // 300ms
}

impl Default for ArcDisplayConfig {
    fn default() -> Self {
        Self {
            start_angle: default_start_angle(),
            end_angle: default_end_angle(),
            arc_width: default_arc_width(),
            radius_percent: default_radius_percent(),
            segmented: default_false(),
            segment_count: default_segment_count(),
            segment_spacing: default_segment_spacing(),
            cap_style: ArcCapStyle::default(),
            taper_style: ArcTaperStyle::default(),
            taper_amount: default_taper_amount(),
            color_stops: default_color_stops(),
            color_transition: ColorTransitionStyle::default(),
            color_mode: ColorApplicationMode::default(),
            show_background_arc: default_show_background_arc(),
            background_color: default_background_color(),
            overlay_background: default_false(),
            animate: default_false(),
            animation_duration: default_animation_duration(),
            text_overlay: TextOverlayConfig::default(),
            theme: ComboThemeConfig::default(),
        }
    }
}
