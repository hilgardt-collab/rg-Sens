//! Graph displayer implementation

use anyhow::Result;
use cairo::Context;
use crate::core::{ConfigOption, ConfigSchema, Displayer, PanelTransform, ANIMATION_FRAME_INTERVAL};
use crate::ui::graph_display::{render_graph, DataPoint, GraphDisplayConfig};
use gtk4::prelude::*;
use gtk4::{DrawingArea, Widget};
use serde_json::Value;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

/// Graph displayer - displays values over time as a line or bar chart
pub struct GraphDisplayer {
    id: String,
    name: String,
    data: Arc<Mutex<GraphData>>,
}

struct GraphData {
    config: GraphDisplayConfig,
    data_points: VecDeque<DataPoint>,
    animated_points: VecDeque<DataPoint>, // Smoothly animated version of data_points
    source_values: HashMap<String, Value>,
    transform: PanelTransform,
    start_time: f64,
    last_update_time: f64,
    last_frame_time: std::time::Instant, // For smooth animation timing
    scroll_offset: f64, // 0.0 to 1.0, represents progress toward next point position
    dirty: bool, // Flag to indicate data has changed and needs redraw
}

impl GraphDisplayer {
    pub fn new() -> Self {
        let start_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64();

        Self {
            id: "graph".to_string(),
            name: "Graph".to_string(),
            data: Arc::new(Mutex::new(GraphData {
                config: GraphDisplayConfig::default(),
                data_points: VecDeque::new(),
                animated_points: VecDeque::new(),
                source_values: HashMap::new(),
                transform: PanelTransform::default(),
                start_time,
                last_update_time: start_time,
                last_frame_time: std::time::Instant::now(),
                scroll_offset: 0.0,
                dirty: true,
            })),
        }
    }

    pub fn set_config(&self, config: GraphDisplayConfig) {
        if let Ok(mut data) = self.data.lock() {
            data.config = config;
        }
    }

    pub fn get_config(&self) -> GraphDisplayConfig {
        self.data
            .lock()
            .map(|d| d.config.clone())
            .unwrap_or_default()
    }
}

impl Default for GraphDisplayer {
    fn default() -> Self {
        Self::new()
    }
}

impl Displayer for GraphDisplayer {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn create_widget(&self) -> Widget {
        let drawing_area = DrawingArea::new();
        drawing_area.set_size_request(100, 100);
        let data = self.data.clone();

        drawing_area.set_draw_func(move |_, cr, width, height| {
            if let Ok(data_guard) = data.lock() {
                data_guard.transform.apply(cr, width as f64, height as f64);
                // Use animated points if animation is enabled, otherwise use actual data points
                let points_to_render = if data_guard.config.animate_new_points {
                    &data_guard.animated_points
                } else {
                    &data_guard.data_points
                };

                let _ = render_graph(
                    cr,
                    &data_guard.config,
                    points_to_render,
                    &data_guard.source_values,
                    width as f64,
                    height as f64,
                    data_guard.scroll_offset,
                );
                data_guard.transform.restore(cr);
            }
        });

        // Set up periodic redraw and animation updates
        // The timeout automatically stops when the widget is destroyed (weak reference breaks)
        let data_for_animation = self.data.clone();
        gtk4::glib::timeout_add_local(ANIMATION_FRAME_INTERVAL, {
            let drawing_area_weak = drawing_area.downgrade();
            move || {
                // Check if widget still exists - this automatically stops the timeout
                let Some(drawing_area) = drawing_area_weak.upgrade() else {
                    return gtk4::glib::ControlFlow::Break;
                };

                // Skip animation updates when widget is not visible (saves CPU)
                if !drawing_area.is_mapped() {
                    return gtk4::glib::ControlFlow::Continue;
                }

                // Update animation if enabled - check dirty flag and animation state
                // Use try_lock to avoid blocking UI thread if lock is held
                let needs_redraw = if let Ok(mut data_guard) = data_for_animation.try_lock() {
                        let mut redraw = false;

                        // Always calculate elapsed time since last frame to ensure smooth animation
                        let now = std::time::Instant::now();
                        let elapsed = now.duration_since(data_guard.last_frame_time).as_secs_f64();
                        data_guard.last_frame_time = now;

                        // Check if data changed (dirty flag)
                        if data_guard.dirty {
                            data_guard.dirty = false;
                            redraw = true;
                        }

                        if data_guard.config.animate_new_points {
                            let current_time = SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs_f64();

                            // Update scroll offset for smooth horizontal scrolling
                            // scroll_offset advances at rate of 1.0 per update_interval seconds
                            let update_interval = data_guard.config.update_interval.max(0.1);
                            data_guard.scroll_offset += elapsed / update_interval;

                            // Clamp scroll_offset - it will be reset when new data arrives
                            // Allow it to go slightly beyond 1.0 to handle timing variations
                            data_guard.scroll_offset = data_guard.scroll_offset.min(1.5);

                            // Always redraw when animating for smooth scrolling
                            redraw = true;

                            // Smooth interpolation factor for Y-value animation
                            let lerp_factor = 0.15;

                            // Ensure animated_points has the same length as data_points
                            let target_len = data_guard.data_points.len();
                            let animated_len = data_guard.animated_points.len();

                            // Add new points if needed (copy values to avoid borrow conflicts)
                            for i in animated_len..target_len {
                                if let Some(p) = data_guard.data_points.get(i) {
                                    let timestamp = p.timestamp; // Copy before mutable borrow
                                    data_guard.animated_points.push_back(DataPoint {
                                        value: 0.0, // Start from baseline
                                        timestamp,
                                    });
                                }
                            }

                            // Remove excess points if needed
                            while data_guard.animated_points.len() > target_len {
                                data_guard.animated_points.pop_front();
                            }

                            // Interpolate all points toward their target values
                            // Access by index to avoid intermediate Vec allocation
                            let len = data_guard.animated_points.len();
                            for i in 0..len {
                                // Get target values first (immutable borrow)
                                let (target_value, target_timestamp) = if let Some(target) = data_guard.data_points.get(i) {
                                    (target.value, target.timestamp)
                                } else {
                                    continue;
                                };
                                // Then update animated point (mutable borrow)
                                if let Some(animated) = data_guard.animated_points.get_mut(i) {
                                    animated.value += (target_value - animated.value) * lerp_factor;
                                    animated.timestamp = target_timestamp;
                                }
                            }

                            data_guard.last_update_time = current_time;
                            redraw
                        } else {
                            // Animation disabled - render uses data_points directly,
                            // no need to copy to animated_points
                            data_guard.scroll_offset = 0.0;
                            redraw // Still redraw if dirty flag was set
                        }
                    } else {
                        false
                    };

                // Only queue draw if animation actually updated
                if needs_redraw {
                    drawing_area.queue_draw();
                }
                gtk4::glib::ControlFlow::Continue
            }
        });

