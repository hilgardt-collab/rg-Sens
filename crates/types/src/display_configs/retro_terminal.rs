//! Retro Terminal (CRT) display configuration types
//!
//! Provides a vintage CRT terminal aesthetic with:
//! - Green or amber phosphor text on dark background
//! - CRT scanline and curvature effects
//! - Monitor bezel frame styling
//! - Phosphor glow (screen burn) around bright elements
//! - Optional flicker and vignette effects

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::color::Color;
use crate::combo::{ComboFrameConfig, GroupTitleConfig, LayoutFrameConfig, ThemedFrameConfig};
use crate::display_configs::lcars::{ContentItemConfig, SplitOrientation};
use crate::theme::{deserialize_font_or_source, ComboThemeConfig, FontSource};

/// Phosphor color presets (classic CRT colors).
///
/// Legacy: the frame renders from `theme.color1`, and these presets live on
/// as the Theme node's "… phosphor" palettes (`palette`). The loader folds a
/// saved non-default value into the panel's theme once.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum PhosphorColor {
    /// Classic P1 green phosphor (#33ff33)
    #[default]
    Green,
    /// P3 amber phosphor (#ffb000)
    Amber,
    /// P4 white phosphor
    White,
    /// Rare blue terminal
    Blue,
    /// Custom color
    Custom(Color),
}

impl PhosphorColor {
    /// Get the actual color value
    pub fn to_color(&self) -> Color {
        match self {
            PhosphorColor::Green => Color {
                r: 0.2,
                g: 1.0,
                b: 0.2,
                a: 1.0,
            },
            PhosphorColor::Amber => Color {
                r: 1.0,
                g: 0.69,
                b: 0.0,
                a: 1.0,
            },
            PhosphorColor::White => Color {
                r: 0.9,
                g: 0.9,
                b: 0.85,
                a: 1.0,
            },
            PhosphorColor::Blue => Color {
                r: 0.4,
                g: 0.6,
                b: 1.0,
                a: 1.0,
            },
            PhosphorColor::Custom(c) => *c,
        }
    }

    /// Get a dimmed version for secondary elements
    pub fn to_dim_color(&self) -> Color {
        let c = self.to_color();
        Color {
            r: c.r * 0.5,
            g: c.g * 0.5,
            b: c.b * 0.5,
            a: c.a * 0.7,
        }
    }

    /// Whether this is the default (`Green`): the value a frame saved by this
    /// version carries implicitly.
    pub fn is_default(&self) -> bool {
        matches!(self, PhosphorColor::Green)
    }

    /// The theme a terminal of this phosphor renders with: the retro default
    /// theme (near-black gradient, monospace fonts) with its four colours set
    /// to the palette the pre-schema widget wrote when this preset was
    /// picked (primary, dark, medium, light); a custom colour gets the same
    /// variations derived from it.
    pub fn palette(&self) -> ComboThemeConfig {
        let [color1, color2, color3, color4] = self.palette_colors();
        ComboThemeConfig {
            color1,
            color2,
            color3,
            color4,
            ..ComboThemeConfig::default_for_retro_terminal()
        }
    }

    fn palette_colors(&self) -> [Color; 4] {
        match self {
            PhosphorColor::Green => [
                Color::new(0.2, 1.0, 0.2, 1.0),
                Color::new(0.1, 0.6, 0.1, 1.0),
                Color::new(0.3, 0.8, 0.3, 1.0),
                Color::new(0.5, 1.0, 0.5, 1.0),
            ],
            PhosphorColor::Amber => [
                Color::new(1.0, 0.69, 0.0, 1.0),
                Color::new(0.6, 0.4, 0.0, 1.0),
                Color::new(0.8, 0.55, 0.0, 1.0),
                Color::new(1.0, 0.8, 0.3, 1.0),
            ],
            PhosphorColor::White => [
                Color::new(0.9, 0.9, 0.85, 1.0),
                Color::new(0.5, 0.5, 0.48, 1.0),
                Color::new(0.7, 0.7, 0.67, 1.0),
                Color::new(1.0, 1.0, 0.95, 1.0),
            ],
            PhosphorColor::Blue => [
                Color::new(0.4, 0.6, 1.0, 1.0),
                Color::new(0.2, 0.3, 0.6, 1.0),
                Color::new(0.3, 0.5, 0.8, 1.0),
                Color::new(0.6, 0.8, 1.0, 1.0),
            ],
            PhosphorColor::Custom(c) => [
                *c,
                Color::new(c.r * 0.6, c.g * 0.6, c.b * 0.6, c.a),
                Color::new(c.r * 0.8, c.g * 0.8, c.b * 0.8, c.a),
                Color::new(
                    (c.r * 1.2).min(1.0),
                    (c.g * 1.2).min(1.0),
                    (c.b * 1.2).min(1.0),
                    c.a,
                ),
            ],
        }
    }
}

