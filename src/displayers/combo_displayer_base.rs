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
use crate::displayers::layer_canvas::{LayerKey, LayerKind, LayerSink, Rect};
use crate::ui::arc_display::render_arc;
use crate::ui::clock_display::{render_analog_clock_with_theme, AnalogClockConfig};
use crate::ui::graph_display::DataPoint;
use crate::ui::lcars_display::{
    calculate_item_layouts_with_orientation, render_content_bar, render_content_core_bars,
    render_content_graph, render_content_static, render_content_text, ContentDisplayType,
    ContentItemConfig, ContentItemData, SplitOrientation,
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

/// Draw an analog clock face in an item's box from the slot's time values
/// (`hour`, `minute`, `second`, `millisecond` of a clock child source).
/// Shared by the generic and the LCARS content paths.
#[allow(clippy::too_many_arguments)]
pub fn render_content_clock(
    cr: &Context,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    config: &AnalogClockConfig,
    theme: &ComboThemeConfig,
    slot_values: &HashMap<String, Value>,
) -> Result<(), cairo::Error> {
    if !(x.is_finite() && y.is_finite() && w > 0.0 && h > 0.0) {
        return Ok(());
    }
    let (hour, minute, second) = combo_utils::clock_hands(slot_values, config.smooth_seconds);
    // Restore even if the renderer errors out part-way
    let _guard = CairoGuard::new(cr)?;
    cr.translate(x, y);
    render_analog_clock_with_theme(cr, config, hour, minute, second, w, h, Some(theme))
}

/// Draw content items in a given area.
/// This is the shared implementation used by all combo displayers.
///
/// Each item is a layer of `sink` (see `layer_canvas`): one that reads the
/// same inputs as on the frame before keeps its render node. `config_version`
/// stands for the whole config in the items' keys.
///
/// The `draw_item_frame` closure is called for each item to draw style-specific framing.
/// Pass `|_, _, _, _, _| {}` if no item framing is needed.
#[allow(clippy::too_many_arguments)]
pub fn draw_content_items_generic<F>(
    sink: &mut LayerSink,
    config_version: u64,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    base_prefix: &str,
    count: u32,
    group_idx: usize,
    params: &ContentDrawParams,
    draw_item_frame: F,
) where
    F: Fn(&Context, f64, f64, f64, f64),
{
    if count == 0 || w <= 0.0 || h <= 0.0 {
        return;
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
            // Use fixed size if auto_height is disabled or for Graph/LevelBar
            // display types (kept in sync with the LCARS path)
            if !cfg.auto_height
                || matches!(
                    cfg.display_as,
                    ContentDisplayType::Graph | ContentDisplayType::LevelBar
                )
            {
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

    // Hoisted default so per-item config lookups can borrow instead of clone
    let default_item_config = ContentItemConfig::default();
    let empty_history = VecDeque::new();

    // Draw each item
    for (i, &(item_x, item_y, item_w, item_h)) in layouts.iter().enumerate() {
        // Build prefix without allocation using reusable buffer
        prefix_buf.clear();
        use std::fmt::Write;
        let _ = write!(prefix_buf, "{}{}", base_prefix, i + 1);
        let prefix = prefix_buf.as_str();
        let item_data = combo_utils::get_item_data(params.values, prefix);
        let slot_values = combo_utils::get_slot_values(params.values, prefix);

        // Get item config (or use default) — borrow, don't clone per frame
        let item_config = params
            .content_items
            .get(prefix)
            .unwrap_or(&default_item_config);
        let display_as = item_config.display_as;

        // Get animated percent (use KeyBuffer to avoid allocation)
        let animated_percent = combo_utils::with_key_buffer(|buf| {
            let bar_key = buf.build_bar_key(prefix);
            params.bar_values.get(bar_key).map(|av| av.current)
        })
        .unwrap_or_else(|| item_data.percent());
        let animated_percent = shown_glide(item_config, animated_percent);

        let history = if display_as == ContentDisplayType::Graph {
            // Use KeyBuffer to avoid allocation for graph_key lookup
            combo_utils::with_key_buffer(|buf| {
                let graph_key = buf.build_graph_key(prefix);
                params.graph_history.get(graph_key)
            })
            .unwrap_or(&empty_history)
        } else {
            &empty_history
        };

        let core_values = if display_as == ContentDisplayType::CoreBars {
            core_bar_item_values(prefix, item_config, params.values, params.core_bar_values)
        } else {
            Vec::new()
        };

        let rect = (item_x, item_y, item_w, item_h);
        let key = content_item_key(
            config_version,
            rect,
            display_as,
            &slot_values,
            animated_percent,
            &core_values,
            history,
        );
        sink.layer(
            (LayerKind::Item, group_idx as u32, i as u32),
            key,
            Some(rect),
            &|cr| {
                // The style's frame around the item, then the item
                draw_item_frame(cr, item_x, item_y, item_w, item_h);
                if let Err(e) = draw_content_item(
                    cr,
                    rect,
                    prefix,
                    item_config,
                    params.theme,
                    &item_data,
                    &slot_values,
                    animated_percent,
                    history,
                    &core_values,
                ) {
                    log::debug!("Content item {} render error: {}", prefix, e);
                }
                0.0
            },
        );
    }
}

/// An item's glide position as the item draws it. A segmented bar or arc
/// changes only when another segment lights, so its position is the step it
/// is on (`shown_value` of its renderer): the picture is the same, and the
/// item's layer key, which is made of this, stays the same over the frames
/// of a glide that light nothing new. Without it every such frame recorded,
/// rasterised and uploaded the item again.
pub fn shown_glide(item_config: &ContentItemConfig, animated_percent: f64) -> f64 {
    match item_config.display_as {
        ContentDisplayType::Bar | ContentDisplayType::LevelBar => {
            crate::ui::bar_display::shown_value(&item_config.bar_config, animated_percent)
        }
        ContentDisplayType::Arc => {
            crate::ui::arc_display::shown_value(&item_config.arc_config, animated_percent)
        }
        _ => animated_percent,
    }
}

/// The key of a content item's layer (see `layer_canvas`): everything
/// [`draw_content_item`] reads. The config (the item's and the theme) is in
/// `config_version`, the slot's values hold what the item data was made from,
/// and the glide position goes in only for the types that draw it (every slot
/// has one, and a text item must not be drawn again while it moves).
pub fn content_item_key(
    config_version: u64,
    rect: Rect,
    display_as: ContentDisplayType,
    slot_values: &HashMap<String, Value>,
    animated_percent: f64,
    core_values: &[f64],
    history: &VecDeque<DataPoint>,
) -> u64 {
    let mut key = LayerKey::new();
    key.u64(config_version)
        .rect(rect)
        .values(slot_values)
        .f64s(core_values)
        .u64(history.len() as u64);
    if matches!(
        display_as,
        ContentDisplayType::Bar
            | ContentDisplayType::LevelBar
            | ContentDisplayType::Arc
            | ContentDisplayType::Speedometer
    ) {
        key.f64(animated_percent);
    }
    for point in history {
        key.f64(point.timestamp).f64(point.value);
    }
    key.finish()
}

/// The values a core bars item shows: the animated ones if there are
/// any (as the bars draw them, see [`shown_glide`]), otherwise the raw
/// readings (the first frame).
pub fn core_bar_item_values(
    prefix: &str,
    item_config: &ContentItemConfig,
    values: &HashMap<String, Value>,
    core_bar_values: &HashMap<String, Vec<AnimatedValue>>,
) -> Vec<f64> {
    let core_bars_config = &item_config.core_bars_config;
    if let Some(animated) = core_bar_values.get(prefix) {
        // as the bars draw them: segmented ones a step at a time
        return animated
            .iter()
            .map(|av| crate::ui::core_bars_display::shown_value(core_bars_config, av.current))
            .collect();
    }
    let capacity = core_bars_config
        .end_core
        .saturating_sub(core_bars_config.start_core)
        + 1;
    let mut raw_values: Vec<f64> = Vec::with_capacity(capacity);
    // Use KeyBuffer to avoid format! allocation per core
    for core_idx in core_bars_config.start_core..=core_bars_config.end_core {
        let value = combo_utils::with_key_buffer(|buf| {
            let core_key = buf.build_core_key(prefix, core_idx);
            values.get(core_key).and_then(|v| v.as_f64())
        })
        .unwrap_or(0.0);
        raw_values.push(value / 100.0);
    }

    // If no specific core values found, try to find any core values
    if raw_values.is_empty() {
        for core_idx in 0..128 {
            let value = combo_utils::with_key_buffer(|buf| {
                let core_key = buf.build_core_key(prefix, core_idx);
                values.get(core_key).and_then(|v| v.as_f64())
            });
            if let Some(v) = value {
                raw_values.push(v / 100.0);
            } else {
                break;
            }
        }
    }
    raw_values
}

/// Draw one content item in its box: the one implementation behind every
/// combo panel. What this reads is what [`content_item_key`] covers; keep the
/// two in step.
#[allow(clippy::too_many_arguments)]
pub fn draw_content_item(
    cr: &Context,
    (item_x, item_y, item_w, item_h): Rect,
    prefix: &str,
    item_config: &ContentItemConfig,
    theme: &ComboThemeConfig,
    item_data: &ContentItemData,
    slot_values: &HashMap<String, Value>,
    animated_percent: f64,
    history: &VecDeque<DataPoint>,
    core_values: &[f64],
) -> Result<(), cairo::Error> {
    match item_config.display_as {
        // A level bar is a bar at the item's fixed height (sized like a
        // graph); the item page shares the bar editor
        ContentDisplayType::Bar | ContentDisplayType::LevelBar => {
            render_content_bar(
                cr,
                item_x,
                item_y,
                item_w,
                item_h,
                &item_config.bar_config,
                theme,
                item_data,
                animated_percent,
                Some(slot_values),
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
                theme,
                item_data,
                Some(slot_values),
            )?;
        }
        ContentDisplayType::Graph => {
            if let Err(e) = render_content_graph(
                cr,
                item_x,
                item_y,
                item_w,
                item_h,
                &item_config.graph_config,
                theme,
                history,
                slot_values,
            ) {
                log::warn!("Failed to render graph for {}: {}", prefix, e);
                // Fall back to text display on error
                render_content_text(
                    cr,
                    item_x,
                    item_y,
                    item_w,
                    item_h,
                    &item_config.bar_config,
                    theme,
                    item_data,
                    Some(slot_values),
                )?;
            }
        }
        ContentDisplayType::CoreBars => {
            render_content_core_bars(
                cr,
                item_x,
                item_y,
                item_w,
                item_h,
                &item_config.core_bars_config,
                theme,
                core_values,
                Some(slot_values),
            )?;
        }
        ContentDisplayType::Static => {
            // Render static background with optional text overlay
            render_content_static(
                cr,
                item_x,
                item_y,
                item_w,
                item_h,
                &item_config.static_config,
                theme,
                Some(slot_values),
            )?;
        }
        ContentDisplayType::Arc => {
            cr.save()?;
            cr.translate(item_x, item_y);
            render_arc(
                cr,
                &item_config.arc_config,
                theme,
                animated_percent,
                slot_values,
                item_w,
                item_h,
            )?;
            cr.restore()?;
        }
        ContentDisplayType::ClockAnalog => {
            render_content_clock(
                cr,
                item_x,
                item_y,
                item_w,
                item_h,
                &item_config.clock_analog_config,
                theme,
                slot_values,
            )?;
        }
        ContentDisplayType::Speedometer => {
            cr.save()?;
            cr.translate(item_x, item_y);
            if let Err(e) = render_speedometer_with_theme(
                cr,
                &item_config.speedometer_config,
                animated_percent,
                slot_values,
                item_w,
                item_h,
                theme,
            ) {
                log::warn!("Failed to render speedometer for {}: {}", prefix, e);
            }
            cr.restore()?;
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
            if count < 5 {
                log::debug!(
                    "Animation tick: try_lock failed ({} total failures)",
                    count + 1
                );
            } else if count.is_multiple_of(1000) {
                log::info!(
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

            // Refresh the timestamp on EVERY tick, not just while animating:
            // the first animating frame after idle must not see the stale
            // `elapsed` accumulated since the previous animation converged
            // (with default speed 8.0 that overshot the target several times).
            let elapsed = {
                let combo = get_combo(&mut data);
                let now = Instant::now();
                let elapsed = now.duration_since(combo.last_update).as_secs_f64();
                combo.last_update = now;
                elapsed
            };

            if has_custom_anim || has_bar_animations || has_core_animations {
                // Run custom animation if provided
                if let Some(ref custom_anim) = custom_animation {
                    if custom_anim(&mut data, elapsed) {
                        redraw = true;
                    }
                }

                // Run bar/core bar animations if enabled
                if has_bar_animations || has_core_animations {
                    let speed = animation_speed(&data);
                    // Cap the step so a long frame lands at most exactly on
                    // the target, never past it
                    let step = (speed * elapsed).min(1.0);
                    let combo = get_combo(&mut data);

                    // Animate bar values
                    if has_bar_animations {
                        for anim in combo.bar_values.values_mut() {
                            if (anim.current - anim.target).abs() > ANIMATION_SNAP_THRESHOLD {
                                let delta = (anim.target - anim.current) * step;
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
                                    let delta = (anim.target - anim.current) * step;
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

/// Pre-computed data for a combo update, produced by `prepare_combo_update`.
///
/// Expensive work (prefix generation, value filtering, item data extraction) is done
/// up-front without holding the mutex, so `apply_combo_update` only needs a brief lock.
pub struct ComboUpdatePrep {
    prefixes: Vec<String>,
    prefix_set: std::collections::HashSet<String>,
    filtered_values: HashMap<String, Value>,
    item_updates: Vec<ComboItemUpdate>,
    timestamp: f64,
    animation_enabled: bool,
    group_item_counts: Vec<usize>,
    transform: PanelTransform,
}

struct ComboItemUpdate {
    target_percent: f64,
    numerical_value: f64,
    display_as: ContentDisplayType,
    graph_max_points: usize,
    core_bars_config: crate::ui::core_bars_display::CoreBarsConfig,
}

/// Phase 1: Pre-compute expensive work outside the mutex lock.
///
/// Call this after extracting config from a brief lock, then pass the result
/// to `apply_combo_update` under a second brief lock.
pub fn prepare_combo_update(
    input: &HashMap<String, Value>,
    group_item_counts: Vec<usize>,
    content_items: &HashMap<String, ContentItemConfig>,
    animation_enabled: bool,
    timestamp: f64,
) -> ComboUpdatePrep {
    let prefixes = combo_utils::generate_prefixes(&group_item_counts);
    let prefix_set: std::collections::HashSet<String> = prefixes.iter().cloned().collect();

    let mut filtered_values = HashMap::with_capacity(prefixes.len() * 8);
    combo_utils::filter_values_with_owned_prefix_set(input, &prefix_set, &mut filtered_values);

    let item_updates: Vec<ComboItemUpdate> = prefixes
        .iter()
        .map(|prefix| {
            let item_data = combo_utils::get_item_data(input, prefix);
            let default_config = ContentItemConfig::default();
            let item_config = content_items.get(prefix).unwrap_or(&default_config);
            ComboItemUpdate {
                target_percent: item_data.percent(),
                numerical_value: item_data.numerical_value,
                display_as: item_config.display_as,
                graph_max_points: item_config.graph_config.max_data_points,
                core_bars_config: item_config.core_bars_config.clone(),
            }
        })
        .collect();

    let transform = PanelTransform::from_values(input);

    ComboUpdatePrep {
        prefixes,
        prefix_set,
        filtered_values,
        item_updates,
        timestamp,
        animation_enabled,
        group_item_counts,
        transform,
    }
}

/// Phase 2: Apply pre-computed update results under a brief mutex lock.
///
/// Only does quick HashMap mutations and simple math — no string allocations
/// or expensive filtering. Typically completes in <1ms.
pub fn apply_combo_update(
    data: &mut ComboDisplayData,
    prep: ComboUpdatePrep,
    input: &HashMap<String, Value>,
) {
    // Update caches
    data.cached_group_counts.clear();
    data.cached_group_counts
        .extend_from_slice(&prep.group_item_counts);

    // Swap in pre-computed values
    data.values = prep.filtered_values;
    data.cached_prefixes = prep.prefixes;
    data.cached_prefix_set = prep.prefix_set;

    // Apply per-item animation updates (quick math per entry)
    for (i, update) in prep.item_updates.iter().enumerate() {
        let prefix = &data.cached_prefixes[i];
        combo_utils::update_bar_animation(
            &mut data.bar_values,
            prefix,
            update.target_percent,
            prep.animation_enabled,
        );

        match update.display_as {
            ContentDisplayType::Graph => {
                let prefix = &data.cached_prefixes[i];
                combo_utils::update_graph_history(
                    &mut data.graph_history,
                    prefix,
                    update.numerical_value,
                    prep.timestamp,
                    update.graph_max_points,
                );
            }
            ContentDisplayType::CoreBars => {
                let prefix = &data.cached_prefixes[i];
                combo_utils::update_core_bars(
                    input,
                    &mut data.core_bar_values,
                    prefix,
                    &update.core_bars_config,
                    prep.animation_enabled,
                );
            }
            _ => {}
        }
    }

    // Clean up stale animation entries (builds the prefix set once)
    combo_utils::cleanup_all_animation_state(
        &mut data.bar_values,
        &mut data.core_bar_values,
        &mut data.graph_history,
        &data.cached_prefixes,
    );

    data.transform = prep.transform;
    data.dirty = true;
}

// ============================================================================
// Generic Combo Panel Framework
// ============================================================================
//
// Re-export FrameRenderer from the render crate
pub use rg_sens_render::combo_traits::FrameRenderer;

#[cfg(test)]
mod core_bars_tests {
    use super::*;
    use rg_sens_types::display_configs::core_bars::CoreBarsConfig;

    fn core_item(animate: bool) -> HashMap<String, ContentItemConfig> {
        let item = ContentItemConfig {
            display_as: ContentDisplayType::CoreBars,
            core_bars_config: CoreBarsConfig {
                start_core: 0,
                end_core: 3,
                animate,
                ..Default::default()
            },
            ..Default::default()
        };
        HashMap::from([("group1_1".to_string(), item)])
    }

    fn reading(first: f64) -> HashMap<String, Value> {
        (0..4)
            .map(|i| {
                (
                    format!("group1_1_core{}_usage", i),
                    Value::from(first + 10.0 * i as f64),
                )
            })
            .collect()
    }

    fn push(
        data: &mut ComboDisplayData,
        items: &HashMap<String, ContentItemConfig>,
        values: &HashMap<String, Value>,
    ) {
        let prep = prepare_combo_update(values, vec![1], items, true, 0.0);
        apply_combo_update(data, prep, values);
    }

    fn currents(data: &ComboDisplayData) -> Vec<f64> {
        data.core_bar_values["group1_1"]
            .iter()
            .map(|a| a.current)
            .collect()
    }

    fn targets(data: &ComboDisplayData) -> Vec<f64> {
        data.core_bar_values["group1_1"]
            .iter()
            .map(|a| a.target)
            .collect()
    }

    #[test]
    fn core_bars_follow_each_reading() {
        let items = core_item(true);
        let mut data = ComboDisplayData::default();
        push(&mut data, &items, &reading(10.0));
        assert_eq!(currents(&data), vec![0.1, 0.2, 0.3, 0.4]);
        // The next reading moves the targets; the animation tick carries
        // `current` towards them
        push(&mut data, &items, &reading(50.0));
        assert_eq!(targets(&data), vec![0.5, 0.6, 0.7, 0.8]);
        assert_eq!(currents(&data), vec![0.1, 0.2, 0.3, 0.4]);
    }

    #[test]
    fn an_item_with_animation_off_steps_to_each_reading() {
        let items = core_item(false);
        let mut data = ComboDisplayData::default();
        push(&mut data, &items, &reading(10.0));
        push(&mut data, &items, &reading(50.0));
        assert_eq!(currents(&data), vec![0.5, 0.6, 0.7, 0.8]);
    }
}
