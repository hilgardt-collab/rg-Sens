//! Background configuration types for panels.

use serde::{Deserialize, Serialize};

use crate::color::{Color, ColorStop, RadialGradientConfig, LinearGradientConfig};
use crate::theme::ColorSource;

/// Tiling polygons configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PolygonConfig {
    pub tile_size: u32,
    pub num_sides: u32,
    pub rotation_angle: f64,
    pub colors: Vec<ColorSource>,
    /// Background color drawn behind polygons (fills gaps)
    #[serde(default = "default_polygon_background")]
    pub background_color: ColorSource,
}

fn default_polygon_background() -> ColorSource {
    ColorSource::custom(Color::new(0.1, 0.1, 0.12, 1.0))
}

impl Default for PolygonConfig {
    fn default() -> Self {
        Self {
            tile_size: 60,
            num_sides: 6, // Hexagons by default
            rotation_angle: 0.0,
            colors: vec![
                ColorSource::custom(Color::new(0.2, 0.2, 0.25, 1.0)),
                ColorSource::custom(Color::new(0.15, 0.15, 0.2, 1.0)),
            ],
            background_color: default_polygon_background(),
        }
    }
}

/// Image display mode
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum ImageDisplayMode {
    #[serde(rename = "fit")]
    #[default]
    Fit,
    #[serde(rename = "stretch")]
    Stretch,
    #[serde(rename = "zoom")]
    Zoom,
    #[serde(rename = "tile")]
    Tile,
}

/// Background type configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum BackgroundType {
    #[serde(rename = "solid")]
    Solid { color: ColorSource },
    #[serde(rename = "linear_gradient")]
    LinearGradient(LinearGradientConfig),
    #[serde(rename = "radial_gradient")]
    RadialGradient(RadialGradientConfig),
    #[serde(rename = "image")]
    Image {
        path: String,
        #[serde(default)]
        display_mode: ImageDisplayMode,
        #[serde(default = "default_alpha")]
        alpha: f64,
    },
    #[serde(rename = "polygons")]
    Polygons(PolygonConfig),
    #[serde(rename = "indicator")]
    Indicator(IndicatorBackgroundConfig),
}

impl BackgroundType {
    /// Returns true if this is an indicator background type
    pub fn is_indicator(&self) -> bool {
        matches!(self, BackgroundType::Indicator(_))
    }
}

/// Configuration for indicator background (value-based color from gradient)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IndicatorBackgroundConfig {
    /// Gradient stops defining the color mapping (position 0.0 = 0%, position 1.0 = 100%)
    #[serde(default = "default_indicator_gradient")]
    pub gradient_stops: Vec<ColorStop>,
    /// Shape to display
    #[serde(default)]
    pub shape: IndicatorBackgroundShape,
    /// Size of shape (0.0-1.0 relative to panel)
    #[serde(default = "default_indicator_size")]
    pub shape_size: f64,
    /// Rotation angle in degrees
    #[serde(default)]
    pub rotation_angle: f64,
    /// Border width
    #[serde(default)]
    pub border_width: f64,
    /// Border color
    #[serde(default = "default_indicator_border_color")]
    pub border_color: Color,
    /// Static value to use when no live data available (0-100)
    #[serde(default = "default_indicator_value")]
    pub static_value: f64,
    /// Field to bind to for live value updates
    #[serde(default)]
    pub value_field: String,
    /// Min value for mapping
    #[serde(default)]
    pub min_value: f64,
    /// Max value for mapping
    #[serde(default = "default_indicator_max")]
    pub max_value: f64,
}

fn default_indicator_gradient() -> Vec<ColorStop> {
    vec![
        ColorStop::new(0.0, Color::new(0.0, 0.5, 1.0, 1.0)), // Blue at 0%
        ColorStop::new(0.4, Color::new(0.0, 1.0, 0.0, 1.0)), // Green at 40%
        ColorStop::new(0.7, Color::new(1.0, 1.0, 0.0, 1.0)), // Yellow at 70%
        ColorStop::new(1.0, Color::new(1.0, 0.0, 0.0, 1.0)), // Red at 100%
    ]
}

fn default_indicator_size() -> f64 {
    0.8
}

fn default_indicator_border_color() -> Color {
    Color::new(1.0, 1.0, 1.0, 0.5)
}

fn default_indicator_value() -> f64 {
    50.0
}

fn default_indicator_max() -> f64 {
    100.0
}

/// Shape for indicator background
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum IndicatorBackgroundShape {
    #[default]
    Fill,
    Circle,
    Square,
    Polygon(u32),
}

impl Default for IndicatorBackgroundConfig {
    fn default() -> Self {
        Self {
            gradient_stops: default_indicator_gradient(),
            shape: IndicatorBackgroundShape::default(),
            shape_size: default_indicator_size(),
            rotation_angle: 0.0,
            border_width: 0.0,
            border_color: default_indicator_border_color(),
            static_value: default_indicator_value(),
            value_field: "value".to_string(),
            min_value: 0.0,
            max_value: 100.0,
        }
    }
}

fn default_alpha() -> f64 {
    1.0
}

impl Default for BackgroundType {
    fn default() -> Self {
        Self::Solid {
            color: ColorSource::custom(Color::new(0.15, 0.15, 0.15, 1.0)),
        }
    }
}

/// Background configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct BackgroundConfig {
    pub background: BackgroundType,
}
