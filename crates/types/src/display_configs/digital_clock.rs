//! Digital clock display configuration types

use serde::{Deserialize, Serialize};

use crate::color::Color;

/// Digital clock display style
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum DigitalStyle {
    #[serde(rename = "simple")]
    #[default]
    Simple,
    #[serde(rename = "segment")]
    Segment, // 7-segment LED style
    #[serde(rename = "lcd")]
    LCD,
}

/// Digital clock configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DigitalClockConfig {
    #[serde(default)]
    pub style: DigitalStyle,

    // Time display
    #[serde(default = "default_time_font")]
    pub time_font: String,
    #[serde(default = "default_time_size")]
    pub time_size: f64,
    #[serde(default = "default_time_color")]
    pub time_color: Color,
    #[serde(default = "default_true")]
    pub time_bold: bool,
    #[serde(default)]
    pub time_italic: bool,

    // Date display
    #[serde(default = "default_true")]
    pub show_date: bool,
    #[serde(default = "default_date_font")]
    pub date_font: String,
    #[serde(default = "default_date_size")]
    pub date_size: f64,
    #[serde(default = "default_date_color")]
    pub date_color: Color,
    #[serde(default)]
    pub date_bold: bool,
    #[serde(default)]
    pub date_italic: bool,

    // Day name
    #[serde(default)]
    pub show_day_name: bool,

    // Timer display
    #[serde(default)]
    pub show_timer: bool,
    #[serde(default = "default_timer_color")]
    pub timer_color: Color,

    // Alarm indicator
    #[serde(default = "default_true")]
    pub show_alarm_indicator: bool,
    #[serde(default = "default_alarm_color")]
    pub alarm_color: Color,

    // Blinking colon
    #[serde(default = "default_true")]
    pub blink_colon: bool,

    // Vertical layout
    #[serde(default)]
    pub vertical_layout: bool,

    // Alarm/Timer icon
    #[serde(default = "default_true")]
    pub show_icon: bool,
    #[serde(default = "default_icon_text")]
    pub icon_text: String,
    #[serde(default = "default_icon_font")]
    pub icon_font: String,
    #[serde(default = "default_icon_size")]
    pub icon_size: f64, // In pixels
    #[serde(default)]
    pub icon_bold: bool,
}

fn default_time_font() -> String {
    "Monospace".to_string()
}

fn default_time_size() -> f64 {
    48.0
}

fn default_time_color() -> Color {
    Color::new(0.9, 0.9, 0.9, 1.0)
}

fn default_true() -> bool {
    true
}

fn default_date_font() -> String {
    "Sans".to_string()
}

fn default_date_size() -> f64 {
    16.0
}

fn default_date_color() -> Color {
    Color::new(0.7, 0.7, 0.7, 1.0)
}

fn default_timer_color() -> Color {
    Color::new(0.3, 0.8, 0.3, 1.0)
}

fn default_alarm_color() -> Color {
    Color::new(1.0, 0.3, 0.3, 1.0)
}

fn default_icon_text() -> String {
    "\u{23f1}\u{fe0f}".to_string() // stopwatch emoji
}

fn default_icon_font() -> String {
    "Sans".to_string()
}

fn default_icon_size() -> f64 {
    16.0 // In pixels
}

impl Default for DigitalClockConfig {
    fn default() -> Self {
        Self {
            style: DigitalStyle::Simple,
            time_font: default_time_font(),
            time_size: default_time_size(),
            time_color: default_time_color(),
            time_bold: true,
            time_italic: false,
            show_date: true,
            date_font: default_date_font(),
            date_size: default_date_size(),
            date_color: default_date_color(),
            date_bold: false,
            date_italic: false,
            show_day_name: false,
            show_timer: false,
            timer_color: default_timer_color(),
            show_alarm_indicator: true,
            alarm_color: default_alarm_color(),
            blink_colon: true,
            vertical_layout: false,
            show_icon: true,
            icon_text: default_icon_text(),
            icon_font: default_icon_font(),
            icon_size: default_icon_size(),
            icon_bold: false,
        }
    }
}
