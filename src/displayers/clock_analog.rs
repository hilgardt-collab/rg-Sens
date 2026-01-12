//! Analog clock displayer - displays time as a traditional clock face

use anyhow::Result;
use cairo::Context;
use gtk4::{prelude::*, DrawingArea, Widget};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::core::{ConfigOption, ConfigSchema, Displayer, PanelTransform, register_animation};
use crate::displayers::TextPosition;
use crate::ui::clock_display::{render_analog_clock_with_theme, AnalogClockConfig};
use crate::ui::pango_text::{pango_show_text, pango_text_extents};
use crate::ui::theme::ComboThemeConfig;

/// Analog clock displayer
pub struct ClockAnalogDisplayer {
    id: String,
    name: String,
    data: Arc<Mutex<DisplayData>>,
}

#[derive(Clone)]
struct DisplayData {
    config: AnalogClockConfig,
    theme: ComboThemeConfig,
    hour: f64,
    minute: f64,
    second: f64,
    alarm_triggered: bool,
    alarm_enabled: bool,
    next_alarm_time: Option<String>,  // Next alarm time to display (e.g., "08:30")
    next_alarm_label: Option<String>, // Optional label for next alarm
    timer_state: String,
    timer_display: String,
    flash_state: bool,
    flash_elapsed: f64, // Track elapsed time for flash toggle (every 0.5s)
    transform: PanelTransform,
    // Icon bounds for click detection (x, y, width, height) - updated on each draw
    icon_bounds: Option<(f64, f64, f64, f64)>,
    // Last known widget dimensions for bounds calculation
    last_width: f64,
    last_height: f64,
}

impl ClockAnalogDisplayer {
    pub fn new() -> Self {
        let data = Arc::new(Mutex::new(DisplayData {
            config: AnalogClockConfig::default(),
            theme: ComboThemeConfig::default(),
            hour: 0.0,
            minute: 0.0,
            second: 0.0,
            alarm_triggered: false,
            alarm_enabled: false,
            next_alarm_time: None,
            next_alarm_label: None,
            timer_state: "stopped".to_string(),
            timer_display: String::new(),
            flash_state: false,
            flash_elapsed: 0.0,
            transform: PanelTransform::default(),
            icon_bounds: None,
            last_width: 0.0,
            last_height: 0.0,
        }));

        Self {
            id: "clock_analog".to_string(),
            name: "Analog Clock".to_string(),
            data,
        }
    }

