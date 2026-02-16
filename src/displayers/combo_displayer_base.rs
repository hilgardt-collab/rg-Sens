//! Shared base functionality for combo-style displayers
//!
//! This module provides common structures, traits, and helper functions
//! for combo displayers (Cyberpunk, Material, Industrial, Fighter HUD,
//! Retro Terminal, Synthwave, Art Deco, Art Nouveau, LCARS).
//!
//! Using this module eliminates ~400 lines of duplicate code per displayer.

use anyhow::Result;
use cairo::Context;
use gtk4::{prelude::*, DrawingArea};
use serde_json::Value;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// RAII guard for Cairo state that ensures restore() is called even on panic
struct CairoGuard<'a>(&'a Context);

impl<'a> CairoGuard<'a> {
    fn new(cr: &'a Context) -> std::result::Result<Self, cairo::Error> {
        cr.save()?;
        Ok(Self(cr))
    }
}

impl Drop for CairoGuard<'_> {
    fn drop(&mut self) {
        // Ignore errors during restore in drop to avoid panic during unwinding
        let _ = self.0.restore();
    }
}

use crate::core::{register_animation, PanelTransform, ANIMATION_SNAP_THRESHOLD};
use crate::displayers::combo_utils::{self, AnimatedValue};
use crate::ui::arc_display::render_arc;
use crate::ui::graph_display::DataPoint;
use crate::ui::lcars_display::{
    calculate_item_layouts_with_orientation, render_content_bar, render_content_core_bars,
    render_content_graph, render_content_static, render_content_text, ContentDisplayType,
    ContentItemConfig, SplitOrientation,
};
use crate::ui::speedometer_display::render_speedometer_with_theme;
use crate::ui::theme::ComboThemeConfig;

// Re-export combo traits from the types crate
pub use rg_sens_types::combo::{ComboFrameConfig, LayoutFrameConfig, ThemedFrameConfig};

/// Parameters needed for drawing content items.
/// This struct collects all the data needed by draw_content_items_generic.
pub struct ContentDrawParams<'a> {
    pub values: &'a HashMap<String, Value>,
    pub bar_values: &'a HashMap<String, AnimatedValue>,
    pub core_bar_values: &'a HashMap<String, Vec<AnimatedValue>>,
    pub graph_history: &'a HashMap<String, VecDeque<DataPoint>>,
    pub content_items: &'a HashMap<String, ContentItemConfig>,
    pub group_item_orientations: &'a [SplitOrientation],
    pub split_orientation: SplitOrientation,
    pub theme: &'a ComboThemeConfig,
}

