//! Auto-Scroll System
//!
//! Provides automatic scrolling functionality for the main window:
//! - Smooth scrolling with easing animation
//! - Page-by-page or continuous scrolling modes
//! - Pattern: right across row, then down to next row, wrap at end

use gtk4::prelude::*;
use gtk4::{DrawingArea, ScrolledWindow};
use std::cell::RefCell;
use std::rc::Rc;

use crate::config::AppConfig;
use crate::ui::GridLayout;

/// Ease-in-out function for smooth animation
fn ease_in_out(t: f64) -> f64 {
    if t < 0.5 {
        2.0 * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
    }
}

/// Schedule auto-scroll animation
///
/// Pattern: scroll right until edge, then move down+left to start of next row, repeat.
/// When at bottom-right, wrap to top-left.
/// Uses a generation counter to prevent overlapping scroll cycles.
fn schedule_auto_scroll(
    scrolled: ScrolledWindow,
    config: Rc<RefCell<AppConfig>>,
    layout: Rc<RefCell<GridLayout>>,
    active: Rc<RefCell<bool>>,
    generation: Rc<RefCell<u32>>,
    current_gen: u32,
    bg: DrawingArea,
) {
    // Check if this is a stale callback from an old generation
    if *generation.borrow() != current_gen {
        return;
    }

    let cfg = config.borrow();
    if !cfg.window.auto_scroll_enabled {
        *active.borrow_mut() = false;
        return;
    }
    let delay_ms = cfg.window.auto_scroll_delay_ms;
    drop(cfg);

    *active.borrow_mut() = true;

    // Schedule the scroll after delay
    gtk4::glib::timeout_add_local_once(std::time::Duration::from_millis(delay_ms), move || {
        // Check generation again - might have been reset while waiting
        if *generation.borrow() != current_gen {
            return;
        }

        let cfg = config.borrow();
        if !cfg.window.auto_scroll_enabled {
            *active.borrow_mut() = false;
            return;
        }
        drop(cfg);

        // Get scroll info
        let h_adj = scrolled.hadjustment();
        let v_adj = scrolled.vadjustment();
        let content_size = layout.borrow().get_content_size();
        let content_width = content_size.0 as f64;
        let content_height = content_size.1 as f64;
        let viewport_width = h_adj.page_size();
        let viewport_height = v_adj.page_size();

        // Check if whole pages mode is enabled
        let cfg = config.borrow();
        let whole_pages = cfg.window.auto_scroll_whole_pages;
        drop(cfg);

        // Calculate effective scroll bounds and container size
        // When whole_pages is enabled, align to complete page boundaries
        let (max_h_scroll, max_v_scroll, container_width, container_height) = if whole_pages && viewport_width > 0.0 && viewport_height > 0.0 {
            // Calculate number of complete pages needed to cover content
            let h_pages = (content_width / viewport_width).ceil() as i32;
            let v_pages = (content_height / viewport_height).ceil() as i32;
            // Max scroll position is (pages - 1) * viewport_size
            let max_h = ((h_pages - 1).max(0) as f64) * viewport_width;
            let max_v = ((v_pages - 1).max(0) as f64) * viewport_height;
            // Container size must be large enough to scroll to all page boundaries
            // Size = pages * viewport_size (so we can scroll to the last page)
            let cont_w = (h_pages as f64 * viewport_width) as i32;
            let cont_h = (v_pages as f64 * viewport_height) as i32;
            (max_h, max_v, cont_w, cont_h)
        } else {
            // Default: scroll to content bounds
            ((content_width - viewport_width).max(0.0), (content_height - viewport_height).max(0.0), content_size.0, content_size.1)
        };

        let needs_h_scroll = max_h_scroll > 1.0;
        let needs_v_scroll = max_v_scroll > 1.0;

        if !needs_h_scroll && !needs_v_scroll {
            // No scrolling needed, reschedule check
            schedule_auto_scroll(scrolled, config, layout, active, generation, current_gen, bg);
            return;
        }

        // Update background size (expanded for whole pages mode)
        bg.set_size_request(container_width, container_height);

        // Check current position against effective bounds
        let at_h_end = h_adj.value() >= max_h_scroll - 1.0;
        let at_v_end = v_adj.value() >= max_v_scroll - 1.0;

        // Determine scroll action based on position
        // Pattern: right across row, then down+left to next row, repeat
        let (h_start, h_target, v_start, v_target) = if !at_h_end && needs_h_scroll {
            // Scroll right one viewport width
            let h_start = h_adj.value();
            let h_target = (h_start + viewport_width).min(max_h_scroll);
            (h_start, h_target, v_adj.value(), v_adj.value())
        } else if at_h_end && !at_v_end && needs_v_scroll {
            // At right edge, move down one row and back to left
            let v_start = v_adj.value();
            let v_target = (v_start + viewport_height).min(max_v_scroll);
            (h_adj.value(), 0.0, v_start, v_target)
        } else {
            // At bottom-right or only horizontal content, wrap to top-left
            (h_adj.value(), 0.0, v_adj.value(), 0.0)
        };

        // Run animation (200ms total, ~12 frames)
        const ANIMATION_MS: u64 = 200;
        const FRAME_MS: u64 = 16;
        let frame_count = Rc::new(RefCell::new(0u32));
        let total_frames = (ANIMATION_MS / FRAME_MS) as u32;

        gtk4::glib::timeout_add_local(std::time::Duration::from_millis(FRAME_MS), move || {
            // Check generation - stop if a new cycle was started
            if *generation.borrow() != current_gen {
                return gtk4::glib::ControlFlow::Break;
            }

            let mut frame = frame_count.borrow_mut();
            *frame += 1;

            let progress = (*frame as f64) / (total_frames as f64);
            let eased = ease_in_out(progress.min(1.0));

            // Animate both h and v positions
            let h_pos = h_start + (h_target - h_start) * eased;
            let v_pos = v_start + (v_target - v_start) * eased;
            h_adj.set_value(h_pos);
            v_adj.set_value(v_pos);

            if *frame >= total_frames {
                // Animation done, schedule next scroll after delay
                schedule_auto_scroll(scrolled.clone(), config.clone(), layout.clone(), active.clone(), generation.clone(), current_gen, bg.clone());
                return gtk4::glib::ControlFlow::Break;
            }

            gtk4::glib::ControlFlow::Continue
        });
    });
}

