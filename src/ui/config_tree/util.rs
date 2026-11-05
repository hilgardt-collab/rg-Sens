//! Pure helpers for the tree dialog: config flattening, combo slot
//! summaries and field metadata. No GTK.

use crate::core::{global_registry, FieldMetadata, SourceConfig};
use crate::sources::ComboSourceConfig;
use serde_json::Value;
use std::collections::HashMap;

/// The flattened per-slot form of a source config (what
/// `SlotConfig::source_config` stores): the typed config's own fields,
/// without the enum tag or the `{id}_config` wrapper.
pub fn flatten_source_config(cfg: &SourceConfig) -> HashMap<String, Value> {
    cfg.to_hashmap()
        .into_values()
        .next()
        .and_then(|v| v.as_object().cloned())
        .map(|o| o.into_iter().collect())
        .unwrap_or_default()
}

/// Inverse of [`flatten_source_config`].
pub fn source_config_from_flat(
    source_id: &str,
    flat: &HashMap<String, Value>,
) -> Option<SourceConfig> {
    if flat.is_empty() {
        return SourceConfig::default_for_type(source_id);
    }
    let mut map = HashMap::new();
    map.insert(
        format!("{}_config", source_id),
        Value::Object(flat.iter().map(|(k, v)| (k.clone(), v.clone())).collect()),
    );
    SourceConfig::extract_from_hashmap(&map, source_id)
        .or_else(|| SourceConfig::default_for_type(source_id))
}

/// Human-readable name of a source type.
pub fn source_display_name(id: &str) -> String {
    global_registry()
        .get_source_info(id)
        .map(|s| s.display_name)
        .unwrap_or_else(|| id.to_string())
}

/// Human-readable name of a displayer type.
pub fn displayer_display_name(id: &str) -> String {
    global_registry()
        .get_displayer_info(id)
        .map(|d| d.display_name)
        .unwrap_or_else(|| id.to_string())
}

/// Slot summaries in the form the themed editors expect:
/// `(slot, summary, group_num 1-based, item_idx 1-based)`.
pub fn combo_summaries(cfg: &ComboSourceConfig) -> Vec<(String, String, usize, u32)> {
    let mut out = Vec::new();
    for (g, group) in cfg.groups.iter().enumerate() {
        for m in 1..=group.item_count {
            let slot = format!("group{}_{}", g + 1, m);
            let summary = match cfg.slots.get(&slot) {
                Some(s) if s.source_id.is_empty() || s.source_id == "none" => {
                    "(Not configured)".to_string()
                }
                Some(s) if !s.caption_override.is_empty() => s.caption_override.clone(),
                Some(s) => source_display_name(&s.source_id),
                None => "(Not configured)".to_string(),
            };
            out.push((slot, summary, g + 1, m));
        }
    }
    out
}

/// Prefixed field metadata for one slot.
pub fn slot_fields(slot: &str, source_id: &str) -> Vec<FieldMetadata> {
    global_registry().get_source_fields_for_combo_slot(slot, source_id)
}

/// Prefixed field metadata for every configured slot of a combo source.
pub fn combo_fields(cfg: &ComboSourceConfig) -> Vec<FieldMetadata> {
    let mut fields = Vec::new();
    for slot in rg_sens_sources::slot_names_for(cfg) {
        if let Some(s) = cfg.slots.get(&slot) {
            if !s.source_id.is_empty() && s.source_id != "none" {
                fields.extend(slot_fields(&slot, &s.source_id));
            }
        }
    }
    fields
}

#[cfg(test)]
mod tests {
    use super::*;
    use rg_sens_types::source_configs::cpu::CpuSourceConfig;

    #[test]
    fn flatten_round_trips_through_slot_form() {
        let cfg = SourceConfig::Cpu(CpuSourceConfig {
            update_interval_ms: 2500,
            ..CpuSourceConfig::default()
        });
        let flat = flatten_source_config(&cfg);
        assert!(flat.contains_key("update_interval_ms"));
        assert!(
            !flat.contains_key("source_type"),
            "enum tag must not leak into the slot map"
        );
        let back = source_config_from_flat("cpu", &flat).unwrap();
        assert_eq!(back.update_interval_ms(), 2500);
        assert_eq!(back.source_type(), "cpu");
    }

    #[test]
    fn empty_flat_map_gives_type_default() {
        let back = source_config_from_flat("memory", &HashMap::new()).unwrap();
        assert_eq!(back.source_type(), "memory");
    }
}
