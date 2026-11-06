//! Core bars display configuration types

use serde::{Deserialize, Serialize};

use crate::text::TextOverlayConfig;
use crate::theme::{deserialize_color_or_source, ColorSource};

use super::bar::{
    BarBackgroundType, BarFillDirection, BarFillType, BarOrientation, BarStyle, BorderConfig,
};

/// Label position relative to the bar
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum LabelPosition {
    #[serde(rename = "start")]
    #[default]
    Start, // Left for horizontal, Top for vertical
    #[serde(rename = "end")]
    End, // Right for horizontal, Bottom for vertical
    #[serde(rename = "inside")]
    Inside, // Inside the bar
}

/// Core bars display configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CoreBarsConfig {
    // Core selection
    #[serde(default)]
    pub start_core: usize,
    #[serde(default = "default_end_core")]
    pub end_core: usize,

    // Padding
    #[serde(default)]
    pub padding_top: f64,
    #[serde(default)]
    pub padding_bottom: f64,
    #[serde(default)]
    pub padding_left: f64,
    #[serde(default)]
    pub padding_right: f64,

    // Bar styling (unified for all bars)
    #[serde(default)]
    pub bar_style: BarStyle,
    #[serde(default)]
    pub orientation: BarOrientation,
    #[serde(default)]
    pub fill_direction: BarFillDirection,
    #[serde(default)]
    pub foreground: BarFillType,
    #[serde(default)]
    pub background: BarBackgroundType,
    #[serde(default = "default_corner_radius")]
    pub corner_radius: f64,
    #[serde(default = "default_bar_spacing")]
    pub bar_spacing: f64,

    // Segmented bar options
    #[serde(default = "default_segment_count")]
    pub segment_count: u32,
    #[serde(default = "default_segment_spacing")]
    pub segment_spacing: f64,

    // Border
    #[serde(default)]
    pub border: BorderConfig,

    // Labels
    #[serde(default = "default_true")]
    pub show_labels: bool,
    #[serde(default = "default_label_prefix")]
    pub label_prefix: String,
    #[serde(default)]
    pub label_position: LabelPosition,
    #[serde(default = "default_label_font")]
    pub label_font: String,
    #[serde(default = "default_label_size")]
    pub label_size: f64,
    #[serde(
        default = "default_label_color",
        deserialize_with = "deserialize_color_or_source"
    )]
    pub label_color: ColorSource,
    #[serde(default)]
    pub label_bold: bool,

    // Animation
    #[serde(default = "default_true")]
    pub animate: bool,
    #[serde(default = "default_animation_speed")]
    pub animation_speed: f64,

    // Gradient across bars - when true, gradient colors span across all bars
    // (each bar is a solid color sampled from gradient position)
    #[serde(default)]
    pub gradient_spans_bars: bool,

    // Text overlay
    #[serde(default)]
    pub text_overlay: TextOverlayConfig,
}

fn default_end_core() -> usize {
    15 // Default to 16 cores (0-15)
}

fn default_corner_radius() -> f64 {
    3.0
}

fn default_bar_spacing() -> f64 {
    4.0
}

fn default_segment_count() -> u32 {
    10
}

fn default_segment_spacing() -> f64 {
    1.0
}

fn default_true() -> bool {
    true
}

fn default_label_prefix() -> String {
    "".to_string()
}

fn default_label_font() -> String {
    "Sans".to_string()
}

fn default_label_size() -> f64 {
    10.0
}

fn default_label_color() -> ColorSource {
    // Default to Theme Color 3 (typically text/accent color)
    ColorSource::Theme { index: 3 }
}

fn default_animation_speed() -> f64 {
    8.0
}

impl Default for CoreBarsConfig {
    fn default() -> Self {
        Self {
            start_core: 0,
            end_core: default_end_core(),
            padding_top: 0.0,
            padding_bottom: 0.0,
            padding_left: 0.0,
            padding_right: 0.0,
            bar_style: BarStyle::default(),
            orientation: BarOrientation::default(),
            fill_direction: BarFillDirection::default(),
            foreground: BarFillType::default(),
            background: BarBackgroundType::default(),
            corner_radius: default_corner_radius(),
            bar_spacing: default_bar_spacing(),
            segment_count: default_segment_count(),
            segment_spacing: default_segment_spacing(),
            border: BorderConfig::default(),
            show_labels: default_true(),
            label_prefix: default_label_prefix(),
            label_position: LabelPosition::default(),
            label_font: default_label_font(),
            label_size: default_label_size(),
            label_color: default_label_color(),
            label_bold: false,
            animate: default_true(),
            animation_speed: default_animation_speed(),
            gradient_spans_bars: false,
            text_overlay: TextOverlayConfig::default(),
        }
    }
}

impl CoreBarsConfig {
    /// Get the number of cores to display based on config
    pub fn core_count(&self) -> usize {
        if self.end_core >= self.start_core {
            self.end_core - self.start_core + 1
        } else {
            0
        }
    }
}

