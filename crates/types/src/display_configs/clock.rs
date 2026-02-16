//! Analog clock display configuration types

use serde::{Deserialize, Serialize};

use crate::background::BackgroundConfig;
use crate::color::Color;
use crate::text::TextPosition;
use crate::theme::{ColorSource, FontSource};
use crate::BackgroundType;

/// Clock hand style
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum HandStyle {
    #[serde(rename = "line")]
    #[default]
    Line,
    #[serde(rename = "arrow")]
    Arrow,
    #[serde(rename = "sword")]
    Sword,
    #[serde(rename = "fancy")]
    Fancy,
}

/// Clock face style
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum FaceStyle {
    #[serde(rename = "minimal")]
    Minimal,
    #[serde(rename = "classic")]
    #[default]
    Classic,
    #[serde(rename = "modern")]
    Modern,
    #[serde(rename = "roman")]
    Roman,
    #[serde(rename = "numbers")]
    Numbers,
}

/// Clock tick mark style
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum TickStyle {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "squares")]
    Squares,
    #[serde(rename = "lines")]
    #[default]
    Lines,
    #[serde(rename = "dots")]
    Dots,
    #[serde(rename = "triangles")]
    Triangles,
}

/// Analog clock display configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalogClockConfig {
    // Face background (uses common background system)
    #[serde(default)]
    pub face_background: BackgroundConfig,
    #[serde(default)]
    pub face_style: FaceStyle,
    #[serde(default = "default_border_color")]
    pub border_color: ColorSource,
    #[serde(default = "default_border_width")]
    pub border_width: f64,

    // Hour tick marks
    #[serde(default)]
    pub hour_tick_style: TickStyle,
    #[serde(default = "default_tick_color")]
    pub hour_tick_color: ColorSource,
    #[serde(default = "default_hour_tick_outer_percent")]
    pub hour_tick_outer_percent: f64,
    #[serde(default = "default_hour_tick_inner_percent")]
    pub hour_tick_inner_percent: f64,

    // Minute tick marks
    #[serde(default)]
    pub minute_tick_style: TickStyle,
    #[serde(default = "default_tick_color")]
    pub minute_tick_color: ColorSource,
    #[serde(default = "default_minute_tick_outer_percent")]
    pub minute_tick_outer_percent: f64,
    #[serde(default = "default_minute_tick_inner_percent")]
    pub minute_tick_inner_percent: f64,

    // Numbers
    #[serde(default = "default_true")]
    pub show_numbers: bool,
    #[serde(default = "default_number_color")]
    pub number_color: ColorSource,
    #[serde(default = "default_number_font")]
    pub number_font: FontSource,
    #[serde(default)]
    pub number_bold: bool,
    #[serde(default)]
    pub number_italic: bool,

    // Hour hand
    #[serde(default)]
    pub hour_hand_style: HandStyle,
    #[serde(default = "default_hour_hand_color")]
    pub hour_hand_color: ColorSource,
    #[serde(default = "default_hour_hand_width")]
    pub hour_hand_width: f64,
    #[serde(default = "default_hour_hand_length")]
    pub hour_hand_length: f64,

    // Minute hand
    #[serde(default)]
    pub minute_hand_style: HandStyle,
    #[serde(default = "default_minute_hand_color")]
    pub minute_hand_color: ColorSource,
    #[serde(default = "default_minute_hand_width")]
    pub minute_hand_width: f64,
    #[serde(default = "default_minute_hand_length")]
    pub minute_hand_length: f64,

    // Second hand
    #[serde(default = "default_true")]
    pub show_second_hand: bool,
    #[serde(default)]
    pub second_hand_style: HandStyle,
    #[serde(default = "default_second_hand_color")]
    pub second_hand_color: ColorSource,
    #[serde(default = "default_second_hand_width")]
    pub second_hand_width: f64,
    #[serde(default = "default_second_hand_length")]
    pub second_hand_length: f64,

    // Center hub
    #[serde(default = "default_true")]
    pub show_center_hub: bool,
    #[serde(default = "default_center_hub_color")]
    pub center_hub_color: ColorSource,
    #[serde(default = "default_center_hub_size")]
    pub center_hub_size: f64,

    // Smooth movement
    #[serde(default = "default_true")]
    pub smooth_seconds: bool,

    // Alarm/Timer icon
    #[serde(default = "default_true")]
    pub show_icon: bool,
    #[serde(default = "default_icon_text")]
    pub icon_text: String,
    #[serde(default = "default_icon_font")]
    pub icon_font: String,
    #[serde(default = "default_icon_size")]
    pub icon_size: f64, // As percentage of panel size (5-30%)
    #[serde(default)]
    pub icon_bold: bool,

    // Indicator layout options
    #[serde(default = "default_true")]
    pub center_indicator: bool, // DEPRECATED: Use icon_position instead
    #[serde(default = "default_true")]
    pub shrink_for_indicator: bool, // Shrink clock face when indicator is visible

    // New icon positioning (3x3 grid with offset)
    #[serde(default = "default_icon_position")]
    pub icon_position: TextPosition,
    #[serde(default)]
    pub icon_offset_x: f64, // Pixels offset from calculated position
    #[serde(default)]
    pub icon_offset_y: f64, // Pixels offset from calculated position
}