/// Create and return the auto-scroll start function
///
/// Returns a closure that can be called to start/restart the auto-scroll system.
/// The closure increments the generation counter to invalidate any pending cycles
/// and starts a new scroll cycle if auto-scroll is enabled.
pub fn create_auto_scroll_starter(
    scrolled_window: &ScrolledWindow,
    app_config: &Rc<RefCell<AppConfig>>,
    grid_layout: &Rc<RefCell<GridLayout>>,
    window_background: &DrawingArea,
) -> impl Fn() + Clone + 'static {
    let scrolled_window = scrolled_window.clone();
    let app_config = app_config.clone();
    let grid_layout = grid_layout.clone();
    let window_background = window_background.clone();
    let active = Rc::new(RefCell::new(false));
    let generation = Rc::new(RefCell::new(0u32));

    move || {
        *active.borrow_mut() = false;

        // Increment generation to invalidate any pending scroll cycles
        let new_gen = {
            let mut gen = generation.borrow_mut();
            *gen = gen.wrapping_add(1);
            *gen
        };

        let cfg = app_config.borrow();
        if !cfg.window.auto_scroll_enabled {
            return;
        }
        drop(cfg);

        // Reset scroll position to top-left when starting
        scrolled_window.hadjustment().set_value(0.0);
        scrolled_window.vadjustment().set_value(0.0);

        // Start the auto-scroll cycle with current generation
        schedule_auto_scroll(
            scrolled_window.clone(),
            app_config.clone(),
            grid_layout.clone(),
            active.clone(),
            generation.clone(),
            new_gen,
            window_background.clone(),
        );
    }
}
