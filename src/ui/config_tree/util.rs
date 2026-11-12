//! Pure helpers for the tree dialog: config flattening, combo slot
//! summaries and field metadata. No GTK.

use crate::core::{global_registry, FieldMetadata, FieldPurpose, FieldType, SourceConfig};
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
                // Source name first: the CSS Template auto-configure reads the
                // slot's source type from the first word, and a caption alone
                // ("CPU Fan" on a fan slot) named the wrong one.
                Some(s) if !s.caption_override.is_empty() => format!(
                    "{} ({})",
                    source_display_name(&s.source_id),
                    s.caption_override
                ),
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

/// The fields a content item's display editor offers: the generic item keys
/// the renderer always provides (caption, value, unit, numeric value), then
/// the slot's source fields by their short ids. An item is rendered against
/// the slot's values with the `"{slot}_"` prefix stripped, so its field
/// pickers must write the short id (`time`), never the combo-wide
/// `group1_1_time` of [`slot_fields`] — a line bound to that renders nothing.
pub fn item_fields(source_id: &str) -> Vec<FieldMetadata> {
    let mut fields = vec![
        FieldMetadata::new(
            "caption",
            "Caption",
            "Label text",
            FieldType::Text,
            FieldPurpose::Caption,
        ),
        FieldMetadata::new(
            "value",
            "Value",
            "Current value",
            FieldType::Text,
            FieldPurpose::Value,
        ),
        FieldMetadata::new(
            "unit",
            "Unit",
            "Unit of measurement",
            FieldType::Text,
            FieldPurpose::Unit,
        ),
        FieldMetadata::new(
            "numerical_value",
            "Numeric Value",
            "Raw numeric value",
            FieldType::Numerical,
            FieldPurpose::Value,
        ),
    ];
    if !source_id.is_empty() && source_id != "none" {
        for field in global_registry().get_source_fields_cached(source_id) {
            if !fields.iter().any(|f| f.id == field.id) {
                fields.push(field);
            }
        }
    }
    fields
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

    /// The renderer's own keys come first and nothing carries a slot prefix.
    #[test]
    fn item_fields_are_short_ids() {
        let ids: Vec<String> = item_fields("none").into_iter().map(|f| f.id).collect();
        assert_eq!(ids, vec!["caption", "value", "unit", "numerical_value"]);
        for f in item_fields("clock") {
            assert!(!f.id.starts_with("group"), "{} is a combo-wide id", f.id);
        }
    }
}
