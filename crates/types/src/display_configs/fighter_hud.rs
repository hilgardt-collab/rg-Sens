//! Fighter Jet HUD display configuration types
//!
//! Provides a military fighter jet heads-up display aesthetic with:
//! - Military green/amber monochrome color scheme
//! - Thin line frames with corner brackets [ ]
//! - Targeting reticle aesthetics for gauges
//! - Altitude/heading ladder-style scales
//! - Stencil military font styling

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::color::Color;
use crate::combo::{ComboFrameConfig, LayoutFrameConfig, ThemedFrameConfig};
use crate::display_configs::lcars::{ContentItemConfig, SplitOrientation};
use crate::theme::{deserialize_font_or_source, ColorSource, ComboThemeConfig, FontSource};

/// HUD color presets (military display colors)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum HudColorPreset {
    /// Classic military green (night vision friendly)
    #[default]
    MilitaryGreen,
    /// Amber/orange (high contrast)
    Amber,
    /// Cyan (modern fighter jets)
    Cyan,
    /// White (daytime mode)
    White,
    /// Custom color
    Custom(Color),
}

impl HudColorPreset {
    /// Get the actual color value
    pub fn to_color(&self) -> Color {
        match self {
            HudColorPreset::MilitaryGreen => Color {
                r: 0.0,
                g: 0.9,
                b: 0.3,
                a: 1.0,
            },
            HudColorPreset::Amber => Color {
                r: 1.0,
                g: 0.75,
                b: 0.0,
                a: 1.0,
            },
            HudColorPreset::Cyan => Color {
                r: 0.0,
                g: 0.9,
                b: 1.0,
                a: 1.0,
            },
            HudColorPreset::White => Color {
                r: 0.95,
                g: 0.95,
                b: 0.95,
                a: 1.0,
            },
            HudColorPreset::Custom(c) => *c,
        }
    }

    /// Get a dimmed version for secondary elements
    pub fn to_dim_color(&self) -> Color {
        let c = self.to_color();
        Color {
            r: c.r * 0.5,
            g: c.g * 0.5,
            b: c.b * 0.5,
            a: c.a * 0.6,
        }
    }

    /// Get a bright version for emphasis
    pub fn to_bright_color(&self) -> Color {
        let c = self.to_color();
        Color {
            r: (c.r * 1.2).min(1.0),
            g: (c.g * 1.2).min(1.0),
            b: (c.b * 1.2).min(1.0),
            a: c.a,
        }
    }

    /// Generate theme colors based on this preset
    /// Returns (color1, color2, color3, color4) for the theme
    pub fn to_theme_colors(&self) -> (Color, Color, Color, Color) {
        let primary = self.to_color();
        let dim = self.to_dim_color();
        let bright = self.to_bright_color();
        // Color 4: semi-transparent version for backgrounds/accents
        let accent = Color {
            r: primary.r,
            g: primary.g,
            b: primary.b,
            a: 0.5,
        };
        (primary, dim, bright, accent)
    }
}

/// Frame style for the HUD
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum HudFrameStyle {
    /// Corner brackets [ ] with tick marks
    #[default]
    CornerBrackets,
    /// Targeting reticle corners
    TargetingReticle,
    /// Full box with corner accents
    TacticalBox,
    /// Minimal with just corner marks
    Minimal,
    /// No frame
    None,
}

/// Header style for the HUD
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum HudHeaderStyle {
    /// Status bar with designation
    #[default]
    StatusBar,
    /// Mission callout style
    MissionCallout,
    /// System ID style
    SystemId,
    /// No header
    None,
}

/// Divider style between content groups
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum HudDividerStyle {
    /// Tick mark ladder style
    #[default]
    TickLadder,
    /// Thin line with arrows
    ArrowLine,
    /// Dashed tactical line
    TacticalDash,
    /// Subtle gradient fade
    Fade,
    /// No divider
    None,
}

// Default value functions
fn default_line_width() -> f64 {
    1.5
}
fn default_bracket_size() -> f64 {
    20.0
}
fn default_bracket_thickness() -> f64 {
    2.0
}
fn default_content_padding() -> f64 {
    16.0
}
fn default_header_font_source() -> FontSource {
    FontSource::theme(1, 12.0)
} // Theme font 1
fn default_header_height() -> f64 {
    24.0
}
fn default_divider_padding() -> f64 {
    6.0
}
fn default_group_count() -> usize {
    1
}
fn default_glow_intensity() -> f64 {
    0.3
}
fn default_tick_spacing() -> f64 {
    8.0
}
fn default_reticle_size() -> f64 {
    0.15
}
fn default_reticle_color() -> ColorSource {
    ColorSource::Theme { index: 1 }
} // Primary theme color

