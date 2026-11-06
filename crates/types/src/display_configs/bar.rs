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

/// Templates for the fill variants: the old widget seeded a background
/// gradient with two greys and left a foreground gradient to the editor's
/// stops; here both start usable (foreground: theme colours 1 → 2).
fn fg_solid() -> serde_json::Value {
    serde_json::to_value(BarFillType::Solid {
        color: ColorSource::Theme { index: 1 },
    })
    .expect("fill serializes")
}

fn fg_gradient() -> serde_json::Value {
    serde_json::to_value(BarFillType::Gradient {
        stops: vec![
            ColorStopSource {
                position: 0.0,
                color: ColorSource::Theme { index: 1 },
            },
            ColorStopSource {
                position: 1.0,
                color: ColorSource::Theme { index: 2 },
            },
        ],
        angle: 90.0,
    })
    .expect("fill serializes")
}

fn bg_solid() -> serde_json::Value {
    serde_json::to_value(BarBackgroundType::Solid {
        color: ColorSource::Custom {
            color: Color::new(0.15, 0.15, 0.15, 0.8),
        },
    })
    .expect("background serializes")
}

fn bg_gradient() -> serde_json::Value {
    serde_json::to_value(BarBackgroundType::Gradient {
        stops: vec![
            ColorStopSource::custom(0.0, Color::new(0.2, 0.2, 0.2, 1.0)),
            ColorStopSource::custom(1.0, Color::new(0.1, 0.1, 0.1, 1.0)),
        ],
        angle: 90.0,
    })
    .expect("background serializes")
}

fn bg_transparent() -> serde_json::Value {
    serde_json::to_value(BarBackgroundType::Transparent).expect("background serializes")
}

