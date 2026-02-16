//! Fighter Jet HUD display rendering
//!
//! Provides a military fighter jet heads-up display aesthetic with:
//! - Military green/amber monochrome color scheme
//! - Thin line frames with corner brackets [ ]
//! - Targeting reticle aesthetics for gauges
//! - Altitude/heading ladder-style scales
//! - Stencil military font styling

use anyhow::Result;
use cairo::Context;

use crate::displayers::combo_displayer_base::FrameRenderer;
use crate::ui::background::Color;
use crate::ui::lcars_display::SplitOrientation;
use crate::ui::pango_text::{pango_show_text, pango_text_extents};

// Re-export all type definitions from rg_sens_types
pub use rg_sens_types::display_configs::fighter_hud::*;

/// Frame renderer for Fighter HUD theme
pub struct FighterHudRenderer;

impl FrameRenderer for FighterHudRenderer {
    type Config = FighterHudFrameConfig;

    fn theme_id(&self) -> &'static str {
        "fighter_hud"
    }

    fn theme_name(&self) -> &'static str {
        "Fighter HUD"
    }

    fn default_config(&self) -> Self::Config {
        FighterHudFrameConfig::default()
    }

    fn render_frame(
        &self,
        cr: &Context,
        config: &Self::Config,
        width: f64,
        height: f64,
    ) -> anyhow::Result<(f64, f64, f64, f64)> {
        render_fighter_hud_frame(cr, config, width, height).map_err(|e| anyhow::anyhow!("{}", e))
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

/// Draw the HUD frame corners
fn draw_frame_corners(
    cr: &Context,
    config: &FighterHudFrameConfig,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) {
    let color = config.hud_color.to_color();
    let bracket_size = config.bracket_size;
    let thickness = config.bracket_thickness;

    cr.save().ok();
    cr.set_source_rgba(color.r, color.g, color.b, color.a);
    cr.set_line_width(thickness);
    cr.set_line_cap(cairo::LineCap::Square);

    match config.frame_style {
        HudFrameStyle::CornerBrackets => {
            // Top-left bracket [
            cr.move_to(x + bracket_size, y);
            cr.line_to(x, y);
            cr.line_to(x, y + bracket_size);
            cr.stroke().ok();

            // Top-right bracket ]
            cr.move_to(x + w - bracket_size, y);
            cr.line_to(x + w, y);
            cr.line_to(x + w, y + bracket_size);
            cr.stroke().ok();

            // Bottom-left bracket [
            cr.move_to(x, y + h - bracket_size);
            cr.line_to(x, y + h);
            cr.line_to(x + bracket_size, y + h);
            cr.stroke().ok();

            // Bottom-right bracket ]
            cr.move_to(x + w, y + h - bracket_size);
            cr.line_to(x + w, y + h);
            cr.line_to(x + w - bracket_size, y + h);
            cr.stroke().ok();

            // Optional tick marks along edges
            let dim_color = config.hud_color.to_dim_color();
            cr.set_source_rgba(dim_color.r, dim_color.g, dim_color.b, dim_color.a);
            cr.set_line_width(1.0);

            // Top edge ticks
            let tick_len = 4.0;
            let mut tx = x + bracket_size + 10.0;
            while tx < x + w - bracket_size - 10.0 {
                cr.move_to(tx, y);
                cr.line_to(tx, y + tick_len);
                tx += config.tick_spacing * 3.0;
            }
            cr.stroke().ok();

            // Bottom edge ticks
            tx = x + bracket_size + 10.0;
            while tx < x + w - bracket_size - 10.0 {
                cr.move_to(tx, y + h);
                cr.line_to(tx, y + h - tick_len);
                tx += config.tick_spacing * 3.0;
            }
            cr.stroke().ok();
        }
        HudFrameStyle::TargetingReticle => {
            // Targeting corners with crosshair elements
            let corner_gap = 8.0;

            // Top-left
            cr.move_to(x + bracket_size, y);
            cr.line_to(x + corner_gap, y);
            cr.move_to(x, y + corner_gap);
            cr.line_to(x, y + bracket_size);
            cr.stroke().ok();

            // Top-right
            cr.move_to(x + w - bracket_size, y);
            cr.line_to(x + w - corner_gap, y);
            cr.move_to(x + w, y + corner_gap);
            cr.line_to(x + w, y + bracket_size);
            cr.stroke().ok();

            // Bottom-left
            cr.move_to(x, y + h - bracket_size);
            cr.line_to(x, y + h - corner_gap);
            cr.move_to(x + corner_gap, y + h);
            cr.line_to(x + bracket_size, y + h);
            cr.stroke().ok();

            // Bottom-right
            cr.move_to(x + w, y + h - bracket_size);
            cr.line_to(x + w, y + h - corner_gap);
            cr.move_to(x + w - corner_gap, y + h);
            cr.line_to(x + w - bracket_size, y + h);
            cr.stroke().ok();

            // Small targeting pips at corners (stroked outlines)
            let pip_size = 3.0;
            cr.set_line_width(1.5);
            cr.new_sub_path();
            cr.arc(
                x + corner_gap / 2.0,
                y + corner_gap / 2.0,
                pip_size,
                0.0,
                std::f64::consts::TAU,
            );
            cr.new_sub_path();
            cr.arc(
                x + w - corner_gap / 2.0,
                y + corner_gap / 2.0,
                pip_size,
                0.0,
                std::f64::consts::TAU,
            );
            cr.new_sub_path();
            cr.arc(
                x + corner_gap / 2.0,
                y + h - corner_gap / 2.0,
                pip_size,
                0.0,
                std::f64::consts::TAU,
            );
            cr.new_sub_path();
            cr.arc(
                x + w - corner_gap / 2.0,
                y + h - corner_gap / 2.0,
                pip_size,
                0.0,
                std::f64::consts::TAU,
            );
            cr.stroke().ok();
        }
        HudFrameStyle::TacticalBox => {
            // Full box with corner accents
            let dim_color = config.hud_color.to_dim_color();
            cr.set_source_rgba(dim_color.r, dim_color.g, dim_color.b, dim_color.a * 0.5);
            cr.set_line_width(1.0);
            cr.rectangle(x, y, w, h);
            cr.stroke().ok();

            // Bright corner accents
            cr.set_source_rgba(color.r, color.g, color.b, color.a);
            cr.set_line_width(thickness);

            // Top-left L
            cr.move_to(x + bracket_size, y);
            cr.line_to(x, y);
            cr.line_to(x, y + bracket_size);
            cr.stroke().ok();

            // Top-right L
            cr.move_to(x + w - bracket_size, y);
            cr.line_to(x + w, y);
            cr.line_to(x + w, y + bracket_size);
            cr.stroke().ok();

            // Bottom-left L
            cr.move_to(x, y + h - bracket_size);
            cr.line_to(x, y + h);
            cr.line_to(x + bracket_size, y + h);
            cr.stroke().ok();

            // Bottom-right L
            cr.move_to(x + w, y + h - bracket_size);
            cr.line_to(x + w, y + h);
            cr.line_to(x + w - bracket_size, y + h);
            cr.stroke().ok();
        }
        HudFrameStyle::Minimal => {
            // Just small corner marks
            let mark_size = bracket_size * 0.5;

            // Top-left
            cr.move_to(x + mark_size, y);
            cr.line_to(x, y);
            cr.line_to(x, y + mark_size);
            cr.stroke().ok();

            // Top-right
            cr.move_to(x + w - mark_size, y);
            cr.line_to(x + w, y);
            cr.line_to(x + w, y + mark_size);
            cr.stroke().ok();

            // Bottom-left
            cr.move_to(x, y + h - mark_size);
            cr.line_to(x, y + h);
            cr.line_to(x + mark_size, y + h);
            cr.stroke().ok();

            // Bottom-right
            cr.move_to(x + w, y + h - mark_size);
            cr.line_to(x + w, y + h);
            cr.line_to(x + w - mark_size, y + h);
            cr.stroke().ok();
        }
        HudFrameStyle::None => {}
    }

    cr.restore().ok();
}

/// Draw center targeting reticle
fn draw_center_reticle(
    cr: &Context,
    config: &FighterHudFrameConfig,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) {
    if !config.show_center_reticle {
        return;
    }

    // Resolve the theme-aware reticle color
    let color = config.reticle_color.resolve(&config.theme);
    let cx = x + w / 2.0;
    let cy = y + h / 2.0;
    let size = w.min(h) * config.reticle_size;

    cr.save().ok();

    // Clear any existing path to prevent lines from previous drawing operations
    cr.new_path();

    cr.set_source_rgba(color.r, color.g, color.b, color.a * 0.8);
    cr.set_line_width(config.line_width);

    // Center circle
    cr.new_sub_path();
    cr.arc(cx, cy, size * 0.3, 0.0, std::f64::consts::TAU);
    cr.stroke().ok();

    // Crosshair lines (with gap in center)
    let gap = size * 0.4;
    let line_len = size * 0.8;

    // Top
    cr.move_to(cx, cy - gap);
    cr.line_to(cx, cy - line_len);
    // Bottom
    cr.move_to(cx, cy + gap);
    cr.line_to(cx, cy + line_len);
    // Left
    cr.move_to(cx - gap, cy);
    cr.line_to(cx - line_len, cy);
    // Right
    cr.move_to(cx + gap, cy);
    cr.line_to(cx + line_len, cy);
    cr.stroke().ok();

    // Corner brackets around reticle (dimmed version of reticle color)
    let bracket_offset = size * 0.6;
    let bracket_len = size * 0.2;
    let dim_color = Color {
        r: color.r * 0.5,
        g: color.g * 0.5,
        b: color.b * 0.5,
        a: color.a * 0.6,
    };
    cr.set_source_rgba(dim_color.r, dim_color.g, dim_color.b, dim_color.a);
    cr.set_line_width(1.0);

    // Top-left
    cr.move_to(cx - bracket_offset, cy - bracket_offset + bracket_len);
    cr.line_to(cx - bracket_offset, cy - bracket_offset);
    cr.line_to(cx - bracket_offset + bracket_len, cy - bracket_offset);
    // Top-right
    cr.move_to(cx + bracket_offset - bracket_len, cy - bracket_offset);
    cr.line_to(cx + bracket_offset, cy - bracket_offset);
    cr.line_to(cx + bracket_offset, cy - bracket_offset + bracket_len);
    // Bottom-left
    cr.move_to(cx - bracket_offset, cy + bracket_offset - bracket_len);
    cr.line_to(cx - bracket_offset, cy + bracket_offset);
    cr.line_to(cx - bracket_offset + bracket_len, cy + bracket_offset);
    // Bottom-right
    cr.move_to(cx + bracket_offset - bracket_len, cy + bracket_offset);
    cr.line_to(cx + bracket_offset, cy + bracket_offset);
    cr.line_to(cx + bracket_offset, cy + bracket_offset - bracket_len);
    cr.stroke().ok();

    cr.restore().ok();
}

/// Draw the HUD header
fn draw_header(cr: &Context, config: &FighterHudFrameConfig, x: f64, y: f64, w: f64) -> f64 {
    if !config.show_header || matches!(config.header_style, HudHeaderStyle::None) {
        return 0.0;
    }

    let header_h = config.header_height;
    let color = config.hud_color.to_color();
    let dim_color = config.hud_color.to_dim_color();

    cr.save().ok();

    // Resolve header font from FontSource using theme
    let (header_font_family, header_font_size) = config.header_font.resolve(&config.theme);

    let text = if config.header_text.is_empty() {
        "HUD"
    } else {
        &config.header_text
    };

    let text_extents = pango_text_extents(
        cr,
        text,
        &header_font_family,
        cairo::FontSlant::Normal,
        cairo::FontWeight::Bold,
        header_font_size,
    );
    let (_text_width, text_height) = (text_extents.width(), text_extents.height());

    match config.header_style {
        HudHeaderStyle::StatusBar => {
            // Underline bar
            cr.set_source_rgba(dim_color.r, dim_color.g, dim_color.b, dim_color.a * 0.5);
            cr.set_line_width(1.0);
            cr.move_to(x, y + header_h - 2.0);
            cr.line_to(x + w, y + header_h - 2.0);
            cr.stroke().ok();

            // Left-aligned text with status prefix
            let status_text = format!("[ {} ]", text);
            let text_x = x + 4.0;
            let text_y = y + header_h / 2.0 + text_height / 3.0;

            // Glow effect
            if config.glow_intensity > 0.0 {
                cr.set_source_rgba(color.r, color.g, color.b, config.glow_intensity * 0.4);
                cr.move_to(text_x, text_y);
                pango_show_text(
                    cr,
                    &status_text,
                    &header_font_family,
                    cairo::FontSlant::Normal,
                    cairo::FontWeight::Bold,
                    header_font_size,
                );
            }

            cr.set_source_rgba(color.r, color.g, color.b, color.a);
            cr.move_to(text_x, text_y);
            pango_show_text(
                cr,
                &status_text,
                &header_font_family,
                cairo::FontSlant::Normal,
                cairo::FontWeight::Bold,
                header_font_size,
            );

            // Right side status indicators
            let status_right = "ONLINE";
            let right_extents = pango_text_extents(
                cr,
                status_right,
                &header_font_family,
                cairo::FontSlant::Normal,
                cairo::FontWeight::Bold,
                header_font_size,
            );
            let right_width = right_extents.width();

            cr.set_source_rgba(color.r, color.g, color.b, color.a * 0.7);
            cr.move_to(x + w - right_width - 4.0, text_y);
            pango_show_text(
                cr,
                status_right,
                &header_font_family,
                cairo::FontSlant::Normal,
                cairo::FontWeight::Bold,
                header_font_size,
            );
        }
        HudHeaderStyle::MissionCallout => {
            // Centered mission-style callout
            let callout_text = format!("<<< {} >>>", text.to_uppercase());
            let callout_extents = pango_text_extents(
                cr,
                &callout_text,
                &header_font_family,
                cairo::FontSlant::Normal,
                cairo::FontWeight::Bold,
                header_font_size,
            );
            let callout_width = callout_extents.width();

            let text_x = x + (w - callout_width) / 2.0;
            let text_y = y + header_h / 2.0 + text_height / 3.0;

            // Glow
            if config.glow_intensity > 0.0 {
                cr.set_source_rgba(color.r, color.g, color.b, config.glow_intensity * 0.4);
                cr.move_to(text_x, text_y);
                pango_show_text(
                    cr,
                    &callout_text,
                    &header_font_family,
                    cairo::FontSlant::Normal,
                    cairo::FontWeight::Bold,
                    header_font_size,
                );
            }

            cr.set_source_rgba(color.r, color.g, color.b, color.a);
            cr.move_to(text_x, text_y);
            pango_show_text(
                cr,
                &callout_text,
                &header_font_family,
                cairo::FontSlant::Normal,
                cairo::FontWeight::Bold,
                header_font_size,
            );

            // Decorative lines on sides
            cr.set_line_width(1.0);
            let line_y = y + header_h / 2.0;
            cr.move_to(x, line_y);
            cr.line_to(text_x - 10.0, line_y);
            cr.move_to(text_x + callout_width + 10.0, line_y);
            cr.line_to(x + w, line_y);
            cr.stroke().ok();
        }
        HudHeaderStyle::SystemId => {
            // Technical system ID style
            let id_text = format!("SYS:{}", text.to_uppercase().replace(' ', "_"));

            let text_x = x + 8.0;
            let text_y = y + header_h / 2.0 + text_height / 3.0;

            // Prefix indicator box
            cr.set_source_rgba(color.r, color.g, color.b, color.a * 0.3);
            cr.rectangle(x, y + 2.0, 4.0, header_h - 4.0);
            cr.fill().ok();

            cr.set_source_rgba(color.r, color.g, color.b, color.a);
            cr.move_to(text_x, text_y);
            pango_show_text(
                cr,
                &id_text,
                &header_font_family,
                cairo::FontSlant::Normal,
                cairo::FontWeight::Bold,
                header_font_size,
            );
        }
        HudHeaderStyle::None => {}
    }

    cr.restore().ok();

    header_h
}

/// Draw a tick-ladder style divider
fn draw_divider(
    cr: &Context,
    config: &FighterHudFrameConfig,
    x: f64,
    y: f64,
    length: f64,
    horizontal: bool,
) {
    if matches!(config.divider_style, HudDividerStyle::None) {
        return;
    }

    let color = config.hud_color.to_dim_color();

    cr.save().ok();
    cr.set_source_rgba(color.r, color.g, color.b, color.a);
    cr.set_line_width(1.0);

    match config.divider_style {
        HudDividerStyle::TickLadder => {
            // Central line
            if horizontal {
                cr.move_to(x, y);
                cr.line_to(x + length, y);
            } else {
                cr.move_to(x, y);
                cr.line_to(x, y + length);
            }
            cr.stroke().ok();

            // Tick marks
            let tick_len = 4.0;
            let spacing = config.tick_spacing;

            if horizontal {
                let mut tx = x + spacing;
                while tx < x + length - spacing {
                    cr.move_to(tx, y - tick_len);
                    cr.line_to(tx, y + tick_len);
                    tx += spacing;
                }
            } else {
                let mut ty = y + spacing;
                while ty < y + length - spacing {
                    cr.move_to(x - tick_len, ty);
                    cr.line_to(x + tick_len, ty);
                    ty += spacing;
                }
            }
            cr.stroke().ok();
        }
        HudDividerStyle::ArrowLine => {
            // Line with arrows at ends
            let arrow_size = 6.0;

            if horizontal {
                // Main line
                cr.move_to(x + arrow_size, y);
                cr.line_to(x + length - arrow_size, y);
                cr.stroke().ok();

                // Left arrow
                cr.move_to(x, y);
                cr.line_to(x + arrow_size, y - arrow_size / 2.0);
                cr.line_to(x + arrow_size, y + arrow_size / 2.0);
                cr.close_path();
                cr.fill().ok();

                // Right arrow
                cr.move_to(x + length, y);
                cr.line_to(x + length - arrow_size, y - arrow_size / 2.0);
                cr.line_to(x + length - arrow_size, y + arrow_size / 2.0);
                cr.close_path();
                cr.fill().ok();
            } else {
                // Vertical version
                cr.move_to(x, y + arrow_size);
                cr.line_to(x, y + length - arrow_size);
                cr.stroke().ok();

                // Top arrow
                cr.move_to(x, y);
                cr.line_to(x - arrow_size / 2.0, y + arrow_size);
                cr.line_to(x + arrow_size / 2.0, y + arrow_size);
                cr.close_path();
                cr.fill().ok();

                // Bottom arrow
                cr.move_to(x, y + length);
                cr.line_to(x - arrow_size / 2.0, y + length - arrow_size);
                cr.line_to(x + arrow_size / 2.0, y + length - arrow_size);
                cr.close_path();
                cr.fill().ok();
            }
        }
        HudDividerStyle::TacticalDash => {
            // Dashed line with longer dashes
            cr.set_dash(&[10.0, 5.0], 0.0);
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
        HudDividerStyle::Fade => {
            // Gradient fade effect (simulated with multiple lines)
            let steps = 5;
            for i in 0..steps {
                let alpha = (steps - i) as f64 / steps as f64 * color.a;
                cr.set_source_rgba(color.r, color.g, color.b, alpha);

                if horizontal {
                    cr.move_to(x, y + i as f64 - (steps as f64 / 2.0));
                    cr.line_to(x + length, y + i as f64 - (steps as f64 / 2.0));
                } else {
                    cr.move_to(x + i as f64 - (steps as f64 / 2.0), y);
                    cr.line_to(x + i as f64 - (steps as f64 / 2.0), y + length);
                }
                cr.stroke().ok();
            }
        }
        HudDividerStyle::None => {}
    }

    cr.restore().ok();
}

/// Get the HUD color for content rendering
pub fn get_hud_color(config: &FighterHudFrameConfig) -> Color {
    config.hud_color.to_color()
}

/// Render the complete Fighter HUD frame
/// Returns the content area bounds (x, y, width, height)
pub fn render_fighter_hud_frame(
    cr: &Context,
    config: &FighterHudFrameConfig,
    width: f64,
    height: f64,
) -> Result<(f64, f64, f64, f64)> {
    // Guard against invalid dimensions
    if width < 1.0 || height < 1.0 {
        return Ok((0.0, 0.0, 0.0, 0.0));
    }

    cr.save()?;

    // Draw background (usually transparent for HUD overlay effect)
    if config.background_color.a > 0.0 {
        cr.set_source_rgba(
            config.background_color.r,
            config.background_color.g,
            config.background_color.b,
            config.background_color.a,
        );
        cr.rectangle(0.0, 0.0, width, height);
        cr.fill()?;
    }

    // Draw frame corners
    let frame_margin = config.bracket_size * 0.2;
    draw_frame_corners(
        cr,
        config,
        frame_margin,
        frame_margin,
        width - frame_margin * 2.0,
        height - frame_margin * 2.0,
    );

    // Draw header
    let content_x = config.content_padding;
    let header_height = draw_header(
        cr,
        config,
        content_x,
        frame_margin,
        width - config.content_padding * 2.0,
    );

    // Draw center reticle (on top of everything else)
    draw_center_reticle(cr, config, 0.0, 0.0, width, height);

    cr.restore()?;

    // Calculate content area
    let content_y = frame_margin + header_height + config.content_padding * 0.5;
    let content_w = width - config.content_padding * 2.0;
    let content_h = height - content_y - config.content_padding;

    Ok((content_x, content_y, content_w.max(0.0), content_h.max(0.0)))
}

/// Calculate group layouts within content area
pub fn calculate_group_layouts(
    config: &FighterHudFrameConfig,
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
    config: &FighterHudFrameConfig,
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