fn default_background_color() -> Color {
    Color {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    } // Transparent by default (HUD overlay)
}

fn default_fighter_hud_theme() -> ComboThemeConfig {
    ComboThemeConfig::default_for_fighter_hud()
}

fn default_true() -> bool {
    true
}

fn default_animation_speed() -> f64 {
    8.0
}

/// Main configuration for the Fighter HUD frame
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FighterHudFrameConfig {
    // Color scheme
    #[serde(default)]
    pub hud_color: HudColorPreset,
    #[serde(default = "default_background_color")]
    pub background_color: Color,
    #[serde(default = "default_glow_intensity")]
    pub glow_intensity: f64,

    // Frame styling
    #[serde(default)]
    pub frame_style: HudFrameStyle,
    #[serde(default = "default_line_width")]
    pub line_width: f64,
    #[serde(default = "default_bracket_size")]
    pub bracket_size: f64,
    #[serde(default = "default_bracket_thickness")]
    pub bracket_thickness: f64,

    // Targeting reticle (optional center element)
    #[serde(default)]
    pub show_center_reticle: bool,
    #[serde(default = "default_reticle_size")]
    pub reticle_size: f64,
    #[serde(default = "default_reticle_color")]
    pub reticle_color: ColorSource,

    // Header
    #[serde(default = "default_true")]
    pub show_header: bool,
    #[serde(default)]
    pub header_text: String,
    #[serde(default)]
    pub header_style: HudHeaderStyle,
    #[serde(
        default = "default_header_font_source",
        deserialize_with = "deserialize_font_or_source"
    )]
    pub header_font: FontSource,
    #[serde(default = "default_header_height")]
    pub header_height: f64,

    // Layout
    #[serde(default = "default_content_padding")]
    pub content_padding: f64,
    #[serde(default = "default_group_count")]
    pub group_count: usize,
    #[serde(default)]
    pub group_item_counts: Vec<usize>,
    #[serde(default)]
    pub group_size_weights: Vec<f64>,
    #[serde(default)]
    pub split_orientation: SplitOrientation,
    /// Item orientation within each group - defaults to same as split_orientation
    #[serde(default)]
    pub group_item_orientations: Vec<SplitOrientation>,

    // Dividers
    #[serde(default)]
    pub divider_style: HudDividerStyle,
    #[serde(default = "default_divider_padding")]
    pub divider_padding: f64,
    #[serde(default = "default_tick_spacing")]
    pub tick_spacing: f64,

    // Content items (per slot)
    #[serde(default)]
    pub content_items: HashMap<String, ContentItemConfig>,

    // Animation
    #[serde(default = "default_true")]
    pub animation_enabled: bool,
    #[serde(default = "default_animation_speed")]
    pub animation_speed: f64,
    #[serde(default)]
    pub scan_line_effect: bool,

    /// Theme configuration
    #[serde(default = "default_fighter_hud_theme")]
    pub theme: ComboThemeConfig,
}

impl Default for FighterHudFrameConfig {
    fn default() -> Self {
        Self {
            hud_color: HudColorPreset::MilitaryGreen,
            background_color: default_background_color(),
            glow_intensity: default_glow_intensity(),

            frame_style: HudFrameStyle::CornerBrackets,
            line_width: default_line_width(),
            bracket_size: default_bracket_size(),
            bracket_thickness: default_bracket_thickness(),

            show_center_reticle: false,
            reticle_size: default_reticle_size(),
            reticle_color: default_reticle_color(),

            show_header: true,
            header_text: "SYS MONITOR".to_string(),
            header_style: HudHeaderStyle::StatusBar,
            header_font: default_header_font_source(),
            header_height: default_header_height(),

            content_padding: default_content_padding(),
            group_count: default_group_count(),
            group_item_counts: vec![4],
            group_size_weights: vec![1.0],
            split_orientation: SplitOrientation::Vertical,
            group_item_orientations: Vec::new(),

            divider_style: HudDividerStyle::TickLadder,
            divider_padding: default_divider_padding(),
            tick_spacing: default_tick_spacing(),

            content_items: HashMap::new(),

            animation_enabled: true,
            animation_speed: default_animation_speed(),
            scan_line_effect: false,

            theme: default_fighter_hud_theme(),
        }
    }
}

impl LayoutFrameConfig for FighterHudFrameConfig {
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

    fn split_orientation(&self) -> SplitOrientation {
        self.split_orientation
    }
}

