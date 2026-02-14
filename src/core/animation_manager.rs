//! Global animation manager for coordinating displayer animations.
//!
//! Instead of each animated displayer creating its own `glib::timeout_add_local()` timer,
//! this module provides a single global timer that processes all registered animation callbacks.
//! This reduces overhead from N timers to 1 timer, where N is the number of animated panels.
//!
//! The manager uses GTK's frame clock for synchronization with the display's refresh rate.
//! This ensures queue_draw() calls happen at the right time in the rendering pipeline,
//! avoiding issues with Vulkan swapchain synchronization.
//!
//! The manager uses adaptive frame rate: full frame rate when animations are active,
//! ~4fps when idle to save CPU.
//!
//! A watchdog timer runs every 2 seconds to detect and recover from silently dead
//! frame clock callbacks (GTK removes tick callbacks without notification when their
//! widget is destroyed or unmapped).

use gtk4::glib;
use gtk4::prelude::*;
use gtk4::DrawingArea;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, Instant};

use crate::core::ANIMATION_FRAME_INTERVAL;

/// Idle polling interval when no animations are active (250ms = ~4fps)
/// This should be >= update_manager interval to avoid constant polling
const IDLE_FRAME_INTERVAL: Duration = Duration::from_millis(250);

/// Time without active animations before switching to idle mode (1 second)
/// This is time-based rather than frame-based to work correctly at any frame rate
const IDLE_TIME_THRESHOLD: Duration = Duration::from_millis(1000);

/// Watchdog check interval - how often we verify the tick mechanism is alive
const WATCHDOG_INTERVAL: Duration = Duration::from_secs(2);

/// Watchdog staleness threshold - if no tick has occurred for this long, restart
/// Must be > IDLE_FRAME_INTERVAL to avoid false positives during idle mode
const WATCHDOG_STALE_THRESHOLD: Duration = Duration::from_secs(3);

thread_local! {
    /// Thread-local animation manager. Since GTK operations must happen on the main thread,
    /// we use thread-local storage instead of a global static.
    static ANIMATION_MANAGER: AnimationManager = AnimationManager::new();
}

