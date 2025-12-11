//! Analog clock displayer - displays time as a traditional clock face

use anyhow::Result;
use cairo::Context;
use gtk4::{glib, prelude::*, DrawingArea, Widget};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::core::{ConfigOption, ConfigSchema, Displayer};
use crate::ui::clock_display::{render_analog_clock, AnalogClockConfig};

/// Analog clock displayer
pub struct ClockAnalogDisplayer {
    id: String,
    name: String,
    data: Arc<Mutex<DisplayData>>,
}

#[derive(Clone)]
struct DisplayData {
    config: AnalogClockConfig,
    hour: f64,
    minute: f64,
    second: f64,
    alarm_triggered: bool,
    alarm_enabled: bool,
    timer_state: String,
    timer_display: String,
    flash_state: bool,
}

impl ClockAnalogDisplayer {
    pub fn new() -> Self {
        let data = Arc::new(Mutex::new(DisplayData {
            config: AnalogClockConfig::default(),
            hour: 0.0,
            minute: 0.0,
            second: 0.0,
            alarm_triggered: false,
            alarm_enabled: false,
            timer_state: "stopped".to_string(),
            timer_display: String::new(),
            flash_state: false,
        }));

        Self {
            id: "clock_analog".to_string(),
            name: "Analog Clock".to_string(),
            data,
        }
    }
}

impl Default for ClockAnalogDisplayer {
    fn default() -> Self {
        Self::new()
    }
}

impl Displayer for ClockAnalogDisplayer {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn create_widget(&self) -> Widget {
        let drawing_area = DrawingArea::new();
        drawing_area.set_size_request(150, 150);

        let data_clone = self.data.clone();
        drawing_area.set_draw_func(move |_, cr, width, height| {
            if let Ok(data) = data_clone.lock() {
                // Calculate smooth time values
                let hour = data.hour;
                let minute = data.minute;
                let second = data.second;

                let _ = render_analog_clock(
                    cr,
                    &data.config,
                    hour,
                    minute,
                    second,
                    width as f64,
                    height as f64,
                );

                // Flash effect when alarm/timer triggers
                let show_flash = (data.alarm_triggered || data.timer_state == "finished") && data.flash_state;
                if show_flash {
                    cr.save().ok();
                    cr.set_source_rgba(1.0, 0.3, 0.3, 0.4);
                    cr.arc(
                        width as f64 / 2.0,
                        height as f64 / 2.0,
                        width.min(height) as f64 / 2.0 - 5.0,
                        0.0,
                        2.0 * std::f64::consts::PI,
                    );
                    cr.fill().ok();
                    cr.restore().ok();
                }

                // Bottom-right corner: show timer countdown when running/paused/finished, or icon when idle
                let timer_active = data.timer_state == "running" || data.timer_state == "paused" || data.timer_state == "finished";

                // Get icon config from data.config
                let icon_font = &data.config.icon_font;
                let icon_text = &data.config.icon_text;
                let icon_size_pct = data.config.icon_size;
                let icon_bold = data.config.icon_bold;
                let font_weight = if icon_bold { cairo::FontWeight::Bold } else { cairo::FontWeight::Normal };

                if timer_active && !data.timer_display.is_empty() {
                    // Show countdown timer text
                    cr.save().ok();

                    let font_size = (width.min(height) as f64 * icon_size_pct / 100.0).max(12.0).min(24.0);
                    cr.select_font_face(icon_font, cairo::FontSlant::Normal, cairo::FontWeight::Bold);
                    cr.set_font_size(font_size);

                    let (text_w, text_h) = if let Ok(te) = cr.text_extents(&data.timer_display) {
                        (te.width(), te.height())
                    } else {
                        (50.0, 12.0) // Fallback dimensions
                    };
                    let text_x = width as f64 - text_w - 6.0;
                    let text_y = height as f64 - 6.0;

                    // Background for readability
                    cr.set_source_rgba(0.0, 0.0, 0.0, 0.6);
                    cr.rectangle(
                        text_x - 4.0,
                        text_y - text_h - 2.0,
                        text_w + 8.0,
                        text_h + 6.0,
                    );
                    cr.fill().ok();

                    // Timer text color based on state
                    if data.timer_state == "finished" {
                        if data.flash_state {
                            cr.set_source_rgba(1.0, 0.3, 0.3, 1.0); // Red when flashing
                        } else {
                            cr.set_source_rgba(1.0, 0.6, 0.3, 1.0); // Orange
                        }
                    } else if data.timer_state == "paused" {
                        cr.set_source_rgba(1.0, 0.9, 0.3, 1.0); // Yellow for paused
                    } else {
                        cr.set_source_rgba(0.3, 1.0, 0.5, 1.0); // Green for running
                    }

                    cr.move_to(text_x, text_y);
                    cr.show_text(&data.timer_display).ok();
                    cr.restore().ok();
                } else {
                    // Show custom icon when no timer is active
                    cr.save().ok();

                    let icon_size = (width.min(height) as f64 * icon_size_pct / 100.0).max(14.0).min(28.0);
                    cr.select_font_face(icon_font, cairo::FontSlant::Normal, font_weight);
                    cr.set_font_size(icon_size);

                    let icon_w = if let Ok(te) = cr.text_extents(icon_text) {
                        te.width()
                    } else {
                        icon_size * 0.8 // Fallback width
                    };
                    let icon_x = width as f64 - icon_w - 6.0;
                    let icon_y = height as f64 - 6.0;

                    // Color based on state
                    if data.alarm_triggered {
                        if data.flash_state {
                            cr.set_source_rgba(1.0, 0.3, 0.3, 1.0);
                        } else {
                            cr.set_source_rgba(1.0, 0.6, 0.3, 1.0);
                        }
                    } else if data.alarm_enabled {
                        cr.set_source_rgba(0.3, 0.7, 1.0, 1.0); // Blue for alarm enabled
                    } else {
                        cr.set_source_rgba(0.6, 0.6, 0.6, 0.8); // Gray for inactive
                    }

                    cr.move_to(icon_x, icon_y);
                    cr.show_text(icon_text).ok();
                    cr.restore().ok();
                }
            }
        });

