//! WebKit Backend for CSS Template Displayer
//!
//! This module implements the CSS Template rendering using WebKitGTK.
//! It provides full HTML/CSS/JavaScript support with a WebView widget.
//!
//! Every 5 minutes, the WebView is completely destroyed and recreated
//! to prevent memory accumulation from WebKitGTK internals.

use gtk4::{glib, prelude::*, Box as GtkBox, DrawingArea, Orientation, Overlay, Widget};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use webkit6::prelude::WebViewExt;
use webkit6::{CacheModel, WebView};

use crate::displayers::css_template_backend::{DisplayData, TemplateBackend};
use crate::ui::css_template_display::{build_document, write_mapped_value};

/// Global shutdown flag - when set, all CSS template timers will stop
static SHUTDOWN_FLAG: AtomicBool = AtomicBool::new(false);

/// Diagnostics for the leak harness (`examples/webkit_leak.rs`): how many
/// JavaScript updates were sent, how many WebViews were recycled, and how many
/// backend timers are alive.
pub static EVAL_COUNT: AtomicU64 = AtomicU64::new(0);
pub static RECYCLE_COUNT: AtomicU64 = AtomicU64::new(0);
pub static LIVE_TIMERS: AtomicI64 = AtomicI64::new(0);

/// WebViews that are still alive (not finalized), whether or not they are
/// still in a window.
pub fn live_webviews() -> usize {
    ACTIVE_WEBVIEWS.with(|views| {
        views
            .borrow()
            .iter()
            .filter(|weak| weak.upgrade().is_some())
            .count()
    })
}

/// Memory limit handed to WebKit for each web process (MB, 0 = WebKit's own
/// default, which is a fraction of system RAM). WebKit starts releasing
/// caches at a third of the limit and does so aggressively at half.
const DEFAULT_MEMORY_LIMIT_MB: u64 = 192;

/// Value updates between WebView recycles (destroy and recreate); 0 never
/// recycles. Off by default: see where it is read.
const DEFAULT_RECYCLE_EVERY: u64 = 0;

