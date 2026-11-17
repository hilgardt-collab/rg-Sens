//! LCARS Combo Displayer - A Star Trek-inspired interface for displaying multiple data sources
//!
//! This displayer provides a sophisticated LCARS-style interface with:
//! - A sidebar with colored segments and labels
//! - Optional top/bottom extensions with headers
//! - A content area that can display bars, text, or graphs
//! - Support for split-screen layouts
//! - Smooth animation for bar values

use anyhow::Result;
use cairo::Context;
use gtk4::{prelude::*, DrawingArea, Widget};
use serde_json::Value;
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::core::{
    register_animation, ConfigOption, ConfigSchema, Displayer, PanelTransform,
    ANIMATION_SNAP_THRESHOLD,
};
use crate::displayers::combo_displayer_base::{
    content_item_key, core_bar_item_values, draw_content_item, shown_glide,
};
use crate::displayers::combo_utils::{self, AnimatedValue};
use crate::displayers::layer_canvas::{LayerCache, LayerCanvas, LayerKey, LayerKind, LayerSink};
use crate::ui::graph_display::DataPoint;
use crate::ui::group_title::render_group_title;
use crate::ui::lcars_display::{
    calculate_item_layouts_with_orientation, get_content_bounds, render_content_background,
    render_divider, render_lcars_frame, ContentDisplayType, ContentItemConfig, SplitOrientation,
};

pub use rg_sens_types::display_configs::themed_configs::LcarsDisplayConfig;

/// Cached frame layout data for LCARS (stored in draw closure, not Send/Sync).
/// Only stores computed geometry; the picture itself is kept as render nodes
/// (see `layer_canvas`).
struct LcarsLayoutCache {
    /// Size the cache was computed at
    width: i32,
    height: i32,
    /// Config version when cache was created
    config_version: u64,
    /// Pre-calculated group layouts: Vec of (x, y, w, h) for each group
    group_layouts: Vec<(f64, f64, f64, f64)>,
}

/// Internal display data
struct DisplayData {
    config: LcarsDisplayConfig,
    values: HashMap<String, Value>,
    bar_values: HashMap<String, AnimatedValue>,
    /// Animated values for CoreBars content items (keyed by prefix, e.g., "group1_1")
    core_bar_values: HashMap<String, Vec<AnimatedValue>>,
    graph_history: HashMap<String, VecDeque<DataPoint>>,
    graph_start_time: Instant,
    last_update: Instant,
    transform: PanelTransform,
    dirty: bool,
    /// Cached prefixes per group to avoid format! allocations in draw loop
    /// group_prefixes[0] = ["group1_1", "group1_2", ...], etc.
    group_prefixes: Vec<Vec<String>>,
    /// Config version counter for cache invalidation
    config_version: u64,
}

impl Default for DisplayData {
    fn default() -> Self {
        Self {
            config: LcarsDisplayConfig::default(),
            values: HashMap::new(),
            bar_values: HashMap::new(),
            core_bar_values: HashMap::new(),
            graph_history: HashMap::new(),
            graph_start_time: Instant::now(),
            last_update: Instant::now(),
            transform: PanelTransform::default(),
            dirty: true,
            group_prefixes: Vec::new(),
            config_version: 0,
        }
    }
}

/// LCARS Combo Displayer
pub struct LcarsComboDisplayer {
    id: String,
    name: String,
    data: Arc<Mutex<DisplayData>>,
}

impl LcarsComboDisplayer {
    pub fn new() -> Self {
        Self {
            id: "lcars".to_string(),
            name: "LCARS".to_string(),
            data: Arc::new(Mutex::new(DisplayData::default())),
        }
    }

