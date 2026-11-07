//! Combo data source that aggregates multiple data sources
//!
//! This source is designed for complex displayers (like LCARS) that need
//! to show data from multiple sources simultaneously.

use anyhow::Result;
use rg_sens_core::{
    global_registry, DataSource, FieldMetadata, FieldPurpose, FieldType, SourceMetadata,
};
use serde_json::Value;
use std::collections::HashMap;
use std::time::{Duration, Instant};

// Re-export combo source config types from rg-sens-types
pub use rg_sens_types::source_configs::combo::{ComboSourceConfig, GroupConfig, SlotConfig};

/// Slack on a slot's own interval: the scheduler polls the combo a few
/// milliseconds either side of the nominal time, and without it a slot whose
/// interval is a multiple of the poll would skip every other poll.
const CHILD_INTERVAL_TOLERANCE: Duration = Duration::from_millis(10);

/// Combo data source that aggregates multiple child sources
///
/// This source manages multiple child data sources and aggregates their
/// values into a single HashMap with prefixed keys.
pub struct ComboSource {
    metadata: SourceMetadata,
    config: ComboSourceConfig,
    /// Child source instances, keyed by slot name
    child_sources: HashMap<String, Box<dyn DataSource>>,
    /// The (source_id, source_config) each child was last configured with,
    /// so a reconcile can tell "unchanged" from "needs configure()".
    child_configs: HashMap<String, (String, HashMap<String, Value>)>,
    /// When each child last refreshed. A slot with its own `update_interval_ms`
    /// is refreshed once that much time has passed; the combo itself is polled
    /// at the fastest interval any slot asks for (`poll_interval_ms`).
    child_last_update: HashMap<String, Instant>,
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
            child_configs: HashMap::new(),
            child_last_update: HashMap::new(),
            values: HashMap::new(),
        }
    }

    /// Rebuild every child source from scratch.
    ///
    /// Prefer [`ComboSource::apply_config`], which reconciles incrementally;
    /// this is the full rebuild for callers that have no previous state.
    pub fn setup_child_sources(&mut self) {
        self.child_sources.clear();
        let stats = self.reconcile_children();
        log::info!(
            "ComboSource: Set up {} child sources ({:?})",
            self.child_sources.len(),
            stats
        );
    }

    /// Replace the configuration and reconcile the child sources with the
    /// minimum work (see [`ComboSource::reconcile_children`]).
    pub fn apply_config(&mut self, mut new_config: ComboSourceConfig) -> ReconcileStats {
        new_config.migrate_legacy();
        self.config = new_config;
        let stats = self.reconcile_children();
        log::info!(
            "ComboSource: reconciled {} child sources ({:?})",
            self.child_sources.len(),
            stats
        );
        stats
    }

    /// Bring `child_sources` in line with `self.config` without recreating
    /// children that did not change:
    ///
    /// | old → new                                   | action              |
    /// |---------------------------------------------|---------------------|
    /// | same `source_id`, same `source_config`      | keep child          |
    /// | same `source_id`, different `source_config` | `child.configure()` |
    /// | different `source_id` (including `none`)    | drop, create new    |
    /// | slot removed or group shrunk                | drop                |
    /// | slot added                                  | create              |
    ///
    /// The source config a child was last given is remembered in
    /// `child_configs`, so "different" is a value comparison, not a guess.
    pub fn reconcile_children(&mut self) -> ReconcileStats {
        let mut stats = ReconcileStats::default();
        let slot_names = slot_names_for(&self.config);

        // Drop children whose slot vanished, is now unconfigured, or changed type.
        let mut to_drop = Vec::new();
        for slot_name in self.child_sources.keys() {
            let wanted = slot_names.contains(slot_name);
            let keep_type = self
                .config
                .slots
                .get(slot_name)
                .filter(|slot| is_configured(slot))
                .map(|slot| {
                    self.child_configs
                        .get(slot_name)
                        .map(|(id, _)| id == &slot.source_id)
                        .unwrap_or(false)
                })
                .unwrap_or(false);
            if !wanted || !keep_type {
                to_drop.push(slot_name.clone());
            }
        }
        for slot_name in to_drop {
            self.child_sources.remove(&slot_name);
            self.child_configs.remove(&slot_name);
            stats.dropped += 1;
        }

        // Keep, reconfigure, or create the rest.
        for slot_name in slot_names {
            let Some(slot_config) = self.config.slots.get(&slot_name).cloned() else {
                continue;
            };
            if !is_configured(&slot_config) {
                continue;
            }
            if let Some(child) = self.child_sources.get_mut(&slot_name) {
                let unchanged = self
                    .child_configs
                    .get(&slot_name)
                    .map(|(_, cfg)| *cfg == slot_config.source_config)
                    .unwrap_or(false);
                if unchanged {
                    stats.kept += 1;
                    continue;
                }
                if let Err(e) = configure_child(child.as_mut(), &slot_name, &slot_config) {
                    log::warn!(
                        "ComboSource: failed to reconfigure slot '{}': {}",
                        slot_name,
                        e
                    );
                }
                self.child_configs.insert(
                    slot_name.clone(),
                    (
                        slot_config.source_id.clone(),
                        slot_config.source_config.clone(),
                    ),
                );
                stats.reconfigured += 1;
                continue;
            }
            if let Some(child) = create_child(&slot_name, &slot_config) {
                self.child_sources.insert(slot_name.clone(), child);
                self.child_configs.insert(
                    slot_name,
                    (
                        slot_config.source_id.clone(),
                        slot_config.source_config.clone(),
                    ),
                );
                stats.created += 1;
            }
        }
        stats
    }

    /// Get the list of slot names based on configuration
    fn get_slot_names(&self) -> Vec<String> {
        slot_names_for(&self.config)
    }
}