        drawing_area.upcast()
    }

    fn update_data(&mut self, values: &HashMap<String, Value>) {
        if let Ok(mut data) = self.data.lock() {
            // Get the current value
            if let Some(Value::Number(num)) = values.get("value") {
                if let Some(value) = num.as_f64() {
                    let current_time = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs_f64();
                    let relative_time = current_time - data.start_time;

                    // Add new data point
                    data.data_points.push_back(DataPoint {
                        value,
                        timestamp: relative_time,
                    });

                    // Remove old data points
                    while data.data_points.len() > data.config.max_data_points {
                        data.data_points.pop_front();
                    }

                    // Reset scroll offset when new data arrives for smooth continuous scrolling
                    // The graph has now "scrolled" one position, so we start fresh
                    data.scroll_offset = 0.0;
                }
            }

            // Store all source values for text overlay
            data.source_values = values.clone();

            // Extract transform from values
            data.transform = PanelTransform::from_values(values);

            // Mark as dirty to trigger redraw
            data.dirty = true;
        }
    }

    fn draw(&self, cr: &Context, width: f64, height: f64) -> Result<()> {
        if let Ok(data_guard) = self.data.lock() {
            data_guard.transform.apply(cr, width, height);
            render_graph(
                cr,
                &data_guard.config,
                &data_guard.data_points,
                &data_guard.source_values,
                width,
                height,
                data_guard.scroll_offset,
            )?;
            data_guard.transform.restore(cr);
        }
        Ok(())
    }

    fn config_schema(&self) -> ConfigSchema {
        ConfigSchema {
            options: vec![
                ConfigOption {
                    key: "graph_type".to_string(),
                    name: "Graph Type".to_string(),
                    description: "Type of graph to display".to_string(),
                    value_type: "enum".to_string(),
                    default: Value::String("Line".to_string()),
                },
                ConfigOption {
                    key: "max_data_points".to_string(),
                    name: "Max Data Points".to_string(),
                    description: "Maximum number of data points to display".to_string(),
                    value_type: "number".to_string(),
                    default: Value::Number(serde_json::Number::from(60)),
                },
                ConfigOption {
                    key: "line_width".to_string(),
                    name: "Line Width".to_string(),
                    description: "Width of the graph line".to_string(),
                    value_type: "number".to_string(),
                    default: Value::Number(serde_json::Number::from(2)),
                },
                ConfigOption {
                    key: "auto_scale".to_string(),
                    name: "Auto Scale".to_string(),
                    description: "Automatically scale the Y-axis based on data".to_string(),
                    value_type: "boolean".to_string(),
                    default: Value::Bool(true),
                },
            ],
        }
    }

    fn apply_config(&mut self, config: &HashMap<String, Value>) -> Result<()> {
        if let Some(graph_config_value) = config.get("graph_config") {
            if let Ok(graph_config) = serde_json::from_value::<GraphDisplayConfig>(graph_config_value.clone()) {
                self.set_config(graph_config);
            }
        }
        Ok(())
    }

    fn needs_redraw(&self) -> bool {
        // Always redraw for graphs since data changes frequently
        true
    }
}
