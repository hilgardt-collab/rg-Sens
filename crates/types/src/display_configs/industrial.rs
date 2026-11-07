//! Industrial/Gauge Panel display configuration types
//!
//! Features:
//! - Brushed metal or carbon fiber textures (simulated with gradients)
//! - Physical gauge aesthetics (rivets, bezels, 3D effects)
//! - Warning stripe accents (yellow/black diagonal stripes)
//! - Pressure gauge-style circular displays
//! - Heavy bold typography

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::color::Color;
use crate::combo::{ComboFrameConfig, GroupTitleConfig, LayoutFrameConfig, ThemedFrameConfig};
use crate::display_configs::lcars::ContentItemConfig;
use crate::theme::ComboThemeConfig;

// Re-export types from lcars that this module uses
pub use crate::display_configs::lcars::SplitOrientation;

/// Surface texture style
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum SurfaceTexture {
    #[default]
    #[serde(rename = "brushed_metal")]
    BrushedMetal,
    #[serde(rename = "carbon_fiber")]
    CarbonFiber,
    #[serde(rename = "diamond_plate")]
    DiamondPlate,
    #[serde(rename = "solid")]
    Solid,
}

/// Rivet style for panel decoration
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum RivetStyle {
    #[default]
    #[serde(rename = "hex")]
    Hex,
    #[serde(rename = "phillips")]
    Phillips,
    #[serde(rename = "flat")]
    Flat,
    #[serde(rename = "none")]
    None,
}

/// Warning stripe position
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum WarningStripePosition {
    #[serde(rename = "none")]
    #[default]
    None,
    #[serde(rename = "top")]
    Top,
    #[serde(rename = "bottom")]
    Bottom,
    #[serde(rename = "left")]
    Left,
    #[serde(rename = "right")]
    Right,
    #[serde(rename = "all")]
    All,
}

/// Header style for the panel
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum HeaderStyle {
    #[default]
    #[serde(rename = "plate")]
    Plate, // Metal plate with embossed text
    #[serde(rename = "stencil")]
    Stencil, // Stenciled text
    #[serde(rename = "label")]
    Label, // Label plate (like equipment labels)
    #[serde(rename = "none")]
    None,
}

/// Divider style between groups
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum DividerStyle {
    #[default]
    #[serde(rename = "groove")]
    Groove, // Grooved metal divider
    #[serde(rename = "raised")]
    Raised, // Raised metal bar
    #[serde(rename = "warning")]
    Warning, // Warning stripes
    #[serde(rename = "none")]
    None,
}

fn default_animation_enabled() -> bool {
    true
}

fn default_animation_speed() -> f64 {
    8.0
}

fn default_industrial_theme() -> ComboThemeConfig {
    ComboThemeConfig::default_for_industrial()
}

/// Industrial frame configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndustrialFrameConfig {
    // Surface appearance
    pub surface_texture: SurfaceTexture,
    pub surface_color: Color,      // Base metal/surface color
    pub surface_color_dark: Color, // For gradient/texture
    pub highlight_color: Color,    // Specular highlights

    // Border and frame
    pub show_border: bool,
    pub border_width: f64,
    pub border_color: Color,
    pub corner_radius: f64,
    pub show_beveled_edge: bool,
    pub bevel_width: f64,

    // Rivets
    pub rivet_style: RivetStyle,
    pub rivet_size: f64,
    pub rivet_color: Color,
    pub rivet_spacing: f64, // Spacing between rivets
    pub show_corner_rivets: bool,
    pub show_edge_rivets: bool,

    // Warning stripes
    pub warning_stripe_position: WarningStripePosition,
    pub warning_stripe_width: f64,
    pub warning_color_1: Color,    // Usually yellow
    pub warning_color_2: Color,    // Usually black
    pub warning_stripe_angle: f64, // Degrees

    // Header
    pub show_header: bool,
    pub header_text: String,
    pub header_style: HeaderStyle,
    pub header_height: f64,
    pub header_font: String,
    pub header_font_size: f64,
    pub header_color: Color,

    // Layout
    pub content_padding: f64,
    pub item_spacing: f64,
    pub group_count: usize,
    pub group_item_counts: Vec<usize>,
    pub group_size_weights: Vec<f64>,
    pub split_orientation: SplitOrientation,
    /// Item orientation within each group - defaults to same as split_orientation
    #[serde(default)]
    pub group_item_orientations: Vec<SplitOrientation>,
    /// Title bar of each group (see `GroupTitleConfig`)
    #[serde(default)]
    pub group_titles: Vec<GroupTitleConfig>,

    // Dividers
    pub divider_style: DividerStyle,
    pub divider_width: f64,
    pub divider_color: Color,

    // Content items config
    pub content_items: HashMap<String, ContentItemConfig>,

    /// Theme configuration
    pub theme: ComboThemeConfig,

    /// Animation enabled (for ComboFrameConfig trait)
    #[serde(default = "default_animation_enabled")]
    pub animation_enabled: bool,

    /// Animation speed multiplier (for ComboFrameConfig trait)
    #[serde(default = "default_animation_speed")]
    pub animation_speed: f64,
}

