//! Synthwave/Outrun display configuration types
//!
//! Provides a retro-futuristic 80s aesthetic with:
//! - Purple/pink/cyan gradient backgrounds
//! - Neon grid lines (classic 80s grid horizon)
//! - Chrome/metallic text effects
//! - Sunset gradient accents
//! - Retro-futuristic fonts

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::color::Color;
use crate::combo::{ComboFrameConfig, LayoutFrameConfig, ThemedFrameConfig};
use crate::display_configs::lcars::ContentItemConfig;
use crate::theme::ComboThemeConfig;

// Re-export types from lcars that this module uses
pub use crate::display_configs::lcars::SplitOrientation;

/// Color scheme presets
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SynthwaveColorScheme {
    /// Classic purple/pink/cyan
    #[default]
    Classic,
    /// Hot sunset (orange/pink/purple)
    Sunset,
    /// Cool blue/cyan/purple
    NightDrive,
    /// Neon green/cyan/blue (Miami Vice)
    Miami,
    /// Custom colors
    Custom {
        primary: Color,
        secondary: Color,
        accent: Color,
    },
}

impl SynthwaveColorScheme {
    /// Get primary color (usually for main elements)
    pub fn primary(&self) -> Color {
        match self {
            SynthwaveColorScheme::Classic => Color {
                r: 0.58,
                g: 0.0,
                b: 0.83,
                a: 1.0,
            }, // Purple
            SynthwaveColorScheme::Sunset => Color {
                r: 1.0,
                g: 0.4,
                b: 0.0,
                a: 1.0,
            }, // Orange
            SynthwaveColorScheme::NightDrive => Color {
                r: 0.1,
                g: 0.1,
                b: 0.4,
                a: 1.0,
            }, // Deep blue
            SynthwaveColorScheme::Miami => Color {
                r: 0.0,
                g: 0.9,
                b: 0.7,
                a: 1.0,
            }, // Teal
            SynthwaveColorScheme::Custom { primary, .. } => *primary,
        }
    }

    /// Get secondary color (for gradients)
    pub fn secondary(&self) -> Color {
        match self {
            SynthwaveColorScheme::Classic => Color {
                r: 1.0,
                g: 0.08,
                b: 0.58,
                a: 1.0,
            }, // Hot pink
            SynthwaveColorScheme::Sunset => Color {
                r: 1.0,
                g: 0.0,
                b: 0.5,
                a: 1.0,
            }, // Magenta
            SynthwaveColorScheme::NightDrive => Color {
                r: 0.4,
                g: 0.0,
                b: 0.6,
                a: 1.0,
            }, // Purple
            SynthwaveColorScheme::Miami => Color {
                r: 1.0,
                g: 0.4,
                b: 0.7,
                a: 1.0,
            }, // Pink
            SynthwaveColorScheme::Custom { secondary, .. } => *secondary,
        }
    }

    /// Get accent color (for highlights, neon effects)
    pub fn accent(&self) -> Color {
        match self {
            SynthwaveColorScheme::Classic => Color {
                r: 0.0,
                g: 1.0,
                b: 1.0,
                a: 1.0,
            }, // Cyan
            SynthwaveColorScheme::Sunset => Color {
                r: 0.5,
                g: 0.0,
                b: 0.5,
                a: 1.0,
            }, // Purple
            SynthwaveColorScheme::NightDrive => Color {
                r: 0.0,
                g: 0.9,
                b: 1.0,
                a: 1.0,
            }, // Cyan
            SynthwaveColorScheme::Miami => Color {
                r: 0.0,
                g: 0.5,
                b: 1.0,
                a: 1.0,
            }, // Blue
            SynthwaveColorScheme::Custom { accent, .. } => *accent,
        }
    }

    /// Get neon glow color (typically the brightest)
    pub fn neon(&self) -> Color {
        let accent = self.accent();
        Color {
            r: (accent.r * 1.2).min(1.0),
            g: (accent.g * 1.2).min(1.0),
            b: (accent.b * 1.2).min(1.0),
            a: 1.0,
        }
    }