impl crate::schema::Schema for BarDisplayConfig {
    fn props() -> Vec<crate::schema::Prop> {
        use crate::schema::{Condition, Prop, PropKind, Variant};
        let shaped = Condition::Equals {
            key: "style",
            values: vec!["rectangle", "segmented"],
        };
        let tapered = Condition::Equals {
            key: "taper_style",
            values: vec!["start", "end", "both"],
        };
        let angle = |key: &'static str| {
            Prop::new(
                key,
                "Angle (°)",
                PropKind::Float {
                    min: 0.0,
                    max: 360.0,
                    step: 5.0,
                    digits: 0,
                },
            )
        };
        vec![
            Prop::new(
                "style",
                "Style",
                PropKind::Enum {
                    options: vec![
                        ("full", "Full panel"),
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
                "rectangle_width",
                "Width (fraction)",
                PropKind::Float {
                    min: 0.05,
                    max: 1.0,
                    step: 0.05,
                    digits: 2,
                },
            )
            .section("Style")
            .visible_when(Condition::Equals {
                key: "style",
                values: vec!["rectangle"],
            }),
            Prop::new(
                "rectangle_height",
                "Height (fraction)",
                PropKind::Float {
                    min: 0.05,
                    max: 1.0,
                    step: 0.05,
                    digits: 2,
                },
            )
            .section("Style")
            .visible_when(Condition::Equals {
                key: "style",
                values: vec!["rectangle"],
            }),
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
                key: "style",
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
                key: "style",
                values: vec!["segmented"],
            }),
            Prop::new(
                "segment_width",
                "Segment width (fraction)",
                PropKind::Float {
                    min: 0.05,
                    max: 1.0,
                    step: 0.05,
                    digits: 2,
                },
            )
            .section("Style")
            .visible_when(Condition::Equals {
                key: "style",
                values: vec!["segmented"],
            }),
            Prop::new(
                "segment_height",
                "Segment height (fraction)",
                PropKind::Float {
                    min: 0.05,
                    max: 1.0,
                    step: 0.05,
                    digits: 2,
                },
            )
            .section("Style")
            .visible_when(Condition::Equals {
                key: "style",
                values: vec!["segmented"],
            }),
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
            .section("Style")
            .visible_when(shaped.clone()),
            Prop::new(
                "padding",
                "Padding (px)",
                PropKind::Float {
                    min: 0.0,
                    max: 50.0,
                    step: 1.0,
                    digits: 0,
                },
            )
            .section("Style")
            .visible_when(shaped),
            Prop::new(
                "taper_style",
                "Taper",
                PropKind::Enum {
                    options: vec![
                        ("none", "None"),
                        ("start", "Start"),
                        ("end", "End"),
                        ("both", "Both"),
                    ],
                },
            )
            .section("Style"),
            Prop::new(
                "taper_amount",
                "Taper amount",
                PropKind::Float {
                    min: 0.0,
                    max: 1.0,
                    step: 0.05,
                    digits: 2,
                },
            )
            .section("Style")
            .visible_when(tapered.clone()),
            Prop::new(
                "taper_alignment",
                "Taper anchor",
                PropKind::Enum {
                    options: vec![("start", "Start"), ("center", "Center"), ("end", "End")],
                },
            )
            .section("Style")
            .help("Top/left, centre, or bottom/right of the bar")
            .visible_when(tapered),
            Prop::new(
                "foreground",
                "Fill",
                PropKind::Variant {
                    tag: "type",
                    variants: vec![
                        Variant {
                            value: "solid",
                            label: "Solid colour",
                            props: vec![Prop::new("color", "Colour", PropKind::Color)],
                            template: fg_solid,
                        },
                        Variant {
                            value: "gradient",
                            label: "Gradient",
                            props: vec![
                                Prop::new(
                                    "stops",
                                    "Stops",
                                    PropKind::ColorStops {
                                        linear: true,
                                        themed: true,
                                    },
                                ),
                                angle("angle"),
                            ],
                            template: fg_gradient,
                        },
                    ],
                },
            )
            .section("Foreground"),
            Prop::new(
                "background",
                "Background",
                PropKind::Variant {
                    tag: "type",
                    variants: vec![
                        Variant {
                            value: "transparent",
                            label: "Transparent",
                            props: vec![],
                            template: bg_transparent,
                        },
                        Variant {
                            value: "solid",
                            label: "Solid colour",
                            props: vec![Prop::new("color", "Colour", PropKind::Color)],
                            template: bg_solid,
                        },
                        Variant {
                            value: "gradient",
                            label: "Gradient",
                            props: vec![
                                Prop::new(
                                    "stops",
                                    "Stops",
                                    PropKind::ColorStops {
                                        linear: true,
                                        themed: true,
                                    },
                                ),
                                angle("angle"),
                            ],
                            template: bg_gradient,
                        },
                    ],
                },
            )
            .section("Background"),
            Prop::new(
                "border",
                "Border",
                PropKind::Object {
                    props: vec![
                        Prop::new("enabled", "Draw border", PropKind::Bool),
                        Prop::new("color", "Colour", PropKind::Color)
                            .visible_when(Condition::IsTrue { key: "enabled" }),
                        Prop::new(
                            "width",
                            "Width (px)",
                            PropKind::Float {
                                min: 0.5,
                                max: 20.0,
                                step: 0.5,
                                digits: 1,
                            },
                        )
                        .visible_when(Condition::IsTrue { key: "enabled" }),
                    ],
                },
            )
            .section("Border"),
            Prop::new(
                "text_overlay",
                "Text overlay",
                PropKind::Object {
                    props: <crate::text::TextOverlayConfig as crate::schema::Schema>::props(),
                },
            )
            .section("Text overlay"),
            Prop::new("smooth_animation", "Smooth animation", PropKind::Bool).section("Animation"),
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
            .visible_when(Condition::IsTrue {
                key: "smooth_animation",
            }),
        ]
    }
}

#[cfg(test)]
mod schema_tests {
    use super::*;
    use crate::schema::{check_props_fit, Schema};

    #[test]
    fn props_fit_the_type() {
        check_props_fit::<BarDisplayConfig>(&BarDisplayConfig::props()).unwrap();
    }

    #[test]
    fn default_round_trips_through_json() {
        let default = BarDisplayConfig::default();
        let value = serde_json::to_value(&default).unwrap();
        let back: BarDisplayConfig = serde_json::from_value(value).unwrap();
        assert_eq!(back, default);
    }
}