impl ThemedFrameConfig for FighterHudFrameConfig {
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

impl ComboFrameConfig for FighterHudFrameConfig {
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

impl crate::schema::Schema for FighterHudFrameConfig {
    fn props() -> Vec<crate::schema::Prop> {
        #[allow(unused_imports)]
        use crate::schema::{Condition, Prop, PropKind};
        vec![
            Prop::new(
                "hud_color",
                "HUD colour",
                PropKind::Custom {
                    editor: "hud_color",
                },
            )
            .section("HUD"),
            Prop::new("background_color", "Background colour", PropKind::Rgba).section("HUD"),
            Prop::new(
                "glow_intensity",
                "Glow intensity",
                PropKind::Float {
                    min: 0.0,
                    max: 1.0,
                    step: 0.05,
                    digits: 2,
                },
            )
            .section("HUD"),
            Prop::new("scan_line_effect", "Scan line effect", PropKind::Bool).section("HUD"),
            Prop::new(
                "frame_style",
                "Frame style",
                PropKind::Enum {
                    options: vec![
                        ("corner_brackets", "Corner brackets"),
                        ("targeting_reticle", "Targeting reticle"),
                        ("tactical_box", "Tactical box"),
                        ("minimal", "Minimal"),
                        ("none", "None"),
                    ],
                },
            )
            .section("Frame"),
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
            .section("Frame"),
            Prop::new(
                "bracket_size",
                "Bracket size (px)",
                PropKind::Float {
                    min: 0.0,
                    max: 200.0,
                    step: 1.0,
                    digits: 0,
                },
            )
            .section("Frame"),
            Prop::new(
                "bracket_thickness",
                "Bracket thickness (px)",
                PropKind::Float {
                    min: 0.5,
                    max: 20.0,
                    step: 0.5,
                    digits: 1,
                },
            )
            .section("Frame"),
            Prop::new("show_center_reticle", "Centre reticle", PropKind::Bool).section("Reticle"),
            Prop::new(
                "reticle_size",
                "Reticle size (px)",
                PropKind::Float {
                    min: 1.0,
                    max: 200.0,
                    step: 1.0,
                    digits: 0,
                },
            )
            .section("Reticle")
            .visible_when(Condition::IsTrue {
                key: "show_center_reticle",
            }),
            Prop::new("reticle_color", "Reticle colour", PropKind::Color)
                .section("Reticle")
                .visible_when(Condition::IsTrue {
                    key: "show_center_reticle",
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
                        ("status_bar", "Status bar"),
                        ("mission_callout", "Mission callout"),
                        ("system_id", "System ID"),
                        ("none", "None"),
                    ],
                },
            )
            .section("Header")
            .visible_when(Condition::IsTrue { key: "show_header" }),
            Prop::new(
                "header_font",
                "Header font",
                PropKind::Font { legacy: None },
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
                "group_item_orientations",
                "Items within group",
                PropKind::EnumList {
                    options: vec![("vertical", "Vertical"), ("horizontal", "Horizontal")],
                    count_key: "group_count",
                    default_key: Some("split_orientation"),
                    item_label: "Group",
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
                "divider_style",
                "Divider",
                PropKind::Enum {
                    options: vec![
                        ("tick_ladder", "Tick ladder"),
                        ("arrow_line", "Arrow line"),
                        ("tactical_dash", "Tactical dash"),
                        ("fade", "Fade"),
                        ("none", "None"),
                    ],
                },
            )
            .section("Layout"),
            Prop::new(
                "divider_padding",
                "Divider padding (px)",
                PropKind::Float {
                    min: 0.0,
                    max: 50.0,
                    step: 1.0,
                    digits: 0,
                },
            )
            .section("Layout"),
            Prop::new(
                "tick_spacing",
                "Tick spacing (px)",
                PropKind::Float {
                    min: 1.0,
                    max: 100.0,
                    step: 1.0,
                    digits: 0,
                },
            )
            .section("Layout")
            .visible_when(Condition::Equals {
                key: "divider_style",
                values: vec!["tick_ladder"],
            }),
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
        check_props_fit::<FighterHudFrameConfig>(&FighterHudFrameConfig::props()).unwrap();
    }

    #[test]
    fn default_round_trips_through_json() {
        let value = serde_json::to_value(FighterHudFrameConfig::default()).unwrap();
        let back: FighterHudFrameConfig = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(serde_json::to_value(back).unwrap(), value);
    }
}

#[cfg(test)]
mod hud_color_editor_tests {
    use super::*;

    #[test]
    fn every_value_the_editor_writes_deserializes() {
        for v in [
            serde_json::json!("military_green"),
            serde_json::json!("amber"),
            serde_json::json!("cyan"),
            serde_json::json!("white"),
            serde_json::json!({ "custom": { "r": 0.5, "g": 0.5, "b": 0.5, "a": 1.0 } }),
        ] {
            serde_json::from_value::<HudColorPreset>(v).unwrap();
        }
    }
}
