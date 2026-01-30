//! Digital clock displayer - displays time as digital text with optional date, alarm, and timer

use anyhow::Result;
use cairo::Context;
use gtk4::{cairo, prelude::*, DrawingArea, Widget};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::core::{register_animation, ConfigOption, ConfigSchema, Displayer, PanelTransform};
use crate::ui::background::Color;
use crate::ui::pango_text::{pango_show_text, pango_text_extents};

/// Digital clock display style
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum DigitalStyle {
    #[serde(rename = "simple")]
    #[default]
    Simple,
    #[serde(rename = "segment")]
    Segment, // 7-segment LED style
    #[serde(rename = "lcd")]
    LCD,
}

/// Digital clock configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DigitalClockConfig {
    #[serde(default)]
    pub style: DigitalStyle,

    // Time display
    #[serde(default = "default_time_font")]
    pub time_font: String,
    #[serde(default = "default_time_size")]
    pub time_size: f64,
    #[serde(default = "default_time_color")]
    pub time_color: Color,
    #[serde(default = "default_true")]
    pub time_bold: bool,
    #[serde(default)]
    pub time_italic: bool,

    // Date display
    #[serde(default = "default_true")]
    pub show_date: bool,
    #[serde(default = "default_date_font")]
    pub date_font: String,
    #[serde(default = "default_date_size")]
    pub date_size: f64,
    #[serde(default = "default_date_color")]
    pub date_color: Color,
    #[serde(default)]
    pub date_bold: bool,
    #[serde(default)]
    pub date_italic: bool,

    // Day name
    #[serde(default)]
    pub show_day_name: bool,

    // Timer display
    #[serde(default)]
    pub show_timer: bool,
    #[serde(default = "default_timer_color")]
    pub timer_color: Color,

    // Alarm indicator
    #[serde(default = "default_true")]
    pub show_alarm_indicator: bool,
    #[serde(default = "default_alarm_color")]
    pub alarm_color: Color,

    // Blinking colon
    #[serde(default = "default_true")]
    pub blink_colon: bool,

    // Vertical layout
    #[serde(default)]
    pub vertical_layout: bool,

    // Alarm/Timer icon
    #[serde(default = "default_true")]
    pub show_icon: bool,
    #[serde(default = "default_icon_text")]
    pub icon_text: String,
    #[serde(default = "default_icon_font")]
    pub icon_font: String,
    #[serde(default = "default_icon_size")]
    pub icon_size: f64, // In pixels
    #[serde(default)]
    pub icon_bold: bool,
}

fn default_time_font() -> String {
    "Monospace".to_string()
}

fn default_time_size() -> f64 {
    48.0
}

fn default_time_color() -> Color {
    Color::new(0.9, 0.9, 0.9, 1.0)
}

fn default_true() -> bool {
    true
}

fn default_date_font() -> String {
    "Sans".to_string()
}

fn default_date_size() -> f64 {
    16.0
}

fn default_date_color() -> Color {
    Color::new(0.7, 0.7, 0.7, 1.0)
}

fn default_timer_color() -> Color {
    Color::new(0.3, 0.8, 0.3, 1.0)
}

fn default_alarm_color() -> Color {
    Color::new(1.0, 0.3, 0.3, 1.0)
}

fn default_icon_text() -> String {
    "\u{23f1}\u{fe0f}".to_string() // ⏱️
}

fn default_icon_font() -> String {
    "Sans".to_string()
}

fn default_icon_size() -> f64 {
    16.0 // In pixels
}

impl Default for DigitalClockConfig {
    fn default() -> Self {
        Self {
            style: DigitalStyle::Simple,
            time_font: default_time_font(),
            time_size: default_time_size(),
            time_color: default_time_color(),
            time_bold: true,
            time_italic: false,
            show_date: true,
            date_font: default_date_font(),
            date_size: default_date_size(),
            date_color: default_date_color(),
            date_bold: false,
            date_italic: false,
            show_day_name: false,
            show_timer: false,
            timer_color: default_timer_color(),
            show_alarm_indicator: true,
            alarm_color: default_alarm_color(),
            blink_colon: true,
            vertical_layout: false,
            show_icon: true,
            icon_text: default_icon_text(),
            icon_font: default_icon_font(),
            icon_size: default_icon_size(),
            icon_bold: false,
        }
    }
}

/// Digital clock displayer
pub struct ClockDigitalDisplayer {
    id: String,
    name: String,
    data: Arc<Mutex<DisplayData>>,
}

