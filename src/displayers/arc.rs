//! Arc gauge displayer - visualizes numeric values as circular arc gauges

use anyhow::Result;
use cairo::Context;
use gtk4::{glib, prelude::*, DrawingArea, Widget};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::core::{ConfigOption, ConfigSchema, Displayer, PanelTransform};
use crate::ui::arc_display::{render_arc, ArcDisplayConfig};

/// Arc gauge displayer
pub struct ArcDisplayer {
    id: String,
    name: String,
    data: Arc<Mutex<DisplayData>>,
}

#[derive(Clone)]
struct DisplayData {
    config: ArcDisplayConfig,
    value: f64,
    target_value: f64,
    animated_value: f64,
    last_update: std::time::Instant,
    values: HashMap<String, Value>, // All source data for text overlay
    transform: PanelTransform,
    dirty: bool, // Flag to indicate data has changed and needs redraw
    initialized: bool, // Flag to track if animated_value has been set
}

impl ArcDisplayer {
    pub fn new() -> Self {
        let data = Arc::new(Mutex::new(DisplayData {
            config: ArcDisplayConfig::default(),
            value: 0.0,
            target_value: 0.0,
            animated_value: 0.0,
            last_update: std::time::Instant::now(),
            values: HashMap::new(),
            transform: PanelTransform::default(),
            dirty: true,
            initialized: false,
        }));

        Self {
            id: "arc".to_string(),
            name: "Arc Gauge".to_string(),
            data,
        }
    }
}

impl Default for ArcDisplayer {
    fn default() -> Self {
        Self::new()
    }
}

impl Displayer for ArcDisplayer {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn create_widget(&self) -> Widget {
        let drawing_area = DrawingArea::new();

        // Set minimum size (arc gauges look best in square layouts)
        drawing_area.set_size_request(150, 150);

        // Set up draw function
        let data_clone = self.data.clone();
        drawing_area.set_draw_func(move |_, cr, width, height| {
            if let Ok(data) = data_clone.lock() {
                data.transform.apply(cr, width as f64, height as f64);
                let display_value = if data.config.animate {
                    data.animated_value
                } else {
                    data.value
                };
                let _ = render_arc(cr, &data.config, display_value, &data.values, width as f64, height as f64);
                data.transform.restore(cr);
            }
        });

        // Set up periodic animation/redraw at 60fps
        // The timeout automatically stops when the widget is destroyed (weak reference breaks)
        glib::timeout_add_local(std::time::Duration::from_millis(16), {
            let data_clone = self.data.clone();
            let drawing_area_weak = drawing_area.downgrade();
            move || {
                // Check if widget still exists - this automatically stops the timeout
                let Some(drawing_area) = drawing_area_weak.upgrade() else {
                    return glib::ControlFlow::Break;
                };

                // Update animation state and check if redraw needed
                // Use try_lock to avoid blocking UI thread if lock is held
                let needs_redraw = if let Ok(mut data) = data_clone.try_lock() {
                    let mut redraw = false;

                    // Always calculate elapsed time since last frame to ensure smooth animation
                    let now = std::time::Instant::now();
                    let elapsed = now.duration_since(data.last_update).as_secs_f64();
                    data.last_update = now;

                    // Check if data changed (dirty flag)
                    if data.dirty {
                        data.dirty = false;
                        redraw = true;
                    }

                    // Check if animation is active
                    if data.config.animate && (data.animated_value - data.target_value).abs() > 0.001 {
                        // Calculate animation speed based on duration (prevent division by zero)
                        let animation_speed = 1.0 / data.config.animation_duration.max(0.1);
                        let delta = (data.target_value - data.animated_value) * animation_speed * elapsed;

                        // Apply easing (ease-out)
                        data.animated_value += delta;

                        // Snap to target if very close
                        if (data.animated_value - data.target_value).abs() < 0.001 {
                            data.animated_value = data.target_value;
                        }
                        redraw = true;
                    }

                    redraw
                } else {
                    false
                };

                // Only queue draw if needed
                if needs_redraw {
                    drawing_area.queue_draw();
                }
                glib::ControlFlow::Continue
            }
        });

        drawing_area.upcast()
    }

    fn update_data(&mut self, data: &HashMap<String, Value>) {
        // Use shared helper to extract and normalize value
        let new_value = super::extract_normalized_value(data);

        if let Ok(mut display_data) = self.data.lock() {
            display_data.value = new_value;
            display_data.target_value = new_value;

            // On first update or if animation is disabled, set animated value immediately
            if !display_data.initialized || !display_data.config.animate {
                display_data.animated_value = new_value;
                display_data.initialized = true;
            }

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
            render_arc(cr, &data.config, data.value, &data.values, width, height)?;
            data.transform.restore(cr);
        }
        Ok(())
    }

    fn config_schema(&self) -> ConfigSchema {
        ConfigSchema {
            options: vec![
                ConfigOption {
                    key: "start_angle".to_string(),
                    name: "Start Angle".to_string(),
                    description: "Starting angle in degrees (0 = right)".to_string(),
                    value_type: "number".to_string(),
                    default: serde_json::json!(135.0),
                },
                ConfigOption {
                    key: "end_angle".to_string(),
                    name: "End Angle".to_string(),
                    description: "Ending angle in degrees".to_string(),
                    value_type: "number".to_string(),
                    default: serde_json::json!(45.0),
                },
                ConfigOption {
                    key: "arc_width".to_string(),
                    name: "Arc Width".to_string(),
                    description: "Width of the arc as percentage of radius".to_string(),
                    value_type: "number".to_string(),
                    default: serde_json::json!(0.15),
                },
                ConfigOption {
                    key: "segmented".to_string(),
                    name: "Segmented".to_string(),
                    description: "Display as segments instead of continuous arc".to_string(),
                    value_type: "boolean".to_string(),
                    default: serde_json::json!(false),
                },
            ],
        }
    }

    fn apply_config(&mut self, config: &HashMap<String, Value>) -> Result<()> {
        // Check for full arc_config first
        if let Some(arc_config_value) = config.get("arc_config") {
            if let Ok(arc_config) = serde_json::from_value::<crate::ui::ArcDisplayConfig>(arc_config_value.clone()) {
                if let Ok(mut display_data) = self.data.lock() {
                    display_data.config = arc_config;
                }
                return Ok(());
            }
        }

        // Fallback: Apply individual settings for backward compatibility
        if let Some(segmented) = config.get("segmented").and_then(|v| v.as_bool()) {
            if let Ok(mut display_data) = self.data.lock() {
                display_data.config.segmented = segmented;
            }
        }

        Ok(())
    }

    fn needs_redraw(&self) -> bool {
        true
    }
}
