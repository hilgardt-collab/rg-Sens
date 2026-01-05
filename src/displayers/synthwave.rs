//! Synthwave/Outrun Displayer
//!
//! A retro-futuristic 80s aesthetic with:
//! - Purple/pink/cyan gradient backgrounds
//! - Neon grid lines (classic 80s grid horizon)
//! - Chrome/metallic text effects
//! - Sunset gradient accents
//! - Retro-futuristic fonts

use anyhow::Result;
use cairo::Context;
use gtk4::{prelude::*, DrawingArea, Widget};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::core::{ConfigOption, ConfigSchema, Displayer, PanelTransform};
use crate::displayers::combo_displayer_base::{
    ComboDisplayData, ContentDrawParams, draw_content_items_generic, handle_combo_update_data,
    setup_combo_animation_timer_ext,
};
use crate::ui::synthwave_display::{
    render_synthwave_frame, render_scanline_overlay, calculate_group_layouts, draw_group_dividers,
    SynthwaveFrameConfig,
};

/// Full Synthwave display configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthwaveDisplayConfig {
    /// Frame configuration
    #[serde(default)]
    pub frame: SynthwaveFrameConfig,

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

impl Default for SynthwaveDisplayConfig {
    fn default() -> Self {
        Self {
            frame: SynthwaveFrameConfig::default(),
            animation_enabled: default_animation_enabled(),
            animation_speed: default_animation_speed(),
        }
    }
}

/// Internal display data combining config with shared combo data
#[derive(Clone)]
struct DisplayData {
    config: SynthwaveDisplayConfig,
    combo: ComboDisplayData,
    // Synthwave-specific animation state
    scanline_offset: f64,
}

impl Default for DisplayData {
    fn default() -> Self {
        Self {
            config: SynthwaveDisplayConfig::default(),
            combo: ComboDisplayData::default(),
            scanline_offset: 0.0,
        }
    }
}

/// Synthwave/Outrun Displayer
pub struct SynthwaveDisplayer {
    id: String,
    name: String,
    data: Arc<Mutex<DisplayData>>,
}

impl SynthwaveDisplayer {
    pub fn new() -> Self {
        Self {
            id: "synthwave".to_string(),
            name: "Synthwave".to_string(),
            data: Arc::new(Mutex::new(DisplayData::default())),
        }
    }
}

impl Default for SynthwaveDisplayer {
    fn default() -> Self {
        Self::new()
    }
}

impl Displayer for SynthwaveDisplayer {
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

                // Draw the Synthwave frame and get content bounds
                let content_bounds = match render_synthwave_frame(cr, &data.config.frame, w, h) {
                    Ok(bounds) => bounds,
                    Err(e) => {
                        log::debug!("Synthwave frame render error (usually harmless during layout): {}", e);
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
                        |_, _, _, _, _| {},
                    );
                }

                cr.restore().ok();

                // Render scanline overlay for retro effect
                if data.config.frame.scanline_effect {
                    let _ = render_scanline_overlay(cr, &data.config.frame, w, h, data.scanline_offset);
                }

                data.combo.transform.restore(cr);
            }
        });

        // Set up animation timer with synthwave-specific scanline effect
        setup_combo_animation_timer_ext(
            &drawing_area,
            self.data.clone(),
            |d| d.config.animation_enabled,
            |d| d.config.animation_speed,
            |d| &mut d.combo,
            Some(|d: &mut DisplayData, elapsed: f64| -> bool {
                // Update scanline effect (synthwave-specific)
                if d.config.frame.scanline_effect {
                    d.scanline_offset += elapsed * 30.0;
                    if d.scanline_offset > 100.0 {
                        d.scanline_offset = 0.0;
                    }
                    true
                } else {
                    false
                }
            }),
        );

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
        if let Ok(data) = self.data.try_lock() {
            data.combo.transform.apply(cr, width, height);
            render_synthwave_frame(cr, &data.config.frame, width, height)?;
            data.combo.transform.restore(cr);
        }
        Ok(())
    }

    fn config_schema(&self) -> ConfigSchema {
        ConfigSchema {
            options: vec![
                ConfigOption {
                    key: "color_scheme".to_string(),
                    name: "Color Scheme".to_string(),
                    description: "Synthwave color palette".to_string(),
                    value_type: "string".to_string(),
                    default: serde_json::json!("sunset"),
                },
                ConfigOption {
                    key: "grid_enabled".to_string(),
                    name: "Grid Lines".to_string(),
                    description: "Enable retro grid horizon".to_string(),
                    value_type: "boolean".to_string(),
                    default: serde_json::json!(true),
                },
                ConfigOption {
                    key: "scanline_effect".to_string(),
                    name: "Scanline Effect".to_string(),
                    description: "Enable CRT scanline effect".to_string(),
                    value_type: "boolean".to_string(),
                    default: serde_json::json!(false),
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
        // Check for full synthwave_config first
        if let Some(config_value) = config.get("synthwave_config") {
            if let Ok(sw_config) = serde_json::from_value::<SynthwaveDisplayConfig>(config_value.clone()) {
                if let Ok(mut display_data) = self.data.lock() {
                    display_data.config = sw_config;
                    display_data.combo.dirty = true;
                }
                return Ok(());
            }
        }

        // Apply individual settings for backward compatibility
        if let Ok(mut display_data) = self.data.lock() {
            if let Some(scanline) = config.get("scanline_effect").and_then(|v| v.as_bool()) {
                display_data.config.frame.scanline_effect = scanline;
            }

            if let Some(animation) = config.get("animation_enabled").and_then(|v| v.as_bool()) {
                display_data.config.animation_enabled = animation;
            }

            display_data.combo.dirty = true;
        }

        Ok(())
    }

    fn needs_redraw(&self) -> bool {
        self.data.try_lock().map(|data| data.combo.dirty).unwrap_or(true)
    }

    fn get_typed_config(&self) -> Option<crate::core::DisplayerConfig> {
        if let Ok(display_data) = self.data.try_lock() {
            Some(crate::core::DisplayerConfig::Synthwave(display_data.config.clone()))
        } else {
            None
        }
    }
}
