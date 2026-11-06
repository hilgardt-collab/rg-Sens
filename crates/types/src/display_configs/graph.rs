//! Graph display configuration types

use serde::{Deserialize, Deserializer, Serialize};

use crate::color::Color;
use crate::text::{TextDisplayerConfig, TextLineConfig, TextOverlayConfig};
use crate::theme::{deserialize_color_or_source, ColorSource, ComboThemeConfig};

/// Custom deserializer for text_overlay that handles multiple formats:
/// - New format: TextOverlayConfig { enabled: bool, text_config: TextDisplayerConfig }
/// - Legacy format: Vec<TextLineConfig> directly (converted to TextOverlayConfig with enabled=true)
pub fn deserialize_text_overlay<'de, D>(deserializer: D) -> Result<TextOverlayConfig, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::{MapAccess, SeqAccess, Visitor};
    use std::fmt;

    struct TextOverlayVisitor;

    impl<'de> Visitor<'de> for TextOverlayVisitor {
        type Value = TextOverlayConfig;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a TextOverlayConfig object or a sequence of TextLineConfig")
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            // Legacy format: Vec<TextLineConfig> - convert to TextOverlayConfig
            let mut lines = Vec::new();
            while let Some(line) = seq.next_element()? {
                lines.push(line);
            }
            // If there are lines, enable the overlay; if empty, disable
            let enabled = !lines.is_empty();
            Ok(TextOverlayConfig {
                enabled,
                text_config: TextDisplayerConfig { lines },
            })
        }

        fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
        where
            M: MapAccess<'de>,
        {
            // New format: { enabled: bool, text_config: { lines: [...] } }
            let mut enabled = true;
            let mut lines = Vec::new();
            while let Some(key) = map.next_key::<String>()? {
                match key.as_str() {
                    "text_config" => {
                        #[derive(Deserialize)]
                        struct TextConfig {
                            #[serde(default)]
                            lines: Vec<TextLineConfig>,
                        }
                        let config: TextConfig = map.next_value()?;
                        lines = config.lines;
                    }
                    "enabled" => {
                        enabled = map.next_value()?;
                    }
                    _ => {
                        // Skip unknown fields
                        let _: serde::de::IgnoredAny = map.next_value()?;
                    }
                }
            }
            Ok(TextOverlayConfig {
                enabled,
                text_config: TextDisplayerConfig { lines },
            })
        }
    }

    deserializer.deserialize_any(TextOverlayVisitor)
}

/// Graph type
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub enum GraphType {
    #[default]
    Line,
    Bar,
    Area,
    SteppedLine,
}

/// Graph line style
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub enum LineStyle {
    #[default]
    Solid,
    Dashed,
    Dotted,
}

/// Graph fill mode
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub enum FillMode {
    #[default]
    None,
    Solid,
    Gradient,
}

/// Axis configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AxisConfig {
    pub show: bool,
    #[serde(deserialize_with = "deserialize_color_or_source")]
    pub color: ColorSource,
    pub width: f64,
    pub show_labels: bool,
    #[serde(deserialize_with = "deserialize_color_or_source")]
    pub label_color: ColorSource,
    #[serde(default = "default_label_font_family")]
    pub label_font_family: String,
    pub label_font_size: f64,
    #[serde(default)]
    pub label_bold: bool,
    #[serde(default)]
    pub label_italic: bool,
    pub show_grid: bool,
    #[serde(deserialize_with = "deserialize_color_or_source")]
    pub grid_color: ColorSource,
    pub grid_width: f64,
    pub grid_line_style: LineStyle,
}

fn default_label_font_family() -> String {
    "Sans".to_string()
}

impl Default for AxisConfig {
    fn default() -> Self {
        Self {
            show: true,
            color: ColorSource::custom(Color {
                r: 0.7,
                g: 0.7,
                b: 0.7,
                a: 1.0,
            }),
            width: 1.0,
            show_labels: true,
            label_color: ColorSource::custom(Color {
                r: 0.8,
                g: 0.8,
                b: 0.8,
                a: 1.0,
            }),
            label_font_family: "Sans".to_string(),
            label_font_size: 10.0,
            label_bold: false,
            label_italic: false,
            show_grid: true,
            grid_color: ColorSource::custom(Color {
                r: 0.3,
                g: 0.3,
                b: 0.3,
                a: 0.5,
            }),
            grid_width: 0.5,
            grid_line_style: LineStyle::Dotted,
        }
    }
}

