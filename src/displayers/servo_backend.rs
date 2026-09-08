//! Servo Backend for CSS Template Displayer (Experimental)
//!
//! This module implements the CSS Template rendering using Servo's embedding API.
//! It provides Rust-native HTML/CSS rendering without WebKit dependencies.
//!
//! ## Current Status
//!
//! This is a skeleton implementation with TODOs for the actual Servo integration.
//! Servo's embedding API is still evolving, so this serves as a placeholder for
//! future implementation once the API stabilizes.
//!
//! ## Architecture
//!
//! Unlike WebKit which provides a native GTK widget, Servo renders to a pixel buffer
//! using SoftwareRenderingContext. We display this buffer in a GTK DrawingArea
//! using Cairo's ImageSurface.
//!
//! ```text
//! ┌─────────────────────────────────────────┐
//! │  Servo WebView                          │
//! │  └─ SoftwareRenderingContext            │
//! │     └─ Renders to pixel buffer (RGBA)   │
//! └─────────────────────────────────────────┘
//!                     ↓
//! ┌─────────────────────────────────────────┐
//! │  GTK4 DrawingArea                       │
//! │  └─ Cairo ImageSurface from buffer      │
//! │     └─ cr.set_source_surface() + paint  │
//! └─────────────────────────────────────────┘
//! ```
//!
//! ## Requirements
//!
//! - Rust nightly toolchain (Servo requires nightly features)
//! - Servo git dependency with specific revision
//! - Servo `resources/` directory copied to app data dir

use gtk4::{glib, prelude::*, DrawingArea, Widget};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, Weak};
use std::time::Duration;

use crate::displayers::css_template_backend::{DisplayData, TemplateBackend};

/// Global shutdown flag - when set, all Servo backend timers will stop
static SHUTDOWN_FLAG: AtomicBool = AtomicBool::new(false);

/// Number of consecutive timer ticks the widget must stay orphaned (no root)
/// before the instance is shut down. Widgets can be transiently unparented
/// (displayer swap, panel re-parenting), so shutting down on the first orphan
/// tick would permanently kill a panel that still exists.
const ORPHAN_GRACE_TICKS: u32 = 5;

// Thread-local registry of active Servo instances for proper shutdown.
// Holds weak references (like the WebKit backend) so the registry never keeps
// destroyed instances alive until app shutdown.
thread_local! {
    static ACTIVE_SERVOS: RefCell<Vec<Weak<Mutex<ServoInstance>>>> =
        const { RefCell::new(Vec::new()) };
}

/// Placeholder for Servo instance state
/// TODO: Replace with actual Servo WebView and rendering context
struct ServoInstance {
    /// Pixel buffer for software rendering (RGBA format)
    pixel_buffer: Vec<u8>,
    /// Width of the render surface
    width: i32,
    /// Height of the render surface
    height: i32,
    /// Flag indicating if the instance is active
    active: bool,
    /// Incremented whenever paint() produces a new frame; lets the draw
    /// function and the update timer detect new content without cloning the
    /// pixel buffer or redrawing unconditionally.
    frame_serial: u64,
    // TODO: Add actual Servo fields:
    // servo: Servo<SoftwareRenderingContext>,
    // webview: WebView,
}

impl ServoInstance {
    fn new() -> Self {
        Self {
            pixel_buffer: Vec::new(),
            width: 0,
            height: 0,
            active: true,
            frame_serial: 0,
        }
    }

    /// Resize the render surface
    fn resize(&mut self, width: i32, height: i32) {
        if width > 0 && height > 0 {
            self.width = width;
            self.height = height;
            // RGBA format: 4 bytes per pixel
            let size = (width * height * 4) as usize;
            self.pixel_buffer.resize(size, 0);
        }
    }

    /// Load HTML content
    /// TODO: Implement actual Servo HTML loading
    fn load_html(&mut self, _html: &str, _base_uri: Option<&str>) {
        log::debug!("Servo backend: load_html called (TODO: implement)");
        // TODO: webview.load_html(html, base_uri);
    }

    /// Evaluate JavaScript to update values
    /// TODO: Implement actual JavaScript evaluation
    fn evaluate_javascript(&mut self, _script: &str) {
        log::debug!("Servo backend: evaluate_javascript called (TODO: implement)");
        // TODO: webview.evaluate_javascript(script);
    }

