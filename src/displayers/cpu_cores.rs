//! CPU Cores displayer - visualizes per-core CPU usage as animated bars
//!
//! This displayer works with the CPU source and displays usage bars for
//! individual CPU cores. The user can select which cores to display.

use anyhow::Result;
use cairo::Context;
use gtk4::{prelude::*, DrawingArea, Widget};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::core::{ConfigOption, ConfigSchema, Displayer, PanelTransform, register_animation, ANIMATION_SNAP_THRESHOLD};
use crate::ui::core_bars_display::{render_core_bars_with_values, CoreBarsConfig};
use crate::ui::theme::ComboThemeConfig;

/// Animation state for a single value
#[derive(Debug, Clone)]
struct AnimatedValue {
    current: f64,
    target: f64,
    first_update: bool,
}

impl Default for AnimatedValue {
    fn default() -> Self {
        Self {
            current: 0.0,
            target: 0.0,
            first_update: true,
        }
    }
}

/// CPU Cores displayer
pub struct CpuCoresDisplayer {
    id: String,
    name: String,
    data: Arc<Mutex<DisplayData>>,
}

#[derive(Clone)]
struct DisplayData {
    config: CoreBarsConfig,
    theme: ComboThemeConfig,
    core_values: Vec<AnimatedValue>, // Animated values per displayed core
    render_cache: Vec<f64>,          // Cached current values for rendering (avoids allocation per frame)
    detected_core_count: usize,      // Total cores detected from source
    source_values: HashMap<String, Value>, // Source values for text overlay
    last_update: Instant,
    transform: PanelTransform,
    dirty: bool,
}

impl Default for DisplayData {
    fn default() -> Self {
        Self {
            config: CoreBarsConfig::default(),
            theme: ComboThemeConfig::default(),
            core_values: Vec::new(),
            render_cache: Vec::new(),
            detected_core_count: 0,
            source_values: HashMap::new(),
            last_update: Instant::now(),
            transform: PanelTransform::default(),
            dirty: true,
        }
    }
}

impl CpuCoresDisplayer {
    pub fn new() -> Self {
        let data = Arc::new(Mutex::new(DisplayData::default()));

        Self {
            id: "cpu_cores".to_string(),
            name: "CPU Cores".to_string(),
            data,
        }
    }

    /// Extract core usage values from source data
    fn extract_core_values(data: &HashMap<String, Value>) -> Vec<(usize, f64)> {
        // Pre-allocate for typical core count (most systems have 8-64 cores)
        let mut cores: Vec<(usize, f64)> = Vec::with_capacity(32);

        for (key, value) in data {
            // Match keys like "core0_usage", "core1_usage", etc.
            if key.starts_with("core") && key.ends_with("_usage") {
                let index_str = key
                    .trim_start_matches("core")
                    .trim_end_matches("_usage");

                if let Ok(index) = index_str.parse::<usize>() {
                    // Sanity check: reasonable max cores to prevent memory issues
                    if index > 512 {
                        continue;
                    }
                    if let Some(usage) = value.as_f64() {
                        // Normalize from 0-100 to 0-1
                        let normalized = if usage > 1.0 {
                            usage / 100.0
                        } else {
                            usage
                        };
                        cores.push((index, normalized.clamp(0.0, 1.0)));
                    }
                }
            }
        }

        // Sort by core index
        cores.sort_by_key(|(idx, _)| *idx);
        cores
    }
}

impl Default for CpuCoresDisplayer {
    fn default() -> Self {
        Self::new()
    }
}

impl Displayer for CpuCoresDisplayer {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn create_widget(&self) -> Widget {
        let drawing_area = DrawingArea::new();
        drawing_area.set_size_request(100, 100);

        // Set up draw function
        let data_clone = self.data.clone();
        drawing_area.set_draw_func(move |_, cr, width, height| {
            if let Ok(data) = data_clone.lock() {
                data.transform.apply(cr, width as f64, height as f64);
                // Use cached render values (updated by animation timer)
                if !data.render_cache.is_empty() {
                    let _ = render_core_bars_with_values(
                        cr,
                        &data.config,
                        &data.theme,
                        &data.render_cache,
                        width as f64,
                        height as f64,
                        &data.source_values,
                    );
                }
                data.transform.restore(cr);
            }
        });

        // Register with global animation manager for centralized animation timing
        let data_for_animation = self.data.clone();
        register_animation(drawing_area.downgrade(), move || {
            // Use try_lock to avoid blocking UI thread if lock is held
            if let Ok(mut data) = data_for_animation.try_lock() {
                let now = Instant::now();
                let delta = now.duration_since(data.last_update).as_secs_f64();
                data.last_update = now;

                let mut any_animating = false;

                if data.config.animate {
                    let speed = data.config.animation_speed;

                    for val in &mut data.core_values {
                        if val.first_update {
                            val.current = val.target;
                            val.first_update = false;
                        } else {
                            let diff = (val.target - val.current).abs();
                            if diff > ANIMATION_SNAP_THRESHOLD {
                                val.current += (val.target - val.current) * (speed * delta).min(1.0);
                                any_animating = true;
                            } else {
                                val.current = val.target;
                            }
                        }
                    }
                } else {
                    // No animation - snap to target
                    for val in &mut data.core_values {
                        val.current = val.target;
                    }
                }

                // Update render cache (reuse allocation, only reallocate if size changes)
                let core_count = data.core_values.len();
                if data.render_cache.len() != core_count {
                    data.render_cache.resize(core_count, 0.0);
                }
                for i in 0..core_count {
                    data.render_cache[i] = data.core_values[i].current;
                }

                let should_redraw = data.dirty || any_animating;
                data.dirty = false;
                should_redraw
            } else {
                false
            }
        });

