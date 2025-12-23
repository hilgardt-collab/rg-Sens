//! Industrial/Gauge Panel displayer
//!
//! Visualizes combo source data with industrial aesthetic:
//! - Brushed metal/carbon fiber textures
//! - Physical gauge aesthetics (rivets, bezels)
//! - Warning stripe accents
//! - Heavy bold typography

use anyhow::Result;
use cairo::Context;
use gtk4::{glib, prelude::*, DrawingArea, Widget};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::core::{ConfigOption, ConfigSchema, Displayer, PanelTransform, ANIMATION_FRAME_INTERVAL, ANIMATION_SNAP_THRESHOLD};
use crate::ui::graph_display::DataPoint;
use crate::ui::industrial_display::{
    render_industrial_frame, calculate_group_layouts, draw_group_dividers, draw_group_panel,
    IndustrialFrameConfig,
};
use crate::ui::lcars_display::{
    render_content_bar, render_content_text, render_content_graph,
    render_content_core_bars, render_content_static, calculate_item_layouts,
    ContentDisplayType, ContentItemData, ContentItemConfig,
};
use crate::ui::arc_display::render_arc;
use crate::ui::speedometer_display::render_speedometer;

/// Animation state for a single value
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

/// Industrial display configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndustrialDisplayConfig {
    pub frame: IndustrialFrameConfig,
    pub animation_enabled: bool,
    pub animation_speed: f64,
}

impl Default for IndustrialDisplayConfig {
    fn default() -> Self {
        Self {
            frame: IndustrialFrameConfig::default(),
            animation_enabled: true,
            animation_speed: 8.0,
        }
    }
}

/// Display data for rendering
struct DisplayData {
    config: IndustrialDisplayConfig,
    values: HashMap<String, Value>,
    bar_values: HashMap<String, AnimatedValue>,
    core_bar_values: HashMap<String, Vec<AnimatedValue>>,
    graph_history: HashMap<String, VecDeque<DataPoint>>,
    graph_start_time: Instant,
    last_update: Instant,
    transform: PanelTransform,
    dirty: bool,
}

impl Clone for DisplayData {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            values: self.values.clone(),
            bar_values: self.bar_values.clone(),
            core_bar_values: self.core_bar_values.clone(),
            graph_history: self.graph_history.clone(),
            graph_start_time: self.graph_start_time,
            last_update: Instant::now(),
            transform: self.transform.clone(),
            dirty: self.dirty,
        }
    }
}

/// Industrial/Gauge Panel displayer
pub struct IndustrialDisplayer {
    id: String,
    name: String,
    data: Arc<Mutex<DisplayData>>,
}

