//! Combo source configuration types.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;

/// Configuration for a single data source slot
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SlotConfig {
    /// ID of the source to use (e.g., "cpu", "gpu", "memory")
    #[serde(default)]
    pub source_id: String,
    /// Custom caption override (if empty, uses source's default)
    #[serde(default)]
    pub caption_override: String,
    /// Source-specific configuration to pass to the child source
    #[serde(default)]
    pub source_config: HashMap<String, Value>,
}

impl SlotConfig {
    /// The slot's own update interval, when its source config carries one.
    pub fn update_interval_ms(&self) -> Option<u64> {
        self.source_config
            .get("update_interval_ms")
            .and_then(Value::as_u64)
            .filter(|&ms| ms > 0)
    }

    /// `source_config` is the source's own fields, flat. An earlier Scan
    /// Placeholders stored it under the `{source_id}_config` key instead; the
    /// combo source wraps a slot's config under that key itself, so such a
    /// slot ran on its source's defaults. Unwrap it; returns whether it was.
    pub fn unwrap_legacy_config(&mut self) -> bool {
        let key = format!("{}_config", self.source_id);
        if self.source_config.len() != 1 {
            return false;
        }
        match self.source_config.get(&key).and_then(Value::as_object) {
            Some(inner) => {
                self.source_config = inner.clone().into_iter().collect();
                true
            }
            None => false,
        }
    }
}

/// Configuration for a group of content items
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupConfig {
    /// Number of items in this group (1-8)
    #[serde(default = "default_group_item_count")]
    pub item_count: u32,
    /// Relative size weight for this group (like segment height_weight)
    #[serde(default = "default_group_size_weight")]
    pub size_weight: f64,
}

fn default_group_item_count() -> u32 {
    1
}

fn default_group_size_weight() -> f64 {
    1.0
}

impl Default for GroupConfig {
    fn default() -> Self {
        Self {
            item_count: default_group_item_count(),
            size_weight: default_group_size_weight(),
        }
    }
}

fn default_update_interval() -> u64 {
    1000
}

fn default_mode() -> String {
    "lcars".to_string()
}

/// Configuration for the combo source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComboSourceConfig {
    /// Mode determines the slot naming convention (lcars, arc, level_bar)
    #[serde(default = "default_mode")]
    pub mode: String,
    /// Groups configuration (new format) - each group has an item count
    #[serde(default)]
    pub groups: Vec<GroupConfig>,
    /// Legacy: Number of primary content items (for backwards compatibility)
    #[serde(default, skip_serializing)]
    pub primary_count: u32,
    /// Legacy: Number of secondary content items (for backwards compatibility)
    #[serde(default, skip_serializing)]
    pub secondary_count: u32,
    /// Per-slot source configurations, keyed by slot name (e.g., "group1_1", "group2_1")
    #[serde(default)]
    pub slots: HashMap<String, SlotConfig>,
    /// Update interval in milliseconds (how often to refresh all child sources)
    #[serde(default = "default_update_interval")]
    pub update_interval_ms: u64,
}

impl Default for ComboSourceConfig {
    fn default() -> Self {
        Self {
            mode: default_mode(),
            groups: vec![GroupConfig {
                item_count: 2,
                ..Default::default()
            }],
            primary_count: 0,
            secondary_count: 0,
            slots: HashMap::new(),
            update_interval_ms: default_update_interval(),
        }
    }
}

impl ComboSourceConfig {
    /// Migrate legacy primary/secondary config to groups format
    pub fn migrate_legacy(&mut self) {
        for slot in self.slots.values_mut() {
            slot.unwrap_legacy_config();
        }
        if self.groups.is_empty() && (self.primary_count > 0 || self.secondary_count > 0) {
            if self.primary_count > 0 {
                self.groups.push(GroupConfig {
                    item_count: self.primary_count,
                    ..Default::default()
                });
            }
            if self.secondary_count > 0 {
                self.groups.push(GroupConfig {
                    item_count: self.secondary_count,
                    ..Default::default()
                });
            }

            let mut new_slots = HashMap::new();
            for (old_name, config) in &self.slots {
                let new_name = if old_name.starts_with("primary") {
                    let num: String = old_name.chars().filter(|c| c.is_ascii_digit()).collect();
                    format!("group1_{}", num)
                } else if old_name.starts_with("secondary") {
                    let num: String = old_name.chars().filter(|c| c.is_ascii_digit()).collect();
                    format!("group2_{}", num)
                } else {
                    old_name.clone()
                };
                new_slots.insert(new_name, config.clone());
            }
            self.slots = new_slots;

            self.primary_count = 0;
            self.secondary_count = 0;
        }

        if self.groups.is_empty() {
            self.groups.push(GroupConfig {
                item_count: 2,
                ..Default::default()
            });
        }
    }

    /// Get total number of items across all groups
    pub fn total_item_count(&self) -> u32 {
        self.groups.iter().map(|g| g.item_count).sum()
    }

    /// Swap two groups by 0-based index, carrying their slot sources with them.
    ///
    /// Reorders the `groups` vector and renames every `"group{N}_*"` slot key
    /// via the shared [`crate::combo::reorder`] helpers — the same helpers the
    /// display config uses, so the two never desync.
    pub fn swap_groups(&mut self, group_a: usize, group_b: usize) {
        if group_a == group_b || group_a >= self.groups.len() || group_b >= self.groups.len() {
            return;
        }
        self.groups.swap(group_a, group_b);
        crate::combo::reorder::swap_group_prefixes(&mut self.slots, group_a + 1, group_b + 1);
    }

    /// Swap two item slots within a group, all 0-based.
    pub fn swap_items(&mut self, group: usize, item_i: usize, item_j: usize) {
        crate::combo::reorder::swap_item_keys(&mut self.slots, group + 1, item_i + 1, item_j + 1);
    }

    /// Get update interval as Duration
    pub fn update_interval(&self) -> Duration {
        Duration::from_millis(self.update_interval_ms)
    }

    /// Get the update interval in milliseconds
    pub fn update_interval_ms(&self) -> u64 {
        self.update_interval_ms
    }

    /// The cadence the combo is polled at: its own interval, or a slot's when
    /// one asks for something faster. The combo source refreshes each slot on
    /// its own interval and skips it on the polls in between, so every item
    /// sees the interval it was given.
    pub fn poll_interval_ms(&self) -> u64 {
        self.slots
            .values()
            .filter_map(SlotConfig::update_interval_ms)
            .fold(self.update_interval_ms, u64::min)
    }
}

#[cfg(test)]
mod interval_tests {
    use super::*;

    fn slot(ms: Option<u64>) -> SlotConfig {
        let mut slot = SlotConfig {
            source_id: "cpu".to_string(),
            ..Default::default()
        };
        if let Some(ms) = ms {
            slot.source_config
                .insert("update_interval_ms".to_string(), Value::from(ms));
        }
        slot
    }

    #[test]
    fn the_poll_interval_is_the_fastest_configured() {
        let mut cfg = ComboSourceConfig {
            update_interval_ms: 1000,
            ..Default::default()
        };
        cfg.slots.insert("group1_1".to_string(), slot(None));
        assert_eq!(cfg.poll_interval_ms(), 1000);
        cfg.slots.insert("group1_2".to_string(), slot(Some(5000)));
        assert_eq!(cfg.poll_interval_ms(), 1000);
        cfg.slots.insert("group1_3".to_string(), slot(Some(100)));
        assert_eq!(cfg.poll_interval_ms(), 100);
        assert_eq!(slot(Some(0)).update_interval_ms(), None);
    }
}
