//! Retro Terminal (CRT) display rendering
//!
//! Provides a vintage CRT terminal aesthetic with:
//! - Green or amber phosphor text on dark background
//! - CRT scanline and curvature effects
//! - Monitor bezel frame styling
//! - Phosphor glow (screen burn) around bright elements
//! - Optional flicker and vignette effects

use anyhow::Result;
use cairo::Context;

use crate::combo_traits::FrameRenderer;
use crate::background::Color;
use crate::lcars_display::SplitOrientation;
use crate::pango_text::{pango_show_text, pango_text_extents};

// Re-export all type definitions from rg_sens_types
pub use rg_sens_types::display_configs::retro_terminal::*;

/// Frame renderer for Retro Terminal theme
pub struct RetroTerminalRenderer;

impl FrameRenderer for RetroTerminalRenderer {
    type Config = RetroTerminalFrameConfig;

    fn theme_id(&self) -> &'static str {
        "retro_terminal"
    }

    fn theme_name(&self) -> &'static str {
        "Retro Terminal"
    }

    fn default_config(&self) -> Self::Config {
        RetroTerminalFrameConfig::default()
    }

    fn render_frame(
        &self,
        cr: &Context,
        config: &Self::Config,
        width: f64,
        height: f64,
    ) -> anyhow::Result<(f64, f64, f64, f64)> {
        render_retro_terminal_frame(cr, config, width, height).map_err(|e| anyhow::anyhow!("{}", e))
    }

    fn calculate_group_layouts(
        &self,
        config: &Self::Config,
        content_x: f64,
        content_y: f64,
        content_w: f64,
        content_h: f64,
    ) -> Vec<(f64, f64, f64, f64)> {
        calculate_group_layouts(config, content_x, content_y, content_w, content_h)
    }

    fn draw_group_dividers(
        &self,
        cr: &Context,
        config: &Self::Config,
        group_layouts: &[(f64, f64, f64, f64)],
    ) {
        draw_group_dividers(cr, config, group_layouts);
    }
}

