//! WebKit Backend for CSS Template Displayer
//!
//! This module implements the CSS Template rendering using WebKitGTK.
//! It provides full HTML/CSS/JavaScript support with a WebView widget.

use gtk4::{glib, prelude::*, DrawingArea, Overlay, Widget};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
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
        std::cell::RefCell::new(Vec::new());
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

/// Unregister a WebView (called when timer detects orphan/shutdown)
fn unregister_webview(webview: &WebView) {
    ACTIVE_WEBVIEWS.with(|views| {
        let mut views = views.borrow_mut();
        // Remove entries that match this webview or are already dead
        let initial_len = views.len();
        views.retain(|weak| {
            if let Some(v) = weak.upgrade() {
                // Keep if it's a different webview that's still alive
                !std::ptr::eq(v.as_ptr(), webview.as_ptr())
            } else {
                // Remove dead references
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
/// This is called on app shutdown and directly cleans up WebKit processes
/// instead of waiting for timers to notice the shutdown flag.
///
/// IMPORTANT: Must be called from the GTK main thread!
pub fn shutdown_all() {
    SHUTDOWN_FLAG.store(true, Ordering::SeqCst);
    log::info!("WebKit backend shutdown: terminating all WebViews immediately");

    // Directly terminate all registered WebViews
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
/// Using a static function avoids closure allocation on each call.
fn js_callback_ignore(result: Result<webkit6::javascriptcore::Value, glib::Error>) {
    drop(result);
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

impl TemplateBackend for WebKitBackend {
    fn create_widget(&self, data: Arc<Mutex<DisplayData>>) -> Widget {
        // Create WebView
        let webview = WebView::new();

        // Set cache model to DocumentViewer for minimal caching (reduces memory accumulation)
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

        // Disable WebView's own context menu (let parent handle right-click)
        webview.connect_context_menu(|_, _, _| true);

        // Register WebView for shutdown tracking
        register_webview(&webview);

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

        if let Some(html_content) = html {
            webview.load_html(&html_content, base_uri.as_deref());
            if let Ok(mut data_guard) = data.lock() {
                data_guard.cached_html = Some(html_content);
            }
        }

        // Track file modification times for hot-reload
        let last_html_modified: Arc<Mutex<Option<std::time::SystemTime>>> =
            Arc::new(Mutex::new(None));
        let last_css_modified: Arc<Mutex<Option<std::time::SystemTime>>> =
            Arc::new(Mutex::new(None));

        // Initialize modification times
        if let Ok(data_guard) = data.lock() {
            if data_guard.config.hot_reload {
                if let Ok(metadata) = std::fs::metadata(&data_guard.config.html_path) {
                    if let Ok(modified) = metadata.modified() {
                        *last_html_modified.lock().unwrap() = Some(modified);
                    }
                }
                if let Some(ref css_path) = data_guard.config.css_path {
                    if let Ok(metadata) = std::fs::metadata(css_path) {
                        if let Ok(modified) = metadata.modified() {
                            *last_css_modified.lock().unwrap() = Some(modified);
                        }
                    }
                }
            }
        }

        // Create a cancellable for JavaScript calls
        let js_cancellable: Arc<Mutex<gtk4::gio::Cancellable>> =
            Arc::new(Mutex::new(gtk4::gio::Cancellable::new()));

        // Set up periodic check for reload and value updates
        glib::timeout_add_local(Duration::from_millis(1000), {
            let data_clone = data.clone();
            let webview_weak = webview.downgrade();
            let last_html_modified = last_html_modified.clone();
            let last_css_modified = last_css_modified.clone();
            let js_cancellable = js_cancellable.clone();
            move || {
                // Check global shutdown flag first
                if SHUTDOWN_FLAG.load(Ordering::SeqCst) {
                    log::debug!("WebKit backend timer stopping: shutdown signal received");

                    if let Ok(cancellable) = js_cancellable.lock() {
                        cancellable.cancel();
                    }

                    if let Ok(mut data) = data_clone.try_lock() {
                        clear_display_data(&mut data);
                    }

                    if let Some(webview) = webview_weak.upgrade() {
                        unregister_webview(&webview);
                        webview.stop_loading();
                        webview.terminate_web_process();
                    }
                    return glib::ControlFlow::Break;
                }

                let Some(webview) = webview_weak.upgrade() else {
                    log::debug!("WebKit backend timer stopping: WebView destroyed");
                    return glib::ControlFlow::Break;
                };

                // Check if widget is orphaned
                if webview.root().is_none() {
                    log::debug!("WebKit backend timer stopping: WebView orphaned (no root)");

                    if let Ok(cancellable) = js_cancellable.lock() {
                        cancellable.cancel();
                    }

                    if let Ok(mut data) = data_clone.try_lock() {
                        clear_display_data(&mut data);
                    }

                    unregister_webview(&webview);
                    webview.stop_loading();
                    webview.terminate_web_process();
                    return glib::ControlFlow::Break;
                }

                // Skip if not visible
                if !webview.is_mapped() {
                    return glib::ControlFlow::Continue;
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
                        let html_path = &data.config.html_path;
                        let css_path = &data.config.css_path;

                        if !html_path.as_os_str().is_empty() {
                            if let Ok(metadata) = std::fs::metadata(html_path) {
                                if let Ok(modified) = metadata.modified() {
                                    if let Ok(mut last) = last_html_modified.try_lock() {
                                        if *last != Some(modified) {
                                            *last = Some(modified);
                                            files_changed = true;
                                        }
                                    }
                                }
                            }
                        }
                        if let Some(ref css_path) = css_path {
                            if let Ok(metadata) = std::fs::metadata(css_path) {
                                if let Ok(modified) = metadata.modified() {
                                    if let Ok(mut last) = last_css_modified.try_lock() {
                                        if *last != Some(modified) {
                                            *last = Some(modified);
                                            files_changed = true;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Reload if files changed or config changed
                if files_changed || config_changed {
                    let html_content = {
                        let mut should_reload = config_changed;
                        let config = data_clone.try_lock().ok().map(|mut d| {
                            if d.config_changed {
                                d.config_changed = false;
                                should_reload = true;
                            }
                            d.config.clone()
                        });

                        if let Some(config) = config {
                            if should_reload || config.hot_reload {
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
                        if let Ok(mut data) = data_clone.try_lock() {
                            data.cached_html = Some(html);
                        }
                    }
                }

                // Update values via JavaScript
                if let Ok(mut data) = data_clone.try_lock() {
                    if data.dirty {
                        data.dirty = false;

                        let mut key_buffer = std::mem::take(&mut data.key_buffer);
                        let mut entries = std::mem::take(&mut data.entries_buffer);
                        let mut value_buffer = std::mem::take(&mut data.value_buffer);

                        let mappings_len = data.config.mappings.len();
                        for (i, mapping) in data.config.mappings.iter().enumerate() {
                            value_buffer.clear();
                            write_mapped_value_to_buffer(
                                &data.values,
                                mapping,
                                &mut key_buffer,
                                &mut value_buffer,
                            );

                            if i < entries.len() {
                                entries[i].clear();
                            } else {
                                entries.push(String::with_capacity(32));
                            }
                            use std::fmt::Write;
                            entries[i].push('"');
                            let _ = write!(entries[i], "{}", mapping.index);
                            entries[i].push_str("\": \"");
                            for c in value_buffer.chars() {
                                match c {
                                    '\\' => entries[i].push_str("\\\\"),
                                    '"' => entries[i].push_str("\\\""),
                                    _ => entries[i].push(c),
                                }
                            }
                            entries[i].push('"');
                        }
                        entries.truncate(mappings_len);
                        entries.sort();

                        let mut js_values_buffer = std::mem::take(&mut data.js_values_buffer);
                        js_values_buffer.clear();
                        for (i, entry) in entries.iter().enumerate() {
                            if i > 0 {
                                js_values_buffer.push_str(", ");
                            }
                            js_values_buffer.push_str(entry);
                        }

                        data.key_buffer = key_buffer;
                        data.entries_buffer = entries;
                        data.value_buffer = value_buffer;

                        if js_values_buffer != data.last_js_values {
                            std::mem::swap(&mut data.last_js_values, &mut js_values_buffer);
                            data.js_values_buffer = js_values_buffer;
                            data.js_update_count += 1;

                            // Every 300 updates, fully reload WebView to combat memory leak
                            if data.js_update_count % 300 == 0 {
                                log::debug!(
                                    "WebKit backend: periodic WebView reset to release memory"
                                );

                                let values_len = data.values.len();
                                data.values = HashMap::with_capacity(values_len);
                                let entries_len = data.entries_buffer.len();
                                data.entries_buffer = Vec::with_capacity(entries_len.min(64));
                                data.js_values_buffer = String::with_capacity(1024);
                                data.value_buffer = String::with_capacity(64);
                                data.key_buffer = String::with_capacity(64);
                                data.js_call_buffer = String::with_capacity(2048);

                                if let Some(ref html) = data.cached_html {
                                    let html_clone = html.clone();
                                    let base_uri =
                                        if !data.config.html_path.as_os_str().is_empty() {
                                            data.config
                                                .html_path
                                                .parent()
                                                .map(|p| format!("file://{}/", p.display()))
                                        } else {
                                            None
                                        };
                                    data.last_js_values.clear();
                                    drop(data);

                                    if let Ok(cancellable) = js_cancellable.lock() {
                                        cancellable.cancel();
                                    }

                                    webview.evaluate_javascript(
                                        "if(typeof gc==='function')gc();window.updateValues=null;",
                                        None,
                                        None,
                                        None::<&gtk4::gio::Cancellable>,
                                        js_callback_ignore,
                                    );

                                    webview.stop_loading();
                                    webview.terminate_web_process();

                                    if let Ok(mut cancellable) = js_cancellable.lock() {
                                        *cancellable = gtk4::gio::Cancellable::new();
                                    }

                                    let webview_for_reload = webview.clone();
                                    glib::timeout_add_local_once(
                                        Duration::from_millis(50),
                                        move || {
                                            webview_for_reload
                                                .load_html(&html_clone, base_uri.as_deref());
                                        },
                                    );
                                    return glib::ControlFlow::Continue;
                                }
                            }

                            let mut js_call_buffer = std::mem::take(&mut data.js_call_buffer);
                            js_call_buffer.clear();
                            js_call_buffer
                                .push_str("if (window.updateValues) { window.updateValues({");
                            js_call_buffer.push_str(&data.last_js_values);
                            js_call_buffer.push_str("}); }");

                            let cancellable_guard = js_cancellable.lock().ok();
                            webview.evaluate_javascript(
                                &js_call_buffer,
                                None,
                                None,
                                cancellable_guard.as_deref(),
                                js_callback_ignore,
                            );
                            drop(cancellable_guard);

                            data.js_call_buffer = js_call_buffer;
                        } else {
                            data.js_values_buffer = js_values_buffer;
                        }
                    }
                }

                glib::ControlFlow::Continue
            }
        });

        // Wrap WebView in an Overlay with a transparent event-catching layer
        let overlay = Overlay::new();
        overlay.set_child(Some(&webview));

        let event_layer = DrawingArea::new();
        event_layer.set_hexpand(true);
        event_layer.set_vexpand(true);
        event_layer.set_draw_func(|_, _, _, _| {});
        overlay.add_overlay(&event_layer);

        overlay.upcast()
    }
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

/// Helper to write mapped value into output buffer using reusable key buffer
fn write_mapped_value_to_buffer(
    values: &HashMap<String, Value>,
    mapping: &PlaceholderMapping,
    key_buffer: &mut String,
    output: &mut String,
) {
    if mapping.slot_prefix.is_empty() {
        output.push_str("--");
        return;
    }

    key_buffer.clear();
    key_buffer.push_str(&mapping.slot_prefix);
    key_buffer.push('_');
    key_buffer.push_str(&mapping.field);

    if let Some(value) = values.get(key_buffer.as_str()) {
        write_formatted_value_to_buffer(value, mapping.format.as_deref(), output);
        return;
    }

    if let Some(value) = values.get(&mapping.slot_prefix) {
        write_formatted_value_to_buffer(value, mapping.format.as_deref(), output);
        return;
    }

    output.push_str("--");
}

/// Helper to write formatted JSON value into output buffer
#[inline]
fn write_formatted_value_to_buffer(value: &Value, format: Option<&str>, output: &mut String) {
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