    /// Trigger a paint and update the pixel buffer
    /// TODO: Implement actual Servo rendering
    fn paint(&mut self) -> bool {
        // TODO: Call servo.paint() and copy rendered pixels to buffer
        // For now, fill with a placeholder color to show the widget is working
        if !self.pixel_buffer.is_empty() {
            // Fill with dark gray to indicate Servo backend (placeholder)
            for chunk in self.pixel_buffer.chunks_exact_mut(4) {
                chunk[0] = 0x40; // B
                chunk[1] = 0x40; // G
                chunk[2] = 0x40; // R
                chunk[3] = 0xFF; // A
            }
            self.frame_serial = self.frame_serial.wrapping_add(1);
            return true;
        }
        false
    }

    /// Shutdown the Servo instance
    fn shutdown(&mut self) {
        self.active = false;
        self.pixel_buffer.clear();
        self.pixel_buffer.shrink_to_fit();
        // TODO: servo.shutdown();
        log::debug!("Servo backend: instance shut down");
    }
}

/// Register a Servo instance for tracking
fn register_servo(instance: &Arc<Mutex<ServoInstance>>) {
    ACTIVE_SERVOS.with(|servos| {
        let mut servos = servos.borrow_mut();
        // Prune entries whose instances are already gone (there is no
        // explicit unregister; weak refs make this cheap)
        servos.retain(|weak| weak.strong_count() > 0);
        servos.push(Arc::downgrade(instance));
        log::debug!(
            "Registered Servo instance for shutdown tracking (total: {})",
            servos.len()
        );
    });
}

/// Signal all Servo backend timers to stop AND shutdown all instances.
///
/// IMPORTANT: Must be called from the GTK main thread!
pub fn shutdown_all() {
    SHUTDOWN_FLAG.store(true, Ordering::SeqCst);
    log::info!("Servo backend shutdown: terminating all instances immediately");

    ACTIVE_SERVOS.with(|servos| {
        let mut servos = servos.borrow_mut();
        let mut count = 0;
        for weak in servos.drain(..) {
            if let Some(instance) = weak.upgrade() {
                if let Ok(mut inst) = instance.lock() {
                    inst.shutdown();
                    count += 1;
                }
            }
        }
        if count > 0 {
            log::info!("Terminated {} Servo instance(s)", count);
        }
    });
}

/// Servo-based backend for CSS Template rendering
pub struct ServoBackend;

impl ServoBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ServoBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl TemplateBackend for ServoBackend {
    fn create_widget(&self, data: Arc<Mutex<DisplayData>>) -> Widget {
        // Create drawing area for displaying Servo's rendered output
        let drawing_area = DrawingArea::new();
        drawing_area.set_hexpand(true);
        drawing_area.set_vexpand(true);

        // Create Servo instance
        let servo_instance = Arc::new(Mutex::new(ServoInstance::new()));
        register_servo(&servo_instance);

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
            if let Ok(mut inst) = servo_instance.lock() {
                inst.load_html(&html_content, base_uri.as_deref());
            }
            if let Ok(mut data_guard) = data.lock() {
                data_guard.cached_html = Some(html_content);
            }
        }