/// Draw the monitor bezel (outer frame)
fn draw_bezel(
    cr: &Context,
    config: &RetroTerminalFrameConfig,
    width: f64,
    height: f64,
) -> (f64, f64, f64, f64) {
    if matches!(config.bezel_style, BezelStyle::None) {
        return (0.0, 0.0, width, height);
    }

    let bezel_w = config.bezel_width;

    cr.save().ok();

    match config.bezel_style {
        BezelStyle::Classic => {
            // Outer bezel rectangle with rounded corners
            cr.set_source_rgba(
                config.bezel_color.r,
                config.bezel_color.g,
                config.bezel_color.b,
                config.bezel_color.a,
            );

            // Draw bezel with subtle 3D effect
            let corner_radius = 8.0;

            // Main bezel fill
            draw_rounded_rect(cr, 0.0, 0.0, width, height, corner_radius);
            cr.fill().ok();

            // Lighter top/left edge (highlight)
            cr.set_source_rgba(
                (config.bezel_color.r + 0.1).min(1.0),
                (config.bezel_color.g + 0.1).min(1.0),
                (config.bezel_color.b + 0.08).min(1.0),
                0.6,
            );
            cr.set_line_width(2.0);
            cr.move_to(corner_radius, 0.0);
            cr.line_to(width - corner_radius, 0.0);
            cr.stroke().ok();
            cr.move_to(0.0, corner_radius);
            cr.line_to(0.0, height - corner_radius);
            cr.stroke().ok();

            // Darker bottom/right edge (shadow)
            cr.set_source_rgba(0.0, 0.0, 0.0, 0.4);
            cr.set_line_width(2.0);
            cr.move_to(corner_radius, height);
            cr.line_to(width - corner_radius, height);
            cr.stroke().ok();
            cr.move_to(width, corner_radius);
            cr.line_to(width, height - corner_radius);
            cr.stroke().ok();

            // Inner bezel edge (dark inset)
            let inner_x = bezel_w - 4.0;
            let inner_y = bezel_w - 4.0;
            let inner_w = width - 2.0 * inner_x;
            let inner_h = height - 2.0 * inner_y;

            cr.set_source_rgba(0.0, 0.0, 0.0, 0.6);
            cr.set_line_width(3.0);
            draw_rounded_rect(cr, inner_x, inner_y, inner_w, inner_h, 4.0);
            cr.stroke().ok();

            // Power LED
            if config.show_power_led {
                let led_x = bezel_w / 2.0;
                let led_y = height - bezel_w / 2.0;
                let led_radius = 3.0;

                // LED glow
                cr.set_source_rgba(
                    config.power_led_color.r,
                    config.power_led_color.g,
                    config.power_led_color.b,
                    0.3,
                );
                cr.arc(led_x, led_y, led_radius * 2.0, 0.0, std::f64::consts::TAU);
                cr.fill().ok();

                // LED body
                cr.set_source_rgba(
                    config.power_led_color.r,
                    config.power_led_color.g,
                    config.power_led_color.b,
                    config.power_led_color.a,
                );
                cr.arc(led_x, led_y, led_radius, 0.0, std::f64::consts::TAU);
                cr.fill().ok();
            }
        }
        BezelStyle::Slim => {
            // Thin bezel
            cr.set_source_rgba(
                config.bezel_color.r,
                config.bezel_color.g,
                config.bezel_color.b,
                config.bezel_color.a,
            );
            draw_rounded_rect(cr, 0.0, 0.0, width, height, 4.0);
            cr.fill().ok();

            // Inner edge
            cr.set_source_rgba(0.0, 0.0, 0.0, 0.5);
            cr.set_line_width(1.0);
            draw_rounded_rect(
                cr,
                bezel_w - 1.0,
                bezel_w - 1.0,
                width - 2.0 * bezel_w + 2.0,
                height - 2.0 * bezel_w + 2.0,
                2.0,
            );
            cr.stroke().ok();
        }
        BezelStyle::Industrial => {
            // Heavy industrial bezel with ventilation
            cr.set_source_rgba(
                config.bezel_color.r * 0.8,
                config.bezel_color.g * 0.8,
                config.bezel_color.b * 0.8,
                config.bezel_color.a,
            );
            cr.rectangle(0.0, 0.0, width, height);
            cr.fill().ok();

            // Ventilation slots on sides
            cr.set_source_rgba(0.0, 0.0, 0.0, 0.7);
            let slot_height = 4.0;
            let slot_gap = 6.0;
            let mut y_pos = bezel_w + 10.0;
            while y_pos < height - bezel_w - 10.0 {
                // Left side slots
                cr.rectangle(4.0, y_pos, bezel_w - 8.0, slot_height);
                // Right side slots
                cr.rectangle(width - bezel_w + 4.0, y_pos, bezel_w - 8.0, slot_height);
                y_pos += slot_height + slot_gap;
            }
            cr.fill().ok();

            // Inner edge
            cr.set_source_rgba(0.0, 0.0, 0.0, 0.6);
            cr.set_line_width(3.0);
            cr.rectangle(
                bezel_w - 2.0,
                bezel_w - 2.0,
                width - 2.0 * bezel_w + 4.0,
                height - 2.0 * bezel_w + 4.0,
            );
            cr.stroke().ok();

            // Power LED
            if config.show_power_led {
                let led_x = width / 2.0;
                let led_y = height - bezel_w / 2.0;

                cr.set_source_rgba(
                    config.power_led_color.r,
                    config.power_led_color.g,
                    config.power_led_color.b,
                    config.power_led_color.a,
                );
                cr.rectangle(led_x - 8.0, led_y - 3.0, 16.0, 6.0);
                cr.fill().ok();
            }
        }
        BezelStyle::None => {}
    }

    cr.restore().ok();

    // Return screen area (inside bezel)
    (
        bezel_w,
        bezel_w,
        width - 2.0 * bezel_w,
        height - 2.0 * bezel_w,
    )
}

