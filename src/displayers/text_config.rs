//! Configuration for text displayer

use crate::ui::background::{Color, ColorStop};
use crate::ui::theme::{ColorSource, ColorStopSource, ComboThemeConfig, FontSource};
use serde::{Deserialize, Serialize};

/// Vertical position of text
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub enum VerticalPosition {
    Top,
    #[default]
    Center,
    Bottom,
}

/// Horizontal position of text
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub enum HorizontalPosition {
    Left,
    #[default]
    Center,
    Right,
}

/// Combined position for 3x3 grid selection
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub enum TextPosition {
    TopLeft,
    TopCenter,
    TopRight,
    CenterLeft,
    #[default]
    Center,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

impl TextPosition {
    /// Convert to (VerticalPosition, HorizontalPosition) tuple
    pub fn to_positions(&self) -> (VerticalPosition, HorizontalPosition) {
        match self {
            TextPosition::TopLeft => (VerticalPosition::Top, HorizontalPosition::Left),
            TextPosition::TopCenter => (VerticalPosition::Top, HorizontalPosition::Center),
            TextPosition::TopRight => (VerticalPosition::Top, HorizontalPosition::Right),
            TextPosition::CenterLeft => (VerticalPosition::Center, HorizontalPosition::Left),
            TextPosition::Center => (VerticalPosition::Center, HorizontalPosition::Center),
            TextPosition::CenterRight => (VerticalPosition::Center, HorizontalPosition::Right),
            TextPosition::BottomLeft => (VerticalPosition::Bottom, HorizontalPosition::Left),
            TextPosition::BottomCenter => (VerticalPosition::Bottom, HorizontalPosition::Center),
            TextPosition::BottomRight => (VerticalPosition::Bottom, HorizontalPosition::Right),
        }
    }

    /// Create from (VerticalPosition, HorizontalPosition)
    pub fn from_positions(v: VerticalPosition, h: HorizontalPosition) -> Self {
        match (v, h) {
            (VerticalPosition::Top, HorizontalPosition::Left) => TextPosition::TopLeft,
            (VerticalPosition::Top, HorizontalPosition::Center) => TextPosition::TopCenter,
            (VerticalPosition::Top, HorizontalPosition::Right) => TextPosition::TopRight,
            (VerticalPosition::Center, HorizontalPosition::Left) => TextPosition::CenterLeft,
            (VerticalPosition::Center, HorizontalPosition::Center) => TextPosition::Center,
            (VerticalPosition::Center, HorizontalPosition::Right) => TextPosition::CenterRight,
            (VerticalPosition::Bottom, HorizontalPosition::Left) => TextPosition::BottomLeft,
            (VerticalPosition::Bottom, HorizontalPosition::Center) => TextPosition::BottomCenter,
            (VerticalPosition::Bottom, HorizontalPosition::Right) => TextPosition::BottomRight,
        }
    }
}

/// Direction for combining text lines
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub enum CombineDirection {
    #[default]
    Horizontal, // Lines flow left-to-right (existing behavior)
    Vertical,   // Lines stack top-to-bottom
}

/// Legacy alignment for combined text groups (kept for backward compatibility)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub enum CombineAlignment {
    Start,  // Left for horizontal, Top for vertical
    #[default]
    Center, // Center alignment
    End,    // Right for horizontal, Bottom for vertical
}

impl CombineAlignment {
    /// Convert to TextPosition (centered on the relevant axis)
    pub fn to_text_position(&self, direction: CombineDirection) -> TextPosition {
        match direction {
            CombineDirection::Horizontal => {
                // For horizontal, alignment is vertical (top/center/bottom)
                match self {
                    CombineAlignment::Start => TextPosition::TopCenter,
                    CombineAlignment::Center => TextPosition::Center,
                    CombineAlignment::End => TextPosition::BottomCenter,
                }
            }
            CombineDirection::Vertical => {
                // For vertical, alignment is horizontal (left/center/right)
                match self {
                    CombineAlignment::Start => TextPosition::CenterLeft,
                    CombineAlignment::Center => TextPosition::Center,
                    CombineAlignment::End => TextPosition::CenterRight,
                }
            }
        }
    }
}

/// Text fill type (solid color or gradient)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum TextFillType {
    #[serde(rename = "solid")]
    Solid { color: ColorSource },
    #[serde(rename = "linear_gradient")]
    LinearGradient {
        stops: Vec<ColorStop>,
        angle: f64, // Angle in degrees, relative to text baseline
    },
}

impl Default for TextFillType {
    fn default() -> Self {
        TextFillType::Solid {
            color: ColorSource::Custom {
                color: Color::new(1.0, 1.0, 1.0, 1.0),
            },
        }
    }
}