impl Default for IndustrialFrameConfig {
    fn default() -> Self {
        Self {
            // Surface - brushed steel look
            surface_texture: SurfaceTexture::BrushedMetal,
            surface_color: Color {
                r: 0.55,
                g: 0.57,
                b: 0.58,
                a: 1.0,
            }, // Steel gray
            surface_color_dark: Color {
                r: 0.40,
                g: 0.42,
                b: 0.43,
                a: 1.0,
            }, // Darker steel
            highlight_color: Color {
                r: 0.75,
                g: 0.77,
                b: 0.78,
                a: 1.0,
            }, // Highlight

            // Border
            show_border: true,
            border_width: 3.0,
            border_color: Color {
                r: 0.25,
                g: 0.25,
                b: 0.25,
                a: 1.0,
            },
            corner_radius: 8.0,
            show_beveled_edge: true,
            bevel_width: 4.0,

            // Rivets
            rivet_style: RivetStyle::Hex,
            rivet_size: 8.0,
            rivet_color: Color {
                r: 0.35,
                g: 0.35,
                b: 0.35,
                a: 1.0,
            },
            rivet_spacing: 60.0,
            show_corner_rivets: true,
            show_edge_rivets: false,

            // Warning stripes
            warning_stripe_position: WarningStripePosition::None,
            warning_stripe_width: 20.0,
            warning_color_1: Color {
                r: 1.0,
                g: 0.8,
                b: 0.0,
                a: 1.0,
            }, // Yellow
            warning_color_2: Color {
                r: 0.1,
                g: 0.1,
                b: 0.1,
                a: 1.0,
            }, // Black
            warning_stripe_angle: 45.0,

            // Header
            show_header: true,
            header_text: "SYSTEM MONITOR".to_string(),
            header_style: HeaderStyle::Plate,
            header_height: 36.0,
            header_font: "Sans Bold".to_string(),
            header_font_size: 16.0,
            header_color: Color {
                r: 0.1,
                g: 0.1,
                b: 0.1,
                a: 1.0,
            },

            // Layout
            content_padding: 12.0,
            item_spacing: 8.0,
            group_count: 1,
            group_item_counts: vec![3],
            group_size_weights: vec![1.0],
            split_orientation: SplitOrientation::Horizontal,
            group_item_orientations: Vec::new(),
            group_titles: Vec::new(),

            // Dividers
            divider_style: DividerStyle::Groove,
            divider_width: 4.0,
            divider_color: Color {
                r: 0.3,
                g: 0.3,
                b: 0.3,
                a: 1.0,
            },

            content_items: HashMap::new(),
            theme: default_industrial_theme(),
            animation_enabled: default_animation_enabled(),
            animation_speed: default_animation_speed(),
        }
    }
}

impl LayoutFrameConfig for IndustrialFrameConfig {
    fn group_count(&self) -> usize {
        self.group_count
    }

    fn group_size_weights(&self) -> &Vec<f64> {
        &self.group_size_weights
    }

    fn group_size_weights_mut(&mut self) -> &mut Vec<f64> {
        &mut self.group_size_weights
    }

    fn group_item_orientations(&self) -> &Vec<SplitOrientation> {
        &self.group_item_orientations
    }

    fn group_item_orientations_mut(&mut self) -> &mut Vec<SplitOrientation> {
        &mut self.group_item_orientations
    }

    fn group_titles(&self) -> &[GroupTitleConfig] {
        &self.group_titles
    }

    fn group_titles_mut(&mut self) -> &mut Vec<GroupTitleConfig> {
        &mut self.group_titles
    }

    fn split_orientation(&self) -> SplitOrientation {
        self.split_orientation
    }
}

impl ThemedFrameConfig for IndustrialFrameConfig {
    fn theme(&self) -> &ComboThemeConfig {
        &self.theme
    }

    fn theme_mut(&mut self) -> &mut ComboThemeConfig {
        &mut self.theme
    }

    fn content_items(&self) -> &HashMap<String, ContentItemConfig> {
        &self.content_items
    }