/// Draw a rounded rectangle path
fn draw_rounded_rect(cr: &Context, x: f64, y: f64, w: f64, h: f64, r: f64) {
    let r = r.min(w / 2.0).min(h / 2.0);
    cr.move_to(x + r, y);
    cr.line_to(x + w - r, y);
    cr.arc(x + w - r, y + r, r, -std::f64::consts::FRAC_PI_2, 0.0);
    cr.line_to(x + w, y + h - r);
    cr.arc(x + w - r, y + h - r, r, 0.0, std::f64::consts::FRAC_PI_2);
    cr.line_to(x + r, y + h);
    cr.arc(
        x + r,
        y + h - r,
        r,
        std::f64::consts::FRAC_PI_2,
        std::f64::consts::PI,
    );
    cr.line_to(x, y + r);
    cr.arc(
        x + r,
        y + r,
        r,
        std::f64::consts::PI,
        3.0 * std::f64::consts::FRAC_PI_2,
    );
    cr.close_path();
}

/// Draw the screen background with phosphor effect
fn draw_screen_background(
    cr: &Context,
    config: &RetroTerminalFrameConfig,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) {
    cr.save().ok();

    // Dark background
    cr.set_source_rgba(
        config.background_color.r,
        config.background_color.g,
        config.background_color.b,
        config.background_color.a,
    );
    cr.rectangle(x, y, w, h);
    cr.fill().ok();

    // Subtle phosphor glow from text area (simulates screen persistence)
    if config.screen_glow > 0.0 {
        let phosphor = config.phosphor_color.to_color();
        let glow_alpha = config.screen_glow * 0.05;

        // Radial gradient for screen glow
        let cx = x + w / 2.0;
        let cy = y + h / 2.0;
        let radius = (w.max(h)) * 0.8;

        let gradient = cairo::RadialGradient::new(cx, cy, 0.0, cx, cy, radius);
        gradient.add_color_stop_rgba(0.0, phosphor.r, phosphor.g, phosphor.b, glow_alpha);
        gradient.add_color_stop_rgba(0.5, phosphor.r, phosphor.g, phosphor.b, glow_alpha * 0.5);
        gradient.add_color_stop_rgba(1.0, phosphor.r, phosphor.g, phosphor.b, 0.0);

        cr.set_source(&gradient).ok();
        cr.rectangle(x, y, w, h);
        cr.fill().ok();
    }

    cr.restore().ok();
}

/// Draw CRT scanlines effect
fn draw_scanlines(cr: &Context, config: &RetroTerminalFrameConfig, x: f64, y: f64, w: f64, h: f64) {
    if config.scanline_intensity <= 0.0 {
        return;
    }

    cr.save().ok();

    // Clip to screen area
    cr.rectangle(x, y, w, h);
    cr.clip();

    cr.set_source_rgba(0.0, 0.0, 0.0, config.scanline_intensity * 0.5);

    let spacing = config.scanline_spacing.max(1.0);
    let mut y_pos = y;
    while y_pos < y + h {
        cr.rectangle(x, y_pos, w, 1.0);
        y_pos += spacing;
    }
    cr.fill().ok();

    cr.restore().ok();
}

/// Draw CRT curvature and vignette effect
fn draw_curvature_vignette(
    cr: &Context,
    config: &RetroTerminalFrameConfig,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) {
    if config.curvature_amount <= 0.0 && config.vignette_intensity <= 0.0 {
        return;
    }

    cr.save().ok();

    // Clip to screen area
    cr.rectangle(x, y, w, h);
    cr.clip();

    // Combined curvature + vignette effect using radial gradient
    let cx = x + w / 2.0;
    let cy = y + h / 2.0;
    let radius = (w.max(h)) * 0.75;

    let gradient = cairo::RadialGradient::new(cx, cy, 0.0, cx, cy, radius);
    gradient.add_color_stop_rgba(0.0, 0.0, 0.0, 0.0, 0.0);
    gradient.add_color_stop_rgba(0.6, 0.0, 0.0, 0.0, config.vignette_intensity * 0.2);
    gradient.add_color_stop_rgba(0.85, 0.0, 0.0, 0.0, config.vignette_intensity * 0.5);
    gradient.add_color_stop_rgba(
        1.0,
        0.0,
        0.0,
        0.0,
        config.vignette_intensity * 0.9 + config.curvature_amount * 2.0,
    );

    cr.set_source(&gradient).ok();
    cr.paint().ok();

    cr.restore().ok();
}

