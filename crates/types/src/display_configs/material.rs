//! Material Design Cards display configuration types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::color::Color;
use crate::combo::{ComboFrameConfig, LayoutFrameConfig, ThemedFrameConfig};
use crate::display_configs::lcars::{ContentItemConfig, SplitOrientation};
use crate::theme::{deserialize_color_or_source, deserialize_font_or_source, ColorSource, ComboThemeConfig, FontSource};

// ============================================================================
// Enums
// ============================================================================

/// Card elevation level (affects shadow intensity)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum CardElevation {
    /// Flat - no shadow
    Flat,
    /// Low elevation - subtle shadow
    #[default]
    Low,
    /// Medium elevation - moderate shadow
    Medium,
    /// High elevation - prominent shadow
    High,
}

/// Header style for cards
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum HeaderStyle {
    /// Colored bar at top
    #[default]
    ColorBar,
    /// Full colored background with white text
    Filled,
    /// Text only with colored text
    TextOnly,
    /// No header
    None,
}

/// Theme variant
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ThemeVariant {
    /// Light theme (white cards on light gray background)
    #[default]
    Light,
    /// Dark theme (dark gray cards on darker background)
    Dark,
    /// Teal theme (teal accent on light background)
    Teal,
    /// Purple theme (deep purple accent on dark background)
    Purple,
}

/// Divider style between groups
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum DividerStyle {
    /// No visible divider, just spacing
    #[default]
    Space,
    /// Thin line divider
    Line,
    /// Subtle gradient fade
    Fade,
}

/// Header text alignment
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum HeaderAlignment {
    /// Left-aligned text (default)
    #[default]
    Left,
    /// Center-aligned text
    Center,
    /// Right-aligned text
    Right,
}

// ============================================================================
// Default functions
// ============================================================================

fn default_corner_radius() -> f64 {
    12.0
}
fn default_card_padding() -> f64 {
    16.0
}
fn default_content_padding() -> f64 {
    20.0
}
fn default_item_spacing() -> f64 {
    12.0
}
fn default_header_height() -> f64 {
    40.0
}
fn default_shadow_blur() -> f64 {
    8.0
}
fn default_shadow_offset_y() -> f64 {
    2.0
}
fn default_divider_spacing() -> f64 {
    16.0
}
fn default_group_count() -> usize {
    2
}

fn default_accent_color() -> ColorSource {
    ColorSource::theme(3) // Theme color 3 (accent)
}

fn default_surface_color_light() -> Color {
    Color {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    } // White
}

fn default_surface_color_dark() -> Color {
    Color {
        r: 0.12,
        g: 0.12,
        b: 0.12,
        a: 1.0,
    } // Dark gray
}

fn default_background_color_light() -> Color {
    Color {
        r: 0.96,
        g: 0.96,
        b: 0.96,
        a: 1.0,
    } // Light gray
}

fn default_background_color_dark() -> Color {
    Color {
        r: 0.06,
        g: 0.06,
        b: 0.06,
        a: 1.0,
    } // Near black
}

fn default_text_color_light() -> Color {
    Color {
        r: 0.13,
        g: 0.13,
        b: 0.13,
        a: 1.0,
    } // Near black
}

fn default_text_color_dark() -> Color {
    Color {
        r: 0.93,
        g: 0.93,
        b: 0.93,
        a: 1.0,
    } // Near white
}

fn default_shadow_color() -> Color {
    Color {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.15,
    } // Subtle black shadow
}

fn default_divider_color() -> ColorSource {
    ColorSource::custom(Color {
        r: 0.5,
        g: 0.5,
        b: 0.5,
        a: 0.2,
    }) // Subtle gray
}

fn default_header_font_source() -> FontSource {
    FontSource::theme(1, 14.0) // Theme font 1
}

fn default_animation_enabled() -> bool {
    true
}
fn default_animation_speed() -> f64 {
    8.0
}

fn default_material_theme() -> ComboThemeConfig {
    ComboThemeConfig::default_for_material()
}

// ============================================================================
// MaterialFrameConfig
// ============================================================================

