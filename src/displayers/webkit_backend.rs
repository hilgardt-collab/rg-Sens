//! WebKit Backend for CSS Template Displayer
//!
//! This module implements the CSS Template rendering using WebKitGTK.
//! It provides full HTML/CSS/JavaScript support with a WebView widget.
//!
//! Every 5 minutes, the WebView is completely destroyed and recreated
//! to prevent memory accumulation from WebKitGTK internals.

use gtk4::{glib, prelude::*, Box as GtkBox, DrawingArea, Orientation, Overlay, Widget};
use serde_json::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use webkit6::prelude::WebViewExt;
use webkit6::{CacheModel, WebView};

use crate::displayers::css_template_backend::{DisplayData, TemplateBackend};
use crate::ui::css_template_display::{
    prepare_html_document, transform_template, write_format_value_to_buffer, PlaceholderMapping,
};

/// Global shutdown flag - when set, all CSS template timers will stop
static SHUTDOWN_FLAG: AtomicBool = AtomicBool::new(false);

// Thread-local registry of active WebViews for proper shutdown.
// Uses thread_local because GTK widgets are not Send+Sync and all GTK
// operations happen on the main thread anyway.
thread_local! {
    static ACTIVE_WEBVIEWS: std::cell::RefCell<Vec<glib::WeakRef<WebView>>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

/// Register a WebView for tracking (called when created)
fn register_webview(webview: &WebView) {
    ACTIVE_WEBVIEWS.with(|views| {
        let mut views = views.borrow_mut();
        views.push(webview.downgrade());
        log::debug!(
            "Registered WebView for shutdown tracking (total: {})",
            views.len()
        );
    });
}

/// Unregister a WebView (called when destroying)
fn unregister_webview(webview: &WebView) {
    ACTIVE_WEBVIEWS.with(|views| {
        let mut views = views.borrow_mut();
        let initial_len = views.len();
        views.retain(|weak| {
            if let Some(v) = weak.upgrade() {
                !std::ptr::eq(v.as_ptr(), webview.as_ptr())
            } else {
                false
            }
        });
        let removed = initial_len - views.len();
        if removed > 0 {
            log::debug!(
                "Unregistered {} WebView(s), remaining: {}",
                removed,
                views.len()
            );
        }
    });
}

/// Signal all CSS template timers to stop AND immediately terminate all WebViews.
/// IMPORTANT: Must be called from the GTK main thread!
pub fn shutdown_all() {
    SHUTDOWN_FLAG.store(true, Ordering::SeqCst);
    log::info!("WebKit backend shutdown: terminating all WebViews immediately");

    ACTIVE_WEBVIEWS.with(|views| {
        let mut views = views.borrow_mut();
        let count = views.len();
        for weak in views.drain(..) {
            if let Some(webview) = weak.upgrade() {
                log::debug!("Terminating WebView web process");
                webview.stop_loading();
                webview.terminate_web_process();
            }
        }
        if count > 0 {
            log::info!("Terminated {} WebView web process(es)", count);
        }
    });
}

/// Static callback for evaluate_javascript that explicitly drops the result.
fn js_callback_ignore(result: Result<webkit6::javascriptcore::Value, glib::Error>) {
    drop(result);
}

/// State shared between the timer and the widget, allowing WebView replacement
struct WebViewState {
    /// The current WebView (can be replaced)
    webview: Option<WebView>,
    /// The overlay containing the WebView
    overlay: Overlay,
    /// Cancellable for JavaScript calls
    js_cancellable: gtk4::gio::Cancellable,
    /// Last HTML modification time for hot-reload
    last_html_modified: Option<std::time::SystemTime>,
    /// Last CSS modification time for hot-reload
    last_css_modified: Option<std::time::SystemTime>,
}

/// WebKit-based backend for CSS Template rendering
pub struct WebKitBackend;

impl WebKitBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WebKitBackend {
    fn default() -> Self {
        Self::new()
    }
}

/// Create and configure a new WebView
fn create_webview() -> WebView {
    let webview = WebView::new();

    // Set cache model to DocumentViewer for minimal caching
    if let Some(context) = webview.web_context() {
        context.set_cache_model(CacheModel::DocumentViewer);
    }

    // Configure WebView settings to minimize memory usage
    if let Some(settings) = WebViewExt::settings(&webview) {
        settings.set_enable_javascript(true);
        settings.set_allow_file_access_from_file_urls(true);
        settings.set_allow_universal_access_from_file_urls(true);
        settings.set_enable_developer_extras(false);
        settings.set_enable_page_cache(false);
        settings.set_enable_html5_database(false);
        settings.set_enable_html5_local_storage(false);
        settings.set_enable_offline_web_application_cache(false);
        settings.set_enable_media(false);
        settings.set_enable_webaudio(false);
        settings.set_enable_webgl(false);
    }

    // Set transparent background
    webview.set_background_color(&gtk4::gdk::RGBA::new(0.0, 0.0, 0.0, 0.0));

    // Disable WebView's own context menu
    webview.connect_context_menu(|_, _, _| true);

    // Register for shutdown tracking
    register_webview(&webview);

    webview
}

/// Destroy a WebView completely
fn destroy_webview(webview: &WebView) {
    log::info!("Destroying WebView completely to release memory");

    // Unregister from tracking
    unregister_webview(webview);

    // Stop any loading
    webview.stop_loading();

    // Try to trigger JavaScript GC
    webview.evaluate_javascript(
        "if(typeof gc==='function')gc();window.updateValues=null;",
        None,
        None,
        None::<&gtk4::gio::Cancellable>,
        js_callback_ignore,
    );

    // Terminate the web process
    webview.terminate_web_process();
}

impl TemplateBackend for WebKitBackend {
    fn create_widget(&self, data: Arc<Mutex<DisplayData>>) -> Widget {
        // Create the overlay that will hold the WebView
        let overlay = Overlay::new();
        overlay.set_hexpand(true);
        overlay.set_vexpand(true);

        // Create event layer for catching drag/right-click events
        let event_layer = DrawingArea::new();
        event_layer.set_hexpand(true);
        event_layer.set_vexpand(true);
        event_layer.set_draw_func(|_, _, _, _| {});

        // Create initial WebView
        let webview = create_webview();

        // Load initial template
        let (html, base_uri) = {
            let data_guard = data.lock().ok();
            let html = data_guard
                .as_ref()
                .and_then(|d| Self::load_template(&d.config));
            let base_uri = data_guard
                .as_ref()
                .and_then(|d| Self::get_base_uri(&d.config));
            (html, base_uri)
        };

        if let Some(html_content) = html.clone() {
            webview.load_html(&html_content, base_uri.as_deref());
            if let Ok(mut data_guard) = data.lock() {
                data_guard.cached_html = Some(html_content);
            }
        }

        // Set up the overlay with WebView and event layer
        overlay.set_child(Some(&webview));
        overlay.add_overlay(&event_layer);

        // Initialize modification times for hot-reload
        let (last_html_modified, last_css_modified) = {
            let mut html_mod = None;
            let mut css_mod = None;
            if let Ok(data_guard) = data.lock() {
                if data_guard.config.hot_reload {
                    if let Ok(metadata) = std::fs::metadata(&data_guard.config.html_path) {
                        html_mod = metadata.modified().ok();
                    }
                    if let Some(ref css_path) = data_guard.config.css_path {
                        if let Ok(metadata) = std::fs::metadata(css_path) {
                            css_mod = metadata.modified().ok();
                        }
                    }
                }
            }
            (html_mod, css_mod)
        };

        // Create shared state that allows WebView replacement
        let state = Rc::new(RefCell::new(WebViewState {
            webview: Some(webview),
            overlay: overlay.clone(),
            js_cancellable: gtk4::gio::Cancellable::new(),
            last_html_modified,
            last_css_modified,
        }));

        // Set up periodic timer for updates and WebView recycling
        glib::timeout_add_local(Duration::from_millis(1000), {
            let data_clone = data.clone();
            let state_clone = state.clone();
            let event_layer_clone = event_layer.clone();
            move || {
                // Check global shutdown flag
                if SHUTDOWN_FLAG.load(Ordering::SeqCst) {
                    log::debug!("WebKit backend timer stopping: shutdown signal received");
                    let mut state = state_clone.borrow_mut();
                    state.js_cancellable.cancel();
                    if let Some(ref webview) = state.webview {
                        destroy_webview(webview);
                    }
                    state.webview = None;
                    if let Ok(mut data) = data_clone.try_lock() {
                        clear_display_data(&mut data);
                    }
                    return glib::ControlFlow::Break;
                }

                let mut state = state_clone.borrow_mut();

                // Check if we have a WebView
                if state.webview.is_none() {
                    log::debug!("WebKit backend timer stopping: no WebView");
                    return glib::ControlFlow::Break;
                }

                // Check if widget is orphaned
                {
                    let webview = state.webview.as_ref().unwrap();
                    if webview.root().is_none() {
                        log::debug!("WebKit backend timer stopping: WebView orphaned");
                        state.js_cancellable.cancel();
                        destroy_webview(webview);
                        state.webview = None;
                        if let Ok(mut data) = data_clone.try_lock() {
                            clear_display_data(&mut data);
                        }
                        return glib::ControlFlow::Break;
                    }

                    // Skip if not visible
                    if !webview.is_mapped() {
                        return glib::ControlFlow::Continue;
                    }
                }

                // Check for config change
                let config_changed = data_clone
                    .try_lock()
                    .ok()
                    .map(|d| d.config_changed)
                    .unwrap_or(false);

                // Check for hot-reload
                let mut files_changed = false;
                if let Ok(data) = data_clone.try_lock() {
                    if data.config.hot_reload {
                        if !data.config.html_path.as_os_str().is_empty() {
                            if let Ok(metadata) = std::fs::metadata(&data.config.html_path) {
                                if let Ok(modified) = metadata.modified() {
                                    if state.last_html_modified != Some(modified) {
                                        state.last_html_modified = Some(modified);
                                        files_changed = true;
                                    }
                                }
                            }
                        }
                        if let Some(ref css_path) = data.config.css_path {
                            if let Ok(metadata) = std::fs::metadata(css_path) {
                                if let Ok(modified) = metadata.modified() {
                                    if state.last_css_modified != Some(modified) {
                                        state.last_css_modified = Some(modified);
                                        files_changed = true;
                                    }
                                }
                            }
                        }
                    }
                }

                // Reload if files changed or config changed
                if files_changed || config_changed {
                    if let Some((html, base_uri)) = load_html_content(&data_clone) {
                        if let Some(ref webview) = state.webview {
                            webview.load_html(&html, base_uri.as_deref());
                        }
                        if let Ok(mut data) = data_clone.try_lock() {
                            data.cached_html = Some(html);
                            data.config_changed = false;
                        }
                    }
                }

                // Update values via JavaScript
                if let Ok(mut data) = data_clone.try_lock() {
                    if data.dirty {
                        data.dirty = false;

                        let js_values = build_js_values(&data);

                        if js_values != data.last_js_values {
                            data.last_js_values = js_values.clone();
                            data.js_update_count += 1;

                            // Every 300 updates (~5 minutes), completely destroy and recreate WebView
                            if data.js_update_count % 300 == 0 {
                                log::info!(
                                    "WebKit backend: recycling WebView after {} updates to release memory",
                                    data.js_update_count
                                );

                                // Clear buffers
                                let values_len = data.values.len();
                                data.values = HashMap::with_capacity(values_len);
                                data.entries_buffer = Vec::with_capacity(64);
                                data.js_values_buffer = String::with_capacity(1024);
                                data.value_buffer = String::with_capacity(64);
                                data.key_buffer = String::with_capacity(64);
                                data.js_call_buffer = String::with_capacity(2048);
                                data.last_js_values.clear();

                                let cached_html = data.cached_html.clone();
                                let base_uri = if !data.config.html_path.as_os_str().is_empty() {
                                    data.config
                                        .html_path
                                        .parent()
                                        .map(|p| format!("file://{}/", p.display()))
                                } else {
                                    None
                                };

                                // Release the data lock before GTK operations
                                drop(data);

                                // Cancel pending JS calls
                                state.js_cancellable.cancel();

                                // Destroy old WebView completely
                                if let Some(ref old_webview) = state.webview {
                                    // Remove from overlay first
                                    state.overlay.set_child(None::<&Widget>);
                                    destroy_webview(old_webview);
                                }

                                // Create fresh WebView
                                let new_webview = create_webview();

                                // Load cached HTML into new WebView
                                if let Some(ref html) = cached_html {
                                    new_webview.load_html(html, base_uri.as_deref());
                                }

                                // Put new WebView in overlay
                                state.overlay.set_child(Some(&new_webview));
                                // Re-add event layer on top
                                state.overlay.add_overlay(&event_layer_clone);

                                // Update state
                                state.webview = Some(new_webview);
                                state.js_cancellable = gtk4::gio::Cancellable::new();

                                return glib::ControlFlow::Continue;
                            }

                            // Normal JS update
                            if let Some(ref webview) = state.webview {
                                let js_call = format!(
                                    "if (window.updateValues) {{ window.updateValues({{{}}}); }}",
                                    js_values
                                );
                                webview.evaluate_javascript(
                                    &js_call,
                                    None,
                                    None,
                                    Some(&state.js_cancellable),
                                    js_callback_ignore,
                                );
                            }
                        }
                    }
                }

                glib::ControlFlow::Continue
            }
        });

        // Wrap in a Box so we return a stable widget reference
        let container = GtkBox::new(Orientation::Vertical, 0);
        container.append(&overlay);
        container.upcast()
    }
}

/// Build JavaScript values string from display data
fn build_js_values(data: &DisplayData) -> String {
    let mut entries = Vec::with_capacity(data.config.mappings.len());

    for mapping in &data.config.mappings {
        let mut value = String::new();
        write_mapped_value(&data.values, mapping, &mut value);

        // Escape for JavaScript
        let escaped: String = value
            .chars()
            .flat_map(|c| match c {
                '\\' => vec!['\\', '\\'],
                '"' => vec!['\\', '"'],
                _ => vec![c],
            })
            .collect();

        entries.push(format!("\"{}\": \"{}\"", mapping.index, escaped));
    }

    entries.sort();
    entries.join(", ")
}

/// Write mapped value to output
fn write_mapped_value(
    values: &HashMap<String, Value>,
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

    if let Some(value) = values.get(&mapping.slot_prefix) {
        write_formatted_value(value, mapping.format.as_deref(), output);
        return;
    }

    output.push_str("--");
}

/// Write formatted JSON value
fn write_formatted_value(value: &Value, format: Option<&str>, output: &mut String) {
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

/// Load HTML content from config
fn load_html_content(data: &Arc<Mutex<DisplayData>>) -> Option<(String, Option<String>)> {
    let data_guard = data.try_lock().ok()?;

    let base_uri = if !data_guard.config.html_path.as_os_str().is_empty()
        && data_guard.config.html_path.exists()
    {
        data_guard
            .config
            .html_path
            .parent()
            .map(|p| format!("file://{}/", p.display()))
    } else {
        None
    };

    let html = if !data_guard.config.html_path.as_os_str().is_empty()
        && data_guard.config.html_path.exists()
    {
        fs::read_to_string(&data_guard.config.html_path).ok()
    } else {
        data_guard.config.embedded_html.clone()
    };

    let css = data_guard.config.css_path.as_ref().and_then(|p| {
        if p.exists() {
            fs::read_to_string(p).ok()
        } else {
            None
        }
    });

    html.map(|h| {
        let transformed = transform_template(&h);
        (
            prepare_html_document(
                &transformed,
                css.as_deref(),
                data_guard.config.embedded_css.as_deref(),
            ),
            base_uri,
        )
    })
}

/// Clear all buffers in DisplayData to release memory
fn clear_display_data(data: &mut DisplayData) {
    data.values.clear();
    data.values.shrink_to_fit();
    data.cached_html = None;
    data.last_js_values = String::new();
    data.entries_buffer = Vec::new();
    data.js_values_buffer = String::new();
    data.value_buffer = String::new();
    data.key_buffer = String::new();
    data.js_call_buffer = String::new();
    data.cached_prefix_set.clear();
    data.cached_prefix_set.shrink_to_fit();
}