    /// Get background gradient colors (top, bottom)
    pub fn background_gradient(&self) -> (Color, Color) {
        match self {
            SynthwaveColorScheme::Classic => (
                Color {
                    r: 0.05,
                    g: 0.0,
                    b: 0.15,
                    a: 1.0,
                }, // Dark purple
                Color {
                    r: 0.0,
                    g: 0.0,
                    b: 0.05,
                    a: 1.0,
                }, // Near black
            ),
            SynthwaveColorScheme::Sunset => (
                Color {
                    r: 0.3,
                    g: 0.05,
                    b: 0.15,
                    a: 1.0,
                }, // Dark red
                Color {
                    r: 0.05,
                    g: 0.0,
                    b: 0.1,
                    a: 1.0,
                }, // Dark purple
            ),
            SynthwaveColorScheme::NightDrive => (
                Color {
                    r: 0.0,
                    g: 0.02,
                    b: 0.1,
                    a: 1.0,
                }, // Dark blue
                Color {
                    r: 0.0,
                    g: 0.0,
                    b: 0.02,
                    a: 1.0,
                }, // Near black
            ),
            SynthwaveColorScheme::Miami => (
                Color {
                    r: 0.0,
                    g: 0.1,
                    b: 0.15,
                    a: 1.0,
                }, // Dark teal
                Color {
                    r: 0.05,
                    g: 0.0,
                    b: 0.1,
                    a: 1.0,
                }, // Dark purple
            ),
            SynthwaveColorScheme::Custom {
                primary, secondary, ..
            } => (
                Color {
                    r: primary.r * 0.2,
                    g: primary.g * 0.2,
                    b: primary.b * 0.2,
                    a: 1.0,
                },
                Color {
                    r: secondary.r * 0.1,
                    g: secondary.g * 0.1,
                    b: secondary.b * 0.1,
                    a: 1.0,
                },
            ),
        }
    }
}

/// Frame style for the synthwave display
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SynthwaveFrameStyle {
    /// Neon border with glow
    #[default]
    NeonBorder,
    /// Chrome/metallic frame
    Chrome,
    /// Minimal corner accents
    Minimal,
    /// Double-line retro frame
    RetroDouble,
    /// No frame
    None,
}

/// Grid style for the background
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum GridStyle {
    /// Classic perspective grid (horizon effect)
    #[default]
    Perspective,
    /// Flat grid pattern
    Flat,
    /// Hexagonal grid
    Hexagon,
    /// Scanlines only
    Scanlines,
    /// No grid
    None,
}

/// Header style
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SynthwaveHeaderStyle {
    /// Chrome text with reflection
    #[default]
    Chrome,
    /// Neon glow text
    Neon,
    /// Outlined text
    Outline,
    /// Simple text
    Simple,
    /// No header
    None,
}

/// Divider style between groups
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SynthwaveDividerStyle {
    /// Neon line with glow
    #[default]
    NeonLine,
    /// Gradient fade
    Gradient,
    /// Dotted neon
    NeonDots,
    /// Minimal line
    Line,
    /// No divider
    None,
}

// Default value functions
fn default_grid_spacing() -> f64 {
    30.0
}
fn default_grid_line_width() -> f64 {
    1.0
}
fn default_grid_horizon() -> f64 {
    0.4
}
fn default_grid_perspective() -> f64 {
    0.8
}
fn default_neon_glow() -> f64 {
    0.6
}
fn default_content_padding() -> f64 {
    16.0
}
fn default_header_font() -> String {
    "sans-serif".to_string()
}
fn default_header_font_size() -> f64 {
    16.0
}
fn default_header_height() -> f64 {
    32.0
}
fn default_divider_padding() -> f64 {
    8.0
}
fn default_group_count() -> usize {
    1
}
fn default_frame_width() -> f64 {
    2.0
}
fn default_corner_radius() -> f64 {
    8.0
}
fn default_true() -> bool {
    true
}
fn default_animation_speed() -> f64 {
    8.0
}
fn default_synthwave_theme() -> ComboThemeConfig {
    ComboThemeConfig::default_for_synthwave()
}