impl TextFillType {
    /// Get the primary color resolved against a theme (for compatibility or fallback)
    pub fn primary_color(&self, theme: Option<&ComboThemeConfig>) -> Color {
        match self {
            TextFillType::Solid { color } => {
                if let Some(theme) = theme {
                    color.resolve(theme)
                } else {
                    // Fallback for when no theme is available
                    match color {
                        ColorSource::Custom { color } => *color,
                        ColorSource::Theme { index } => {
                            // Return a default based on theme index
                            match index {
                                1 => Color::new(1.0, 0.5, 0.0, 1.0), // Orange
                                2 => Color::new(0.0, 0.8, 1.0, 1.0), // Cyan
                                3 => Color::new(1.0, 0.0, 0.5, 1.0), // Pink
                                _ => Color::new(0.5, 1.0, 0.0, 1.0), // Green
                            }
                        }
                    }
                }
            }
            TextFillType::LinearGradient { stops, .. } => {
                stops.first().map(|s| s.color).unwrap_or(Color::new(1.0, 1.0, 1.0, 1.0))
            }
        }
    }

    /// Get the color source (for UI)
    pub fn color_source(&self) -> ColorSource {
        match self {
            TextFillType::Solid { color } => color.clone(),
            TextFillType::LinearGradient { stops, .. } => {
                let c = stops.first().map(|s| s.color).unwrap_or(Color::new(1.0, 1.0, 1.0, 1.0));
                ColorSource::Custom { color: c }
            }
        }
    }

    /// Create from legacy color tuple
    pub fn from_color_tuple(r: f64, g: f64, b: f64, a: f64) -> Self {
        TextFillType::Solid {
            color: ColorSource::Custom {
                color: Color::new(r, g, b, a),
            },
        }
    }

    /// Create from color source
    pub fn from_color_source(source: ColorSource) -> Self {
        TextFillType::Solid { color: source }
    }
}

/// Text background type (simplified from BackgroundType)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(tag = "type")]
pub enum TextBackgroundType {
    #[serde(rename = "none")]
    #[default]
    None,
    #[serde(rename = "solid")]
    Solid { color: ColorSource },
    #[serde(rename = "linear_gradient")]
    LinearGradient {
        stops: Vec<ColorStopSource>,
        angle: f64,
    },
}

impl TextBackgroundType {
    /// Resolve the background color against a theme
    pub fn resolved_color(&self, theme: Option<&ComboThemeConfig>) -> Option<Color> {
        match self {
            TextBackgroundType::None => None,
            TextBackgroundType::Solid { color } => {
                Some(if let Some(theme) = theme {
                    color.resolve(theme)
                } else {
                    match color {
                        ColorSource::Custom { color } => *color,
                        ColorSource::Theme { index } => {
                            // Fallback colors when no theme
                            match index {
                                1 => Color::new(1.0, 0.5, 0.0, 0.5),
                                2 => Color::new(0.0, 0.8, 1.0, 0.5),
                                3 => Color::new(1.0, 0.0, 0.5, 0.5),
                                _ => Color::new(0.5, 1.0, 0.0, 0.5),
                            }
                        }
                    }
                })
            }
            TextBackgroundType::LinearGradient { stops, .. } => {
                stops.first().map(|s| s.color.resolve(theme.unwrap_or(&ComboThemeConfig::default())))
            }
        }
    }

    /// Check if this background type is None
    pub fn is_none(&self) -> bool {
        matches!(self, TextBackgroundType::None)
    }

    /// Resolve the gradient to concrete colors using theme
    pub fn resolve_gradient(&self, theme: Option<&ComboThemeConfig>) -> Option<(Vec<ColorStop>, f64)> {
        match self {
            TextBackgroundType::LinearGradient { stops, angle } => {
                let default_theme = ComboThemeConfig::default();
                let theme_ref = theme.unwrap_or(&default_theme);
                let resolved_stops: Vec<ColorStop> = stops
                    .iter()
                    .map(|s| s.resolve(theme_ref))
                    .collect();
                Some((resolved_stops, *angle))
            }
            _ => None,
        }
    }
}

fn default_bg_padding() -> f64 {
    4.0
}
fn default_bg_corner_radius() -> f64 {
    0.0
}

/// Background configuration for text
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TextBackgroundConfig {
    pub background: TextBackgroundType,
    #[serde(default = "default_bg_padding")]
    pub padding: f64, // Padding around text in pixels
    #[serde(default = "default_bg_corner_radius")]
    pub corner_radius: f64, // Corner radius for rounded background
}

