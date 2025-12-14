//! Analog clock display rendering
//!
//! Renders a traditional analog clock face with hour, minute, and second hands.

use gtk4::cairo;
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

use crate::ui::background::Color;

/// Clock hand style
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[derive(Default)]
pub enum HandStyle {
    #[serde(rename = "line")]
    #[default]
    Line,
    #[serde(rename = "arrow")]
    Arrow,
    #[serde(rename = "sword")]
    Sword,
    #[serde(rename = "fancy")]
    Fancy,
}


/// Clock face style
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[derive(Default)]
pub enum FaceStyle {
    #[serde(rename = "minimal")]
    Minimal,
    #[serde(rename = "classic")]
    #[default]
    Classic,
    #[serde(rename = "modern")]
    Modern,
    #[serde(rename = "roman")]
    Roman,
    #[serde(rename = "numbers")]
    Numbers,
}


/// Tick mark style
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[derive(Default)]
pub enum TickStyle {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "dots")]
    Dots,
    #[serde(rename = "lines")]
    #[default]
    Lines,
    #[serde(rename = "mixed")]
    Mixed, // Lines for hours, dots for minutes
}


/// Analog clock display configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalogClockConfig {
    // Face background (uses common background system)
    #[serde(default)]
    pub face_background: crate::ui::BackgroundConfig,
    #[serde(default)]
    pub face_style: FaceStyle,
    #[serde(default = "default_border_color")]
    pub border_color: Color,
    #[serde(default = "default_border_width")]
    pub border_width: f64,

    // Tick marks
    #[serde(default)]
    pub tick_style: TickStyle,
    #[serde(default = "default_tick_color")]
    pub tick_color: Color,
    #[serde(default = "default_hour_tick_length")]
    pub hour_tick_length: f64,
    #[serde(default = "default_minute_tick_length")]
    pub minute_tick_length: f64,

    // Numbers
    #[serde(default = "default_true")]
    pub show_numbers: bool,
    #[serde(default = "default_number_color")]
    pub number_color: Color,
    #[serde(default = "default_number_font")]
    pub number_font: String,
    #[serde(default = "default_number_size")]
    pub number_size: f64,
    #[serde(default)]
    pub number_bold: bool,
    #[serde(default)]
    pub number_italic: bool,

    // Hour hand
    #[serde(default)]
    pub hour_hand_style: HandStyle,
    #[serde(default = "default_hour_hand_color")]
    pub hour_hand_color: Color,
    #[serde(default = "default_hour_hand_width")]
    pub hour_hand_width: f64,
    #[serde(default = "default_hour_hand_length")]
    pub hour_hand_length: f64,

    // Minute hand
    #[serde(default)]
    pub minute_hand_style: HandStyle,
    #[serde(default = "default_minute_hand_color")]
    pub minute_hand_color: Color,
    #[serde(default = "default_minute_hand_width")]
    pub minute_hand_width: f64,
    #[serde(default = "default_minute_hand_length")]
    pub minute_hand_length: f64,

    // Second hand
    #[serde(default = "default_true")]
    pub show_second_hand: bool,
    #[serde(default)]
    pub second_hand_style: HandStyle,
    #[serde(default = "default_second_hand_color")]
    pub second_hand_color: Color,
    #[serde(default = "default_second_hand_width")]
    pub second_hand_width: f64,
    #[serde(default = "default_second_hand_length")]
    pub second_hand_length: f64,

    // Center hub
    #[serde(default = "default_true")]
    pub show_center_hub: bool,
    #[serde(default = "default_center_hub_color")]
    pub center_hub_color: Color,
    #[serde(default = "default_center_hub_size")]
    pub center_hub_size: f64,

    // Smooth movement
    #[serde(default = "default_true")]
    pub smooth_seconds: bool,

    // Alarm/Timer icon
    #[serde(default = "default_true")]
    pub show_icon: bool,
    #[serde(default = "default_icon_text")]
    pub icon_text: String,
    #[serde(default = "default_icon_font")]
    pub icon_font: String,
    #[serde(default = "default_icon_size")]
    pub icon_size: f64, // As percentage of panel size (5-30%)
    #[serde(default)]
    pub icon_bold: bool,
}

fn default_face_color() -> Color {
    Color::new(0.15, 0.15, 0.15, 1.0)
}

fn default_border_color() -> Color {
    Color::new(0.5, 0.5, 0.5, 1.0)
}

fn default_border_width() -> f64 {
    3.0
}

fn default_tick_color() -> Color {
    Color::new(0.7, 0.7, 0.7, 1.0)
}

fn default_hour_tick_length() -> f64 {
    0.1 // As fraction of radius
}

