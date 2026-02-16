//! Global animation manager for coordinating displayer animations.
//!
//! Instead of each animated displayer creating its own `glib::timeout_add_local()` timer,
//! this module provides a single global timer that processes all registered animation callbacks.
//! This reduces overhead from N timers to 1 timer, where N is the number of animated panels.
//!
//! Uses a single repeating timeout at DEFAULT priority for reliable animation delivery.
//! GTK4's `queue_draw()` already synchronizes with the display's frame clock for actual
//! rendering, so timeout-based tick scheduling provides equivalent visual quality without
//! the stall risks of frame clock callbacks (which silently die when the Wayland compositor
//! stops sending frame events).
//!
//! The manager uses adaptive frame rate: full frame rate when animations are active,
//! ~4fps when idle to save CPU (by skipping frames within the repeating timer callback).

use gtk4::glib;
use gtk4::prelude::*;
use gtk4::DrawingArea;
use std::cell::{Cell, RefCell};
use std::time::{Duration, Instant};

use crate::core::ANIMATION_FRAME_INTERVAL;

/// Time without active animations before switching to idle mode (1 second)
const IDLE_TIME_THRESHOLD: Duration = Duration::from_millis(1000);

/// In idle mode, process 1 out of every N timer ticks.
/// At 60fps (16ms interval), skipping 14/15 gives ~4fps effective rate.
const IDLE_SKIP_RATIO: u32 = 15;

thread_local! {
    /// Thread-local animation manager. Since GTK operations must happen on the main thread,
    /// we use thread-local storage instead of a global static.
    static ANIMATION_MANAGER: AnimationManager = AnimationManager::new();
}

/// Initialize the global animation manager. Call this once at startup.
pub fn init_global_animation_manager() {
    // Thread-local storage is lazily initialized on first access.
    ANIMATION_MANAGER.with(|_manager| {});
}

/// Access the global animation manager and perform an operation.
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

