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
use serde::{Deserialize, Serialize};
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
use crate::displayers::combo_utils::{self, AnimatedValue};
use crate::ui::arc_display::render_arc;
use crate::ui::graph_display::DataPoint;
use crate::ui::lcars_display::{
    calculate_item_layouts_with_orientation, get_content_bounds, render_content_background,
    render_content_bar, render_content_core_bars, render_content_graph, render_content_static,
    render_content_text, render_divider, render_lcars_frame, ContentDisplayType, ContentItemConfig,
    LcarsFrameConfig, SplitOrientation,
};
use crate::ui::speedometer_display::render_speedometer_with_theme;

/// Full LCARS display configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LcarsDisplayConfig {
    /// Frame and sidebar configuration
    #[serde(default)]
    pub frame: LcarsFrameConfig,

    /// Animation settings
    #[serde(default = "default_animation_enabled")]
    pub animation_enabled: bool,
    #[serde(default = "default_animation_speed")]
    pub animation_speed: f64,
}

fn default_animation_enabled() -> bool {
    true
}

fn default_animation_speed() -> f64 {
    8.0 // Speed of lerp
}

impl Default for LcarsDisplayConfig {
    fn default() -> Self {
        Self {
            frame: LcarsFrameConfig::default(),
            animation_enabled: default_animation_enabled(),
            animation_speed: default_animation_speed(),
        }
    }
}

