//! Bar displayer - visualizes numeric values as bars

use anyhow::Result;
use cairo::Context;
use gtk4::{glib, prelude::*, DrawingArea, Widget};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::core::{ConfigOption, ConfigSchema, Displayer};
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
}

impl BarDisplayer {
    pub fn new() -> Self {
        let data = Arc::new(Mutex::new(DisplayData {
            config: BarDisplayConfig::default(),
            value: 0.0,
            values: HashMap::new(),
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
                let _ = render_bar(cr, &data.config, data.value, &data.values, width as f64, height as f64);
            }
        });

        // Set up periodic redraw
        glib::timeout_add_local(std::time::Duration::from_millis(500), {
            let drawing_area = drawing_area.clone();
            move || {
                drawing_area.queue_draw();
                glib::ControlFlow::Continue
            }
        });

        drawing_area.upcast()
    }

    fn update_data(&mut self, data: &HashMap<String, Value>) {
        // Try to find a numeric value to display
        // Look for common keys like "value", "percent", "usage", etc.
        let new_value = data
            .get("value")
            .or_else(|| data.get("percent"))
            .or_else(|| data.get("usage"))
            .or_else(|| data.get("level"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        // Get min/max limits from data source if available
        let min_limit = data
            .get("min_limit")
            .and_then(|v| v.as_f64());

        let max_limit = data
            .get("max_limit")
            .and_then(|v| v.as_f64());

        // Normalize to 0.0-1.0 range
        let normalized = if let (Some(min), Some(max)) = (min_limit, max_limit) {
            // Use min/max range if available
            if max > min {
                (new_value - min) / (max - min)
            } else {
                0.0
            }
        } else if new_value <= 1.0 {
            // Value already in 0-1 range
            new_value
        } else if new_value <= 100.0 {
            // Assume percentage (0-100)
            new_value / 100.0
        } else {
            // For values > 100 without explicit range, normalize to 0-1
            // This might not be ideal, but it's better than nothing
            0.0
        };

        if let Ok(mut display_data) = self.data.lock() {
            display_data.value = normalized.clamp(0.0, 1.0);
            // Store all values for text overlay
            display_data.values = data.clone();
        }
    }

    fn draw(&self, cr: &Context, width: f64, height: f64) -> Result<()> {
        if let Ok(data) = self.data.lock() {
            render_bar(cr, &data.config, data.value, &data.values, width, height)?;
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