/// Draw content items in a given area.
/// This is the shared implementation used by all combo displayers.
///
/// The `draw_item_frame` closure is called for each item to draw style-specific framing.
/// Pass `|_, _, _, _, _| {}` if no item framing is needed.
#[allow(clippy::too_many_arguments)]
pub fn draw_content_items_generic<F>(
    cr: &Context,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    base_prefix: &str,
    count: u32,
    group_idx: usize,
    params: &ContentDrawParams,
    draw_item_frame: F,
) -> Result<(), cairo::Error>
where
    F: Fn(&Context, f64, f64, f64, f64),
{
    if count == 0 || w <= 0.0 || h <= 0.0 {
        return Ok(());
    }

    // Get item orientation for this group (default to split orientation)
    let item_orientation = params
        .group_item_orientations
        .get(group_idx)
        .copied()
        .unwrap_or(params.split_orientation);

    // Reusable prefix buffer to avoid format!() allocations in loops
    // Uses write!() which is more efficient than format!() for repeated use
    let mut prefix_buf = String::with_capacity(base_prefix.len() + 4);

    // Determine fixed sizes for items that need them
    let mut fixed_sizes: HashMap<usize, f64> = HashMap::new();
    for i in 0..count as usize {
        // Build prefix without allocation using reusable buffer
        prefix_buf.clear();
        use std::fmt::Write;
        let _ = write!(prefix_buf, "{}{}", base_prefix, i + 1);
        if let Some(cfg) = params.content_items.get(&prefix_buf) {
            if !cfg.auto_height || matches!(cfg.display_as, ContentDisplayType::Graph) {
                fixed_sizes.insert(i, cfg.item_height);
            }
        }
    }

    // Calculate layouts with orientation
    let layouts = calculate_item_layouts_with_orientation(
        x,
        y,
        w,
        h,
        count,
        4.0,
        &fixed_sizes,
        item_orientation,
    );

    // Draw each item
    for (i, &(item_x, item_y, item_w, item_h)) in layouts.iter().enumerate() {
        // Build prefix without allocation using reusable buffer
        prefix_buf.clear();
        use std::fmt::Write;
        let _ = write!(prefix_buf, "{}{}", base_prefix, i + 1);
        let item_data = combo_utils::get_item_data(params.values, &prefix_buf);
        let slot_values = combo_utils::get_slot_values(params.values, &prefix_buf);

        // Get item config (or use default)
        let item_config = params
            .content_items
            .get(&prefix_buf)
            .cloned()
            .unwrap_or_default();

        // Draw item frame using the provided closure
        draw_item_frame(cr, item_x, item_y, item_w, item_h);

        // Get animated percent (use KeyBuffer to avoid allocation)
        let animated_percent = combo_utils::with_key_buffer(|buf| {
            let bar_key = buf.build_bar_key(&prefix_buf);
            params.bar_values.get(bar_key).map(|av| av.current)
        })
        .unwrap_or_else(|| item_data.percent());

        match item_config.display_as {
            ContentDisplayType::Bar => {
                render_content_bar(
                    cr,
                    item_x,
                    item_y,
                    item_w,
                    item_h,
                    &item_config.bar_config,
                    params.theme,
                    &item_data,
                    animated_percent,
                    Some(&slot_values),
                )?;
            }
            ContentDisplayType::Text => {
                render_content_text(
                    cr,
                    item_x,
                    item_y,
                    item_w,
                    item_h,
                    &item_config.bar_config,
                    params.theme,
                    &item_data,
                    Some(&slot_values),
                )?;
            }
            ContentDisplayType::Graph => {
                // Use KeyBuffer to avoid allocation for graph_key lookup
                let empty_history = VecDeque::new();
                let history = combo_utils::with_key_buffer(|buf| {
                    let graph_key = buf.build_graph_key(&prefix_buf);
                    params.graph_history.get(graph_key)
                })
                .unwrap_or(&empty_history);

                if let Err(e) = render_content_graph(
                    cr,
                    item_x,
                    item_y,
                    item_w,
                    item_h,
                    &item_config.graph_config,
                    params.theme,
                    history,
                    &slot_values,
                ) {
                    log::warn!("Failed to render graph for {}: {}", prefix_buf, e);
                    render_content_text(
                        cr,
                        item_x,
                        item_y,
                        item_w,
                        item_h,
                        &item_config.bar_config,
                        params.theme,
                        &item_data,
                        Some(&slot_values),
                    )?;
                }
            }
            ContentDisplayType::LevelBar => {
                render_content_text(
                    cr,
                    item_x,
                    item_y,
                    item_w,
                    item_h,
                    &item_config.bar_config,
                    params.theme,
                    &item_data,
                    Some(&slot_values),
                )?;
            }
            ContentDisplayType::CoreBars => {
                let core_bars_config = &item_config.core_bars_config;
                let core_values: Vec<f64> =
                    if let Some(animated) = params.core_bar_values.get(&prefix_buf) {
                        animated.iter().map(|av| av.current).collect()
                    } else {
                        let capacity = core_bars_config
                            .end_core
                            .saturating_sub(core_bars_config.start_core)
                            + 1;
                        let mut raw_values: Vec<f64> = Vec::with_capacity(capacity);
                        // Use KeyBuffer to avoid allocation for core_key lookups
                        for core_idx in core_bars_config.start_core..=core_bars_config.end_core {
                            let value = combo_utils::with_key_buffer(|buf| {
                                let core_key = buf.build_core_key(&prefix_buf, core_idx);
                                params.values.get(core_key).and_then(|v| v.as_f64())
                            })
                            .unwrap_or(0.0);
                            raw_values.push(value / 100.0);
                        }

                        if raw_values.is_empty() {
                            for core_idx in 0..128 {
                                let value = combo_utils::with_key_buffer(|buf| {
                                    let core_key = buf.build_core_key(&prefix_buf, core_idx);
                                    params.values.get(core_key).and_then(|v| v.as_f64())
                                });
                                if let Some(v) = value {
                                    raw_values.push(v / 100.0);
                                } else {
                                    break;
                                }
                            }
                        }
                        raw_values
                    };

                render_content_core_bars(
                    cr,
                    item_x,
                    item_y,
                    item_w,
                    item_h,
                    core_bars_config,
                    params.theme,
                    &core_values,
                    Some(&slot_values),
                )?;
            }
            ContentDisplayType::Static => {
                render_content_static(
                    cr,
                    item_x,
                    item_y,
                    item_w,
                    item_h,
                    &item_config.static_config,
                    params.theme,
                    Some(&slot_values),
                )?;
            }
            ContentDisplayType::Arc => {
                // Use CairoGuard to ensure restore() even if render_arc panics
                let _guard = CairoGuard::new(cr)?;
                cr.translate(item_x, item_y);
                render_arc(
                    cr,
                    &item_config.arc_config,
                    params.theme,
                    animated_percent,
                    &slot_values,
                    item_w,
                    item_h,
                )?;
                // _guard drops here, calling restore()
            }
            ContentDisplayType::Speedometer => {
                // Use CairoGuard to ensure restore() even if render panics
                let _guard = CairoGuard::new(cr)?;
                cr.translate(item_x, item_y);
                if let Err(e) = render_speedometer_with_theme(
                    cr,
                    &item_config.speedometer_config,
                    animated_percent,
                    &slot_values,
                    item_w,
                    item_h,
                    params.theme,
                ) {
                    log::warn!("Failed to render speedometer for {}: {}", prefix_buf, e);
                }
                // _guard drops here, calling restore()
            }
        }
    }

    Ok(())
}

