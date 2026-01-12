//! Analog clock display rendering
//!
//! Renders a traditional analog clock face with hour, minute, and second hands.

use gtk4::cairo;
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

use crate::ui::background::Color;
use crate::ui::pango_text::{pango_show_text, pango_text_extents};
use crate::ui::theme::{ColorSource, ComboThemeConfig, FontSource};

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
    #[serde(rename = "squares")]
    Squares,
    #[serde(rename = "lines")]
    #[default]
    Lines,
    #[serde(rename = "dots")]
    Dots,
    #[serde(rename = "triangles")]
    Triangles,
}

/// Icon position on a 3x3 grid
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum IconPosition {
    #[serde(rename = "top_left")]
    TopLeft,
    #[serde(rename = "top_center")]
    TopCenter,
    #[serde(rename = "top_right")]
    TopRight,
    #[serde(rename = "middle_left")]
    MiddleLeft,
    #[serde(rename = "center")]
    Center,
    #[serde(rename = "middle_right")]
    MiddleRight,
    #[serde(rename = "bottom_left")]
    BottomLeft,
    #[serde(rename = "bottom_center")]
    #[default]
    BottomCenter,
    #[serde(rename = "bottom_right")]
    BottomRight,
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
    pub border_color: ColorSource,
    #[serde(default = "default_border_width")]
    pub border_width: f64,

    // Hour tick marks
    #[serde(default)]
    pub hour_tick_style: TickStyle,
    #[serde(default = "default_tick_color")]
    pub hour_tick_color: ColorSource,
    #[serde(default = "default_hour_tick_outer_percent")]
    pub hour_tick_outer_percent: f64,
    #[serde(default = "default_hour_tick_inner_percent")]
    pub hour_tick_inner_percent: f64,

    // Minute tick marks
    #[serde(default)]
    pub minute_tick_style: TickStyle,
    #[serde(default = "default_tick_color")]
    pub minute_tick_color: ColorSource,
    #[serde(default = "default_minute_tick_outer_percent")]
    pub minute_tick_outer_percent: f64,
    #[serde(default = "default_minute_tick_inner_percent")]
    pub minute_tick_inner_percent: f64,

    // Numbers
    #[serde(default = "default_true")]
    pub show_numbers: bool,
    #[serde(default = "default_number_color")]
    pub number_color: ColorSource,
    #[serde(default = "default_number_font")]
    pub number_font: FontSource,
    #[serde(default)]
    pub number_bold: bool,
    #[serde(default)]
    pub number_italic: bool,

    // Hour hand
    #[serde(default)]
    pub hour_hand_style: HandStyle,
    #[serde(default = "default_hour_hand_color")]
    pub hour_hand_color: ColorSource,
    #[serde(default = "default_hour_hand_width")]
    pub hour_hand_width: f64,
    #[serde(default = "default_hour_hand_length")]
    pub hour_hand_length: f64,

    // Minute hand
    #[serde(default)]
    pub minute_hand_style: HandStyle,
    #[serde(default = "default_minute_hand_color")]
    pub minute_hand_color: ColorSource,
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
    pub second_hand_color: ColorSource,
    #[serde(default = "default_second_hand_width")]
    pub second_hand_width: f64,
    #[serde(default = "default_second_hand_length")]
    pub second_hand_length: f64,

    // Center hub
    #[serde(default = "default_true")]
    pub show_center_hub: bool,
    #[serde(default = "default_center_hub_color")]
    pub center_hub_color: ColorSource,
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

    // Indicator layout options
    #[serde(default = "default_true")]
    pub center_indicator: bool, // DEPRECATED: Use icon_position instead
    #[serde(default = "default_true")]
    pub shrink_for_indicator: bool, // Shrink clock face when indicator is visible

    // New icon positioning (3x3 grid with offset)
    #[serde(default)]
    pub icon_position: IconPosition,
    #[serde(default)]
    pub icon_offset_x: f64, // Pixels offset from calculated position
    #[serde(default)]
    pub icon_offset_y: f64, // Pixels offset from calculated position
}

fn default_face_color() -> Color {
    Color::new(0.15, 0.15, 0.15, 1.0)
}

