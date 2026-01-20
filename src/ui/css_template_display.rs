//! CSS Template Display - Template parser and utilities for CSS-templated combo panels
//!
//! This module provides:
//! - Placeholder detection (`{{0}}`, `{{1}}`, etc.)
//! - Template transformation for JavaScript injection
//! - JavaScript bridge generation for value updates

use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

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

/// Configuration for a placeholder mapping
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaceholderMapping {
    /// Placeholder index (0, 1, 2...)
    pub index: u32,
    /// Slot prefix from combo source (e.g., "group1_1")
    pub slot_prefix: String,
    /// Field to use (e.g., "value", "caption", "unit", "percent")
    pub field: String,
    /// Optional format string (e.g., "{:.1}%")
    #[serde(default)]
    pub format: Option<String>,
}

impl Default for PlaceholderMapping {
    fn default() -> Self {
        Self {
            index: 0,
            slot_prefix: String::new(),
            field: "value".to_string(),
            format: None,
        }
    }
}

/// Default configuration for a placeholder (from template)
///
/// This defines what source type and field a placeholder expects,
/// allowing auto-configuration when the template is loaded.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlaceholderDefault {
    /// Human-readable description/hint for this placeholder
    #[serde(default)]
    pub hint: String,
    /// Source type ID (e.g., "cpu", "gpu", "memory", "clock", "disk")
    #[serde(default)]
    pub source: String,
    /// Instance index for sources with multiple instances (e.g., CPU core 0, 1, 2)
    #[serde(default)]
    pub instance: u32,
    /// Field to use from the source (e.g., "value", "caption", "unit", "time")
    #[serde(default = "default_field")]
    pub field: String,
    /// Optional format string (e.g., "{:.1}%")
    #[serde(default)]
    pub format: Option<String>,
}

fn default_field() -> String {
    "value".to_string()
}

/// Configuration for the CSS Template displayer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CssTemplateDisplayConfig {
    /// Path to the HTML template file
    #[serde(default)]
    pub html_path: PathBuf,
    /// Optional path to external CSS file
    #[serde(default)]
    pub css_path: Option<PathBuf>,
    /// Mappings from placeholder indices to data sources
    #[serde(default)]
    pub mappings: Vec<PlaceholderMapping>,
    /// Enable hot-reload when template files change
    #[serde(default = "default_hot_reload")]
    pub hot_reload: bool,
    /// Background color for the WebView (RGBA)
    #[serde(default = "default_background_color")]
    pub background_color: [f64; 4],
    /// Enable CSS animations
    #[serde(default = "default_animation_enabled")]
    pub animation_enabled: bool,
    /// Animation speed multiplier
    #[serde(default = "default_animation_speed")]
    pub animation_speed: f64,
    /// Embedded HTML content (used when no file is specified)
    #[serde(default)]
    pub embedded_html: Option<String>,
    /// Embedded CSS content (used when no file is specified)
    #[serde(default)]
    pub embedded_css: Option<String>,
}

fn default_hot_reload() -> bool {
    true
}

fn default_background_color() -> [f64; 4] {
    [0.0, 0.0, 0.0, 0.0] // Transparent
}

fn default_animation_enabled() -> bool {
    true
}

fn default_animation_speed() -> f64 {
    1.0
}

impl Default for CssTemplateDisplayConfig {
    fn default() -> Self {
        Self {
            html_path: PathBuf::new(),
            css_path: None,
            mappings: Vec::new(),
            hot_reload: default_hot_reload(),
            background_color: default_background_color(),
            animation_enabled: default_animation_enabled(),
            animation_speed: default_animation_speed(),
            embedded_html: None,
            embedded_css: None,
        }
    }
}

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
    let has_html_tag = transformed_html.to_lowercase().contains("<html");
    let has_head_tag = transformed_html.to_lowercase().contains("<head");
    let has_body_tag = transformed_html.to_lowercase().contains("<body");

    if has_html_tag && has_head_tag && has_body_tag {
        // Template has full structure - inject our styles and scripts
        let mut result = transformed_html.to_string();

        // Inject base styles at the start of <head>
        if let Some(pos) = result.to_lowercase().find("<head>") {
            let insert_pos = pos + 6;
            let styles = format!("<style>{}</style>", base_styles);
            result.insert_str(insert_pos, &styles);
        }

        // Inject user CSS if provided
        if let Some(css) = user_css {
            if let Some(pos) = result.to_lowercase().find("</head>") {
                let styles = format!("<style>{}</style>", css);
                result.insert_str(pos, &styles);
            }
        }

        // Inject embedded CSS if provided
        if let Some(css) = embedded_css {
            if let Some(pos) = result.to_lowercase().find("</head>") {
                let styles = format!("<style>{}</style>", css);
                result.insert_str(pos, &styles);
            }
        }

        // Inject bridge script at end of body
        if let Some(pos) = result.to_lowercase().find("</body>") {
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

    #[test]
    fn test_format_value() {
        assert_eq!(format_value(45.678, Some("{:.1}%")), "45.7%");
        assert_eq!(format_value(45.678, Some("{:.2}")), "45.68");
        assert_eq!(format_value(45.0, None), "45");
    }
}