/// Shared display data for all combo displayers.
/// This can be used by displayers that want to use a common data structure.
#[derive(Clone)]
pub struct ComboDisplayData {
    pub values: HashMap<String, Value>,
    pub bar_values: HashMap<String, AnimatedValue>,
    pub core_bar_values: HashMap<String, Vec<AnimatedValue>>,
    pub graph_history: HashMap<String, VecDeque<DataPoint>>,
    pub graph_start_time: Instant,
    pub last_update: Instant,
    pub transform: PanelTransform,
    pub dirty: bool,
    /// Cached prefixes to avoid regenerating every frame
    pub cached_prefixes: Vec<String>,
    /// Cached prefix set for O(1) lookups (regenerated when prefixes change)
    pub cached_prefix_set: std::collections::HashSet<String>,
    /// Group item counts used to generate cached_prefixes (for invalidation)
    pub cached_group_counts: Vec<usize>,
}

impl Default for ComboDisplayData {
    fn default() -> Self {
        Self {
            values: HashMap::new(),
            bar_values: HashMap::new(),
            core_bar_values: HashMap::new(),
            graph_history: HashMap::new(),
            graph_start_time: Instant::now(),
            last_update: Instant::now(),
            transform: PanelTransform::default(),
            dirty: true,
            cached_prefixes: Vec::new(),
            cached_prefix_set: std::collections::HashSet::new(),
            cached_group_counts: Vec::new(),
        }
    }
}

/// Set up the animation timer for a combo displayer.
/// Registers the animation callback with the global AnimationManager.
/// Call this after creating the drawing area.
pub fn setup_combo_animation_timer<F, G>(
    drawing_area: &DrawingArea,
    data: Arc<Mutex<ComboDisplayData>>,
    animation_enabled: F,
    animation_speed: G,
) where
    F: Fn(&ComboDisplayData) -> bool + 'static,
    G: Fn(&ComboDisplayData) -> f64 + 'static,
{
    register_animation(drawing_area.downgrade(), move || {
        if let Ok(mut data) = data.try_lock() {
            let mut redraw = data.dirty;
            if data.dirty {
                data.dirty = false;
            }

            if animation_enabled(&data) {
                // Quick check: any animations in progress?
                // This avoids Instant::now() and iteration when nothing is animating
                let has_bar_animations = data
                    .bar_values
                    .values()
                    .any(|a| (a.current - a.target).abs() > ANIMATION_SNAP_THRESHOLD);
                let has_core_animations = data.core_bar_values.values().any(|v| {
                    v.iter()
                        .any(|a| (a.current - a.target).abs() > ANIMATION_SNAP_THRESHOLD)
                });

                if has_bar_animations || has_core_animations {
                    let now = Instant::now();
                    let elapsed = now.duration_since(data.last_update).as_secs_f64();
                    data.last_update = now;

                    let speed = animation_speed(&data);

                    // Animate bar values
                    if has_bar_animations {
                        for anim in data.bar_values.values_mut() {
                            if (anim.current - anim.target).abs() > ANIMATION_SNAP_THRESHOLD {
                                let delta = (anim.target - anim.current) * speed * elapsed;
                                anim.current += delta;

                                if (anim.current - anim.target).abs() < ANIMATION_SNAP_THRESHOLD {
                                    anim.current = anim.target;
                                }
                                redraw = true;
                            }
                        }
                    }

                    // Animate core bar values
                    if has_core_animations {
                        for core_anims in data.core_bar_values.values_mut() {
                            for anim in core_anims.iter_mut() {
                                if (anim.current - anim.target).abs() > ANIMATION_SNAP_THRESHOLD {
                                    let delta = (anim.target - anim.current) * speed * elapsed;
                                    anim.current += delta;

                                    if (anim.current - anim.target).abs() < ANIMATION_SNAP_THRESHOLD
                                    {
                                        anim.current = anim.target;
                                    }
                                    redraw = true;
                                }
                            }
                        }
                    }
                }
            }

            redraw
        } else {
            false
        }
    });
}

