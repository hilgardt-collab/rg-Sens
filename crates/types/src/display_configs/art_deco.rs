//! Art Deco display configuration types
//!
//! Provides a 1920s-inspired Art Deco display with:
//! - Sunburst and fan corner decorations
//! - Stepped/ziggurat border patterns
//! - Chevron dividers and accents
//! - Gold, copper, brass metallic color schemes
//! - Geometric background patterns

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::combo::{ComboFrameConfig, LayoutFrameConfig, ThemedFrameConfig};
use crate::display_configs::lcars::ContentItemConfig;
use crate::theme::{deserialize_color_or_source, deserialize_font_or_source, ColorSource, ComboThemeConfig, FontSource};

// Re-export types from lcars that this module uses
pub use crate::display_configs::lcars::SplitOrientation;

/// Border style for the frame
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum BorderStyle {
    /// Sunburst radiating lines from corners
    #[default]
    Sunburst,
    /// V-pattern chevron border
    Chevron,
    /// Stepped ziggurat-style edges
    Stepped,
    /// Simple geometric lines
    Geometric,
    /// Full ornate frame with multiple elements
    Ornate,
}

/// Corner decoration style
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum CornerStyle {
    /// Radiating fan pattern
    #[default]
    Fan,
    /// Stepped pyramid/ziggurat
    Ziggurat,
    /// Diamond accent
    Diamond,
    /// Simple L-bracket
    Bracket,
    /// Hexagon medallion with extending lines
    Hexagon,
    /// Octagon medallion with extending lines
    Octagon,
    /// Circle medallion with extending lines
    Circle,
    /// Double-line L bracket with inner step
    DoubleBracket,
    /// Stacked geometric shapes (diamond, circle, lines)
    GeometricStack,
    /// No corner decoration
    None,
}

/// Background pattern
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum BackgroundPattern {
    /// Solid color background
    #[default]
    Solid,
    /// Vertical pinstripes
    VerticalLines,
    /// Diamond grid pattern
    DiamondGrid,
    /// Radial sunburst from center
    Sunburst,
    /// Chevron/arrow pattern
    Chevrons,
}

/// Header display style
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum HeaderStyle {
    /// Centered with decorative side elements
    #[default]
    Centered,
    /// Full-width banner bar
    Banner,
    /// Stepped header with tiered effect
    Stepped,
    /// No header
    None,
}

/// Divider style between content groups
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum DividerStyle {
    /// Chevron/arrow pattern divider
    #[default]
    Chevron,
    /// Double line with diamond center
    DoubleLine,
    /// Simple solid line
    Line,
    /// Stepped zigzag pattern
    Stepped,
    /// Stacked/overlapping diamond cluster
    DiamondCluster,
    /// Crescent moon with dots
    Crescent,
    /// Arrows pointing to central diamond
    ArrowDiamond,
    /// Three circles connected by lines
    CircleChain,
    /// Crossed/woven lines pattern
    CrossedLines,
    /// Fleur-de-lis / leaf ornament
    FleurDeLis,
    /// Zigzag heartbeat pattern with diamond accents
    Heartbeat,
    /// Interlocked diamond grid pattern
    DiamondGrid,
    /// No divider
    None,
}

fn default_border_width() -> f64 {
    3.0
}
fn default_corner_size() -> f64 {
    24.0
}
fn default_accent_width() -> f64 {
    2.0
}
fn default_pattern_spacing() -> f64 {
    16.0
}
fn default_content_padding() -> f64 {
    12.0
}
fn default_divider_width() -> f64 {
    2.0
}
fn default_divider_padding() -> f64 {
    6.0
}
fn default_group_count() -> usize {
    2
}
fn default_sunburst_rays() -> usize {
    12
}

// ColorSource defaults for theme-aware fields
fn default_border_color_source() -> ColorSource {
    ColorSource::theme(1) // Gold
}

fn default_accent_color_source() -> ColorSource {
    ColorSource::theme(2) // Copper
}

fn default_background_color_source() -> ColorSource {
    ColorSource::theme(4) // Dark charcoal
}

fn default_pattern_color_source() -> ColorSource {
    ColorSource::theme(1) // Gold with low opacity handled in render
}

fn default_header_color_source() -> ColorSource {
    ColorSource::theme(1) // Gold
}

fn default_divider_color_source() -> ColorSource {
    ColorSource::theme(2) // Copper
}

fn default_header_font_source() -> FontSource {
    FontSource::theme(1, 16.0) // Theme font 1, 16pt
}

fn default_art_deco_theme() -> ComboThemeConfig {
    ComboThemeConfig::default_for_art_deco()
}

fn default_animation_enabled() -> bool {
    true
}

fn default_animation_speed() -> f64 {
    8.0
}

/// Main configuration for the Art Deco frame
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtDecoFrameConfig {
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
    #[serde(default = "default_accent_width")]
    pub accent_width: f64,

    // Background
    #[serde(
        default = "default_background_color_source",
        deserialize_with = "deserialize_color_or_source"
    )]
    pub background_color: ColorSource,
    #[serde(default)]
    pub background_pattern: BackgroundPattern,
    #[serde(
        default = "default_pattern_color_source",
        deserialize_with = "deserialize_color_or_source"
    )]
    pub pattern_color: ColorSource,
    #[serde(default = "default_pattern_spacing")]
    pub pattern_spacing: f64,
    #[serde(default = "default_sunburst_rays")]
    pub sunburst_rays: usize,

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
    /// Item orientation within each group - defaults to same as split_orientation
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

    /// Theme configuration
    #[serde(default = "default_art_deco_theme")]
    pub theme: ComboThemeConfig,

    /// Animation enabled
    #[serde(default = "default_animation_enabled")]
    pub animation_enabled: bool,

    /// Animation speed
    #[serde(default = "default_animation_speed")]
    pub animation_speed: f64,
}

impl Default for ArtDecoFrameConfig {
    fn default() -> Self {
        Self {
            border_style: BorderStyle::default(),
            border_width: default_border_width(),
            border_color: default_border_color_source(),
            corner_style: CornerStyle::default(),
            corner_size: default_corner_size(),
            accent_color: default_accent_color_source(),
            accent_width: default_accent_width(),
            background_color: default_background_color_source(),
            background_pattern: BackgroundPattern::default(),
            pattern_color: default_pattern_color_source(),
            pattern_spacing: default_pattern_spacing(),
            sunburst_rays: default_sunburst_rays(),
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
            theme: default_art_deco_theme(),
            animation_enabled: default_animation_enabled(),
            animation_speed: default_animation_speed(),
        }
    }
}

impl LayoutFrameConfig for ArtDecoFrameConfig {
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

impl ThemedFrameConfig for ArtDecoFrameConfig {
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

impl ComboFrameConfig for ArtDecoFrameConfig {
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
