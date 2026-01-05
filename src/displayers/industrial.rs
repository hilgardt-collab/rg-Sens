//! Industrial/Gauge Panel displayer
//!
//! Visualizes combo source data with industrial aesthetic:
//! - Brushed metal/carbon fiber textures
//! - Physical gauge aesthetics (rivets, bezels)
//! - Warning stripe accents
//! - Heavy bold typography

use anyhow::Result;
use cairo::Context;
use gtk4::{prelude::*, DrawingArea, Widget};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::core::{ConfigOption, ConfigSchema, Displayer, PanelTransform};
use crate::displayers::combo_displayer_base::{ComboDisplayData, ContentDrawParams, draw_content_items_generic, handle_combo_update_data};
use crate::ui::industrial_display::{
    render_industrial_frame, calculate_group_layouts, draw_group_dividers, draw_group_panel,
    IndustrialFrameConfig,
};

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
#[derive(Clone)]
struct DisplayData {
    config: IndustrialDisplayConfig,
    combo: ComboDisplayData,
}

impl Default for DisplayData {
    fn default() -> Self {
        Self {
            config: IndustrialDisplayConfig::default(),
            combo: ComboDisplayData::default(),
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
            data: Arc::new(Mutex::new(DisplayData::default())),
        }
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
                data.combo.transform.apply(cr, w, h);

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

                // Build draw params
                let draw_params = ContentDrawParams {
                    values: &data.combo.values,
                    bar_values: &data.combo.bar_values,
                    core_bar_values: &data.combo.core_bar_values,
                    graph_history: &data.combo.graph_history,
                    content_items: &data.config.frame.content_items,
                    group_item_orientations: &data.config.frame.group_item_orientations,
                    split_orientation: data.config.frame.split_orientation,
                    theme: &data.config.frame.theme,
                };

                // Draw content items for each group
                for (group_idx, (group_x, group_y, group_w, group_h, item_count)) in group_layouts.iter().enumerate() {
                    // Draw subtle group panel
                    if let Err(e) = draw_group_panel(cr, *group_x, *group_y, *group_w, *group_h, &data.config.frame) {
                        log::debug!("Failed to draw group panel: {}", e);
                    }

                    let base_prefix = format!("group{}_", group_idx + 1);

                    // Use shared drawing function with no custom frame drawing
                    let _ = draw_content_items_generic(
                        cr,
                        *group_x,
                        *group_y,
                        *group_w,
                        *group_h,
                        &base_prefix,
                        *item_count as u32,
                        group_idx,
                        &draw_params,
                        |_, _, _, _, _| {},
                    );
                }

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
                            if (anim.current - anim.target).abs() > crate::core::ANIMATION_SNAP_THRESHOLD {
                                let delta = (anim.target - anim.current) * speed * elapsed;
                                anim.current += delta;

                                if (anim.current - anim.target).abs() < crate::core::ANIMATION_SNAP_THRESHOLD {
                                    anim.current = anim.target;
                                }
                                redraw = true;
                            }
                        }

                        // Animate core bar values
                        for core_anims in data.combo.core_bar_values.values_mut() {
                            for anim in core_anims.iter_mut() {
                                if (anim.current - anim.target).abs() > crate::core::ANIMATION_SNAP_THRESHOLD {
                                    let delta = (anim.target - anim.current) * speed * elapsed;
                                    anim.current += delta;

                                    if (anim.current - anim.target).abs() < crate::core::ANIMATION_SNAP_THRESHOLD {
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

            handle_combo_update_data(
                &mut display_data.combo,
                data,
                &group_item_counts,
                &content_items,
                animation_enabled,
            );

            display_data.combo.transform = PanelTransform::from_values(data);
            display_data.combo.dirty = true;
        }
    }

    fn draw(&self, cr: &Context, width: f64, height: f64) -> Result<()> {
        if width < 10.0 || height < 10.0 {
            return Ok(());
        }
        // Use try_lock to avoid blocking the GTK main thread
        if let Ok(data) = self.data.try_lock() {
            data.combo.transform.apply(cr, width, height);
            render_industrial_frame(cr, &data.config.frame, width, height)?;
            data.combo.transform.restore(cr);
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
                    display_data.combo.dirty = true;
                }
                return Ok(());
            }
        }

        // Apply individual settings for backward compatibility
        if let Ok(mut display_data) = self.data.lock() {
            if let Some(animation) = config.get("animation_enabled").and_then(|v| v.as_bool()) {
                display_data.config.animation_enabled = animation;
            }
            display_data.combo.dirty = true;
        }

        Ok(())
    }

    fn needs_redraw(&self) -> bool {
        // Use try_lock to avoid blocking the GTK main thread
        self.data.try_lock().map(|data| data.combo.dirty).unwrap_or(true)
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