/// Bezel style for the monitor frame
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum BezelStyle {
    /// Thick bezel with rounded inner edge (classic CRT)
    #[default]
    Classic,
    /// Thin modern bezel
    Slim,
    /// Heavy-duty industrial monitor
    Industrial,
    /// No bezel, just the screen
    None,
}

/// Header/title bar style
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum TerminalHeaderStyle {
    /// Window title bar style
    #[default]
    TitleBar,
    /// VT100-style status line at top
    StatusLine,
    /// Shell prompt style
    Prompt,
    /// No header
    None,
}

/// Divider style between content groups
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum TerminalDividerStyle {
    /// Dashed line: ------
    #[default]
    Dashed,
    /// Solid line
    Solid,
    /// Box drawing chars
    BoxDrawing,
    /// Vertical pipes: |||||||
    Pipe,
    /// ASCII equals: ========
    Ascii,
    /// No divider
    None,
}

// Default value functions
fn default_scanline_intensity() -> f64 {
    0.25
}
fn default_scanline_spacing() -> f64 {
    2.0
}
fn default_curvature_amount() -> f64 {
    0.02
}
fn default_vignette_intensity() -> f64 {
    0.4
}
fn default_screen_glow() -> f64 {
    0.5
}
fn default_bezel_width() -> f64 {
    16.0
}
fn default_content_padding() -> f64 {
    12.0
}
fn default_header_font_source() -> FontSource {
    FontSource::theme(1, 14.0)
} // Theme font 1
fn default_header_height() -> f64 {
    28.0
}
fn default_divider_padding() -> f64 {
    4.0
}
fn default_group_count() -> usize {
    1
}
fn default_text_brightness() -> f64 {
    0.9
}

fn default_background_color() -> Color {
    Color {
        r: 0.02,
        g: 0.02,
        b: 0.02,
        a: 1.0,
    }
}

fn default_bezel_color() -> Color {
    Color {
        r: 0.12,
        g: 0.12,
        b: 0.10,
        a: 1.0,
    }
}

fn default_power_led_color() -> Color {
    Color {
        r: 0.2,
        g: 0.8,
        b: 0.2,
        a: 1.0,
    }
}

fn default_retro_terminal_theme() -> ComboThemeConfig {
    ComboThemeConfig::default_for_retro_terminal()
}

fn default_true() -> bool {
    true
}

fn default_animation_speed() -> f64 {
    8.0
}