        // Set up draw function to render Servo's pixel buffer via Cairo.
        // The surface is cached per frame_serial so the pixel buffer is copied
        // once per new frame instead of being cloned on every draw.
        let servo_for_draw = servo_instance.clone();
        let surface_cache: Rc<RefCell<Option<(u64, cairo::ImageSurface)>>> =
            Rc::new(RefCell::new(None));
        drawing_area.set_draw_func(move |_, cr, width, height| {
            if let Ok(inst) = servo_for_draw.lock() {
                if inst.pixel_buffer.is_empty() || inst.width <= 0 || inst.height <= 0 {
                    // No content yet, draw placeholder
                    cr.set_source_rgba(0.2, 0.2, 0.2, 1.0);
                    let _ = cr.paint();

                    // Draw "Servo Backend (TODO)" text
                    cr.set_source_rgba(0.6, 0.6, 0.6, 1.0);
                    cr.select_font_face(
                        "sans-serif",
                        cairo::FontSlant::Normal,
                        cairo::FontWeight::Normal,
                    );
                    cr.set_font_size(14.0);
                    let text = "Servo Backend (TODO)";
                    if let Ok(extents) = cr.text_extents(text) {
                        cr.move_to(
                            (width as f64 - extents.width()) / 2.0,
                            (height as f64 + extents.height()) / 2.0,
                        );
                    } else {
                        cr.move_to(width as f64 / 2.0 - 80.0, height as f64 / 2.0);
                    }
                    let _ = cr.show_text(text);
                    return;
                }

                // Rebuild the cached surface only when a new frame was rendered
                // (or the surface size changed) — never per draw.
                let mut cache = surface_cache.borrow_mut();
                let cache_valid = cache.as_ref().is_some_and(|(serial, surface)| {
                    *serial == inst.frame_serial
                        && surface.width() == inst.width
                        && surface.height() == inst.height
                });
                if !cache_valid {
                    // Create Cairo ImageSurface from Servo's pixel buffer
                    // Servo renders in BGRA format, which matches Cairo's
                    // Format::ARgb32 on little-endian
                    match cairo::ImageSurface::create_for_data(
                        inst.pixel_buffer.clone(),
                        cairo::Format::ARgb32,
                        inst.width,
                        inst.height,
                        inst.width * 4, // stride
                    ) {
                        Ok(surface) => *cache = Some((inst.frame_serial, surface)),
                        Err(e) => {
                            log::error!("Servo backend: failed to create surface: {}", e);
                            *cache = None;
                        }
                    }
                }

                match cache.as_ref() {
                    Some((_, surface)) => {
                        // Scale to fit the drawing area if needed
                        let scale_x = width as f64 / inst.width as f64;
                        let scale_y = height as f64 / inst.height as f64;
                        let scale = scale_x.min(scale_y);

                        cr.save().ok();
                        cr.scale(scale, scale);
                        let _ = cr.set_source_surface(surface, 0.0, 0.0);
                        let _ = cr.paint();
                        // Clear source reference to prevent GL texture memory leak
                        cr.set_source_rgba(0.0, 0.0, 0.0, 0.0);
                        cr.restore().ok();
                    }
                    None => {
                        cr.set_source_rgba(0.5, 0.0, 0.0, 1.0);
                        let _ = cr.paint();
                    }
                }
            }
        });

        // Handle resize events
        let servo_for_resize = servo_instance.clone();
        let drawing_area_for_resize = drawing_area.clone();
        drawing_area.connect_resize(move |_, width, height| {
            if let Ok(mut inst) = servo_for_resize.lock() {
                inst.resize(width, height);
                inst.paint();
            }
            drawing_area_for_resize.queue_draw();
        });