impl crate::schema::Schema for CoreBarsConfig {
    fn props() -> Vec<crate::schema::Prop> {
        use super::bar::{background_prop, border_prop, fill_prop};
        use crate::schema::{Condition, Prop, PropKind};
        let core = |key: &'static str, label: &'static str| {
            Prop::new(
                key,
                label,
                PropKind::Int {
                    min: 0,
                    max: 255,
                    step: 1,
                },
            )
            .help("Clamped to the cores the source reports")
        };
        let padding = |key: &'static str, label: &'static str| {
            Prop::new(
                key,
                label,
                PropKind::Float {
                    min: 0.0,
                    max: 100.0,
                    step: 1.0,
                    digits: 0,
                },
            )
        };
        let labelled = Condition::IsTrue { key: "show_labels" };
        vec![
            core("start_core", "First core").section("Cores"),
            core("end_core", "Last core").section("Cores"),
            Prop::new(
                "bar_style",
                "Style",
                PropKind::Enum {
                    options: vec![
                        ("full", "Full"),
                        ("rectangle", "Rectangle"),
                        ("segmented", "Segmented"),
                    ],
                },
            )
            .section("Style"),
            Prop::new(
                "orientation",
                "Orientation",
                PropKind::Enum {
                    options: vec![("horizontal", "Horizontal"), ("vertical", "Vertical")],
                },
            )
            .section("Style"),
            Prop::new(
                "fill_direction",
                "Fill direction",
                PropKind::Enum {
                    options: vec![
                        ("left_to_right", "Left to right"),
                        ("right_to_left", "Right to left"),
                        ("bottom_to_top", "Bottom to top"),
                        ("top_to_bottom", "Top to bottom"),
                    ],
                },
            )
            .section("Style"),
            Prop::new(
                "corner_radius",
                "Corner radius (px)",
                PropKind::Float {
                    min: 0.0,
                    max: 50.0,
                    step: 1.0,
                    digits: 0,
                },
            )
            .section("Style"),
            Prop::new(
                "bar_spacing",
                "Bar spacing (px)",
                PropKind::Float {
                    min: 0.0,
                    max: 50.0,
                    step: 1.0,
                    digits: 0,
                },
            )
            .section("Style"),
            Prop::new(
                "segment_count",
                "Segments",
                PropKind::Int {
                    min: 1,
                    max: 100,
                    step: 1,
                },
            )
            .section("Style")
            .visible_when(Condition::Equals {
                key: "bar_style",
                values: vec!["segmented"],
            }),
            Prop::new(
                "segment_spacing",
                "Segment spacing (px)",
                PropKind::Float {
                    min: 0.0,
                    max: 50.0,
                    step: 1.0,
                    digits: 0,
                },
            )
            .section("Style")
            .visible_when(Condition::Equals {
                key: "bar_style",
                values: vec!["segmented"],
            }),
            padding("padding_top", "Padding top (px)").section("Style"),
            padding("padding_bottom", "Padding bottom (px)").section("Style"),
            padding("padding_left", "Padding left (px)").section("Style"),
            padding("padding_right", "Padding right (px)").section("Style"),
            fill_prop("foreground", "Fill").section("Colours"),
            Prop::new(
                "gradient_spans_bars",
                "Gradient spans all bars",
                PropKind::Bool,
            )
            .section("Colours")
            .help("One gradient across the whole set instead of per bar"),
            background_prop("background", "Background").section("Colours"),
            border_prop("border", "Border").section("Colours"),
            Prop::new("show_labels", "Show core labels", PropKind::Bool).section("Labels"),
            Prop::new(
                "label_prefix",
                "Label prefix",
                PropKind::Text {
                    placeholder: "e.g., C",
                },
            )
            .section("Labels")
            .visible_when(labelled.clone()),
            Prop::new(
                "label_position",
                "Label position",
                PropKind::Enum {
                    options: vec![("start", "Start"), ("end", "End"), ("inside", "Inside")],
                },
            )
            .section("Labels")
            .visible_when(Condition::IsTrue { key: "show_labels" }),
            Prop::new(
                "label_font",
                "Label font",
                PropKind::PlainFont {
                    size_key: "label_size",
                },
            )
            .section("Labels")
            .visible_when(Condition::IsTrue { key: "show_labels" }),
            Prop::new("label_color", "Label colour", PropKind::Color)
                .section("Labels")
                .visible_when(Condition::IsTrue { key: "show_labels" }),
            Prop::new("label_bold", "Bold labels", PropKind::Bool)
                .section("Labels")
                .visible_when(Condition::IsTrue { key: "show_labels" }),
            Prop::new("animate", "Smooth animation", PropKind::Bool).section("Animation"),
            Prop::new(
                "animation_speed",
                "Animation speed",
                PropKind::Float {
                    min: 0.01,
                    max: 1.0,
                    step: 0.05,
                    digits: 2,
                },
            )
            .section("Animation")
            .visible_when(Condition::IsTrue { key: "animate" }),
            Prop::new(
                "text_overlay",
                "Text overlay",
                PropKind::Object {
                    props: <crate::text::TextOverlayConfig as crate::schema::Schema>::props(),
                },
            )
            .section("Text overlay"),
        ]
    }
}

#[cfg(test)]
mod schema_tests {
    use super::*;
    use crate::schema::{check_props_fit, Schema};

    #[test]
    fn props_fit_the_type() {
        check_props_fit::<CoreBarsConfig>(&CoreBarsConfig::props()).unwrap();
    }

    #[test]
    fn default_round_trips_through_json() {
        let value = serde_json::to_value(CoreBarsConfig::default()).unwrap();
        let back: CoreBarsConfig = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(serde_json::to_value(back).unwrap(), value);
    }
}
