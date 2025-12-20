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
use gtk4::{glib, prelude::*, DrawingArea, Widget};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::core::{ConfigOption, ConfigSchema, Displayer, PanelTransform};
use crate::ui::graph_display::DataPoint;
use crate::ui::lcars_display::{
    get_content_bounds, render_content_background, render_content_bar, render_content_text,
    render_content_graph, render_content_core_bars, render_divider, render_lcars_frame, calculate_item_layouts,
    ContentDisplayType, ContentItemData, LcarsFrameConfig, SplitOrientation,
};

/// Animation state for a single bar value
#[derive(Debug, Clone)]
struct AnimatedValue {
    current: f64,
    target: f64,
    first_update: bool,
}

impl Default for AnimatedValue {
    fn default() -> Self {
        Self {
            current: 0.0,
            target: 0.0,
            first_update: true,
        }
    }
}

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

/// Internal display data
#[derive(Clone)]
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

    /// Get all values for a slot with the prefix stripped
    /// This extracts all values with keys like "group1_1_hour" -> "hour"
    fn get_slot_values(values: &HashMap<String, Value>, prefix: &str) -> HashMap<String, Value> {
        let prefix_with_underscore = format!("{}_", prefix);
        values.iter()
            .filter(|(k, _)| k.starts_with(&prefix_with_underscore))
            .map(|(k, v)| {
                let short_key = k.strip_prefix(&prefix_with_underscore).unwrap_or(k);
                (short_key.to_string(), v.clone())
            })
            .collect()
    }

    /// Get content item data from values with a given prefix
    fn get_item_data(values: &HashMap<String, Value>, prefix: &str) -> ContentItemData {
        let caption = values
            .get(&format!("{}_caption", prefix))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let value = values
            .get(&format!("{}_value", prefix))
            .map(|v| match v {
                Value::String(s) => s.clone(),
                Value::Number(n) => {
                    if let Some(f) = n.as_f64() {
                        format!("{:.1}", f)
                    } else {
                        n.to_string()
                    }
                }
                _ => v.to_string(),
            })
            .unwrap_or_default();

        let unit = values
            .get(&format!("{}_unit", prefix))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let numerical_value = values
            .get(&format!("{}_numerical_value", prefix))
            .or_else(|| values.get(&format!("{}_value", prefix)))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let min_value = values
            .get(&format!("{}_min_limit", prefix))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let max_value = values
            .get(&format!("{}_max_limit", prefix))
            .and_then(|v| v.as_f64())
            .unwrap_or(100.0);

        ContentItemData {
            caption,
            value,
            unit,
            numerical_value,
            min_value,
            max_value,
        }
    }

    /// Draw content items in a given area
    #[allow(clippy::too_many_arguments)]
    fn draw_content_items(
        cr: &Context,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        base_prefix: &str,
        count: u32,
        config: &LcarsDisplayConfig,
        values: &HashMap<String, Value>,
        bar_values: &HashMap<String, AnimatedValue>,
        core_bar_values: &HashMap<String, Vec<AnimatedValue>>,
        graph_history: &HashMap<String, VecDeque<DataPoint>>,
    ) -> Result<(), cairo::Error> {
        if count == 0 || w <= 0.0 || h <= 0.0 {
            return Ok(());
        }

        // Determine fixed heights for items that need them (like graphs)
        let mut fixed_heights: HashMap<usize, f64> = HashMap::new();
        for i in 0..count as usize {
            let prefix = format!("{}{}", base_prefix, i + 1);
            let item_config = config.frame.content_items.get(&prefix);
            if let Some(cfg) = item_config {
                if matches!(cfg.display_as, ContentDisplayType::Graph | ContentDisplayType::LevelBar) {
                    fixed_heights.insert(i, cfg.item_height);
                }
            }
        }

        // Calculate layouts
        let layouts = calculate_item_layouts(x, y, w, h, count, config.frame.item_spacing, &fixed_heights);

        // Draw each item
        for (i, &(item_x, item_y, item_w, item_h)) in layouts.iter().enumerate() {
            let prefix = format!("{}{}", base_prefix, i + 1);
            let item_data = Self::get_item_data(values, &prefix);
            let slot_values = Self::get_slot_values(values, &prefix);

            // Get item config (or use default)
            let item_config = config.frame.content_items.get(&prefix)
                .cloned()
                .unwrap_or_default();

            // Get animated percent
            let bar_key = format!("{}_bar", prefix);
            let animated_percent = bar_values
                .get(&bar_key)
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
                        &item_data,
                        Some(&slot_values),
                    )?;
                }
                ContentDisplayType::CoreBars => {
                    // Use animated core values if available, otherwise fall back to raw values
                    let core_bars_config = &item_config.core_bars_config;
                    let core_values: Vec<f64> = if let Some(animated) = core_bar_values.get(&prefix) {
                        // Use animated current values
                        animated.iter().map(|av| av.current).collect()
                    } else {
                        // Fall back to raw values (for first frame before animation starts)
                        let capacity = core_bars_config.end_core.saturating_sub(core_bars_config.start_core) + 1;
                        let mut raw_values: Vec<f64> = Vec::with_capacity(capacity);
                        for core_idx in core_bars_config.start_core..=core_bars_config.end_core {
                            let core_key = format!("{}_core{}_usage", prefix, core_idx);
                            let value = values.get(&core_key)
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
                        &core_values,
                    )?;
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

        // Set up draw function
        let data_clone = self.data.clone();
        drawing_area.set_draw_func(move |_, cr, width, height| {
            if let Ok(data) = data_clone.lock() {
                let w = width as f64;
                let h = height as f64;
                data.transform.apply(cr, w, h);

                // Draw the LCARS frame
                if let Err(e) = render_lcars_frame(cr, &data.config.frame, w, h) {
                    log::warn!("LCARS frame render error: {}", e);
                }

                // Draw content background
                if let Err(e) = render_content_background(cr, &data.config.frame, w, h) {
                    log::warn!("LCARS content background render error: {}", e);
                }

                // Get content bounds
                let (content_x, content_y, content_w, content_h) =
                    get_content_bounds(&data.config.frame, w, h);

                // Clip to content area
                cr.save().ok();
                cr.rectangle(content_x, content_y, content_w, content_h);
                cr.clip();

                // Draw content based on groups with dividers between them
                let group_count = data.config.frame.group_item_counts.len();
                let divider_config = &data.config.frame.divider_config;

                if group_count == 0 {
                    // No groups configured
                } else if group_count == 1 {
                    // Single group - no dividers needed
                    let item_count = data.config.frame.group_item_counts[0];
                    let _ = Self::draw_content_items(
                        cr,
                        content_x,
                        content_y,
                        content_w,
                        content_h,
                        "group1_",
                        item_count,
                        &data.config,
                        &data.values,
                        &data.bar_values,
                        &data.core_bar_values,
                        &data.graph_history,
                    );
                } else {
                    // Multiple groups - calculate space with dividers between each
                    let total_divider_space = divider_config.width
                        + divider_config.spacing_before
                        + divider_config.spacing_after;
                    let num_dividers = (group_count - 1) as f64;

                    // Calculate total weight for group sizing
                    let total_weight: f64 = (0..group_count)
                        .map(|i| data.config.frame.group_size_weights.get(i).copied().unwrap_or(1.0))
                        .sum();
                    let total_weight = if total_weight <= 0.0 { 1.0 } else { total_weight };

                    match data.config.frame.layout_orientation {
                        SplitOrientation::Vertical => {
                            // Groups arranged side by side (left to right)
                            let available_w = content_w - num_dividers * total_divider_space;

                            let mut current_x = content_x;
                            for (group_idx, &item_count) in data.config.frame.group_item_counts.iter().enumerate() {
                                let group_num = group_idx + 1;
                                let weight = data.config.frame.group_size_weights.get(group_idx).copied().unwrap_or(1.0);
                                let group_w = (weight / total_weight) * available_w;

                                // Draw group content
                                let _ = Self::draw_content_items(
                                    cr,
                                    current_x,
                                    content_y,
                                    group_w,
                                    content_h,
                                    &format!("group{}_", group_num),
                                    item_count,
                                    &data.config,
                                    &data.values,
                                    &data.bar_values,
                                    &data.core_bar_values,
                                    &data.graph_history,
                                );

                                // Draw divider after this group (except for the last group)
                                if group_idx < group_count - 1 {
                                    let divider_x = current_x + group_w + divider_config.spacing_before;
                                    let _ = render_divider(
                                        cr,
                                        divider_x,
                                        content_y,
                                        divider_config.width,
                                        content_h,
                                        divider_config,
                                        SplitOrientation::Vertical,
                                    );
                                    current_x = divider_x + divider_config.width + divider_config.spacing_after;
                                } else {
                                    current_x += group_w;
                                }
                            }
                        }
                        SplitOrientation::Horizontal => {
                            // Groups stacked vertically (top to bottom)
                            let available_h = content_h - num_dividers * total_divider_space;

                            let mut current_y = content_y;
                            for (group_idx, &item_count) in data.config.frame.group_item_counts.iter().enumerate() {
                                let group_num = group_idx + 1;
                                let weight = data.config.frame.group_size_weights.get(group_idx).copied().unwrap_or(1.0);
                                let group_h = (weight / total_weight) * available_h;

                                // Draw group content
                                let _ = Self::draw_content_items(
                                    cr,
                                    content_x,
                                    current_y,
                                    content_w,
                                    group_h,
                                    &format!("group{}_", group_num),
                                    item_count,
                                    &data.config,
                                    &data.values,
                                    &data.bar_values,
                                    &data.core_bar_values,
                                    &data.graph_history,
                                );

                                // Draw divider after this group (except for the last group)
                                if group_idx < group_count - 1 {
                                    let divider_y = current_y + group_h + divider_config.spacing_before;
                                    let _ = render_divider(
                                        cr,
                                        content_x,
                                        divider_y,
                                        content_w,
                                        divider_config.width,
                                        divider_config,
                                        SplitOrientation::Horizontal,
                                    );
                                    current_y = divider_y + divider_config.width + divider_config.spacing_after;
                                } else {
                                    current_y += group_h;
                                }
                            }
                        }
                    }
                }

                cr.restore().ok();
                data.transform.restore(cr);
            }
        });

        // Set up animation timer (60fps)
        glib::timeout_add_local(std::time::Duration::from_millis(16), {
            let data_clone = self.data.clone();
            let drawing_area_weak = drawing_area.downgrade();
            move || {
                let Some(drawing_area) = drawing_area_weak.upgrade() else {
                    return glib::ControlFlow::Break;
                };

                // Skip animation updates when widget is not visible (saves CPU)
                if !drawing_area.is_mapped() {
                    return glib::ControlFlow::Continue;
                }

                // Use try_lock to avoid blocking UI thread if lock is held
                let needs_redraw = if let Ok(mut data) = data_clone.try_lock() {
                    let mut redraw = data.dirty;
                    if data.dirty {
                        data.dirty = false;
                    }

                    // Update animation state
                    if data.config.animation_enabled {
                        let now = Instant::now();
                        let elapsed = now.duration_since(data.last_update).as_secs_f64();
                        data.last_update = now;

                        let speed = data.config.animation_speed;

                        // Animate bar values
                        for (_key, anim) in data.bar_values.iter_mut() {
                            if (anim.current - anim.target).abs() > 0.001 {
                                // Lerp toward target
                                let delta = (anim.target - anim.current) * speed * elapsed;
                                anim.current += delta;

                                // Snap if very close
                                if (anim.current - anim.target).abs() < 0.001 {
                                    anim.current = anim.target;
                                }
                                redraw = true;
                            }
                        }

                        // Animate core bar values
                        for (_key, core_anims) in data.core_bar_values.iter_mut() {
                            for anim in core_anims.iter_mut() {
                                if (anim.current - anim.target).abs() > 0.001 {
                                    // Lerp toward target
                                    let delta = (anim.target - anim.current) * speed * elapsed;
                                    anim.current += delta;

                                    // Snap if very close
                                    if (anim.current - anim.target).abs() < 0.001 {
                                        anim.current = anim.target;
                                    }
                                    redraw = true;
                                }
                            }
                        }
                    }

                    redraw
                } else {
                    false
                };

                if needs_redraw {
                    drawing_area.queue_draw();
                }

                glib::ControlFlow::Continue
            }
        });

        drawing_area.upcast()
    }

    fn update_data(&mut self, data: &HashMap<String, Value>) {
        if let Ok(mut display_data) = self.data.lock() {
            // Store all values
            display_data.values = data.clone();

            // Copy config values we need
            let animation_enabled = display_data.config.animation_enabled;
            let group_item_counts = display_data.config.frame.group_item_counts.clone();
            let content_items = display_data.config.frame.content_items.clone();

            // Calculate timestamp for graph data points
            let timestamp = display_data.graph_start_time.elapsed().as_secs_f64();

            // Collect all prefixes to update (group{N}_{M} format)
            let mut prefixes = Vec::new();
            for (group_idx, &item_count) in group_item_counts.iter().enumerate() {
                let group_num = group_idx + 1;
                for item_idx in 1..=item_count {
                    prefixes.push(format!("group{}_{}", group_num, item_idx));
                }
            }

            // Update each item
            for prefix in &prefixes {
                let item_data = Self::get_item_data(data, prefix);
                let bar_key = format!("{}_bar", prefix);
                let target = item_data.percent();

                // Update bar animation
                let anim = display_data.bar_values.entry(bar_key).or_default();
                anim.target = target;
                if anim.first_update || !animation_enabled {
                    anim.current = target;
                    anim.first_update = false;
                }

                // Check if this item is configured as a graph or core bars
                if let Some(item_config) = content_items.get(prefix) {
                    if matches!(item_config.display_as, ContentDisplayType::Graph) {
                        let graph_key = format!("{}_graph", prefix);
                        let history = display_data.graph_history.entry(graph_key).or_insert_with(VecDeque::new);

                        // Add new data point
                        history.push_back(DataPoint {
                            value: item_data.numerical_value,
                            timestamp,
                        });

                        // Keep only max_data_points
                        let max_points = item_config.graph_config.max_data_points;
                        while history.len() > max_points {
                            history.pop_front();
                        }
                    } else if matches!(item_config.display_as, ContentDisplayType::CoreBars) {
                        // Update core bar animated values
                        let core_bars_config = &item_config.core_bars_config;

                        // Collect core values from source data (pre-allocate for expected size)
                        let capacity = core_bars_config.end_core.saturating_sub(core_bars_config.start_core) + 1;
                        let mut core_targets: Vec<f64> = Vec::with_capacity(capacity);
                        for core_idx in core_bars_config.start_core..=core_bars_config.end_core {
                            let core_key = format!("{}_core{}_usage", prefix, core_idx);
                            let value = data.get(&core_key)
                                .and_then(|v| v.as_f64())
                                .unwrap_or(0.0) / 100.0; // Normalize to 0.0-1.0
                            core_targets.push(value);
                        }

                        // If no specific core values found, try to find any core values
                        if core_targets.is_empty() {
                            for core_idx in 0..128 {
                                let core_key = format!("{}_core{}_usage", prefix, core_idx);
                                if let Some(v) = data.get(&core_key).and_then(|v| v.as_f64()) {
                                    core_targets.push(v / 100.0);
                                } else {
                                    break;
                                }
                            }
                        }

                        // Get or create animated values for this prefix
                        let anims = display_data.core_bar_values.entry(prefix.clone()).or_insert_with(Vec::new);

                        // Ensure we have enough AnimatedValue entries
                        while anims.len() < core_targets.len() {
                            anims.push(AnimatedValue::default());
                        }
                        // Truncate if we have too many
                        anims.truncate(core_targets.len());

                        // Update targets
                        for (i, &target) in core_targets.iter().enumerate() {
                            if let Some(anim) = anims.get_mut(i) {
                                anim.target = target;
                                if anim.first_update || !animation_enabled {
                                    anim.current = target;
                                    anim.first_update = false;
                                }
                            }
                        }
                    }
                }
            }

            // Clean up stale animation entries that no longer match active prefixes
            // This prevents memory leaks when config changes remove content items
            {
                // Collect keys to remove (can't modify while iterating)
                let bar_keys_to_remove: Vec<String> = display_data.bar_values.keys()
                    .filter(|k| {
                        // Extract prefix from key (e.g., "group1_1_bar" -> "group1_1")
                        k.strip_suffix("_bar")
                            .map(|prefix| !prefixes.iter().any(|p| p == prefix))
                            .unwrap_or(true)
                    })
                    .cloned()
                    .collect();

                let core_keys_to_remove: Vec<String> = display_data.core_bar_values.keys()
                    .filter(|k| !prefixes.iter().any(|p| p == *k))
                    .cloned()
                    .collect();

                let graph_keys_to_remove: Vec<String> = display_data.graph_history.keys()
                    .filter(|k| {
                        // Extract prefix from key (e.g., "group1_1_graph" -> "group1_1")
                        k.strip_suffix("_graph")
                            .map(|prefix| !prefixes.iter().any(|p| p == prefix))
                            .unwrap_or(true)
                    })
                    .cloned()
                    .collect();

                // Remove stale entries
                for key in bar_keys_to_remove {
                    display_data.bar_values.remove(&key);
                }
                for key in core_keys_to_remove {
                    display_data.core_bar_values.remove(&key);
                }
                for key in graph_keys_to_remove {
                    display_data.graph_history.remove(&key);
                }
            }

            // Extract transform from values
            display_data.transform = PanelTransform::from_values(data);

            display_data.dirty = true;
        }
    }

    fn draw(&self, cr: &Context, width: f64, height: f64) -> Result<()> {
        if let Ok(data) = self.data.lock() {
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
            if let Ok(mut lcars_config) = serde_json::from_value::<LcarsDisplayConfig>(lcars_config_value.clone()) {
                // Migrate legacy primary/secondary format to groups format
                lcars_config.frame.migrate_legacy();
                if let Ok(mut display_data) = self.data.lock() {
                    display_data.config = lcars_config;
                }
                return Ok(());
            }
        }

        // Apply individual settings for backward compatibility
        if let Ok(mut display_data) = self.data.lock() {
            if let Some(sidebar_width) = config.get("sidebar_width").and_then(|v| v.as_f64()) {
                display_data.config.frame.sidebar_width = sidebar_width;
            }

            if let Some(animation_enabled) = config.get("animation_enabled").and_then(|v| v.as_bool()) {
                display_data.config.animation_enabled = animation_enabled;
            }

            if let Some(segment_count) = config.get("segment_count").and_then(|v| v.as_u64()) {
                display_data.config.frame.segment_count = segment_count as u32;
            }
        }

        Ok(())
    }

    fn needs_redraw(&self) -> bool {
        true
    }

    fn get_typed_config(&self) -> Option<crate::core::DisplayerConfig> {
        if let Ok(display_data) = self.data.lock() {
            Some(crate::core::DisplayerConfig::Lcars(display_data.config.clone()))
        } else {
            None
        }
    }
}
