//! Combo panel traits and shared types
//!
//! These traits define the interface for combo-style displayer configurations
//! (LCARS, Cyberpunk, Material, Industrial, etc.) that support theming,
//! layout/grouping, and animation.

use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::HashMap;

use crate::display_configs::lcars::{ContentItemConfig, SplitOrientation};
use crate::theme::ComboThemeConfig;

/// Trait for combo panel frame configurations that support theming
pub trait ThemedFrameConfig {
    /// Get reference to the theme configuration
    fn theme(&self) -> &ComboThemeConfig;
    /// Get mutable reference to the theme configuration
    fn theme_mut(&mut self) -> &mut ComboThemeConfig;
    /// Get reference to content items
    fn content_items(&self) -> &HashMap<String, ContentItemConfig>;
    /// Get mutable reference to content items
    fn content_items_mut(&mut self) -> &mut HashMap<String, ContentItemConfig>;
}

/// Trait for combo panel frame configurations that support layout/grouping
pub trait LayoutFrameConfig {
    /// Get the number of groups
    fn group_count(&self) -> usize;
    /// Get reference to group size weights
    fn group_size_weights(&self) -> &Vec<f64>;
    /// Get mutable reference to group size weights
    fn group_size_weights_mut(&mut self) -> &mut Vec<f64>;
    /// Get reference to per-group item orientations
    fn group_item_orientations(&self) -> &Vec<SplitOrientation>;
    /// Get mutable reference to per-group item orientations
    fn group_item_orientations_mut(&mut self) -> &mut Vec<SplitOrientation>;
    /// Get the split orientation (used as default for item orientations)
    fn split_orientation(&self) -> SplitOrientation;
}

/// Trait for combo panel frame configurations with animation support.
/// Combines theming and layout capabilities.
pub trait ComboFrameConfig:
    ThemedFrameConfig
    + LayoutFrameConfig
    + Clone
    + Default
    + Serialize
    + DeserializeOwned
    + Send
    + Sync
    + 'static
{
    /// Get animation enabled state
    fn animation_enabled(&self) -> bool;

    /// Set animation enabled state
    fn set_animation_enabled(&mut self, enabled: bool);

    /// Get animation speed multiplier
    fn animation_speed(&self) -> f64;

    /// Set animation speed multiplier
    fn set_animation_speed(&mut self, speed: f64);

    /// Get the group item counts
    fn group_item_counts(&self) -> &[usize];

    /// Get mutable reference to group item counts
    fn group_item_counts_mut(&mut self) -> &mut Vec<usize>;
}

/// Transferable configuration that can be preserved when switching between combo panel types.
/// This excludes theme-specific settings (colors, fonts, frame styles) but includes
/// layout and content configuration.
#[derive(Debug, Clone, Default)]
pub struct TransferableComboConfig {
    /// Number of groups
    pub group_count: usize,
    /// Number of items in each group
    pub group_item_counts: Vec<u32>,
    /// Size weight for each group
    pub group_size_weights: Vec<f64>,
    /// Item orientation within each group
    pub group_item_orientations: Vec<SplitOrientation>,
    /// Layout orientation (how groups are arranged)
    pub layout_orientation: SplitOrientation,
    /// Content items configuration (keyed by slot name like "group1_1")
    pub content_items: HashMap<String, ContentItemConfig>,
    /// Content padding
    pub content_padding: f64,
    /// Item spacing within groups
    pub item_spacing: f64,
    /// Animation enabled
    pub animation_enabled: bool,
    /// Animation speed
    pub animation_speed: f64,
}

impl TransferableComboConfig {
    /// Check if this config has meaningful content to transfer
    pub fn has_content(&self) -> bool {
        self.group_count > 0 || !self.content_items.is_empty()
    }
}