        // Note: Click handling for alarm/timer icon is done in grid_layout.rs

        // Animation timer for smooth second hand and flash effect
        let data_for_timer = self.data.clone();
        let drawing_area_weak = drawing_area.downgrade();
        let mut flash_counter = 0u32;
        glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
            if let Some(da) = drawing_area_weak.upgrade() {
                let needs_redraw = if let Ok(mut data) = data_for_timer.lock() {
                    // Toggle flash state every 500ms (10 * 50ms)
                    flash_counter += 1;
                    let mut redraw = false;

                    if flash_counter >= 10 {
                        flash_counter = 0;
                        // Only toggle flash if alarm or timer is active
                        if data.alarm_triggered || data.timer_state == "finished" {
                            data.flash_state = !data.flash_state;
                            redraw = true;
                        }
                    }

                    // Need smooth redraw if smooth_seconds is enabled and show_second_hand is true
                    if data.config.smooth_seconds && data.config.show_second_hand {
                        redraw = true;
                    }

                    redraw
                } else {
                    false
                };

                if needs_redraw {
                    da.queue_draw();
                }
                glib::ControlFlow::Continue
            } else {
                glib::ControlFlow::Break
            }
        });

        drawing_area.upcast()
    }

    fn update_data(&mut self, values: &HashMap<String, Value>) {
        if let Ok(mut data) = self.data.lock() {
            // Get time components with fractional parts for smooth movement
            if let Some(hour_val) = values.get("hour_value") {
                if let Some(h) = hour_val.as_f64() {
                    data.hour = h * 12.0; // Convert 0-1 to 0-12
                }
            } else if let Some(hour) = values.get("hour") {
                if let Some(h) = hour.as_f64() {
                    let minute = values.get("minute").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    data.hour = (h % 12.0) + minute / 60.0;
                }
            }

            if let Some(minute_val) = values.get("minute_value") {
                if let Some(m) = minute_val.as_f64() {
                    data.minute = m * 60.0; // Convert 0-1 to 0-60
                }
            } else if let Some(minute) = values.get("minute") {
                if let Some(m) = minute.as_f64() {
                    let second = values.get("second").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    data.minute = m + second / 60.0;
                }
            }

            if let Some(second_val) = values.get("second_value") {
                if let Some(s) = second_val.as_f64() {
                    data.second = s * 60.0; // Convert 0-1 to 0-60
                }
            } else if let Some(second) = values.get("second") {
                if let Some(s) = second.as_f64() {
                    let ms = values.get("millisecond").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    data.second = if data.config.smooth_seconds {
                        s + ms / 1000.0
                    } else {
                        s
                    };
                }
            }

            // Alarm state
            if let Some(alarm) = values.get("alarm_triggered") {
                data.alarm_triggered = alarm.as_bool().unwrap_or(false);
            }
            if let Some(enabled) = values.get("alarm_enabled") {
                data.alarm_enabled = enabled.as_bool().unwrap_or(false);
            }

            // Timer state and display
            if let Some(state) = values.get("timer_state") {
                if let Some(s) = state.as_str() {
                    data.timer_state = s.to_string();
                }
            }
            if let Some(display) = values.get("timer_display") {
                if let Some(d) = display.as_str() {
                    data.timer_display = d.to_string();
                }
            }
        }
    }

    fn draw(&self, cr: &Context, width: f64, height: f64) -> Result<()> {
        if let Ok(data) = self.data.lock() {
            render_analog_clock(
                cr,
                &data.config,
                data.hour,
                data.minute,
                data.second,
                width,
                height,
            )?;
        }
        Ok(())
    }

    fn config_schema(&self) -> ConfigSchema {
        ConfigSchema {
            options: vec![
                ConfigOption {
                    key: "face_style".to_string(),
                    name: "Face Style".to_string(),
                    description: "Style of the clock face".to_string(),
                    value_type: "select".to_string(),
                    default: serde_json::json!("Classic"),
                },
                ConfigOption {
                    key: "tick_style".to_string(),
                    name: "Tick Style".to_string(),
                    description: "Style of the tick marks".to_string(),
                    value_type: "select".to_string(),
                    default: serde_json::json!("Lines"),
                },
                ConfigOption {
                    key: "hand_style".to_string(),
                    name: "Hand Style".to_string(),
                    description: "Style of the clock hands".to_string(),
                    value_type: "select".to_string(),
                    default: serde_json::json!("Line"),
                },
                ConfigOption {
                    key: "show_second_hand".to_string(),
                    name: "Show Second Hand".to_string(),
                    description: "Whether to display the second hand".to_string(),
                    value_type: "boolean".to_string(),
                    default: serde_json::json!(true),
                },
                ConfigOption {
                    key: "smooth_seconds".to_string(),
                    name: "Smooth Second Hand".to_string(),
                    description: "Smooth second hand movement".to_string(),
                    value_type: "boolean".to_string(),
                    default: serde_json::json!(true),
                },
                ConfigOption {
                    key: "show_numbers".to_string(),
                    name: "Show Numbers".to_string(),
                    description: "Whether to display hour numbers".to_string(),
                    value_type: "boolean".to_string(),
                    default: serde_json::json!(true),
                },
            ],
        }
    }

    fn apply_config(&mut self, config: &HashMap<String, Value>) -> Result<()> {
        if let Ok(mut data) = self.data.lock() {
            if let Some(cfg) = config.get("analog_clock_config") {
                if let Ok(new_config) = serde_json::from_value(cfg.clone()) {
                    data.config = new_config;
                }
            }
        }
        Ok(())
    }
}
