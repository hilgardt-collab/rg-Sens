//! Cyberpunk/Neon HUD Displayer
//!
//! A futuristic heads-up display with:
//! - Angular chamfered corners with neon glow effects
//! - Dark translucent backgrounds with grid patterns
//! - Scanline overlay for CRT/hologram effect
//! - Support for multiple data source groups

use anyhow::Result;
use cairo::Context;
use gtk4::{prelude::*, DrawingArea, Widget};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::core::{ConfigOption, ConfigSchema, Displayer};
use crate::displayers::combo_displayer_base::{
    draw_content_items_generic, handle_combo_update_data, ComboDisplayData, ContentDrawParams,
};
use crate::ui::cyberpunk_display::{
    calculate_group_layouts, draw_group_dividers, draw_item_frame, render_cyberpunk_frame,
    CyberpunkFrameConfig,
};

/// Full Cyberpunk display configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CyberpunkDisplayConfig {
    /// Frame configuration
    #[serde(default)]
    pub frame: CyberpunkFrameConfig,

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
    8.0
}

impl Default for CyberpunkDisplayConfig {
    fn default() -> Self {
        Self {
            frame: CyberpunkFrameConfig::default(),
            animation_enabled: default_animation_enabled(),
            animation_speed: default_animation_speed(),
        }
    }
}

/// Internal display data combining config with shared combo data
#[derive(Clone)]
struct DisplayData {
    config: CyberpunkDisplayConfig,
    combo: ComboDisplayData,
}

impl Default for DisplayData {
    fn default() -> Self {
        Self {
            config: CyberpunkDisplayConfig::default(),
            combo: ComboDisplayData::default(),
        }
    }
}

/// Cyberpunk/Neon HUD Displayer
pub struct CyberpunkDisplayer {
    id: String,
    name: String,
    data: Arc<Mutex<DisplayData>>,
}

impl CyberpunkDisplayer {
    pub fn new() -> Self {
        Self {
            id: "cyberpunk".to_string(),
            name: "Cyberpunk HUD".to_string(),
            data: Arc::new(Mutex::new(DisplayData::default())),
        }
    }
}

impl Default for CyberpunkDisplayer {
    fn default() -> Self {
        Self::new()
    }
}

impl Displayer for CyberpunkDisplayer {
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
                data.combo.transform.apply(cr, w, h);

                // Draw the Cyberpunk frame and get content bounds
                let content_bounds = match render_cyberpunk_frame(cr, &data.config.frame, w, h) {
                    Ok(bounds) => bounds,
                    Err(e) => {
                        log::debug!(
                            "Cyberpunk frame render error (usually harmless during layout): {}",
                            e
                        );
                        return;
                    }
                };

                let (content_x, content_y, content_w, content_h) = content_bounds;

                // Calculate group layouts
                let group_layouts = calculate_group_layouts(
                    &data.config.frame,
                    content_x,
                    content_y,
                    content_w,
                    content_h,
                );

                // Draw dividers between groups
                draw_group_dividers(cr, &data.config.frame, &group_layouts);

                // Clip to content area
                cr.save().ok();
                cr.rectangle(content_x, content_y, content_w, content_h);
                cr.clip();

                // Prepare draw params
                let params = ContentDrawParams {
                    values: &data.combo.values,
                    bar_values: &data.combo.bar_values,
                    core_bar_values: &data.combo.core_bar_values,
                    graph_history: &data.combo.graph_history,
                    content_items: &data.config.frame.content_items,
                    group_item_orientations: &data.config.frame.group_item_orientations,
                    split_orientation: data.config.frame.split_orientation,
                    theme: &data.config.frame.theme,
                };

                // Draw content for each group
                let group_item_counts = &data.config.frame.group_item_counts;
                for (group_idx, &(gx, gy, gw, gh)) in group_layouts.iter().enumerate() {
                    let group_num = group_idx + 1;
                    let item_count = group_item_counts.get(group_idx).copied().unwrap_or(1) as u32;

                    // Capture frame reference for the closure
                    let frame = &data.config.frame;
                    let _ = draw_content_items_generic(
                        cr,
                        gx,
                        gy,
                        gw,
                        gh,
                        &format!("group{}_", group_num),
                        item_count,
                        group_idx,
                        &params,
                        |cr, x, y, w, h| draw_item_frame(cr, frame, x, y, w, h),
                    );
                }