/// Graph margin/padding
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Margin {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}

impl Default for Margin {
    fn default() -> Self {
        Self {
            top: 10.0,
            right: 10.0,
            bottom: 30.0,
            left: 50.0,
        }
    }
}

/// Graph display configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphDisplayConfig {
    // Graph type and style
    pub graph_type: GraphType,
    pub line_style: LineStyle,
    pub line_width: f64,
    #[serde(deserialize_with = "deserialize_color_or_source")]
    pub line_color: ColorSource,

    // Fill configuration
    pub fill_mode: FillMode,
    #[serde(deserialize_with = "deserialize_color_or_source")]
    pub fill_color: ColorSource,
    #[serde(deserialize_with = "deserialize_color_or_source")]
    pub fill_gradient_start: ColorSource,
    #[serde(deserialize_with = "deserialize_color_or_source")]
    pub fill_gradient_end: ColorSource,
    pub fill_opacity: f64,

    // Data points
    pub max_data_points: usize,
    pub point_radius: f64,
    pub show_points: bool,
    #[serde(deserialize_with = "deserialize_color_or_source")]
    pub point_color: ColorSource,

    // Value range
    pub auto_scale: bool,
    pub min_value: f64,
    pub max_value: f64,
    pub value_padding: f64, // Percentage padding when auto-scaling

    // Axes
    pub x_axis: AxisConfig,
    pub y_axis: AxisConfig,

    // Graph area
    pub margin: Margin,
    pub background_color: Color,
    pub plot_background_color: Color,

    // Animation/smoothing
    pub smooth_lines: bool,
    pub animate_new_points: bool,
    #[serde(default = "default_update_interval")]
    pub update_interval: f64, // Expected time between data updates in seconds (for smooth scrolling)

    // Text overlay - supports both new TextOverlayConfig and legacy Vec<TextLineConfig> format
    #[serde(default, deserialize_with = "deserialize_text_overlay")]
    pub text_overlay: TextOverlayConfig,

    // Theme configuration for resolving theme color/font references
    #[serde(default)]
    pub theme: ComboThemeConfig,
}

fn default_update_interval() -> f64 {
    1.0 // Default 1 second between updates
}

impl Default for GraphDisplayConfig {
    fn default() -> Self {
        let default_graph_color = Color {
            r: 0.2,
            g: 0.8,
            b: 0.4,
            a: 1.0,
        };
        Self {
            graph_type: GraphType::Line,
            line_style: LineStyle::Solid,
            line_width: 2.0,
            line_color: ColorSource::custom(default_graph_color),

            fill_mode: FillMode::Gradient,
            fill_color: ColorSource::custom(Color {
                r: 0.2,
                g: 0.8,
                b: 0.4,
                a: 0.3,
            }),
            fill_gradient_start: ColorSource::custom(Color {
                r: 0.2,
                g: 0.8,
                b: 0.4,
                a: 0.6,
            }),
            fill_gradient_end: ColorSource::custom(Color {
                r: 0.2,
                g: 0.8,
                b: 0.4,
                a: 0.0,
            }),
            fill_opacity: 0.3,

            max_data_points: 60,
            point_radius: 3.0,
            show_points: false,
            point_color: ColorSource::custom(default_graph_color),

            auto_scale: true,
            min_value: 0.0,
            max_value: 100.0,
            value_padding: 10.0,

            x_axis: AxisConfig::default(),
            y_axis: AxisConfig::default(),

            margin: Margin::default(),
            background_color: Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            },
            plot_background_color: Color {
                r: 0.1,
                g: 0.1,
                b: 0.1,
                a: 0.5,
            },

            smooth_lines: true,
            animate_new_points: false,
            update_interval: default_update_interval(),

            text_overlay: TextOverlayConfig::default(),
            theme: ComboThemeConfig::default(),
        }
    }
}

/// Graph data point
#[derive(Debug, Clone, Copy)]
pub struct DataPoint {
    pub value: f64,
    pub timestamp: f64, // Relative time in seconds
}