    /// Draw content items in a given area
    /// `cached_prefixes` contains pre-computed prefixes like ["group1_1", "group1_2", ...]
    #[allow(clippy::too_many_arguments)]
    fn draw_content_items(
        sink: &mut LayerSink,
        config_version: u64,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        cached_prefixes: &[String],
        group_idx: usize,
        config: &LcarsDisplayConfig,
        values: &HashMap<String, Value>,
        bar_values: &HashMap<String, AnimatedValue>,
        core_bar_values: &HashMap<String, Vec<AnimatedValue>>,
        graph_history: &HashMap<String, VecDeque<DataPoint>>,
    ) {
        let count = cached_prefixes.len() as u32;
        if count == 0 || w <= 0.0 || h <= 0.0 {
            return;
        }

        // Get item orientation for this group (default to layout orientation)
        let item_orientation = config
            .frame
            .group_item_orientations
            .get(group_idx)
            .copied()
            .unwrap_or(config.frame.layout_orientation);

        // Determine fixed sizes for items that need them
        // Items with auto_height=false, Graph, or LevelBar display type get fixed sizes
        let mut fixed_sizes: HashMap<usize, f64> = HashMap::new();
        for (i, prefix) in cached_prefixes.iter().enumerate() {
            let item_config = config.frame.content_items.get(prefix);
            if let Some(cfg) = item_config {
                // Use fixed size if auto_height is disabled or for Graph/LevelBar display types
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

        // Calculate layouts with the specified orientation
        let layouts = calculate_item_layouts_with_orientation(
            x,
            y,
            w,
            h,
            count,
            config.frame.item_spacing,
            &fixed_sizes,
            item_orientation,
        );

        // Pre-allocate key buffer to avoid repeated allocations (reused for
        // bar and graph key lookups)
        let mut key_buf = String::with_capacity(32);
        // Hoisted default so per-item config lookups can borrow instead of clone
        let default_item_config = ContentItemConfig::default();
        let empty_history = VecDeque::new();

        // Each item is a layer of its own: one that reads the same inputs as
        // on the frame before keeps its render node and costs nothing.
        for (i, &(item_x, item_y, item_w, item_h)) in layouts.iter().enumerate() {
            let prefix = &cached_prefixes[i];
            let item_data = combo_utils::get_item_data(values, prefix);
            let slot_values = combo_utils::get_slot_values(values, prefix);

            // Get item config (or use default) — borrow, don't clone per frame
            let item_config = config
                .frame
                .content_items
                .get(prefix)
                .unwrap_or(&default_item_config);
            let display_as = item_config.display_as;

            // Get animated percent - reuse buffer for bar_key
            key_buf.clear();
            key_buf.push_str(prefix);
            key_buf.push_str("_bar");
            let animated_percent = bar_values
                .get(key_buf.as_str())
                .map(|av| av.current)
                .unwrap_or_else(|| item_data.percent());
            let animated_percent = shown_glide(item_config, animated_percent);

            let history = if display_as == ContentDisplayType::Graph {
                key_buf.clear();
                key_buf.push_str(prefix);
                key_buf.push_str("_graph");
                graph_history
                    .get(key_buf.as_str())
                    .unwrap_or(&empty_history)
            } else {
                &empty_history
            };

            let core_values = if display_as == ContentDisplayType::CoreBars {
                core_bar_item_values(prefix, item_config, values, core_bar_values)
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
                    if let Err(e) = draw_content_item(
                        cr,
                        rect,
                        prefix,
                        item_config,
                        &config.frame.theme,
                        &item_data,
                        &slot_values,
                        animated_percent,
                        history,
                        &core_values,
                    ) {
                        log::debug!("LCARS item {} render error: {}", prefix, e);
                    }
                    0.0
                },
            );
        }
    }

    /// Render frame, dividers and content items to any Cairo context, for
    /// example an offscreen `ImageSurface` (tests, previews). The GTK widget
    /// path is `create_widget`; this shares its drawing code.
    pub fn render_to_context(&self, cr: &Context, width: i32, height: i32) {
        if let Ok(data) = self.data.lock() {
            let cache = Rc::new(RefCell::new(None));
            Self::render_frame_and_content(cr, width, height, &data, &cache);
        }
    }

    /// Render the complete panel (frame and content) to one Cairo context,
    /// under the panel's transform. Every layer is drawn; the widget goes
    /// through `render_layers` with a cache instead.
    fn render_frame_and_content(
        cr: &Context,
        width: i32,
        height: i32,
        data: &DisplayData,
        layout_cache: &Rc<RefCell<Option<LcarsLayoutCache>>>,
    ) {
        data.transform.apply(cr, width as f64, height as f64);
        let mut sink = LayerSink::direct(cr);
        Self::render_layers(&mut sink, width, height, data, layout_cache);
        data.transform.restore(cr);
    }

    /// The panel as layers (see `layer_canvas`): the frame with the content
    /// background and the dividers, each enabled group title, each content
    /// item. Group layouts are cached to avoid redundant computation.
    fn render_layers(
        sink: &mut LayerSink,
        width: i32,
        height: i32,
        data: &DisplayData,
        layout_cache: &Rc<RefCell<Option<LcarsLayoutCache>>>,
    ) {
        let w = width as f64;
        let h = height as f64;

        let content_bounds = get_content_bounds(&data.config.frame, w, h);

        // Use cached group layouts if size and config haven't changed
        let cached_layouts = {
            let cache_ref = layout_cache.borrow();
            cache_ref
                .as_ref()
                .filter(|c| {
                    c.width == width
                        && c.height == height
                        && c.config_version == data.config_version
                })
                .map(|c| c.group_layouts.clone())
        };

        let group_layouts = cached_layouts.unwrap_or_else(|| {
            let (content_x, content_y, content_w, content_h) = content_bounds;
            let group_count = data.config.frame.group_item_counts.len();
            let divider_config = &data.config.frame.divider_config;
            let mut layouts = Vec::with_capacity(group_count);

            if group_count == 1 {
                layouts.push((content_x, content_y, content_w, content_h));
            } else if group_count > 1 {
                let total_divider_space = divider_config.width
                    + divider_config.spacing_before
                    + divider_config.spacing_after;
                let num_dividers = (group_count - 1) as f64;

                let total_weight: f64 = (0..group_count)
                    .map(|i| {
                        data.config
                            .frame
                            .group_size_weights
                            .get(i)
                            .copied()
                            .unwrap_or(1.0)
                    })
                    .sum();
                let total_weight = if total_weight <= 0.0 {
                    1.0
                } else {
                    total_weight
                };

                match data.config.frame.layout_orientation {
                    SplitOrientation::Vertical => {
                        let available_w = content_w - num_dividers * total_divider_space;
                        let mut current_x = content_x;

                        for group_idx in 0..group_count {
                            let weight = data
                                .config
                                .frame
                                .group_size_weights
                                .get(group_idx)
                                .copied()
                                .unwrap_or(1.0);
                            let group_w = (weight / total_weight) * available_w;
                            layouts.push((current_x, content_y, group_w, content_h));
                            current_x += group_w + total_divider_space;
                        }
                    }
                    SplitOrientation::Horizontal => {
                        let available_h = content_h - num_dividers * total_divider_space;
                        let mut current_y = content_y;

                        for group_idx in 0..group_count {
                            let weight = data
                                .config
                                .frame
                                .group_size_weights
                                .get(group_idx)
                                .copied()
                                .unwrap_or(1.0);
                            let group_h = (weight / total_weight) * available_h;
                            layouts.push((content_x, current_y, content_w, group_h));
                            current_y += group_h + total_divider_space;
                        }
                    }
                }
            }

            *layout_cache.borrow_mut() = Some(LcarsLayoutCache {
                width,
                height,
                config_version: data.config_version,
                group_layouts: layouts.clone(),
            });

            layouts
        });

        // The frame, the content background and the dividers: one layer that
        // only the config and the size decide
        let (content_x, content_y, content_w, content_h) = content_bounds;
        let group_count = data.config.frame.group_item_counts.len();
        let divider_config = &data.config.frame.divider_config;
        let mut chrome_key = LayerKey::new();
        chrome_key.u64(data.config_version).f64(w).f64(h);
        sink.layer(
            (LayerKind::Chrome, 0, 0),
            chrome_key.finish(),
            None,
            &|cr| {
                if let Err(e) = render_lcars_frame(cr, &data.config.frame, w, h) {
                    log::debug!("LCARS frame render error: {}", e);
                }
                if let Err(e) = render_content_background(cr, &data.config.frame, w, h) {
                    log::debug!("LCARS content background render error: {}", e);
                }
                if group_count > 1 {
                    for &(gx, gy, gw, gh) in group_layouts.iter().take(group_count - 1) {
                        match data.config.frame.layout_orientation {
                            SplitOrientation::Vertical => {
                                let divider_x = gx + gw + divider_config.spacing_before;
                                let _ = render_divider(
                                    cr,
                                    divider_x,
                                    content_y,
                                    divider_config.width,
                                    content_h,
                                    divider_config,
                                    SplitOrientation::Vertical,
                                    &data.config.frame.theme,
                                );
                            }
                            SplitOrientation::Horizontal => {
                                let divider_y = gy + gh + divider_config.spacing_before;
                                let _ = render_divider(
                                    cr,
                                    content_x,
                                    divider_y,
                                    content_w,
                                    divider_config.width,
                                    divider_config,
                                    SplitOrientation::Horizontal,
                                    &data.config.frame.theme,
                                );
                            }
                        }
                    }
                }

                0.0
            },
        );

        // Clip to content area for dynamic content
        sink.set_clip(Some((content_x, content_y, content_w, content_h)));

        // Draw dynamic content items for each group
        for (group_idx, &(gx, gy, gw, gh)) in group_layouts.iter().enumerate() {
            let prefixes = data
                .group_prefixes
                .get(group_idx)
                .map(|v| v.as_slice())
                .unwrap_or(&[]);
            // An enabled title takes a band at the top of the group's
            // rectangle; the items get the rest (as the generic path does).
            let (gy, gh) = match data.config.frame.group_titles.get(group_idx) {
                Some(title) if title.enabled => {
                    let mut key = LayerKey::new();
                    key.u64(data.config_version).rect((gx, gy, gw, gh));
                    let used = sink.layer(
                        (LayerKind::Title, group_idx as u32, 0),
                        key.finish(),
                        None,
                        &|cr| {
                            render_group_title(cr, gx, gy, gw, gh, title, &data.config.frame.theme)
                                .unwrap_or(0.0)
                        },
                    );
                    (gy + used, gh - used)
                }
                _ => (gy, gh),
            };
            Self::draw_content_items(
                sink,
                data.config_version,
                gx,
                gy,
                gw,
                gh,
                prefixes,
                group_idx,
                &data.config,
                &data.values,
                &data.bar_values,
                &data.core_bar_values,
                &data.graph_history,
            );
        }

        sink.set_clip(None);
    }
}

impl Default for LcarsComboDisplayer {
    fn default() -> Self {
        Self::new()
    }
}

impl Displayer for LcarsComboDisplayer {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn create_widget(&self) -> Widget {
        let canvas = LayerCanvas::new();
        canvas.set_size_request(400, 300);