/// Main configuration for the Synthwave frame
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthwaveFrameConfig {
    // Color scheme
    #[serde(default)]
    pub color_scheme: SynthwaveColorScheme,
    #[serde(default = "default_neon_glow")]
    pub neon_glow_intensity: f64,

    // Frame styling
    #[serde(default)]
    pub frame_style: SynthwaveFrameStyle,
    #[serde(default = "default_frame_width")]
    pub frame_width: f64,
    #[serde(default = "default_corner_radius")]
    pub corner_radius: f64,

    // Grid background
    #[serde(default)]
    pub grid_style: GridStyle,
    #[serde(default = "default_true")]
    pub show_grid: bool,
    #[serde(default = "default_grid_spacing")]
    pub grid_spacing: f64,
    #[serde(default = "default_grid_line_width")]
    pub grid_line_width: f64,
    #[serde(default = "default_grid_horizon")]
    pub grid_horizon: f64,
    #[serde(default = "default_grid_perspective")]
    pub grid_perspective: f64,

    // Sun/sunset effect
    #[serde(default)]
    pub show_sun: bool,
    #[serde(default)]
    pub sun_position: f64, // 0.0 = bottom, 1.0 = horizon

    // Header
    #[serde(default = "default_true")]
    pub show_header: bool,
    #[serde(default)]
    pub header_text: String,
    #[serde(default)]
    pub header_style: SynthwaveHeaderStyle,
    #[serde(default = "default_header_font")]
    pub header_font: String,
    #[serde(default = "default_header_font_size")]
    pub header_font_size: f64,
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
    #[serde(default)]
    pub item_spacing: f64,

    // Dividers
    #[serde(default)]
    pub divider_style: SynthwaveDividerStyle,
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
    #[serde(default)]
    pub scanline_effect: bool,

    // Theme configuration
    #[serde(default = "default_synthwave_theme")]
    pub theme: ComboThemeConfig,
}

impl Default for SynthwaveFrameConfig {
    fn default() -> Self {
        Self {
            color_scheme: SynthwaveColorScheme::Classic,
            neon_glow_intensity: default_neon_glow(),

            frame_style: SynthwaveFrameStyle::NeonBorder,
            frame_width: default_frame_width(),
            corner_radius: default_corner_radius(),

            grid_style: GridStyle::Perspective,
            show_grid: true,
            grid_spacing: default_grid_spacing(),
            grid_line_width: default_grid_line_width(),
            grid_horizon: default_grid_horizon(),
            grid_perspective: default_grid_perspective(),

            show_sun: true,
            sun_position: 0.3,

            show_header: true,
            header_text: "SYNTHWAVE".to_string(),
            header_style: SynthwaveHeaderStyle::Chrome,
            header_font: default_header_font(),
            header_font_size: default_header_font_size(),
            header_height: default_header_height(),

            content_padding: default_content_padding(),
            group_count: default_group_count(),
            group_item_counts: vec![4],
            group_size_weights: vec![1.0],
            split_orientation: SplitOrientation::Vertical,
            group_item_orientations: Vec::new(),
            item_spacing: 8.0,

            divider_style: SynthwaveDividerStyle::NeonLine,
            divider_padding: default_divider_padding(),

            content_items: HashMap::new(),

            animation_enabled: true,
            animation_speed: default_animation_speed(),
            scanline_effect: false,

            theme: default_synthwave_theme(),
        }
    }
}

impl LayoutFrameConfig for SynthwaveFrameConfig {
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

impl ThemedFrameConfig for SynthwaveFrameConfig {
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

impl ComboFrameConfig for SynthwaveFrameConfig {
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