#[derive(Clone)]
struct DisplayData {
    config: DigitalClockConfig,
    time_string: String,
    date_string: String,
    day_name: String,
    timer_display: String,
    timer_state: String,
    alarm_enabled: bool,
    alarm_triggered: bool,
    next_alarm_time: Option<String>, // Next alarm time to display (e.g., "08:30")
    next_alarm_label: Option<String>, // Optional label for next alarm
    second: u32,
    blink_state: bool,
    last_blink_time: Instant, // Track actual time for blink toggle (every 0.5s)
    dirty: bool,              // Flag to indicate data has changed and needs redraw
    transform: PanelTransform,
}

impl ClockDigitalDisplayer {
    pub fn new() -> Self {
        let data = Arc::new(Mutex::new(DisplayData {
            config: DigitalClockConfig::default(),
            time_string: "00:00:00".to_string(),
            date_string: String::new(),
            day_name: String::new(),
            timer_display: String::new(),
            timer_state: "stopped".to_string(),
            alarm_enabled: false,
            alarm_triggered: false,
            next_alarm_time: None,
            next_alarm_label: None,
            second: 0,
            blink_state: true,
            last_blink_time: Instant::now(),
            dirty: true,
            transform: PanelTransform::default(),
        }));

        Self {
            id: "clock_digital".to_string(),
            name: "Digital Clock".to_string(),
            data,
        }
    }
}

impl Default for ClockDigitalDisplayer {
    fn default() -> Self {
        Self::new()
    }
}

impl Displayer for ClockDigitalDisplayer {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn create_widget(&self) -> Widget {
        let drawing_area = DrawingArea::new();
        drawing_area.set_size_request(200, 80);