impl Default for TextBackgroundConfig {
    fn default() -> Self {
        Self {
            background: TextBackgroundType::None,
            padding: default_bg_padding(),
            corner_radius: default_bg_corner_radius(),
        }
    }
}

/// Configuration for a single line of text
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TextLineConfig {
    /// ID of the field to display (e.g., "caption", "usage", "unit")
    pub field_id: String,

    /// Font family (e.g., "Sans", "Monospace") - used when font_source is None or Custom
    pub font_family: String,

    /// Font size in points - used when font_source is None or Custom
    pub font_size: f64,

    /// Font source (theme or custom) - when Some, overrides font_family/font_size
    #[serde(default)]
    pub font_source: Option<FontSource>,

    /// Whether the font is bold
    #[serde(default)]
    pub bold: bool,

    /// Whether the font is italic
    #[serde(default)]
    pub italic: bool,

    /// Text fill (solid color or gradient)
    #[serde(default)]
    pub fill: TextFillType,

    /// Legacy color field - kept for backward compatibility during deserialization
    #[serde(default, skip_serializing, rename = "color")]
    legacy_color: Option<(f64, f64, f64, f64)>,

    /// Text background configuration
    #[serde(default)]
    pub text_background: TextBackgroundConfig,

    /// Combined position (replaces vertical_position + horizontal_position)
    #[serde(default)]
    pub position: TextPosition,

    /// Legacy vertical position - for backward compatibility
    #[serde(default, skip_serializing, rename = "vertical_position")]
    legacy_vertical_position: Option<VerticalPosition>,

    /// Legacy horizontal position - for backward compatibility
    #[serde(default, skip_serializing, rename = "horizontal_position")]
    legacy_horizontal_position: Option<HorizontalPosition>,

    /// Rotation angle in degrees (0-360)
    #[serde(default)]
    pub rotation_angle: f64,

    /// Whether this line is combined with others
    #[serde(default)]
    pub is_combined: bool,

    /// Group ID for combined lines
    #[serde(default)]
    pub group_id: Option<String>,

    /// Direction for combining (horizontal or vertical) - used from first line in group
    #[serde(default)]
    pub combine_direction: CombineDirection,

    /// Alignment within combined group (as TextPosition) - used from first line in group
    #[serde(default)]
    pub combine_alignment: TextPosition,

    /// Legacy alignment field for backward compatibility
    #[serde(default, skip_serializing, rename = "legacy_combine_alignment")]
    legacy_combine_alignment: Option<CombineAlignment>,

    /// Horizontal offset in pixels
    #[serde(default)]
    pub offset_x: f64,

    /// Vertical offset in pixels
    #[serde(default)]
    pub offset_y: f64,
}

impl TextLineConfig {
    /// Apply post-deserialization migrations for legacy fields
    pub fn migrate(&mut self) {
        // Migrate legacy color to fill
        if let Some((r, g, b, a)) = self.legacy_color.take() {
            // Only override if fill is still default white
            if self.fill == TextFillType::default() {
                self.fill = TextFillType::from_color_tuple(r, g, b, a);
            }
        }

        // Migrate legacy positions to combined position
        if let (Some(v), Some(h)) = (
            self.legacy_vertical_position.take(),
            self.legacy_horizontal_position.take(),
        ) {
            self.position = TextPosition::from_positions(v, h);
        }

        // Migrate legacy combine_alignment to TextPosition
        if let Some(legacy_align) = self.legacy_combine_alignment.take() {
            self.combine_alignment = legacy_align.to_text_position(self.combine_direction);
        }

        // Migrate old font_family/font_size to font_source if not already set
        if self.font_source.is_none() {
            self.font_source = Some(FontSource::Custom {
                family: self.font_family.clone(),
                size: self.font_size,
            });
        }
    }

    /// Get vertical position (for compatibility with existing code)
    pub fn vertical_position(&self) -> VerticalPosition {
        self.position.to_positions().0
    }

    /// Get horizontal position (for compatibility with existing code)
    pub fn horizontal_position(&self) -> HorizontalPosition {
        self.position.to_positions().1
    }

    /// Get color as tuple resolved against theme (for compatibility with existing code)
    pub fn color(&self) -> (f64, f64, f64, f64) {
        let c = self.fill.primary_color(None);
        (c.r, c.g, c.b, c.a)
    }

    /// Get color resolved against a theme
    pub fn resolved_color(&self, theme: Option<&ComboThemeConfig>) -> Color {
        self.fill.primary_color(theme)
    }

    /// Get the font source (for UI). Returns current font_source or creates Custom from legacy fields.
    pub fn get_font_source(&self) -> FontSource {
        self.font_source.clone().unwrap_or_else(|| FontSource::Custom {
            family: self.font_family.clone(),
            size: self.font_size,
        })
    }