    /// Get the current icon bounds for click detection
    /// Returns (x, y, width, height) or None if icon is not shown
    pub fn get_icon_bounds(&self) -> Option<(f64, f64, f64, f64)> {
        if let Ok(data) = self.data.lock() {
            data.icon_bounds
        } else {
            None
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
            if let Ok(mut data) = data_clone.lock() {
                let width = width as f64;
                let height = height as f64;
                data.last_width = width;
                data.last_height = height;
                data.transform.apply(cr, width, height);

                // Calculate smooth time values
                let hour = data.hour;
                let minute = data.minute;
                let second = data.second;

                // Determine if indicator should be shown (icon always shown if show_icon is true)
                let timer_active = data.timer_state == "running" || data.timer_state == "paused" || data.timer_state == "finished";

                // Get indicator text and calculate its size (clone to avoid borrow conflicts)
                let icon_font = data.config.icon_font.clone();
                let icon_text = data.config.icon_text.clone();
                let icon_size_pct = data.config.icon_size;
                let icon_bold = data.config.icon_bold;
                let font_weight = if icon_bold { cairo::FontWeight::Bold } else { cairo::FontWeight::Normal };

                // Calculate indicator height for layout purposes - reserve space if shrink_for_indicator is enabled
                let indicator_height = if data.config.show_icon && data.config.shrink_for_indicator {
                    let font_size = (width.min(height) * icon_size_pct / 100.0).clamp(14.0, 32.0);
                    font_size + 16.0 // Font height + padding
                } else {
                    0.0
                };

                // Calculate clock area - shrink if icon is shown and shrink option enabled
                let (clock_width, clock_height, clock_offset_y) = if data.config.show_icon && data.config.shrink_for_indicator {
                    let available_height = height - indicator_height;
                    let clock_size = width.min(available_height);
                    (clock_size, clock_size, (available_height - clock_size) / 2.0)
                } else {
                    (width, height, 0.0)
                };

                // Draw clock in calculated area
                cr.save().ok();
                if clock_offset_y > 0.0 || clock_width < width {
                    let offset_x = (width - clock_width) / 2.0;
                    cr.translate(offset_x, clock_offset_y);
                }

                let _ = render_analog_clock_with_theme(
                    cr,
                    &data.config,
                    hour,
                    minute,
                    second,
                    clock_width,
                    clock_height,
                    Some(&data.theme),
                );

                // Flash effect when alarm/timer triggers
                let show_flash = (data.alarm_triggered || data.timer_state == "finished") && data.flash_state;
                if show_flash {
                    cr.set_source_rgba(1.0, 0.3, 0.3, 0.4);
                    cr.arc(
                        clock_width / 2.0,
                        clock_height / 2.0,
                        clock_width.min(clock_height) / 2.0 - 5.0,
                        0.0,
                        2.0 * std::f64::consts::PI,
                    );
                    cr.fill().ok();
                }

                cr.restore().ok();

                // Restore transform BEFORE drawing icon so icon is in screen coordinates
                data.transform.restore(cr);

                // Draw indicator using 3x3 grid positioning (in screen coordinates)
                if data.config.show_icon {
                    let font_size = (width.min(height) * icon_size_pct / 100.0).clamp(14.0, 32.0);

                    // Build display text based on state
                    let display_text = if timer_active && !data.timer_display.is_empty() {
                        data.timer_display.clone()
                    } else if let Some(ref next_time) = data.next_alarm_time {
                        if data.alarm_enabled {
                            format!("{} {}", icon_text, next_time)
                        } else {
                            icon_text.clone()
                        }
                    } else {
                        icon_text.clone()
                    };

                    let te = pango_text_extents(cr, &display_text, &icon_font, cairo::FontSlant::Normal, font_weight, font_size);
                    let (text_w, text_h) = (te.width().max(font_size * 3.0), te.height().max(font_size));

                    // Calculate base position from 3x3 grid
                    // For baseline-positioned text: y is the baseline, text draws ABOVE that
                    let padding = 6.0;
                    let (base_x, base_y) = match data.config.icon_position {
                        // Top row: baseline at padding + text_h so text top is at padding
                        TextPosition::TopLeft => (padding, padding + text_h),
                        TextPosition::TopCenter => ((width - text_w) / 2.0, padding + text_h),
                        TextPosition::TopRight => (width - text_w - padding, padding + text_h),
                        // Middle row: baseline at center + text_h/2 so text is vertically centered
                        TextPosition::CenterLeft => (padding, (height + text_h) / 2.0),
                        TextPosition::Center => ((width - text_w) / 2.0, (height + text_h) / 2.0),
                        TextPosition::CenterRight => (width - text_w - padding, (height + text_h) / 2.0),
                        // Bottom row: baseline at height - padding so text bottom is at height - padding
                        TextPosition::BottomLeft => (padding, height - padding),
                        TextPosition::BottomCenter => ((width - text_w) / 2.0, height - padding),
                        TextPosition::BottomRight => (width - text_w - padding, height - padding),
                    };

                    // Apply user offset
                    let text_x = base_x + data.config.icon_offset_x;
                    let text_y = base_y + data.config.icon_offset_y;

                    // Store icon bounds for click detection (with padding for easier clicking)
                    let bounds_padding = 4.0;
                    data.icon_bounds = Some((
                        text_x - bounds_padding,
                        text_y - text_h - bounds_padding,
                        text_w + bounds_padding * 2.0,
                        text_h + bounds_padding * 2.0,
                    ));

                    cr.save().ok();

                    // Background for readability
                    let show_background = timer_active || (data.next_alarm_time.is_some() && data.alarm_enabled) || data.alarm_triggered;
                    if show_background {
                        cr.set_source_rgba(0.0, 0.0, 0.0, 0.6);
                        cr.rectangle(
                            text_x - 4.0,
                            text_y - text_h - 2.0,
                            text_w + 8.0,
                            text_h + 6.0,
                        );
                        cr.fill().ok();
                    }

                    // Text color based on state
                    if timer_active {
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
                    } else if data.alarm_triggered {
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

                    cr.move_to(text_x, text_y);
                    pango_show_text(cr, &display_text, &icon_font, cairo::FontSlant::Normal, font_weight, font_size);
                    cr.restore().ok();
                } else {
                    // No icon shown, clear bounds
                    data.icon_bounds = None;
                }
            }
        });

        // Note: Click handling for alarm/timer icon is done in grid_layout.rs

        // Register with global animation manager for smooth second hand and flash effect
        let data_for_animation = self.data.clone();
        register_animation(drawing_area.downgrade(), move || {
            // Use try_lock to avoid blocking UI thread if lock is held
            if let Ok(mut data) = data_for_animation.try_lock() {
                let mut redraw = false;

                // Toggle flash state every ~500ms (using elapsed time at 60fps = ~30 frames)
                // Only track flash if alarm or timer is active
                if data.alarm_triggered || data.timer_state == "finished" {
                    data.flash_elapsed += 1.0 / 60.0; // ~16ms per frame
                    if data.flash_elapsed >= 0.5 {
                        data.flash_elapsed = 0.0;
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

            // Next alarm information
            if let Some(next_time) = values.get("next_alarm_time") {
                data.next_alarm_time = next_time.as_str().map(|s| s.to_string());
            }
            if let Some(next_label) = values.get("next_alarm_label") {
                data.next_alarm_label = next_label.as_str().map(|s| s.to_string());
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

            // Extract transform from values
            data.transform = PanelTransform::from_values(values);
        }
    }

    fn draw(&self, cr: &Context, width: f64, height: f64) -> Result<()> {
        if let Ok(data) = self.data.lock() {
            data.transform.apply(cr, width, height);
            render_analog_clock_with_theme(
                cr,
                &data.config,
                data.hour,
                data.minute,
                data.second,
                width,
                height,
                Some(&data.theme),
            )?;
            data.transform.restore(cr);
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
            // Check for global_theme in config
            if let Some(theme_value) = config.get("global_theme") {
                if let Ok(theme) = serde_json::from_value(theme_value.clone()) {
                    data.theme = theme;
                }
            }

            // Check for clock_analog_config (new format from PanelData)
            if let Some(cfg) = config.get("clock_analog_config") {
                if let Ok(new_config) = serde_json::from_value(cfg.clone()) {
                    data.config = new_config;
                    return Ok(());
                }
            }
            // Fallback: legacy key name
            if let Some(cfg) = config.get("analog_clock_config") {
                if let Ok(new_config) = serde_json::from_value(cfg.clone()) {
                    data.config = new_config;
                }
            }
        }
        Ok(())
    }

    fn get_typed_config(&self) -> Option<crate::core::DisplayerConfig> {
        if let Ok(data) = self.data.lock() {
            Some(crate::core::DisplayerConfig::ClockAnalog(data.config.clone()))
        } else {
            None
        }
    }

    fn get_icon_bounds(&self) -> Option<(f64, f64, f64, f64)> {
        if let Ok(data) = self.data.lock() {
            data.icon_bounds
        } else {
            None
        }
    }
}
