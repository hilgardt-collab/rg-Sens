//! Combo data source that aggregates multiple data sources
//!
//! This source is designed for complex displayers (like LCARS) that need
//! to show data from multiple sources simultaneously.

use crate::core::{
    global_registry, DataSource, FieldMetadata, FieldPurpose, FieldType, SourceMetadata,
};
use anyhow::Result;
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

fn default_update_interval() -> u64 {
    1000 // Default 1 second
}

fn default_mode() -> String {
    "lcars".to_string()
}

impl Default for ComboSourceConfig {
    fn default() -> Self {
        Self {
            mode: default_mode(),
            groups: vec![GroupConfig {
                item_count: 2,
                ..Default::default()
            }], // Default: 1 group with 2 items
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
        // Only migrate if groups is empty and we have legacy counts
        if self.groups.is_empty() && (self.primary_count > 0 || self.secondary_count > 0) {
            log::info!("ComboSourceConfig: Migrating legacy primary/secondary to groups format");

            // Convert primary to group 1
            if self.primary_count > 0 {
                self.groups.push(GroupConfig {
                    item_count: self.primary_count,
                    ..Default::default()
                });
            }
            // Convert secondary to group 2
            if self.secondary_count > 0 {
                self.groups.push(GroupConfig {
                    item_count: self.secondary_count,
                    ..Default::default()
                });
            }

            // Migrate slot names: primary1 -> group1_1, secondary1 -> group2_1
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

            // Clear legacy fields
            self.primary_count = 0;
            self.secondary_count = 0;
        }

        // Ensure at least one group exists
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

/// Combo data source that aggregates multiple child sources
///
/// This source manages multiple child data sources and aggregates their
/// values into a single HashMap with prefixed keys.
pub struct ComboSource {
    metadata: SourceMetadata,
    config: ComboSourceConfig,
    /// Child source instances, keyed by slot name
    child_sources: HashMap<String, Box<dyn DataSource>>,
    /// Aggregated values from all children
    values: HashMap<String, Value>,
}

impl ComboSource {
    pub fn new() -> Self {
        let metadata = SourceMetadata {
            id: "combination".to_string(),
            name: "Combination Source".to_string(),
            description: "Aggregates multiple data sources for complex displays".to_string(),
            available_keys: vec![
                // Keys are dynamic based on child sources
                "primary1_caption".to_string(),
                "primary1_value".to_string(),
                "primary1_unit".to_string(),
                "primary1_numerical_value".to_string(),
                "primary1_min_limit".to_string(),
                "primary1_max_limit".to_string(),
            ],
            default_interval: Duration::from_millis(1000),
        };

        Self {
            metadata,
            config: ComboSourceConfig::default(),
            child_sources: HashMap::new(),
            values: HashMap::new(),
        }
    }

    /// Set up child sources based on current configuration
    pub fn setup_child_sources(&mut self) {
        // Clear existing child sources
        self.child_sources.clear();

        // Get slot names based on mode
        let slot_names = self.get_slot_names();

        // Create child sources for each configured slot
        for slot_name in slot_names {
            if let Some(slot_config) = self.config.slots.get(&slot_name) {
                if !slot_config.source_id.is_empty() && slot_config.source_id != "none" {
                    match global_registry().create_source(&slot_config.source_id) {
                        Ok(mut source) => {
                            // Configure the child source
                            if !slot_config.source_config.is_empty() {
                                // The source_config HashMap already contains the flattened config
                                // (e.g., {"field": "usage", "unit": "percent", ...})
                                // Wrap it under the expected key (e.g., "cpu_config")
                                let config_key = format!("{}_config", slot_config.source_id);
                                let mut config_map = HashMap::new();
                                // Convert the HashMap to a Value object
                                let config_obj = Value::Object(
                                    slot_config
                                        .source_config
                                        .iter()
                                        .map(|(k, v)| (k.clone(), v.clone()))
                                        .collect(),
                                );
                                log::info!(
                                    "ComboSource: Configuring '{}' for slot '{}' with config: {:?}",
                                    slot_config.source_id,
                                    slot_name,
                                    config_obj
                                );
                                config_map.insert(config_key, config_obj);
                                if let Err(e) = source.configure(&config_map) {
                                    log::warn!(
                                        "ComboSource: Failed to configure '{}' for slot '{}': {}",
                                        slot_config.source_id,
                                        slot_name,
                                        e
                                    );
                                }
                            }
                            self.child_sources.insert(slot_name.clone(), source);
                            log::debug!(
                                "ComboSource: Created child source '{}' for slot '{}'",
                                slot_config.source_id,
                                slot_name
                            );
                        }
                        Err(e) => {
                            log::warn!(
                                "ComboSource: Failed to create source '{}' for slot '{}': {}",
                                slot_config.source_id,
                                slot_name,
                                e
                            );
                        }
                    }
                }
            }
        }

        log::info!(
            "ComboSource: Set up {} child sources",
            self.child_sources.len()
        );
    }

    /// Get the list of slot names based on configuration
    fn get_slot_names(&self) -> Vec<String> {
        let mut names = Vec::new();

        match self.config.mode.as_str() {
            "lcars" => {
                // Groups with items: group1_1, group1_2, group2_1, etc.
                for (group_idx, group) in self.config.groups.iter().enumerate() {
                    let group_num = group_idx + 1;
                    for item_idx in 1..=group.item_count {
                        names.push(format!("group{}_{}", group_num, item_idx));
                    }
                }
            }
            "arc" => {
                // Center source
                names.push("center".to_string());
                // Arc sources from first group's item count
                let arc_count = self
                    .config
                    .groups
                    .first()
                    .map(|g| g.item_count)
                    .unwrap_or(4);
                for i in 1..=arc_count {
                    names.push(format!("arc{}", i));
                }
            }
            "level_bar" => {
                // Bar sources from first group's item count
                let bar_count = self
                    .config
                    .groups
                    .first()
                    .map(|g| g.item_count)
                    .unwrap_or(4);
                for i in 1..=bar_count {
                    names.push(format!("bar{}", i));
                }
            }
            _ => {
                // Default to lcars pattern
                for (group_idx, group) in self.config.groups.iter().enumerate() {
                    let group_num = group_idx + 1;
                    for item_idx in 1..=group.item_count {
                        names.push(format!("group{}_{}", group_num, item_idx));
                    }
                }
            }
        }

        names
    }

    /// Aggregate values from all child sources
    fn aggregate_values(&mut self) {
        self.values.clear();

        for (slot_name, source) in &self.child_sources {
            let source_values = source.get_values();
            let slot_config = self.config.slots.get(slot_name);

            // Log limit values for debugging
            if let Some(min) = source_values.get("min_limit") {
                log::debug!("ComboSource: slot '{}' min_limit = {:?}", slot_name, min);
            }
            if let Some(max) = source_values.get("max_limit") {
                log::debug!("ComboSource: slot '{}' max_limit = {:?}", slot_name, max);
            }

            // Prefix all keys with the slot name
            for (key, value) in source_values {
                let prefixed_key = format!("{}_{}", slot_name, key);
                self.values.insert(prefixed_key, value);
            }

            // Handle caption override
            if let Some(slot_config) = slot_config {
                if !slot_config.caption_override.is_empty() {
                    let caption_key = format!("{}_caption", slot_name);
                    self.values.insert(
                        caption_key,
                        Value::from(slot_config.caption_override.clone()),
                    );
                }
            }
        }
    }
}

impl Default for ComboSource {
    fn default() -> Self {
        Self::new()
    }
}

impl DataSource for ComboSource {
    fn metadata(&self) -> &SourceMetadata {
        &self.metadata
    }

    fn fields(&self) -> Vec<FieldMetadata> {
        let mut fields = Vec::new();

        // For each child source, get its actual fields and prefix them with slot name
        for (slot_name, source) in &self.child_sources {
            let child_fields = source.fields();
            for field in child_fields {
                fields.push(FieldMetadata::new(
                    format!("{}_{}", slot_name, field.id),
                    format!("{} {}", slot_name, field.name),
                    &field.description,
                    field.field_type.clone(),
                    field.purpose.clone(),
                ));
            }
        }

        // Also add basic fallback fields for any unconfigured slots
        let slot_names = self.get_slot_names();
        for slot_name in slot_names {
            // Only add basic fields if this slot doesn't have a child source
            if !self.child_sources.contains_key(&slot_name) {
                fields.push(FieldMetadata::new(
                    format!("{}_caption", slot_name),
                    format!("{} Caption", slot_name),
                    "Label for this data slot",
                    FieldType::Text,
                    FieldPurpose::Caption,
                ));
                fields.push(FieldMetadata::new(
                    format!("{}_value", slot_name),
                    format!("{} Value", slot_name),
                    "Main value for this data slot",
                    FieldType::Numerical,
                    FieldPurpose::Value,
                ));
                fields.push(FieldMetadata::new(
                    format!("{}_unit", slot_name),
                    format!("{} Unit", slot_name),
                    "Unit for this data slot",
                    FieldType::Text,
                    FieldPurpose::Unit,
                ));
            }
        }

        fields
    }

    fn update(&mut self) -> Result<()> {
        // Update all child sources
        for (slot_name, source) in self.child_sources.iter_mut() {
            if let Err(e) = source.update() {
                log::warn!("ComboSource: Failed to update slot '{}': {}", slot_name, e);
            }
        }

        // Aggregate values
        self.aggregate_values();

        Ok(())
    }

    fn get_values(&self) -> HashMap<String, Value> {
        self.values.clone()
    }

    fn values_ref(&self) -> Option<&HashMap<String, Value>> {
        Some(&self.values)
    }

    fn configure(&mut self, config: &HashMap<String, Value>) -> Result<()> {
        // Look for combo_config in the configuration
        if let Some(combo_config_value) = config.get("combo_config") {
            if let Ok(mut combo_config) =
                serde_json::from_value::<ComboSourceConfig>(combo_config_value.clone())
            {
                // Migrate legacy primary/secondary config to groups format
                combo_config.migrate_legacy();
                self.config = combo_config;
                // Re-setup child sources with new configuration
                self.setup_child_sources();
            }
        }

        // Also accept individual slot configurations for compatibility
        // Format: "group1_1_source" -> "cpu", "group1_1_source_config" -> {...}
        let mut modified = false;
        for (key, value) in config {
            if key.ends_with("_source") && !key.contains("_source_config") {
                let slot_name = key.trim_end_matches("_source");
                if let Some(source_id) = value.as_str() {
                    let slot_config = self.config.slots.entry(slot_name.to_string()).or_default();
                    slot_config.source_id = source_id.to_string();
                    modified = true;
                }
            }
            if key.ends_with("_caption") {
                let slot_name = key.trim_end_matches("_caption");
                if let Some(caption) = value.as_str() {
                    let slot_config = self.config.slots.entry(slot_name.to_string()).or_default();
                    slot_config.caption_override = caption.to_string();
                    modified = true;
                }
            }
        }

        if modified {
            self.setup_child_sources();
        }

        Ok(())
    }

    fn get_typed_config(&self) -> Option<crate::core::SourceConfig> {
        Some(crate::core::SourceConfig::Combo(self.config.clone()))
    }
}
