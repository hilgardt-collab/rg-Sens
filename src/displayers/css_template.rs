//! CSS Template Combo Displayer - Renders HTML/CSS templates with WebKitGTK WebView
//!
//! This displayer allows users to create custom visualizations using HTML and CSS,
//! with mustache-style enumerated placeholders (`{{0}}`, `{{1}}`, etc.) that are
//! mapped to data sources via the configuration dialog.
//!
//! Features:
//! - Full CSS3 support (flexbox, grid, animations, transitions)
//! - Hot-reload when template files change
//! - JavaScript bridge for smooth value updates without re-rendering
//! - Theme color integration via CSS custom properties

use anyhow::Result;
use cairo::Context;
use gtk4::{gio, glib, prelude::*, DrawingArea, Overlay, Widget};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};

use crate::core::{ConfigOption, ConfigSchema, Displayer, PanelTransform};
use crate::displayers::combo_utils;
use crate::ui::css_template_display::{
    detect_placeholders, extract_placeholder_hints, format_value, prepare_html_document,
    transform_template, CssTemplateDisplayConfig, PlaceholderMapping,
};

use webkit6::prelude::WebViewExt;
use webkit6::WebView;

/// Internal display data shared between widget and displayer
#[derive(Clone)]
struct DisplayData {
    config: CssTemplateDisplayConfig,
    values: HashMap<String, Value>,
    transform: PanelTransform,
    dirty: bool,
    /// Cached HTML content (transformed and ready to load)
    cached_html: Option<String>,
    /// Currently detected placeholders
    detected_placeholders: Vec<u32>,
    /// Placeholder hints extracted from template (index -> description)
    placeholder_hints: HashMap<u32, String>,
    /// Flag to signal that config changed and WebView needs reload
    config_changed: bool,
}

impl Default for DisplayData {
    fn default() -> Self {
        Self {
            config: CssTemplateDisplayConfig::default(),
            values: HashMap::new(),
            transform: PanelTransform::default(),
            dirty: true,
            cached_html: None,
            detected_placeholders: Vec::new(),
            placeholder_hints: HashMap::new(),
            config_changed: false,
        }
    }
}

/// CSS Template Combo Displayer
pub struct CssTemplateDisplayer {
    id: String,
    name: String,
    data: Arc<Mutex<DisplayData>>,
}

impl CssTemplateDisplayer {
    pub fn new() -> Self {
        Self {
            id: "css_template".to_string(),
            name: "CSS Template".to_string(),
            data: Arc::new(Mutex::new(DisplayData::default())),
        }
    }

    /// Get the base URI for loading the template (for resolving relative paths)
    fn get_base_uri(&self) -> Option<String> {
        let data = self.data.lock().ok()?;
        if !data.config.html_path.as_os_str().is_empty() && data.config.html_path.exists() {
            if let Some(parent) = data.config.html_path.parent() {
                return Some(format!("file://{}/", parent.display()));
            }
        }
        None
    }

