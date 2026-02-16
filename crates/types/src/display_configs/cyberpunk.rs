//! Cyberpunk/Neon HUD display configuration types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::color::Color;
use crate::combo::{ComboFrameConfig, LayoutFrameConfig, ThemedFrameConfig};
use crate::display_configs::lcars::{ContentItemConfig, SplitOrientation};
use crate::theme::{deserialize_color_or_source, deserialize_font_or_source, ColorSource, ComboThemeConfig, FontSource};

// ============================================================================
// Enums
// ============================================================================

/// Corner style for the frame
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum CornerStyle {
    /// 45 degree chamfered corners (default)
    #[default]
    Chamfer,
    /// Corner bracket [ ] decorations
    Bracket,
    /// Sharp angular pointed corners
    Angular,
}

/// Header display style
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum HeaderStyle {
    /// Brackets around title
    #[default]
    Brackets,
    /// Title with underline
    Underline,
    /// Boxed header
    Box,
    /// No header
    None,
}

/// Divider style between content groups
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum DividerStyle {
    /// Solid line
    #[default]
    Line,
    /// Dashed line
    Dashed,
    /// Line with glow effect
    Glow,
    /// Dotted line
    Dots,
    /// No divider
    None,
}

// ============================================================================
// Default functions
// ============================================================================

fn default_border_width() -> f64 {
    2.0
}
fn default_glow_intensity() -> f64 {
    0.6
}
fn default_corner_size() -> f64 {
    12.0
}
fn default_grid_spacing() -> f64 {
    20.0
}
fn default_scanline_opacity() -> f64 {
    0.08
}
fn default_content_padding() -> f64 {
    10.0
}
fn default_divider_width() -> f64 {
    1.0
}
fn default_divider_padding() -> f64 {
    4.0
}
fn default_group_count() -> usize {
    2
}

// ColorSource defaults for theme-aware fields
fn default_border_color_source() -> ColorSource {
    ColorSource::theme(1) // Theme color 1 (primary)
}

fn default_background_color_source() -> ColorSource {
    ColorSource::custom(Color {
        r: 0.04,
        g: 0.06,
        b: 0.1,
        a: 0.9,
    }) // Dark blue-black
}

fn default_grid_color_source() -> ColorSource {
    ColorSource::theme(2) // Theme color 2 (secondary) with low opacity handled in render
}

fn default_header_color_source() -> ColorSource {
    ColorSource::theme(1) // Theme color 1
}

fn default_divider_color_source() -> ColorSource {
    ColorSource::theme(1) // Theme color 1
}

fn default_item_frame_color_source() -> ColorSource {
    ColorSource::theme(1) // Theme color 1 with low opacity handled in render
}

fn default_header_font_source() -> FontSource {
    FontSource::theme(1, 18.0) // Theme font 1, 18pt
}

fn default_animation_enabled() -> bool {
    true
}
fn default_animation_speed() -> f64 {
    8.0
}

fn default_cyberpunk_theme() -> ComboThemeConfig {
    ComboThemeConfig::default_for_cyberpunk()
}

fn default_true() -> bool {
    true
}

// ============================================================================
// CyberpunkFrameConfig
// ============================================================================

/// Main configuration for the Cyberpunk frame
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CyberpunkFrameConfig {
    // Frame styling
    #[serde(default = "default_border_width")]
    pub border_width: f64,
    /// Theme-aware border color (replaces border_color)
    #[serde(
        default = "default_border_color_source",
        deserialize_with = "deserialize_color_or_source"
    )]
    pub border_color: ColorSource,
    #[serde(default = "default_glow_intensity")]
    pub glow_intensity: f64,
    #[serde(default)]
    pub corner_style: CornerStyle,
    #[serde(default = "default_corner_size")]
    pub corner_size: f64,

    // Background
    /// Theme-aware background color (replaces background_color)
    #[serde(
        default = "default_background_color_source",
        deserialize_with = "deserialize_color_or_source"
    )]
    pub background_color: ColorSource,
    #[serde(default = "default_true")]
    pub show_grid: bool,
    /// Theme-aware grid color (replaces grid_color)
    #[serde(
        default = "default_grid_color_source",
        deserialize_with = "deserialize_color_or_source"
    )]
    pub grid_color: ColorSource,
    #[serde(default = "default_grid_spacing")]
    pub grid_spacing: f64,

    // Scanline effect
    #[serde(default = "default_true")]
    pub show_scanlines: bool,
    #[serde(default = "default_scanline_opacity")]
    pub scanline_opacity: f64,

    // Header
    #[serde(default)]
    pub show_header: bool,
    #[serde(default)]
    pub header_text: String,
    /// Theme-aware header font (replaces header_font)
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
    /// Theme-aware divider color (replaces divider_color)
    #[serde(
        default = "default_divider_color_source",
        deserialize_with = "deserialize_color_or_source"
    )]
    pub divider_color: ColorSource,
    #[serde(default = "default_divider_width")]
    pub divider_width: f64,
    /// Padding above and below dividers (in pixels)
    #[serde(default = "default_divider_padding")]
    pub divider_padding: f64,

    // Content item framing
    #[serde(default)]
    pub item_frame_enabled: bool,
    /// Theme-aware item frame color (replaces item_frame_color)
    #[serde(
        default = "default_item_frame_color_source",
        deserialize_with = "deserialize_color_or_source"
    )]
    pub item_frame_color: ColorSource,
    #[serde(default)]
    pub item_glow_enabled: bool,

    // Content items (per slot)
    #[serde(default)]
    pub content_items: HashMap<String, ContentItemConfig>,

    // Animation
    #[serde(default = "default_animation_enabled")]
    pub animation_enabled: bool,
    #[serde(default = "default_animation_speed")]
    pub animation_speed: f64,

    /// Theme configuration
    #[serde(default = "default_cyberpunk_theme")]
    pub theme: ComboThemeConfig,
}

// ============================================================================
// Default impl
// ============================================================================

impl Default for CyberpunkFrameConfig {
    fn default() -> Self {
        Self {
            border_width: default_border_width(),
            border_color: default_border_color_source(),
            glow_intensity: default_glow_intensity(),
            corner_style: CornerStyle::default(),
            corner_size: default_corner_size(),
            background_color: default_background_color_source(),
            show_grid: true,
            grid_color: default_grid_color_source(),
            grid_spacing: default_grid_spacing(),
            show_scanlines: true,
            scanline_opacity: default_scanline_opacity(),
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
            item_frame_enabled: false,
            item_frame_color: default_item_frame_color_source(),
            item_glow_enabled: false,
            content_items: HashMap::new(),
            animation_enabled: default_animation_enabled(),
            animation_speed: default_animation_speed(),
            theme: default_cyberpunk_theme(),
        }
    }
}

// ============================================================================
// Trait implementations
// ============================================================================

impl LayoutFrameConfig for CyberpunkFrameConfig {
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

impl ThemedFrameConfig for CyberpunkFrameConfig {
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

impl ComboFrameConfig for CyberpunkFrameConfig {
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
