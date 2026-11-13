//! CSS Template Display - Template parser and utilities for CSS-templated combo panels
//!
//! This module provides:
//! - Placeholder detection (`{{0}}`, `{{1}}`, etc.)
//! - Template transformation for JavaScript injection
//! - JavaScript bridge generation for value updates

use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashSet;

// Cached compiled regexes to avoid recompilation on every call
static PLACEHOLDER_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\{\{(\d+)\}\}").expect("Invalid placeholder regex"));

static PLACEHOLDER_HINTS_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"<script\s+type\s*=\s*["']application/json["']\s+id\s*=\s*["']rg-placeholder-hints["']\s*>([\s\S]*?)</script>"#,
    )
    .expect("Invalid placeholder hints regex")
});

static PLACEHOLDER_CONFIG_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"<script\s+type\s*=\s*["']application/json["']\s+id\s*=\s*["']rg-placeholder-config["']\s*>([\s\S]*?)</script>"#,
    )
    .expect("Invalid placeholder config regex")
});

static TEMPLATE_OPTIONS_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"<script\s+type\s*=\s*["']application/json["']\s+id\s*=\s*["']rg-template-options["']\s*>([\s\S]*?)</script>"#,
    )
    .expect("Invalid template options regex")
});
static CSS_VAR_NAME: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^--[A-Za-z0-9-]+$").expect("Invalid css var regex"));
/// What an option may write as a CSS value: words, numbers, colours and
/// simple functions. Nothing that can close the declaration or the block.
static CSS_SAFE_VALUE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[A-Za-z0-9 #.,%()+-]*$").expect("Invalid css value regex"));
static CSS_HEX_COLOR: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^#(?:[0-9A-Fa-f]{3,4}|[0-9A-Fa-f]{6}|[0-9A-Fa-f]{8})$")
        .expect("Invalid colour regex")
});

// Re-export CSS template config types from rg-sens-types
pub use rg_sens_types::display_configs::css_template::{
    CssTemplateDisplayConfig, PlaceholderDefault, PlaceholderMapping,
};

/// Detect placeholder indices in an HTML template
///
/// Scans the HTML for `{{0}}`, `{{1}}`, etc. patterns and returns
/// a sorted list of unique indices found.
pub fn detect_placeholders(html: &str) -> Vec<u32> {
    let mut indices: HashSet<u32> = HashSet::new();

    for cap in PLACEHOLDER_REGEX.captures_iter(html) {
        if let Some(m) = cap.get(1) {
            if let Ok(idx) = m.as_str().parse::<u32>() {
                indices.insert(idx);
            }
        }
    }

    let mut result: Vec<u32> = indices.into_iter().collect();
    result.sort();
    result
}