fn axis_props() -> Vec<crate::schema::Prop> {
    use crate::schema::{Condition, Prop, PropKind};
    let shown = Condition::IsTrue { key: "show" };
    let labelled = Condition::IsTrue { key: "show_labels" };
    let gridded = Condition::IsTrue { key: "show_grid" };
    vec![
        Prop::new("show", "Show axis", PropKind::Bool),
        Prop::new("color", "Axis colour", PropKind::Color).visible_when(shown.clone()),
        Prop::new(
            "width",
            "Axis width (px)",
            PropKind::Float {
                min: 0.5,
                max: 10.0,
                step: 0.5,
                digits: 1,
            },
        )
        .visible_when(shown),
        Prop::new("show_labels", "Show labels", PropKind::Bool),
        Prop::new("label_color", "Label colour", PropKind::Color).visible_when(labelled.clone()),
        Prop::new(
            "label_font_family",
            "Label font",
            PropKind::PlainFont {
                size_key: "label_font_size",
            },
        )
        .visible_when(labelled.clone()),
        Prop::new("label_bold", "Bold labels", PropKind::Bool).visible_when(labelled.clone()),
        Prop::new("label_italic", "Italic labels", PropKind::Bool).visible_when(labelled),
        Prop::new("show_grid", "Show grid", PropKind::Bool),
        Prop::new("grid_color", "Grid colour", PropKind::Color).visible_when(gridded.clone()),
        Prop::new(
            "grid_width",
            "Grid width (px)",
            PropKind::Float {
                min: 0.1,
                max: 5.0,
                step: 0.1,
                digits: 1,
            },
        )
        .visible_when(gridded.clone()),
        Prop::new(
            "grid_line_style",
            "Grid line style",
            PropKind::Enum {
                options: vec![
                    ("Solid", "Solid"),
                    ("Dashed", "Dashed"),
                    ("Dotted", "Dotted"),
                ],
            },
        )
        .visible_when(gridded),
    ]
}

fn margin_prop(key: &'static str, label: &'static str) -> crate::schema::Prop {
    use crate::schema::{Prop, PropKind};
    let side = |k: &'static str, l: &'static str| {
        Prop::new(
            k,
            l,
            PropKind::Float {
                min: 0.0,
                max: 200.0,
                step: 1.0,
                digits: 0,
            },
        )
    };
    Prop::new(
        key,
        label,
        PropKind::Object {
            props: vec![
                side("top", "Top (px)"),
                side("right", "Right (px)"),
                side("bottom", "Bottom (px)"),
                side("left", "Left (px)"),
            ],
        },
    )
}

