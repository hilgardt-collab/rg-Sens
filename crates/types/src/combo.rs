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

/// Reorder helpers for slot/content maps keyed by `"group{N}_{M}"` (1-based).
///
/// Both the source config (`slots`) and every themed display config
/// (`content_items`) key their per-slot data by the same `"group{N}_{M}"`
/// scheme, so these pure functions are the single source of truth for keeping
/// the two sides in lock-step when groups or items are reordered.
pub mod reorder {
    use std::collections::HashMap;

    /// Swap the group prefix of every key between two 1-based group numbers.
    ///
    /// A key like `"group2_1"` becomes `"group3_1"` (and vice-versa) when
    /// swapping groups 2 and 3. All other keys are preserved untouched.
    ///
    /// Two correctness details:
    /// - Matching uses the trailing `'_'` (`"group2_"`), so `"group1_"` never
    ///   matches `"group11_3"` — essential now that up to 16 groups exist.
    /// - A fresh map is built rather than renaming in place, so a swap can
    ///   never clobber a key mid-iteration.
    pub fn swap_group_prefixes<T>(map: &mut HashMap<String, T>, group_a: usize, group_b: usize) {
        if group_a == group_b {
            return;
        }
        let prefix_a = format!("group{}_", group_a);
        let prefix_b = format!("group{}_", group_b);
        let old = std::mem::take(map);
        let mut fresh = HashMap::with_capacity(old.len());
        for (key, value) in old {
            let new_key = if let Some(rest) = key.strip_prefix(&prefix_a) {
                format!("group{}_{}", group_b, rest)
            } else if let Some(rest) = key.strip_prefix(&prefix_b) {
                format!("group{}_{}", group_a, rest)
            } else {
                key
            };
            fresh.insert(new_key, value);
        }
        *map = fresh;
    }

    /// Remap a single `"group{N}_{M}"` slot reference after swapping two
    /// 1-based groups. For configs (like CSS template mappings) that store the
    /// slot name as a plain string rather than a map key. Uses the same
    /// trailing-`_` prefix matching as [`swap_group_prefixes`].
    pub fn remap_group_swap(slot: &str, group_a: usize, group_b: usize) -> String {
        if group_a == group_b {
            return slot.to_string();
        }
        let prefix_a = format!("group{}_", group_a);
        let prefix_b = format!("group{}_", group_b);
        if let Some(rest) = slot.strip_prefix(&prefix_a) {
            format!("group{}_{}", group_b, rest)
        } else if let Some(rest) = slot.strip_prefix(&prefix_b) {
            format!("group{}_{}", group_a, rest)
        } else {
            slot.to_string()
        }
    }

    /// Remap a single `"group{g}_{i}"` slot reference after swapping two 1-based
    /// items within a group. String counterpart of [`swap_item_keys`].
    pub fn remap_item_swap(slot: &str, group: usize, item_i: usize, item_j: usize) -> String {
        if item_i == item_j {
            return slot.to_string();
        }
        let key_i = format!("group{}_{}", group, item_i);
        let key_j = format!("group{}_{}", group, item_j);
        if slot == key_i {
            key_j
        } else if slot == key_j {
            key_i
        } else {
            slot.to_string()
        }
    }