/// Main configuration for the Retro Terminal frame
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetroTerminalFrameConfig {
    // Color scheme
    /// Legacy phosphor preset. The frame renders from `theme.color1`; the
    /// loader folds a non-default value into the panel's theme once
    /// (`config::settings::adopt_frame_color_preset`), so this is never shown
    /// in the editor and is not written when it is the default.
    #[serde(default, skip_serializing_if = "PhosphorColor::is_default")]
    pub phosphor_color: PhosphorColor,
    #[serde(default = "default_background_color")]
    pub background_color: Color,
    #[serde(default = "default_text_brightness")]
    pub text_brightness: f64,

    // CRT Effects
    #[serde(default = "default_scanline_intensity")]
    pub scanline_intensity: f64,
    #[serde(default = "default_scanline_spacing")]
    pub scanline_spacing: f64,
    #[serde(default = "default_curvature_amount")]
    pub curvature_amount: f64,
    #[serde(default = "default_vignette_intensity")]
    pub vignette_intensity: f64,
    #[serde(default = "default_screen_glow")]
    pub screen_glow: f64,
    #[serde(default)]
    pub flicker_enabled: bool,

    // Bezel/Frame
    #[serde(default)]
    pub bezel_style: BezelStyle,
    #[serde(default = "default_bezel_color")]
    pub bezel_color: Color,
    #[serde(default = "default_bezel_width")]
    pub bezel_width: f64,
    #[serde(default = "default_true")]
    pub show_power_led: bool,
    #[serde(default = "default_power_led_color")]
    pub power_led_color: Color,

    // Header
    #[serde(default = "default_true")]
    pub show_header: bool,
    #[serde(default)]
    pub header_text: String,
    #[serde(default)]
    pub header_style: TerminalHeaderStyle,
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
    /// Title bar of each group (see `GroupTitleConfig`)
    #[serde(default)]
    pub group_titles: Vec<GroupTitleConfig>,

    // Dividers
    #[serde(default)]
    pub divider_style: TerminalDividerStyle,
    #[serde(default = "default_divider_padding")]
    pub divider_padding: f64,

    // Content items (per slot)
    #[serde(default)]
    pub content_items: HashMap<String, ContentItemConfig>,

    // Animation
    #[serde(default = "default_true")]
    pub animation_enabled: bool,
    #[serde(default = "default_animation_speed")]
    pub animation_speed: f64,
    #[serde(default = "default_true")]
    pub cursor_blink: bool,
    #[serde(default)]
    pub typewriter_effect: bool,

    /// Theme configuration
    #[serde(default = "default_retro_terminal_theme")]
    pub theme: ComboThemeConfig,
}

impl Default for RetroTerminalFrameConfig {
    fn default() -> Self {
        Self {
            phosphor_color: PhosphorColor::Green,
            background_color: default_background_color(),
            text_brightness: default_text_brightness(),

            scanline_intensity: default_scanline_intensity(),
            scanline_spacing: default_scanline_spacing(),
            curvature_amount: default_curvature_amount(),
            vignette_intensity: default_vignette_intensity(),
            screen_glow: default_screen_glow(),
            flicker_enabled: false,

            bezel_style: BezelStyle::Classic,
            bezel_color: default_bezel_color(),
            bezel_width: default_bezel_width(),
            show_power_led: true,
            power_led_color: default_power_led_color(),

            show_header: true,
            header_text: "SYSTEM MONITOR".to_string(),
            header_style: TerminalHeaderStyle::TitleBar,
            header_font: default_header_font_source(),
            header_height: default_header_height(),

            content_padding: default_content_padding(),
            group_count: default_group_count(),
            group_item_counts: vec![4],
            group_size_weights: vec![1.0],
            split_orientation: SplitOrientation::Vertical,
            group_item_orientations: Vec::new(),
            group_titles: Vec::new(),

            divider_style: TerminalDividerStyle::Dashed,
            divider_padding: default_divider_padding(),

            content_items: HashMap::new(),

            animation_enabled: true,
            animation_speed: default_animation_speed(),
            cursor_blink: true,
            typewriter_effect: false,

            theme: default_retro_terminal_theme(),
        }
    }
}