fn default_minute_tick_length() -> f64 {
    0.05
}

fn default_true() -> bool {
    true
}

fn default_number_color() -> Color {
    Color::new(0.9, 0.9, 0.9, 1.0)
}

fn default_number_font() -> String {
    "Sans".to_string()
}

fn default_number_size() -> f64 {
    0.12 // As fraction of radius
}

fn default_hour_hand_color() -> Color {
    Color::new(0.9, 0.9, 0.9, 1.0)
}

fn default_hour_hand_width() -> f64 {
    6.0
}

fn default_hour_hand_length() -> f64 {
    0.5 // As fraction of radius
}

fn default_minute_hand_color() -> Color {
    Color::new(0.9, 0.9, 0.9, 1.0)
}

fn default_minute_hand_width() -> f64 {
    4.0
}

fn default_minute_hand_length() -> f64 {
    0.75
}

fn default_second_hand_color() -> Color {
    Color::new(1.0, 0.3, 0.3, 1.0)
}

fn default_second_hand_width() -> f64 {
    2.0
}

fn default_second_hand_length() -> f64 {
    0.85
}

fn default_center_hub_color() -> Color {
    Color::new(0.8, 0.8, 0.8, 1.0)
}

fn default_center_hub_size() -> f64 {
    0.05 // As fraction of radius
}

fn default_icon_text() -> String {
    "\u{23f1}\u{fe0f}".to_string() // ⏱️
}

fn default_icon_font() -> String {
    "Sans".to_string()
}

fn default_icon_size() -> f64 {
    12.0 // As percentage of panel size
}

impl Default for AnalogClockConfig {
    fn default() -> Self {
        // Create a default circular background with dark color
        let face_background = crate::ui::BackgroundConfig {
            background: crate::ui::BackgroundType::Solid {
                color: default_face_color(),
            },
        };

        Self {
            face_background,
            face_style: FaceStyle::default(),
            border_color: default_border_color(),
            border_width: default_border_width(),
            tick_style: TickStyle::default(),
            tick_color: default_tick_color(),
            hour_tick_length: default_hour_tick_length(),
            minute_tick_length: default_minute_tick_length(),
            show_numbers: true,
            number_color: default_number_color(),
            number_font: default_number_font(),
            number_size: default_number_size(),
            number_bold: true,
            number_italic: false,
            hour_hand_style: HandStyle::default(),
            hour_hand_color: default_hour_hand_color(),
            hour_hand_width: default_hour_hand_width(),
            hour_hand_length: default_hour_hand_length(),
            minute_hand_style: HandStyle::default(),
            minute_hand_color: default_minute_hand_color(),
            minute_hand_width: default_minute_hand_width(),
            minute_hand_length: default_minute_hand_length(),
            show_second_hand: true,
            second_hand_style: HandStyle::default(),
            second_hand_color: default_second_hand_color(),
            second_hand_width: default_second_hand_width(),
            second_hand_length: default_second_hand_length(),
            show_center_hub: true,
            center_hub_color: default_center_hub_color(),
            center_hub_size: default_center_hub_size(),
            smooth_seconds: true,
            show_icon: true,
            icon_text: default_icon_text(),
            icon_font: default_icon_font(),
            icon_size: default_icon_size(),
            icon_bold: false,
        }
    }
}

/// Render an analog clock
pub fn render_analog_clock(
    cr: &cairo::Context,
    config: &AnalogClockConfig,
    hour: f64,      // 0-11 with fractional minutes
    minute: f64,    // 0-59 with fractional seconds
    second: f64,    // 0-59 with fractional milliseconds
    width: f64,
    height: f64,
) -> Result<(), cairo::Error> {
    let center_x = width / 2.0;
    let center_y = height / 2.0;
    let radius = (width.min(height) / 2.0) - 10.0;

    // Draw face
    draw_face(cr, config, center_x, center_y, radius)?;

    // Draw tick marks
    draw_ticks(cr, config, center_x, center_y, radius)?;

    // Draw numbers
    if config.show_numbers {
        draw_numbers(cr, config, center_x, center_y, radius)?;
    }

    // Draw hands (hour first, then minute, then second on top)
    let hour_angle = (hour / 12.0) * 2.0 * PI - PI / 2.0;
    let minute_angle = (minute / 60.0) * 2.0 * PI - PI / 2.0;
    let second_angle = (second / 60.0) * 2.0 * PI - PI / 2.0;

    draw_hand(
        cr,
        config.hour_hand_style,
        center_x,
        center_y,
        hour_angle,
        radius * config.hour_hand_length,
        config.hour_hand_width,
        &config.hour_hand_color,
    )?;

    draw_hand(
        cr,
        config.minute_hand_style,
        center_x,
        center_y,
        minute_angle,
        radius * config.minute_hand_length,
        config.minute_hand_width,
        &config.minute_hand_color,
    )?;

    if config.show_second_hand {
        draw_hand(
            cr,
            config.second_hand_style,
            center_x,
            center_y,
            second_angle,
            radius * config.second_hand_length,
            config.second_hand_width,
            &config.second_hand_color,
        )?;
    }

    // Draw center hub
    if config.show_center_hub {
        draw_center_hub(cr, config, center_x, center_y, radius)?;
    }

    Ok(())
}

