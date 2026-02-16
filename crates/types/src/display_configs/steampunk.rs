//! Steampunk display configuration types
//!
//! Provides a Victorian-era steampunk display with:
//! - Brass, copper, and bronze metallic color schemes
//! - Decorative gears and cogs
//! - Ornate rivets and bolts
//! - Victorian flourishes and filigree
//! - Steam pipe and gauge aesthetics
//! - Weathered patina textures

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::color::Color;
use crate::combo::{ComboFrameConfig, LayoutFrameConfig, ThemedFrameConfig};
use crate::display_configs::lcars::ContentItemConfig;
use crate::theme::{deserialize_color_or_source, deserialize_font_or_source, ColorSource, ComboThemeConfig, FontSource};

// Re-export types from lcars that this module uses
pub use crate::display_configs::lcars::SplitOrientation;

/// Border style for the steampunk frame
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum BorderStyle {
    /// Ornate Victorian frame with flourishes
    #[default]
    Victorian,
    /// Industrial pipe frame
    PipeFrame,
    /// Riveted metal plate frame
    Riveted,
    /// Simple brass border
    Brass,
    /// Gear-accented border
    GearBorder,
}

/// Corner decoration style
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum CornerStyle {
    /// Decorative gear in each corner
    #[default]
    Gear,
    /// Victorian flourish
    Flourish,
    /// Brass bolt/rivet
    Rivet,
    /// Pipe elbow/joint
    PipeJoint,
    /// No corner decoration
    None,
}

/// Background texture style
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum BackgroundTexture {
    /// Brushed brass texture
    #[default]
    BrushedBrass,
    /// Weathered copper with patina
    Patina,
    /// Dark leather with stitching
    Leather,
    /// Riveted metal plate
    MetalPlate,
    /// Solid color
    Solid,
}

/// Header display style
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum HeaderStyle {
    /// Brass nameplate with rivets
    #[default]
    Nameplate,
    /// Victorian banner with flourishes
    Banner,
    /// Industrial label plate
    Industrial,
    /// No header
    None,
}

/// Divider style between content groups
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum DividerStyle {
    /// Pipe with pressure gauges
    #[default]
    Pipe,
    /// Gear chain
    GearChain,
    /// Simple riveted bar
    RivetedBar,
    /// Victorian ornament line
    Ornament,
    /// No divider
    None,
}

fn default_border_width() -> f64 {
    4.0
}
fn default_corner_size() -> f64 {
    28.0
}
fn default_rivet_size() -> f64 {
    6.0
}
fn default_rivet_spacing() -> f64 {
    24.0
}
fn default_content_padding() -> f64 {
    14.0
}
fn default_divider_width() -> f64 {
    8.0
}
fn default_divider_padding() -> f64 {
    6.0
}
fn default_group_count() -> usize {
    2
}
fn default_gear_teeth() -> usize {
    12
}
fn default_patina_intensity() -> f64 {
    0.3
}

// ColorSource defaults for theme-aware fields
fn default_border_color_source() -> ColorSource {
    ColorSource::theme(1) // Brass
}

fn default_accent_color_source() -> ColorSource {
    ColorSource::theme(2) // Copper
}

fn default_background_color_source() -> ColorSource {
    ColorSource::theme(4) // Dark brown
}

fn default_rivet_color_source() -> ColorSource {
    ColorSource::theme(3) // Bronze
}

fn default_header_color_source() -> ColorSource {
    ColorSource::theme(1) // Brass
}

fn default_divider_color_source() -> ColorSource {
    ColorSource::theme(2) // Copper
}

fn default_patina_color_source() -> ColorSource {
    ColorSource::Custom {
        color: Color::new(0.2, 0.5, 0.4, 0.4),
    } // Verdigris
}

fn default_header_font_source() -> FontSource {
    FontSource::theme(1, 16.0)
}

fn default_steampunk_theme() -> ComboThemeConfig {
    ComboThemeConfig::default_for_steampunk()
}

fn default_animation_enabled() -> bool {
    true
}
fn default_animation_speed() -> f64 {
    8.0
}

