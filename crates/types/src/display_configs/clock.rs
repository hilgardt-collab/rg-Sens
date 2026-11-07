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

/// Clock numerals. `modern` and `numbers` were variants that drew exactly
/// like Classic; saved configs carrying them load as Classic.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum FaceStyle {
    #[serde(rename = "minimal")]
    Minimal,
    #[serde(rename = "classic", alias = "modern", alias = "numbers")]
    #[default]
    Classic,
    #[serde(rename = "roman")]
    Roman,
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
    /// The outline around the face
    #[serde(default = "default_true")]
    pub show_border: bool,
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
    /// Distance of the numerals from the centre, as a percentage of the radius
    #[serde(default = "default_number_radius_percent")]
    pub number_radius_percent: f64,
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

fn default_number_radius_percent() -> f64 {
    75.0
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
            show_border: true,
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
            number_radius_percent: default_number_radius_percent(),
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

impl crate::schema::Schema for AnalogClockConfig {
    fn props() -> Vec<crate::schema::Prop> {
        use crate::schema::{Condition, Prop, PropKind};

        let fraction = |key: &'static str, label: &'static str| {
            Prop::new(
                key,
                label,
                PropKind::Float {
                    min: 0.1,
                    max: 1.0,
                    step: 0.05,
                    digits: 2,
                },
            )
        };
        let px = |key: &'static str, label: &'static str, min: f64, max: f64| {
            Prop::new(
                key,
                label,
                PropKind::Float {
                    min,
                    max,
                    step: 0.5,
                    digits: 1,
                },
            )
        };
        let radius_percent = |key: &'static str, label: &'static str| {
            Prop::new(
                key,
                label,
                PropKind::Float {
                    min: 50.0,
                    max: 100.0,
                    step: 1.0,
                    digits: 0,
                },
            )
        };
        let tick_style = |key: &'static str, label: &'static str| {
            Prop::new(
                key,
                label,
                PropKind::Enum {
                    options: vec![
                        ("none", "None"),
                        ("squares", "Squares"),
                        ("lines", "Lines"),
                        ("dots", "Dots"),
                        ("triangles", "Triangles"),
                    ],
                },
            )
        };
        let hand_style = |key: &'static str, label: &'static str| {
            Prop::new(
                key,
                label,
                PropKind::Enum {
                    options: vec![
                        ("line", "Line"),
                        ("arrow", "Arrow"),
                        ("sword", "Sword"),
                        ("fancy", "Fancy"),
                    ],
                },
            )
        };
        let offset = |key: &'static str, label: &'static str| {
            Prop::new(
                key,
                label,
                PropKind::Float {
                    min: -100.0,
                    max: 100.0,
                    step: 1.0,
                    digits: 0,
                },
            )
        };
        let numbers = Condition::IsTrue {
            key: "show_numbers",
        };
        let second = Condition::IsTrue {
            key: "show_second_hand",
        };
        let hub = Condition::IsTrue {
            key: "show_center_hub",
        };
        let icon = Condition::IsTrue { key: "show_icon" };
        let border = Condition::IsTrue { key: "show_border" };

        vec![
            // Face
            Prop::new(
                "face_background",
                "Face background",
                PropKind::Custom {
                    editor: "background",
                },
            )
            .section("Face"),
            Prop::new("show_border", "Show outline", PropKind::Bool).section("Face"),
            Prop::new("border_color", "Outline colour", PropKind::Color)
                .section("Face")
                .visible_when(border.clone()),
            px("border_width", "Outline width (px)", 0.0, 20.0)
                .section("Face")
                .visible_when(border),
            // Numbers
            Prop::new("show_numbers", "Show numbers", PropKind::Bool).section("Numbers"),
            Prop::new(
                "face_style",
                "Numerals",
                PropKind::Enum {
                    options: vec![
                        ("classic", "Classic (1–12)"),
                        ("minimal", "Minimal (12, 3, 6, 9)"),
                        ("roman", "Roman"),
                    ],
                },
            )
            .section("Numbers")
            .visible_when(numbers.clone()),
            Prop::new(
                "number_radius_percent",
                "Distance from centre (% of radius)",
                PropKind::Float {
                    min: 30.0,
                    max: 100.0,
                    step: 1.0,
                    digits: 0,
                },
            )
            .section("Numbers")
            .visible_when(numbers.clone()),
            Prop::new(
                "number_font",
                "Font",
                PropKind::Custom {
                    editor: "radius_font",
                },
            )
            .section("Numbers")
            .help("Size as a percentage of the radius")
            .visible_when(numbers.clone()),
            Prop::new("number_bold", "Bold", PropKind::Bool)
                .section("Numbers")
                .visible_when(numbers.clone()),
            Prop::new("number_italic", "Italic", PropKind::Bool)
                .section("Numbers")
                .visible_when(numbers.clone()),
            Prop::new("number_color", "Colour", PropKind::Color)
                .section("Numbers")
                .visible_when(numbers),
            // Ticks
            tick_style("hour_tick_style", "Hour ticks").section("Ticks"),
            Prop::new("hour_tick_color", "Hour tick colour", PropKind::Color).section("Ticks"),
            radius_percent("hour_tick_outer_percent", "Hour tick outer radius (%)")
                .section("Ticks"),
            radius_percent("hour_tick_inner_percent", "Hour tick inner radius (%)")
                .section("Ticks"),
            tick_style("minute_tick_style", "Minute ticks").section("Ticks"),
            Prop::new("minute_tick_color", "Minute tick colour", PropKind::Color).section("Ticks"),
            radius_percent("minute_tick_outer_percent", "Minute tick outer radius (%)")
                .section("Ticks"),
            radius_percent("minute_tick_inner_percent", "Minute tick inner radius (%)")
                .section("Ticks"),
            // Hands
            hand_style("hour_hand_style", "Hour hand").section("Hands"),
            Prop::new("hour_hand_color", "Hour hand colour", PropKind::Color).section("Hands"),
            px("hour_hand_width", "Hour hand width (px)", 1.0, 20.0).section("Hands"),
            fraction("hour_hand_length", "Hour hand length (fraction of radius)").section("Hands"),
            hand_style("minute_hand_style", "Minute hand").section("Hands"),
            Prop::new("minute_hand_color", "Minute hand colour", PropKind::Color).section("Hands"),
            px("minute_hand_width", "Minute hand width (px)", 1.0, 20.0).section("Hands"),
            fraction(
                "minute_hand_length",
                "Minute hand length (fraction of radius)",
            )
            .section("Hands"),
            Prop::new("show_second_hand", "Show second hand", PropKind::Bool).section("Hands"),
            hand_style("second_hand_style", "Second hand")
                .section("Hands")
                .visible_when(second.clone()),
            Prop::new("second_hand_color", "Second hand colour", PropKind::Color)
                .section("Hands")
                .visible_when(second.clone()),
            px("second_hand_width", "Second hand width (px)", 0.5, 10.0)
                .section("Hands")
                .visible_when(second.clone()),
            fraction(
                "second_hand_length",
                "Second hand length (fraction of radius)",
            )
            .section("Hands")
            .visible_when(second.clone()),
            Prop::new("smooth_seconds", "Sweep the second hand", PropKind::Bool)
                .section("Hands")
                .help("Move continuously instead of ticking once a second")
                .visible_when(second),
            Prop::new("show_center_hub", "Show centre hub", PropKind::Bool).section("Hands"),
            Prop::new("center_hub_color", "Hub colour", PropKind::Color)
                .section("Hands")
                .visible_when(hub.clone()),
            Prop::new(
                "center_hub_size",
                "Hub size (fraction of radius)",
                PropKind::Float {
                    min: 0.01,
                    max: 0.2,
                    step: 0.005,
                    digits: 3,
                },
            )
            .section("Hands")
            .visible_when(hub),
            // Icon
            Prop::new("show_icon", "Show alarm/timer icon", PropKind::Bool).section("Icon"),
            Prop::new(
                "icon_text",
                "Icon text",
                PropKind::Text {
                    placeholder: "\u{23f1}\u{fe0f}",
                },
            )
            .section("Icon")
            .visible_when(icon.clone()),
            Prop::new(
                "icon_font",
                "Icon font",
                PropKind::PlainFont {
                    size_key: "icon_size",
                },
            )
            .section("Icon")
            .help("The size is a percentage of the panel's shorter side (5–30)")
            .visible_when(icon.clone()),
            Prop::new("icon_bold", "Bold icon", PropKind::Bool)
                .section("Icon")
                .visible_when(icon.clone()),
            Prop::new(
                "icon_position",
                "Icon position",
                PropKind::Custom {
                    editor: "text_position",
                },
            )
            .section("Icon")
            .visible_when(icon.clone()),
            offset("icon_offset_x", "Horizontal offset (px)")
                .section("Icon")
                .visible_when(icon.clone()),
            offset("icon_offset_y", "Vertical offset (px)")
                .section("Icon")
                .visible_when(icon.clone()),
            Prop::new(
                "shrink_for_indicator",
                "Shrink the face to make room",
                PropKind::Bool,
            )
            .section("Icon")
            .visible_when(icon),
        ]
    }
}

#[cfg(test)]
mod schema_tests {
    use super::*;
    use crate::schema::{check_props_fit, Schema};

    #[test]
    fn props_fit_the_type() {
        check_props_fit::<AnalogClockConfig>(&AnalogClockConfig::props()).unwrap();
    }

    #[test]
    fn default_round_trips_through_json() {
        let value = serde_json::to_value(AnalogClockConfig::default()).unwrap();
        let back: AnalogClockConfig = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(serde_json::to_value(back).unwrap(), value);
    }

    /// Two numeral styles that drew like Classic were dropped from the
    /// dropdown; configs saved with them load as Classic, and a config saved
    /// before the outline toggle existed keeps its outline.
    #[test]
    fn retired_numeral_styles_load_as_classic() {
        for old in ["modern", "numbers"] {
            let cfg: AnalogClockConfig =
                serde_json::from_value(serde_json::json!({ "face_style": old })).unwrap();
            assert_eq!(cfg.face_style, FaceStyle::Classic, "{}", old);
        }
        let cfg: AnalogClockConfig = serde_json::from_value(serde_json::json!({})).unwrap();
        assert!(cfg.show_border);
        assert_eq!(cfg.number_radius_percent, 75.0);
    }

    /// The deprecated `center_indicator` has no row; it must still survive
    /// a load/store cycle because the editor keeps the whole object.
    #[test]
    fn unlisted_keys_are_not_schema_keys() {
        let keys: Vec<&str> = AnalogClockConfig::props().iter().map(|p| p.key).collect();
        assert!(!keys.contains(&"center_indicator"));
        assert!(keys.contains(&"shrink_for_indicator"));
    }
}