/// Draw the terminal header
fn draw_header(cr: &Context, config: &RetroTerminalFrameConfig, x: f64, y: f64, w: f64) -> f64 {
    if !config.show_header || matches!(config.header_style, TerminalHeaderStyle::None) {
        return 0.0;
    }

    let header_h = config.header_height;
    let phosphor = config.phosphor_color.to_color();
    let dim_phosphor = config.phosphor_color.to_dim_color();

    cr.save().ok();

    // Resolve the theme-aware font
    let (font_family, font_size) = config.header_font.resolve(&config.theme);

    let text = if config.header_text.is_empty() {
        "TERMINAL"
    } else {
        &config.header_text
    };

    let text_extents = pango_text_extents(
        cr,
        text,
        &font_family,
        cairo::FontSlant::Normal,
        cairo::FontWeight::Bold,
        font_size,
    );
    let (text_width, text_height) = (text_extents.width(), text_extents.height());

    match config.header_style {
        TerminalHeaderStyle::TitleBar => {
            // Draw title bar background
            cr.set_source_rgba(dim_phosphor.r, dim_phosphor.g, dim_phosphor.b, 0.15);
            cr.rectangle(x, y, w, header_h);
            cr.fill().ok();

            // Bottom border
            cr.set_source_rgba(phosphor.r, phosphor.g, phosphor.b, 0.5);
            cr.set_line_width(1.0);
            cr.move_to(x, y + header_h);
            cr.line_to(x + w, y + header_h);
            cr.stroke().ok();

            // Centered title
            let text_x = x + (w - text_width) / 2.0;
            let text_y = y + header_h / 2.0 + text_height / 3.0;

            // Text glow
            if config.screen_glow > 0.0 {
                cr.set_source_rgba(phosphor.r, phosphor.g, phosphor.b, config.screen_glow * 0.3);
                cr.move_to(text_x, text_y);
                pango_show_text(
                    cr,
                    text,
                    &font_family,
                    cairo::FontSlant::Normal,
                    cairo::FontWeight::Bold,
                    font_size,
                );
            }

            // Main text
            cr.set_source_rgba(
                phosphor.r * config.text_brightness,
                phosphor.g * config.text_brightness,
                phosphor.b * config.text_brightness,
                1.0,
            );
            cr.move_to(text_x, text_y);
            pango_show_text(
                cr,
                text,
                &font_family,
                cairo::FontSlant::Normal,
                cairo::FontWeight::Bold,
                font_size,
            );
        }
        TerminalHeaderStyle::StatusLine => {
            // VT100-style reverse video status line
            cr.set_source_rgba(phosphor.r * 0.8, phosphor.g * 0.8, phosphor.b * 0.8, 0.9);
            cr.rectangle(x, y, w, header_h);
            cr.fill().ok();

            // Dark text on bright background (reverse video)
            let text_x = x + 8.0;
            let text_y = y + header_h / 2.0 + text_height / 3.0;

            cr.set_source_rgba(0.0, 0.0, 0.0, 1.0);
            cr.move_to(text_x, text_y);
            pango_show_text(
                cr,
                text,
                &font_family,
                cairo::FontSlant::Normal,
                cairo::FontWeight::Bold,
                font_size,
            );

            // Right-aligned info
            let info = "STATUS: OK";
            let info_extents = pango_text_extents(
                cr,
                info,
                &font_family,
                cairo::FontSlant::Normal,
                cairo::FontWeight::Bold,
                font_size,
            );
            let info_width = info_extents.width();
            cr.move_to(x + w - info_width - 8.0, text_y);
            pango_show_text(
                cr,
                info,
                &font_family,
                cairo::FontSlant::Normal,
                cairo::FontWeight::Bold,
                font_size,
            );
        }
        TerminalHeaderStyle::Prompt => {
            // Shell prompt style: $ SYSTEM MONITOR _
            let prompt = format!("$ {} _", text.to_uppercase());

            let text_x = x + 8.0;
            let text_y = y + header_h / 2.0 + text_height / 3.0;

            // Glow
            if config.screen_glow > 0.0 {
                cr.set_source_rgba(phosphor.r, phosphor.g, phosphor.b, config.screen_glow * 0.3);
                cr.move_to(text_x, text_y);
                pango_show_text(
                    cr,
                    &prompt,
                    &font_family,
                    cairo::FontSlant::Normal,
                    cairo::FontWeight::Bold,
                    font_size,
                );
            }

            cr.set_source_rgba(
                phosphor.r * config.text_brightness,
                phosphor.g * config.text_brightness,
                phosphor.b * config.text_brightness,
                1.0,
            );
            cr.move_to(text_x, text_y);
            pango_show_text(
                cr,
                &prompt,
                &font_family,
                cairo::FontSlant::Normal,
                cairo::FontWeight::Bold,
                font_size,
            );
        }
        TerminalHeaderStyle::None => {}
    }

    cr.restore().ok();

    header_h
}