/// Extract placeholder hints from an HTML template
///
/// Looks for a JSON block in the format:
/// ```html
/// <script type="application/json" id="rg-placeholder-hints">
/// {
///   "0": "CPU Usage - Caption",
///   "1": "CPU Usage - Value",
///   "2": "CPU Usage - Unit"
/// }
/// </script>
/// ```
///
/// Returns a HashMap of placeholder index to hint string.
pub fn extract_placeholder_hints(html: &str) -> std::collections::HashMap<u32, String> {
    use std::collections::HashMap;

    let mut hints: HashMap<u32, String> = HashMap::new();

    // First, try to extract from rg-placeholder-config (richer format)
    let defaults = extract_placeholder_defaults(html);
    for (idx, default) in &defaults {
        if !default.hint.is_empty() {
            hints.insert(*idx, default.hint.clone());
        }
    }

    // If we found hints from config, return them
    if !hints.is_empty() {
        return hints;
    }

    // Fall back to legacy rg-placeholder-hints format
    if let Some(caps) = PLACEHOLDER_HINTS_REGEX.captures(html) {
        if let Some(json_match) = caps.get(1) {
            let json_str = json_match.as_str().trim();
            // Parse the JSON
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_str) {
                if let Some(obj) = parsed.as_object() {
                    for (key, value) in obj {
                        if let Ok(idx) = key.parse::<u32>() {
                            if let Some(hint) = value.as_str() {
                                hints.insert(idx, hint.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    hints
}

/// Extract placeholder default configurations from an HTML template
///
/// Looks for a JSON block in the format:
/// ```html
/// <script type="application/json" id="rg-placeholder-config">
/// {
///   "0": {
///     "hint": "Current time display",
///     "source": "clock",
///     "field": "time"
///   },
///   "1": {
///     "hint": "CPU Temperature",
///     "source": "cpu",
///     "instance": 0,
///     "field": "value",
///     "format": "{:.0}"
///   }
/// }
/// </script>
/// ```
///
/// Returns a HashMap of placeholder index to PlaceholderDefault.
pub fn extract_placeholder_defaults(
    html: &str,
) -> std::collections::HashMap<u32, PlaceholderDefault> {
    use std::collections::HashMap;

    let mut defaults: HashMap<u32, PlaceholderDefault> = HashMap::new();

    // Look for the JSON config block
    if let Some(caps) = PLACEHOLDER_CONFIG_REGEX.captures(html) {
        if let Some(json_match) = caps.get(1) {
            let json_str = json_match.as_str().trim();
            // Parse the JSON
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_str) {
                if let Some(obj) = parsed.as_object() {
                    for (key, value) in obj {
                        if let Ok(idx) = key.parse::<u32>() {
                            // Try to parse as PlaceholderDefault
                            if let Ok(default) =
                                serde_json::from_value::<PlaceholderDefault>(value.clone())
                            {
                                defaults.insert(idx, default);
                            } else if let Some(hint) = value.as_str() {
                                // Fall back to simple string (just a hint)
                                defaults.insert(
                                    idx,
                                    PlaceholderDefault {
                                        hint: hint.to_string(),
                                        ..Default::default()
                                    },
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    defaults
}

/// One choice of a [`OptionKind::Choice`] option.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct OptionChoice {
    pub value: String,
    #[serde(default)]
    pub label: String,
    /// Custom properties this choice sets (a palette sets several). Empty
    /// when the option has a `var` of its own, which then gets `value`.
    #[serde(default)]
    pub set: std::collections::BTreeMap<String, String>,
}

/// What kind of control an option is.
#[derive(Debug, Clone, PartialEq)]
pub enum OptionKind {
    Choice(Vec<OptionChoice>),
    Number {
        min: f64,
        max: f64,
        step: f64,
    },
    /// `#RRGGBB`
    Color,
    /// Written as `1` or `0`.
    Toggle,
}

/// A setting a template offers, shown on the dialog's Display tab and
/// applied as CSS custom properties on `:root`.
///
/// Declared in the template:
/// ```html
/// <script type="application/json" id="rg-template-options">
/// [
///   {"id": "fps", "label": "Frames per second", "type": "number",
///    "var": "--motion-fps", "min": 0, "max": 30, "step": 1, "default": 6},
///   {"id": "palette", "label": "Palette", "type": "choice", "default": "cyan",
///    "choices": [{"value": "cyan", "label": "Cyan + Magenta",
///                 "set": {"--a": "#22E4FF", "--b": "#FF2BD6"}}]},
///   {"id": "glow", "label": "Glow", "type": "color", "var": "--glow", "default": "#FF7A1A"},
///   {"id": "alert", "label": "Alert when hot", "type": "toggle", "var": "--auto-alert", "default": true}
/// ]
/// </script>
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct TemplateOption {
    pub id: String,
    pub label: String,
    /// Shown under the label.
    pub help: String,
    pub kind: OptionKind,
    /// The custom property the value is written to (`--motion-fps`).
    pub var: Option<String>,
    pub default: String,
}

impl TemplateOption {
    /// `value` as this option accepts it: a declared choice, a number inside
    /// the range, a hex colour, `1`/`0`. `None` for anything else.
    pub fn normalize(&self, value: &str) -> Option<String> {
        let value = value.trim();
        match &self.kind {
            OptionKind::Choice(choices) => choices
                .iter()
                .find(|c| c.value == value)
                .map(|c| c.value.clone()),
            OptionKind::Number { min, max, .. } => value
                .parse::<f64>()
                .ok()
                .filter(|n| n.is_finite())
                .map(|n| trim_number(n.clamp(*min, *max))),
            OptionKind::Color => CSS_HEX_COLOR
                .is_match(value)
                .then(|| value.to_ascii_uppercase()),
            OptionKind::Toggle => match value {
                "1" | "true" => Some("1".to_string()),
                "0" | "false" => Some("0".to_string()),
                _ => None,
            },
        }
    }

    /// The declarations this option writes for `value` (already normalised).
    fn declarations(&self, value: &str) -> Vec<(String, String)> {
        let mut out = Vec::new();
        if let Some(var) = &self.var {
            out.push((var.clone(), value.to_string()));
        }
        if let OptionKind::Choice(choices) = &self.kind {
            if let Some(choice) = choices.iter().find(|c| c.value == value) {
                out.extend(choice.set.iter().map(|(k, v)| (k.clone(), v.clone())));
            }
        }
        out
    }
}

fn trim_number(n: f64) -> String {
    let s = format!("{n:.3}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// The options a template declares (see [`TemplateOption`]). Entries that are
/// malformed, name an unusable custom property or repeat an id are dropped.
pub fn extract_template_options(html: &str) -> Vec<TemplateOption> {
    let Some(json) = TEMPLATE_OPTIONS_REGEX
        .captures(html)
        .and_then(|caps| caps.get(1))
    else {
        return Vec::new();
    };
    let Ok(serde_json::Value::Array(entries)) =
        serde_json::from_str::<serde_json::Value>(json.as_str().trim())
    else {
        log::warn!("CSS template: rg-template-options is not a JSON array");
        return Vec::new();
    };
    let text = |v: &serde_json::Value, key: &str| -> String {
        match v.get(key) {
            Some(serde_json::Value::String(s)) => s.clone(),
            Some(serde_json::Value::Bool(b)) => if *b { "1" } else { "0" }.to_string(),
            Some(serde_json::Value::Number(n)) => n.as_f64().map(trim_number).unwrap_or_default(),
            _ => String::new(),
        }
    };
    let number = |v: &serde_json::Value, key: &str, fallback: f64| {
        v.get(key).and_then(|n| n.as_f64()).unwrap_or(fallback)
    };
    let mut out: Vec<TemplateOption> = Vec::new();
    for entry in &entries {
        let id = text(entry, "id");
        if id.is_empty() || out.iter().any(|o| o.id == id) {
            continue;
        }
        let var = Some(text(entry, "var")).filter(|v| !v.is_empty());
        if var.as_deref().is_some_and(|v| !CSS_VAR_NAME.is_match(v)) {
            continue;
        }
        let kind = match text(entry, "type").as_str() {
            "choice" => {
                let choices: Vec<OptionChoice> = entry
                    .get("choices")
                    .cloned()
                    .and_then(|c| serde_json::from_value(c).ok())
                    .unwrap_or_default();
                let usable = !choices.is_empty()
                    && choices.iter().all(|c| {
                        CSS_SAFE_VALUE.is_match(&c.value)
                            && c.set.iter().all(|(k, v)| {
                                CSS_VAR_NAME.is_match(k) && CSS_SAFE_VALUE.is_match(v)
                            })
                    });
                if !usable {
                    continue;
                }
                OptionKind::Choice(choices)
            }
            "number" => {
                let (min, max) = (number(entry, "min", 0.0), number(entry, "max", 100.0));
                if min.partial_cmp(&max) != Some(std::cmp::Ordering::Less) {
                    continue;
                }
                OptionKind::Number {
                    min,
                    max,
                    step: number(entry, "step", 1.0).max(0.001),
                }
            }
            "color" => OptionKind::Color,
            "toggle" => OptionKind::Toggle,
            _ => continue,
        };
        let label = Some(text(entry, "label"))
            .filter(|l| !l.is_empty())
            .unwrap_or_else(|| id.clone());
        let mut option = TemplateOption {
            id,
            label,
            help: text(entry, "help"),
            kind,
            var,
            default: String::new(),
        };
        // A default the option itself rejects would make the dialog lie.
        match option.normalize(&text(entry, "default")) {
            Some(default) => option.default = default,
            None => continue,
        }
        out.push(option);
    }
    out
}

/// The stylesheet that applies the chosen option values: one `:root` block
/// holding only declared custom properties with validated values. Options
/// left at their default write nothing (the template's own stylesheet has
/// them). Appended after the template's stylesheet, so it wins.
pub fn options_css(
    options: &[TemplateOption],
    values: &std::collections::BTreeMap<String, String>,
) -> String {
    let mut body = String::new();
    for option in options {
        let Some(value) = values.get(&option.id).and_then(|v| option.normalize(v)) else {
            continue;
        };
        if value == option.default {
            continue;
        }
        for (name, value) in option.declarations(&value) {
            body.push_str(&format!("{name}: {value};"));
        }
    }
    if body.is_empty() {
        String::new()
    } else {
        format!(":root{{{body}}}")
    }
}

/// The document a backend loads for `html`: placeholders turned into spans,
/// the template's option values, the user and embedded stylesheets and the
/// bridge script. Every loader goes through here.
pub fn build_document(
    html: &str,
    config: &CssTemplateDisplayConfig,
    user_css: Option<&str>,
) -> String {
    let options = options_css(&extract_template_options(html), &config.options);
    let embedded = match (config.embedded_css.as_deref(), options.is_empty()) {
        (Some(css), false) => Some(format!("{css}\n{options}")),
        (Some(css), true) => Some(css.to_string()),
        (None, false) => Some(options),
        (None, true) => None,
    };
    prepare_html_document(&transform_template(html), user_css, embedded.as_deref())
}

/// One slot of the panel's combination source, as auto-mapping sees it.
#[derive(Debug, Clone, PartialEq)]
pub struct SlotKind {
    /// `group1_2`
    pub prefix: String,
    /// The slot's source id (`cpu`, `fan_speed`); empty when not configured.
    pub source: String,
    /// The source's configured field (`Temperature`, `MemoryUsed`), if it has one.
    pub metric: String,
    /// What the sensor picker calls the slot (`GPU · Memory used`).
    pub label: String,
}

/// One thing a template shows, bound to one sensor: the placeholders that
/// read the same slot (a gauge's caption / value / unit / max, a clock's
/// time and date parts). The Bindings page has one row per binding.
#[derive(Debug, Clone, PartialEq)]
pub struct Binding {
    /// From the template's hints (`CPU temperature (gauge)`).
    pub title: String,
    /// The source the template asks for; empty when it names none.
    pub source: String,
    /// The metric it asks for, as [`PlaceholderDefault::metric`].
    pub metric: String,
    /// The placeholders, ascending.
    pub indices: Vec<u32>,
}

/// A hint is `"{what} - {part}"` (`CPU usage - Value`); a hint without the
/// separator is all `what`.
pub fn split_hint(hint: &str) -> (&str, &str) {
    match hint.rsplit_once(" - ") {
        Some((what, part)) => (what.trim(), part.trim()),
        None => (hint.trim(), ""),
    }
}

/// Group a template's placeholders into [`Binding`]s, in template order.
///
/// Placeholders share a binding when auto-mapping would give them one slot:
/// the caption / value / unit / max run of one source, and every placeholder
/// that names the same source, metric and (non-zero) instance. Anything else
/// is a binding of its own, so a template without a config block gets one
/// row per placeholder.
pub fn bindings(
    placeholders: &[u32],
    defaults: &std::collections::HashMap<u32, PlaceholderDefault>,
    hints: &std::collections::HashMap<u32, String>,
) -> Vec<Binding> {
    let indices = all_indices(placeholders, defaults);
    let mut out: Vec<Binding> = Vec::new();
    // (source, metric, instance) of a named slot -> its binding
    let mut named: std::collections::HashMap<(String, String, u32), usize> =
        std::collections::HashMap::new();
    for request in requests(&indices, defaults) {
        let default = defaults.get(&request[0]);
        let key = default
            .filter(|d| !d.source.is_empty() && d.instance > 0)
            .map(|d| (d.source.clone(), d.metric.clone(), d.instance));
        if let Some(at) = key.as_ref().and_then(|k| named.get(k)) {
            out[*at].indices.extend_from_slice(request);
            continue;
        }
        let hint = hints.get(&request[0]).map(String::as_str).unwrap_or("");
        let (what, part) = split_hint(hint);
        // Placeholders that name no source cannot be grouped by slot; the
        // ones whose hints name the same thing (`CPU power - Value`) follow
        // each other into one binding.
        let unsourced = default.is_none_or(|d| d.source.is_empty());
        if unsourced && !what.is_empty() && !part.is_empty() {
            if let Some(last) = out
                .last_mut()
                .filter(|b| b.source.is_empty() && b.title == what)
            {
                last.indices.extend_from_slice(request);
                continue;
            }
        }
        // A lone placeholder keeps its whole hint unless more may join it.
        let title = if what.is_empty() {
            format!("Placeholder {{{{{}}}}}", request[0])
        } else if request.len() > 1 || key.is_some() || unsourced || part.is_empty() {
            what.to_string()
        } else {
            hint.trim().to_string()
        };
        if let Some(key) = key {
            named.insert(key, out.len());
        }
        out.push(Binding {
            title,
            source: default.map(|d| d.source.clone()).unwrap_or_default(),
            metric: default.map(|d| d.metric.clone()).unwrap_or_default(),
            indices: request.to_vec(),
        });
    }
    for binding in &mut out {
        binding.indices.sort_unstable();
    }
    out
}

/// The text a placeholder shows for `mapping`, as the panel renders it:
/// `--` when unmapped or when the slot has no such value.
pub fn mapped_value(
    values: &std::collections::HashMap<String, serde_json::Value>,
    mapping: &PlaceholderMapping,
) -> String {
    let mut out = String::new();
    write_mapped_value(values, mapping, &mut out);
    out
}

/// Write mapped value to output
pub fn write_mapped_value(
    values: &std::collections::HashMap<String, serde_json::Value>,
    mapping: &PlaceholderMapping,
    output: &mut String,
) {
    if mapping.slot_prefix.is_empty() {
        output.push_str("--");
        return;
    }

    let key = format!("{}_{}", mapping.slot_prefix, mapping.field);

    if let Some(value) = values.get(&key) {
        write_formatted_value(value, mapping.format.as_deref(), output);
        return;
    }

    // The mapping editor and the templates call a slot's limits "min"/"max";
    // sources publish them as `min_limit`/`max_limit`.
    if matches!(mapping.field.as_str(), "min" | "max") {
        let key = format!("{}_{}_limit", mapping.slot_prefix, mapping.field);
        if let Some(value) = values.get(&key) {
            write_formatted_value(value, mapping.format.as_deref(), output);
            return;
        }
    }

    if let Some(value) = values.get(&mapping.slot_prefix) {
        write_formatted_value(value, mapping.format.as_deref(), output);
        return;
    }

    output.push_str("--");
}

/// Write formatted JSON value
pub fn write_formatted_value(value: &serde_json::Value, format: Option<&str>, output: &mut String) {
    use serde_json::Value;
    match value {
        Value::Number(n) => {
            if let Some(f) = n.as_f64() {
                write_format_value_to_buffer(f, format, output);
            } else {
                use std::fmt::Write;
                let _ = write!(output, "{}", n);
            }
        }
        Value::String(s) => output.push_str(s),
        Value::Bool(b) => output.push_str(if *b { "true" } else { "false" }),
        _ => {
            use std::fmt::Write;
            let _ = write!(output, "{}", value);
        }
    }
}

pub(crate) fn normalized(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

/// A template names a source by id or by its first word (`fan` for
/// `fan_speed`, `system` for `system_temp`).
pub(crate) fn source_matches(slot_source: &str, wanted: &str) -> bool {
    let (slot, wanted) = (normalized(slot_source), normalized(wanted));
    !wanted.is_empty() && !slot.is_empty() && (slot == wanted || slot.starts_with(&wanted))
}

/// Every placeholder index a template uses or configures, ascending.
fn all_indices(
    placeholders: &[u32],
    defaults: &std::collections::HashMap<u32, PlaceholderDefault>,
) -> Vec<u32> {
    placeholders
        .iter()
        .chain(defaults.keys())
        .copied()
        .collect::<std::collections::BTreeSet<u32>>()
        .into_iter()
        .collect()
}

/// Split the placeholders into requests for one slot each: four consecutive
/// indices asking for caption / value / unit / max of one source form one
/// request, every other placeholder is its own.
fn requests<'a>(
    indices: &'a [u32],
    defaults: &std::collections::HashMap<u32, PlaceholderDefault>,
) -> Vec<&'a [u32]> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < indices.len() {
        let index = indices[i];
        let group = defaults.get(&index).is_some_and(|first| {
            !first.source.is_empty()
                && i + 3 < indices.len()
                && ["caption", "value", "unit", "max"]
                    .iter()
                    .enumerate()
                    .all(|(j, field)| {
                        indices[i + j] == index + j as u32
                            && defaults.get(&indices[i + j]).is_some_and(|d| {
                                d.field == *field
                                    && d.source == first.source
                                    && d.metric == first.metric
                                    && d.instance == first.instance
                            })
                    })
        });
        let members = if group { 4 } else { 1 };
        out.push(&indices[i..i + members]);
        i += members;
    }
    out
}

/// The slots a template needs, as `(source, metric)` in the order the
/// template first asks for them; a pair appears once per slot it needs.
/// Scan Placeholders creates the ones the panel's source does not have yet.
pub fn wanted_slots(
    placeholders: &[u32],
    defaults: &std::collections::HashMap<u32, PlaceholderDefault>,
) -> Vec<(String, String)> {
    let indices = all_indices(placeholders, defaults);
    // per (source, metric): [slots taken in turn, highest slot named outright]
    let mut order: Vec<(String, String)> = Vec::new();
    let mut counts: std::collections::HashMap<(String, String), [u32; 2]> =
        std::collections::HashMap::new();
    for request in requests(&indices, defaults) {
        let Some(default) = defaults.get(&request[0]).filter(|d| !d.source.is_empty()) else {
            continue;
        };
        let key = (default.source.clone(), default.metric.clone());
        if !counts.contains_key(&key) {
            order.push(key.clone());
        }
        let count = counts.entry(key).or_insert([0, 0]);
        if default.instance == 0 {
            count[0] += 1;
        } else {
            count[1] = count[1].max(default.instance);
        }
    }
    order
        .into_iter()
        .flat_map(|key| {
            let n = counts[&key].iter().copied().max().unwrap_or(0) as usize;
            std::iter::repeat_n(key, n)
        })
        .collect()
}

/// Map every placeholder of a template to a slot of the panel's source.
///
/// `placeholders` are the `{{N}}` indices found in the template, `defaults`
/// its `rg-placeholder-config` block. Four consecutive placeholders that ask
/// for caption / value / unit / max of one source share a slot. A slot is
/// chosen by source type and, when the template names one, by metric, so the
/// order the slots were added in does not matter; `instance` picks among
/// several matching slots. A mapping in `keep` that already has a slot is
/// left alone. What nothing matches stays unmapped, which a template shows as
/// "--" or hides.
pub fn auto_map(
    placeholders: &[u32],
    defaults: &std::collections::HashMap<u32, PlaceholderDefault>,
    keep: &std::collections::HashMap<u32, PlaceholderMapping>,
    slots: &[SlotKind],
) -> Vec<PlaceholderMapping> {
    use std::collections::HashSet;

    let indices = all_indices(placeholders, defaults);
    let mut taken: HashSet<&str> = HashSet::new();
    let mut pick = |default: &PlaceholderDefault| -> String {
        let candidates: Vec<&SlotKind> = slots
            .iter()
            .filter(|slot| source_matches(&slot.source, &default.source))
            .filter(|slot| {
                default.metric.is_empty() || normalized(&slot.metric) == normalized(&default.metric)
            })
            .collect();
        let chosen = if default.instance > 0 {
            candidates.get(default.instance as usize - 1).copied()
        } else {
            candidates
                .iter()
                .find(|slot| !taken.contains(slot.prefix.as_str()))
                .copied()
        };
        match chosen {
            Some(slot) => {
                taken.insert(slot.prefix.as_str());
                slot.prefix.clone()
            }
            None => String::new(),
        }
    };

    let mut out = Vec::with_capacity(indices.len());
    for request in requests(&indices, defaults) {
        let default = defaults.get(&request[0]);
        let kept = request
            .iter()
            .filter_map(|idx| keep.get(idx))
            .find(|m| !m.slot_prefix.is_empty())
            .map(|m| m.slot_prefix.clone());
        let prefix = match (kept, default) {
            (Some(prefix), _) => prefix,
            (None, Some(default)) => pick(default),
            (None, None) => String::new(),
        };
        for idx in request {
            let default = defaults.get(idx);
            let existing = keep.get(idx).filter(|m| !m.slot_prefix.is_empty());
            out.push(PlaceholderMapping {
                index: *idx,
                slot_prefix: prefix.clone(),
                field: existing
                    .map(|m| m.field.clone())
                    .or_else(|| default.map(|d| d.field.clone()))
                    .unwrap_or_else(|| "value".to_string()),
                format: existing
                    .and_then(|m| m.format.clone())
                    .or_else(|| default.and_then(|d| d.format.clone())),
            });
        }
    }
    out
}

/// Transform placeholders in HTML for JavaScript injection
///
/// Converts `{{0}}` to `<span data-placeholder="0" class="rg-placeholder">--</span>`
/// so that values can be updated via JavaScript without re-rendering HTML.
pub fn transform_template(html: &str) -> String {
    PLACEHOLDER_REGEX
        .replace_all(html, |caps: &regex::Captures| {
            let idx = &caps[1];
            format!(
                r#"<span data-placeholder="{}" class="rg-placeholder">--</span>"#,
                idx
            )
        })
        .to_string()
}

/// Generate the JavaScript bridge code for value updates
///
/// This creates a `window.updateValues(values)` function that
/// efficiently updates DOM elements with data-placeholder attributes.
/// If the template already defines `updateValues`, it wraps it to also
/// update data-placeholder elements.
pub fn generate_update_script() -> &'static str {
    r#"
(function() {
    // Save any existing updateValues function from the template
    var templateUpdateValues = window.updateValues;

    // Value update function - called from Rust via evaluate_javascript
    window.updateValues = function(values) {
        // First, update all data-placeholder elements (fallback/default behavior)
        for (const [idx, value] of Object.entries(values)) {
            const elements = document.querySelectorAll(`[data-placeholder="${idx}"]`);
            elements.forEach(el => {
                // Preserve any CSS transitions by just updating content
                if (el.textContent !== value) {
                    el.textContent = value;
                }
            });
        }

        // Then call the template's custom updateValues if it exists
        // This allows templates to have their own animation/update logic
        if (typeof templateUpdateValues === 'function') {
            templateUpdateValues(values);
        }
    };

    // Animation speed control
    window.setAnimationSpeed = function(speed) {
        document.documentElement.style.setProperty('--rg-animation-speed', speed);
    };

    // The panel's Animation switch and speed. Called with every value update;
    // a template that animates listens for the `rg-motion` event (detail:
    // {enabled, speed}) or reads `data-rg-motion="on|off"` on <html>.
    var motionState = '';
    window.rgSetMotion = function(enabled, speed) {
        var state = (enabled ? 'on' : 'off') + ':' + speed;
        if (state === motionState) return;
        motionState = state;
        if (speed > 0) window.setAnimationSpeed(speed);
        document.documentElement.setAttribute('data-rg-motion', enabled ? 'on' : 'off');
        window.dispatchEvent(new CustomEvent('rg-motion', { detail: { enabled: !!enabled, speed: speed } }));
    };

    // Theme color injection (for templates that want to use panel theme)
    window.setThemeColors = function(colors) {
        const root = document.documentElement;
        if (colors.color1) root.style.setProperty('--rg-theme-color1', colors.color1);
        if (colors.color2) root.style.setProperty('--rg-theme-color2', colors.color2);
        if (colors.color3) root.style.setProperty('--rg-theme-color3', colors.color3);
        if (colors.color4) root.style.setProperty('--rg-theme-color4', colors.color4);
    };

    // Fit to the panel. A template that states the size it was drawn for
    //   <meta name="rg-design-size" content="480x1920">
    // is scaled to the panel it is shown in. It may list other arrangements
    // of the same design, narrowest first
    //   <meta name="rg-layouts" content="480x1920 960x1040 1440x800">
    // and the one that fills the panel best is chosen; the page lays itself
    // out from --rg-w / --rg-h and data-rg-columns on <html>. Two custom
    // properties steer it (template options usually set them):
    //   --rg-fit      cover (default: fill the panel, cropping) | contain | stretch
    //   --rg-columns  0 = choose (default), N = use the Nth layout
    // A template without the meta tag is left exactly as it is.
    (function () {
        var meta = document.querySelector('meta[name="rg-design-size"]');
        if (!meta) return;
        var size = function (text) {
            var m = /^(\d+)x(\d+)$/.exec(text);
            return m ? { w: +m[1], h: +m[2] } : null;
        };
        var design = size(meta.content.trim());
        if (!design) return;
        var more = document.querySelector('meta[name="rg-layouts"]');
        var layouts = more ? more.content.trim().split(/\s+/).map(size).filter(Boolean) : [];
        if (!layouts.length) layouts = [design];
        var root = document.documentElement, body = document.body;
        var css = getComputedStyle(root);
        var fit = (css.getPropertyValue('--rg-fit') || '').trim() || 'cover';
        var columns = parseInt(css.getPropertyValue('--rg-columns'), 10) || 0;
        root.style.overflow = 'hidden';
        body.style.position = 'absolute';
        body.style.left = '0';
        body.style.top = '0';
        body.style.transformOrigin = '0 0';
        var shown = '';
        function apply() {
            var iw = window.innerWidth, ih = window.innerHeight;
            if (!(iw > 0 && ih > 0)) return;
            var at = 0;
            if (columns > 0) {
                at = Math.min(columns, layouts.length) - 1;
            } else {
                var best = 0;
                layouts.forEach(function (l, i) {
                    var s = Math.min(iw / l.w, ih / l.h);
                    if (s > best * 1.02) { best = s; at = i; }
                });
            }
            var l = layouts[at];
            var sx = iw / l.w, sy = ih / l.h;
            if (fit === 'contain') sx = sy = Math.min(sx, sy);
            else if (fit !== 'stretch') sx = sy = Math.max(sx, sy);
            var key = [at, l.w, l.h, sx.toFixed(4), sy.toFixed(4), iw, ih].join(' ');
            if (key === shown) return;
            shown = key;
            root.style.setProperty('--rg-w', l.w + 'px');
            root.style.setProperty('--rg-h', l.h + 'px');
            root.setAttribute('data-rg-columns', String(at + 1));
            body.style.width = l.w + 'px';
            body.style.height = l.h + 'px';
            body.style.transform = 'translate(' + ((iw - l.w * sx) / 2).toFixed(2) + 'px,' +
                ((ih - l.h * sy) / 2).toFixed(2) + 'px) scale(' + sx.toFixed(5) + ',' + sy.toFixed(5) + ')';
            window.dispatchEvent(new CustomEvent('rg-layout', { detail: { columns: at + 1, width: l.w, height: l.h } }));
        }
        window.addEventListener('resize', apply);
        apply();
    })();

    // Signal that the bridge is ready
    window.rgBridgeReady = true;
    console.log('rg-sens CSS Template bridge initialized');
})();
"#
}

/// Generate the base CSS styles for templates
///
/// These styles ensure placeholders have sensible defaults and
/// provide CSS custom properties for theme integration.
pub fn generate_base_styles() -> &'static str {
    r#"
:root {
    /* Default theme colors (can be overridden by setThemeColors) */
    --rg-theme-color1: #ff6b6b;
    --rg-theme-color2: #4ecdc4;
    --rg-theme-color3: #45b7d1;
    --rg-theme-color4: #96ceb4;

    /* Animation speed (can be overridden by setAnimationSpeed) */
    --rg-animation-speed: 1;
}

/* Base styles for placeholders */
.rg-placeholder {
    display: inline;
    transition: all calc(0.3s / var(--rg-animation-speed)) ease-out;
}

/* Utility classes for templates */
.rg-fade-in {
    animation: rgFadeIn calc(0.5s / var(--rg-animation-speed)) ease-out;
}

@keyframes rgFadeIn {
    from { opacity: 0; }
    to { opacity: 1; }
}

/* Ensure body fills WebView */
html, body {
    margin: 0;
    padding: 0;
    width: 100%;
    height: 100%;
    overflow: hidden;
}
"#
}

/// Case-insensitive search for an ASCII needle, returning a byte offset valid
/// for the ORIGINAL haystack.
///
/// Searching in `haystack.to_lowercase()` is NOT safe for computing insertion
/// offsets: some Unicode lowercase mappings change byte length (e.g. 'İ' is 2
/// bytes but lowercases to 3), shifting every later offset and potentially
/// causing `insert_str` to panic on a non-char-boundary. Comparing raw byte
/// windows with `eq_ignore_ascii_case` preserves offsets, and any match starts
/// with an ASCII byte ('<'), which is always a char boundary in UTF-8.
fn find_ascii_ci(haystack: &str, needle: &str) -> Option<usize> {
    debug_assert!(needle.is_ascii());
    let h = haystack.as_bytes();
    let n = needle.as_bytes();
    if n.is_empty() {
        return Some(0);
    }
    if h.len() < n.len() {
        return None;
    }
    h.windows(n.len()).position(|w| w.eq_ignore_ascii_case(n))
}

/// Combine HTML template with CSS and JavaScript for WebView loading
///
/// This creates a complete HTML document ready for loading into the WebView,
/// injecting the base styles, user CSS, and JavaScript bridge.
pub fn prepare_html_document(
    transformed_html: &str,
    user_css: Option<&str>,
    embedded_css: Option<&str>,
) -> String {
    let base_styles = generate_base_styles();
    let bridge_script = generate_update_script();

    // Check if the template already has <html> structure
    let has_html_tag = find_ascii_ci(transformed_html, "<html").is_some();
    let has_head_tag = find_ascii_ci(transformed_html, "<head").is_some();
    let has_body_tag = find_ascii_ci(transformed_html, "<body").is_some();

    if has_html_tag && has_head_tag && has_body_tag {
        // Template has full structure - inject our styles and scripts
        let mut result = transformed_html.to_string();

        // Inject base styles at the start of <head>
        if let Some(pos) = find_ascii_ci(&result, "<head>") {
            let insert_pos = pos + 6;
            let styles = format!("<style>{}</style>", base_styles);
            result.insert_str(insert_pos, &styles);
        }

        // Inject user CSS if provided
        if let Some(css) = user_css {
            if let Some(pos) = find_ascii_ci(&result, "</head>") {
                let styles = format!("<style>{}</style>", css);
                result.insert_str(pos, &styles);
            }
        }

        // Inject embedded CSS if provided
        if let Some(css) = embedded_css {
            if let Some(pos) = find_ascii_ci(&result, "</head>") {
                let styles = format!("<style>{}</style>", css);
                result.insert_str(pos, &styles);
            }
        }

        // Inject bridge script at end of body
        if let Some(pos) = find_ascii_ci(&result, "</body>") {
            let script = format!("<script>{}</script>", bridge_script);
            result.insert_str(pos, &script);
        }

        result
    } else {
        // Template is just content - wrap in full HTML structure
        let user_css_block = user_css
            .map(|css| format!("<style>{}</style>", css))
            .unwrap_or_default();
        let embedded_css_block = embedded_css
            .map(|css| format!("<style>{}</style>", css))
            .unwrap_or_default();

        format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <style>{}</style>
    {}
    {}
</head>
<body>
{}
<script>{}</script>
</body>
</html>"#,
            base_styles, user_css_block, embedded_css_block, transformed_html, bridge_script
        )
    }
}

/// Format a value using an optional format string
///
/// Supports basic format patterns:
/// - `{:.1}` - 1 decimal place
/// - `{:.2}%` - 2 decimal places with % suffix
/// - `{}` or None - raw value
pub fn format_value(value: f64, format: Option<&str>) -> String {
    let mut output = String::with_capacity(16);
    write_format_value_to_buffer(value, format, &mut output);
    output
}

/// Write a formatted value directly to a buffer without intermediate allocations
///
/// This is the allocation-free version of format_value for use in hot paths.
/// Supports the same format patterns as format_value.
pub fn write_format_value_to_buffer(value: f64, format: Option<&str>, output: &mut String) {
    use std::fmt::Write;

    match format {
        Some(fmt) if fmt.contains("{:.") => {
            // Extract precision from format like "{:.1}" or "{:.2}%"
            if let Some(start) = fmt.find("{:.") {
                let rest = &fmt[start + 3..];
                if let Some(end) = rest.find('}') {
                    let precision_str = &rest[..end];
                    if let Ok(precision) = precision_str.parse::<usize>() {
                        // Write prefix
                        let prefix = &fmt[..start];
                        output.push_str(prefix);
                        // Write formatted value directly to buffer
                        let _ = write!(output, "{:.prec$}", value, prec = precision);
                        // Write suffix
                        let suffix = &rest[end + 1..];
                        output.push_str(suffix);
                        return;
                    }
                }
            }
            // Fallback: write raw value
            let _ = write!(output, "{}", value);
        }
        Some(fmt) if fmt.contains("{}") => {
            // Find {} and replace inline
            if let Some(pos) = fmt.find("{}") {
                output.push_str(&fmt[..pos]);
                let _ = write!(output, "{}", value);
                output.push_str(&fmt[pos + 2..]);
            } else {
                let _ = write!(output, "{}", value);
            }
        }
        Some(_) | None => {
            let _ = write!(output, "{}", value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_placeholders() {
        let html = r#"<div>{{0}} and {{1}} and {{0}} and {{5}}</div>"#;
        let indices = detect_placeholders(html);
        assert_eq!(indices, vec![0, 1, 5]);
    }

    #[test]
    fn test_transform_template() {
        let html = r#"<span>{{0}}</span>"#;
        let transformed = transform_template(html);
        assert!(transformed.contains(r#"data-placeholder="0""#));
        assert!(transformed.contains("rg-placeholder"));
    }

    /// The Art Deco example: the gauge's four placeholders are one binding,
    /// the clock's five (four up front, the hour hand at the end) another,
    /// and CPU power, which names no source, still gets its row.
    #[test]
    fn bindings_group_what_shares_a_sensor() {
        let html = include_str!("../../examples/art_deco_panel.html");
        let (placeholders, defaults, hints) = (
            detect_placeholders(html),
            extract_placeholder_defaults(html),
            extract_placeholder_hints(html),
        );
        let all = bindings(&placeholders, &defaults, &hints);
        let total: usize = all.iter().map(|b| b.indices.len()).sum();
        assert_eq!(total, 74, "every placeholder is in exactly one binding");
        assert_eq!(all[0].title, "Clock");
        assert_eq!(all[0].indices, [0, 1, 2, 3, 72]);
        assert_eq!(all[1].title, "CPU temperature (gauge)");
        assert_eq!(all[1].indices, [4, 5, 6, 7]);
        assert_eq!(
            (all[1].source.as_str(), all[1].metric.as_str()),
            ("cpu", "temperature")
        );
        let power = all.iter().find(|b| b.indices == [16, 17, 18, 19]).unwrap();
        assert!(power.source.is_empty());
    }

    /// No config block: one row per placeholder, named by its number.
    #[test]
    fn bindings_without_a_config_block() {
        let none = std::collections::HashMap::new();
        let all = bindings(&[0, 3], &none, &std::collections::HashMap::new());
        assert_eq!(all.len(), 2);
        assert_eq!(all[1].title, "Placeholder {{3}}");
        assert_eq!(all[1].indices, [3]);
    }

    const OPTIONS: &str = r##"<script type="application/json" id="rg-template-options">[
        {"id": "fps", "label": "FPS", "type": "number", "var": "--motion-fps", "min": 0, "max": 30, "default": 6},
        {"id": "palette", "type": "choice", "default": "cyan", "choices": [
            {"value": "cyan", "set": {"--a": "#22E4FF", "--b": "#FF2BD6"}},
            {"value": "lime", "set": {"--a": "#B6FF3B", "--b": "#22E4FF"}}]},
        {"id": "glow", "type": "color", "var": "--glow", "default": "#ff7a1a"},
        {"id": "alert", "type": "toggle", "var": "--auto-alert", "default": true},
        {"id": "evil", "type": "choice", "var": "--x", "default": "a", "choices": [{"value": "a;} body{display:none"}]},
        {"id": "badvar", "type": "number", "var": "color", "default": 1},
        {"id": "fps", "type": "toggle", "var": "--dup", "default": false}
    ]</script>"##;

    #[test]
    fn template_options_are_parsed_and_unsafe_ones_dropped() {
        let options = extract_template_options(OPTIONS);
        let ids: Vec<&str> = options.iter().map(|o| o.id.as_str()).collect();
        assert_eq!(ids, ["fps", "palette", "glow", "alert"]);
        assert_eq!(options[0].default, "6");
        assert_eq!(options[2].default, "#FF7A1A");
        assert_eq!(options[3].default, "1");
    }

    /// Only declared properties with values the option accepts reach the page.
    #[test]
    fn options_css_writes_only_validated_values() {
        let options = extract_template_options(OPTIONS);
        let values = |pairs: &[(&str, &str)]| {
            pairs
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect::<std::collections::BTreeMap<_, _>>()
        };
        assert_eq!(options_css(&options, &values(&[])), "");
        assert_eq!(
            options_css(&options, &values(&[("fps", "6"), ("palette", "cyan")])),
            "",
            "defaults write nothing"
        );
        assert_eq!(
            options_css(
                &options,
                &values(&[("fps", "99"), ("palette", "lime"), ("alert", "0")])
            ),
            ":root{--motion-fps: 30;--a: #B6FF3B;--b: #22E4FF;--auto-alert: 0;}"
        );
        // not a choice, not a colour, not declared: nothing
        let hostile = values(&[
            ("palette", "x;} body{display:none}"),
            ("glow", "red;} *{display:none"),
            ("unknown", "1"),
        ]);
        assert_eq!(options_css(&options, &hostile), "");
    }

    /// The shipped panels: every option writes custom properties their
    /// stylesheet really has, and the declared default is the stylesheet's
    /// value, so the dialog shows what the panel does before anything is set.
    #[test]
    fn example_panel_options_match_their_stylesheets() {
        let panels = [
            (
                include_str!("../../examples/art_deco_panel.html"),
                include_str!("../../examples/art_deco_panel.css"),
            ),
            (
                include_str!("../../examples/art_nouveau_panel.html"),
                include_str!("../../examples/art_nouveau_panel.css"),
            ),
            (
                include_str!("../../examples/steampunk_panel.html"),
                include_str!("../../examples/steampunk_panel.css"),
            ),
            (
                include_str!("../../examples/neon_panel.html"),
                include_str!("../../examples/neon_panel.css"),
            ),
            (
                include_str!("../../examples/lcars_panel.html"),
                include_str!("../../examples/lcars_panel.css"),
            ),
            (
                include_str!("../../examples/warrior_panel.html"),
                include_str!("../../examples/warrior_panel.css"),
            ),
            (
                include_str!("../../examples/leather_wood_panel.html"),
                include_str!("../../examples/leather_wood_panel.css"),
            ),
            (
                include_str!("../../examples/lcd_panel.html"),
                include_str!("../../examples/lcd_panel.css"),
            ),
        ];
        for (html, css) in panels {
            assert!(html.contains(r#"<meta name="rg-design-size" content="480x1920">"#));
            let options = extract_template_options(html);
            assert!(
                options.iter().any(|o| o.id == "fit"),
                "every panel can be fitted"
            );
            // the first `:root { ... }` block holds the defaults
            let root = css
                .split_once(":root {")
                .and_then(|(_, rest)| rest.split_once('}'))
                .unwrap()
                .0;
            let declared = |name: &str| -> Option<String> {
                root.lines()
                    .filter_map(|line| line.trim().strip_suffix(';')?.split_once(':'))
                    .find(|(k, _)| k.trim() == name)
                    .map(|(_, v)| v.trim().to_string())
            };
            for option in &options {
                let values: std::collections::BTreeMap<String, String> =
                    [(option.id.clone(), option.default.clone())].into();
                assert_eq!(
                    options_css(&options, &values),
                    "",
                    "{}: the default writes nothing",
                    option.id
                );
                for (name, value) in option.declarations(&option.default) {
                    let in_css = declared(&name)
                        .unwrap_or_else(|| panic!("{name} is not in the stylesheet"));
                    assert!(
                        in_css.eq_ignore_ascii_case(&value)
                            || in_css.parse::<f64>().ok() == value.parse::<f64>().ok()
                                && value.parse::<f64>().is_ok(),
                        "{}: default {value} but the stylesheet has {in_css}",
                        option.id
                    );
                }
            }
        }
    }

    #[test]
    fn build_document_applies_options_after_the_template_stylesheet() {
        let html = format!("<html><head><link rel=\"stylesheet\" href=\"x.css\"></head><body>{{{{0}}}}{OPTIONS}</body></html>");
        let mut config = CssTemplateDisplayConfig::default();
        config.options.insert("fps".to_string(), "12".to_string());
        let doc = build_document(&html, &config, None);
        let (link, style) = (
            doc.find("x.css").unwrap(),
            doc.find(":root{--motion-fps: 12;}").unwrap(),
        );
        assert!(link < style);
        assert!(doc.contains("data-placeholder=\"0\""));
    }

    #[test]
    fn test_format_value() {
        assert_eq!(format_value(45.678, Some("{:.1}%")), "45.7%");
        assert_eq!(format_value(45.678, Some("{:.2}")), "45.68");
        assert_eq!(format_value(45.0, None), "45");
    }
}
