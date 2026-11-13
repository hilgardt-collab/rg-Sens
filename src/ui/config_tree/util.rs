//! Pure helpers for the tree dialog: config flattening, combo slot
//! summaries and field metadata. No GTK.

use crate::core::{global_registry, FieldMetadata, FieldPurpose, FieldType, SourceConfig};
use crate::sources::{ComboSourceConfig, SlotConfig};
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

/// What each slot measures, for the CSS Template's placeholder mapping: the
/// source id and the source's configured `field` (a slot without its own
/// config measures what the source's default config does).
pub fn combo_slot_kinds(cfg: &ComboSourceConfig) -> Vec<crate::ui::css_template_display::SlotKind> {
    let field_of = |map: &std::collections::HashMap<String, serde_json::Value>| {
        map.values()
            .find_map(|v| v.get("field").and_then(|f| f.as_str()).map(str::to_string))
    };
    let mut out = Vec::new();
    for (g, group) in cfg.groups.iter().enumerate() {
        for m in 1..=group.item_count {
            let prefix = format!("group{}_{}", g + 1, m);
            let (source, metric) = match cfg.slots.get(&prefix) {
                Some(s) if !s.source_id.is_empty() && s.source_id != "none" => {
                    let metric = field_of(&s.source_config).or_else(|| {
                        rg_sens_types::panel::SourceConfig::default_for_type(&s.source_id)
                            .and_then(|d| field_of(&d.to_hashmap()))
                    });
                    (s.source_id.clone(), metric.unwrap_or_default())
                }
                _ => (String::new(), String::new()),
            };
            out.push(crate::ui::css_template_display::SlotKind {
                prefix,
                source,
                metric,
            });
        }
    }
    out
}

/// Source ids a template may name by their first word (`fan`, `system`).
const TEMPLATE_SOURCES: &[&str] = &[
    "clock",
    "cpu",
    "gpu",
    "memory",
    "disk",
    "network",
    "fan_speed",
    "system_temp",
    "uptime",
    "claude",
    "static_text",
];

/// A new slot for `(source, metric)` as a template names them, or `None` when
/// no such source exists. `nth` is how many slots of that source the panel
/// already has: fans take the next fan in turn.
fn slot_for(wanted_source: &str, metric: &str, nth: usize) -> Option<SlotConfig> {
    use crate::ui::css_template_display::source_matches;
    use rg_sens_types::panel::SourceConfig;

    let source_id = TEMPLATE_SOURCES
        .iter()
        .find(|id| **id == wanted_source)
        .or_else(|| {
            TEMPLATE_SOURCES
                .iter()
                .find(|id| source_matches(id, wanted_source))
        })?;
    let mut config = SourceConfig::default_for_type(source_id)?.to_hashmap();
    if !metric.is_empty() {
        // The metric is the source's `field` enum variant in snake_case.
        let variant: String = metric
            .split('_')
            .map(|word| {
                let mut chars = word.chars();
                chars
                    .next()
                    .map(|c| c.to_uppercase().chain(chars).collect::<String>())
                    .unwrap_or_default()
            })
            .collect();
        let mut candidate = config.clone();
        for value in candidate.values_mut() {
            if let Some(obj) = value.as_object_mut().filter(|o| o.contains_key("field")) {
                obj.insert(
                    "field".to_string(),
                    serde_json::Value::from(variant.clone()),
                );
            }
        }
        // Only a field the source really has; otherwise its default stays.
        if SourceConfig::extract_from_hashmap(&candidate, source_id).is_some() {
            config = candidate;
        }
    }
    if *source_id == "fan_speed" {
        for value in config.values_mut() {
            if let Some(obj) = value.as_object_mut() {
                obj.insert("sensor_index".to_string(), serde_json::Value::from(nth));
            }
        }
    }
    Some(SlotConfig {
        source_id: source_id.to_string(),
        caption_override: String::new(),
        source_config: config,
    })
}