fn default_border_color() -> ColorSource {
    ColorSource::custom(Color::new(0.5, 0.5, 0.5, 1.0))
}

fn default_border_width() -> f64 {
    3.0
}

fn default_tick_color() -> ColorSource {
    ColorSource::custom(Color::new(0.7, 0.7, 0.7, 1.0))
}

fn default_hour_tick_outer_percent() -> f64 {
    95.0 // As percentage of radius
}

fn default_hour_tick_inner_percent() -> f64 {
    85.0
}

fn default_minute_tick_outer_percent() -> f64 {
    95.0
}

fn default_minute_tick_inner_percent() -> f64 {
    90.0
}

fn default_true() -> bool {
    true
}

fn default_number_color() -> ColorSource {
    ColorSource::custom(Color::new(0.9, 0.9, 0.9, 1.0))
}

fn default_number_font() -> FontSource {
    FontSource::custom("Sans".to_string(), 0.12) // Size as fraction of radius
}

fn default_hour_hand_color() -> ColorSource {
    ColorSource::custom(Color::new(0.9, 0.9, 0.9, 1.0))
}

fn default_hour_hand_width() -> f64 {
    6.0
}

fn default_hour_hand_length() -> f64 {
    0.5 // As fraction of radius
}

fn default_minute_hand_color() -> ColorSource {
    ColorSource::custom(Color::new(0.9, 0.9, 0.9, 1.0))
}

fn default_minute_hand_width() -> f64 {
    4.0
}

fn default_minute_hand_length() -> f64 {
    0.75
}

fn default_second_hand_color() -> ColorSource {
    ColorSource::custom(Color::new(1.0, 0.3, 0.3, 1.0))
}

fn default_second_hand_width() -> f64 {
    2.0
}

fn default_second_hand_length() -> f64 {
    0.85
}

fn default_center_hub_color() -> ColorSource {
    ColorSource::custom(Color::new(0.8, 0.8, 0.8, 1.0))
}

fn default_center_hub_size() -> f64 {
    0.05 // As fraction of radius
}

fn default_icon_text() -> String {
    "\u{23f1}\u{fe0f}".to_string() // ⏱️
}

fn default_icon_font() -> String {
    "Noto Color Emoji".to_string()
}

fn default_icon_size() -> f64 {
    12.0 // As percentage of panel size
}

