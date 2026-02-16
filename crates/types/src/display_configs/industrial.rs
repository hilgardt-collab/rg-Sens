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
use crate::combo::{ComboFrameConfig, LayoutFrameConfig, ThemedFrameConfig};
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