/// Initialize the global animation manager. Call this once at startup.
/// Starts the watchdog timer that monitors tick mechanism health.
pub fn init_global_animation_manager() {
    // Thread-local storage is lazily initialized on first access.
    ANIMATION_MANAGER.with(|manager| {
        manager.start_watchdog();
    });
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

/// Check if the animation tick mechanism appears stalled.
/// Returns Some(elapsed) if stalled, None if healthy or no entries exist.
pub fn check_animation_stall() -> Option<Duration> {
    with_animation_manager(|manager| manager.check_stall())
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
    /// Whether we're using frame clock tick callback
    using_frame_clock: Cell<bool>,
    /// Flag to signal the frame clock callback to stop
    stop_frame_clock: Rc<Cell<bool>>,
    /// Generation counter for frame clock callbacks.
    /// Incremented each time a new callback is registered. Old callbacks detect
    /// a generation mismatch and return Break, preventing callback accumulation.
    frame_clock_generation: Rc<Cell<u64>>,
    /// Reference widget for frame clock (first registered widget that's still valid)
    reference_widget: RefCell<Option<glib::WeakRef<DrawingArea>>>,
    /// Timestamp of the last successful tick() call - used by watchdog to detect stalls.
    /// Updated every time tick() runs, regardless of whether any animations were active.
    last_tick_time: Cell<Option<Instant>>,
    /// Whether the watchdog timer has been started.
    watchdog_active: Cell<bool>,
}

impl AnimationManager {
    /// Create a new animation manager.
    fn new() -> Self {
        Self {
            entries: RefCell::new(Vec::new()),
            timer_active: Cell::new(false),
            last_active_time: Cell::new(None),
            in_idle_mode: Cell::new(false),
            using_frame_clock: Cell::new(false),
            stop_frame_clock: Rc::new(Cell::new(false)),
            frame_clock_generation: Rc::new(Cell::new(0)),
            reference_widget: RefCell::new(None),
            last_tick_time: Cell::new(None),
            watchdog_active: Cell::new(false),
        }
    }

    /// Register an animated widget with its tick callback.
    fn register<F>(&self, widget_weak: glib::WeakRef<DrawingArea>, tick_fn: F)
    where
        F: Fn() -> bool + 'static,
    {
        // If this is the first entry, use this widget as the reference for frame clock
        if self.reference_widget.borrow().is_none() {
            *self.reference_widget.borrow_mut() = Some(widget_weak.clone());
        }

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

    /// Check if the reference widget is still valid (mapped with a root).
    fn is_reference_widget_valid(&self) -> bool {
        self.reference_widget
            .borrow()
            .as_ref()
            .and_then(|w| w.upgrade())
            .map(|w| w.is_mapped() && w.root().is_some())
            .unwrap_or(false)
    }

    /// Ensure the global animation timer is running.
    fn ensure_timer_running(&self) {
        if self.timer_active.get() {
            // If using frame clock, verify the reference widget is still alive.
            // GTK silently removes tick callbacks when their widget is destroyed/unmapped,
            // which leaves us thinking the frame clock is active when it's actually dead.
            if self.using_frame_clock.get() && !self.is_reference_widget_valid() {
                log::warn!(
                    "Animation manager: frame clock reference widget lost, restarting tick mechanism"
                );
                self.stop_frame_clock();
                // Fall through to restart
            } else {
                return;
            }
        }

        self.timer_active.set(true);
        self.schedule_next_tick();
    }

    /// Try to use frame clock tick callback for synchronization
    fn try_use_frame_clock(&self) -> bool {
        // Check if we already have a tick callback
        if self.using_frame_clock.get() {
            // Verify the reference widget is still valid. If the widget was destroyed
            // or unmapped, GTK silently removed the tick callback - we must re-attach.
            if self.is_reference_widget_valid() {
                return true;
            }
            log::warn!(
                "Animation manager: reference widget invalid while frame clock claimed active, re-attaching"
            );
            self.using_frame_clock.set(false);
            self.stop_frame_clock.set(true);
            // Fall through to find a new widget and attach a new callback
        }

        // Try to get a valid reference widget
        let reference_widget = self.reference_widget.borrow();
        let widget = match reference_widget.as_ref().and_then(|w| w.upgrade()) {
            Some(w) if w.is_mapped() && w.root().is_some() => w,
            _ => {
                // Try to find a new reference widget from entries
                drop(reference_widget);
                if let Some(new_ref) = self.find_valid_reference_widget() {
                    *self.reference_widget.borrow_mut() = Some(new_ref.downgrade());
                    new_ref
                } else {
                    return false;
                }
            }
        };

        // Reset the stop flag and increment generation counter.
        // The generation counter prevents callback accumulation: if a new callback
        // is registered before the old one checks the stop flag, the old callback
        // will detect the generation mismatch and return Break.
        self.stop_frame_clock.set(false);
        let stop_flag = self.stop_frame_clock.clone();

        let new_generation = self.frame_clock_generation.get() + 1;
        self.frame_clock_generation.set(new_generation);
        let generation = self.frame_clock_generation.clone();
        let my_generation = new_generation;

        // Set up tick callback on the widget
        // This synchronizes with GTK's frame clock (VSync)
        widget.add_tick_callback(move |_widget, _frame_clock| {
            // Check if we should stop
            if stop_flag.get() {
                return glib::ControlFlow::Break;
            }

            // Check if this callback is stale (a newer one has been registered).
            // This prevents the race where: old callback sets stop_flag=true,
            // new registration resets stop_flag=false, old callback never dies.
            if generation.get() != my_generation {
                log::debug!(
                    "Animation manager: removing stale frame clock callback (gen {} vs current {})",
                    my_generation,
                    generation.get()
                );
                return glib::ControlFlow::Break;
            }

            with_animation_manager(|manager| {
                manager.process_frame_clock_tick();
            });
            glib::ControlFlow::Continue
        });

        self.using_frame_clock.set(true);
        log::debug!("Animation manager: attached to frame clock");
        true
    }

    /// Find a valid widget to use as reference for frame clock
    fn find_valid_reference_widget(&self) -> Option<DrawingArea> {
        let entries = self.entries.borrow();
        for entry in entries.iter() {
            if let Some(widget) = entry.widget_weak.upgrade() {
                if widget.is_mapped() && widget.root().is_some() {
                    return Some(widget);
                }
            }
        }
        None
    }

    /// Process tick from frame clock (synchronized with display refresh)
    fn process_frame_clock_tick(&self) {
        // In idle mode, skip most frames to save CPU
        if self.in_idle_mode.get() {
            static IDLE_SKIP_COUNTER: std::sync::atomic::AtomicU32 =
                std::sync::atomic::AtomicU32::new(0);
            let count = IDLE_SKIP_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            // At 60fps, skip 14 out of 15 frames (~4fps effective rate)
            if !count.is_multiple_of(15) {
                // Still update last_tick_time so the watchdog knows we're alive
                self.last_tick_time.set(Some(Instant::now()));
                return;
            }
        }

        let any_active = self.tick();
        let now = Instant::now();

        // Track activity for idle mode transitions
        if any_active {
            // Any redraw activity resets the idle timer and exits idle mode
            self.last_active_time.set(Some(now));
            if self.in_idle_mode.get() {
                self.in_idle_mode.set(false);
                log::debug!("Animation manager exiting idle mode (activity detected)");
            }
        } else {
            // Switch to idle mode if no activity for IDLE_TIME_THRESHOLD
            if let Some(last_active) = self.last_active_time.get() {
                if now.duration_since(last_active) >= IDLE_TIME_THRESHOLD
                    && !self.in_idle_mode.get()
                {
                    self.in_idle_mode.set(true);
                    log::debug!(
                        "Animation manager entering idle mode (no activity for {:?})",
                        IDLE_TIME_THRESHOLD
                    );
                }
            } else if !self.in_idle_mode.get() {
                self.in_idle_mode.set(true);
            }
        }

        // Check if we should stop
        if self.entries.borrow().is_empty() {
            self.stop_frame_clock();
            self.timer_active.set(false);
        } else if !self.is_reference_widget_valid() {
            // Entries still exist but the reference widget is gone (destroyed/orphaned).
            // tick() may have removed the reference widget's entry while other entries remain.
            // We're still inside the dying widget's callback, so we have one last chance to
            // re-attach to a new widget's frame clock before GTK stops calling us.
            log::warn!(
                "Animation manager: reference widget lost while {} entries remain, re-attaching",
                self.entries.borrow().len()
            );
            self.stop_frame_clock();
            self.schedule_next_tick();
        }
    }

    /// Stop using frame clock
    fn stop_frame_clock(&self) {
        if self.using_frame_clock.get() {
            // Signal the tick callback to stop on next invocation
            self.stop_frame_clock.set(true);
            self.using_frame_clock.set(false);
        }
    }

    /// Schedule the next tick with appropriate interval based on idle state.
    fn schedule_next_tick(&self) {
        // Try to use frame clock first (synchronized with display)
        if self.try_use_frame_clock() {
            return; // Frame clock will handle ticks
        }

        // Fall back to timeout-based ticking if no widget available yet
        let interval = if self.in_idle_mode.get() {
            IDLE_FRAME_INTERVAL
        } else {
            ANIMATION_FRAME_INTERVAL
        };

        // Use DEFAULT_IDLE priority so user input events (mouse, keyboard) are always
        // processed before animation ticks. This prevents animations from starving the
        // event loop and keeps the UI responsive during user interaction.
        glib::source::timeout_add_local_full(interval, glib::Priority::DEFAULT_IDLE, move || {
            with_animation_manager(|manager| {
                // Try again to attach to frame clock
                if manager.try_use_frame_clock() {
                    return;
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
                } else {
                    // Switch to idle mode if no activity for IDLE_TIME_THRESHOLD
                    if let Some(last_active) = manager.last_active_time.get() {
                        if now.duration_since(last_active) >= IDLE_TIME_THRESHOLD
                            && !manager.in_idle_mode.get()
                        {
                            manager.in_idle_mode.set(true);
                            log::debug!(
                                "Animation manager entering idle mode (no activity for {:?})",
                                IDLE_TIME_THRESHOLD
                            );
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
            // Fire once, then re-schedule via schedule_next_tick() above
            glib::ControlFlow::Break
        });
    }

    /// Process one animation frame for all registered widgets.
    /// Returns true if any animation needed a redraw.
    fn tick(&self) -> bool {
        // Record tick time for watchdog monitoring
        self.last_tick_time.set(Some(Instant::now()));

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
    /// This stops the timer (since empty entries means no rescheduling)
    /// and breaks any reference cycles held by tick_fn closures.
    fn shutdown(&self) {
        log::info!(
            "Animation manager shutdown: clearing {} entries",
            self.entries.borrow().len()
        );
        self.stop_frame_clock();
        self.entries.borrow_mut().clear();
        *self.reference_widget.borrow_mut() = None;
        self.last_tick_time.set(None);
        // The timer will stop on next tick since entries is empty
    }

    /// Start the watchdog timer that periodically checks if the tick mechanism is alive.
    /// This runs independently of the frame clock and can detect/recover from a silently
    /// dead frame clock callback.
    fn start_watchdog(&self) {
        if self.watchdog_active.get() {
            return;
        }
        self.watchdog_active.set(true);

        glib::timeout_add_local(WATCHDOG_INTERVAL, move || {
            with_animation_manager(|manager| {
                let entry_count = manager.entries.borrow().len();
                if entry_count == 0 {
                    // No entries, nothing to watch
                    return;
                }

                // Check if ticks have stalled
                if let Some(last_tick) = manager.last_tick_time.get() {
                    let elapsed = Instant::now().duration_since(last_tick);
                    if elapsed >= WATCHDOG_STALE_THRESHOLD {
                        log::warn!(
                            "Animation watchdog: tick mechanism stalled for {:.1}s with {} entries, restarting",
                            elapsed.as_secs_f64(),
                            entry_count
                        );
                        manager.restart_tick_mechanism();
                    }
                } else if manager.timer_active.get() {
                    // Timer claims to be active but no tick has ever run - may be stuck
                    log::warn!(
                        "Animation watchdog: timer_active=true but no tick recorded with {} entries, restarting",
                        entry_count
                    );
                    manager.restart_tick_mechanism();
                }
            });
            glib::ControlFlow::Continue
        });
    }

    /// Forcibly restart the tick mechanism.
    /// Called by the watchdog when it detects the frame clock callback has silently died.
    fn restart_tick_mechanism(&self) {
        // Reset all tick state
        self.stop_frame_clock();
        self.timer_active.set(false);
        self.in_idle_mode.set(false);
        self.last_active_time.set(Some(Instant::now()));

        // Try to find a new valid reference widget
        if let Some(new_ref) = self.find_valid_reference_widget() {
            *self.reference_widget.borrow_mut() = Some(new_ref.downgrade());
        }

        // Restart
        self.ensure_timer_running();
        log::info!("Animation watchdog: tick mechanism restarted successfully");
    }

    /// Check if the tick mechanism appears stalled.
    /// Returns Some(elapsed) if stalled, None if healthy or no entries.
    fn check_stall(&self) -> Option<Duration> {
        if self.entries.borrow().is_empty() {
            return None;
        }
        if let Some(last_tick) = self.last_tick_time.get() {
            let elapsed = Instant::now().duration_since(last_tick);
            if elapsed >= WATCHDOG_STALE_THRESHOLD {
                Some(elapsed)
            } else {
                None
            }
        } else if self.timer_active.get() {
            // Active but never ticked
            Some(Duration::from_secs(0))
        } else {
            None
        }
    }
}