/// Give the combination source the slots a CSS template asks for
/// (`wanted_slots`), keeping what it has: a slot of the right source and
/// metric counts, the rest are created. A source with nothing configured yet
/// is laid out afresh, one group per source type; otherwise a new slot joins
/// the group that already holds its source type, or a new group. Returns how
/// many slots were created.
pub fn add_wanted_slots(cfg: &mut ComboSourceConfig, wanted: &[(String, String)]) -> usize {
    use crate::ui::css_template_display::{normalized, source_matches};
    const MAX: usize = 16; // groups per source, items per group

    let configured = |s: &SlotConfig| !s.source_id.is_empty() && s.source_id != "none";
    if !cfg.slots.values().any(configured) {
        cfg.groups.clear();
        cfg.slots.clear();
    }
    let mut created = 0;
    let mut seen: Vec<(String, String)> = Vec::new();
    for want in wanted {
        if seen.contains(want) {
            continue;
        }
        seen.push(want.clone());
        let (source, metric) = want;
        let need = wanted.iter().filter(|w| *w == want).count();
        let kinds = combo_slot_kinds(cfg);
        let have = kinds
            .iter()
            .filter(|k| {
                source_matches(&k.source, source)
                    && (metric.is_empty() || normalized(&k.metric) == normalized(metric))
            })
            .count();
        for _ in have..need {
            let same_source: Vec<_> = combo_slot_kinds(cfg)
                .into_iter()
                .filter(|k| source_matches(&k.source, source))
                .collect();
            let nth = same_source.len();
            let Some(slot) = slot_for(source, metric, nth) else {
                break;
            };
            // The group that already holds this source type, if it has room.
            let home = same_source
                .iter()
                .filter_map(|k| {
                    let group: usize = k
                        .prefix
                        .strip_prefix("group")?
                        .split('_')
                        .next()?
                        .parse()
                        .ok()?;
                    Some(group - 1)
                })
                .find(|g| {
                    cfg.groups
                        .get(*g)
                        .is_some_and(|grp| (grp.item_count as usize) < MAX)
                });
            let group = match home {
                Some(g) => {
                    cfg.groups[g].item_count += 1;
                    g
                }
                None if cfg.groups.len() < MAX => {
                    cfg.groups.push(crate::sources::GroupConfig {
                        item_count: 1,
                        size_weight: 1.0,
                    });
                    cfg.groups.len() - 1
                }
                None => break,
            };
            let prefix = format!("group{}_{}", group + 1, cfg.groups[group].item_count);
            cfg.slots.insert(prefix, slot);
            created += 1;
        }
    }
    if cfg.groups.is_empty() {
        cfg.groups.push(crate::sources::GroupConfig {
            item_count: 1,
            size_weight: 1.0,
        });
    }
    created
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

#[cfg(test)]
mod css_template_setup_tests {
    use super::*;
    use crate::ui::css_template_display::{
        auto_map, detect_placeholders, extract_placeholder_defaults, wanted_slots,
    };

    const PANELS: [&str; 3] = [
        include_str!("../../../examples/art_deco_panel.html"),
        include_str!("../../../examples/art_nouveau_panel.html"),
        include_str!("../../../examples/steampunk_panel.html"),
    ];

    fn kinds_of(cfg: &ComboSourceConfig) -> Vec<(String, String)> {
        let mut kinds: Vec<(String, String)> = combo_slot_kinds(cfg)
            .into_iter()
            .map(|k| (k.source, k.metric))
            .collect();
        kinds.sort();
        kinds
    }

    /// Scan Placeholders on a fresh panel: the template's slots are created
    /// with the right source and metric, and every placeholder that names a
    /// source is mapped to one of them.
    #[test]
    fn scanning_an_example_panel_sets_the_whole_panel_up() {
        for html in PANELS {
            let placeholders = detect_placeholders(html);
            let defaults = extract_placeholder_defaults(html);
            let wanted = wanted_slots(&placeholders, &defaults);
            assert_eq!(wanted.len(), 18, "{wanted:?}");

            let mut cfg = ComboSourceConfig::default();
            assert_eq!(add_wanted_slots(&mut cfg, &wanted), 18);
            let s = |a: &str, b: &str| (a.to_string(), b.to_string());
            let mut expected = vec![
                s("clock", ""),
                s("cpu", "Temperature"),
                s("cpu", "Usage"),
                s("cpu", "Frequency"),
                s("gpu", "Temperature"),
                s("gpu", "Utilization"),
                s("gpu", "ClockCore"),
                s("gpu", "PowerUsage"),
                s("gpu", "MemoryUsed"),
                s("fan_speed", ""),
                s("fan_speed", ""),
                s("fan_speed", ""),
                s("fan_speed", ""),
                s("memory", "Used"),
                s("disk", "Used"),
                s("network", "DownloadSpeed"),
                s("network", "UploadSpeed"),
                s("uptime", ""),
            ];
            expected.sort();
            assert_eq!(kinds_of(&cfg), expected);
            // one group per source type, each fan its own sensor
            assert_eq!(cfg.groups.len(), 8);
            let mut fans: Vec<u64> = cfg
                .slots
                .values()
                .filter(|slot| slot.source_id == "fan_speed")
                .map(|slot| {
                    slot.source_config["fan_speed_config"]["sensor_index"]
                        .as_u64()
                        .unwrap()
                })
                .collect();
            fans.sort();
            assert_eq!(fans, [0, 1, 2, 3]);

            let mappings = auto_map(
                &placeholders,
                &defaults,
                &HashMap::new(),
                &combo_slot_kinds(&cfg),
            );
            assert_eq!(mappings.len(), 74);
            let unmapped: Vec<u32> = mappings
                .iter()
                .filter(|m| m.slot_prefix.is_empty())
                .map(|m| m.index)
                .collect();
            assert_eq!(unmapped, [16, 17, 18, 19], "only CPU power has no source");
            // the gauge reads the CPU temperature slot, whatever its position
            let slot_of = |index: u32| {
                let prefix = &mappings
                    .iter()
                    .find(|m| m.index == index)
                    .unwrap()
                    .slot_prefix;
                combo_slot_kinds(&cfg)
                    .into_iter()
                    .find(|k| &k.prefix == prefix)
                    .map(|k| (k.source, k.metric))
                    .unwrap()
            };
            assert_eq!(slot_of(5), s("cpu", "Temperature"));
            assert_eq!(slot_of(57), s("gpu", "MemoryUsed"));
            assert_eq!(slot_of(69), s("network", "UploadSpeed"));
            // time, day, month and hour all read the one clock slot
            assert_eq!(slot_of(0), slot_of(72));

            // scanning again changes nothing
            assert_eq!(add_wanted_slots(&mut cfg, &wanted), 0);
        }
    }

    /// Slots the user already made are used wherever they sit, and only the
    /// missing ones are added.
    #[test]
    fn existing_slots_are_kept_and_matched_by_metric() {
        let html = PANELS[0];
        let (placeholders, defaults) = (
            detect_placeholders(html),
            extract_placeholder_defaults(html),
        );
        let wanted = wanted_slots(&placeholders, &defaults);

        // usage before temperature, with a caption, in one group
        let mut cfg = ComboSourceConfig::default();
        add_wanted_slots(
            &mut cfg,
            &[
                ("cpu".into(), "usage".into()),
                ("cpu".into(), "temperature".into()),
            ],
        );
        for slot in cfg.slots.values_mut() {
            slot.caption_override = "My CPU".to_string();
        }
        let before = cfg.slots.clone();
        assert_eq!(add_wanted_slots(&mut cfg, &wanted), 16);
        for (prefix, slot) in &before {
            assert_eq!(cfg.slots[prefix].source_config, slot.source_config);
            assert_eq!(cfg.slots[prefix].caption_override, "My CPU");
        }
        let kinds = combo_slot_kinds(&cfg);
        let mappings = auto_map(&placeholders, &defaults, &HashMap::new(), &kinds);
        let prefix = &mappings.iter().find(|m| m.index == 5).unwrap().slot_prefix;
        let slot = kinds.iter().find(|k| &k.prefix == prefix).unwrap();
        assert_eq!(slot.metric, "Temperature");
        assert!(
            before.contains_key(prefix),
            "the gauge uses the user's temperature slot"
        );
    }
}