/// Counts of what [`ComboSource::reconcile_children`] did; mostly for tests
/// and logs.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ReconcileStats {
    pub kept: usize,
    pub reconfigured: usize,
    pub created: usize,
    pub dropped: usize,
}

fn is_configured(slot: &SlotConfig) -> bool {
    !slot.source_id.is_empty() && slot.source_id != "none"
}

/// Wrap a slot's flattened source config under the `{source_id}_config` key
/// the child source expects and apply it.
fn configure_child(
    child: &mut dyn DataSource,
    slot_name: &str,
    slot_config: &SlotConfig,
) -> Result<()> {
    if slot_config.source_config.is_empty() {
        return Ok(());
    }
    let config_key = format!("{}_config", slot_config.source_id);
    let config_obj = Value::Object(
        slot_config
            .source_config
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
    );
    log::debug!(
        "ComboSource: configuring '{}' for slot '{}'",
        slot_config.source_id,
        slot_name
    );
    let mut config_map = HashMap::new();
    config_map.insert(config_key, config_obj);
    child.configure(&config_map)
}

/// Create and configure the child source for one slot.
fn create_child(slot_name: &str, slot_config: &SlotConfig) -> Option<Box<dyn DataSource>> {
    match global_registry().create_source(&slot_config.source_id) {
        Ok(mut source) => {
            if let Err(e) = configure_child(source.as_mut(), slot_name, slot_config) {
                log::warn!(
                    "ComboSource: Failed to configure '{}' for slot '{}': {}",
                    slot_config.source_id,
                    slot_name,
                    e
                );
            }
            log::debug!(
                "ComboSource: Created child source '{}' for slot '{}'",
                slot_config.source_id,
                slot_name
            );
            Some(source)
        }
        Err(e) => {
            log::warn!(
                "ComboSource: Failed to create source '{}' for slot '{}': {}",
                slot_config.source_id,
                slot_name,
                e
            );
            None
        }
    }
}

