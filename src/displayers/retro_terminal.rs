//! Retro Terminal (CRT) Displayer
//!
//! A vintage CRT terminal aesthetic with:
//! - Green or amber phosphor text on dark background
//! - CRT scanline and curvature effects
//! - Monitor bezel frame styling
//! - Phosphor glow (screen burn) around bright elements

use anyhow::Result;
use cairo::Context;
use gtk4::{prelude::*, DrawingArea, Widget};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::core::{ConfigOption, ConfigSchema, Displayer};
use crate::displayers::combo_displayer_base::{
    ComboDisplayData, ContentDrawParams, draw_content_items_generic, handle_combo_update_data,
    setup_combo_animation_timer_ext,
};
use crate::ui::retro_terminal_display::{
    render_retro_terminal_frame, calculate_group_layouts, draw_group_dividers,
    RetroTerminalFrameConfig,
};

/// Full Retro Terminal display configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetroTerminalDisplayConfig {
    /// Frame configuration
    #[serde(default)]
    pub frame: RetroTerminalFrameConfig,

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

impl Default for RetroTerminalDisplayConfig {
    fn default() -> Self {
        Self {
            frame: RetroTerminalFrameConfig::default(),
            animation_enabled: default_animation_enabled(),
            animation_speed: default_animation_speed(),
        }
    }
}

/// Internal display data combining config with shared combo data
#[derive(Clone)]
struct DisplayData {
    config: RetroTerminalDisplayConfig,
    combo: ComboDisplayData,
    // CRT-specific animation state
    cursor_visible: bool,
    cursor_blink_time: f64,
    flicker_offset: f64,
}

impl Default for DisplayData {
    fn default() -> Self {
        Self {
            config: RetroTerminalDisplayConfig::default(),
            combo: ComboDisplayData::default(),
            cursor_visible: true,
            cursor_blink_time: 0.0,
            flicker_offset: 0.0,
        }
    }
}

/// Retro Terminal (CRT) Displayer
pub struct RetroTerminalDisplayer {
    id: String,
    name: String,
    data: Arc<Mutex<DisplayData>>,
}

impl RetroTerminalDisplayer {
    pub fn new() -> Self {
        Self {
            id: "retro_terminal".to_string(),
            name: "Retro Terminal".to_string(),
            data: Arc::new(Mutex::new(DisplayData::default())),
        }
    }
}

impl Default for RetroTerminalDisplayer {
    fn default() -> Self {
        Self::new()
    }
}

impl Displayer for RetroTerminalDisplayer {
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
            // Skip rendering if dimensions are too small
            if width < 10 || height < 10 {
                return;
            }
            if let Ok(data) = data_clone.lock() {
                let w = width as f64;
                let h = height as f64;

                // Clear to transparent so panel background shows through
                cr.set_operator(cairo::Operator::Clear);
                cr.paint().ok();
                cr.set_operator(cairo::Operator::Over);

                data.combo.transform.apply(cr, w, h);

                // Draw the Retro Terminal frame and get content bounds
                let content_bounds = match render_retro_terminal_frame(cr, &data.config.frame, w, h) {
                    Ok(bounds) => bounds,
                    Err(e) => {
                        log::debug!("Retro Terminal frame render error (usually harmless during layout): {}", e);
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
                data.combo.transform.restore(cr);
            }
        });

        // Set up animation timer with CRT-specific effects (cursor blink, flicker)
        setup_combo_animation_timer_ext(
            &drawing_area,
            self.data.clone(),
            |d| d.config.animation_enabled,
            |d| d.config.animation_speed,
            |d| &mut d.combo,
            Some(|d: &mut DisplayData, elapsed: f64| -> bool {
                let mut redraw = false;

                // Update cursor blink state (CRT-specific)
                if d.config.frame.cursor_blink {
                    d.cursor_blink_time += elapsed;
                    if d.cursor_blink_time >= 0.5 {
                        d.cursor_blink_time = 0.0;
                        d.cursor_visible = !d.cursor_visible;
                        redraw = true;
                    }
                }

                // Update flicker effect (CRT-specific)
                if d.config.frame.flicker_enabled {
                    d.flicker_offset = (rand::random::<f64>() - 0.5) * 2.0;
                    redraw = true;
                }

                redraw
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
        }
    }

    fn draw(&self, cr: &Context, width: f64, height: f64) -> Result<()> {
        // Skip rendering if dimensions are too small
        if width < 10.0 || height < 10.0 {
            return Ok(());
        }
        // Use try_lock to avoid blocking the GTK main thread
        if let Ok(data) = self.data.try_lock() {
            data.combo.transform.apply(cr, width, height);
            render_retro_terminal_frame(cr, &data.config.frame, width, height)?;
            data.combo.transform.restore(cr);
        }
        Ok(())
    }

    fn config_schema(&self) -> ConfigSchema {
        ConfigSchema {
            options: vec![
                ConfigOption {
                    key: "phosphor_color".to_string(),
                    name: "Phosphor Color".to_string(),
                    description: "Terminal phosphor color preset".to_string(),
                    value_type: "string".to_string(),
                    default: serde_json::json!("green"),
                },
                ConfigOption {
                    key: "scanline_intensity".to_string(),
                    name: "Scanline Intensity".to_string(),
                    description: "Intensity of CRT scanline effect".to_string(),
                    value_type: "number".to_string(),
                    default: serde_json::json!(0.25),
                },
                ConfigOption {
                    key: "screen_glow".to_string(),
                    name: "Screen Glow".to_string(),
                    description: "Phosphor glow/bloom intensity".to_string(),
                    value_type: "number".to_string(),
                    default: serde_json::json!(0.5),
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
        // Check for full retro_terminal_config first
        if let Some(config_value) = config.get("retro_terminal_config") {
            if let Ok(rt_config) = serde_json::from_value::<RetroTerminalDisplayConfig>(config_value.clone()) {
                if let Ok(mut display_data) = self.data.lock() {
                    display_data.config = rt_config;
                    display_data.combo.dirty = true;
                }
                return Ok(());
            }
        }

        // Apply individual settings for backward compatibility
        if let Ok(mut display_data) = self.data.lock() {
            if let Some(scanline) = config.get("scanline_intensity").and_then(|v| v.as_f64()) {
                display_data.config.frame.scanline_intensity = scanline;
            }

            if let Some(glow) = config.get("screen_glow").and_then(|v| v.as_f64()) {
                display_data.config.frame.screen_glow = glow;
            }

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
            Some(crate::core::DisplayerConfig::RetroTerminal(display_data.config.clone()))
        } else {
            None
        }
    }
}
