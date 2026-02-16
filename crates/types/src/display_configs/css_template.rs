//! CSS Template display configuration types

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
