//! Bar display configuration types

use serde::{Deserialize, Serialize};

use crate::color::{Color, ColorStop};
use crate::text::TextOverlayConfig;
use crate::theme::{
    deserialize_color_or_source, deserialize_color_stops_vec, ColorSource, ColorStopSource,
    ComboThemeConfig,
};

/// Bar display style
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum BarStyle {
    #[serde(rename = "full")]
    #[default]
    Full, // Fill entire panel
    #[serde(rename = "rectangle")]
    Rectangle, // Rectangular bar with rounded corners
    #[serde(rename = "segmented")]
    Segmented, // Multiple segments with spacing
}

/// Bar orientation
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum BarOrientation {
    #[serde(rename = "horizontal")]
    #[default]
    Horizontal,
    #[serde(rename = "vertical")]
    Vertical,
}

/// Bar fill direction
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum BarFillDirection {
    #[serde(rename = "left_to_right")]
    #[default]
    LeftToRight,
    #[serde(rename = "right_to_left")]
    RightToLeft,
    #[serde(rename = "bottom_to_top")]
    BottomToTop,
    #[serde(rename = "top_to_bottom")]
    TopToBottom,
}

/// Bar tapering style
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum BarTaperStyle {
    #[serde(rename = "none")]
    #[default]
    None, // No tapering, constant width
    #[serde(rename = "start")]
    Start, // Narrower at start
    #[serde(rename = "end")]
    End, // Narrower at end
    #[serde(rename = "both")]
    Both, // Narrower at both ends
}

/// Bar taper alignment (where the taper is anchored)
/// For horizontal bars: Start=Top, Center=Center, End=Bottom
/// For vertical bars: Start=Left, Center=Center, End=Right
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum BarTaperAlignment {
    #[serde(rename = "start")]
    Start, // Top for horizontal, Left for vertical
    #[serde(rename = "center")]
    #[default]
    Center, // Centered (default)
    #[serde(rename = "end")]
    End, // Bottom for horizontal, Right for vertical
}

/// Foreground fill type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum BarFillType {
    #[serde(rename = "solid")]
    Solid {
        #[serde(deserialize_with = "deserialize_color_or_source")]
        color: ColorSource,
    },
    #[serde(rename = "gradient")]
    Gradient {
        #[serde(deserialize_with = "deserialize_color_stops_vec")]
        stops: Vec<ColorStopSource>,
        #[serde(default = "default_gradient_angle")]
        angle: f64,
    },
}

fn default_gradient_angle() -> f64 {
    90.0
}

impl BarFillType {
    /// Resolve to actual colors using theme
    pub fn resolve(&self, theme: &ComboThemeConfig) -> ResolvedBarFill {
        match self {
            BarFillType::Solid { color } => ResolvedBarFill::Solid {
                color: color.resolve(theme),
            },
            BarFillType::Gradient { stops, angle } => ResolvedBarFill::Gradient {
                stops: stops.iter().map(|s| s.resolve(theme)).collect(),
                angle: *angle,
            },
        }
    }
}

/// Resolved bar fill with actual colors (no theme references)
#[derive(Debug, Clone)]
pub enum ResolvedBarFill {
    Solid { color: Color },
    Gradient { stops: Vec<ColorStop>, angle: f64 },
}

impl Default for BarFillType {
    fn default() -> Self {
        // Default to theme color 1 (primary)
        Self::Solid {
            color: ColorSource::Theme { index: 1 },
        }
    }
}

/// Background fill type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
#[derive(Default)]
pub enum BarBackgroundType {
    #[serde(rename = "solid")]
    Solid {
        #[serde(deserialize_with = "deserialize_color_or_source")]
        color: ColorSource,
    },
    #[serde(rename = "gradient")]
    Gradient {
        #[serde(deserialize_with = "deserialize_color_stops_vec")]
        stops: Vec<ColorStopSource>,
        #[serde(default = "default_gradient_angle")]
        angle: f64,
    },
    #[serde(rename = "transparent")]
    #[default]
    Transparent,
}

impl BarBackgroundType {
    /// Resolve to actual colors using theme
    pub fn resolve(&self, theme: &ComboThemeConfig) -> ResolvedBarBackground {
        match self {
            BarBackgroundType::Solid { color } => ResolvedBarBackground::Solid {
                color: color.resolve(theme),
            },
            BarBackgroundType::Gradient { stops, angle } => ResolvedBarBackground::Gradient {
                stops: stops.iter().map(|s| s.resolve(theme)).collect(),
                angle: *angle,
            },
            BarBackgroundType::Transparent => ResolvedBarBackground::Transparent,
        }
    }
}

/// Resolved bar background with actual colors (no theme references)
#[derive(Debug, Clone)]
pub enum ResolvedBarBackground {
    Solid { color: Color },
    Gradient { stops: Vec<ColorStop>, angle: f64 },
    Transparent,
}

/// Border configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BorderConfig {
    pub enabled: bool,
    #[serde(deserialize_with = "deserialize_color_or_source")]
    pub color: ColorSource,
    pub width: f64,
}

impl BorderConfig {
    /// Resolve to actual color using theme
    pub fn resolve_color(&self, theme: &ComboThemeConfig) -> Color {
        self.color.resolve(theme)
    }
}

impl Default for BorderConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            color: ColorSource::Theme { index: 2 }, // Theme secondary color
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

    // Taper style
    #[serde(default)]
    pub taper_style: BarTaperStyle,
    #[serde(default = "default_taper_amount")]
    pub taper_amount: f64, // 0.0 to 1.0 (how much to taper)
    #[serde(default)]
    pub taper_alignment: BarTaperAlignment, // Where the taper is anchored

    // Text overlay
    #[serde(default)]
    pub text_overlay: TextOverlayConfig,

    // Animation
    #[serde(default = "default_true")]
    pub smooth_animation: bool,
    #[serde(default = "default_animation_speed")]
    pub animation_speed: f64, // 0.0 to 1.0

    // Theme configuration for resolving theme color/font references
    #[serde(default)]
    pub theme: ComboThemeConfig,
}

fn default_corner_radius() -> f64 {
    5.0
}

fn default_padding() -> f64 {
    4.0
}

fn default_rectangle_width() -> f64 {
    0.8 // 80% of panel width
}

fn default_rectangle_height() -> f64 {
    0.6 // 60% of panel height
}

fn default_segment_count() -> u32 {
    10
}

fn default_segment_spacing() -> f64 {
    2.0
}

fn default_segment_width() -> f64 {
    0.9 // 90% of panel width
}

fn default_segment_height() -> f64 {
    0.8 // 80% of panel height
}

fn default_true() -> bool {
    true
}

fn default_animation_speed() -> f64 {
    0.5
}

fn default_taper_amount() -> f64 {
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
            taper_style: BarTaperStyle::default(),
            taper_amount: default_taper_amount(),
            taper_alignment: BarTaperAlignment::default(),
            text_overlay: TextOverlayConfig::default(),
            smooth_animation: default_true(),
            animation_speed: default_animation_speed(),
            theme: ComboThemeConfig::default(),
        }
    }
}