fn draw_face(
    cr: &cairo::Context,
    config: &AnalogClockConfig,
    cx: f64,
    cy: f64,
    radius: f64,
) -> Result<(), cairo::Error> {
    cr.save()?;

    // Clip to circle for background
    cr.arc(cx, cy, radius, 0.0, 2.0 * PI);
    cr.clip();

    // Translate to top-left of the face area so render_background draws correctly
    cr.translate(cx - radius, cy - radius);

    // Render background using common background system
    let face_width = radius * 2.0;
    let face_height = radius * 2.0;
    let _ = crate::ui::render_background(cr, &config.face_background, face_width, face_height);

    // Restore translation before resetting clip
    cr.identity_matrix();
    cr.reset_clip();

    // Draw border
    if config.border_width > 0.0 {
        cr.arc(cx, cy, radius - config.border_width / 2.0, 0.0, 2.0 * PI);
        cr.set_source_rgba(
            config.border_color.r,
            config.border_color.g,
            config.border_color.b,
            config.border_color.a,
        );
        cr.set_line_width(config.border_width);
        cr.stroke()?;
    }

    cr.restore()?;
    Ok(())
}

fn draw_ticks(
    cr: &cairo::Context,
    config: &AnalogClockConfig,
    cx: f64,
    cy: f64,
    radius: f64,
) -> Result<(), cairo::Error> {
    if config.tick_style == TickStyle::None {
        return Ok(());
    }

    cr.save()?;
    cr.set_source_rgba(
        config.tick_color.r,
        config.tick_color.g,
        config.tick_color.b,
        config.tick_color.a,
    );

    for i in 0..60 {
        let angle = (i as f64 / 60.0) * 2.0 * PI - PI / 2.0;
        let is_hour = i % 5 == 0;

        let tick_length = if is_hour {
            radius * config.hour_tick_length
        } else {
            radius * config.minute_tick_length
        };

        let inner_radius = radius - tick_length - 5.0;
        let outer_radius = radius - 5.0;

        let cos_angle = angle.cos();
        let sin_angle = angle.sin();

        match config.tick_style {
            TickStyle::Lines => {
                cr.set_line_width(if is_hour { 3.0 } else { 1.0 });
                cr.move_to(cx + inner_radius * cos_angle, cy + inner_radius * sin_angle);
                cr.line_to(cx + outer_radius * cos_angle, cy + outer_radius * sin_angle);
                cr.stroke()?;
            }
            TickStyle::Dots => {
                let dot_radius = if is_hour { 4.0 } else { 2.0 };
                let dot_pos = radius - 10.0;
                cr.arc(
                    cx + dot_pos * cos_angle,
                    cy + dot_pos * sin_angle,
                    dot_radius,
                    0.0,
                    2.0 * PI,
                );
                cr.fill()?;
            }
            TickStyle::Mixed => {
                if is_hour {
                    cr.set_line_width(3.0);
                    cr.move_to(cx + inner_radius * cos_angle, cy + inner_radius * sin_angle);
                    cr.line_to(cx + outer_radius * cos_angle, cy + outer_radius * sin_angle);
                    cr.stroke()?;
                } else {
                    let dot_pos = radius - 8.0;
                    cr.arc(
                        cx + dot_pos * cos_angle,
                        cy + dot_pos * sin_angle,
                        1.5,
                        0.0,
                        2.0 * PI,
                    );
                    cr.fill()?;
                }
            }
            TickStyle::None => {}
        }
    }

    cr.restore()?;
    Ok(())
}