        // Set up periodic update timer (matches WebKit's 1000ms interval)
        glib::timeout_add_local(Duration::from_millis(1000), {
            let data_clone = data.clone();
            let servo_clone = servo_instance.clone();
            let drawing_area_weak = drawing_area.downgrade();
            let mut orphan_ticks: u32 = 0;
            let mut last_drawn_serial: u64 = 0;
            move || {
                // Check shutdown flag
                if SHUTDOWN_FLAG.load(Ordering::SeqCst) {
                    log::debug!("Servo backend timer stopping: shutdown signal received");
                    if let Ok(mut inst) = servo_clone.lock() {
                        inst.shutdown();
                    }
                    return glib::ControlFlow::Break;
                }

                let Some(drawing_area) = drawing_area_weak.upgrade() else {
                    log::debug!("Servo backend timer stopping: DrawingArea destroyed");
                    return glib::ControlFlow::Break;
                };

                // Check if widget is orphaned. Only shut down after the widget
                // has stayed orphaned for several consecutive ticks — transient
                // reparents (displayer swap, panel drag) must not kill it.
                if drawing_area.root().is_none() {
                    orphan_ticks += 1;
                    if orphan_ticks < ORPHAN_GRACE_TICKS {
                        return glib::ControlFlow::Continue;
                    }
                    log::debug!("Servo backend timer stopping: DrawingArea orphaned");
                    if let Ok(mut inst) = servo_clone.lock() {
                        inst.shutdown();
                    }
                    if let Ok(mut data) = data_clone.try_lock() {
                        clear_display_data(&mut data);
                    }
                    return glib::ControlFlow::Break;
                }
                orphan_ticks = 0;

                // Skip if not visible
                if !drawing_area.is_mapped() {
                    return glib::ControlFlow::Continue;
                }

                // Check for config change and reload if needed
                let config_changed = data_clone
                    .try_lock()
                    .ok()
                    .map(|d| d.config_changed)
                    .unwrap_or(false);

                if config_changed {
                    if let Ok(mut data) = data_clone.try_lock() {
                        data.config_changed = false;
                        if let Some(html) = Self::load_template(&data.config) {
                            let base_uri = Self::get_base_uri(&data.config);
                            if let Ok(mut inst) = servo_clone.lock() {
                                inst.load_html(&html, base_uri.as_deref());
                            }
                            data.cached_html = Some(html);
                        }
                    }
                }

                // Update values via JavaScript
                if let Ok(mut data) = data_clone.try_lock() {
                    if data.dirty {
                        data.dirty = false;

                        // Build JavaScript call (same format as WebKit backend)
                        // TODO: Actually evaluate this JavaScript in Servo
                        let js_call = build_js_update_call(&data);
                        if !js_call.is_empty() && js_call != data.last_js_values {
                            data.last_js_values = js_call.clone();
                            if let Ok(mut inst) = servo_clone.lock() {
                                inst.evaluate_javascript(&format!(
                                    "if (window.updateValues) {{ window.updateValues({{{}}}); }}",
                                    js_call
                                ));
                                // Trigger repaint after value update
                                inst.paint();
                            }
                        }
                    }
                }

                // Request redraw only when a new frame was actually rendered
                let current_serial = servo_clone
                    .lock()
                    .map(|inst| inst.frame_serial)
                    .unwrap_or(last_drawn_serial);
                if current_serial != last_drawn_serial {
                    last_drawn_serial = current_serial;
                    drawing_area.queue_draw();
                }

                glib::ControlFlow::Continue
            }
        });

        drawing_area.upcast()
    }
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

/// Build the JavaScript update call string from display data
/// Returns the inner object content (without the outer braces)
fn build_js_update_call(data: &DisplayData) -> String {
    use std::fmt::Write;

    let mut entries = Vec::with_capacity(data.config.mappings.len());

    for mapping in &data.config.mappings {
        let mut value = String::new();
        write_mapped_value(&data.values, mapping, &mut value);

        let mut entry = String::new();
        entry.push('"');
        let _ = write!(entry, "{}", mapping.index);
        entry.push_str("\": ");
        // Serialize via serde_json: unlike hand-rolled escaping this handles
        // newlines, CR, control chars, etc. (one multi-line value would
        // otherwise syntax-error the whole updateValues payload). JSON string
        // output is valid JS for all these cases.
        match serde_json::to_string(&value) {
            Ok(quoted) => entry.push_str(&quoted),
            Err(_) => entry.push_str("\"--\""),
        }
        entries.push(entry);
    }

    entries.sort();
    entries.join(", ")
}

/// Write mapped value into output buffer
fn write_mapped_value(
    values: &std::collections::HashMap<String, serde_json::Value>,
    mapping: &crate::ui::css_template_display::PlaceholderMapping,
    output: &mut String,
) {
    if mapping.slot_prefix.is_empty() {
        output.push_str("--");
        return;
    }

    // Build key with field suffix
    let key = format!("{}_{}", mapping.slot_prefix, mapping.field);

    if let Some(value) = values.get(&key) {
        write_value_to_buffer(value, mapping.format.as_deref(), output);
        return;
    }

    if let Some(value) = values.get(&mapping.slot_prefix) {
        write_value_to_buffer(value, mapping.format.as_deref(), output);
        return;
    }

    output.push_str("--");
}

/// Write formatted JSON value into output buffer
fn write_value_to_buffer(value: &serde_json::Value, format: Option<&str>, output: &mut String) {
    use crate::ui::css_template_display::write_format_value_to_buffer;
    use std::fmt::Write;

    match value {
        serde_json::Value::Number(n) => {
            if let Some(f) = n.as_f64() {
                write_format_value_to_buffer(f, format, output);
            } else {
                let _ = write!(output, "{}", n);
            }
        }
        serde_json::Value::String(s) => output.push_str(s),
        serde_json::Value::Bool(b) => output.push_str(if *b { "true" } else { "false" }),
        _ => {
            let _ = write!(output, "{}", value);
        }
    }
}
