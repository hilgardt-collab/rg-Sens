//! Speedometer gauge displayer - visualizes numeric values as traditional analog gauges

use anyhow::Result;
use cairo::Context;
use gtk4::{prelude::*, DrawingArea, Widget};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::core::{
    register_animation, ConfigOption, ConfigSchema, Displayer, PanelTransform,
    ANIMATION_SNAP_THRESHOLD,
};
use crate::ui::speedometer_display::{render_speedometer_with_theme, SpeedometerConfig};
use crate::ui::theme::ComboThemeConfig;

/// Speedometer gauge displayer
pub struct SpeedometerDisplayer {
    id: String,
    name: String,
    data: Arc<Mutex<DisplayData>>,
}

#[derive(Clone)]
struct DisplayData {
    config: SpeedometerConfig,
    theme: ComboThemeConfig,
    value: f64,
    target_value: f64,
    animated_value: f64,
    last_update: std::time::Instant,
    values: HashMap<String, Value>, // All source data for text overlay
    transform: PanelTransform,
    dirty: bool,       // Flag to indicate data has changed and needs redraw
    initialized: bool, // Flag to track if animated_value has been set
}

impl SpeedometerDisplayer {
    pub fn new() -> Self {
        let data = Arc::new(Mutex::new(DisplayData {
            config: SpeedometerConfig::default(),
            theme: ComboThemeConfig::default(),
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
            id: "speedometer".to_string(),
            name: "Speedometer Gauge".to_string(),
            data,
        }
    }
}

impl Default for SpeedometerDisplayer {
    fn default() -> Self {
        Self::new()
    }
}

impl Displayer for SpeedometerDisplayer {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn create_widget(&self) -> Widget {
        let drawing_area = DrawingArea::new();

        // Set minimum size (speedometers look best in square layouts)
        drawing_area.set_size_request(200, 200);

        // Set up draw function
        let data_clone = self.data.clone();
        drawing_area.set_draw_func(move |_, cr, width, height| {
            // Use try_lock to avoid blocking GTK main thread if update is in progress
            let Ok(data) = data_clone.try_lock() else {
                return; // Skip frame if lock contention
            };
            // Ensure we start with a clean, transparent state
            // This prevents background bleed-through issues
            cr.save().ok();
            cr.set_operator(cairo::Operator::Over);

            data.transform.apply(cr, width as f64, height as f64);
            let display_value = if data.config.animate {
                data.animated_value
            } else {
                data.value
            };
            let _ = render_speedometer_with_theme(
                cr,
                &data.config,
                display_value,
                &data.values,
                width as f64,
                height as f64,
                &data.theme,
            );
            data.transform.restore(cr);

            cr.restore().ok();
        });

        // Register with global animation manager for centralized animation timing
        let data_for_animation = self.data.clone();
        register_animation(drawing_area.downgrade(), move || {
            // Update animation state and check if redraw needed
            // Use try_lock to avoid blocking UI thread if lock is held
            if let Ok(mut data) = data_for_animation.try_lock() {
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
                if data.config.animate
                    && (data.animated_value - data.target_value).abs() > ANIMATION_SNAP_THRESHOLD
                {
                    // Use exponential decay formula for smooth animation
                    // This ensures the needle reaches ~95% of target in animation_duration seconds
                    let duration = data.config.animation_duration.max(0.05);
                    // decay_rate of ~3/duration gives 95% completion in duration seconds
                    let decay_rate = 3.0 / duration;
                    let lerp_factor = 1.0 - (-decay_rate * elapsed).exp();

                    if data.config.bounce_animation {
                        // Bounce animation: overshoot then settle
                        let diff = data.target_value - data.animated_value;
                        // Use faster lerp for bounce, with slight overshoot
                        let fast_lerp = 1.0 - (-decay_rate * 2.0 * elapsed).exp();
                        data.animated_value += diff * fast_lerp;

                        // Check if we've overshot and add bounce back
                        if diff.abs() < 0.05 {
                            data.animated_value += (data.target_value - data.animated_value) * 0.15;
                        }
                    } else {
                        // Smooth ease-out animation using exponential interpolation
                        let diff = data.target_value - data.animated_value;
                        data.animated_value += diff * lerp_factor;
                    }

                    // Snap to target if very close
                    if (data.animated_value - data.target_value).abs() < ANIMATION_SNAP_THRESHOLD {
                        data.animated_value = data.target_value;
                    }
                    redraw = true;
                }

                redraw
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
            display_data.value = normalized;
            display_data.target_value = normalized;

            // On first update or if animation is disabled, set animated value immediately
            if !display_data.initialized || !display_data.config.animate {
                display_data.animated_value = normalized;
                display_data.initialized = true;
            }

            // Extract only needed values for text overlay (avoids cloning entire HashMap)
            let mut values =
                super::extract_text_values(data, &display_data.config.text_overlay.text_config);
            // Also include min/max limits for tick label calculation
            if let Some(v) = data.get("min_limit") {
                values.insert("min_limit".to_string(), v.clone());
            }
            if let Some(v) = data.get("max_limit") {
                values.insert("max_limit".to_string(), v.clone());
            }
            display_data.values = values;

            // Extract transform from values
            display_data.transform = PanelTransform::from_values(data);

            // Mark as dirty to trigger redraw
            display_data.dirty = true;
        }
    }

    fn draw(&self, cr: &Context, width: f64, height: f64) -> Result<()> {
        if let Ok(data) = self.data.lock() {
            data.transform.apply(cr, width, height);
            let display_value = if data.config.animate {
                data.animated_value
            } else {
                data.value
            };
            render_speedometer_with_theme(
                cr,
                &data.config,
                display_value,
                &data.values,
                width,
                height,
                &data.theme,
            )
            .map_err(|e| anyhow::anyhow!("Failed to render speedometer: {}", e))?;
            data.transform.restore(cr);
        }
        Ok(())
    }

    fn config_schema(&self) -> ConfigSchema {
        ConfigSchema {
            options: vec![ConfigOption {
                key: "speedometer_config".to_string(),
                name: "Speedometer Configuration".to_string(),
                description: "Configuration for speedometer display".to_string(),
                value_type: "speedometer_config".to_string(),
                default: serde_json::to_value(SpeedometerConfig::default()).unwrap_or(Value::Null),
            }],
        }
    }

    fn apply_config(&mut self, config: &HashMap<String, Value>) -> Result<()> {
        if let Ok(mut data) = self.data.lock() {
            // Handle global theme
            if let Some(theme_value) = config.get("global_theme") {
                if let Ok(theme) = serde_json::from_value(theme_value.clone()) {
                    data.theme = theme;
                    data.dirty = true;
                }
            }

            // Handle speedometer-specific config
            if let Some(config_value) = config.get("speedometer_config") {
                if let Ok(speedometer_config) = serde_json::from_value(config_value.clone()) {
                    data.config = speedometer_config;
                    data.dirty = true;
                }
            }
        }
        Ok(())
    }

    fn needs_redraw(&self) -> bool {
        self.data.lock().map(|data| data.dirty).unwrap_or(false)
    }

    fn get_typed_config(&self) -> Option<crate::core::DisplayerConfig> {
        if let Ok(data) = self.data.lock() {
            Some(crate::core::DisplayerConfig::Speedometer(
                data.config.clone(),
            ))
        } else {
            None
        }
    }
}