        // Layout and layer caches live in the draw closure (not Send/Sync required)
        let layout_cache: Rc<RefCell<Option<LcarsLayoutCache>>> = Rc::new(RefCell::new(None));
        let layers = Rc::new(RefCell::new(LayerCache::default()));

        // The panel is drawn as layers, so that a frame on which one item
        // moved records, rasterises and uploads that item and nothing else
        // (see `layer_canvas`). A poisoned lock draws nothing (transparent).
        let data_clone = self.data.clone();
        canvas.set_snapshot_func(move |snapshot, width, height| {
            let Ok(data) = data_clone.lock() else {
                return;
            };
            let mut layers = layers.borrow_mut();
            let mut sink = LayerSink::cached(
                snapshot,
                &mut layers,
                width as f64,
                height as f64,
                &data.transform,
            );
            Self::render_layers(&mut sink, width, height, &data, &layout_cache);
            sink.finish();
        });
        let drawing_area: DrawingArea = canvas.upcast();

        // Register with global animation manager for smooth animations
        let data_for_animation = self.data.clone();
        register_animation(drawing_area.downgrade(), move || {
            // Use try_lock to avoid blocking UI thread if lock is held
            if let Ok(mut data) = data_for_animation.try_lock() {
                let mut redraw = data.dirty;
                if data.dirty {
                    data.dirty = false;
                }

                // Update animation state - with early exit optimization
                if data.config.animation_enabled {
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

                    // Refresh the timestamp on EVERY tick, not just while
                    // animating: the first animating frame after idle must not
                    // see the stale `elapsed` since the previous animation
                    // converged (with speed 8.0 that overshot the target 4-8x).
                    let now = Instant::now();
                    let elapsed = now.duration_since(data.last_update).as_secs_f64();
                    data.last_update = now;

                    if has_bar_animations || has_core_animations {
                        let speed = data.config.animation_speed;
                        // Cap the step so a long frame lands at most exactly on
                        // the target, never past it
                        let step = (speed * elapsed).min(1.0);

                        // Animate bar values
                        if has_bar_animations {
                            for (_key, anim) in data.bar_values.iter_mut() {
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

                        // Animate core bar values
                        if has_core_animations {
                            for (_key, core_anims) in data.core_bar_values.iter_mut() {
                                for anim in core_anims.iter_mut() {
                                    if (anim.current - anim.target).abs() > ANIMATION_SNAP_THRESHOLD
                                    {
                                        let delta = (anim.target - anim.current) * step;
                                        anim.current += delta;

                                        if (anim.current - anim.target).abs()
                                            < ANIMATION_SNAP_THRESHOLD
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

        drawing_area.upcast()
    }

    fn update_data(&mut self, data: &HashMap<String, Value>) {
        let start = std::time::Instant::now();

        // Phase 1: Brief lock to extract config
        let config_snapshot = {
            let Ok(display_data) = self.data.lock() else {
                return;
            };
            let group_item_counts: Vec<usize> = display_data
                .config
                .frame
                .group_item_counts
                .iter()
                .map(|&x| x as usize)
                .collect();
            // Shape of the cached group_prefixes (lengths only) so phase 2 can
            // decide whether a rebuild is needed without holding the lock
            let group_prefixes_shape: Vec<usize> = display_data
                .group_prefixes
                .iter()
                .map(|v| v.len())
                .collect();
            (
                display_data.config.animation_enabled,
                display_data.graph_start_time.elapsed().as_secs_f64(),
                group_item_counts,
                display_data.config.frame.content_items.clone(),
                group_prefixes_shape,
            )
        };
        let (animation_enabled, timestamp, group_item_counts, content_items, group_prefixes_shape) =
            config_snapshot;

        // Phase 2: Expensive pre-computation (no lock held)
        let prefixes = combo_utils::generate_prefixes(&group_item_counts);
        let prefix_set: std::collections::HashSet<String> = prefixes.iter().cloned().collect();
        let mut filtered_values = HashMap::with_capacity(prefixes.len() * 8);
        combo_utils::filter_values_with_owned_prefix_set(data, &prefix_set, &mut filtered_values);

        // Pre-compute group prefixes only if the structure actually changed —
        // rebuilding the full nested string Vecs on every update just to
        // discard them wastes allocations
        let needs_prefix_rebuild = group_prefixes_shape.len() != group_item_counts.len()
            || group_prefixes_shape
                .iter()
                .zip(group_item_counts.iter())
                .any(|(&cached, &count)| cached != count);
        let group_prefixes: Option<Vec<Vec<String>>> = needs_prefix_rebuild.then(|| {
            group_item_counts
                .iter()
                .enumerate()
                .map(|(group_idx, &count)| {
                    let group_num = group_idx + 1;
                    (1..=count)
                        .map(|item_idx| format!("group{}_{}", group_num, item_idx))
                        .collect()
                })
                .collect()
        });

        // Pre-extract per-item data
        struct ItemUpdate {
            target_percent: f64,
            numerical_value: f64,
            display_as: ContentDisplayType,
            graph_max_points: usize,
            core_bars_config: crate::ui::core_bars_display::CoreBarsConfig,
        }
        let item_updates: Vec<ItemUpdate> = prefixes
            .iter()
            .map(|prefix| {
                let item_data = combo_utils::get_item_data(data, prefix);
                let default_config = ContentItemConfig::default();
                let item_config = content_items.get(prefix).unwrap_or(&default_config);
                ItemUpdate {
                    target_percent: item_data.percent(),
                    numerical_value: item_data.numerical_value,
                    display_as: item_config.display_as,
                    graph_max_points: item_config.graph_config.max_data_points,
                    core_bars_config: item_config.core_bars_config.clone(),
                }
            })
            .collect();

        let transform = PanelTransform::from_values(data);

        // Phase 3: Brief lock to apply results
        if let Ok(mut display_data) = self.data.lock() {
            display_data.values = filtered_values;

            // Only update group_prefixes if structure changed (built in phase 2)
            if let Some(new_group_prefixes) = group_prefixes {
                display_data.group_prefixes = new_group_prefixes;
            }

            // Apply per-item animation updates (quick math)
            for (i, update) in item_updates.iter().enumerate() {
                let prefix = &prefixes[i];
                combo_utils::update_bar_animation(
                    &mut display_data.bar_values,
                    prefix,
                    update.target_percent,
                    animation_enabled,
                );

                match update.display_as {
                    ContentDisplayType::Graph => {
                        combo_utils::update_graph_history(
                            &mut display_data.graph_history,
                            prefix,
                            update.numerical_value,
                            timestamp,
                            update.graph_max_points,
                        );
                    }
                    ContentDisplayType::CoreBars => {
                        combo_utils::update_core_bars(
                            data,
                            &mut display_data.core_bar_values,
                            prefix,
                            &update.core_bars_config,
                            animation_enabled,
                        );
                    }
                    _ => {}
                }
            }

            // Clean up stale animation entries (builds the prefix set once).
            // Reborrow through the guard so the field borrows can be split.
            let dd = &mut *display_data;
            combo_utils::cleanup_all_animation_state(
                &mut dd.bar_values,
                &mut dd.core_bar_values,
                &mut dd.graph_history,
                &prefixes,
            );

            display_data.transform = transform;
            display_data.dirty = true;
        }

        // Log slow updates (>50ms is concerning)
        let elapsed = start.elapsed();
        if elapsed.as_millis() > 50 {
            log::warn!("LCARS update_data took {}ms", elapsed.as_millis());
        }
    }

    fn draw(&self, cr: &Context, width: f64, height: f64) -> Result<()> {
        // Use try_lock to avoid blocking the GTK main thread
        if let Ok(data) = self.data.try_lock() {
            data.transform.apply(cr, width, height);
            render_lcars_frame(cr, &data.config.frame, width, height)?;
            render_content_background(cr, &data.config.frame, width, height)?;
            data.transform.restore(cr);
        }
        Ok(())
    }

    fn config_schema(&self) -> ConfigSchema {
        ConfigSchema {
            options: vec![
                ConfigOption {
                    key: "sidebar_width".to_string(),
                    name: "Sidebar Width".to_string(),
                    description: "Width of the sidebar in pixels".to_string(),
                    value_type: "number".to_string(),
                    default: serde_json::json!(150.0),
                },
                ConfigOption {
                    key: "sidebar_position".to_string(),
                    name: "Sidebar Position".to_string(),
                    description: "Position of the sidebar (left or right)".to_string(),
                    value_type: "string".to_string(),
                    default: serde_json::json!("left"),
                },
                ConfigOption {
                    key: "segment_count".to_string(),
                    name: "Segment Count".to_string(),
                    description: "Number of sidebar segments".to_string(),
                    value_type: "number".to_string(),
                    default: serde_json::json!(3),
                },
                ConfigOption {
                    key: "animation_enabled".to_string(),
                    name: "Animation".to_string(),
                    description: "Enable smooth bar animations".to_string(),
                    value_type: "boolean".to_string(),
                    default: serde_json::json!(true),
                },
            ],
        }
    }

    fn apply_config(&mut self, config: &HashMap<String, Value>) -> Result<()> {
        // Check for full lcars_config first
        if let Some(lcars_config_value) = config.get("lcars_config") {
            if let Ok(mut lcars_config) =
                serde_json::from_value::<LcarsDisplayConfig>(lcars_config_value.clone())
            {
                // Migrate legacy primary/secondary format to groups format
                lcars_config.frame.migrate_legacy();
                if let Ok(mut display_data) = self.data.lock() {
                    display_data.config = lcars_config;
                    display_data.config_version = display_data.config_version.wrapping_add(1);
                }
                return Ok(());
            }
        }

        // Apply individual settings for backward compatibility
        if let Ok(mut display_data) = self.data.lock() {
            if let Some(sidebar_width) = config.get("sidebar_width").and_then(|v| v.as_f64()) {
                display_data.config.frame.sidebar_width = sidebar_width;
            }

            if let Some(animation_enabled) =
                config.get("animation_enabled").and_then(|v| v.as_bool())
            {
                display_data.config.animation_enabled = animation_enabled;
            }

            if let Some(segment_count) = config.get("segment_count").and_then(|v| v.as_u64()) {
                display_data.config.frame.segment_count = segment_count as u32;
            }

            display_data.config_version = display_data.config_version.wrapping_add(1);
        }

        Ok(())
    }

    fn needs_redraw(&self) -> bool {
        // Use try_lock to avoid blocking the GTK main thread
        self.data.try_lock().map(|data| data.dirty).unwrap_or(true)
    }

    fn get_typed_config(&self) -> Option<crate::core::DisplayerConfig> {
        // Use try_lock to avoid blocking the GTK main thread
        if let Ok(display_data) = self.data.try_lock() {
            Some(crate::core::DisplayerConfig::Lcars(
                display_data.config.clone(),
            ))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{DisplayerConfig, PanelData};
    use crate::displayers::layer_canvas::test_support::{same_picture, slot_values};

    /// The saved LCARS panel, taken through a sequence of updates, must look
    /// the same drawn as cached layers as it does drawn in one go (see the
    /// themed frames' test in `combo_generic`):
    /// `GDK_BACKEND=broadway BROADWAY_DISPLAY=:7 cargo test --lib lcars_layers_match -- --ignored`
    #[test]
    #[ignore = "needs a display"]
    fn lcars_layers_match_the_direct_drawing() {
        gtk4::init().expect("a display");
        let (width, height) = (480, 960);
        let mut panel: PanelData =
            serde_json::from_str(include_str!("../../examples/LcarS.json")).expect("panel file");
        panel.sync_group_structure();
        assert!(matches!(panel.displayer_config, DisplayerConfig::Lcars(_)));
        let mut displayer = LcarsComboDisplayer::new();
        displayer
            .apply_config_typed(&panel.displayer_config)
            .expect("config applies");
        let counts: Vec<usize> = {
            let data = displayer.data.lock().unwrap();
            let counts = &data.config.frame.group_item_counts;
            counts.iter().map(|&n| n as usize).collect()
        };
        let prefixes = combo_utils::generate_prefixes(&counts);
        assert!(!prefixes.is_empty());

        let mut layers = LayerCache::default();
        let layout_cache = Rc::new(RefCell::new(None));
        let direct_cache = Rc::new(RefCell::new(None));
        let mut redrawn = Vec::new();
        // step 0 draws everything; 1 and 2 change a third of the slots, the
        // glide half way on 2; 3 changes nothing; 4 scales the panel (every layer anew); 5
        // applies the config again
        for step in 0..6u64 {
            let mut values = slot_values(&prefixes, &|i| match step {
                0 => 0,
                1 => u64::from(i % 3 == 0),
                2 | 3 => u64::from(i % 3 == 0) + 2 * u64::from(i % 3 == 1),
                _ => 5,
            });
            if step >= 4 {
                values.insert("_panel_scale".to_string(), Value::from(0.9));
            }
            if step == 5 {
                displayer
                    .apply_config_typed(&panel.displayer_config)
                    .expect("config applies");
            }
            displayer.update_data(&values);
            if step == 2 {
                let mut data = displayer.data.lock().unwrap();
                let data = &mut *data;
                let bars = data.bar_values.values_mut();
                for value in bars.chain(data.core_bar_values.values_mut().flatten()) {
                    value.current += (value.target - value.current) * 0.5;
                }
            }
            let data = displayer.data.lock().unwrap();
            redrawn.push(same_picture(
                &format!("LCARS, step {step}"),
                step >= 4,
                width,
                height,
                &mut layers,
                &|cr| {
                    LcarsComboDisplayer::render_frame_and_content(
                        cr,
                        width,
                        height,
                        &data,
                        &direct_cache,
                    )
                },
                &|snapshot, layers| {
                    let mut sink = LayerSink::cached(
                        snapshot,
                        layers,
                        width as f64,
                        height as f64,
                        &data.transform,
                    );
                    LcarsComboDisplayer::render_layers(
                        &mut sink,
                        width,
                        height,
                        &data,
                        &layout_cache,
                    );
                    sink.finish();
                },
            ));
        }
        let all = redrawn[0];
        assert!(all > 2, "{redrawn:?}");
        assert!(redrawn[1] > 0 && redrawn[1] < all, "{redrawn:?}");
        // every update adds a point to a graph; nothing else has changed
        let graphs = {
            let data = displayer.data.lock().unwrap();
            let items = &data.config.frame.content_items;
            prefixes
                .iter()
                .filter_map(|prefix| items.get(prefix))
                .filter(|item| item.display_as == ContentDisplayType::Graph)
                .count() as u64
        };
        assert!(
            redrawn[3] <= graphs,
            "unchanged values redraw the graphs alone ({graphs}): {redrawn:?}"
        );
        assert_eq!(redrawn[5], all, "a new config redraws everything");
    }
}
