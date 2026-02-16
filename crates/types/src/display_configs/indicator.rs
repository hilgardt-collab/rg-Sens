//! Indicator displayer configuration types

use serde::{Deserialize, Serialize};

use crate::color::{Color, ColorStop};
use crate::text::TextDisplayerConfig;

/// Shape type for the indicator
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum IndicatorShape {
    /// Fill the entire panel with the color
    #[default]
    Fill,
    /// Circle shape
    Circle,
    /// Square shape
    Square,
    /// Regular polygon with N sides
    Polygon(u32),
}

impl IndicatorShape {
    /// Get display name for UI
    pub fn display_name(&self) -> String {
        match self {
            IndicatorShape::Fill => "Fill".to_string(),
            IndicatorShape::Circle => "Circle".to_string(),
            IndicatorShape::Square => "Square".to_string(),
            IndicatorShape::Polygon(n) => format!("{}-gon", n),
        }
    }

    /// Get common shapes for UI dropdown
    pub fn common_shapes() -> Vec<IndicatorShape> {
        vec![
            IndicatorShape::Fill,
            IndicatorShape::Circle,
            IndicatorShape::Square,
            IndicatorShape::Polygon(3), // Triangle
            IndicatorShape::Polygon(5), // Pentagon
            IndicatorShape::Polygon(6), // Hexagon
            IndicatorShape::Polygon(7), // Heptagon
            IndicatorShape::Polygon(8), // Octagon
        ]
    }
}

/// Configuration for the indicator displayer
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IndicatorConfig {
    /// The field to use for the value (should be 0-100 range)
    #[serde(default = "default_value_field")]
    pub value_field: String,

    /// Gradient stops defining the color mapping
    /// Position 0.0 = value 0%, position 1.0 = value 100%
    #[serde(default = "default_gradient")]
    pub gradient_stops: Vec<ColorStop>,

    /// Shape to display
    #[serde(default)]
    pub shape: IndicatorShape,

    /// Size of the shape as percentage of panel (0.0-1.0)
    /// Only applies to non-Fill shapes
    #[serde(default = "default_shape_size")]
    pub shape_size: f64,

    /// Rotation angle in degrees for the shape
    #[serde(default)]
    pub rotation_angle: f64,

    /// Whether to show text overlay
    #[serde(default)]
    pub show_text: bool,

    /// Text overlay configuration
    #[serde(default)]
    pub text_config: TextDisplayerConfig,

    /// Border width (0 for no border)
    #[serde(default)]
    pub border_width: f64,

    /// Border color
    #[serde(default = "default_border_color")]
    pub border_color: Color,

    /// Minimum value (for mapping to gradient)
    #[serde(default)]
    pub min_value: f64,

    /// Maximum value (for mapping to gradient)
    #[serde(default = "default_max_value")]
    pub max_value: f64,
}

fn default_value_field() -> String {
    "value".to_string()
}

fn default_shape_size() -> f64 {
    0.8
}

fn default_max_value() -> f64 {
    100.0
}

fn default_border_color() -> Color {
    Color::new(1.0, 1.0, 1.0, 0.5)
}

fn default_gradient() -> Vec<ColorStop> {
    vec![
        ColorStop::new(0.0, Color::new(0.0, 0.5, 1.0, 1.0)), // Blue at 0%
        ColorStop::new(0.4, Color::new(0.0, 1.0, 0.0, 1.0)), // Green at 40%
        ColorStop::new(0.7, Color::new(1.0, 1.0, 0.0, 1.0)), // Yellow at 70%
        ColorStop::new(1.0, Color::new(1.0, 0.0, 0.0, 1.0)), // Red at 100%
    ]
}

impl Default for IndicatorConfig {
    fn default() -> Self {
        Self {
            value_field: default_value_field(),
            gradient_stops: default_gradient(),
            shape: IndicatorShape::default(),
            shape_size: default_shape_size(),
            rotation_angle: 0.0,
            show_text: false,
            text_config: TextDisplayerConfig::default(),
            border_width: 0.0,
            border_color: default_border_color(),
            min_value: 0.0,
            max_value: 100.0,
        }
    }
}
