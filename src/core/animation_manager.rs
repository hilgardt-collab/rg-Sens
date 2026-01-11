//! Global animation manager for coordinating displayer animations.
//!
//! Instead of each animated displayer creating its own `glib::timeout_add_local()` timer,
//! this module provides a single global timer that processes all registered animation callbacks.
//! This reduces overhead from N timers to 1 timer, where N is the number of animated panels.

use gtk4::glib;
use gtk4::prelude::*;
use gtk4::DrawingArea;
use std::cell::{Cell, RefCell};

use crate::core::ANIMATION_FRAME_INTERVAL;

thread_local! {
    /// Thread-local animation manager. Since GTK operations must happen on the main thread,
    /// we use thread-local storage instead of a global static.
    static ANIMATION_MANAGER: AnimationManager = const { AnimationManager::new() };
}

/// Initialize the global animation manager. Call this once at startup.
/// (Currently a no-op since thread_local! auto-initializes, but kept for API consistency.)
pub fn init_global_animation_manager() {
    // Thread-local storage is lazily initialized on first access.
    // This function exists for API consistency with other managers.
    ANIMATION_MANAGER.with(|_| {});
}

/// Access the global animation manager and perform an operation.
/// Returns None if called from a thread without an initialized manager.
fn with_animation_manager<F, R>(f: F) -> R
where
    F: FnOnce(&AnimationManager) -> R,
{
    ANIMATION_MANAGER.with(f)
}

/// Register an animated widget with its tick callback.
///
/// The tick function is called every animation frame (~60fps) and should return
/// `true` if the widget needs a redraw, `false` otherwise.
///
/// The widget is automatically unregistered when it is destroyed (detected via
/// the weak reference).
///
/// # Arguments
///
/// * `widget_weak` - Weak reference to the DrawingArea widget
/// * `tick_fn` - Closure that performs animation tick and returns whether redraw is needed
///
/// # Example
///
/// ```ignore
/// use crate::core::animation_manager::register_animation;
///
/// let data = self.data.clone();
/// register_animation(drawing_area.downgrade(), move || {
///     if let Ok(mut d) = data.try_lock() {
///         // Animation logic here...
///         needs_redraw
///     } else {
///         false
///     }
/// });
/// ```
pub fn register_animation<F>(widget_weak: glib::WeakRef<DrawingArea>, tick_fn: F)
where
    F: Fn() -> bool + 'static,
{
    with_animation_manager(|manager| {
        manager.register(widget_weak, tick_fn);
    });
}

/// Get the number of currently registered animations (for debugging/monitoring).
#[allow(dead_code)]
pub fn animation_entry_count() -> usize {
    with_animation_manager(|manager| manager.entry_count())
}

/// Entry in the animation registry.
struct AnimationEntry {
    /// Weak reference to the widget - allows detecting when widget is destroyed.
    widget_weak: glib::WeakRef<DrawingArea>,
    /// Animation tick function. Returns true if the widget needs a redraw.
    tick_fn: Box<dyn Fn() -> bool>,
}

/// Manages a single global animation timer that processes all registered animation callbacks.
///
/// This is stored in thread-local storage since GTK operations must happen on the main thread.
struct AnimationManager {
    /// Registered animation entries.
    entries: RefCell<Vec<AnimationEntry>>,
    /// Whether the global timer is currently active.
    timer_active: Cell<bool>,
}

impl AnimationManager {
    /// Create a new animation manager.
    const fn new() -> Self {
        Self {
            entries: RefCell::new(Vec::new()),
            timer_active: Cell::new(false),
        }
    }

    /// Register an animated widget with its tick callback.
    fn register<F>(&self, widget_weak: glib::WeakRef<DrawingArea>, tick_fn: F)
    where
        F: Fn() -> bool + 'static,
    {
        self.entries.borrow_mut().push(AnimationEntry {
            widget_weak,
            tick_fn: Box::new(tick_fn),
        });

        // Start the timer if not already running
        self.ensure_timer_running();
    }

    /// Ensure the global animation timer is running.
    fn ensure_timer_running(&self) {
        if self.timer_active.get() {
            return;
        }

        self.timer_active.set(true);

        glib::timeout_add_local(ANIMATION_FRAME_INTERVAL, move || {
            let should_continue = with_animation_manager(|manager| {
                manager.tick();
                !manager.entries.borrow().is_empty()
            });

            if should_continue {
                glib::ControlFlow::Continue
            } else {
                with_animation_manager(|manager| {
                    manager.timer_active.set(false);
                });
                glib::ControlFlow::Break
            }
        });
    }

    /// Process one animation frame for all registered widgets.
    fn tick(&self) {
        let mut entries = self.entries.borrow_mut();

        // Use retain to process entries and remove dead ones in a single pass
        entries.retain(|entry| {
            // Check if widget still exists
            let Some(widget) = entry.widget_weak.upgrade() else {
                // Widget destroyed, remove entry
                return false;
            };

            // Skip if widget is not visible (saves CPU)
            if !widget.is_mapped() {
                return true; // Keep entry, just skip this frame
            }

            // Call the tick function
            let needs_redraw = (entry.tick_fn)();

            // Queue redraw if needed
            if needs_redraw {
                widget.queue_draw();
            }

            true // Keep entry
        });
    }

    /// Get the number of currently registered animations.
    fn entry_count(&self) -> usize {
        self.entries.borrow().len()
    }
}
