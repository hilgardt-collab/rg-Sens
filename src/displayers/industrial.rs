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
use crate::displayers::combo_utils::{self, AnimatedValue};
use crate::ui::graph_display::DataPoint;
use crate::ui::industrial_display::{
    render_industrial_frame, calculate_group_layouts, draw_group_dividers, draw_group_panel,
    IndustrialFrameConfig,
};
use crate::ui::lcars_display::{
    render_content_bar, render_content_text, render_content_graph,
    render_content_core_bars, render_content_static, calculate_item_layouts,
    ContentDisplayType, ContentItemConfig,
};
use crate::ui::arc_display::render_arc;
use crate::ui::speedometer_display::render_speedometer_with_theme;

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
            transform: self.transform,
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
            id: "industrial".to_string(),
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
            let item_data = combo_utils::get_item_data(values, &prefix);
            let slot_values = combo_utils::get_slot_values(values, &prefix);

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

                        // Try configured range first
                        for core_idx in core_bars_config.start_core..=core_bars_config.end_core {
                            let core_key = format!("{}_core{}_usage", prefix, core_idx);
                            if let Some(v) = values.get(&core_key).and_then(|v| v.as_f64()) {
                                raw_values.push(v / 100.0);
                            }
                        }

                        // Fallback: scan for any core keys
                        if raw_values.is_empty() {
                            for core_idx in 0..128 {
                                let core_key = format!("{}_core{}_usage", prefix, core_idx);
                                if let Some(v) = values.get(&core_key).and_then(|v| v.as_f64()) {
                                    raw_values.push(v / 100.0);
                                } else if core_idx > 0 {
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
                        &config.frame.theme,
                        &core_values,
                        Some(&slot_values),
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
                        &config.frame.theme,
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
                for (group_idx, (group_x, group_y, group_w, group_h, item_count)) in group_layouts.iter().enumerate() {
                    // Draw subtle group panel
                    if let Err(e) = draw_group_panel(cr, *group_x, *group_y, *group_w, *group_h, &data.config.frame) {
                        log::debug!("Failed to draw group panel: {}", e);
                    }

                    let base_prefix = format!("group{}_", group_idx + 1);

                    if let Err(e) = Self::draw_content_items(
                        cr,
                        *group_x,
                        *group_y,
                        *group_w,
                        *group_h,
                        &base_prefix,
                        *item_count as u32,
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
            let timestamp = display_data.graph_start_time.elapsed().as_secs_f64();

            // Clone config data to avoid borrow conflicts
            let group_item_counts: Vec<usize> = display_data.config.frame.group_item_counts.to_vec();
            let content_items = display_data.config.frame.content_items.clone();

            // Generate prefixes and filter values using optimized utils
            let prefixes = combo_utils::generate_prefixes(&group_item_counts);
            display_data.values = combo_utils::filter_values_by_prefixes(data, &prefixes);

            // Update each item
            for prefix in &prefixes {
                let item_data = combo_utils::get_item_data(data, prefix);
                combo_utils::update_bar_animation(&mut display_data.bar_values, prefix, item_data.percent(), animation_enabled);

                let default_config = ContentItemConfig::default();
                let item_config = content_items.get(prefix).unwrap_or(&default_config);

                match item_config.display_as {
                    ContentDisplayType::Graph => {
                        combo_utils::update_graph_history(
                            &mut display_data.graph_history,
                            prefix,
                            item_data.numerical_value,
                            timestamp,
                            item_config.graph_config.max_data_points,
                        );
                    }
                    ContentDisplayType::CoreBars => {
                        combo_utils::update_core_bars(
                            data,
                            &mut display_data.core_bar_values,
                            prefix,
                            &item_config.core_bars_config,
                            animation_enabled,
                        );
                    }
                    _ => {}
                }
            }

            // Use optimized cleanup with retain() - separate calls to avoid borrow conflicts
            combo_utils::cleanup_bar_values(&mut display_data.bar_values, &prefixes);
            combo_utils::cleanup_core_bar_values(&mut display_data.core_bar_values, &prefixes);
            combo_utils::cleanup_graph_history(&mut display_data.graph_history, &prefixes);

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
                    // Clear stale animation data when config changes
                    display_data.bar_values.clear();
                    display_data.core_bar_values.clear();
                    display_data.graph_history.clear();
                    display_data.dirty = true;
                }
                return Ok(());
            }
        }

        // Apply individual settings for backward compatibility
        if let Ok(mut display_data) = self.data.lock() {
            if let Some(animation) = config.get("animation_enabled").and_then(|v| v.as_bool()) {
                display_data.config.animation_enabled = animation;
            }
            display_data.dirty = true;
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