    /// Swap two item slots within the same 1-based group.
    ///
    /// Exchanges the values at `"group{g}_{i}"` and `"group{g}_{j}"`. Missing
    /// entries (a slot that has no per-item config yet) are handled gracefully.
    pub fn swap_item_keys<T>(map: &mut HashMap<String, T>, group: usize, item_i: usize, item_j: usize) {
        if item_i == item_j {
            return;
        }
        let key_i = format!("group{}_{}", group, item_i);
        let key_j = format!("group{}_{}", group, item_j);
        let value_i = map.remove(&key_i);
        let value_j = map.remove(&key_j);
        if let Some(value) = value_j {
            map.insert(key_i, value);
        }
        if let Some(value) = value_i {
            map.insert(key_j, value);
        }
    }
}

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

    /// Swap two groups (0-based indices), moving all of their display config
    /// with them: the parallel per-group vectors (size weights, item
    /// orientations, item counts) and every `"group{N}_*"` content item.
    ///
    /// This mirrors [`crate::source_configs::ComboSourceConfig::swap_groups`]
    /// exactly so the display config never desyncs from the source config.
    fn swap_groups(&mut self, group_a: usize, group_b: usize) {
        if group_a == group_b {
            return;
        }
        // Mirror the source side's all-or-nothing range check (it aborts when
        // either index is out of range) so the two configs never make different
        // swap/no-swap decisions and desync. The per-group `size_weights` vector
        // has one entry per group, so its length is the authoritative group count.
        let group_count = self.group_size_weights().len();
        if group_a >= group_count || group_b >= group_count {
            return;
        }
        // Parallel per-group vectors (only swap when both indices are present).
        let weights = self.group_size_weights_mut();
        if group_a < weights.len() && group_b < weights.len() {
            weights.swap(group_a, group_b);
        }
        let orientations = self.group_item_orientations_mut();
        if group_a < orientations.len() && group_b < orientations.len() {
            orientations.swap(group_a, group_b);
        }
        let counts = self.group_item_counts_mut();
        if group_a < counts.len() && group_b < counts.len() {
            counts.swap(group_a, group_b);
        }
        // Per-slot content items are keyed by 1-based group number.
        reorder::swap_group_prefixes(self.content_items_mut(), group_a + 1, group_b + 1);
    }

    /// Swap two items within a group (all 0-based), moving their display config
    /// with them. Mirrors
    /// [`crate::source_configs::ComboSourceConfig::swap_items`].
    fn swap_items(&mut self, group: usize, item_i: usize, item_j: usize) {
        reorder::swap_item_keys(self.content_items_mut(), group + 1, item_i + 1, item_j + 1);
    }
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

#[cfg(test)]
mod reorder_tests {
    use super::reorder::{swap_group_prefixes, swap_item_keys};
    use std::collections::HashMap;

    fn map(keys: &[&str]) -> HashMap<String, String> {
        keys.iter()
            .map(|k| (k.to_string(), k.to_string()))
            .collect()
    }

    #[test]
    fn swaps_group_prefixes_with_asymmetric_item_counts() {
        // group2 has 2 items, group3 has 5 items.
        let mut m = map(&[
            "group2_1", "group2_2", "group3_1", "group3_2", "group3_3", "group3_4", "group3_5",
        ]);
        swap_group_prefixes(&mut m, 2, 3);
        // Every old group2 key now lives under group3 carrying its original value.
        assert_eq!(m.get("group3_1").unwrap(), "group2_1");
        assert_eq!(m.get("group3_2").unwrap(), "group2_2");
        assert_eq!(m.get("group2_1").unwrap(), "group3_1");
        assert_eq!(m.get("group2_5").unwrap(), "group3_5");
        assert_eq!(m.len(), 7);
    }

    #[test]
    fn does_not_collide_group1_with_group11() {
        // Prefix-collision trap: "group1_" must not match "group11_".
        let mut m = map(&["group1_1", "group11_1", "group11_2"]);
        swap_group_prefixes(&mut m, 1, 2);
        // group1 became group2; group11 is untouched.
        assert_eq!(m.get("group2_1").unwrap(), "group1_1");
        assert_eq!(m.get("group11_1").unwrap(), "group11_1");
        assert_eq!(m.get("group11_2").unwrap(), "group11_2");
        assert!(!m.contains_key("group1_1"));
    }

    #[test]
    fn swaps_high_numbered_groups() {
        let mut m = map(&["group1_1", "group11_1", "group16_1"]);
        swap_group_prefixes(&mut m, 11, 16);
        assert_eq!(m.get("group16_1").unwrap(), "group11_1");
        assert_eq!(m.get("group11_1").unwrap(), "group16_1");
        assert_eq!(m.get("group1_1").unwrap(), "group1_1");
    }

    #[test]
    fn swap_item_keys_handles_missing_entry() {
        // Only one of the two slots has stored config.
        let mut m = map(&["group2_1"]);
        swap_item_keys(&mut m, 2, 1, 2);
        assert_eq!(m.get("group2_2").unwrap(), "group2_1");
        assert!(!m.contains_key("group2_1"));
        assert_eq!(m.len(), 1);
    }

    #[test]
    fn swap_item_keys_exchanges_both() {
        let mut m = map(&["group2_1", "group2_2", "group2_3"]);
        swap_item_keys(&mut m, 2, 1, 3);
        assert_eq!(m.get("group2_1").unwrap(), "group2_3");
        assert_eq!(m.get("group2_3").unwrap(), "group2_1");
        assert_eq!(m.get("group2_2").unwrap(), "group2_2");
    }