/// Draw a divider between content groups
fn draw_divider(
    cr: &Context,
    config: &RetroTerminalFrameConfig,
    x: f64,
    y: f64,
    length: f64,
    horizontal: bool,
) {
    if matches!(config.divider_style, TerminalDividerStyle::None) {
        return;
    }

    let phosphor = config.phosphor_color.to_dim_color();

    cr.save().ok();

    cr.set_source_rgba(phosphor.r, phosphor.g, phosphor.b, phosphor.a);
    cr.set_line_width(1.0);

    match config.divider_style {
        TerminalDividerStyle::Dashed => {
            cr.set_dash(&[6.0, 4.0], 0.0);
            if horizontal {
                cr.move_to(x, y);
                cr.line_to(x + length, y);
            } else {
                cr.move_to(x, y);
                cr.line_to(x, y + length);
            }
            cr.stroke().ok();
            cr.set_dash(&[], 0.0);
        }
        TerminalDividerStyle::Solid => {
            cr.set_line_width(2.0);
            if horizontal {
                cr.move_to(x, y);
                cr.line_to(x + length, y);
            } else {
                cr.move_to(x, y);
                cr.line_to(x, y + length);
            }
            cr.stroke().ok();
        }
        TerminalDividerStyle::BoxDrawing => {
            // ├──────────┤ style
            let cap_size = 6.0;
            if horizontal {
                // Left cap
                cr.move_to(x, y - cap_size);
                cr.line_to(x, y + cap_size);
                // Line
                cr.move_to(x, y);
                cr.line_to(x + length, y);
                // Right cap
                cr.move_to(x + length, y - cap_size);
                cr.line_to(x + length, y + cap_size);
            } else {
                // Top cap
                cr.move_to(x - cap_size, y);
                cr.line_to(x + cap_size, y);
                // Line
                cr.move_to(x, y);
                cr.line_to(x, y + length);
                // Bottom cap
                cr.move_to(x - cap_size, y + length);
                cr.line_to(x + cap_size, y + length);
            }
            cr.stroke().ok();
        }
        TerminalDividerStyle::Pipe => {
            // ||||||||| style
            let pipe_spacing = 4.0;
            if horizontal {
                let mut px = x;
                while px < x + length {
                    cr.move_to(px, y - 4.0);
                    cr.line_to(px, y + 4.0);
                    px += pipe_spacing;
                }
            } else {
                let mut py = y;
                while py < y + length {
                    cr.move_to(x - 4.0, py);
                    cr.line_to(x + 4.0, py);
                    py += pipe_spacing;
                }
            }
            cr.stroke().ok();
        }
        TerminalDividerStyle::Ascii => {
            // ======== style
            cr.set_line_width(2.0);
            if horizontal {
                cr.move_to(x, y - 1.5);
                cr.line_to(x + length, y - 1.5);
                cr.move_to(x, y + 1.5);
                cr.line_to(x + length, y + 1.5);
            } else {
                cr.move_to(x - 1.5, y);
                cr.line_to(x - 1.5, y + length);
                cr.move_to(x + 1.5, y);
                cr.line_to(x + 1.5, y + length);
            }
            cr.stroke().ok();
        }
        TerminalDividerStyle::None => {}
    }

    cr.restore().ok();
}

