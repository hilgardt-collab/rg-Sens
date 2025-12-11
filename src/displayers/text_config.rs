//! Configuration for text displayer

use serde::{Deserialize, Serialize};

/// Vertical position of text
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum VerticalPosition {
    Top,
    Center,
    Bottom,
}

impl Default for VerticalPosition {
    fn default() -> Self {
        Self::Center
    }
}

/// Horizontal position of text
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum HorizontalPosition {
    Left,
    Center,
    Right,
}

impl Default for HorizontalPosition {
    fn default() -> Self {
        Self::Center
    }
}

/// Configuration for a single line of text
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TextLineConfig {
    /// ID of the field to display (e.g., "caption", "usage", "unit")
    pub field_id: String,

    /// Font family (e.g., "Sans", "Monospace")
    pub font_family: String,

    /// Font size in points
    pub font_size: f64,

    /// Whether the font is bold
    #[serde(default)]
    pub bold: bool,

    /// Whether the font is italic
    #[serde(default)]
    pub italic: bool,

    /// Text color (RGBA, 0.0-1.0)
    pub color: (f64, f64, f64, f64),

    /// Vertical position on panel
    pub vertical_position: VerticalPosition,

    /// Horizontal position on panel
    pub horizontal_position: HorizontalPosition,

    /// Rotation angle in degrees (0-360)
    pub rotation_angle: f64,

    /// Whether this line is combined with others
    /// (when true, respects L/C/R positions within the combined line)
    pub is_combined: bool,

    /// Group ID for combined lines (lines with same group_id are combined)
    pub group_id: Option<String>,

    /// Horizontal offset in pixels for fine-tuning position
    #[serde(default)]
    pub offset_x: f64,

    /// Vertical offset in pixels for fine-tuning position
    #[serde(default)]
    pub offset_y: f64,
}

impl Default for TextLineConfig {
    fn default() -> Self {
        Self {
            field_id: String::new(),
            font_family: "Sans".to_string(),
            font_size: 12.0,
            bold: false,
            italic: false,
            color: (1.0, 1.0, 1.0, 1.0), // White
            vertical_position: VerticalPosition::Center,
            horizontal_position: HorizontalPosition::Center,
            rotation_angle: 0.0,
            is_combined: false,
            group_id: None,
            offset_x: 0.0,
            offset_y: 0.0,
        }
    }
}

/// Configuration for the text displayer
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TextDisplayerConfig {
    /// List of text lines to display
    pub lines: Vec<TextLineConfig>,
}

impl Default for TextDisplayerConfig {
    fn default() -> Self {
        Self {
            lines: vec![
                // Default configuration: show "Caption Value Unit" (e.g., "CPU 45.2 %")
                // Uses generic field names that work with any configured data source
                TextLineConfig {
                    field_id: "caption".to_string(),
                    font_family: "Sans".to_string(),
                    font_size: 14.0,
                    bold: false,
                    italic: false,
                    color: (1.0, 1.0, 1.0, 1.0),
                    vertical_position: VerticalPosition::Center,
                    horizontal_position: HorizontalPosition::Left,
                    rotation_angle: 0.0,
                    is_combined: true,
                    group_id: Some("main".to_string()),
                    offset_x: 0.0,
                    offset_y: 0.0,
                },
                TextLineConfig {
                    field_id: "value".to_string(),
                    font_family: "Sans".to_string(),
                    font_size: 14.0,
                    bold: false,
                    italic: false,
                    color: (0.5, 1.0, 0.5, 1.0), // Light green
                    vertical_position: VerticalPosition::Center,
                    horizontal_position: HorizontalPosition::Center,
                    rotation_angle: 0.0,
                    is_combined: true,
                    group_id: Some("main".to_string()),
                    offset_x: 0.0,
                    offset_y: 0.0,
                },
                TextLineConfig {
                    field_id: "unit".to_string(),
                    font_family: "Sans".to_string(),
                    font_size: 14.0,
                    bold: false,
                    italic: false,
                    color: (1.0, 1.0, 1.0, 1.0),
                    vertical_position: VerticalPosition::Center,
                    horizontal_position: HorizontalPosition::Right,
                    rotation_angle: 0.0,
                    is_combined: true,
                    group_id: Some("main".to_string()),
                    offset_x: 0.0,
                    offset_y: 0.0,
                },
            ],
        }
    }
}