/// Test-only override of a tuning constant through the environment.
fn env_override(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// Number of consecutive timer ticks a WebView must stay orphaned (no root)
/// before its web process is terminated. Widgets can be transiently unparented
/// (displayer swap, panel re-parenting), so terminating on the first orphan
/// tick would permanently kill a panel that still exists.
const ORPHAN_GRACE_TICKS: u32 = 5;

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
    /// Consecutive ticks the WebView has been orphaned (no root)
    orphan_ticks: u32,
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
/// Send the current values again once a page has finished loading.
///
/// A value update that reaches the page before its script has run is lost, and
/// unchanged values are not sent twice, so a page whose values did not change
/// after its first tick (or after a recycle or hot reload) stayed on "--".
fn resend_values_after_load(webview: &WebView, data: &Arc<Mutex<DisplayData>>) {
    let data = data.clone();
    webview.connect_load_changed(move |_, event| {
        if event == webkit6::LoadEvent::Finished {
            if let Ok(mut data) = data.lock() {
                // Not the empty string: a page with nothing mapped has an
                // empty payload and would never get its first call.
                data.last_js_values = "\u{0}".to_string();
                data.dirty = true;
            }
        }
    });
}

fn create_webview() -> WebView {
    let limit_mb = env_override("RG_SENS_WEBKIT_MEMLIMIT_MB", DEFAULT_MEMORY_LIMIT_MB) as u32;
    let webview = if limit_mb > 0 {
        thread_local! {
            static CONTEXT: RefCell<Option<webkit6::WebContext>> = const { RefCell::new(None) };
        }
        let context = CONTEXT.with(|c| {
            c.borrow_mut()
                .get_or_insert_with(|| {
                    let mut settings = webkit6::MemoryPressureSettings::new();
                    settings.set_memory_limit(limit_mb);
                    settings.set_poll_interval(5.0);
                    webkit6::WebContext::builder()
                        .memory_pressure_settings(&settings)
                        .build()
                })
                .clone()
        });
        WebView::builder().web_context(&context).build()
    } else {
        WebView::new()
    };

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
        resend_values_after_load(&webview, &data);

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
            orphan_ticks: 0,
        }));

        // Set up the periodic timer for updates. Recycling the WebView every
        // N value updates was a mitigation for a web-process memory leak that
        // has since been fixed; it is off (0) and only the environment turns
        // it back on. A recycle throws the page away, and with it whatever
        // the page keeps itself (a template's history graphs).
        let tick_ms = env_override("RG_SENS_WEBKIT_TICK_MS", 1000);
        let recycle_every =
            env_override("RG_SENS_WEBKIT_RECYCLE_EVERY", DEFAULT_RECYCLE_EVERY) as u32;
        LIVE_TIMERS.fetch_add(1, Ordering::Relaxed);
        let tick = {
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

                // Check if widget is orphaned - use pattern matching for safety
                let (is_orphaned, is_mapped) = match state.webview.as_ref() {
                    Some(webview) => (webview.root().is_none(), webview.is_mapped()),
                    None => {
                        // webview was None despite earlier check - shouldn't happen but handle gracefully
                        log::debug!("WebKit backend timer stopping: WebView disappeared");
                        return glib::ControlFlow::Break;
                    }
                };

                if is_orphaned {
                    // Only terminate after the widget has stayed orphaned for
                    // several consecutive ticks — transient reparents (displayer
                    // swap, panel drag) must not kill the web process.
                    state.orphan_ticks += 1;
                    if state.orphan_ticks < ORPHAN_GRACE_TICKS {
                        return glib::ControlFlow::Continue;
                    }
                    log::debug!("WebKit backend timer stopping: WebView orphaned");
                    state.js_cancellable.cancel();
                    if let Some(webview) = state.webview.take() {
                        destroy_webview(&webview);
                    }
                    if let Ok(mut data) = data_clone.try_lock() {
                        clear_display_data(&mut data);
                    }
                    return glib::ControlFlow::Break;
                }
                state.orphan_ticks = 0;

                // Skip if not visible
                if !is_mapped {
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
                        let animation = (
                            data.config.animation_enabled,
                            data.config.animation_speed.clamp(0.1, 10.0),
                        );

                        if js_values != data.last_js_values {
                            data.last_js_values = js_values.clone();
                            data.js_update_count += 1;

                            // When recycling is switched on: every `recycle_every`
                            // updates, completely destroy and recreate the WebView
                            if recycle_every > 0 && data.js_update_count % recycle_every == 0 {
                                RECYCLE_COUNT.fetch_add(1, Ordering::Relaxed);
                                log::info!(
                                    "WebKit backend: recycling WebView after {} updates to release memory",
                                    data.js_update_count
                                );

                                // Clear buffers
                                let values_len = data.values.len();
                                data.values = HashMap::with_capacity(values_len);
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

                                // Create new WebView FIRST (before touching the old one)
                                let new_webview = create_webview();
                                resend_values_after_load(&new_webview, &data_clone);

                                // Load cached HTML into new WebView before it's visible
                                if let Some(ref html) = cached_html {
                                    new_webview.load_html(html, base_uri.as_deref());
                                }

                                // Capture old webview for deferred destruction
                                let old_webview = state.webview.take();

                                // Update state with new webview
                                state.webview = Some(new_webview.clone());
                                state.js_cancellable = gtk4::gio::Cancellable::new();

                                // Defer the actual widget swap to an idle callback
                                // This ensures we're not in the middle of a frame render
                                let overlay = state.overlay.clone();
                                let event_layer = event_layer_clone.clone();
                                glib::idle_add_local_once(move || {
                                    // Atomically swap: setting new child removes the old one
                                    overlay.set_child(Some(&new_webview));
                                    // `set_child` replaces only the MAIN child, so the event
                                    // layer is still parented to this overlay. Remove before
                                    // re-adding, otherwise add_overlay re-parents an already-
                                    // parented widget (GTK assertion on every recycle).
                                    overlay.remove_overlay(&event_layer);
                                    overlay.add_overlay(&event_layer);

                                    // Now destroy the old WebView (it's already detached)
                                    if let Some(old_wv) = old_webview {
                                        destroy_webview(&old_wv);
                                    }
                                });

                                return glib::ControlFlow::Continue;
                            }

                            // Normal JS update
                            if let Some(ref webview) = state.webview {
                                EVAL_COUNT.fetch_add(1, Ordering::Relaxed);
                                let js_call = format!(
                                    "if (window.rgSetMotion) {{ window.rgSetMotion({}, {}); }} \
                                     if (window.updateValues) {{ window.updateValues({{{}}}); }}",
                                    animation.0, animation.1, js_values
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
        };
        glib::timeout_add_local(Duration::from_millis(tick_ms), move || {
            let flow = tick();
            if flow == glib::ControlFlow::Break {
                LIVE_TIMERS.fetch_sub(1, Ordering::Relaxed);
            }
            flow
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

        // Serialize via serde_json: unlike hand-rolled escaping this handles
        // newlines, CR, control chars, etc. (one multi-line value would
        // otherwise syntax-error the whole updateValues payload). JSON string
        // output is valid JS for all these cases.
        let quoted = serde_json::to_string(&value).unwrap_or_else(|_| "\"--\"".to_string());
        entries.push(format!("\"{}\": {}", mapping.index, quoted));
    }

    entries.sort();
    entries.join(", ")
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
        (
            build_document(&h, &data_guard.config, css.as_deref()),
            base_uri,
        )
    })
}

/// Clear all buffers in DisplayData to release memory.
///
/// NOTE: `cached_prefix_set` is intentionally NOT cleared — it is only populated
/// in `DisplayData::default()` and never rebuilt, and the same DisplayData Arc is
/// reused if the widget is recreated. Clearing it would filter out every value
/// forever ("--" until restart).
fn clear_display_data(data: &mut DisplayData) {
    data.values.clear();
    data.values.shrink_to_fit();
    data.cached_html = None;
    data.last_js_values = String::new();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::css_template_display::PlaceholderMapping;
    use crate::ui::css_template_display::{detect_placeholders, extract_placeholder_defaults};
    use serde_json::Value;

    fn mapped(values: &HashMap<String, Value>, field: &str) -> String {
        let mapping = PlaceholderMapping {
            index: 0,
            slot_prefix: "group1_12".to_string(),
            field: field.to_string(),
            format: Some("{:.0}".to_string()),
        };
        let mut out = String::new();
        write_mapped_value(values, &mapping, &mut out);
        out
    }

    #[test]
    fn max_and_min_read_the_slot_limits() {
        let mut values = HashMap::new();
        values.insert("group1_12_value".to_string(), Value::from(1240.4));
        values.insert("group1_12_max_limit".to_string(), Value::from(2400.0));
        assert_eq!(mapped(&values, "value"), "1240");
        assert_eq!(mapped(&values, "max"), "2400");
        assert_eq!(mapped(&values, "min"), "--");
    }

    /// The shipped Art Deco, Art Nouveau, Steampunk, Neon, LCARS-style, Warrior, Leather & Wood, LCD, Victorian, Mughal Peacock, Bauhaus, Gothic, Western and Japandi examples: every placeholder they
    /// use has an auto-configure entry, the numbering has no holes, the two
    /// share one mapping, and none loads anything from the network.
    #[test]
    fn example_panels_are_fully_configured() {
        let deco = include_str!("../../examples/art_deco_panel.html");
        let nouveau = include_str!("../../examples/art_nouveau_panel.html");
        let steampunk = include_str!("../../examples/steampunk_panel.html");
        for (html, count) in [(deco, 74u32), (nouveau, 74), (steampunk, 74)] {
            let placeholders = detect_placeholders(html);
            assert_eq!(placeholders, (0..count).collect::<Vec<u32>>());
            let defaults = extract_placeholder_defaults(html);
            for index in &placeholders {
                assert!(
                    defaults.contains_key(index),
                    "placeholder {index} has no default"
                );
            }
            assert!(!html.contains("fonts.googleapis.com"));
        }
        // Neon writes every value from its script (no `{{N}}` in its markup);
        // its config block alone names the same 74 placeholders.
        let neon = include_str!("../../examples/neon_panel.html");
        assert!(detect_placeholders(neon).is_empty());
        assert!(!neon.contains("fonts.googleapis.com"));
        let deco = extract_placeholder_defaults(deco);
        assert_eq!(extract_placeholder_defaults(neon).len(), deco.len());
        // LCARS adds per-core load (74-89) and the year (90) to the shared 74.
        let lcars = include_str!("../../examples/lcars_panel.html");
        assert!(detect_placeholders(lcars).is_empty());
        assert!(!lcars.contains("fonts.googleapis.com"));
        assert_eq!(extract_placeholder_defaults(lcars).len(), 91);
        // Warrior keeps the shared 74 and, like Neon, writes them from script.
        let warrior = include_str!("../../examples/warrior_panel.html");
        assert!(detect_placeholders(warrior).is_empty());
        assert!(!warrior.contains("fonts.googleapis.com"));
        assert_eq!(extract_placeholder_defaults(warrior).len(), deco.len());
        let leather = include_str!("../../examples/leather_wood_panel.html");
        assert!(detect_placeholders(leather).is_empty());
        assert!(!leather.contains("fonts.googleapis.com"));
        assert_eq!(extract_placeholder_defaults(leather).len(), deco.len());
        // LCD adds per-core load (74-89, as LCARS) and the alarm indicator
        // (91; 90 is the year on LCARS, and a kept binding would light ALM).
        let lcd = include_str!("../../examples/lcd_panel.html");
        assert!(detect_placeholders(lcd).is_empty());
        assert!(!lcd.contains("fonts.googleapis.com"));
        let lcd_defaults = extract_placeholder_defaults(lcd);
        assert_eq!(lcd_defaults.len(), 91);
        assert!(!lcd_defaults.contains_key(&90));
        // Victorian adds the year, on 90 as LCARS has it.
        let victorian = include_str!("../../examples/victorian_panel.html");
        assert!(detect_placeholders(victorian).is_empty());
        assert!(!victorian.contains("fonts.googleapis.com"));
        let victorian_defaults = extract_placeholder_defaults(victorian);
        assert_eq!(victorian_defaults.len(), deco.len() + 1);
        assert_eq!(victorian_defaults[&90].field, "year");
        // Mughal Peacock has the year on 90 too.
        let mughal = include_str!("../../examples/mughal_peacock_panel.html");
        assert!(detect_placeholders(mughal).is_empty());
        assert!(!mughal.contains("fonts.googleapis.com"));
        let mughal_defaults = extract_placeholder_defaults(mughal);
        assert_eq!(mughal_defaults.len(), deco.len() + 1);
        assert_eq!(mughal_defaults[&90].field, "year");
        // So has Bauhaus.
        let bauhaus = include_str!("../../examples/bauhaus_panel.html");
        assert!(detect_placeholders(bauhaus).is_empty());
        assert!(!bauhaus.contains("fonts.googleapis.com"));
        let bauhaus_defaults = extract_placeholder_defaults(bauhaus);
        assert_eq!(bauhaus_defaults.len(), deco.len() + 1);
        assert_eq!(bauhaus_defaults[&90].field, "year");
        // And Gothic.
        let gothic = include_str!("../../examples/gothic_panel.html");
        assert!(detect_placeholders(gothic).is_empty());
        assert!(!gothic.contains("fonts.googleapis.com"));
        let gothic_defaults = extract_placeholder_defaults(gothic);
        assert_eq!(gothic_defaults.len(), deco.len() + 1);
        assert_eq!(gothic_defaults[&90].field, "year");
        // Western keeps to the shared 74.
        let western = include_str!("../../examples/western_panel.html");
        assert!(detect_placeholders(western).is_empty());
        assert!(!western.contains("fonts.googleapis.com"));
        assert_eq!(extract_placeholder_defaults(western).len(), deco.len());
        // Japandi has the year on 90.
        let japandi = include_str!("../../examples/japandi_panel.html");
        assert!(detect_placeholders(japandi).is_empty());
        assert!(!japandi.contains("fonts.googleapis.com"));
        let japandi_defaults = extract_placeholder_defaults(japandi);
        assert_eq!(japandi_defaults.len(), deco.len() + 1);
        assert_eq!(japandi_defaults[&90].field, "year");
        // Glass & Gumdrop and Filigree keep to the shared 74.
        let gumdrop = include_str!("../../examples/gumdrop_panel.html");
        let filigree = include_str!("../../examples/filigree_panel.html");
        for plain in [gumdrop, filigree] {
            assert!(detect_placeholders(plain).is_empty());
            assert!(!plain.contains("fonts.googleapis.com"));
            assert_eq!(extract_placeholder_defaults(plain).len(), deco.len());
        }
        for other in [
            nouveau, steampunk, neon, lcars, warrior, leather, lcd, victorian, mughal, bauhaus,
            gothic, western, japandi, gumdrop, filigree,
        ] {
            let other = extract_placeholder_defaults(other);
            for (index, default) in &deco {
                assert_eq!(
                    (&default.source, &default.field),
                    (&other[index].source, &other[index].field),
                    "placeholder {index} differs between the panels"
                );
            }
        }
    }
}