    #[test]
    fn remap_group_swap_string_references() {
        use super::reorder::remap_group_swap;
        assert_eq!(remap_group_swap("group2_1", 2, 3), "group3_1");
        assert_eq!(remap_group_swap("group3_4", 2, 3), "group2_4");
        assert_eq!(remap_group_swap("group5_1", 2, 3), "group5_1");
        // No prefix collision: group1_ must not match group11_.
        assert_eq!(remap_group_swap("group11_2", 1, 2), "group11_2");
        assert_eq!(remap_group_swap("group1_2", 1, 2), "group2_2");
    }

    #[test]
    fn remap_item_swap_string_references() {
        use super::reorder::remap_item_swap;
        assert_eq!(remap_item_swap("group2_1", 2, 1, 3), "group2_3");
        assert_eq!(remap_item_swap("group2_3", 2, 1, 3), "group2_1");
        assert_eq!(remap_item_swap("group2_2", 2, 1, 3), "group2_2");
        // Different group untouched.
        assert_eq!(remap_item_swap("group3_1", 2, 1, 3), "group3_1");
    }

    /// End-to-end check of the actual feature requirement: when groups are
    /// reordered, the per-slot DISPLAY config must move with its SOURCE. Builds
    /// asymmetric groups (2 vs 3 items), applies the same swap both configs get
    /// from the reorder buttons, and asserts they stay aligned — including the
    /// LCARS `Vec<u32>` `group_item_counts` (the shadow-field trap).
    #[test]
    fn display_config_follows_source_through_group_swap() {
        use crate::combo::ComboFrameConfig;
        use crate::display_configs::lcars::{ContentItemConfig, LcarsFrameConfig};
        use crate::source_configs::combo::{ComboSourceConfig, GroupConfig, SlotConfig};

        let groups = [(1usize, 2usize), (2usize, 3usize)];

        // SOURCE: group1 -> 2 items, group2 -> 3 items, each slot a unique id.
        let mut src = ComboSourceConfig {
            groups: vec![
                GroupConfig {
                    item_count: 2,
                    ..Default::default()
                },
                GroupConfig {
                    item_count: 3,
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        for &(g, n) in &groups {
            for i in 1..=n {
                src.slots.insert(
                    format!("group{}_{}", g, i),
                    SlotConfig {
                        source_id: format!("src_{}_{}", g, i),
                        ..Default::default()
                    },
                );
            }
        }

        // DISPLAY (LCARS): item_height is the per-slot "color" — a marker that
        // must follow the source. Group weights/counts mirror the source.
        // (Built via default() + assignment: LcarsFrameConfig has a private
        // shadow field, so an external struct literal isn't allowed.)
        let mut frame = LcarsFrameConfig::default();
        frame.group_count = 2;
        frame.group_item_counts = vec![2, 3];
        frame.group_size_weights = vec![1.0, 2.0];
        frame.content_items.clear();
        for &(g, n) in &groups {
            for i in 1..=n {
                frame.content_items.insert(
                    format!("group{}_{}", g, i),
                    ContentItemConfig {
                        item_height: (g * 100 + i) as f64,
                        ..Default::default()
                    },
                );
            }
        }

        // What the reorder buttons do: same swap on both sides.
        src.swap_groups(0, 1);
        frame.swap_groups(0, 1);

        // SOURCE: group1 now holds the old group2 sources (3 of them).
        assert_eq!(src.groups[0].item_count, 3);
        assert_eq!(src.groups[1].item_count, 2);
        assert_eq!(src.slots["group1_3"].source_id, "src_2_3");
        assert_eq!(src.slots["group2_2"].source_id, "src_1_2");

        // DISPLAY: the per-slot setting (item_height) followed the source, the
        // weights swapped, and the real u32 counts swapped (not just the shadow).
        assert_eq!(frame.group_item_counts, vec![3, 2]);
        assert_eq!(frame.group_size_weights, vec![2.0, 1.0]);
        assert_eq!(frame.content_items["group1_3"].item_height, 203.0); // was group2_3
        assert_eq!(frame.content_items["group2_2"].item_height, 102.0); // was group1_2
        // No leftover keys at the old asymmetric position.
        assert!(!frame.content_items.contains_key("group2_3"));
    }
}