impl Default for AnalogClockConfig {
    fn default() -> Self {
        // Create a default circular background with dark color
        let face_background = crate::ui::BackgroundConfig {
            background: crate::ui::BackgroundType::Solid {
                color: ColorSource::custom(default_face_color()),
            },
        };

        Self {
            face_background,
            face_style: FaceStyle::default(),
            border_color: default_border_color(),
            border_width: default_border_width(),
            hour_tick_style: TickStyle::default(),
            hour_tick_color: default_tick_color(),
            hour_tick_outer_percent: default_hour_tick_outer_percent(),
            hour_tick_inner_percent: default_hour_tick_inner_percent(),
            minute_tick_style: TickStyle::default(),
            minute_tick_color: default_tick_color(),
            minute_tick_outer_percent: default_minute_tick_outer_percent(),
            minute_tick_inner_percent: default_minute_tick_inner_percent(),
            show_numbers: true,
            number_color: default_number_color(),
            number_font: default_number_font(),
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
            center_indicator: true,
            shrink_for_indicator: true,
            icon_position: IconPosition::default(),
            icon_offset_x: 0.0,
            icon_offset_y: 0.0,
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
    render_analog_clock_with_theme(cr, config, hour, minute, second, width, height, None)
}

/// Render an analog clock with theme support
pub fn render_analog_clock_with_theme(
    cr: &cairo::Context,
    config: &AnalogClockConfig,
    hour: f64,
    minute: f64,
    second: f64,
    width: f64,
    height: f64,
    theme: Option<&ComboThemeConfig>,
) -> Result<(), cairo::Error> {
    let center_x = width / 2.0;
    let center_y = height / 2.0;
    let radius = (width.min(height) / 2.0) - 10.0;

    // Draw face
    draw_face(cr, config, center_x, center_y, radius, theme)?;

    // Draw tick marks
    draw_ticks(cr, config, center_x, center_y, radius, theme)?;

    // Draw numbers
    if config.show_numbers {
        draw_numbers(cr, config, center_x, center_y, radius, theme)?;
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
        theme,
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
        theme,
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
            theme,
        )?;
    }

    // Draw center hub
    if config.show_center_hub {
        draw_center_hub(cr, config, center_x, center_y, radius, theme)?;
    }

    Ok(())
}

fn draw_face(
    cr: &cairo::Context,
    config: &AnalogClockConfig,
    cx: f64,
    cy: f64,
    radius: f64,
    theme: Option<&ComboThemeConfig>,
) -> Result<(), cairo::Error> {
    // Draw face background with circular clip
    cr.save()?;
    cr.arc(cx, cy, radius, 0.0, 2.0 * PI);
    cr.clip();

    // Translate to top-left of the face area so render_background draws correctly
    cr.translate(cx - radius, cy - radius);

    // Render background using common background system with theme
    let face_width = radius * 2.0;
    let face_height = radius * 2.0;
    let _ = crate::ui::render_background_with_theme(cr, &config.face_background, face_width, face_height, theme);

    cr.restore()?; // Restores both clip and translation

    // Draw border (outside the clipped region, using original transform)
    if config.border_width > 0.0 {
        cr.save()?;
        cr.arc(cx, cy, radius - config.border_width / 2.0, 0.0, 2.0 * PI);
        let default_theme = ComboThemeConfig::default();
        let theme_ref = theme.unwrap_or(&default_theme);
        let border_color = config.border_color.resolve(theme_ref);
        cr.set_source_rgba(
            border_color.r,
            border_color.g,
            border_color.b,
            border_color.a,
        );
        cr.set_line_width(config.border_width);
        cr.stroke()?;
        cr.restore()?;
    }

    Ok(())
}

fn draw_ticks(
    cr: &cairo::Context,
    config: &AnalogClockConfig,
    cx: f64,
    cy: f64,
    radius: f64,
    theme: Option<&ComboThemeConfig>,
) -> Result<(), cairo::Error> {
    let default_theme = ComboThemeConfig::default();
    let theme_ref = theme.unwrap_or(&default_theme);

    // Draw minute ticks first (so hour ticks are on top)
    if config.minute_tick_style != TickStyle::None {
        let minute_color = config.minute_tick_color.resolve(theme_ref);
        cr.save()?;
        cr.set_source_rgba(minute_color.r, minute_color.g, minute_color.b, minute_color.a);

        let outer_r = radius * (config.minute_tick_outer_percent / 100.0);
        let inner_r = radius * (config.minute_tick_inner_percent / 100.0);

        for i in 0..60 {
            // Skip hour positions (every 5 minutes)
            if i % 5 == 0 {
                continue;
            }

            let angle = (i as f64 / 60.0) * 2.0 * PI - PI / 2.0;
            draw_single_tick(cr, config.minute_tick_style, cx, cy, angle, outer_r, inner_r, 1.0)?;
        }
        cr.restore()?;
    }

    // Draw hour ticks (12 positions)
    if config.hour_tick_style != TickStyle::None {
        let hour_color = config.hour_tick_color.resolve(theme_ref);
        cr.save()?;
        cr.set_source_rgba(hour_color.r, hour_color.g, hour_color.b, hour_color.a);

        let outer_r = radius * (config.hour_tick_outer_percent / 100.0);
        let inner_r = radius * (config.hour_tick_inner_percent / 100.0);

        for i in 0..12 {
            let angle = (i as f64 / 12.0) * 2.0 * PI - PI / 2.0;
            draw_single_tick(cr, config.hour_tick_style, cx, cy, angle, outer_r, inner_r, 3.0)?;
        }
        cr.restore()?;
    }

    Ok(())
}

/// Draw a single tick mark at the given angle
fn draw_single_tick(
    cr: &cairo::Context,
    style: TickStyle,
    cx: f64,
    cy: f64,
    angle: f64,
    outer_radius: f64,
    inner_radius: f64,
    line_width: f64,
) -> Result<(), cairo::Error> {
    let cos_angle = angle.cos();
    let sin_angle = angle.sin();

    let outer_x = cx + outer_radius * cos_angle;
    let outer_y = cy + outer_radius * sin_angle;
    let inner_x = cx + inner_radius * cos_angle;
    let inner_y = cy + inner_radius * sin_angle;

    match style {
        TickStyle::Lines => {
            cr.set_line_width(line_width);
            cr.move_to(inner_x, inner_y);
            cr.line_to(outer_x, outer_y);
            cr.stroke()?;
        }
        TickStyle::Dots => {
            let mid_r = (outer_radius + inner_radius) / 2.0;
            let mid_x = cx + mid_r * cos_angle;
            let mid_y = cy + mid_r * sin_angle;
            let dot_radius = line_width * 1.5;
            cr.arc(mid_x, mid_y, dot_radius, 0.0, 2.0 * PI);
            cr.fill()?;
        }
        TickStyle::Squares => {
            let mid_r = (outer_radius + inner_radius) / 2.0;
            let mid_x = cx + mid_r * cos_angle;
            let mid_y = cy + mid_r * sin_angle;
            let size = line_width * 2.5;

            cr.save()?;
            cr.translate(mid_x, mid_y);
            cr.rotate(angle + PI / 2.0);
            cr.rectangle(-size / 2.0, -size / 2.0, size, size);
            cr.fill()?;
            cr.restore()?;
        }
        TickStyle::Triangles => {
            let mid_r = (outer_radius + inner_radius) / 2.0;
            let size = line_width * 3.0;

            cr.save()?;
            cr.translate(cx, cy);
            cr.rotate(angle + PI / 2.0);
            // Triangle pointing outward
            cr.move_to(mid_r - size / 2.0, 0.0);
            cr.line_to(mid_r + size / 2.0, -size / 2.0);
            cr.line_to(mid_r + size / 2.0, size / 2.0);
            cr.close_path();
            cr.fill()?;
            cr.restore()?;
        }
        TickStyle::None => {}
    }

    Ok(())
}

fn draw_numbers(
    cr: &cairo::Context,
    config: &AnalogClockConfig,
    cx: f64,
    cy: f64,
    radius: f64,
    theme: Option<&ComboThemeConfig>,
) -> Result<(), cairo::Error> {
    cr.save()?;

    // Resolve number color using theme
    let default_theme = ComboThemeConfig::default();
    let theme_ref = theme.unwrap_or(&default_theme);
    let number_color = config.number_color.resolve(theme_ref);
    cr.set_source_rgba(
        number_color.r,
        number_color.g,
        number_color.b,
        number_color.a,
    );

    // Resolve font using theme
    let (font_family, font_size_fraction) = config.number_font.resolve(theme_ref);
    let font_size = radius * font_size_fraction;
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

        let extents = pango_text_extents(cr, num, &font_family, slant, weight, font_size);
        cr.move_to(x - extents.width() / 2.0, y + extents.height() / 2.0);
        pango_show_text(cr, num, &font_family, slant, weight, font_size);
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
    color: &ColorSource,
    theme: Option<&ComboThemeConfig>,
) -> Result<(), cairo::Error> {
    cr.save()?;

    let default_theme = ComboThemeConfig::default();
    let theme_ref = theme.unwrap_or(&default_theme);
    let resolved_color = color.resolve(theme_ref);
    cr.set_source_rgba(resolved_color.r, resolved_color.g, resolved_color.b, resolved_color.a);
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
    theme: Option<&ComboThemeConfig>,
) -> Result<(), cairo::Error> {
    cr.save()?;

    let hub_radius = radius * config.center_hub_size;

    cr.arc(cx, cy, hub_radius, 0.0, 2.0 * PI);
    let default_theme = ComboThemeConfig::default();
    let theme_ref = theme.unwrap_or(&default_theme);
    let hub_color = config.center_hub_color.resolve(theme_ref);
    cr.set_source_rgba(hub_color.r, hub_color.g, hub_color.b, hub_color.a);
    cr.fill()?;

    cr.restore()?;
    Ok(())
}
