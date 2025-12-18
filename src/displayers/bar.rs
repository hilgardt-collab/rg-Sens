//! Bar displayer - visualizes numeric values as bars

use anyhow::Result;
use cairo::Context;
use gtk4::{glib, prelude::*, DrawingArea, Widget};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::core::{ConfigOption, ConfigSchema, Displayer, PanelTransform};
use crate::ui::bar_display::{render_bar, BarDisplayConfig};

/// Bar displayer
pub struct BarDisplayer {
    id: String,
    name: String,
    data: Arc<Mutex<DisplayData>>,
}

#[derive(Clone)]
struct DisplayData {
    config: BarDisplayConfig,
    value: f64,
    values: HashMap<String, Value>, // All source data for text overlay
    transform: PanelTransform,
    dirty: bool, // Flag to indicate data has changed and needs redraw
}

impl BarDisplayer {
    pub fn new() -> Self {
        let data = Arc::new(Mutex::new(DisplayData {
            config: BarDisplayConfig::default(),
            value: 0.0,
            values: HashMap::new(),
            transform: PanelTransform::default(),
            dirty: true,
        }));

        Self {
            id: "bar".to_string(),
            name: "Bar Display".to_string(),
            data,
        }
    }
}

impl Default for BarDisplayer {
    fn default() -> Self {
        Self::new()
    }
}

impl Displayer for BarDisplayer {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn create_widget(&self) -> Widget {
        let drawing_area = DrawingArea::new();

        // Set minimum size
        drawing_area.set_size_request(100, 30);

        // Set up draw function
        let data_clone = self.data.clone();
        drawing_area.set_draw_func(move |_, cr, width, height| {
            if let Ok(data) = data_clone.lock() {
                data.transform.apply(cr, width as f64, height as f64);
                let _ = render_bar(cr, &data.config, data.value, &data.values, width as f64, height as f64);
                data.transform.restore(cr);
            }
        });

        // Set up periodic redraw - only redraw when data has changed
        glib::timeout_add_local(std::time::Duration::from_millis(100), {
            let drawing_area_weak = drawing_area.downgrade();
            let data_for_timer = self.data.clone();
            move || {
                if let Some(drawing_area) = drawing_area_weak.upgrade() {
                    // Only redraw if data changed
                    // Use try_lock to avoid blocking UI thread if lock is held
                    let needs_redraw = if let Ok(mut data) = data_for_timer.try_lock() {
                        if data.dirty {
                            data.dirty = false;
                            true
                        } else {
                            false
                        }
                    } else {
                        false
                    };

                    if needs_redraw {
                        drawing_area.queue_draw();
                    }
                    glib::ControlFlow::Continue
                } else {
                    glib::ControlFlow::Break
                }
            }
        });

        drawing_area.upcast()
    }

    fn update_data(&mut self, data: &HashMap<String, Value>) {
        // Use shared helper to extract and normalize value
        let normalized = super::extract_normalized_value(data);

        if let Ok(mut display_data) = self.data.lock() {
            display_data.value = normalized;
            // Store all values for text overlay
            display_data.values = data.clone();
            // Extract transform
            display_data.transform = PanelTransform::from_values(data);
            // Mark as dirty to trigger redraw
            display_data.dirty = true;
        }
    }

    fn draw(&self, cr: &Context, width: f64, height: f64) -> Result<()> {
        if let Ok(data) = self.data.lock() {
            data.transform.apply(cr, width, height);
            render_bar(cr, &data.config, data.value, &data.values, width, height)?;
            data.transform.restore(cr);
        }
        Ok(())
    }

    fn config_schema(&self) -> ConfigSchema {
        ConfigSchema {
            options: vec![
                ConfigOption {
                    key: "style".to_string(),
                    name: "Bar Style".to_string(),
                    description: "Visual style of the bar".to_string(),
                    value_type: "enum".to_string(),
                    default: serde_json::json!("full"),
                },
                ConfigOption {
                    key: "orientation".to_string(),
                    name: "Orientation".to_string(),
                    description: "Horizontal or vertical".to_string(),
                    value_type: "enum".to_string(),
                    default: serde_json::json!("horizontal"),
                },
                ConfigOption {
                    key: "fill_direction".to_string(),
                    name: "Fill Direction".to_string(),
                    description: "Direction the bar fills".to_string(),
                    value_type: "enum".to_string(),
                    default: serde_json::json!("left_to_right"),
                },
                ConfigOption {
                    key: "show_text".to_string(),
                    name: "Show Text Overlay".to_string(),
                    description: "Display text on the bar".to_string(),
                    value_type: "boolean".to_string(),
                    default: serde_json::json!(true),
                },
            ],
        }
    }

    fn apply_config(&mut self, config: &HashMap<String, Value>) -> Result<()> {
        // Check for full bar_config first
        if let Some(bar_config_value) = config.get("bar_config") {
            if let Ok(bar_config) = serde_json::from_value::<crate::ui::BarDisplayConfig>(bar_config_value.clone()) {
                if let Ok(mut display_data) = self.data.lock() {
                    display_data.config = bar_config;
                }
                return Ok(());
            }
        }

        // Fallback: Apply individual settings for backward compatibility
        if let Some(show_text) = config.get("show_text").and_then(|v| v.as_bool()) {
            if let Ok(mut display_data) = self.data.lock() {
                display_data.config.text_overlay.enabled = show_text;
            }
        }

        Ok(())
    }

    fn needs_redraw(&self) -> bool {
        true
    }
}