        drawing_area.upcast()
    }

    fn update_data(&mut self, data: &HashMap<String, Value>) {
        let all_cores = Self::extract_core_values(data);

        if let Ok(mut display_data) = self.data.lock() {
            // Update detected core count
            display_data.detected_core_count = all_cores.len();

            // Clamp config to available cores
            let max_core = if all_cores.is_empty() {
                0
            } else {
                all_cores.iter().map(|(idx, _)| *idx).max().unwrap_or(0)
            };

            let start = display_data.config.start_core.min(max_core);
            let end = display_data.config.end_core.min(max_core);

            // Get values for selected range using binary search (O(log n) per lookup)
            // all_cores is sorted by index, so binary search is efficient
            let selected_cores: Vec<f64> = (start..=end)
                .map(|idx| {
                    all_cores
                        .binary_search_by_key(&idx, |(i, _)| *i)
                        .ok()
                        .map(|pos| all_cores[pos].1)
                        .unwrap_or(0.0)
                })
                .collect();

            // Ensure we have enough AnimatedValue entries
            while display_data.core_values.len() < selected_cores.len() {
                display_data.core_values.push(AnimatedValue::default());
            }
            display_data.core_values.truncate(selected_cores.len());

            // Update target values
            for (i, value) in selected_cores.iter().enumerate() {
                display_data.core_values[i].target = *value;
            }

            // Extract source values for text overlay
            display_data.source_values = super::extract_text_values(
                data,
                &display_data.config.text_overlay.text_config,
            );

            // Extract transform from values
            display_data.transform = PanelTransform::from_values(data);

            display_data.dirty = true;
        }
    }

    fn draw(&self, cr: &Context, width: f64, height: f64) -> Result<()> {
        if let Ok(data) = self.data.lock() {
            data.transform.apply(cr, width, height);
            // Use cached render values (avoids Vec allocation per frame)
            if !data.render_cache.is_empty() {
                render_core_bars_with_values(
                    cr,
                    &data.config,
                    &data.theme,
                    &data.render_cache,
                    width,
                    height,
                    &data.source_values,
                )?;
            }
            data.transform.restore(cr);
        }
        Ok(())
    }

    fn config_schema(&self) -> ConfigSchema {
        ConfigSchema {
            options: vec![
                ConfigOption {
                    key: "start_core".to_string(),
                    name: "Start Core".to_string(),
                    description: "First CPU core to display (0-based)".to_string(),
                    value_type: "integer".to_string(),
                    default: serde_json::json!(0),
                },
                ConfigOption {
                    key: "end_core".to_string(),
                    name: "End Core".to_string(),
                    description: "Last CPU core to display (inclusive)".to_string(),
                    value_type: "integer".to_string(),
                    default: serde_json::json!(15),
                },
                ConfigOption {
                    key: "orientation".to_string(),
                    name: "Orientation".to_string(),
                    description: "Bar orientation".to_string(),
                    value_type: "enum".to_string(),
                    default: serde_json::json!("horizontal"),
                },
                ConfigOption {
                    key: "show_labels".to_string(),
                    name: "Show Labels".to_string(),
                    description: "Show core index labels".to_string(),
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
                    display_data.theme = theme;
                    display_data.dirty = true;
                }
            }
        }

        // Check for full core_bars_config first
        if let Some(config_value) = config.get("core_bars_config") {
            if let Ok(bars_config) = serde_json::from_value::<CoreBarsConfig>(config_value.clone()) {
                if let Ok(mut display_data) = self.data.lock() {
                    display_data.config = bars_config;
                    display_data.dirty = true;
                }
                return Ok(());
            }
        }

        // Apply individual settings for backward compatibility
        if let Ok(mut display_data) = self.data.lock() {
            if let Some(v) = config.get("start_core").and_then(|v| v.as_u64()) {
                display_data.config.start_core = v as usize;
            }
            if let Some(v) = config.get("end_core").and_then(|v| v.as_u64()) {
                display_data.config.end_core = v as usize;
            }
            if let Some(v) = config.get("show_labels").and_then(|v| v.as_bool()) {
                display_data.config.show_labels = v;
            }
            display_data.dirty = true;
        }

        Ok(())
    }

    fn needs_redraw(&self) -> bool {
        self.data.lock().map(|data| data.dirty).unwrap_or(false)
    }

    fn get_typed_config(&self) -> Option<crate::core::DisplayerConfig> {
        if let Ok(data) = self.data.lock() {
            Some(crate::core::DisplayerConfig::CpuCores(data.config.clone()))
        } else {
            None
        }
    }
}