/// Main configuration for the Material Cards frame
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialFrameConfig {
    // Theme variant (light/dark)
    #[serde(default)]
    pub theme_variant: ThemeVariant,
    #[serde(
        default = "default_accent_color",
        deserialize_with = "deserialize_color_or_source"
    )]
    pub accent_color: ColorSource,

    // Card styling
    #[serde(default)]
    pub elevation: CardElevation,
    #[serde(default = "default_corner_radius")]
    pub corner_radius: f64,
    #[serde(default = "default_card_padding")]
    pub card_padding: f64,
    #[serde(default = "default_shadow_blur")]
    pub shadow_blur: f64,
    #[serde(default = "default_shadow_offset_y")]
    pub shadow_offset_y: f64,
    #[serde(default = "default_shadow_color")]
    pub shadow_color: Color,

    // Surface colors (card background)
    #[serde(default = "default_surface_color_light")]
    pub surface_color_light: Color,
    #[serde(default = "default_surface_color_dark")]
    pub surface_color_dark: Color,

    // Background colors (behind cards)
    #[serde(default = "default_background_color_light")]
    pub background_color_light: Color,
    #[serde(default = "default_background_color_dark")]
    pub background_color_dark: Color,

    // Text colors
    #[serde(default = "default_text_color_light")]
    pub text_color_light: Color,
    #[serde(default = "default_text_color_dark")]
    pub text_color_dark: Color,

    // Header
    #[serde(default)]
    pub show_header: bool,
    #[serde(default)]
    pub header_text: String,
    #[serde(default)]
    pub header_style: HeaderStyle,
    #[serde(
        default = "default_header_font_source",
        deserialize_with = "deserialize_font_or_source"
    )]
    pub header_font: FontSource,
    #[serde(default = "default_header_height")]
    pub header_height: f64,
    #[serde(default)]
    pub header_alignment: HeaderAlignment,

    // Layout
    #[serde(default = "default_content_padding")]
    pub content_padding: f64,
    #[serde(default = "default_item_spacing")]
    pub item_spacing: f64,
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

    /// Per-group accent colors for headers
    #[serde(default)]
    pub group_accent_colors: Vec<Color>,

    /// Per-group header labels
    #[serde(default)]
    pub group_headers: Vec<String>,

    // Dividers
    #[serde(default)]
    pub divider_style: DividerStyle,
    #[serde(
        default = "default_divider_color",
        deserialize_with = "deserialize_color_or_source"
    )]
    pub divider_color: ColorSource,
    #[serde(default = "default_divider_spacing")]
    pub divider_spacing: f64,

    // Content items (per slot)
    #[serde(default)]
    pub content_items: HashMap<String, ContentItemConfig>,

    // Animation
    #[serde(default = "default_animation_enabled")]
    pub animation_enabled: bool,
    #[serde(default = "default_animation_speed")]
    pub animation_speed: f64,

    /// Theme configuration
    #[serde(default = "default_material_theme")]
    pub theme: ComboThemeConfig,
}

// ============================================================================
// Default impl
// ============================================================================

impl Default for MaterialFrameConfig {
    fn default() -> Self {
        Self {
            theme_variant: ThemeVariant::default(),
            accent_color: default_accent_color(),
            elevation: CardElevation::default(),
            corner_radius: default_corner_radius(),
            card_padding: default_card_padding(),
            shadow_blur: default_shadow_blur(),
            shadow_offset_y: default_shadow_offset_y(),
            shadow_color: default_shadow_color(),
            surface_color_light: default_surface_color_light(),
            surface_color_dark: default_surface_color_dark(),
            background_color_light: default_background_color_light(),
            background_color_dark: default_background_color_dark(),
            text_color_light: default_text_color_light(),
            text_color_dark: default_text_color_dark(),
            show_header: false,
            header_text: String::new(),
            header_style: HeaderStyle::default(),
            header_font: default_header_font_source(),
            header_height: default_header_height(),
            header_alignment: HeaderAlignment::default(),
            content_padding: default_content_padding(),
            item_spacing: default_item_spacing(),
            group_count: default_group_count(),
            group_item_counts: vec![1, 1],
            group_size_weights: vec![1.0, 1.0],
            split_orientation: SplitOrientation::default(),
            group_item_orientations: Vec::new(),
            group_accent_colors: Vec::new(),
            group_headers: Vec::new(),
            divider_style: DividerStyle::default(),
            divider_color: default_divider_color(),
            divider_spacing: default_divider_spacing(),
            content_items: HashMap::new(),
            animation_enabled: default_animation_enabled(),
            animation_speed: default_animation_speed(),
            theme: default_material_theme(),
        }
    }
}

// ============================================================================
// Trait implementations
// ============================================================================

impl LayoutFrameConfig for MaterialFrameConfig {
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

impl ThemedFrameConfig for MaterialFrameConfig {
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

impl ComboFrameConfig for MaterialFrameConfig {
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

// ============================================================================
// Helper methods
// ============================================================================

impl MaterialFrameConfig {
    /// Get the surface color based on current theme variant
    pub fn surface_color(&self) -> Color {
        match self.theme_variant {
            ThemeVariant::Light | ThemeVariant::Teal => self.surface_color_light,
            ThemeVariant::Dark | ThemeVariant::Purple => self.surface_color_dark,
        }
    }

    /// Get the background color based on current theme variant
    pub fn background_color(&self) -> Color {
        match self.theme_variant {
            ThemeVariant::Light | ThemeVariant::Teal => self.background_color_light,
            ThemeVariant::Dark | ThemeVariant::Purple => self.background_color_dark,
        }
    }

    /// Get the text color based on current theme variant
    pub fn text_color(&self) -> Color {
        match self.theme_variant {
            ThemeVariant::Light | ThemeVariant::Teal => self.text_color_light,
            ThemeVariant::Dark | ThemeVariant::Purple => self.text_color_dark,
        }
    }

    /// Get accent color for a specific group (resolved through theme)
    pub fn group_accent(&self, group_idx: usize) -> Color {
        self.group_accent_colors
            .get(group_idx)
            .copied()
            .unwrap_or_else(|| self.accent_color.resolve(&self.theme))
    }

    /// Get header text for a specific group
    pub fn group_header(&self, group_idx: usize) -> &str {
        self.group_headers
            .get(group_idx)
            .map(|s| s.as_str())
            .unwrap_or("")
    }
}