        let data_clone = self.data.clone();
        drawing_area.set_draw_func(move |_, cr, width, height| {
            // Use try_lock to avoid blocking GTK main thread if update is in progress
            let Ok(data) = data_clone.try_lock() else {
                return; // Skip frame if lock contention
            };
            data.transform.apply(cr, width as f64, height as f64);
            let _ = render_digital_clock(cr, &data, width as f64, height as f64);

            // Flash effect when alarm/timer triggers
            let show_flash =
                (data.alarm_triggered || data.timer_state == "finished") && data.blink_state;
            if show_flash {
                cr.save().ok();
                cr.set_source_rgba(1.0, 0.3, 0.3, 0.2);
                cr.rectangle(0.0, 0.0, width as f64, height as f64);
                cr.fill().ok();
                cr.restore().ok();
            }

            // Bottom-right corner: show timer countdown when running/paused/finished, or icon when idle
            if data.config.show_icon {
                let timer_active = data.timer_state == "running"
                    || data.timer_state == "paused"
                    || data.timer_state == "finished";

                // Get icon config from data.config
                let icon_font = &data.config.icon_font;
                let icon_text = &data.config.icon_text;
                let icon_size_px = data.config.icon_size;
                let icon_bold = data.config.icon_bold;
                let font_weight = if icon_bold {
                    cairo::FontWeight::Bold
                } else {
                    cairo::FontWeight::Normal
                };

                if timer_active && !data.timer_display.is_empty() {
                    // Show countdown timer text
                    cr.save().ok();

                    let font_size = icon_size_px.min(height as f64 * 0.2);

                    let te = pango_text_extents(
                        cr,
                        &data.timer_display,
                        icon_font,
                        cairo::FontSlant::Normal,
                        cairo::FontWeight::Bold,
                        font_size,
                    );
                    let (text_w, text_h) = (te.width().max(50.0), te.height().max(12.0));
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
                        if data.blink_state {
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
                    pango_show_text(
                        cr,
                        &data.timer_display,
                        icon_font,
                        cairo::FontSlant::Normal,
                        cairo::FontWeight::Bold,
                        font_size,
                    );
                    cr.restore().ok();
                } else {
                    // Show icon with optional next alarm time
                    cr.save().ok();

                    let icon_size = icon_size_px.min(height as f64 * 0.25);

                    // Build display text: icon + optional next alarm time
                    let display_text = if let Some(ref next_time) = data.next_alarm_time {
                        if data.alarm_enabled {
                            format!("{} {}", icon_text, next_time)
                        } else {
                            icon_text.clone()
                        }
                    } else {
                        icon_text.clone()
                    };

                    let te = pango_text_extents(
                        cr,
                        &display_text,
                        icon_font,
                        cairo::FontSlant::Normal,
                        font_weight,
                        icon_size,
                    );
                    let (text_w, text_h) = (
                        te.width().max(icon_size * 0.8),
                        te.height().max(icon_size * 0.8),
                    );
                    let text_x = width as f64 - text_w - 6.0;
                    let text_y = height as f64 - 6.0;

                    // Background for readability when showing time
                    if data.next_alarm_time.is_some() && data.alarm_enabled {
                        cr.set_source_rgba(0.0, 0.0, 0.0, 0.5);
                        cr.rectangle(
                            text_x - 4.0,
                            text_y - text_h - 2.0,
                            text_w + 8.0,
                            text_h + 6.0,
                        );
                        cr.fill().ok();
                    }

                    // Color based on state
                    if data.alarm_triggered {
                        if data.blink_state {
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
                    pango_show_text(
                        cr,
                        &display_text,
                        icon_font,
                        cairo::FontSlant::Normal,
                        font_weight,
                        icon_size,
                    );
                    cr.restore().ok();
                }
            }
            data.transform.restore(cr);
        });

        // Note: Click handling for alarm/timer icon is done in grid_layout.rs

        // Register with global animation manager for blink effect
        let data_for_animation = self.data.clone();
        register_animation(drawing_area.downgrade(), move || {
            // Use try_lock to avoid blocking UI thread if lock is held
            if let Ok(mut data) = data_for_animation.try_lock() {
                let now = Instant::now();
                let mut redraw = false;

                // Toggle blink state every 500ms using actual time tracking
                // This works correctly regardless of frame rate (60fps or idle mode 4fps)
                if now.duration_since(data.last_blink_time).as_millis() >= 500 {
                    data.last_blink_time = now;
                    data.blink_state = !data.blink_state;

                    // Redraw if blink effect is visible (alarm/timer active or blinking colon)
                    if data.alarm_triggered
                        || data.timer_state == "finished"
                        || data.timer_state == "paused"
                        || data.config.blink_colon
                    {
                        redraw = true;
                    }
                }

                // Check if data was updated (dirty flag)
                if data.dirty {
                    data.dirty = false;
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
            if let Some(time) = values.get("time") {
                if let Some(t) = time.as_str() {
                    data.time_string = t.to_string();
                }
            }

            if let Some(date) = values.get("date") {
                if let Some(d) = date.as_str() {
                    data.date_string = d.to_string();
                }
            }

            if let Some(day) = values.get("day_name") {
                if let Some(d) = day.as_str() {
                    data.day_name = d.to_string();
                }
            }

            if let Some(timer) = values.get("timer_display") {
                if let Some(t) = timer.as_str() {
                    data.timer_display = t.to_string();
                }
            }

            if let Some(state) = values.get("timer_state") {
                if let Some(s) = state.as_str() {
                    data.timer_state = s.to_string();
                }
            }

            if let Some(alarm_en) = values.get("alarm_enabled") {
                data.alarm_enabled = alarm_en.as_bool().unwrap_or(false);
            }

            if let Some(alarm_trig) = values.get("alarm_triggered") {
                data.alarm_triggered = alarm_trig.as_bool().unwrap_or(false);
            }

            // Next alarm information
            if let Some(next_time) = values.get("next_alarm_time") {
                data.next_alarm_time = next_time.as_str().map(|s| s.to_string());
            }
            if let Some(next_label) = values.get("next_alarm_label") {
                data.next_alarm_label = next_label.as_str().map(|s| s.to_string());
            }

            if let Some(sec) = values.get("second") {
                if let Some(s) = sec.as_u64() {
                    data.second = s as u32;
                }
            }

            // Extract transform from values
            data.transform = PanelTransform::from_values(values);

            // Mark as dirty to trigger redraw
            data.dirty = true;
        }
    }

    fn draw(&self, cr: &Context, width: f64, height: f64) -> Result<()> {
        if let Ok(data) = self.data.lock() {
            data.transform.apply(cr, width, height);
            render_digital_clock(cr, &data, width, height)?;
            data.transform.restore(cr);
        }
        Ok(())
    }

    fn config_schema(&self) -> ConfigSchema {
        ConfigSchema {
            options: vec![
                ConfigOption {
                    key: "style".to_string(),
                    name: "Display Style".to_string(),
                    description: "Visual style of the clock".to_string(),
                    value_type: "select".to_string(),
                    default: serde_json::json!("Simple"),
                },
                ConfigOption {
                    key: "show_date".to_string(),
                    name: "Show Date".to_string(),
                    description: "Display the date below time".to_string(),
                    value_type: "boolean".to_string(),
                    default: serde_json::json!(true),
                },
                ConfigOption {
                    key: "show_day_name".to_string(),
                    name: "Show Day Name".to_string(),
                    description: "Display the day of week".to_string(),
                    value_type: "boolean".to_string(),
                    default: serde_json::json!(false),
                },
                ConfigOption {
                    key: "show_timer".to_string(),
                    name: "Show Timer".to_string(),
                    description: "Display the timer".to_string(),
                    value_type: "boolean".to_string(),
                    default: serde_json::json!(false),
                },
                ConfigOption {
                    key: "blink_colon".to_string(),
                    name: "Blinking Colon".to_string(),
                    description: "Blink the colon separator".to_string(),
                    value_type: "boolean".to_string(),
                    default: serde_json::json!(true),
                },
                ConfigOption {
                    key: "time_size".to_string(),
                    name: "Time Font Size".to_string(),
                    description: "Size of the time display".to_string(),
                    value_type: "number".to_string(),
                    default: serde_json::json!(48.0),
                },
            ],
        }
    }

    fn apply_config(&mut self, config: &HashMap<String, Value>) -> Result<()> {
        if let Ok(mut data) = self.data.lock() {
            // Check for clock_digital_config (new format from PanelData)
            if let Some(cfg) = config.get("clock_digital_config") {
                if let Ok(new_config) = serde_json::from_value(cfg.clone()) {
                    data.config = new_config;
                    data.dirty = true; // Trigger redraw with new config
                    return Ok(());
                }
            }
            // Fallback: legacy key name
            if let Some(cfg) = config.get("digital_clock_config") {
                if let Ok(new_config) = serde_json::from_value(cfg.clone()) {
                    data.config = new_config;
                    data.dirty = true; // Trigger redraw with new config
                }
            }
        }
        Ok(())
    }

    fn get_typed_config(&self) -> Option<crate::core::DisplayerConfig> {
        if let Ok(data) = self.data.lock() {
            Some(crate::core::DisplayerConfig::ClockDigital(
                data.config.clone(),
            ))
        } else {
            None
        }
    }
}

fn render_digital_clock(
    cr: &cairo::Context,
    data: &DisplayData,
    width: f64,
    height: f64,
) -> Result<(), cairo::Error> {
    let config = &data.config;

    // Calculate layout - center content vertically
    let mut total_height = config.time_size;
    if config.show_date {
        total_height += config.date_size + 5.0;
    }
    if config.show_day_name {
        total_height += config.date_size + 5.0;
    }
    if config.show_timer && !data.timer_display.is_empty() {
        total_height += config.date_size + 5.0;
    }

    let mut y_offset = (height - total_height) / 2.0;

    // Draw time
    cr.save()?;

    let time_str = if config.blink_colon && !data.blink_state {
        data.time_string.replace(':', " ")
    } else {
        data.time_string.clone()
    };

    let font_weight = if config.time_bold {
        cairo::FontWeight::Bold
    } else {
        cairo::FontWeight::Normal
    };

    let time_slant = if config.time_italic {
        cairo::FontSlant::Italic
    } else {
        cairo::FontSlant::Normal
    };

    match config.style {
        DigitalStyle::Simple => {
            cr.set_source_rgba(
                config.time_color.r,
                config.time_color.g,
                config.time_color.b,
                config.time_color.a,
            );

            let extents = pango_text_extents(
                cr,
                &time_str,
                &config.time_font,
                time_slant,
                font_weight,
                config.time_size,
            );
            let x = (width - extents.width()) / 2.0;
            y_offset += config.time_size;
            cr.move_to(x, y_offset);
            pango_show_text(
                cr,
                &time_str,
                &config.time_font,
                time_slant,
                font_weight,
                config.time_size,
            );
        }
        DigitalStyle::Segment | DigitalStyle::LCD => {
            // Draw 7-segment style
            draw_segment_text(cr, &time_str, width, y_offset, config)?;
            y_offset += config.time_size;
        }
    }

    y_offset += 5.0;

    // Draw date
    if config.show_date && !data.date_string.is_empty() {
        let date_slant = if config.date_italic {
            cairo::FontSlant::Italic
        } else {
            cairo::FontSlant::Normal
        };
        let date_weight = if config.date_bold {
            cairo::FontWeight::Bold
        } else {
            cairo::FontWeight::Normal
        };
        cr.set_source_rgba(
            config.date_color.r,
            config.date_color.g,
            config.date_color.b,
            config.date_color.a,
        );

        let date_text = if config.show_day_name && !data.day_name.is_empty() {
            format!("{}, {}", data.day_name, data.date_string)
        } else {
            data.date_string.clone()
        };

        let extents = pango_text_extents(
            cr,
            &date_text,
            &config.date_font,
            date_slant,
            date_weight,
            config.date_size,
        );
        let x = (width - extents.width()) / 2.0;
        y_offset += config.date_size;
        cr.move_to(x, y_offset);
        pango_show_text(
            cr,
            &date_text,
            &config.date_font,
            date_slant,
            date_weight,
            config.date_size,
        );
        y_offset += 5.0;
    }

    // Draw timer
    if config.show_timer && !data.timer_display.is_empty() && data.timer_state != "stopped" {
        let timer_color = if data.timer_state == "finished" {
            // Blink when finished
            if data.blink_state {
                config.timer_color
            } else {
                Color::new(
                    config.timer_color.r,
                    config.timer_color.g,
                    config.timer_color.b,
                    0.3,
                )
            }
        } else {
            config.timer_color
        };

        cr.set_source_rgba(timer_color.r, timer_color.g, timer_color.b, timer_color.a);

        let timer_text = format!("\u{23f1} {}", data.timer_display);
        let timer_font_size = config.date_size * 1.2;
        let extents = pango_text_extents(
            cr,
            &timer_text,
            &config.time_font,
            cairo::FontSlant::Normal,
            cairo::FontWeight::Bold,
            timer_font_size,
        );
        let x = (width - extents.width()) / 2.0;
        y_offset += timer_font_size;
        cr.move_to(x, y_offset);
        pango_show_text(
            cr,
            &timer_text,
            &config.time_font,
            cairo::FontSlant::Normal,
            cairo::FontWeight::Bold,
            timer_font_size,
        );
    }

    // Draw alarm indicator
    if config.show_alarm_indicator && data.alarm_enabled {
        let alarm_color = if data.alarm_triggered && data.blink_state {
            config.alarm_color
        } else if data.alarm_triggered {
            Color::new(
                config.alarm_color.r * 0.5,
                config.alarm_color.g * 0.5,
                config.alarm_color.b * 0.5,
                config.alarm_color.a,
            )
        } else {
            Color::new(0.5, 0.5, 0.5, 0.5)
        };

        cr.set_source_rgba(alarm_color.r, alarm_color.g, alarm_color.b, alarm_color.a);
        cr.move_to(width - 25.0, 20.0);
        pango_show_text(
            cr,
            "\u{1f514}",
            "Sans",
            cairo::FontSlant::Normal,
            cairo::FontWeight::Normal,
            14.0,
        );
    }

    cr.restore()?;
    Ok(())
}

fn draw_segment_text(
    cr: &cairo::Context,
    text: &str,
    width: f64,
    y_offset: f64,
    config: &DigitalClockConfig,
) -> Result<(), cairo::Error> {
    // For segment style, we use a monospace font with a glow effect
    let time_slant = if config.time_italic {
        cairo::FontSlant::Italic
    } else {
        cairo::FontSlant::Normal
    };
    let time_weight = if config.time_bold {
        cairo::FontWeight::Bold
    } else {
        cairo::FontWeight::Normal
    };

    let extents = pango_text_extents(
        cr,
        text,
        &config.time_font,
        time_slant,
        time_weight,
        config.time_size,
    );
    let x = (width - extents.width()) / 2.0;
    let y = y_offset + config.time_size;

    // Draw background glow for LCD effect
    if config.style == DigitalStyle::LCD {
        cr.set_source_rgba(
            config.time_color.r * 0.2,
            config.time_color.g * 0.2,
            config.time_color.b * 0.2,
            0.3,
        );
        cr.move_to(x, y);
        pango_show_text(
            cr,
            "88:88:88",
            &config.time_font,
            time_slant,
            time_weight,
            config.time_size,
        ); // Show all segments dimly
    }

    // Draw glow using cardinal directions only (4 draws instead of 8)
    // This provides a visually similar glow effect with half the rendering cost
    cr.set_source_rgba(
        config.time_color.r,
        config.time_color.g,
        config.time_color.b,
        0.3,
    );
    for (dx, dy) in [(-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0)] {
        cr.move_to(x + dx, y + dy);
        pango_show_text(
            cr,
            text,
            &config.time_font,
            time_slant,
            time_weight,
            config.time_size,
        );
    }

    // Draw main text
    cr.set_source_rgba(
        config.time_color.r,
        config.time_color.g,
        config.time_color.b,
        config.time_color.a,
    );
    cr.move_to(x, y);
    pango_show_text(
        cr,
        text,
        &config.time_font,
        time_slant,
        time_weight,
        config.time_size,
    );

    Ok(())
}