                cr.restore().ok();
                data.combo.transform.restore(cr);
            }
        });

        // Set up animation timer
        gtk4::glib::timeout_add_local(crate::core::ANIMATION_FRAME_INTERVAL, {
            let data_clone = self.data.clone();
            let drawing_area_weak = drawing_area.downgrade();
            move || {
                let Some(drawing_area) = drawing_area_weak.upgrade() else {
                    return gtk4::glib::ControlFlow::Break;
                };

                if !drawing_area.is_mapped() {
                    return gtk4::glib::ControlFlow::Continue;
                }

                let needs_redraw = if let Ok(mut data) = data_clone.try_lock() {
                    let mut redraw = data.combo.dirty;
                    if data.combo.dirty {
                        data.combo.dirty = false;
                    }

                    if data.config.animation_enabled {
                        let now = std::time::Instant::now();
                        let elapsed = now.duration_since(data.combo.last_update).as_secs_f64();
                        data.combo.last_update = now;

                        let speed = data.config.animation_speed;

                        // Animate bar values
                        for anim in data.combo.bar_values.values_mut() {
                            if (anim.current - anim.target).abs()
                                > crate::core::ANIMATION_SNAP_THRESHOLD
                            {
                                let delta = (anim.target - anim.current) * speed * elapsed;
                                anim.current += delta;

                                if (anim.current - anim.target).abs()
                                    < crate::core::ANIMATION_SNAP_THRESHOLD
                                {
                                    anim.current = anim.target;
                                }
                                redraw = true;
                            }
                        }

                        // Animate core bar values
                        for core_anims in data.combo.core_bar_values.values_mut() {
                            for anim in core_anims.iter_mut() {
                                if (anim.current - anim.target).abs()
                                    > crate::core::ANIMATION_SNAP_THRESHOLD
                                {
                                    let delta = (anim.target - anim.current) * speed * elapsed;
                                    anim.current += delta;

                                    if (anim.current - anim.target).abs()
                                        < crate::core::ANIMATION_SNAP_THRESHOLD
                                    {
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

                gtk4::glib::ControlFlow::Continue
            }
        });

        drawing_area.upcast()
    }

    fn update_data(&mut self, data: &HashMap<String, Value>) {
        if let Ok(mut display_data) = self.data.lock() {
            // Clone config data to avoid borrow conflicts
            let group_item_counts = display_data.config.frame.group_item_counts.clone();
            let content_items = display_data.config.frame.content_items.clone();
            let animation_enabled = display_data.config.animation_enabled;

            // Use shared update logic
            handle_combo_update_data(
                &mut display_data.combo,
                data,
                &group_item_counts,
                &content_items,
                animation_enabled,
            );
        }
    }

    fn draw(&self, cr: &Context, width: f64, height: f64) -> Result<()> {
        if width < 10.0 || height < 10.0 {
            return Ok(());
        }
        if let Ok(data) = self.data.try_lock() {
            data.combo.transform.apply(cr, width, height);
            render_cyberpunk_frame(cr, &data.config.frame, width, height)?;
            data.combo.transform.restore(cr);
        }
        Ok(())
    }

    fn config_schema(&self) -> ConfigSchema {
        ConfigSchema {
            options: vec![
                ConfigOption {
                    key: "border_color".to_string(),
                    name: "Border Color".to_string(),
                    description: "Neon border color".to_string(),
                    value_type: "color".to_string(),
                    default: serde_json::json!([0.0, 1.0, 1.0, 1.0]),
                },
                ConfigOption {
                    key: "glow_intensity".to_string(),
                    name: "Glow Intensity".to_string(),
                    description: "Intensity of the neon glow effect".to_string(),
                    value_type: "number".to_string(),
                    default: serde_json::json!(0.6),
                },
                ConfigOption {
                    key: "show_scanlines".to_string(),
                    name: "Show Scanlines".to_string(),
                    description: "Enable CRT scanline effect".to_string(),
                    value_type: "boolean".to_string(),
                    default: serde_json::json!(true),
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
        if let Some(cyberpunk_config_value) = config.get("cyberpunk_config") {
            if let Ok(cyberpunk_config) =
                serde_json::from_value::<CyberpunkDisplayConfig>(cyberpunk_config_value.clone())
            {
                log::debug!(
                    "CyberpunkDisplayer::apply_config - loaded {} groups, {} content_items",
                    cyberpunk_config.frame.group_count,
                    cyberpunk_config.frame.content_items.len()
                );
                if let Ok(mut display_data) = self.data.lock() {
                    display_data.config = cyberpunk_config;
                }
                return Ok(());
            } else {
                log::warn!(
                    "CyberpunkDisplayer::apply_config - failed to deserialize cyberpunk_config"
                );
            }
        }

        // Apply individual settings for backward compatibility
        if let Ok(mut display_data) = self.data.lock() {
            if let Some(glow) = config.get("glow_intensity").and_then(|v| v.as_f64()) {
                display_data.config.frame.glow_intensity = glow;
            }

            if let Some(scanlines) = config.get("show_scanlines").and_then(|v| v.as_bool()) {
                display_data.config.frame.show_scanlines = scanlines;
            }

            if let Some(animation) = config.get("animation_enabled").and_then(|v| v.as_bool()) {
                display_data.config.animation_enabled = animation;
            }
        }

        Ok(())
    }

    fn needs_redraw(&self) -> bool {
        self.data
            .try_lock()
            .map(|data| data.combo.dirty)
            .unwrap_or(true)
    }

    fn get_typed_config(&self) -> Option<crate::core::DisplayerConfig> {
        if let Ok(display_data) = self.data.try_lock() {
            log::debug!(
                "CyberpunkDisplayer::get_typed_config - saving {} groups, {} content_items",
                display_data.config.frame.group_count,
                display_data.config.frame.content_items.len()
            );
            Some(crate::core::DisplayerConfig::Cyberpunk(
                display_data.config.clone(),
            ))
        } else {
            None
        }
    }
}
