//! CSS Template Backend Abstraction
//!
//! This module defines the common interface for CSS Template rendering backends.
//! Currently supports WebKit (via webkit6) and Servo (experimental).

use gtk4::Widget;
use std::sync::{Arc, Mutex};

use crate::ui::css_template_display::CssTemplateDisplayConfig;

/// Backend type enumeration for CSS Template rendering
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendType {
    /// WebKit-based rendering via webkit6 crate
    #[cfg(feature = "webkit")]
    WebKit,
    /// Servo-based rendering (experimental)
    #[cfg(feature = "servo")]
    Servo,
}

impl std::fmt::Display for BackendType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            #[cfg(feature = "webkit")]
            BackendType::WebKit => write!(f, "WebKit"),
            #[cfg(feature = "servo")]
            BackendType::Servo => write!(f, "Servo"),
        }
    }
}

/// Shared data structure for CSS Template displayers
/// This is the common data that both backends need to access
#[derive(Clone)]
pub struct DisplayData {
    pub config: CssTemplateDisplayConfig,
    pub values: std::collections::HashMap<String, serde_json::Value>,
    pub transform: crate::core::PanelTransform,
    pub dirty: bool,
    /// Cached HTML content (transformed and ready to load)
    pub cached_html: Option<String>,
    /// Currently detected placeholders
    pub detected_placeholders: Vec<u32>,
    /// Placeholder hints extracted from template (index -> description)
    pub placeholder_hints: std::collections::HashMap<u32, String>,
    /// Flag to signal that config changed and backend needs reload
    pub config_changed: bool,
    /// Last JavaScript values string sent to backend (for change detection)
    pub last_js_values: String,
    /// Counter for JavaScript update calls (for periodic GC)
    pub js_update_count: u32,
    /// Cached prefix set for O(1) lookups (avoids regenerating prefixes every frame)
    pub cached_prefix_set: std::collections::HashSet<String>,
    /// Reusable key buffer for get_mapped_value lookups
    pub key_buffer: String,
    /// Reusable buffer for building JS entries (avoids allocation per tick)
    pub entries_buffer: Vec<String>,
    /// Reusable buffer for the final JS values string
    pub js_values_buffer: String,
    /// Reusable buffer for intermediate value formatting
    pub value_buffer: String,
    /// Reusable buffer for building the JS call
    pub js_call_buffer: String,
}

impl Default for DisplayData {
    fn default() -> Self {
        use crate::displayers::combo_utils;

        // Pre-generate prefixes once (group1_1 through group10_10 = 100 prefixes)
        // These are generated once and reused for the lifetime of the displayer
        let group_item_counts: Vec<usize> = (0..10).map(|_| 10).collect();
        let prefixes = combo_utils::generate_prefixes(&group_item_counts);
        let cached_prefix_set = prefixes.into_iter().collect();

        Self {
            config: CssTemplateDisplayConfig::default(),
            values: std::collections::HashMap::new(),
            transform: crate::core::PanelTransform::default(),
            dirty: true,
            cached_html: None,
            detected_placeholders: Vec::new(),
            placeholder_hints: std::collections::HashMap::new(),
            config_changed: false,
            last_js_values: String::new(),
            js_update_count: 0,
            cached_prefix_set,
            key_buffer: String::with_capacity(64),
            entries_buffer: Vec::with_capacity(64),
            js_values_buffer: String::with_capacity(1024),
            value_buffer: String::with_capacity(64),
            js_call_buffer: String::with_capacity(2048),
        }
    }
}

/// Trait defining the interface for CSS Template rendering backends
pub trait TemplateBackend: Send + Sync {
    /// Create the GTK widget for this backend
    ///
    /// The backend should set up the rendering context and return a widget
    /// that can be added to the GTK widget tree.
    fn create_widget(&self, data: Arc<Mutex<DisplayData>>) -> Widget;

    /// Get the base URI for loading templates (for resolving relative paths)
    fn get_base_uri(config: &CssTemplateDisplayConfig) -> Option<String> {
        if !config.html_path.as_os_str().is_empty() && config.html_path.exists() {
            if let Some(parent) = config.html_path.parent() {
                return Some(format!("file://{}/", parent.display()));
            }
        }
        None
    }

    /// Load and transform the template HTML
    fn load_template(config: &CssTemplateDisplayConfig) -> Option<String> {
        use crate::ui::css_template_display::{prepare_html_document, transform_template};
        use std::fs;

        // Get HTML content (from file or embedded)
        let html_content = if !config.html_path.as_os_str().is_empty() && config.html_path.exists()
        {
            fs::read_to_string(&config.html_path).ok()?
        } else if let Some(ref embedded) = config.embedded_html {
            embedded.clone()
        } else {
            // Default template
            r#"
<!DOCTYPE html>
<html>
<head>
    <style>
        body {
            display: flex;
            align-items: center;
            justify-content: center;
            height: 100vh;
            margin: 0;
            background: transparent;
            font-family: sans-serif;
            color: var(--rg-theme-color1, #fff);
        }
        .container {
            text-align: center;
            padding: 20px;
        }
        .value {
            font-size: 48px;
            font-weight: bold;
        }
        .label {
            font-size: 14px;
            opacity: 0.7;
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="value">{{0}}</div>
        <div class="label">No template loaded</div>
    </div>
</body>
</html>
"#
            .to_string()
        };

        // Get CSS content (from file or embedded)
        let css_content = if let Some(ref css_path) = config.css_path {
            if css_path.exists() {
                fs::read_to_string(css_path).ok()
            } else {
                None
            }
        } else {
            None
        };

        // Transform placeholders
        let transformed = transform_template(&html_content);

        // Prepare complete document
        Some(prepare_html_document(
            &transformed,
            css_content.as_deref(),
            config.embedded_css.as_deref(),
        ))
    }
}
