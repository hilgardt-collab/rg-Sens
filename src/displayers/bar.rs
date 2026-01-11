//! Bar displayer - visualizes numeric values as bars

use anyhow::Result;
use cairo::Context;
use gtk4::{prelude::*, DrawingArea, Widget};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::core::{ConfigOption, ConfigSchema, Displayer, PanelTransform, register_animation, ANIMATION_SNAP_THRESHOLD};
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
    animated_value: f64,
    values: HashMap<String, Value>, // Text overlay field values (extracted, not full clone)
    transform: PanelTransform,
    dirty: bool, // Flag to indicate data has changed and needs redraw
    initialized: bool, // Flag to track if animated_value has been set
    last_frame_time: std::time::Instant,
}

impl BarDisplayer {
    pub fn new() -> Self {
        let data = Arc::new(Mutex::new(DisplayData {
            config: BarDisplayConfig::default(),
            value: 0.0,
            animated_value: 0.0,
            values: HashMap::new(),
            transform: PanelTransform::default(),
            dirty: true,
            initialized: false,
            last_frame_time: std::time::Instant::now(),
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
                // Use animated_value if animation is enabled, otherwise use target value
                let display_value = if data.config.smooth_animation {
                    data.animated_value
                } else {
                    data.value
                };
                data.transform.apply(cr, width as f64, height as f64);
                let _ = render_bar(cr, &data.config, &data.config.theme, display_value, &data.values, width as f64, height as f64);
                data.transform.restore(cr);
            }
        });

        // Register with global animation manager for centralized animation timing
        let data_for_animation = self.data.clone();
        register_animation(drawing_area.downgrade(), move || {
            // Use try_lock to avoid blocking UI thread if lock is held
            if let Ok(mut data) = data_for_animation.try_lock() {
                // Check if animation is in progress
                if data.config.smooth_animation && (data.animated_value - data.value).abs() > ANIMATION_SNAP_THRESHOLD {
                    // Calculate elapsed time for smooth animation
                    let now = std::time::Instant::now();
                    let elapsed = now.duration_since(data.last_frame_time).as_secs_f64();
                    data.last_frame_time = now;

                    // Animation speed: higher value = faster animation
                    // animation_speed of 1.0 means very fast, 0.1 means slow
                    let animation_speed = data.config.animation_speed.clamp(0.01, 1.0) * 10.0;
                    let delta = (data.value - data.animated_value) * animation_speed * elapsed;

                    // Apply delta with smoothing
                    data.animated_value += delta;

                    // Snap to target if close enough
                    if (data.animated_value - data.value).abs() < ANIMATION_SNAP_THRESHOLD {
                        data.animated_value = data.value;
                    }

                    true
                } else if data.dirty {
                    data.dirty = false;
                    true
                } else {
                    false
                }
            } else {
                false
            }
        });

        drawing_area.upcast()
    }

    fn update_data(&mut self, data: &HashMap<String, Value>) {
        // Use shared helper to extract and normalize value
        let normalized = super::extract_normalized_value(data);

        if let Ok(mut display_data) = self.data.lock() {
            // On first update or if animation is disabled, set animated value immediately
            if !display_data.initialized || !display_data.config.smooth_animation {
                display_data.animated_value = normalized;
                display_data.initialized = true;
            }

            display_data.value = normalized;
            // Extract only needed values for text overlay (avoids cloning entire HashMap)
            // OPTIMIZATION: Reuse existing HashMap instead of allocating new one
            // Clone line field_ids to satisfy borrow checker (small vec, cheap clone)
            let field_ids: Vec<_> = display_data.config.text_overlay.text_config.lines.iter()
                .map(|l| l.field_id.clone()).collect();
            display_data.values.clear();
            for field_id in field_ids {
                if let Some(value) = data.get(&field_id) {
                    display_data.values.insert(field_id, value.clone());
                }
            }
            // Extract transform
            display_data.transform = PanelTransform::from_values(data);
            // Mark as dirty to trigger redraw
            display_data.dirty = true;
        }
    }

    fn draw(&self, cr: &Context, width: f64, height: f64) -> Result<()> {
        if let Ok(data) = self.data.lock() {
            let display_value = if data.config.smooth_animation {
                data.animated_value
            } else {
                data.value
            };
            data.transform.apply(cr, width, height);
            render_bar(cr, &data.config, &data.config.theme, display_value, &data.values, width, height)?;
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
        // Check for global_theme update (always apply, regardless of other config)
        if let Some(theme_value) = config.get("global_theme") {
            if let Ok(theme) = serde_json::from_value(theme_value.clone()) {
                if let Ok(mut display_data) = self.data.lock() {
                    display_data.config.theme = theme;
                }
            }
        }

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
        self.data.lock().map(|data| data.dirty).unwrap_or(false)
    }

    fn get_typed_config(&self) -> Option<crate::core::DisplayerConfig> {
        if let Ok(data) = self.data.lock() {
            Some(crate::core::DisplayerConfig::Bar(data.config.clone()))
        } else {
            None
        }
    }
}
