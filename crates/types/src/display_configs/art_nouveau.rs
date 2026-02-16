//! Art Nouveau display configuration types
//!
//! Provides an organic, nature-inspired Art Nouveau display with:
//! - Flowing vine and whiplash curve borders
//! - Floral and leaf corner decorations
//! - Wave and tendril dividers
//! - Earthy color schemes (olive, gold, cream)
//! - Organic background patterns

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
    /// Flowing vine border with organic curves
    #[default]
    Vine,
    /// Classic whiplash S-curves
    Whiplash,
    /// Floral/leaf motif border
    Floral,
    /// Simple organic curves
    Organic,
    /// Peacock feather inspired curves
    Peacock,
}

/// Corner decoration style
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum CornerStyle {
    /// Decorative flourish swirl
    #[default]
    Flourish,
    /// Leaf/petal corner decoration
    Leaf,
    /// Spiral tendril
    Spiral,
    /// Simple curved bracket
    Bracket,
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
    /// Subtle vine pattern
    Vines,
    /// Scattered leaf pattern
    Leaves,
    /// Flowing wave lines
    Waves,
    /// Peacock feather pattern
    Peacock,
}

/// Header display style
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum HeaderStyle {
    /// Flowing banner shape
    #[default]
    Banner,
    /// Organic arch header
    Arch,
    /// Header with flourish ends
    Flourish,
    /// No header
    None,
}

/// Divider style between content groups
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum DividerStyle {
    /// Vine with leaf offshoots
    #[default]
    Vine,
    /// Flowing wave pattern
    Wave,
    /// Curling tendril
    Tendril,
    /// Simple curved line
    Line,
    /// No divider
    None,
}

fn default_border_width() -> f64 {
    3.0
}
fn default_corner_size() -> f64 {
    28.0
}
fn default_accent_width() -> f64 {
    2.0
}
fn default_pattern_spacing() -> f64 {
    24.0
}
fn default_content_padding() -> f64 {
    12.0
}
fn default_divider_width() -> f64 {
    2.0
}
fn default_divider_padding() -> f64 {
    8.0
}
fn default_group_count() -> usize {
    2
}
fn default_wave_frequency() -> f64 {
    3.0
}

// ColorSource defaults for theme-aware fields
fn default_border_color_source() -> ColorSource {
    ColorSource::theme(1) // Olive green
}

fn default_accent_color_source() -> ColorSource {
    ColorSource::theme(2) // Goldenrod
}

fn default_background_color_source() -> ColorSource {
    ColorSource::theme(4) // Dark olive
}

fn default_pattern_color_source() -> ColorSource {
    ColorSource::theme(1) // Olive with low opacity
}

fn default_header_color_source() -> ColorSource {
    ColorSource::theme(2) // Goldenrod
}

fn default_divider_color_source() -> ColorSource {
    ColorSource::theme(1) // Olive
}

fn default_header_font_source() -> FontSource {
    FontSource::theme(1, 16.0) // Theme font 1, 16pt
}

fn default_art_nouveau_theme() -> ComboThemeConfig {
    ComboThemeConfig::default_for_art_nouveau()
}

fn default_animation_enabled() -> bool {
    true
}

fn default_animation_speed() -> f64 {
    8.0
}

/// Main configuration for the Art Nouveau frame
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtNouveauFrameConfig {
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
    #[serde(default = "default_wave_frequency")]
    pub wave_frequency: f64,

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
    #[serde(default = "default_art_nouveau_theme")]
    pub theme: ComboThemeConfig,

    /// Animation enabled
    #[serde(default = "default_animation_enabled")]
    pub animation_enabled: bool,

    /// Animation speed
    #[serde(default = "default_animation_speed")]
    pub animation_speed: f64,
}

impl Default for ArtNouveauFrameConfig {
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
            wave_frequency: default_wave_frequency(),
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
            theme: default_art_nouveau_theme(),
            animation_enabled: default_animation_enabled(),
            animation_speed: default_animation_speed(),
        }
    }
}

impl LayoutFrameConfig for ArtNouveauFrameConfig {
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

impl ThemedFrameConfig for ArtNouveauFrameConfig {
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

impl ComboFrameConfig for ArtNouveauFrameConfig {
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