/// Extended animation timer that works with wrapper types containing ComboDisplayData.
///
/// This version allows displayers to wrap ComboDisplayData in their own DisplayData struct
/// while still using the shared animation logic. It also supports custom per-frame animation
/// via an optional callback.
///
/// # Type Parameters
/// * `D` - The wrapper data type (e.g., `DisplayData` containing config + combo)
/// * `AE` - Closure to check if animation is enabled
/// * `AS` - Closure to get animation speed
/// * `GC` - Closure to get mutable reference to ComboDisplayData from wrapper
/// * `CA` - Optional closure for custom animation logic (scanlines, cursor blink, etc.)
///
/// # Arguments
/// * `drawing_area` - The GTK DrawingArea to redraw
/// * `data` - Arc<Mutex<D>> containing the displayer's data
/// * `animation_enabled` - Returns true if bar animations should run
/// * `animation_speed` - Returns the animation speed multiplier
/// * `get_combo` - Returns mutable reference to the ComboDisplayData within D
/// * `custom_animation` - Optional callback for style-specific animations. Called with
///   (data: &mut D, elapsed: f64) and returns true if redraw is needed.
pub fn setup_combo_animation_timer_ext<D, AE, AS, GC, CA>(
    drawing_area: &DrawingArea,
    data: Arc<Mutex<D>>,
    animation_enabled: AE,
    animation_speed: AS,
    get_combo: GC,
    custom_animation: Option<CA>,
) where
    D: 'static,
    AE: Fn(&D) -> bool + 'static,
    AS: Fn(&D) -> f64 + 'static,
    GC: Fn(&mut D) -> &mut ComboDisplayData + 'static,
    CA: Fn(&mut D, f64) -> bool + 'static,
{
    register_animation(drawing_area.downgrade(), move || {
        let lock_result = data.try_lock();
        if lock_result.is_err() {
            // Track lock failures to diagnose drawing issues
            static TICK_LOCK_FAIL_COUNT: std::sync::atomic::AtomicU64 =
                std::sync::atomic::AtomicU64::new(0);
            let count = TICK_LOCK_FAIL_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if count < 5 || count.is_multiple_of(100) {
                log::warn!(
                    "Animation tick: try_lock failed ({} total failures)",
                    count + 1
                );
            }
            return false;
        }
        if let Ok(mut data) = lock_result {
            let combo = get_combo(&mut data);
            let mut redraw = combo.dirty;
            if combo.dirty {
                combo.dirty = false;
            }

            // Check if we have custom animation that needs elapsed time
            let has_custom_anim = custom_animation.is_some();

            // Quick check: any bar animations in progress?
            let has_bar_animations = animation_enabled(&data) && {
                let combo = get_combo(&mut data);
                combo
                    .bar_values
                    .values()
                    .any(|a| (a.current - a.target).abs() > ANIMATION_SNAP_THRESHOLD)
            };
            let has_core_animations = animation_enabled(&data) && {
                let combo = get_combo(&mut data);
                combo.core_bar_values.values().any(|v| {
                    v.iter()
                        .any(|a| (a.current - a.target).abs() > ANIMATION_SNAP_THRESHOLD)
                })
            };

            // Only calculate elapsed time if something actually needs it
            if has_custom_anim || has_bar_animations || has_core_animations {
                let combo = get_combo(&mut data);
                let now = Instant::now();
                let elapsed = now.duration_since(combo.last_update).as_secs_f64();
                combo.last_update = now;

                // Run custom animation if provided
                if let Some(ref custom_anim) = custom_animation {
                    if custom_anim(&mut data, elapsed) {
                        redraw = true;
                    }
                }

                // Run bar/core bar animations if enabled
                if has_bar_animations || has_core_animations {
                    let speed = animation_speed(&data);
                    let combo = get_combo(&mut data);

                    // Animate bar values
                    if has_bar_animations {
                        for anim in combo.bar_values.values_mut() {
                            if (anim.current - anim.target).abs() > ANIMATION_SNAP_THRESHOLD {
                                let delta = (anim.target - anim.current) * speed * elapsed;
                                anim.current += delta;

                                if (anim.current - anim.target).abs() < ANIMATION_SNAP_THRESHOLD {
                                    anim.current = anim.target;
                                }
                                redraw = true;
                            }
                        }
                    }

                    // Animate core bar values
                    if has_core_animations {
                        for core_anims in combo.core_bar_values.values_mut() {
                            for anim in core_anims.iter_mut() {
                                if (anim.current - anim.target).abs() > ANIMATION_SNAP_THRESHOLD {
                                    let delta = (anim.target - anim.current) * speed * elapsed;
                                    anim.current += delta;

                                    if (anim.current - anim.target).abs() < ANIMATION_SNAP_THRESHOLD
                                    {
                                        anim.current = anim.target;
                                    }
                                    redraw = true;
                                }
                            }
                        }
                    }
                }
            }

            redraw
        } else {
            false
        }
    });
}

