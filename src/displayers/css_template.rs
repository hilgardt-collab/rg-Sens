//! CSS Template Combo Displayer - Renders HTML/CSS templates with pluggable backends
//!
//! This displayer allows users to create custom visualizations using HTML and CSS,
//! with mustache-style enumerated placeholders (`{{0}}`, `{{1}}`, etc.) that are
//! mapped to data sources via the configuration dialog.
//!
//! Backends:
//! - WebKit (default): Full HTML/CSS/JavaScript support via WebKitGTK
//! - Servo (experimental): Rust-native rendering via Servo's embedding API
//!
//! Features:
//! - Full CSS3 support (flexbox, grid, animations, transitions)
//! - Hot-reload when template files change
//! - JavaScript bridge for smooth value updates without re-rendering
//! - Theme color integration via CSS custom properties

use anyhow::Result;
use cairo::Context;
use gtk4::Widget;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::core::{ConfigOption, ConfigSchema, Displayer, PanelTransform};
use crate::displayers::combo_utils;

// Re-export shutdown function from the active backend
#[cfg(feature = "webkit")]
pub use crate::displayers::webkit_backend::shutdown_all;

// Backend abstraction
use crate::displayers::css_template_backend::{BackendType, DisplayData, TemplateBackend};

#[cfg(feature = "webkit")]
use crate::displayers::webkit_backend::WebKitBackend;

#[cfg(feature = "servo")]
use crate::displayers::servo_backend::ServoBackend;

use crate::ui::css_template_display::{
    detect_placeholders, extract_placeholder_hints, CssTemplateDisplayConfig,
};

/// CSS Template Combo Displayer
pub struct CssTemplateDisplayer {
    id: String,
    name: String,
    data: Arc<Mutex<DisplayData>>,
    backend_type: BackendType,
}

impl CssTemplateDisplayer {
    /// Create a new CSS Template displayer with WebKit backend
    #[cfg(feature = "webkit")]
    pub fn with_webkit_backend() -> Self {
        Self {
            id: "css_template".to_string(),
            name: "CSS Template".to_string(),
            data: Arc::new(Mutex::new(DisplayData::default())),
            backend_type: BackendType::WebKit,
        }
    }

    /// Create a new CSS Template displayer with Servo backend
    #[cfg(feature = "servo")]
    pub fn with_servo_backend() -> Self {
        Self {
            id: "css_template".to_string(),
            name: "CSS Template".to_string(),
            data: Arc::new(Mutex::new(DisplayData::default())),
            backend_type: BackendType::Servo,
        }
    }

    /// Create a new CSS Template displayer with the default backend
    /// (WebKit if available, otherwise Servo)
    pub fn new() -> Self {
        #[cfg(feature = "webkit")]
        {
            Self::with_webkit_backend()
        }
        #[cfg(all(feature = "servo", not(feature = "webkit")))]
        {
            Self::with_servo_backend()
        }
        #[cfg(not(any(feature = "webkit", feature = "servo")))]
        {
            compile_error!(
                "CSS Template displayer requires either 'webkit' or 'servo' feature to be enabled"
            );
        }
    }

    /// Get placeholder hints extracted from the current template
    pub fn get_placeholder_hints(&self) -> HashMap<u32, String> {
        if let Ok(data) = self.data.lock() {
            data.placeholder_hints.clone()
        } else {
            HashMap::new()
        }
    }
}

impl Default for CssTemplateDisplayer {
    fn default() -> Self {
        Self::new()
    }
}

impl Displayer for CssTemplateDisplayer {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn create_widget(&self) -> Widget {
        match self.backend_type {
            #[cfg(feature = "webkit")]
            BackendType::WebKit => {
                let backend = WebKitBackend::new();
                backend.create_widget(self.data.clone())
            }
            #[cfg(feature = "servo")]
            BackendType::Servo => {
                let backend = ServoBackend::new();
                backend.create_widget(self.data.clone())
            }
        }
    }

    fn update_data(&mut self, data: &HashMap<String, Value>) {
        // Use try_lock to avoid blocking tokio worker threads
        if let Ok(mut display_data) = self.data.try_lock() {
            let mut values = std::mem::take(&mut display_data.values);
            combo_utils::filter_values_with_owned_prefix_set(
                data,
                &display_data.cached_prefix_set,
                &mut values,
            );
            display_data.values = values;
            display_data.transform = PanelTransform::from_values(data);
            display_data.dirty = true;
        }
    }

    fn draw(&self, _cr: &Context, _width: f64, _height: f64) -> Result<()> {
        // Backend handles its own drawing
        Ok(())
    }

    fn config_schema(&self) -> ConfigSchema {
        ConfigSchema {
            options: vec![
                ConfigOption {
                    key: "html_path".to_string(),
                    name: "HTML Template".to_string(),
                    description: "Path to the HTML template file".to_string(),
                    value_type: "file".to_string(),
                    default: serde_json::json!(""),
                },
                ConfigOption {
                    key: "css_path".to_string(),
                    name: "CSS File".to_string(),
                    description: "Optional path to external CSS file".to_string(),
                    value_type: "file".to_string(),
                    default: serde_json::json!(null),
                },
                ConfigOption {
                    key: "hot_reload".to_string(),
                    name: "Hot Reload".to_string(),
                    description: "Automatically reload when files change".to_string(),
                    value_type: "boolean".to_string(),
                    default: serde_json::json!(true),
                },
            ],
        }
    }

    fn apply_config(&mut self, config: &HashMap<String, Value>) -> Result<()> {
        // Check for full config first
        if let Some(config_value) = config.get("css_template_config") {
            if let Ok(css_config) =
                serde_json::from_value::<CssTemplateDisplayConfig>(config_value.clone())
            {
                if let Ok(mut display_data) = self.data.lock() {
                    display_data.config = css_config;
                    display_data.cached_html = None;
                    display_data.config_changed = true;

                    // Detect placeholders and extract hints from the template
                    let html_content = if !display_data.config.html_path.as_os_str().is_empty()
                        && display_data.config.html_path.exists()
                    {
                        fs::read_to_string(&display_data.config.html_path).ok()
                    } else {
                        display_data.config.embedded_html.clone()
                    };

                    if let Some(ref html) = html_content {
                        display_data.detected_placeholders = detect_placeholders(html);
                        display_data.placeholder_hints = extract_placeholder_hints(html);
                    }
                }
                return Ok(());
            }
        }

        // Apply individual settings
        if let Ok(mut display_data) = self.data.lock() {
            if let Some(path) = config.get("html_path").and_then(|v| v.as_str()) {
                display_data.config.html_path = PathBuf::from(path);
                display_data.cached_html = None;
                display_data.config_changed = true;
            }

            if let Some(path) = config.get("css_path").and_then(|v| v.as_str()) {
                display_data.config.css_path = Some(PathBuf::from(path));
                display_data.cached_html = None;
                display_data.config_changed = true;
            }

            if let Some(hot_reload) = config.get("hot_reload").and_then(|v| v.as_bool()) {
                display_data.config.hot_reload = hot_reload;
            }
        }

        Ok(())
    }

    fn needs_redraw(&self) -> bool {
        self.data.try_lock().map(|d| d.dirty).unwrap_or(true)
    }

    fn get_typed_config(&self) -> Option<crate::core::DisplayerConfig> {
        if let Ok(display_data) = self.data.try_lock() {
            Some(crate::core::DisplayerConfig::CssTemplate(
                display_data.config.clone(),
            ))
        } else {
            None
        }
    }
}