/// Slot names a configuration defines, in group/item order.
pub fn slot_names_for(config: &ComboSourceConfig) -> Vec<String> {
    let mut names = Vec::new();

    match config.mode.as_str() {
        "lcars" => {
            // Groups with items: group1_1, group1_2, group2_1, etc.
            for (group_idx, group) in config.groups.iter().enumerate() {
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
            let arc_count = config.groups.first().map(|g| g.item_count).unwrap_or(4);
            for i in 1..=arc_count {
                names.push(format!("arc{}", i));
            }
        }
        "level_bar" => {
            // Bar sources from first group's item count
            let bar_count = config.groups.first().map(|g| g.item_count).unwrap_or(4);
            for i in 1..=bar_count {
                names.push(format!("bar{}", i));
            }
        }
        _ => {
            // Default to lcars pattern
            for (group_idx, group) in config.groups.iter().enumerate() {
                let group_num = group_idx + 1;
                for item_idx in 1..=group.item_count {
                    names.push(format!("group{}_{}", group_num, item_idx));
                }
            }
        }
    }

    names
}

impl ComboSource {
    /// Aggregate values from all child sources
    fn aggregate_values(&mut self) {
        self.values.clear();

        for (slot_name, source) in &self.child_sources {
            // Borrow the child's values in place when possible; get_values()
            // clones the whole map (keys included) only to discard the keys.
            let owned;
            let source_values: &HashMap<String, Value> = match source.values_ref() {
                Some(values) => values,
                None => {
                    owned = source.get_values();
                    &owned
                }
            };
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
                self.values.insert(prefixed_key, value.clone());
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
        // Refresh the children that are due: a slot with its own interval
        // waits it out, every other slot follows each poll
        let now = Instant::now();
        for (slot_name, source) in self.child_sources.iter_mut() {
            let own_interval = self
                .config
                .slots
                .get(slot_name)
                .and_then(SlotConfig::update_interval_ms);
            let due = match (own_interval, self.child_last_update.get(slot_name)) {
                (Some(ms), Some(last)) => {
                    now.duration_since(*last) + CHILD_INTERVAL_TOLERANCE
                        >= Duration::from_millis(ms)
                }
                _ => true,
            };
            if !due {
                continue;
            }
            if let Err(e) = source.update() {
                log::warn!("ComboSource: Failed to update slot '{}': {}", slot_name, e);
            }
            self.child_last_update.insert(slot_name.clone(), now);
        }
        self.child_last_update
            .retain(|slot_name, _| self.child_sources.contains_key(slot_name));

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
            if let Ok(combo_config) =
                serde_json::from_value::<ComboSourceConfig>(combo_config_value.clone())
            {
                // Reconcile children incrementally: unchanged slots keep their
                // (possibly expensive) child source instance.
                self.apply_config(combo_config);
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
            self.reconcile_children();
        }

        Ok(())
    }

    fn get_typed_config(&self) -> Option<rg_sens_core::SourceConfig> {
        Some(rg_sens_core::SourceConfig::Combo(self.config.clone()))
    }
}

#[cfg(test)]
mod reconcile_tests {
    use super::*;
    use std::sync::Once;

    static REGISTER: Once = Once::new();

    fn registry_ready() {
        REGISTER.call_once(|| crate::register_all());
    }

    fn slot(source_id: &str, caption: &str) -> SlotConfig {
        let mut source_config = HashMap::new();
        source_config.insert("custom_caption".to_string(), Value::from(caption));
        SlotConfig {
            source_id: source_id.to_string(),
            caption_override: String::new(),
            source_config,
        }
    }

    fn config(items: &[(&str, &str, &str)]) -> ComboSourceConfig {
        let mut cfg = ComboSourceConfig {
            groups: vec![GroupConfig {
                item_count: items.len() as u32,
                size_weight: 1.0,
            }],
            ..ComboSourceConfig::default()
        };
        for (i, (name, source_id, caption)) in items.iter().enumerate() {
            assert_eq!(*name, format!("group1_{}", i + 1));
            cfg.slots.insert(name.to_string(), slot(source_id, caption));
        }
        cfg
    }

    #[test]
    fn a_slot_with_its_own_interval_waits_it_out() {
        registry_ready();
        let mut cfg = config(&[
            ("group1_1", "static_text", "slow"),
            ("group1_2", "static_text", "every poll"),
        ]);
        cfg.slots
            .get_mut("group1_1")
            .unwrap()
            .source_config
            .insert("update_interval_ms".to_string(), Value::from(60_000u64));
        assert_eq!(cfg.poll_interval_ms(), cfg.update_interval_ms());
        let mut src = ComboSource::new();
        src.apply_config(cfg);
        src.update().unwrap();
        let slow = src.child_last_update["group1_1"];
        let fast = src.child_last_update["group1_2"];
        std::thread::sleep(Duration::from_millis(15));
        src.update().unwrap();
        assert_eq!(src.child_last_update["group1_1"], slow);
        assert!(src.child_last_update["group1_2"] > fast);
    }

    fn child_ptr(src: &ComboSource, slot: &str) -> *const () {
        src.child_sources
            .get(slot)
            .map(|b| b.as_ref() as *const dyn DataSource as *const ())
            .expect("child exists")
    }

    #[test]
    fn unchanged_slots_keep_their_child_instance() {
        registry_ready();
        let mut src = ComboSource::new();
        let stats = src.apply_config(config(&[
            ("group1_1", "static_text", "a"),
            ("group1_2", "static_text", "b"),
        ]));
        assert_eq!(
            stats,
            ReconcileStats {
                created: 2,
                ..Default::default()
            }
        );
        let p1 = child_ptr(&src, "group1_1");
        let p2 = child_ptr(&src, "group1_2");

        // Same config again: nothing recreated
        let stats = src.apply_config(config(&[
            ("group1_1", "static_text", "a"),
            ("group1_2", "static_text", "b"),
        ]));
        assert_eq!(
            stats,
            ReconcileStats {
                kept: 2,
                ..Default::default()
            }
        );
        assert_eq!(child_ptr(&src, "group1_1"), p1);
        assert_eq!(child_ptr(&src, "group1_2"), p2);
    }

    #[test]
    fn changed_source_config_reconfigures_in_place() {
        registry_ready();
        let mut src = ComboSource::new();
        src.apply_config(config(&[("group1_1", "static_text", "a")]));
        let p1 = child_ptr(&src, "group1_1");
        let stats = src.apply_config(config(&[("group1_1", "static_text", "changed")]));
        assert_eq!(
            stats,
            ReconcileStats {
                reconfigured: 1,
                ..Default::default()
            }
        );
        assert_eq!(
            child_ptr(&src, "group1_1"),
            p1,
            "same instance, reconfigured"
        );
    }

    #[test]
    fn changed_source_type_recreates_only_that_slot() {
        registry_ready();
        let mut src = ComboSource::new();
        src.apply_config(config(&[
            ("group1_1", "static_text", "a"),
            ("group1_2", "static_text", "b"),
        ]));
        let p2 = child_ptr(&src, "group1_2");
        let stats = src.apply_config(config(&[
            ("group1_1", "test", "a"),
            ("group1_2", "static_text", "b"),
        ]));
        assert_eq!(
            stats,
            ReconcileStats {
                kept: 1,
                dropped: 1,
                created: 1,
                ..Default::default()
            }
        );
        assert_eq!(child_ptr(&src, "group1_2"), p2);
        assert_eq!(
            src.child_sources.get("group1_1").unwrap().metadata().id,
            "test"
        );
    }

    #[test]
    fn removed_and_none_slots_are_dropped_added_slots_created() {
        registry_ready();
        let mut src = ComboSource::new();
        src.apply_config(config(&[
            ("group1_1", "static_text", "a"),
            ("group1_2", "static_text", "b"),
            ("group1_3", "static_text", "c"),
        ]));
        // Shrink to two items, set the first to none, keep the second
        let mut next = config(&[("group1_1", "none", ""), ("group1_2", "static_text", "b")]);
        next.groups[0].item_count = 2;
        let stats = src.apply_config(next);
        assert_eq!(
            stats,
            ReconcileStats {
                kept: 1,
                dropped: 2,
                ..Default::default()
            }
        );
        assert_eq!(src.child_sources.len(), 1);

        // Grow again
        let stats = src.apply_config(config(&[
            ("group1_1", "static_text", "x"),
            ("group1_2", "static_text", "b"),
            ("group1_3", "static_text", "c"),
        ]));
        assert_eq!(
            stats,
            ReconcileStats {
                kept: 1,
                created: 2,
                ..Default::default()
            }
        );
    }

    #[test]
    fn configure_via_hashmap_is_incremental_too() {
        registry_ready();
        let mut src = ComboSource::new();
        src.apply_config(config(&[("group1_1", "static_text", "a")]));
        let p1 = child_ptr(&src, "group1_1");
        let mut map = HashMap::new();
        map.insert(
            "combo_config".to_string(),
            serde_json::to_value(config(&[("group1_1", "static_text", "a")])).unwrap(),
        );
        src.configure(&map).unwrap();
        assert_eq!(child_ptr(&src, "group1_1"), p1);
    }
}