impl LayoutFrameConfig for RetroTerminalFrameConfig {
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

impl ThemedFrameConfig for RetroTerminalFrameConfig {
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

impl ComboFrameConfig for RetroTerminalFrameConfig {
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

impl crate::schema::Schema for RetroTerminalFrameConfig {
    fn props() -> Vec<crate::schema::Prop> {
        #[allow(unused_imports)]
        use crate::schema::{Condition, Prop, PropKind};
        vec![
            Prop::new("background_color", "Background colour", PropKind::Rgba).section("Screen"),
            Prop::new(
                "text_brightness",
                "Text brightness",
                PropKind::Float {
                    min: 0.0,
                    max: 2.0,
                    step: 0.05,
                    digits: 2,
                },
            )
            .section("Screen"),
            Prop::new(
                "scanline_intensity",
                "Scanline intensity",
                PropKind::Float {
                    min: 0.0,
                    max: 1.0,
                    step: 0.05,
                    digits: 2,
                },
            )
            .section("Effects"),
            Prop::new(
                "scanline_spacing",
                "Scanline spacing (px)",
                PropKind::Float {
                    min: 1.0,
                    max: 20.0,
                    step: 0.5,
                    digits: 1,
                },
            )
            .section("Effects"),
            Prop::new(
                "curvature_amount",
                "Screen curvature",
                PropKind::Float {
                    min: 0.0,
                    max: 1.0,
                    step: 0.05,
                    digits: 2,
                },
            )
            .section("Effects"),
            Prop::new(
                "vignette_intensity",
                "Vignette",
                PropKind::Float {
                    min: 0.0,
                    max: 1.0,
                    step: 0.05,
                    digits: 2,
                },
            )
            .section("Effects"),
            Prop::new(
                "screen_glow",
                "Screen glow",
                PropKind::Float {
                    min: 0.0,
                    max: 1.0,
                    step: 0.05,
                    digits: 2,
                },
            )
            .section("Effects"),
            Prop::new("flicker_enabled", "Flicker", PropKind::Bool).section("Effects"),
            Prop::new("cursor_blink", "Blinking cursor", PropKind::Bool).section("Effects"),
            Prop::new("typewriter_effect", "Typewriter effect", PropKind::Bool).section("Effects"),
            Prop::new(
                "bezel_style",
                "Bezel style",
                PropKind::Enum {
                    options: vec![
                        ("classic", "Classic"),
                        ("slim", "Slim"),
                        ("industrial", "Industrial"),
                        ("none", "None"),
                    ],
                },
            )
            .section("Bezel"),
            Prop::new("bezel_color", "Bezel colour", PropKind::Rgba).section("Bezel"),
            Prop::new(
                "bezel_width",
                "Bezel width (px)",
                PropKind::Float {
                    min: 0.0,
                    max: 100.0,
                    step: 1.0,
                    digits: 0,
                },
            )
            .section("Bezel"),
            Prop::new("show_power_led", "Power LED", PropKind::Bool).section("Bezel"),
            Prop::new("power_led_color", "LED colour", PropKind::Rgba)
                .section("Bezel")
                .visible_when(Condition::IsTrue {
                    key: "show_power_led",
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
                        ("title_bar", "Title bar"),
                        ("status_line", "Status line"),
                        ("prompt", "Prompt"),
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
                        ("dashed", "Dashed"),
                        ("solid", "Solid"),
                        ("box_drawing", "Box drawing"),
                        ("pipe", "Pipe"),
                        ("ascii", "ASCII"),
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
        check_props_fit::<RetroTerminalFrameConfig>(&RetroTerminalFrameConfig::props()).unwrap();
    }

    #[test]
    fn default_round_trips_through_json() {
        let value = serde_json::to_value(RetroTerminalFrameConfig::default()).unwrap();
        let back: RetroTerminalFrameConfig = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(serde_json::to_value(back).unwrap(), value);
    }
}

#[cfg(test)]
mod phosphor_legacy_tests {
    use super::*;

    #[test]
    fn palette_leads_with_the_phosphor_colour() {
        for p in [
            PhosphorColor::Green,
            PhosphorColor::Amber,
            PhosphorColor::White,
            PhosphorColor::Blue,
            PhosphorColor::Custom(Color::new(0.5, 0.2, 0.9, 1.0)),
        ] {
            let palette = p.palette();
            assert_eq!(palette.color1, p.to_color());
            assert_eq!(palette.font1_family, "monospace");
        }
    }

    #[test]
    fn default_phosphor_is_not_written_but_a_legacy_value_is() {
        let value = serde_json::to_value(RetroTerminalFrameConfig::default()).unwrap();
        assert!(value.get("phosphor_color").is_none());
        let amber = RetroTerminalFrameConfig {
            phosphor_color: PhosphorColor::Amber,
            ..Default::default()
        };
        assert_eq!(
            serde_json::to_value(amber).unwrap()["phosphor_color"],
            serde_json::json!("amber")
        );
    }

    /// Values the pre-schema widget and the Phase-4b editor saved.
    #[test]
    fn every_saved_value_deserializes() {
        for v in [
            serde_json::json!("green"),
            serde_json::json!("amber"),
            serde_json::json!("white"),
            serde_json::json!("blue"),
            serde_json::json!({ "custom": { "r": 0.5, "g": 0.5, "b": 0.5, "a": 1.0 } }),
        ] {
            serde_json::from_value::<PhosphorColor>(v).unwrap();
        }
    }
}