fn default_face_color() -> Color {
    Color::new(0.15, 0.15, 0.15, 1.0)
}

fn default_border_color() -> ColorSource {
    ColorSource::custom(Color::new(0.5, 0.5, 0.5, 1.0))
}

fn default_border_width() -> f64 {
    3.0
}

fn default_tick_color() -> ColorSource {
    ColorSource::custom(Color::new(0.7, 0.7, 0.7, 1.0))
}

fn default_hour_tick_outer_percent() -> f64 {
    95.0 // As percentage of radius
}

fn default_hour_tick_inner_percent() -> f64 {
    85.0
}

fn default_minute_tick_outer_percent() -> f64 {
    95.0
}

fn default_minute_tick_inner_percent() -> f64 {
    90.0
}

fn default_true() -> bool {
    true
}

fn default_number_color() -> ColorSource {
    ColorSource::custom(Color::new(0.9, 0.9, 0.9, 1.0))
}

fn default_number_font() -> FontSource {
    FontSource::custom("Sans".to_string(), 0.12) // Size as fraction of radius
}

fn default_hour_hand_color() -> ColorSource {
    ColorSource::custom(Color::new(0.9, 0.9, 0.9, 1.0))
}

fn default_hour_hand_width() -> f64 {
    6.0
}

fn default_hour_hand_length() -> f64 {
    0.5 // As fraction of radius
}

fn default_minute_hand_color() -> ColorSource {
    ColorSource::custom(Color::new(0.9, 0.9, 0.9, 1.0))
}

fn default_minute_hand_width() -> f64 {
    4.0
}

fn default_minute_hand_length() -> f64 {
    0.75
}

fn default_second_hand_color() -> ColorSource {
    ColorSource::custom(Color::new(1.0, 0.3, 0.3, 1.0))
}

fn default_second_hand_width() -> f64 {
    2.0
}

fn default_second_hand_length() -> f64 {
    0.85
}

fn default_center_hub_color() -> ColorSource {
    ColorSource::custom(Color::new(0.8, 0.8, 0.8, 1.0))
}

fn default_center_hub_size() -> f64 {
    0.05 // As fraction of radius
}

fn default_icon_text() -> String {
    "\u{23f1}\u{fe0f}".to_string() // ⏱️
}

fn default_icon_font() -> String {
    "Noto Color Emoji".to_string()
}

fn default_icon_size() -> f64 {
    12.0 // As percentage of panel size
}

fn default_icon_position() -> TextPosition {
    TextPosition::BottomCenter
}

impl Default for AnalogClockConfig {
    fn default() -> Self {
        // Create a default circular background with dark color
        let face_background = BackgroundConfig {
            background: BackgroundType::Solid {
                color: ColorSource::custom(default_face_color()),
            },
        };

        Self {
            face_background,
            face_style: FaceStyle::default(),
            border_color: default_border_color(),
            border_width: default_border_width(),
            hour_tick_style: TickStyle::default(),
            hour_tick_color: default_tick_color(),
            hour_tick_outer_percent: default_hour_tick_outer_percent(),
            hour_tick_inner_percent: default_hour_tick_inner_percent(),
            minute_tick_style: TickStyle::default(),
            minute_tick_color: default_tick_color(),
            minute_tick_outer_percent: default_minute_tick_outer_percent(),
            minute_tick_inner_percent: default_minute_tick_inner_percent(),
            show_numbers: true,
            number_color: default_number_color(),
            number_font: default_number_font(),
            number_bold: true,
            number_italic: false,
            hour_hand_style: HandStyle::default(),
            hour_hand_color: default_hour_hand_color(),
            hour_hand_width: default_hour_hand_width(),
            hour_hand_length: default_hour_hand_length(),
            minute_hand_style: HandStyle::default(),
            minute_hand_color: default_minute_hand_color(),
            minute_hand_width: default_minute_hand_width(),
            minute_hand_length: default_minute_hand_length(),
            show_second_hand: true,
            second_hand_style: HandStyle::default(),
            second_hand_color: default_second_hand_color(),
            second_hand_width: default_second_hand_width(),
            second_hand_length: default_second_hand_length(),
            show_center_hub: true,
            center_hub_color: default_center_hub_color(),
            center_hub_size: default_center_hub_size(),
            smooth_seconds: true,
            show_icon: true,
            icon_text: default_icon_text(),
            icon_font: default_icon_font(),
            icon_size: default_icon_size(),
            icon_bold: false,
            center_indicator: true,
            shrink_for_indicator: true,
            icon_position: default_icon_position(),
            icon_offset_x: 0.0,
            icon_offset_y: 0.0,
        }
    }
}