/// Cached frame rendering for LCARS (stored in draw closure, not Send/Sync)
struct LcarsFrameCache {
    /// Cached frame surface (LCARS sidebar, content background, dividers)
    surface: cairo::ImageSurface,
    /// Size cache was rendered at
    width: i32,
    height: i32,
    /// Config version when cache was created
    config_version: u64,
    /// Content bounds from frame render
    content_bounds: (f64, f64, f64, f64),
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
        cr: &Context,
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
    ) -> Result<(), cairo::Error> {
        let count = cached_prefixes.len() as u32;
        if count == 0 || w <= 0.0 || h <= 0.0 {
            return Ok(());
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

        // Pre-allocate bar_key buffer to avoid repeated allocations
        let mut bar_key_buf = String::with_capacity(32);

        // Draw each item
        for (i, &(item_x, item_y, item_w, item_h)) in layouts.iter().enumerate() {
            let prefix = &cached_prefixes[i];
            let item_data = combo_utils::get_item_data(values, prefix);
            let slot_values = combo_utils::get_slot_values(values, prefix);

            // Get item config (or use default)
            let item_config = config
                .frame
                .content_items
                .get(prefix)
                .cloned()
                .unwrap_or_default();

            // Get animated percent - reuse buffer for bar_key
            bar_key_buf.clear();
            bar_key_buf.push_str(prefix);
            bar_key_buf.push_str("_bar");
            let animated_percent = bar_values
                .get(bar_key_buf.as_str())
                .map(|av| av.current)
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
                        &config.frame.theme,
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
                        &config.frame.theme,
                        &item_data,
                        Some(&slot_values),
                    )?;
                }
                ContentDisplayType::Graph => {
                    // Get graph history for this slot
                    let graph_key = format!("{}_graph", prefix);
                    let empty_history = VecDeque::new();
                    let history = graph_history.get(&graph_key).unwrap_or(&empty_history);

                    if let Err(e) = render_content_graph(
                        cr,
                        item_x,
                        item_y,
                        item_w,
                        item_h,
                        &item_config.graph_config,
                        &config.frame.theme,
                        history,
                        &slot_values,
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
                            &config.frame.theme,
                            &item_data,
                            Some(&slot_values),
                        )?;
                    }
                }
                ContentDisplayType::LevelBar => {
                    // For now, render as text (level bar can be added later)
                    render_content_text(
                        cr,
                        item_x,
                        item_y,
                        item_w,
                        item_h,
                        &item_config.bar_config,
                        &config.frame.theme,
                        &item_data,
                        Some(&slot_values),
                    )?;
                }
                ContentDisplayType::CoreBars => {
                    // Use animated core values if available, otherwise fall back to raw values
                    let core_bars_config = &item_config.core_bars_config;
                    let core_values: Vec<f64> = if let Some(animated) = core_bar_values.get(prefix)
                    {
                        // Use animated current values
                        animated.iter().map(|av| av.current).collect()
                    } else {
                        // Fall back to raw values (for first frame before animation starts)
                        let capacity = core_bars_config
                            .end_core
                            .saturating_sub(core_bars_config.start_core)
                            + 1;
                        let mut raw_values: Vec<f64> = Vec::with_capacity(capacity);
                        for core_idx in core_bars_config.start_core..=core_bars_config.end_core {
                            let core_key = format!("{}_core{}_usage", prefix, core_idx);
                            let value = values
                                .get(&core_key)
                                .and_then(|v| v.as_f64())
                                .unwrap_or(0.0);
                            raw_values.push(value / 100.0);
                        }

                        // If no specific core values found, try to find any core values
                        if raw_values.is_empty() {
                            for core_idx in 0..128 {
                                let core_key = format!("{}_core{}_usage", prefix, core_idx);
                                if let Some(v) = values.get(&core_key).and_then(|v| v.as_f64()) {
                                    raw_values.push(v / 100.0);
                                } else {
                                    break;
                                }
                            }
                        }
                        raw_values
                    };

                    // Render core bars
                    render_content_core_bars(
                        cr,
                        item_x,
                        item_y,
                        item_w,
                        item_h,
                        core_bars_config,
                        &config.frame.theme,
                        &core_values,
                        Some(&slot_values),
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
                        &config.frame.theme,
                        Some(&slot_values),
                    )?;
                }
                ContentDisplayType::Arc => {
                    cr.save()?;
                    cr.translate(item_x, item_y);
                    render_arc(
                        cr,
                        &item_config.arc_config,
                        &config.frame.theme,
                        animated_percent,
                        &slot_values,
                        item_w,
                        item_h,
                    )?;
                    cr.restore()?;
                }
                ContentDisplayType::Speedometer => {
                    cr.save()?;
                    cr.translate(item_x, item_y);
                    if let Err(e) = render_speedometer_with_theme(
                        cr,
                        &item_config.speedometer_config,
                        animated_percent,
                        &slot_values,
                        item_w,
                        item_h,
                        &config.frame.theme,
                    ) {
                        log::warn!("Failed to render speedometer for {}: {}", prefix, e);
                    }
                    cr.restore()?;
                }
            }
        }

        Ok(())
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
        let drawing_area = DrawingArea::new();
        drawing_area.set_size_request(400, 300);

        // Frame cache lives in the draw closure (not Send/Sync required)
        let frame_cache: Rc<RefCell<Option<LcarsFrameCache>>> = Rc::new(RefCell::new(None));
        let frame_cache_clone = frame_cache.clone();

        // Set up draw function
        let data_clone = self.data.clone();
        // Counter for lock contention tracking
        let lock_fail_count = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
        let lock_fail_count_draw = lock_fail_count.clone();
        drawing_area.set_draw_func(move |_, cr, width, height| {
            // Use try_lock to avoid blocking GTK main thread if update is in progress
            let Ok(data) = data_clone.try_lock() else {
                let count = lock_fail_count_draw.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                if count.is_multiple_of(100) {
                    log::warn!("LCARS draw: try_lock failed {} times", count + 1);
                }
                // Lock contention - try to use cached frame to avoid flicker
                if let Some(cache) = frame_cache_clone.borrow().as_ref() {
                    if cache.width == width
                        && cache.height == height
                        && cr.set_source_surface(&cache.surface, 0.0, 0.0).is_ok()
                    {
                        cr.paint().ok();
                        return;
                    }
                }
                // No valid cache available - draw a solid background to avoid blank frame
                cr.set_source_rgba(0.05, 0.05, 0.1, 1.0);
                cr.paint().ok();
                return;
            };
            let w = width as f64;
            let h = height as f64;

            data.transform.apply(cr, w, h);

            // Check if frame cache is valid
                let cache_valid = frame_cache_clone.borrow().as_ref().is_some_and(|cache| {
                    cache.width == width
                        && cache.height == height
                        && cache.config_version == data.config_version
                });

                // Either use cached frame or render fresh
                let (content_bounds, group_layouts) = if cache_valid {
                    // Use cached frame
                    let cache_ref = frame_cache_clone.borrow();
                    let cache = cache_ref.as_ref().unwrap();
                    if let Err(e) = cr.set_source_surface(&cache.surface, 0.0, 0.0) {
                        log::debug!("Failed to set cached surface: {:?}", e);
                    }
                    cr.paint().ok();
                    // Clear source reference to allow cached surface to be deallocated
                    // when cache is invalidated (prevents memory leak)
                    cr.set_source_rgba(0.0, 0.0, 0.0, 0.0);
                    (cache.content_bounds, cache.group_layouts.clone())
                } else {
                    // Try to create cache surface
                    let cache_result =
                        cairo::ImageSurface::create(cairo::Format::ARgb32, width, height)
                            .ok()
                            .and_then(|surface| {
                                cairo::Context::new(&surface).ok().map(|ctx| (surface, ctx))
                            });

                    if let Some((surface, cache_cr)) = cache_result {
                        // Render frame to cache
                        if let Err(e) = render_lcars_frame(&cache_cr, &data.config.frame, w, h) {
                            log::debug!("LCARS frame render error: {}", e);
                        }

                        // Render content background to cache
                        if let Err(e) =
                            render_content_background(&cache_cr, &data.config.frame, w, h)
                        {
                            log::debug!("LCARS content background render error: {}", e);
                        }

                        // Get content bounds
                        let content_bounds = get_content_bounds(&data.config.frame, w, h);
                        let (content_x, content_y, content_w, content_h) = content_bounds;

                        // Calculate group layouts and render dividers to cache
                        let group_count = data.config.frame.group_item_counts.len();
                        let divider_config = &data.config.frame.divider_config;
                        let mut group_layouts = Vec::with_capacity(group_count);

                        if group_count == 0 {
                            // No groups
                        } else if group_count == 1 {
                            group_layouts.push((content_x, content_y, content_w, content_h));
                        } else {
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
                                    let available_w =
                                        content_w - num_dividers * total_divider_space;
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

                                        group_layouts
                                            .push((current_x, content_y, group_w, content_h));

                                        if group_idx < group_count - 1 {
                                            let divider_x =
                                                current_x + group_w + divider_config.spacing_before;
                                            let _ = render_divider(
                                                &cache_cr,
                                                divider_x,
                                                content_y,
                                                divider_config.width,
                                                content_h,
                                                divider_config,
                                                SplitOrientation::Vertical,
                                                &data.config.frame.theme,
                                            );
                                            current_x = divider_x
                                                + divider_config.width
                                                + divider_config.spacing_after;
                                        }
                                    }
                                }
                                SplitOrientation::Horizontal => {
                                    let available_h =
                                        content_h - num_dividers * total_divider_space;
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

                                        group_layouts
                                            .push((content_x, current_y, content_w, group_h));

                                        if group_idx < group_count - 1 {
                                            let divider_y =
                                                current_y + group_h + divider_config.spacing_before;
                                            let _ = render_divider(
                                                &cache_cr,
                                                content_x,
                                                divider_y,
                                                content_w,
                                                divider_config.width,
                                                divider_config,
                                                SplitOrientation::Horizontal,
                                                &data.config.frame.theme,
                                            );
                                            current_y = divider_y
                                                + divider_config.width
                                                + divider_config.spacing_after;
                                        }
                                    }
                                }
                            }
                        }

                        // Flush and store cache
                        drop(cache_cr);
                        surface.flush();

                        *frame_cache_clone.borrow_mut() = Some(LcarsFrameCache {
                            surface,
                            width,
                            height,
                            config_version: data.config_version,
                            content_bounds,
                            group_layouts: group_layouts.clone(),
                        });

                        // Paint cached surface
                        let cache_ref = frame_cache_clone.borrow();
                        let cache = cache_ref.as_ref().unwrap();
                        if let Err(e) = cr.set_source_surface(&cache.surface, 0.0, 0.0) {
                            log::debug!("Failed to set cached surface: {:?}", e);
                        }
                        cr.paint().ok();
                        // Clear source reference to allow cached surface to be deallocated
                        // when cache is invalidated (prevents memory leak)
                        cr.set_source_rgba(0.0, 0.0, 0.0, 0.0);

                        (content_bounds, group_layouts)
                    } else {
                        // Cache creation failed - render directly (fallback)
                        if let Err(e) = render_lcars_frame(cr, &data.config.frame, w, h) {
                            log::warn!("LCARS frame render error: {}", e);
                        }
                        if let Err(e) = render_content_background(cr, &data.config.frame, w, h) {
                            log::warn!("LCARS content background render error: {}", e);
                        }

                        let content_bounds = get_content_bounds(&data.config.frame, w, h);
                        let (content_x, content_y, content_w, content_h) = content_bounds;

                        // Calculate group layouts (without caching)
                        let group_count = data.config.frame.group_item_counts.len();
                        let divider_config = &data.config.frame.divider_config;
                        let mut group_layouts = Vec::with_capacity(group_count);

                        if group_count == 1 {
                            group_layouts.push((content_x, content_y, content_w, content_h));
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
                                    let available_w =
                                        content_w - num_dividers * total_divider_space;
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
                                        group_layouts
                                            .push((current_x, content_y, group_w, content_h));

                                        if group_idx < group_count - 1 {
                                            let divider_x =
                                                current_x + group_w + divider_config.spacing_before;
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
                                            current_x = divider_x
                                                + divider_config.width
                                                + divider_config.spacing_after;
                                        }
                                    }
                                }
                                SplitOrientation::Horizontal => {
                                    let available_h =
                                        content_h - num_dividers * total_divider_space;
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
                                        group_layouts
                                            .push((content_x, current_y, content_w, group_h));

                                        if group_idx < group_count - 1 {
                                            let divider_y =
                                                current_y + group_h + divider_config.spacing_before;
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
                                            current_y = divider_y
                                                + divider_config.width
                                                + divider_config.spacing_after;
                                        }
                                    }
                                }
                            }
                        }

                        (content_bounds, group_layouts)
                    }
                };

                let (content_x, content_y, content_w, content_h) = content_bounds;

                // Clip to content area for dynamic content
                cr.save().ok();
                cr.rectangle(content_x, content_y, content_w, content_h);
                cr.clip();

                // Draw dynamic content items for each group
                for (group_idx, &(gx, gy, gw, gh)) in group_layouts.iter().enumerate() {
                    let prefixes = data
                        .group_prefixes
                        .get(group_idx)
                        .map(|v| v.as_slice())
                        .unwrap_or(&[]);
                    let _ = Self::draw_content_items(
                        cr,
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

            cr.restore().ok();
            data.transform.restore(cr);
        });

        // Register with global animation manager for smooth animations
        let data_for_animation = self.data.clone();
        let lock_fail_count_anim = lock_fail_count;
        register_animation(drawing_area.downgrade(), move || {
            // Use try_lock to avoid blocking UI thread if lock is held
            if let Ok(mut data) = data_for_animation.try_lock() {
                // Periodic diagnostics (every ~5 seconds at 60fps)
                static TICK_COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
                let ticks = TICK_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                if ticks.is_multiple_of(300) {
                    log::info!(
                        "LCARS anim: bar_values={}, core_bar_values={}, graph_history={}, group_prefixes={}",
                        data.bar_values.len(),
                        data.core_bar_values.len(),
                        data.graph_history.len(),
                        data.group_prefixes.len()
                    );
                }
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

                    if has_bar_animations || has_core_animations {
                        let now = Instant::now();
                        let elapsed = now.duration_since(data.last_update).as_secs_f64();
                        data.last_update = now;

                        let speed = data.config.animation_speed;

                        // Animate bar values
                        if has_bar_animations {
                            for (_key, anim) in data.bar_values.iter_mut() {
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

                        // Animate core bar values
                        if has_core_animations {
                            for (_key, core_anims) in data.core_bar_values.iter_mut() {
                                for anim in core_anims.iter_mut() {
                                    if (anim.current - anim.target).abs() > ANIMATION_SNAP_THRESHOLD
                                    {
                                        let delta = (anim.target - anim.current) * speed * elapsed;
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
                let count = lock_fail_count_anim.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                if count.is_multiple_of(100) {
                    log::warn!("LCARS anim: try_lock failed {} times", count + 1);
                }
                false
            }
        });

        drawing_area.upcast()
    }

    fn update_data(&mut self, data: &HashMap<String, Value>) {
        let start = std::time::Instant::now();
        if let Ok(mut display_data) = self.data.lock() {
            let animation_enabled = display_data.config.animation_enabled;
            let timestamp = display_data.graph_start_time.elapsed().as_secs_f64();

            // Convert group_item_counts to usize for generate_prefixes
            let group_item_counts: Vec<usize> = display_data
                .config
                .frame
                .group_item_counts
                .iter()
                .map(|&x| x as usize)
                .collect();

            // Generate prefixes and filter values using optimized utils
            let prefixes = combo_utils::generate_prefixes(&group_item_counts);
            combo_utils::filter_values_by_prefixes_into(data, &prefixes, &mut display_data.values);

            // Cache per-group prefixes for efficient draw loop (avoids format! allocations)
            // Only rebuild if group structure changed
            let needs_rebuild = display_data.group_prefixes.len() != group_item_counts.len()
                || display_data
                    .group_prefixes
                    .iter()
                    .zip(group_item_counts.iter())
                    .any(|(cached, &count)| cached.len() != count);
            if needs_rebuild {
                display_data.group_prefixes = group_item_counts
                    .iter()
                    .enumerate()
                    .map(|(group_idx, &count)| {
                        let group_num = group_idx + 1;
                        (1..=count)
                            .map(|item_idx| format!("group{}_{}", group_num, item_idx))
                            .collect()
                    })
                    .collect();
            }

            // Update each item
            for prefix in &prefixes {
                let item_data = combo_utils::get_item_data(data, prefix);
                combo_utils::update_bar_animation(
                    &mut display_data.bar_values,
                    prefix,
                    item_data.percent(),
                    animation_enabled,
                );

                // Get item config and extract what we need before mutable borrows
                let default_config = ContentItemConfig::default();
                let item_config = display_data
                    .config
                    .frame
                    .content_items
                    .get(prefix)
                    .unwrap_or(&default_config);
                let display_as = item_config.display_as;
                let graph_max_points = item_config.graph_config.max_data_points;
                let core_bars_config = item_config.core_bars_config.clone();

                match display_as {
                    ContentDisplayType::Graph => {
                        combo_utils::update_graph_history(
                            &mut display_data.graph_history,
                            prefix,
                            item_data.numerical_value,
                            timestamp,
                            graph_max_points,
                        );
                    }
                    ContentDisplayType::CoreBars => {
                        combo_utils::update_core_bars(
                            data,
                            &mut display_data.core_bar_values,
                            prefix,
                            &core_bars_config,
                            animation_enabled,
                        );
                    }
                    _ => {}
                }
            }

            // Clean up stale animation entries
            combo_utils::cleanup_bar_values(&mut display_data.bar_values, &prefixes);
            combo_utils::cleanup_core_bar_values(&mut display_data.core_bar_values, &prefixes);
            combo_utils::cleanup_graph_history(&mut display_data.graph_history, &prefixes);

            // Extract transform from values
            display_data.transform = PanelTransform::from_values(data);

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