fn draw_numbers(
    cr: &cairo::Context,
    config: &AnalogClockConfig,
    cx: f64,
    cy: f64,
    radius: f64,
) -> Result<(), cairo::Error> {
    cr.save()?;

    cr.set_source_rgba(
        config.number_color.r,
        config.number_color.g,
        config.number_color.b,
        config.number_color.a,
    );

    let font_size = radius * config.number_size;
    let slant = if config.number_italic {
        cairo::FontSlant::Italic
    } else {
        cairo::FontSlant::Normal
    };
    let weight = if config.number_bold {
        cairo::FontWeight::Bold
    } else {
        cairo::FontWeight::Normal
    };
    cr.select_font_face(&config.number_font, slant, weight);
    cr.set_font_size(font_size);

    let number_radius = radius * 0.75;

    let numbers: Vec<&str> = match config.face_style {
        FaceStyle::Roman => vec!["XII", "I", "II", "III", "IV", "V", "VI", "VII", "VIII", "IX", "X", "XI"],
        FaceStyle::Minimal => vec!["12", "", "", "3", "", "", "6", "", "", "9", "", ""],
        _ => vec!["12", "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11"],
    };

    for (i, num) in numbers.iter().enumerate() {
        if num.is_empty() {
            continue;
        }

        let angle = (i as f64 / 12.0) * 2.0 * PI - PI / 2.0;
        let x = cx + number_radius * angle.cos();
        let y = cy + number_radius * angle.sin();

        let extents = cr.text_extents(num)?;
        cr.move_to(x - extents.width() / 2.0, y + extents.height() / 2.0);
        cr.show_text(num)?;
    }

    cr.restore()?;
    Ok(())
}

fn draw_hand(
    cr: &cairo::Context,
    style: HandStyle,
    cx: f64,
    cy: f64,
    angle: f64,
    length: f64,
    width: f64,
    color: &Color,
) -> Result<(), cairo::Error> {
    cr.save()?;

    cr.set_source_rgba(color.r, color.g, color.b, color.a);
    cr.set_line_cap(cairo::LineCap::Round);
    cr.set_line_join(cairo::LineJoin::Round);

    let tip_x = cx + length * angle.cos();
    let tip_y = cy + length * angle.sin();

    match style {
        HandStyle::Line => {
            cr.set_line_width(width);
            cr.move_to(cx, cy);
            cr.line_to(tip_x, tip_y);
            cr.stroke()?;
        }
        HandStyle::Arrow => {
            // Main line
            cr.set_line_width(width);
            cr.move_to(cx, cy);
            cr.line_to(tip_x, tip_y);
            cr.stroke()?;

            // Arrow head
            let arrow_size = width * 3.0;
            let arrow_angle = 0.5;
            cr.move_to(tip_x, tip_y);
            cr.line_to(
                tip_x - arrow_size * (angle - arrow_angle).cos(),
                tip_y - arrow_size * (angle - arrow_angle).sin(),
            );
            cr.move_to(tip_x, tip_y);
            cr.line_to(
                tip_x - arrow_size * (angle + arrow_angle).cos(),
                tip_y - arrow_size * (angle + arrow_angle).sin(),
            );
            cr.stroke()?;
        }
        HandStyle::Sword => {
            // Tapered hand like a sword
            let base_width = width * 1.5;
            let perp_angle = angle + PI / 2.0;

            cr.move_to(
                cx + base_width * perp_angle.cos(),
                cy + base_width * perp_angle.sin(),
            );
            cr.line_to(tip_x, tip_y);
            cr.line_to(
                cx - base_width * perp_angle.cos(),
                cy - base_width * perp_angle.sin(),
            );
            cr.close_path();
            cr.fill()?;
        }
        HandStyle::Fancy => {
            // Fancy hand with tail
            let tail_length = length * 0.2;
            let tail_x = cx - tail_length * angle.cos();
            let tail_y = cy - tail_length * angle.sin();

            // Tail
            cr.set_line_width(width * 0.5);
            cr.move_to(cx, cy);
            cr.line_to(tail_x, tail_y);
            cr.stroke()?;

            // Main hand
            cr.set_line_width(width);
            cr.move_to(cx, cy);
            cr.line_to(tip_x, tip_y);
            cr.stroke()?;

            // Circle near tip
            let circle_pos = 0.7;
            cr.arc(
                cx + length * circle_pos * angle.cos(),
                cy + length * circle_pos * angle.sin(),
                width,
                0.0,
                2.0 * PI,
            );
            cr.fill()?;
        }
    }

    cr.restore()?;
    Ok(())
}

fn draw_center_hub(
    cr: &cairo::Context,
    config: &AnalogClockConfig,
    cx: f64,
    cy: f64,
    radius: f64,
) -> Result<(), cairo::Error> {
    cr.save()?;

    let hub_radius = radius * config.center_hub_size;

    cr.arc(cx, cy, hub_radius, 0.0, 2.0 * PI);
    cr.set_source_rgba(
        config.center_hub_color.r,
        config.center_hub_color.g,
        config.center_hub_color.b,
        config.center_hub_color.a,
    );
    cr.fill()?;

    cr.restore()?;
    Ok(())
}