/// Main configuration for the Steampunk frame
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteampunkFrameConfig {
    // Frame styling
    #[serde(default)]
    pub border_style: BorderStyle,
    #[serde(default = "default_border_width")]
    pub border_width: f64,
    #[serde(
        default = "default_border_color_source",
        deserialize_with = "deserialize_color_or_source"
    )]
    pub border_color: ColorSource,

    // Corner decorations
    #[serde(default)]
    pub corner_style: CornerStyle,
    #[serde(default = "default_corner_size")]
    pub corner_size: f64,
    #[serde(
        default = "default_accent_color_source",
        deserialize_with = "deserialize_color_or_source"
    )]
    pub accent_color: ColorSource,
    #[serde(default = "default_gear_teeth")]
    pub gear_teeth: usize,

    // Background
    #[serde(
        default = "default_background_color_source",
        deserialize_with = "deserialize_color_or_source"
    )]
    pub background_color: ColorSource,
    #[serde(default)]
    pub background_texture: BackgroundTexture,
    #[serde(
        default = "default_patina_color_source",
        deserialize_with = "deserialize_color_or_source"
    )]
    pub patina_color: ColorSource,
    #[serde(default = "default_patina_intensity")]
    pub patina_intensity: f64,

    // Rivets
    #[serde(default)]
    pub show_rivets: bool,
    #[serde(default = "default_rivet_size")]
    pub rivet_size: f64,
    #[serde(default = "default_rivet_spacing")]
    pub rivet_spacing: f64,
    #[serde(
        default = "default_rivet_color_source",
        deserialize_with = "deserialize_color_or_source"
    )]
    pub rivet_color: ColorSource,

    // Header
    #[serde(default)]
    pub show_header: bool,
    #[serde(default)]
    pub header_text: String,
    #[serde(
        default = "default_header_font_source",
        deserialize_with = "deserialize_font_or_source"
    )]
    pub header_font: FontSource,
    #[serde(
        default = "default_header_color_source",
        deserialize_with = "deserialize_color_or_source"
    )]
    pub header_color: ColorSource,
    #[serde(default)]
    pub header_style: HeaderStyle,

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
    #[serde(default)]
    pub group_item_orientations: Vec<SplitOrientation>,

    // Dividers
    #[serde(default)]
    pub divider_style: DividerStyle,
    #[serde(
        default = "default_divider_color_source",
        deserialize_with = "deserialize_color_or_source"
    )]
    pub divider_color: ColorSource,
    #[serde(default = "default_divider_width")]
    pub divider_width: f64,
    #[serde(default = "default_divider_padding")]
    pub divider_padding: f64,

    // Content items (per slot)
    #[serde(default)]
    pub content_items: HashMap<String, ContentItemConfig>,

    // Animation
    #[serde(default = "default_animation_enabled")]
    pub animation_enabled: bool,
    #[serde(default = "default_animation_speed")]
    pub animation_speed: f64,

    /// Theme configuration
    #[serde(default = "default_steampunk_theme")]
    pub theme: ComboThemeConfig,
}

impl Default for SteampunkFrameConfig {
    fn default() -> Self {
        Self {
            border_style: BorderStyle::default(),
            border_width: default_border_width(),
            border_color: default_border_color_source(),
            corner_style: CornerStyle::default(),
            corner_size: default_corner_size(),
            accent_color: default_accent_color_source(),
            gear_teeth: default_gear_teeth(),
            background_color: default_background_color_source(),
            background_texture: BackgroundTexture::default(),
            patina_color: default_patina_color_source(),
            patina_intensity: default_patina_intensity(),
            show_rivets: true,
            rivet_size: default_rivet_size(),
            rivet_spacing: default_rivet_spacing(),
            rivet_color: default_rivet_color_source(),
            show_header: false,
            header_text: String::new(),
            header_font: default_header_font_source(),
            header_color: default_header_color_source(),
            header_style: HeaderStyle::default(),
            content_padding: default_content_padding(),
            group_count: default_group_count(),
            group_item_counts: vec![1, 1],
            group_size_weights: vec![1.0, 1.0],
            split_orientation: SplitOrientation::default(),
            group_item_orientations: Vec::new(),
            divider_style: DividerStyle::default(),
            divider_color: default_divider_color_source(),
            divider_width: default_divider_width(),
            divider_padding: default_divider_padding(),
            content_items: HashMap::new(),
            animation_enabled: default_animation_enabled(),
            animation_speed: default_animation_speed(),
            theme: default_steampunk_theme(),
        }
    }
}

impl LayoutFrameConfig for SteampunkFrameConfig {
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

impl ThemedFrameConfig for SteampunkFrameConfig {
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

impl ComboFrameConfig for SteampunkFrameConfig {
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