/// Shutdown the animation manager by clearing all entries.
/// This breaks reference cycles and allows clean app exit.
pub fn shutdown_animation_manager() {
    with_animation_manager(|manager| {
        manager.shutdown();
    });
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
/// Uses a single repeating timer (ControlFlow::Continue) so there's no chain to break.
/// Adaptive frame rate is handled by skipping ticks in idle mode.
struct AnimationManager {
    /// Registered animation entries.
    entries: RefCell<Vec<AnimationEntry>>,
    /// Whether the repeating timer is currently active.
    timer_active: Cell<bool>,
    /// Timestamp of the last active redraw (for time-based idle detection).
    last_active_time: Cell<Option<Instant>>,
    /// Whether we're currently in idle mode (skipping frames).
    in_idle_mode: Cell<bool>,
    /// Counter for idle frame skipping.
    idle_skip_counter: Cell<u32>,
}

impl AnimationManager {
    /// Create a new animation manager.
    fn new() -> Self {
        Self {
            entries: RefCell::new(Vec::new()),
            timer_active: Cell::new(false),
            last_active_time: Cell::new(None),
            in_idle_mode: Cell::new(false),
            idle_skip_counter: Cell::new(0),
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

        // Reset idle state when new animation is registered
        self.last_active_time.set(Some(Instant::now()));
        self.in_idle_mode.set(false);

        // Start the repeating timer if not already running
        self.ensure_timer_running();
    }

    /// Ensure the repeating animation timer is running.
    fn ensure_timer_running(&self) {
        if self.timer_active.get() {
            return;
        }

        self.timer_active.set(true);

        // Single repeating timer at DEFAULT priority. Uses ControlFlow::Continue so
        // GTK keeps it alive — no chain to break. Stops only when entries become empty.
        glib::source::timeout_add_local_full(
            ANIMATION_FRAME_INTERVAL,
            glib::Priority::DEFAULT,
            move || {
                with_animation_manager(|manager| {
                    // Stop if no entries remain
                    if manager.entries.borrow().is_empty() {
                        manager.timer_active.set(false);
                        return glib::ControlFlow::Break;
                    }

                    // In idle mode, skip most ticks to save CPU
                    if manager.in_idle_mode.get() {
                        let count = manager.idle_skip_counter.get().wrapping_add(1);
                        manager.idle_skip_counter.set(count);
                        if count % IDLE_SKIP_RATIO != 0 {
                            return glib::ControlFlow::Continue;
                        }
                    }

                    let any_active = manager.tick();
                    let now = Instant::now();

                    // Track activity for idle mode transitions
                    if any_active {
                        manager.last_active_time.set(Some(now));
                        if manager.in_idle_mode.get() {
                            manager.in_idle_mode.set(false);
                            log::debug!(
                                "Animation manager exiting idle mode (activity detected)"
                            );
                        }
                    } else if let Some(last_active) = manager.last_active_time.get() {
                        if now.duration_since(last_active) >= IDLE_TIME_THRESHOLD
                            && !manager.in_idle_mode.get()
                        {
                            manager.in_idle_mode.set(true);
                            log::debug!(
                                "Animation manager entering idle mode (no activity for {:?})",
                                IDLE_TIME_THRESHOLD
                            );
                        }
                    } else if !manager.in_idle_mode.get() {
                        manager.in_idle_mode.set(true);
                    }

                    glib::ControlFlow::Continue
                })
            },
        );
    }

    /// Process one animation frame for all registered widgets.
    /// Returns true if any animation needed a redraw.
    fn tick(&self) -> bool {
        let mut entries = self.entries.borrow_mut();
        let mut any_active = false;
        let mut active_count = 0;
        let mut mapped_count = 0;

        // Use retain to process entries and remove dead ones in a single pass
        entries.retain(|entry| {
            // Check if widget still exists
            let Some(widget) = entry.widget_weak.upgrade() else {
                // Widget destroyed, remove entry
                return false;
            };

            // Remove entry if widget has been orphaned (removed from widget tree)
            // This is critical for preventing memory leaks when displayers are changed -
            // the old widget may still have a parent (e.g., an overlay) but that parent
            // is no longer attached to any window. We check for root() being None to
            // detect widgets that are truly disconnected from the display.
            // Without this check, the tick_fn closure holds Arc references indefinitely.
            if widget.root().is_none() {
                log::debug!("Removing animation entry for orphaned widget (no root)");
                return false;
            }

            // Skip if widget is not visible (saves CPU)
            if !widget.is_mapped() {
                return true; // Keep entry, just skip this frame
            }
            mapped_count += 1;

            // Call the tick function
            let needs_redraw = (entry.tick_fn)();

            // Queue redraw if needed
            if needs_redraw {
                widget.queue_draw();
                any_active = true;
                active_count += 1;
            }

            true // Keep entry
        });

        // Log periodically to debug high CPU and memory issues
        static TICK_COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let count = TICK_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if count.is_multiple_of(300) {
            // Log every ~5 seconds at 60fps
            log::info!(
                "Animation manager: {} entries, {} mapped, {} active",
                entries.len(),
                mapped_count,
                active_count
            );
        }
        // More frequent trace logging for debugging drawing issues
        if count.is_multiple_of(60) {
            // Log every ~1 second at 60fps
            log::trace!(
                "Animation tick {}: {} entries, {} mapped, {} active",
                count,
                entries.len(),
                mapped_count,
                active_count
            );
        }

        any_active
    }

    /// Get the number of currently registered animations.
    fn entry_count(&self) -> usize {
        self.entries.borrow().len()
    }

    /// Shutdown the animation manager by clearing all entries.
    /// This stops the timer on next callback (since entries is empty → Break).
    fn shutdown(&self) {
        log::info!(
            "Animation manager shutdown: clearing {} entries",
            self.entries.borrow().len()
        );
        self.entries.borrow_mut().clear();
    }
}