    /// Resolve font against a theme, returning (family, size)
    pub fn resolved_font(&self, theme: Option<&ComboThemeConfig>) -> (String, f64) {
        match &self.font_source {
            Some(source) => {
                if let Some(theme) = theme {
                    source.resolve(theme)
                } else {
                    // Fallback for when no theme is available
                    match source {
                        FontSource::Custom { family, size } => (family.clone(), *size),
                        FontSource::Theme { index, size } => {
                            // Return a default font based on theme index, use stored size
                            match index {
                                1 => ("Sans Bold".to_string(), *size),
                                _ => ("Sans".to_string(), *size),
                            }
                        }
                    }
                }
            }
            None => (self.font_family.clone(), self.font_size),
        }
    }
}

impl Default for TextLineConfig {
    fn default() -> Self {
        Self {
            field_id: String::new(),
            font_family: "Sans".to_string(),
            font_size: 12.0,
            font_source: Some(FontSource::Custom {
                family: "Sans".to_string(),
                size: 12.0,
            }),
            bold: false,
            italic: false,
            fill: TextFillType::default(),
            legacy_color: None,
            text_background: TextBackgroundConfig::default(),
            position: TextPosition::Center,
            legacy_vertical_position: None,
            legacy_horizontal_position: None,
            rotation_angle: 0.0,
            is_combined: false,
            group_id: None,
            combine_direction: CombineDirection::default(),
            combine_alignment: TextPosition::Center,
            legacy_combine_alignment: None,
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

impl TextDisplayerConfig {
    /// Apply migrations to all lines after deserialization
    pub fn migrate(&mut self) {
        for line in &mut self.lines {
            line.migrate();
        }
    }
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
                    font_source: Some(FontSource::Custom {
                        family: "Sans".to_string(),
                        size: 14.0,
                    }),
                    bold: false,
                    italic: false,
                    fill: TextFillType::Solid {
                        color: ColorSource::Custom {
                            color: Color::new(1.0, 1.0, 1.0, 1.0),
                        },
                    },
                    legacy_color: None,
                    text_background: TextBackgroundConfig::default(),
                    position: TextPosition::CenterLeft,
                    legacy_vertical_position: None,
                    legacy_horizontal_position: None,
                    rotation_angle: 0.0,
                    is_combined: true,
                    group_id: Some("main".to_string()),
                    combine_direction: CombineDirection::Horizontal,
                    combine_alignment: TextPosition::Center,
                    legacy_combine_alignment: None,
                    offset_x: 0.0,
                    offset_y: 0.0,
                },
                TextLineConfig {
                    field_id: "value".to_string(),
                    font_family: "Sans".to_string(),
                    font_size: 14.0,
                    font_source: Some(FontSource::Custom {
                        family: "Sans".to_string(),
                        size: 14.0,
                    }),
                    bold: false,
                    italic: false,
                    fill: TextFillType::Solid {
                        color: ColorSource::Custom {
                            color: Color::new(0.5, 1.0, 0.5, 1.0), // Light green
                        },
                    },
                    legacy_color: None,
                    text_background: TextBackgroundConfig::default(),
                    position: TextPosition::Center,
                    legacy_vertical_position: None,
                    legacy_horizontal_position: None,
                    rotation_angle: 0.0,
                    is_combined: true,
                    group_id: Some("main".to_string()),
                    combine_direction: CombineDirection::Horizontal,
                    combine_alignment: TextPosition::Center,
                    legacy_combine_alignment: None,
                    offset_x: 0.0,
                    offset_y: 0.0,
                },
                TextLineConfig {
                    field_id: "unit".to_string(),
                    font_family: "Sans".to_string(),
                    font_size: 14.0,
                    font_source: Some(FontSource::Custom {
                        family: "Sans".to_string(),
                        size: 14.0,
                    }),
                    bold: false,
                    italic: false,
                    fill: TextFillType::Solid {
                        color: ColorSource::Custom {
                            color: Color::new(1.0, 1.0, 1.0, 1.0),
                        },
                    },
                    legacy_color: None,
                    text_background: TextBackgroundConfig::default(),
                    position: TextPosition::CenterRight,
                    legacy_vertical_position: None,
                    legacy_horizontal_position: None,
                    rotation_angle: 0.0,
                    is_combined: true,
                    group_id: Some("main".to_string()),
                    combine_direction: CombineDirection::Horizontal,
                    combine_alignment: TextPosition::Center,
                    legacy_combine_alignment: None,
                    offset_x: 0.0,
                    offset_y: 0.0,
                },
            ],
        }
    }
}