impl crate::schema::Schema for GraphDisplayConfig {
    fn props() -> Vec<crate::schema::Prop> {
        use crate::schema::{Condition, Prop, PropKind};
        vec![
            Prop::new(
                "graph_type",
                "Graph type",
                PropKind::Enum {
                    options: vec![
                        ("Line", "Line"),
                        ("Bar", "Bars"),
                        ("Area", "Area"),
                        ("SteppedLine", "Stepped line"),
                    ],
                },
            )
            .section("Style"),
            Prop::new(
                "line_style",
                "Line style",
                PropKind::Enum {
                    options: vec![
                        ("Solid", "Solid"),
                        ("Dashed", "Dashed"),
                        ("Dotted", "Dotted"),
                    ],
                },
            )
            .section("Style"),
            Prop::new(
                "line_width",
                "Line width (px)",
                PropKind::Float {
                    min: 0.5,
                    max: 10.0,
                    step: 0.5,
                    digits: 1,
                },
            )
            .section("Style"),
            Prop::new("line_color", "Line colour", PropKind::Color).section("Style"),
            Prop::new("smooth_lines", "Smooth lines", PropKind::Bool).section("Style"),
            Prop::new(
                "fill_mode",
                "Fill",
                PropKind::Enum {
                    options: vec![
                        ("None", "None"),
                        ("Solid", "Solid colour"),
                        ("Gradient", "Gradient"),
                    ],
                },
            )
            .section("Fill"),
            Prop::new("fill_color", "Fill colour", PropKind::Color)
                .section("Fill")
                .visible_when(Condition::Equals {
                    key: "fill_mode",
                    values: vec!["Solid"],
                }),
            Prop::new("fill_gradient_start", "Gradient start", PropKind::Color)
                .section("Fill")
                .visible_when(Condition::Equals {
                    key: "fill_mode",
                    values: vec!["Gradient"],
                }),
            Prop::new("fill_gradient_end", "Gradient end", PropKind::Color)
                .section("Fill")
                .visible_when(Condition::Equals {
                    key: "fill_mode",
                    values: vec!["Gradient"],
                }),
            Prop::new(
                "fill_opacity",
                "Fill opacity",
                PropKind::Float {
                    min: 0.0,
                    max: 1.0,
                    step: 0.05,
                    digits: 2,
                },
            )
            .section("Fill")
            .visible_when(Condition::Equals {
                key: "fill_mode",
                values: vec!["Solid", "Gradient"],
            }),
            Prop::new(
                "max_data_points",
                "Data points kept",
                PropKind::Int {
                    min: 10,
                    max: 10000,
                    step: 10,
                },
            )
            .section("Data"),
            Prop::new("show_points", "Show points", PropKind::Bool).section("Data"),
            Prop::new(
                "point_radius",
                "Point radius (px)",
                PropKind::Float {
                    min: 0.5,
                    max: 20.0,
                    step: 0.5,
                    digits: 1,
                },
            )
            .section("Data")
            .visible_when(Condition::IsTrue { key: "show_points" }),
            Prop::new("point_color", "Point colour", PropKind::Color)
                .section("Data")
                .visible_when(Condition::IsTrue { key: "show_points" }),
            Prop::new("auto_scale", "Auto-scale values", PropKind::Bool).section("Data"),
            Prop::new(
                "value_padding",
                "Scale padding (fraction)",
                PropKind::Float {
                    min: 0.0,
                    max: 1.0,
                    step: 0.05,
                    digits: 2,
                },
            )
            .section("Data")
            .visible_when(Condition::IsTrue { key: "auto_scale" }),
            Prop::new(
                "min_value",
                "Minimum",
                PropKind::Float {
                    min: -1000000.0,
                    max: 1000000.0,
                    step: 1.0,
                    digits: 2,
                },
            )
            .section("Data")
            .visible_when(Condition::IsFalse { key: "auto_scale" }),
            Prop::new(
                "max_value",
                "Maximum",
                PropKind::Float {
                    min: -1000000.0,
                    max: 1000000.0,
                    step: 1.0,
                    digits: 2,
                },
            )
            .section("Data")
            .visible_when(Condition::IsFalse { key: "auto_scale" }),
            Prop::new(
                "x_axis",
                "X axis",
                PropKind::Object {
                    props: axis_props(),
                },
            )
            .section("Axes"),
            Prop::new(
                "y_axis",
                "Y axis",
                PropKind::Object {
                    props: axis_props(),
                },
            )
            .section("Axes"),
            margin_prop("margin", "Margins").section("Layout"),
            Prop::new("background_color", "Background", PropKind::Rgba).section("Layout"),
            Prop::new("plot_background_color", "Plot background", PropKind::Rgba).section("Layout"),
            Prop::new(
                "text_overlay",
                "Text overlay",
                PropKind::Object {
                    props: <crate::text::TextOverlayConfig as crate::schema::Schema>::props(),
                },
            )
            .section("Text overlay"),
            Prop::new(
                "animate_new_points",
                "Scroll new points smoothly",
                PropKind::Bool,
            )
            .section("Animation"),
            Prop::new(
                "update_interval",
                "Expected update interval (s)",
                PropKind::Float {
                    min: 0.1,
                    max: 60.0,
                    step: 0.1,
                    digits: 1,
                },
            )
            .section("Animation")
            .help("Pace of the source, so scrolling matches it")
            .visible_when(Condition::IsTrue {
                key: "animate_new_points",
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
        check_props_fit::<GraphDisplayConfig>(&GraphDisplayConfig::props()).unwrap();
    }

    #[test]
    fn default_round_trips_through_json() {
        let value = serde_json::to_value(GraphDisplayConfig::default()).unwrap();
        let back: GraphDisplayConfig = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(serde_json::to_value(back).unwrap(), value);
    }
}
