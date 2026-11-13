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

    #[test]
    fn test_format_value() {
        assert_eq!(format_value(45.678, Some("{:.1}%")), "45.7%");
        assert_eq!(format_value(45.678, Some("{:.2}")), "45.68");
        assert_eq!(format_value(45.0, None), "45");
    }
}
