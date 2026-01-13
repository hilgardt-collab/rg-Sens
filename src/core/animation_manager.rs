//! Global animation manager for coordinating displayer animations.
//!
//! Instead of each animated displayer creating its own `glib::timeout_add_local()` timer,
//! this module provides a single global timer that processes all registered animation callbacks.
//! This reduces overhead from N timers to 1 timer, where N is the number of animated panels.
//!
//! The manager uses adaptive frame rate: 60fps when animations are active, ~10fps when idle.
//! This dramatically reduces CPU usage when nothing is animating.

use gtk4::glib;
use gtk4::prelude::*;
use gtk4::DrawingArea;
use std::cell::{Cell, RefCell};
use std::time::{Duration, Instant};

use crate::core::ANIMATION_FRAME_INTERVAL;

/// Idle polling interval when no animations are active (250ms = ~4fps)
/// This should be >= update_manager interval to avoid constant polling
const IDLE_FRAME_INTERVAL: Duration = Duration::from_millis(250);

/// Time without active animations before switching to idle mode (1 second)
/// This is time-based rather than frame-based to work correctly at any frame rate
const IDLE_TIME_THRESHOLD: Duration = Duration::from_millis(1000);

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
/// Uses adaptive frame rate: fast (60fps) when animating, slow (~4fps) when idle.
struct AnimationManager {
    /// Registered animation entries.
    entries: RefCell<Vec<AnimationEntry>>,
    /// Whether the global timer is currently active.
    timer_active: Cell<bool>,
    /// Timestamp of the last active redraw (for time-based idle detection).
    last_active_time: Cell<Option<Instant>>,
    /// Whether we're currently in idle mode (slow polling).
    in_idle_mode: Cell<bool>,
    /// Count of consecutive frames with active redraws.
    /// Only sustained activity (2+ frames) keeps us out of idle mode.
    consecutive_active_frames: Cell<u32>,
}

impl AnimationManager {
    /// Create a new animation manager.
    const fn new() -> Self {
        Self {
            entries: RefCell::new(Vec::new()),
            timer_active: Cell::new(false),
            last_active_time: Cell::new(None),
            in_idle_mode: Cell::new(false),
            consecutive_active_frames: Cell::new(0),
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

        // Start the timer if not already running
        self.ensure_timer_running();
    }

    /// Ensure the global animation timer is running.
    fn ensure_timer_running(&self) {
        if self.timer_active.get() {
            return;
        }

        self.timer_active.set(true);
        self.schedule_next_tick();
    }

    /// Schedule the next tick with appropriate interval based on idle state.
    fn schedule_next_tick(&self) {
        let interval = if self.in_idle_mode.get() {
            IDLE_FRAME_INTERVAL
        } else {
            ANIMATION_FRAME_INTERVAL
        };

        glib::timeout_add_local_once(interval, move || {
            with_animation_manager(|manager| {
                let any_active = manager.tick();
                let now = Instant::now();

                // Track consecutive active frames to distinguish sustained animation
                // from single-frame dirty redraws
                if any_active {
                    let consecutive = manager.consecutive_active_frames.get() + 1;
                    manager.consecutive_active_frames.set(consecutive);

                    // Only treat as "real" animation if we have 2+ consecutive active frames
                    // Single-frame activity is likely just a dirty redraw from data update
                    if consecutive >= 2 {
                        manager.last_active_time.set(Some(now));
                        if manager.in_idle_mode.get() {
                            manager.in_idle_mode.set(false);
                            log::debug!("Animation manager exiting idle mode (sustained animation detected)");
                        }
                    }
                } else {
                    manager.consecutive_active_frames.set(0);

                    // Switch to idle mode if no sustained activity for IDLE_TIME_THRESHOLD
                    if let Some(last_active) = manager.last_active_time.get() {
                        if now.duration_since(last_active) >= IDLE_TIME_THRESHOLD && !manager.in_idle_mode.get() {
                            manager.in_idle_mode.set(true);
                            log::debug!("Animation manager entering idle mode (no activity for {:?})", IDLE_TIME_THRESHOLD);
                        }
                    } else {
                        // No activity ever recorded, enter idle mode immediately
                        if !manager.in_idle_mode.get() {
                            manager.in_idle_mode.set(true);
                        }
                    }
                }

                // Continue if we have entries
                if !manager.entries.borrow().is_empty() {
                    manager.schedule_next_tick();
                } else {
                    manager.timer_active.set(false);
                }
            });
        });
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
            // the old widget is removed from its parent but GTK may keep it alive briefly.
            // Without this check, the tick_fn closure holds Arc references indefinitely.
            if widget.parent().is_none() {
                log::debug!("Removing animation entry for orphaned widget");
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

        // Log periodically to debug high CPU
        static TICK_COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let count = TICK_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if count % 60 == 0 {
            log::debug!("Animation tick #{}: {} entries, {} mapped, {} active, idle_mode={}",
                count, entries.len(), mapped_count, active_count, self.in_idle_mode.get());
        }

        any_active
    }

    /// Get the number of currently registered animations.
    fn entry_count(&self) -> usize {
        self.entries.borrow().len()
    }
}
