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

    /// Get update interval as Duration
    pub fn update_interval(&self) -> Duration {
        Duration::from_millis(self.update_interval_ms)
    }

    /// Get the update interval in milliseconds
    pub fn update_interval_ms(&self) -> u64 {
        self.update_interval_ms
    }
}