/// Convenience type alias for displayers without custom animation
pub type NoCustomAnimation = fn(&mut (), f64) -> bool;

/// Handle update_data for a combo displayer.
/// This updates values, animations, and graph history.
pub fn handle_combo_update_data(
    data: &mut ComboDisplayData,
    input: &HashMap<String, Value>,
    group_item_counts: &[usize],
    content_items: &HashMap<String, ContentItemConfig>,
    animation_enabled: bool,
) {
    let timestamp = data.graph_start_time.elapsed().as_secs_f64();

    // Only regenerate prefixes if group_item_counts changed (avoid allocation every frame)
    if data.cached_group_counts.as_slice() != group_item_counts {
        data.cached_prefixes = combo_utils::generate_prefixes(group_item_counts);
        // Also regenerate the prefix set for O(1) lookups
        data.cached_prefix_set = data.cached_prefixes.iter().cloned().collect();
        data.cached_group_counts.clear();
        data.cached_group_counts
            .extend_from_slice(group_item_counts);
    }

    // Filter values using cached prefix set (avoids HashSet creation on every call)
    combo_utils::filter_values_with_owned_prefix_set(
        input,
        &data.cached_prefix_set,
        &mut data.values,
    );

    // Update each item using index-based iteration to avoid cloning cached_prefixes
    let prefix_count = data.cached_prefixes.len();
    for i in 0..prefix_count {
        // Get prefix reference - safe because prefix_count is fixed and we only mutate other fields
        let prefix = &data.cached_prefixes[i];
        let item_data = combo_utils::get_item_data(input, prefix);
        let target_percent = item_data.percent();
        let numerical_value = item_data.numerical_value;

        // Get item config before mutating data
        let default_config = ContentItemConfig::default();
        let item_config = content_items.get(prefix).cloned().unwrap_or(default_config);

        // Now do the mutable operations
        let prefix = &data.cached_prefixes[i]; // Re-borrow after item_config lookup
        combo_utils::update_bar_animation(
            &mut data.bar_values,
            prefix,
            target_percent,
            animation_enabled,
        );

        match item_config.display_as {
            ContentDisplayType::Graph => {
                let prefix = &data.cached_prefixes[i];
                combo_utils::update_graph_history(
                    &mut data.graph_history,
                    prefix,
                    numerical_value,
                    timestamp,
                    item_config.graph_config.max_data_points,
                );
            }
            ContentDisplayType::CoreBars => {
                let prefix = &data.cached_prefixes[i];
                combo_utils::update_core_bars(
                    input,
                    &mut data.core_bar_values,
                    prefix,
                    &item_config.core_bars_config,
                    animation_enabled,
                );
            }
            _ => {}
        }
    }

    // Clean up stale animation entries
    combo_utils::cleanup_bar_values(&mut data.bar_values, &data.cached_prefixes);
    combo_utils::cleanup_core_bar_values(&mut data.core_bar_values, &data.cached_prefixes);
    combo_utils::cleanup_graph_history(&mut data.graph_history, &data.cached_prefixes);

    data.transform = PanelTransform::from_values(input);
    data.dirty = true;
}

/// Helper to check if a combo displayer needs redraw.
pub fn combo_needs_redraw(data: &Arc<Mutex<ComboDisplayData>>) -> bool {
    data.try_lock().map(|data| data.dirty).unwrap_or(true)
}

// ============================================================================
// Generic Combo Panel Framework
// ============================================================================
//
// Re-export FrameRenderer from the render crate
pub use rg_sens_render::combo_traits::FrameRenderer;