    fn content_items_mut(&mut self) -> &mut HashMap<String, ContentItemConfig> {
        &mut self.content_items
    }
}

impl ComboFrameConfig for IndustrialFrameConfig {
    fn animation_enabled(&self) -> bool {
        self.animation_enabled
    }

    fn set_animation_enabled(&mut self, enabled: bool) {
        self.animation_enabled = enabled;
    }

    fn animation_speed(&self) -> f64 {
        self.animation_speed
    }

    fn set_animation_speed(&mut self, speed: f64) {
        self.animation_speed = speed;
    }

    fn group_item_counts(&self) -> &[usize] {
        &self.group_item_counts
    }

    fn group_item_counts_mut(&mut self) -> &mut Vec<usize> {
        &mut self.group_item_counts
    }
}

impl IndustrialFrameConfig {
    /// Get text color based on background
    pub fn text_color(&self) -> Color {
        // Dark text on metal background
        Color {
            r: 0.1,
            g: 0.1,
            b: 0.1,
            a: 1.0,
        }
    }
}

impl crate::schema::Schema for IndustrialFrameConfig {
    fn props() -> Vec<crate::schema::Prop> {
        #[allow(unused_imports)]
        use crate::schema::{Condition, Prop, PropKind};
        vec![
            Prop::new(
                "surface_texture",
                "Surface texture",
                PropKind::Enum {
                    options: vec![
                        ("brushed_metal", "Brushed metal"),
                        ("carbon_fiber", "Carbon fibre"),
                        ("diamond_plate", "Diamond plate"),
                        ("solid", "Solid"),
                    ],
                },
            )
            .section("Surface"),
            Prop::new("surface_color", "Surface colour", PropKind::Rgba).section("Surface"),
            Prop::new("surface_color_dark", "Surface shade", PropKind::Rgba).section("Surface"),
            Prop::new("highlight_color", "Highlight colour", PropKind::Rgba).section("Surface"),
            Prop::new("show_border", "Show border", PropKind::Bool).section("Border"),
            Prop::new(
                "border_width",
                "Border width (px)",
                PropKind::Float {
                    min: 0.5,
                    max: 30.0,
                    step: 0.5,
                    digits: 1,
                },
            )
            .section("Border")
            .visible_when(Condition::IsTrue { key: "show_border" }),
            Prop::new("border_color", "Border colour", PropKind::Rgba)
                .section("Border")
                .visible_when(Condition::IsTrue { key: "show_border" }),
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
            .section("Border"),
            Prop::new("show_beveled_edge", "Bevelled edge", PropKind::Bool).section("Border"),
            Prop::new(
                "bevel_width",
                "Bevel width (px)",
                PropKind::Float {
                    min: 0.5,
                    max: 30.0,
                    step: 0.5,
                    digits: 1,
                },
            )
            .section("Border")
            .visible_when(Condition::IsTrue {
                key: "show_beveled_edge",
            }),
            Prop::new(
                "rivet_style",
                "Rivet style",
                PropKind::Enum {
                    options: vec![
                        ("hex", "Hex"),
                        ("phillips", "Phillips"),
                        ("flat", "Flat"),
                        ("none", "None"),
                    ],
                },
            )
            .section("Rivets"),
            Prop::new(
                "rivet_size",
                "Rivet size (px)",
                PropKind::Float {
                    min: 1.0,
                    max: 40.0,
                    step: 0.5,
                    digits: 1,
                },
            )
            .section("Rivets"),
            Prop::new("rivet_color", "Rivet colour", PropKind::Rgba).section("Rivets"),
            Prop::new(
                "rivet_spacing",
                "Rivet spacing (px)",
                PropKind::Float {
                    min: 5.0,
                    max: 300.0,
                    step: 1.0,
                    digits: 0,
                },
            )
            .section("Rivets"),
            Prop::new("show_corner_rivets", "Corner rivets", PropKind::Bool).section("Rivets"),
            Prop::new("show_edge_rivets", "Edge rivets", PropKind::Bool).section("Rivets"),
            Prop::new(
                "warning_stripe_position",
                "Warning stripes",
                PropKind::Enum {
                    options: vec![
                        ("none", "None"),
                        ("top", "Top"),
                        ("bottom", "Bottom"),
                        ("left", "Left"),
                        ("right", "Right"),
                        ("all", "All sides"),
                    ],
                },
            )
            .section("Warning stripes"),
            Prop::new(
                "warning_stripe_width",
                "Stripe width (px)",
                PropKind::Float {
                    min: 1.0,
                    max: 100.0,
                    step: 1.0,
                    digits: 0,
                },
            )
            .section("Warning stripes")
            .visible_when(Condition::Equals {
                key: "warning_stripe_position",
                values: vec!["top", "bottom", "left", "right", "all"],
            }),
            Prop::new("warning_color_1", "Stripe colour 1", PropKind::Rgba)
                .section("Warning stripes")
                .visible_when(Condition::Equals {
                    key: "warning_stripe_position",
                    values: vec!["top", "bottom", "left", "right", "all"],
                }),
            Prop::new("warning_color_2", "Stripe colour 2", PropKind::Rgba)
                .section("Warning stripes")
                .visible_when(Condition::Equals {
                    key: "warning_stripe_position",
                    values: vec!["top", "bottom", "left", "right", "all"],
                }),
            Prop::new(
                "warning_stripe_angle",
                "Stripe angle (°)",
                PropKind::Float {
                    min: 0.0,
                    max: 180.0,
                    step: 5.0,
                    digits: 0,
                },
            )
            .section("Warning stripes")
            .visible_when(Condition::Equals {
                key: "warning_stripe_position",
                values: vec!["top", "bottom", "left", "right", "all"],
            }),
            Prop::new("show_header", "Show header", PropKind::Bool).section("Header"),
            Prop::new(
                "header_text",
                "Header text",
                PropKind::Text {
                    placeholder: "Panel title",
                },
            )
            .section("Header")
            .visible_when(Condition::IsTrue { key: "show_header" }),
            Prop::new(
                "header_style",
                "Header style",
                PropKind::Enum {
                    options: vec![
                        ("plate", "Plate"),
                        ("stencil", "Stencil"),
                        ("label", "Label"),
                        ("none", "None"),
                    ],
                },
            )
            .section("Header")
            .visible_when(Condition::IsTrue { key: "show_header" }),
            Prop::new(
                "header_height",
                "Header height (px)",
                PropKind::Float {
                    min: 10.0,
                    max: 120.0,
                    step: 1.0,
                    digits: 0,
                },
            )
            .section("Header")
            .visible_when(Condition::IsTrue { key: "show_header" }),
            Prop::new(
                "header_font",
                "Header font",
                PropKind::PlainFont {
                    size_key: "header_font_size",
                },
            )
            .section("Header")
            .visible_when(Condition::IsTrue { key: "show_header" }),
            Prop::new("header_color", "Header colour", PropKind::Rgba)
                .section("Header")
                .visible_when(Condition::IsTrue { key: "show_header" }),
            Prop::new(
                "split_orientation",
                "Group layout",
                PropKind::Enum {
                    options: vec![
                        ("vertical", "Stack groups vertically"),
                        ("horizontal", "Place groups side by side"),
                    ],
                },
            )
            .section("Layout"),
            Prop::new(
                "content_padding",
                "Content padding (px)",
                PropKind::Float {
                    min: 0.0,
                    max: 100.0,
                    step: 1.0,
                    digits: 0,
                },
            )
            .section("Layout"),
            Prop::new(
                "item_spacing",
                "Item spacing (px)",
                PropKind::Float {
                    min: 0.0,
                    max: 50.0,
                    step: 1.0,
                    digits: 0,
                },
            )
            .section("Layout"),
            Prop::new(
                "divider_style",
                "Divider",
                PropKind::Enum {
                    options: vec![
                        ("groove", "Groove"),
                        ("raised", "Raised"),
                        ("warning", "Warning"),
                        ("none", "None"),
                    ],
                },
            )
            .section("Layout"),
            Prop::new(
                "divider_width",
                "Divider width (px)",
                PropKind::Float {
                    min: 0.5,
                    max: 30.0,
                    step: 0.5,
                    digits: 1,
                },
            )
            .section("Layout"),
            Prop::new("divider_color", "Divider colour", PropKind::Rgba).section("Layout"),
            Prop::new("animation_enabled", "Animate value changes", PropKind::Bool)
                .section("Animation"),
            Prop::new(
                "animation_speed",
                "Animation speed",
                PropKind::Float {
                    min: 0.5,
                    max: 30.0,
                    step: 0.5,
                    digits: 1,
                },
            )
            .section("Animation")
            .visible_when(Condition::IsTrue {
                key: "animation_enabled",
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
        check_props_fit::<IndustrialFrameConfig>(&IndustrialFrameConfig::props()).unwrap();
    }

    #[test]
    fn default_round_trips_through_json() {
        let value = serde_json::to_value(IndustrialFrameConfig::default()).unwrap();
        let back: IndustrialFrameConfig = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(serde_json::to_value(back).unwrap(), value);
    }
}
