//! Cyberpunk/Neon HUD display rendering
//!
//! Provides a futuristic heads-up display with:
//! - Angular chamfered corners
//! - Neon glowing borders with bloom effect
//! - Dark translucent backgrounds with grid patterns
//! - Scanline overlay for CRT/hologram effect

use anyhow::Result;
use cairo::Context;

use crate::combo_traits::FrameRenderer;
use crate::lcars_display::SplitOrientation;
use crate::pango_text::{pango_show_text, pango_text_extents};

// Re-export types from rg_sens_types
pub use rg_sens_types::display_configs::cyberpunk::*;

// Re-export types we use
pub use crate::lcars_display::{
    ContentDisplayType as CyberpunkContentType, ContentItemConfig as CyberpunkContentItemConfig,
};

pub struct CyberpunkRenderer;

impl FrameRenderer for CyberpunkRenderer {
    type Config = CyberpunkFrameConfig;

    fn theme_id(&self) -> &'static str {
        "cyberpunk"
    }

    fn theme_name(&self) -> &'static str {
        "Cyberpunk HUD"
    }

    fn default_config(&self) -> Self::Config {
        CyberpunkFrameConfig::default()
    }

    fn render_frame(
        &self,
        cr: &Context,
        config: &Self::Config,
        width: f64,
        height: f64,
    ) -> anyhow::Result<(f64, f64, f64, f64)> {
        render_cyberpunk_frame(cr, config, width, height).map_err(|e| anyhow::anyhow!("{}", e))
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

/// Draw a chamfered rectangle path (45° corners)
fn draw_chamfered_rect(cr: &Context, x: f64, y: f64, w: f64, h: f64, chamfer: f64) {
    let c = chamfer.min(w / 2.0).min(h / 2.0);
    cr.move_to(x + c, y);
    cr.line_to(x + w - c, y);
    cr.line_to(x + w, y + c);
    cr.line_to(x + w, y + h - c);
    cr.line_to(x + w - c, y + h);
    cr.line_to(x + c, y + h);
    cr.line_to(x, y + h - c);
    cr.line_to(x, y + c);
    cr.close_path();
}

/// Draw bracket-style corner decorations
fn draw_bracket_corners(cr: &Context, x: f64, y: f64, w: f64, h: f64, size: f64, line_width: f64) {
    cr.set_line_width(line_width);
    let s = size;

    // Top-left bracket
    cr.move_to(x, y + s);
    cr.line_to(x, y);
    cr.line_to(x + s, y);
    cr.stroke().ok();

    // Top-right bracket
    cr.move_to(x + w - s, y);
    cr.line_to(x + w, y);
    cr.line_to(x + w, y + s);
    cr.stroke().ok();

    // Bottom-right bracket
    cr.move_to(x + w, y + h - s);
    cr.line_to(x + w, y + h);
    cr.line_to(x + w - s, y + h);
    cr.stroke().ok();

    // Bottom-left bracket
    cr.move_to(x + s, y + h);
    cr.line_to(x, y + h);
    cr.line_to(x, y + h - s);
    cr.stroke().ok();
}

/// Draw angular (pointed) corners
fn draw_angular_rect(cr: &Context, x: f64, y: f64, w: f64, h: f64, point_size: f64) {
    let p = point_size.min(w / 4.0).min(h / 4.0);

    // Start from top-left, going clockwise
    cr.move_to(x - p, y + h / 2.0); // Left point
    cr.line_to(x, y); // To top-left
    cr.line_to(x + w / 2.0, y - p); // Top point
    cr.line_to(x + w, y); // To top-right
    cr.line_to(x + w + p, y + h / 2.0); // Right point
    cr.line_to(x + w, y + h); // To bottom-right
    cr.line_to(x + w / 2.0, y + h + p); // Bottom point
    cr.line_to(x, y + h); // To bottom-left
    cr.close_path();
}

/// Draw the glow effect for a path
fn draw_glow(cr: &Context, config: &CyberpunkFrameConfig, x: f64, y: f64, w: f64, h: f64) {
    if config.glow_intensity <= 0.0 {
        return;
    }

    // Resolve theme-aware border color for glow
    let border_color = config.border_color.resolve(&config.theme);

    let glow_steps = 4;
    for i in (1..=glow_steps).rev() {
        let alpha = config.glow_intensity * (i as f64 / glow_steps as f64) * 0.25;
        let extra_width = i as f64 * 2.0;

        cr.set_source_rgba(border_color.r, border_color.g, border_color.b, alpha);
        cr.set_line_width(config.border_width + extra_width);

        match config.corner_style {
            CornerStyle::Chamfer => {
                draw_chamfered_rect(cr, x, y, w, h, config.corner_size);
                cr.stroke().ok();
            }
            CornerStyle::Bracket => {
                // For bracket style, draw glow on the main rectangle
                cr.rectangle(x, y, w, h);
                cr.stroke().ok();
            }
            CornerStyle::Angular => {
                draw_angular_rect(cr, x, y, w, h, config.corner_size);
                cr.stroke().ok();
            }
        }
    }
}

/// Draw the grid pattern background
fn draw_grid(cr: &Context, config: &CyberpunkFrameConfig, x: f64, y: f64, w: f64, h: f64) {
    if !config.show_grid || config.grid_spacing <= 0.0 {
        return;
    }

    cr.save().ok();

    // Clip to frame area
    match config.corner_style {
        CornerStyle::Chamfer => {
            draw_chamfered_rect(cr, x, y, w, h, config.corner_size);
            cr.clip();
        }
        CornerStyle::Angular => {
            draw_angular_rect(cr, x, y, w, h, config.corner_size);
            cr.clip();
        }
        CornerStyle::Bracket => {
            cr.rectangle(x, y, w, h);
            cr.clip();
        }
    }

    // Resolve theme-aware grid color with low opacity
    let grid_color = config.grid_color.resolve(&config.theme);
    cr.set_source_rgba(
        grid_color.r,
        grid_color.g,
        grid_color.b,
        grid_color.a * 0.2, // Apply low opacity for grid
    );
    cr.set_line_width(0.5);

    let spacing = config.grid_spacing;

    // Vertical lines
    let mut gx = x + spacing;
    while gx < x + w {
        cr.move_to(gx, y);
        cr.line_to(gx, y + h);
        gx += spacing;
    }

    // Horizontal lines
    let mut gy = y + spacing;
    while gy < y + h {
        cr.move_to(x, gy);
        cr.line_to(x + w, gy);
        gy += spacing;
    }

    cr.stroke().ok();
    cr.restore().ok();
}

/// Draw scanline overlay effect
fn draw_scanlines(cr: &Context, config: &CyberpunkFrameConfig, x: f64, y: f64, w: f64, h: f64) {
    if !config.show_scanlines || config.scanline_opacity <= 0.0 {
        return;
    }

    cr.save().ok();

    // Clip to frame area
    match config.corner_style {
        CornerStyle::Chamfer => {
            draw_chamfered_rect(cr, x, y, w, h, config.corner_size);
            cr.clip();
        }
        CornerStyle::Angular => {
            draw_angular_rect(cr, x, y, w, h, config.corner_size);
            cr.clip();
        }
        CornerStyle::Bracket => {
            cr.rectangle(x, y, w, h);
            cr.clip();
        }
    }

    cr.set_source_rgba(0.0, 0.0, 0.0, config.scanline_opacity);

    // Draw horizontal scanlines every 2 pixels
    let mut sy = y;
    while sy < y + h {
        cr.rectangle(x, sy, w, 1.0);
        sy += 2.0;
    }
    cr.fill().ok();

    cr.restore().ok();
}

/// Draw the header with configured style
fn draw_header(cr: &Context, config: &CyberpunkFrameConfig, x: f64, y: f64, w: f64) -> f64 {
    if !config.show_header || config.header_text.is_empty() {
        return 0.0;
    }

    // Resolve theme-aware font and colors
    let (font_family, font_size) = config.header_font.resolve(&config.theme);
    let header_color = config.header_color.resolve(&config.theme);
    let border_color = config.border_color.resolve(&config.theme);

    let header_height = font_size + 16.0;
    let padding = 10.0;

    cr.save().ok();

    let text_extents = pango_text_extents(
        cr,
        &config.header_text,
        &font_family,
        cairo::FontSlant::Normal,
        cairo::FontWeight::Bold,
        font_size,
    );
    let text_width = text_extents.width();
    let text_height = text_extents.height();
    let text_x = x + (w - text_width) / 2.0;
    let text_y = y + header_height / 2.0 + text_height / 2.0;

    match config.header_style {
        HeaderStyle::Brackets => {
            // Draw bracket decorations
            let bracket_y = y + header_height / 2.0;

            cr.set_source_rgba(
                border_color.r,
                border_color.g,
                border_color.b,
                border_color.a,
            );
            cr.set_line_width(1.5);

            // Left bracket and line
            let left_x = x + padding;
            cr.move_to(left_x, bracket_y - 8.0);
            cr.line_to(left_x, bracket_y + 8.0);
            cr.move_to(left_x, bracket_y);
            cr.line_to(text_x - 10.0, bracket_y);
            cr.stroke().ok();

            // Right bracket and line
            let right_x = x + w - padding;
            cr.move_to(right_x, bracket_y - 8.0);
            cr.line_to(right_x, bracket_y + 8.0);
            cr.move_to(text_x + text_width + 10.0, bracket_y);
            cr.line_to(right_x, bracket_y);
            cr.stroke().ok();
        }
        HeaderStyle::Underline => {
            // Draw underline
            cr.set_source_rgba(border_color.r, border_color.g, border_color.b, 0.6);
            cr.set_line_width(1.0);
            cr.move_to(x + padding, y + header_height - 4.0);
            cr.line_to(x + w - padding, y + header_height - 4.0);
            cr.stroke().ok();
        }
        HeaderStyle::Box => {
            // Draw box around header
            let box_x = text_x - 10.0;
            let box_y = y + 4.0;
            let box_w = text_width + 20.0;
            let box_h = header_height - 8.0;

            cr.set_source_rgba(border_color.r, border_color.g, border_color.b, 0.3);
            draw_chamfered_rect(cr, box_x, box_y, box_w, box_h, 4.0);
            cr.fill().ok();

            cr.set_source_rgba(
                border_color.r,
                border_color.g,
                border_color.b,
                border_color.a,
            );
            cr.set_line_width(1.0);
            draw_chamfered_rect(cr, box_x, box_y, box_w, box_h, 4.0);
            cr.stroke().ok();
        }
        HeaderStyle::None => {}
    }

    // Draw header text
    cr.set_source_rgba(
        header_color.r,
        header_color.g,
        header_color.b,
        header_color.a,
    );
    cr.move_to(text_x, text_y);
    pango_show_text(
        cr,
        &config.header_text,
        &font_family,
        cairo::FontSlant::Normal,
        cairo::FontWeight::Bold,
        font_size,
    );

    cr.restore().ok();

    header_height
}

/// Draw a divider between content groups
fn draw_divider(
    cr: &Context,
    config: &CyberpunkFrameConfig,
    x: f64,
    y: f64,
    length: f64,
    horizontal: bool,
) {
    if matches!(config.divider_style, DividerStyle::None) {
        return;
    }

    cr.save().ok();

    // Resolve theme-aware divider color
    let divider_color = config.divider_color.resolve(&config.theme);

    cr.set_source_rgba(
        divider_color.r,
        divider_color.g,
        divider_color.b,
        divider_color.a,
    );
    cr.set_line_width(config.divider_width);

    match config.divider_style {
        DividerStyle::Line => {
            if horizontal {
                cr.move_to(x, y);
                cr.line_to(x + length, y);
            } else {
                cr.move_to(x, y);
                cr.line_to(x, y + length);
            }
            cr.stroke().ok();
        }
        DividerStyle::Dashed => {
            let dash_length = 8.0;
            let gap = 4.0;
            cr.set_dash(&[dash_length, gap], 0.0);
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
        DividerStyle::Glow => {
            // Draw glow layers
            for i in (1..=3).rev() {
                let alpha = divider_color.a * (i as f64 / 3.0) * 0.3;
                cr.set_source_rgba(divider_color.r, divider_color.g, divider_color.b, alpha);
                cr.set_line_width(config.divider_width + i as f64 * 2.0);
                if horizontal {
                    cr.move_to(x, y);
                    cr.line_to(x + length, y);
                } else {
                    cr.move_to(x, y);
                    cr.line_to(x, y + length);
                }
                cr.stroke().ok();
            }
            // Main line
            cr.set_source_rgba(
                divider_color.r,
                divider_color.g,
                divider_color.b,
                divider_color.a,
            );
            cr.set_line_width(config.divider_width);
            if horizontal {
                cr.move_to(x, y);
                cr.line_to(x + length, y);
            } else {
                cr.move_to(x, y);
                cr.line_to(x, y + length);
            }
            cr.stroke().ok();
        }
        DividerStyle::Dots => {
            let dot_spacing = 6.0;
            let dot_radius = 1.5;
            if horizontal {
                let mut dx = x;
                while dx < x + length {
                    cr.arc(dx, y, dot_radius, 0.0, std::f64::consts::TAU);
                    cr.fill().ok();
                    dx += dot_spacing;
                }
            } else {
                let mut dy = y;
                while dy < y + length {
                    cr.arc(x, dy, dot_radius, 0.0, std::f64::consts::TAU);
                    cr.fill().ok();
                    dy += dot_spacing;
                }
            }
        }
        DividerStyle::None => {}
    }

    cr.restore().ok();
}

/// Draw an optional frame around a content item
pub fn draw_item_frame(
    cr: &Context,
    config: &CyberpunkFrameConfig,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) {
    if !config.item_frame_enabled {
        return;
    }

    cr.save().ok();

    // Resolve theme-aware item frame color
    let item_frame_color = config.item_frame_color.resolve(&config.theme);

    // Optional glow
    if config.item_glow_enabled {
        for i in (1..=2).rev() {
            let alpha = item_frame_color.a * (i as f64 / 2.0) * 0.3;
            cr.set_source_rgba(
                item_frame_color.r,
                item_frame_color.g,
                item_frame_color.b,
                alpha,
            );
            cr.set_line_width(1.0 + i as f64);
            draw_chamfered_rect(cr, x, y, w, h, 4.0);
            cr.stroke().ok();
        }
    }

    // Main frame
    cr.set_source_rgba(
        item_frame_color.r,
        item_frame_color.g,
        item_frame_color.b,
        item_frame_color.a,
    );
    cr.set_line_width(1.0);
    draw_chamfered_rect(cr, x, y, w, h, 4.0);
    cr.stroke().ok();

    cr.restore().ok();
}

/// Render the complete Cyberpunk frame
/// Returns the content area bounds (x, y, width, height)
pub fn render_cyberpunk_frame(
    cr: &Context,
    config: &CyberpunkFrameConfig,
    width: f64,
    height: f64,
) -> Result<(f64, f64, f64, f64)> {
    // Guard against invalid dimensions
    if width < 1.0 || height < 1.0 {
        return Ok((0.0, 0.0, 0.0, 0.0));
    }

    cr.save()?;

    // Resolve theme-aware colors
    let background_color = config.background_color.resolve(&config.theme);
    let border_color = config.border_color.resolve(&config.theme);

    let margin = config.border_width + config.glow_intensity * 8.0;
    let frame_x = margin;
    let frame_y = margin;
    let frame_w = (width - margin * 2.0).max(1.0);
    let frame_h = (height - margin * 2.0).max(1.0);

    // Draw glow effect first (behind everything)
    draw_glow(cr, config, frame_x, frame_y, frame_w, frame_h);

    // Draw background fill
    cr.set_source_rgba(
        background_color.r,
        background_color.g,
        background_color.b,
        background_color.a,
    );

    match config.corner_style {
        CornerStyle::Chamfer => {
            draw_chamfered_rect(cr, frame_x, frame_y, frame_w, frame_h, config.corner_size);
            cr.fill()?;
        }
        CornerStyle::Angular => {
            draw_angular_rect(cr, frame_x, frame_y, frame_w, frame_h, config.corner_size);
            cr.fill()?;
        }
        CornerStyle::Bracket => {
            cr.rectangle(frame_x, frame_y, frame_w, frame_h);
            cr.fill()?;
        }
    }

    // Draw grid pattern
    draw_grid(cr, config, frame_x, frame_y, frame_w, frame_h);

    // Draw main border
    cr.set_source_rgba(
        border_color.r,
        border_color.g,
        border_color.b,
        border_color.a,
    );
    cr.set_line_width(config.border_width);

    match config.corner_style {
        CornerStyle::Chamfer => {
            draw_chamfered_rect(cr, frame_x, frame_y, frame_w, frame_h, config.corner_size);
            cr.stroke()?;
        }
        CornerStyle::Angular => {
            draw_angular_rect(cr, frame_x, frame_y, frame_w, frame_h, config.corner_size);
            cr.stroke()?;
        }
        CornerStyle::Bracket => {
            cr.rectangle(frame_x, frame_y, frame_w, frame_h);
            cr.stroke()?;
            // Draw bracket corners on top
            draw_bracket_corners(
                cr,
                frame_x,
                frame_y,
                frame_w,
                frame_h,
                config.corner_size,
                config.border_width,
            );
        }
    }

    // Draw header and get its height
    let header_height = draw_header(cr, config, frame_x, frame_y, frame_w);

    // Draw scanlines last (on top of everything except content)
    draw_scanlines(cr, config, frame_x, frame_y, frame_w, frame_h);

    cr.restore()?;

    // Calculate content area
    let content_x = frame_x + config.content_padding;
    let content_y = frame_y + header_height + config.content_padding;
    let content_w = frame_w - config.content_padding * 2.0;
    let content_h = frame_h - header_height - config.content_padding * 2.0;

    Ok((content_x, content_y, content_w, content_h))
}

/// Calculate group layouts within content area
pub fn calculate_group_layouts(
    config: &CyberpunkFrameConfig,
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
    let divider_space =
        divider_count as f64 * (config.divider_width + config.divider_padding * 2.0);

    match config.split_orientation {
        SplitOrientation::Vertical => {
            let available_height = content_h - divider_space;
            let mut current_y = content_y;

            for (i, weight) in weights.iter().enumerate() {
                let group_h = available_height * (weight / total_weight);
                layouts.push((content_x, current_y, content_w, group_h));
                current_y += group_h;

                if i < divider_count {
                    current_y += config.divider_width + config.divider_padding * 2.0;
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
                    current_x += config.divider_width + config.divider_padding * 2.0;
                }
            }
        }
    }

    layouts
}

/// Draw dividers between groups
pub fn draw_group_dividers(
    cr: &Context,
    config: &CyberpunkFrameConfig,
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
