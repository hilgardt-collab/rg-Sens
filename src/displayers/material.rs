//! Material Design Cards Displayer
//!
//! A clean, modern Material Design-inspired interface with:
//! - Clean white/dark cards with subtle shadows
//! - Large rounded corners
//! - Generous whitespace and padding
//! - Color-coded category headers
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
    draw_content_items_generic, handle_combo_update_data, setup_combo_animation_timer_ext,
    ComboDisplayData, ContentDrawParams,
};
use crate::ui::material_display::{
    calculate_group_layouts, draw_group_card, draw_group_dividers, render_material_frame,
    MaterialFrameConfig,
};

/// Full Material display configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialDisplayConfig {
    /// Frame configuration
    #[serde(default)]
    pub frame: MaterialFrameConfig,

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

impl Default for MaterialDisplayConfig {
    fn default() -> Self {
        Self {
            frame: MaterialFrameConfig::default(),
            animation_enabled: default_animation_enabled(),
            animation_speed: default_animation_speed(),
        }
    }
}

/// Internal display data combining config with shared combo data
#[derive(Clone)]
#[derive(Default)]
struct DisplayData {
    config: MaterialDisplayConfig,
    combo: ComboDisplayData,
}


/// Material Design Cards Displayer
pub struct MaterialDisplayer {
    id: String,
    name: String,
    data: Arc<Mutex<DisplayData>>,
}

impl MaterialDisplayer {
    pub fn new() -> Self {
        Self {
            id: "material".to_string(),
            name: "Material Cards".to_string(),
            data: Arc::new(Mutex::new(DisplayData::default())),
        }
    }
}

impl Default for MaterialDisplayer {
    fn default() -> Self {
        Self::new()
    }
}

impl Displayer for MaterialDisplayer {
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

                // Clear to transparent so panel background shows through
                cr.set_operator(cairo::Operator::Clear);
                cr.paint().ok();
                cr.set_operator(cairo::Operator::Over);

                data.combo.transform.apply(cr, w, h);

                // Draw the Material frame and get content bounds
                let content_bounds = match render_material_frame(cr, &data.config.frame, w, h) {
                    Ok(bounds) => bounds,
                    Err(e) => {
                        log::debug!("Material frame render error: {}", e);
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

                    // Draw group card/header if configured
                    let header_h = draw_group_card(cr, &data.config.frame, gx, gy, gw, gh, group_idx);

                    // Adjust content area for header
                    let content_gy = gy + header_h;
                    let content_gh = gh - header_h;

                    // Material doesn't have per-item frames, so pass no-op closure
                    let _ = draw_content_items_generic(
                        cr,
                        gx,
                        content_gy,
                        gw,
                        content_gh,
                        &format!("group{}_", group_num),
                        item_count,
                        group_idx,
                        &params,
                        |_, _, _, _, _| {}, // No item frame for Material design
                    );
                }

                cr.restore().ok();
                data.combo.transform.restore(cr);
            }
        });

        // Set up animation timer using shared helper
        setup_combo_animation_timer_ext(
            &drawing_area,
            self.data.clone(),
            |d| d.config.animation_enabled,
            |d| d.config.animation_speed,
            |d| &mut d.combo,
            None::<fn(&mut DisplayData, f64) -> bool>,
        );

        drawing_area.upcast()
    }

    fn update_data(&mut self, data: &HashMap<String, Value>) {
        if let Ok(mut display_data) = self.data.lock() {
            // Clone config values to satisfy borrow checker (MutexGuard doesn't support split borrowing)
            // group_item_counts is small (1-4 elements), content_items is needed for the duration
            let animation_enabled = display_data.config.animation_enabled;
            let group_item_counts = display_data.config.frame.group_item_counts.clone();
            let content_items = display_data.config.frame.content_items.clone();

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
            render_material_frame(cr, &data.config.frame, width, height)?;
            data.combo.transform.restore(cr);
        }
        Ok(())
    }

    fn config_schema(&self) -> ConfigSchema {
        ConfigSchema {
            options: vec![
                ConfigOption {
                    key: "theme".to_string(),
                    name: "Theme".to_string(),
                    description: "Light or dark theme".to_string(),
                    value_type: "string".to_string(),
                    default: serde_json::json!("light"),
                },
                ConfigOption {
                    key: "accent_color".to_string(),
                    name: "Accent Color".to_string(),
                    description: "Primary accent color".to_string(),
                    value_type: "color".to_string(),
                    default: serde_json::json!([0.24, 0.47, 0.96, 1.0]),
                },
                ConfigOption {
                    key: "elevation".to_string(),
                    name: "Card Elevation".to_string(),
                    description: "Shadow depth of cards".to_string(),
                    value_type: "string".to_string(),
                    default: serde_json::json!("low"),
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
        if let Some(material_config_value) = config.get("material_config") {
            if let Ok(material_config) =
                serde_json::from_value::<MaterialDisplayConfig>(material_config_value.clone())
            {
                if let Ok(mut display_data) = self.data.lock() {
                    display_data.config = material_config;
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
        self.data
            .try_lock()
            .map(|data| data.combo.dirty)
            .unwrap_or(true)
    }

    fn get_typed_config(&self) -> Option<crate::core::DisplayerConfig> {
        if let Ok(display_data) = self.data.try_lock() {
            Some(crate::core::DisplayerConfig::Material(
                display_data.config.clone(),
            ))
        } else {
            None
        }
    }
}