impl IndustrialDisplayer {
    pub fn new() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Industrial".to_string(),
            data: Arc::new(Mutex::new(DisplayData {
                config: IndustrialDisplayConfig::default(),
                values: HashMap::new(),
                bar_values: HashMap::new(),
                core_bar_values: HashMap::new(),
                graph_history: HashMap::new(),
                graph_start_time: Instant::now(),
                last_update: Instant::now(),
                transform: PanelTransform::default(),
                dirty: true,
            })),
        }
    }

    /// Get item data from values for a given prefix
    fn get_item_data(values: &HashMap<String, Value>, prefix: &str) -> ContentItemData {
        let caption = values.get(&format!("{}_label", prefix))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let value = values.get(&format!("{}_value", prefix))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let unit = values.get(&format!("{}_unit", prefix))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let numerical_value = values.get(&format!("{}_numerical_value", prefix))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let min_value = values.get(&format!("{}_min_limit", prefix))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let max_value = values.get(&format!("{}_max_limit", prefix))
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

    /// Get all values for a slot prefix as a HashMap
    fn get_slot_values(values: &HashMap<String, Value>, prefix: &str) -> HashMap<String, Value> {
        values.iter()
            .filter(|(k, _)| k.starts_with(prefix))
            .map(|(k, v)| {
                let key = k.strip_prefix(prefix)
                    .and_then(|s| s.strip_prefix('_'))
                    .unwrap_or(k)
                    .to_string();
                (key, v.clone())
            })
            .collect()
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
        config: &IndustrialDisplayConfig,
        values: &HashMap<String, Value>,
        bar_values: &HashMap<String, AnimatedValue>,
        core_bar_values: &HashMap<String, Vec<AnimatedValue>>,
        graph_history: &HashMap<String, VecDeque<DataPoint>>,
    ) -> Result<(), cairo::Error> {
        if count == 0 || w <= 0.0 || h <= 0.0 {
            return Ok(());
        }

        // Determine fixed heights for items that need them
        let mut fixed_heights: HashMap<usize, f64> = HashMap::new();
        for i in 0..count as usize {
            let prefix = format!("{}{}", base_prefix, i + 1);
            let item_config = config.frame.content_items.get(&prefix);
            if let Some(cfg) = item_config {
                if !cfg.auto_height || matches!(cfg.display_as, ContentDisplayType::Graph) {
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
                    let graph_key = format!("{}_graph", prefix);
                    if let Some(history) = graph_history.get(&graph_key) {
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
                            log::warn!("Failed to render graph: {}", e);
                        }
                    }
                }
                ContentDisplayType::CoreBars => {
                    let core_anims = core_bar_values.get(&prefix);
                    let core_values: Vec<f64> = if let Some(anims) = core_anims {
                        anims.iter().map(|av| av.current).collect()
                    } else {
                        // Fallback to raw values
                        let core_bars_config = &item_config.core_bars_config;
                        let mut raw_values: Vec<f64> = Vec::new();
                        for core_idx in core_bars_config.start_core..=core_bars_config.end_core {
                            let core_key = format!("{}_core{}_usage", prefix, core_idx);
                            let value = values.get(&core_key)
                                .and_then(|v| v.as_f64())
                                .unwrap_or(0.0);
                            raw_values.push(value / 100.0);
                        }

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

                    render_content_core_bars(
                        cr,
                        item_x,
                        item_y,
                        item_w,
                        item_h,
                        &item_config.core_bars_config,
                        &core_values,
                    )?;
                }
                ContentDisplayType::LevelBar => {
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
                ContentDisplayType::Static => {
                    render_content_static(
                        cr,
                        item_x,
                        item_y,
                        item_w,
                        item_h,
                        &item_config.static_config,
                        &item_config.bar_config,
                        Some(&slot_values),
                    )?;
                }
                ContentDisplayType::Arc => {
                    cr.save()?;
                    cr.translate(item_x, item_y);
                    render_arc(
                        cr,
                        &item_config.arc_config,
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
                    if let Err(e) = render_speedometer(
                        cr,
                        &item_config.speedometer_config,
                        animated_percent,
                        &slot_values,
                        item_w,
                        item_h,
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

impl Default for IndustrialDisplayer {
    fn default() -> Self {
        Self::new()
    }
}

impl Displayer for IndustrialDisplayer {
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
            if width < 10 || height < 10 {
                return;
            }
            if let Ok(data) = data_clone.lock() {
                let w = width as f64;
                let h = height as f64;
                data.transform.apply(cr, w, h);

                // Draw the Industrial frame and get content bounds
                let content_bounds = match render_industrial_frame(cr, &data.config.frame, w, h) {
                    Ok(bounds) => bounds,
                    Err(e) => {
                        log::debug!("Industrial frame render error: {}", e);
                        return;
                    }
                };

                let (content_x, content_y, content_w, content_h) = content_bounds;

                // Calculate group layouts
                let group_layouts = calculate_group_layouts(
                    content_x, content_y, content_w, content_h,
                    &data.config.frame,
                );

                // Draw group dividers
                if let Err(e) = draw_group_dividers(cr, &group_layouts, &data.config.frame) {
                    log::debug!("Failed to draw industrial dividers: {}", e);
                }

                // Draw content items for each group
                for (group_x, group_y, group_w, group_h, group_idx) in &group_layouts {
                    // Draw subtle group panel
                    if let Err(e) = draw_group_panel(cr, *group_x, *group_y, *group_w, *group_h, &data.config.frame) {
                        log::debug!("Failed to draw group panel: {}", e);
                    }

                    let item_count = data.config.frame.group_item_counts
                        .get(*group_idx)
                        .copied()
                        .unwrap_or(0) as u32;

                    let base_prefix = format!("group{}_", group_idx + 1);

                    if let Err(e) = Self::draw_content_items(
                        cr,
                        *group_x,
                        *group_y,
                        *group_w,
                        *group_h,
                        &base_prefix,
                        item_count,
                        &data.config,
                        &data.values,
                        &data.bar_values,
                        &data.core_bar_values,
                        &data.graph_history,
                    ) {
                        log::debug!("Failed to draw industrial content items: {}", e);
                    }
                }

                data.transform.restore(cr);
            }
        });

        // Set up animation timer (60fps)
        glib::timeout_add_local(ANIMATION_FRAME_INTERVAL, {
            let data_clone = self.data.clone();
            let drawing_area_weak = drawing_area.downgrade();
            move || {
                let Some(drawing_area) = drawing_area_weak.upgrade() else {
                    return glib::ControlFlow::Break;
                };

                if !drawing_area.is_mapped() {
                    return glib::ControlFlow::Continue;
                }

                let needs_redraw = if let Ok(mut data) = data_clone.try_lock() {
                    let mut redraw = data.dirty;
                    if data.dirty {
                        data.dirty = false;
                    }

                    if data.config.animation_enabled {
                        let now = Instant::now();
                        let elapsed = now.duration_since(data.last_update).as_secs_f64();
                        data.last_update = now;

                        let speed = data.config.animation_speed;

                        // Animate bar values
                        for (_key, anim) in data.bar_values.iter_mut() {
                            if (anim.current - anim.target).abs() > ANIMATION_SNAP_THRESHOLD {
                                let delta = (anim.target - anim.current) * speed * elapsed;
                                anim.current += delta;

                                if (anim.current - anim.target).abs() < ANIMATION_SNAP_THRESHOLD {
                                    anim.current = anim.target;
                                }
                                redraw = true;
                            }
                        }

                        // Animate core bar values
                        for (_key, core_anims) in data.core_bar_values.iter_mut() {
                            for anim in core_anims.iter_mut() {
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
            let animation_enabled = display_data.config.animation_enabled;
            let group_item_counts = display_data.config.frame.group_item_counts.clone();
            let content_items = display_data.config.frame.content_items.clone();
            let timestamp = display_data.graph_start_time.elapsed().as_secs_f64();

            // Collect all prefixes
            let mut prefixes = Vec::new();
            for (group_idx, &item_count) in group_item_counts.iter().enumerate() {
                let group_num = group_idx + 1;
                for item_idx in 1..=item_count {
                    prefixes.push(format!("group{}_{}", group_num, item_idx));
                }
            }

            // Filter values to only those we need
            display_data.values = data.iter()
                .filter(|(k, _)| prefixes.iter().any(|prefix| k.starts_with(prefix)))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();

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

                // Handle graph and core bars - use default config if not present
                let default_item_config = ContentItemConfig::default();
                let item_config = content_items.get(prefix).unwrap_or(&default_item_config);
                {
                    if matches!(item_config.display_as, ContentDisplayType::Graph) {
                        let graph_key = format!("{}_graph", prefix);
                        let history = display_data.graph_history.entry(graph_key).or_insert_with(VecDeque::new);

                        history.push_back(DataPoint {
                            value: item_data.numerical_value,
                            timestamp,
                        });

                        let max_points = item_config.graph_config.max_data_points;
                        while history.len() > max_points {
                            history.pop_front();
                        }
                    } else if matches!(item_config.display_as, ContentDisplayType::CoreBars) {
                        let core_bars_config = &item_config.core_bars_config;
                        let capacity = core_bars_config.end_core.saturating_sub(core_bars_config.start_core) + 1;
                        let mut core_targets: Vec<f64> = Vec::with_capacity(capacity);

                        for core_idx in core_bars_config.start_core..=core_bars_config.end_core {
                            let core_key = format!("{}_core{}_usage", prefix, core_idx);
                            let value = data.get(&core_key)
                                .and_then(|v| v.as_f64())
                                .unwrap_or(0.0) / 100.0;
                            core_targets.push(value);
                        }

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

                        let anims = display_data.core_bar_values.entry(prefix.clone()).or_insert_with(Vec::new);
                        while anims.len() < core_targets.len() {
                            anims.push(AnimatedValue::default());
                        }

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

            // Cleanup: remove graph history for items no longer configured as graphs
            let graph_keys_to_remove: Vec<String> = display_data.graph_history.keys()
                .filter(|k| {
                    let prefix = k.trim_end_matches("_graph");
                    content_items.get(prefix)
                        .map(|c| !matches!(c.display_as, ContentDisplayType::Graph))
                        .unwrap_or(true)
                })
                .cloned()
                .collect();
            for key in graph_keys_to_remove {
                display_data.graph_history.remove(&key);
            }

            display_data.transform = PanelTransform::from_values(data);
            display_data.dirty = true;
        }
    }

    fn draw(&self, cr: &Context, width: f64, height: f64) -> Result<()> {
        if width < 10.0 || height < 10.0 {
            return Ok(());
        }
        // Use try_lock to avoid blocking the GTK main thread
        if let Ok(data) = self.data.try_lock() {
            data.transform.apply(cr, width, height);
            render_industrial_frame(cr, &data.config.frame, width, height)?;
            data.transform.restore(cr);
        }
        Ok(())
    }

    fn config_schema(&self) -> ConfigSchema {
        ConfigSchema {
            options: vec![
                ConfigOption {
                    key: "surface_texture".to_string(),
                    name: "Surface Texture".to_string(),
                    description: "Metal surface texture style".to_string(),
                    value_type: "string".to_string(),
                    default: serde_json::json!("brushed_metal"),
                },
                ConfigOption {
                    key: "rivet_style".to_string(),
                    name: "Rivet Style".to_string(),
                    description: "Style of rivets/bolts".to_string(),
                    value_type: "string".to_string(),
                    default: serde_json::json!("hex"),
                },
                ConfigOption {
                    key: "animation_enabled".to_string(),
                    name: "Animation".to_string(),
                    description: "Enable smooth animations".to_string(),
                    value_type: "boolean".to_string(),
                    default: serde_json::json!(true),
                },
            ],
        }
    }

    fn apply_config(&mut self, config: &HashMap<String, Value>) -> Result<()> {
        // Check for full industrial_config first
        if let Some(industrial_config_value) = config.get("industrial_config") {
            if let Ok(industrial_config) = serde_json::from_value::<IndustrialDisplayConfig>(industrial_config_value.clone()) {
                if let Ok(mut display_data) = self.data.lock() {
                    display_data.config = industrial_config;
                }
                return Ok(());
            }
        }

        // Apply individual settings for backward compatibility
        if let Ok(mut display_data) = self.data.lock() {
            if let Some(animation) = config.get("animation_enabled").and_then(|v| v.as_bool()) {
                display_data.config.animation_enabled = animation;
            }
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
            Some(crate::core::DisplayerConfig::Industrial(display_data.config.clone()))
        } else {
            None
        }
    }
}