    /// Load and transform the template HTML
    fn load_template(&self) -> Option<String> {
        let data = self.data.lock().ok()?;
        let config = &data.config;

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
        // Create WebView
        let webview = WebView::new();

        // Configure WebView settings using the explicit WebViewExt trait
        if let Some(settings) = WebViewExt::settings(&webview) {
            settings.set_enable_javascript(true);
            settings.set_allow_file_access_from_file_urls(true);
            settings.set_allow_universal_access_from_file_urls(true);
            settings.set_enable_developer_extras(false);
            settings.set_enable_page_cache(false);
        }

        // Set transparent background
        webview.set_background_color(&gdk4::RGBA::new(0.0, 0.0, 0.0, 0.0));

        // Disable WebView's own context menu (let parent handle right-click)
        webview.connect_context_menu(|_, _, _| true);

        // Load initial template
        let html = self.load_template();
        let base_uri = self.get_base_uri();
        if let Some(html_content) = html {
            webview.load_html(&html_content, base_uri.as_deref());
            // Cache the HTML
            if let Ok(mut data) = self.data.lock() {
                data.cached_html = Some(html_content);
            }
        }

        // Set up file watcher for hot-reload using atomic flags
        let reload_flag = Arc::new(AtomicBool::new(false));
        let reload_flag_writer = reload_flag.clone();
        // Stop flag to terminate the watcher thread when widget is destroyed
        let stop_flag = Arc::new(AtomicBool::new(false));
        let stop_flag_writer = stop_flag.clone();
        let watcher_data = self.data.clone();

        // Spawn file watcher in a separate thread
        std::thread::spawn(move || {
            let config = Config::default().with_poll_interval(Duration::from_millis(500));

            let flag = reload_flag_writer.clone();
            let mut watcher = match RecommendedWatcher::new(
                move |res: Result<notify::Event, notify::Error>| {
                    if let Ok(event) = res {
                        if event.kind.is_modify() {
                            flag.store(true, Ordering::SeqCst);
                        }
                    }
                },
                config,
            ) {
                Ok(w) => w,
                Err(e) => {
                    log::warn!("Failed to create file watcher: {}", e);
                    return;
                }
            };

            // Watch the template files
            if let Ok(data) = watcher_data.lock() {
                if data.config.hot_reload {
                    if !data.config.html_path.as_os_str().is_empty()
                        && data.config.html_path.exists()
                    {
                        if let Err(e) =
                            watcher.watch(&data.config.html_path, RecursiveMode::NonRecursive)
                        {
                            log::warn!("Failed to watch HTML file: {}", e);
                        }
                    }

                    if let Some(ref css_path) = data.config.css_path {
                        if css_path.exists() {
                            if let Err(e) = watcher.watch(css_path, RecursiveMode::NonRecursive) {
                                log::warn!("Failed to watch CSS file: {}", e);
                            }
                        }
                    }
                }
            }

            // Keep the watcher alive until stop flag is set
            // Check every second instead of every 60 seconds for faster cleanup
            while !stop_flag_writer.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_secs(1));
            }
            log::debug!("CSS template file watcher thread stopped");
            // Watcher is dropped here, releasing file handles
        });

        // Set up periodic check for reload and value updates
        // 250ms is sufficient for hot-reload detection while reducing CPU overhead
        glib::timeout_add_local(Duration::from_millis(250), {
            let data_clone = self.data.clone();
            let webview_weak = webview.downgrade();
            let reload_flag_reader = reload_flag;
            let stop_flag_setter = stop_flag;
            move || {
                let Some(webview) = webview_weak.upgrade() else {
                    // Widget is destroyed - signal the file watcher thread to stop
                    stop_flag_setter.store(true, Ordering::SeqCst);
                    return glib::ControlFlow::Break;
                };

                // Skip if not visible
                if !webview.is_mapped() {
                    return glib::ControlFlow::Continue;
                }

                // Check for config change (new template selected)
                let config_changed = data_clone
                    .lock()
                    .ok()
                    .map(|d| d.config_changed)
                    .unwrap_or(false);

                // Check for hot-reload or config change
                if reload_flag_reader.swap(false, Ordering::SeqCst) || config_changed {
                    // Reload template
                    let html_content = {
                        let mut should_reload = config_changed;
                        let config = data_clone.lock().ok().map(|mut d| {
                            if d.config_changed {
                                d.config_changed = false;
                                should_reload = true;
                            }
                            d.config.clone()
                        });

                        if let Some(config) = config {
                            // Reload if config changed OR hot_reload is enabled and file changed
                            if should_reload || config.hot_reload {
                                // Get base URI for relative paths
                                let base_uri = if !config.html_path.as_os_str().is_empty()
                                    && config.html_path.exists()
                                {
                                    config
                                        .html_path
                                        .parent()
                                        .map(|p| format!("file://{}/", p.display()))
                                } else {
                                    None
                                };

                                // Manual load
                                let html = if !config.html_path.as_os_str().is_empty()
                                    && config.html_path.exists()
                                {
                                    fs::read_to_string(&config.html_path).ok()
                                } else {
                                    config.embedded_html.clone()
                                };

                                let css = config.css_path.as_ref().and_then(|p| {
                                    if p.exists() {
                                        fs::read_to_string(p).ok()
                                    } else {
                                        None
                                    }
                                });

                                if let Some(html) = html {
                                    let transformed = transform_template(&html);
                                    Some((
                                        prepare_html_document(
                                            &transformed,
                                            css.as_deref(),
                                            config.embedded_css.as_deref(),
                                        ),
                                        base_uri,
                                    ))
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    };

                    if let Some((html, base_uri)) = html_content {
                        webview.load_html(&html, base_uri.as_deref());
                        if let Ok(mut data) = data_clone.lock() {
                            data.cached_html = Some(html);
                        }
                    }
                }

                // Update values via JavaScript
                if let Ok(mut data) = data_clone.try_lock() {
                    if data.dirty {
                        data.dirty = false;

                        // Format values and inject via JavaScript
                        let mut js_values: HashMap<String, String> = HashMap::new();

                        for mapping in &data.config.mappings {
                            let value = get_mapped_value_static(&data.values, mapping);
                            js_values.insert(mapping.index.to_string(), value);
                        }

                        // Build JavaScript call
                        let entries: Vec<String> = js_values
                            .iter()
                            .map(|(k, v)| {
                                let escaped = v.replace('\\', "\\\\").replace('"', "\\\"");
                                format!(r#""{}": "{}""#, k, escaped)
                            })
                            .collect();

                        let js = format!(
                            "if (window.updateValues) {{ window.updateValues({{{}}}); }}",
                            entries.join(", ")
                        );

                        // Execute JavaScript
                        webview.evaluate_javascript(
                            &js,
                            None,
                            None,
                            None::<&gio::Cancellable>,
                            |_| {},
                        );
                    }
                }

                glib::ControlFlow::Continue
            }
        });

        // Wrap WebView in an Overlay with a transparent event-catching layer on top.
        // This allows drag and right-click events to propagate to the parent Frame
        // while still displaying the WebView content underneath.
        let overlay = Overlay::new();
        overlay.set_child(Some(&webview));

        // Create transparent event layer that sits on top of WebView
        let event_layer = DrawingArea::new();
        event_layer.set_hexpand(true);
        event_layer.set_vexpand(true);
        // Make it transparent (don't draw anything)
        event_layer.set_draw_func(|_, _, _, _| {});
        overlay.add_overlay(&event_layer);

        overlay.upcast()
    }

    fn update_data(&mut self, data: &HashMap<String, Value>) {
        if let Ok(mut display_data) = self.data.lock() {
            // Convert group_item_counts to usize for generate_prefixes
            let group_item_counts: Vec<usize> = (0..10).map(|_| 10).collect(); // Support up to 100 slots

            // Generate prefixes and filter values
            let prefixes = combo_utils::generate_prefixes(&group_item_counts);
            combo_utils::filter_values_by_prefixes_into(data, &prefixes, &mut display_data.values);

            // Also copy any direct values (not prefixed)
            for (key, value) in data {
                if !display_data.values.contains_key(key) {
                    display_data.values.insert(key.clone(), value.clone());
                }
            }

            // Extract transform from values
            display_data.transform = PanelTransform::from_values(data);
            display_data.dirty = true;
        }
    }

    fn draw(&self, _cr: &Context, _width: f64, _height: f64) -> Result<()> {
        // WebView handles its own drawing
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
                    // Invalidate cached HTML and signal reload needed
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

/// Static helper to get mapped value without &self
fn get_mapped_value_static(
    values: &HashMap<String, Value>,
    mapping: &PlaceholderMapping,
) -> String {
    if mapping.slot_prefix.is_empty() {
        return "--".to_string();
    }

    // Build the key based on slot_prefix and field
    let key = format!("{}_{}", mapping.slot_prefix, mapping.field);

    // Try to get the value
    if let Some(value) = values.get(&key) {
        match value {
            Value::Number(n) => {
                if let Some(f) = n.as_f64() {
                    format_value(f, mapping.format.as_deref())
                } else {
                    n.to_string()
                }
            }
            Value::String(s) => s.clone(),
            Value::Bool(b) => b.to_string(),
            _ => value.to_string(),
        }
    } else {
        // Try without the field suffix
        if let Some(value) = values.get(&mapping.slot_prefix) {
            match value {
                Value::Number(n) => {
                    if let Some(f) = n.as_f64() {
                        format_value(f, mapping.format.as_deref())
                    } else {
                        n.to_string()
                    }
                }
                Value::String(s) => s.clone(),
                _ => value.to_string(),
            }
        } else {
            "--".to_string()
        }
    }
}