/// Get the phosphor color for content rendering
pub fn get_phosphor_color(config: &RetroTerminalFrameConfig) -> Color {
    let base = config.phosphor_color.to_color();
    Color {
        r: base.r * config.text_brightness,
        g: base.g * config.text_brightness,
        b: base.b * config.text_brightness,
        a: base.a,
    }
}

/// Render the complete Retro Terminal frame
/// Returns the content area bounds (x, y, width, height)
pub fn render_retro_terminal_frame(
    cr: &Context,
    config: &RetroTerminalFrameConfig,
    width: f64,
    height: f64,
) -> Result<(f64, f64, f64, f64)> {
    // Guard against invalid dimensions
    if width < 1.0 || height < 1.0 {
        return Ok((0.0, 0.0, 0.0, 0.0));
    }

    cr.save()?;

    // Draw bezel and get screen area
    let (screen_x, screen_y, screen_w, screen_h) = draw_bezel(cr, config, width, height);

    // Draw screen background
    draw_screen_background(cr, config, screen_x, screen_y, screen_w, screen_h);

    // Draw header and get its height
    let header_height = draw_header(cr, config, screen_x, screen_y, screen_w);

    // Draw scanlines (on top of content area but under content items)
    draw_scanlines(cr, config, screen_x, screen_y, screen_w, screen_h);

    // Draw curvature/vignette effect last (on top of everything)
    draw_curvature_vignette(cr, config, screen_x, screen_y, screen_w, screen_h);

    cr.restore()?;

    // Calculate content area
    let content_x = screen_x + config.content_padding;
    let content_y = screen_y + header_height + config.content_padding;
    let content_w = screen_w - config.content_padding * 2.0;
    let content_h = screen_h - header_height - config.content_padding * 2.0;

    Ok((content_x, content_y, content_w.max(0.0), content_h.max(0.0)))
}

/// Calculate group layouts within content area
pub fn calculate_group_layouts(
    config: &RetroTerminalFrameConfig,
    content_x: f64,
    content_y: f64,
    content_w: f64,
    content_h: f64,
) -> Vec<(f64, f64, f64, f64)> {
    let group_count = config.group_count.max(1);
    let mut layouts = Vec::with_capacity(group_count);

    // Get weights (default to equal weights)
    let weights: Vec<f64> = if config.group_size_weights.len() >= group_count {
        config.group_size_weights[..group_count].to_vec()
    } else {
        vec![1.0; group_count]
    };

    let total_weight: f64 = weights.iter().sum();
    let divider_count = group_count.saturating_sub(1);
    let divider_space = divider_count as f64 * (config.divider_padding * 2.0 + 2.0);

    match config.split_orientation {
        SplitOrientation::Vertical => {
            let available_height = content_h - divider_space;
            let mut current_y = content_y;

            for (i, weight) in weights.iter().enumerate() {
                let group_h = available_height * (weight / total_weight);
                layouts.push((content_x, current_y, content_w, group_h));
                current_y += group_h;

                if i < divider_count {
                    current_y += config.divider_padding * 2.0 + 2.0;
                }
            }
        }
        SplitOrientation::Horizontal => {
            let available_width = content_w - divider_space;
            let mut current_x = content_x;

            for (i, weight) in weights.iter().enumerate() {
                let group_w = available_width * (weight / total_weight);
                layouts.push((current_x, content_y, group_w, content_h));
                current_x += group_w;

                if i < divider_count {
                    current_x += config.divider_padding * 2.0 + 2.0;
                }
            }
        }
    }

    layouts
}

/// Draw dividers between groups
pub fn draw_group_dividers(
    cr: &Context,
    config: &RetroTerminalFrameConfig,
    group_layouts: &[(f64, f64, f64, f64)],
) {
    if group_layouts.len() < 2 {
        return;
    }

    for &(x1, y1, w1, h1) in group_layouts.iter().take(group_layouts.len() - 1) {
        match config.split_orientation {
            SplitOrientation::Vertical => {
                let divider_y = y1 + h1 + config.divider_padding;
                draw_divider(cr, config, x1, divider_y, w1, true);
            }
            SplitOrientation::Horizontal => {
                let divider_x = x1 + w1 + config.divider_padding;
                draw_divider(cr, config, divider_x, y1, h1, false);
            }
        }
    }
}
