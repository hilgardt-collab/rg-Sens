//! Material Design Cards display rendering
//!
//! Provides a clean, modern Material Design-inspired interface with:
//! - Clean white/dark cards with subtle drop shadows
//! - Large rounded corners
//! - Generous whitespace and padding
//! - Color-coded category headers
//! - Smooth animations

use anyhow::Result;
use cairo::Context;

use crate::combo_traits::FrameRenderer;
use crate::background::Color;
use crate::lcars_display::SplitOrientation;
use crate::pango_text::{pango_show_text, pango_text_extents};

// Re-export types from rg_sens_types
pub use rg_sens_types::display_configs::material::*;

// Re-export types we use
pub use crate::lcars_display::{
    ContentDisplayType as MaterialContentType, ContentItemConfig as MaterialContentItemConfig,
};

pub struct MaterialRenderer;

impl FrameRenderer for MaterialRenderer {
    type Config = MaterialFrameConfig;

    fn theme_id(&self) -> &'static str {
        "material"
    }

    fn theme_name(&self) -> &'static str {
        "Material Design"
    }

    fn default_config(&self) -> Self::Config {
        MaterialFrameConfig::default()
    }

    fn render_frame(
        &self,
        cr: &Context,
        config: &Self::Config,
        width: f64,
        height: f64,
    ) -> anyhow::Result<(f64, f64, f64, f64)> {
        render_material_frame(cr, config, width, height).map_err(|e| anyhow::anyhow!("{}", e))
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

/// Get the surface color based on current theme variant
fn surface_color(config: &MaterialFrameConfig) -> Color {
    match config.theme_variant {
        ThemeVariant::Light | ThemeVariant::Teal => config.surface_color_light,
        ThemeVariant::Dark | ThemeVariant::Purple => config.surface_color_dark,
    }
}

/// Get the background color based on current theme variant
fn background_color(config: &MaterialFrameConfig) -> Color {
    match config.theme_variant {
        ThemeVariant::Light | ThemeVariant::Teal => config.background_color_light,
        ThemeVariant::Dark | ThemeVariant::Purple => config.background_color_dark,
    }
}

/// Get the text color based on current theme variant
fn text_color(config: &MaterialFrameConfig) -> Color {
    match config.theme_variant {
        ThemeVariant::Light | ThemeVariant::Teal => config.text_color_light,
        ThemeVariant::Dark | ThemeVariant::Purple => config.text_color_dark,
    }
}

/// Get accent color for a specific group (resolved through theme)
fn group_accent(config: &MaterialFrameConfig, group_idx: usize) -> Color {
    config
        .group_accent_colors
        .get(group_idx)
        .copied()
        .unwrap_or_else(|| config.accent_color.resolve(&config.theme))
}

/// Get header text for a specific group
fn group_header(config: &MaterialFrameConfig, group_idx: usize) -> &str {
    config
        .group_headers
        .get(group_idx)
        .map(|s| s.as_str())
        .unwrap_or("")
}

/// Draw a rounded rectangle path
fn draw_rounded_rect(cr: &Context, x: f64, y: f64, w: f64, h: f64, radius: f64) {
    let r = radius.min(w / 2.0).min(h / 2.0);

    cr.new_sub_path();
    cr.arc(x + w - r, y + r, r, -std::f64::consts::FRAC_PI_2, 0.0);
    cr.arc(x + w - r, y + h - r, r, 0.0, std::f64::consts::FRAC_PI_2);
    cr.arc(
        x + r,
        y + h - r,
        r,
        std::f64::consts::FRAC_PI_2,
        std::f64::consts::PI,
    );
    cr.arc(
        x + r,
        y + r,
        r,
        std::f64::consts::PI,
        3.0 * std::f64::consts::FRAC_PI_2,
    );
    cr.close_path();
}

/// Calculate text x position based on alignment
fn calculate_aligned_text_x(
    cr: &Context,
    text: &str,
    area_x: f64,
    area_w: f64,
    padding: f64,
    alignment: HeaderAlignment,
    font_family: &str,
    font_size: f64,
) -> f64 {
    match alignment {
        HeaderAlignment::Left => area_x + padding,
        HeaderAlignment::Center => {
            let extents = pango_text_extents(
                cr,
                text,
                font_family,
                cairo::FontSlant::Normal,
                cairo::FontWeight::Bold,
                font_size,
            );
            area_x + (area_w - extents.width()) / 2.0
        }
        HeaderAlignment::Right => {
            let extents = pango_text_extents(
                cr,
                text,
                font_family,
                cairo::FontSlant::Normal,
                cairo::FontWeight::Bold,
                font_size,
            );
            area_x + area_w - padding - extents.width()
        }
    }
}

/// Draw a header bar that clips to the card's rounded corners
/// This ensures the bar's top corners match the card's corner radius exactly
fn draw_header_bar_clipped(
    cr: &Context,
    card_x: f64,
    card_y: f64,
    card_w: f64,
    card_h: f64,
    bar_h: f64,
    corner_radius: f64,
) {
    cr.save().ok();
    // Clip to the card's rounded rect shape
    draw_rounded_rect(cr, card_x, card_y, card_w, card_h, corner_radius);
    cr.clip();
    // Draw a simple rectangle for the bar - clipping handles the corners
    cr.rectangle(card_x, card_y, card_w, bar_h);
    cr.fill().ok();
    cr.restore().ok();
}

/// Draw a card shadow (simulated drop shadow)
fn draw_shadow(cr: &Context, config: &MaterialFrameConfig, x: f64, y: f64, w: f64, h: f64) {
    let (blur, offset_y, alpha_mult) = match config.elevation {
        CardElevation::Flat => return,
        CardElevation::Low => (config.shadow_blur * 0.5, config.shadow_offset_y * 0.5, 0.5),
        CardElevation::Medium => (config.shadow_blur, config.shadow_offset_y, 1.0),
        CardElevation::High => (config.shadow_blur * 1.5, config.shadow_offset_y * 1.5, 1.5),
    };

    // Draw multiple shadow layers for a soft blur effect
    let layers = 4;
    for i in 0..layers {
        let layer_blur = blur * (i as f64 + 1.0) / layers as f64;
        let alpha = config.shadow_color.a * alpha_mult * (1.0 - i as f64 / layers as f64) * 0.5;

        cr.save().ok();
        cr.set_source_rgba(
            config.shadow_color.r,
            config.shadow_color.g,
            config.shadow_color.b,
            alpha,
        );

        draw_rounded_rect(
            cr,
            x - layer_blur / 2.0,
            y + offset_y - layer_blur / 2.0 + i as f64,
            w + layer_blur,
            h + layer_blur,
            config.corner_radius + layer_blur / 2.0,
        );
        cr.fill().ok();
        cr.restore().ok();
    }
}

/// Draw a card surface (the main card background)
fn draw_card_surface(cr: &Context, config: &MaterialFrameConfig, x: f64, y: f64, w: f64, h: f64) {
    let surface = surface_color(config);

    cr.save().ok();
    cr.set_source_rgba(surface.r, surface.g, surface.b, surface.a);
    draw_rounded_rect(cr, x, y, w, h, config.corner_radius);
    cr.fill().ok();
    cr.restore().ok();
}

/// Draw a group header bar
fn draw_group_header(
    cr: &Context,
    config: &MaterialFrameConfig,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    group_idx: usize,
) -> f64 {
    let header_text = group_header(config, group_idx);
    if header_text.is_empty() && !matches!(config.header_style, HeaderStyle::ColorBar) {
        return 0.0;
    }

    let accent = group_accent(config, group_idx);
    let header_h = config.header_height.min(h / 3.0);

    cr.save().ok();

    match config.header_style {
        HeaderStyle::ColorBar => {
            // Draw a thin colored bar at the top, clipped to card's rounded corners
            let bar_h = 4.0;
            cr.set_source_rgba(accent.r, accent.g, accent.b, accent.a);
            draw_header_bar_clipped(cr, x, y, w, h, bar_h, config.corner_radius);

            cr.restore().ok();

            // Draw text below the bar if present
            if !header_text.is_empty() {
                cr.save().ok();
                let text_color = text_color(config);
                cr.set_source_rgba(text_color.r, text_color.g, text_color.b, 0.87);
                let (font_family, font_size) = config.header_font.resolve(&config.theme);

                let text_y = y + bar_h + 8.0 + font_size;
                cr.move_to(x + config.card_padding, text_y);
                pango_show_text(
                    cr,
                    header_text,
                    &font_family,
                    cairo::FontSlant::Normal,
                    cairo::FontWeight::Bold,
                    font_size,
                );
                cr.restore().ok();

                return bar_h + font_size + 16.0;
            }

            bar_h
        }
        HeaderStyle::Filled => {
            // Draw filled header background, clipped to card's rounded corners
            cr.set_source_rgba(accent.r, accent.g, accent.b, accent.a);
            draw_header_bar_clipped(cr, x, y, w, h, header_h, config.corner_radius);

            // Draw text in white
            cr.set_source_rgba(1.0, 1.0, 1.0, 1.0);
            let (font_family, font_size) = config.header_font.resolve(&config.theme);

            let text_y = y + header_h / 2.0 + font_size / 3.0;
            cr.move_to(x + config.card_padding, text_y);
            pango_show_text(
                cr,
                header_text,
                &font_family,
                cairo::FontSlant::Normal,
                cairo::FontWeight::Bold,
                font_size,
            );

            cr.restore().ok();
            header_h
        }
        HeaderStyle::TextOnly => {
            if header_text.is_empty() {
                cr.restore().ok();
                return 0.0;
            }

            // Draw colored text
            cr.set_source_rgba(accent.r, accent.g, accent.b, accent.a);
            let (font_family, font_size) = config.header_font.resolve(&config.theme);

            let text_y = y + config.card_padding + font_size;
            cr.move_to(x + config.card_padding, text_y);
            pango_show_text(
                cr,
                header_text,
                &font_family,
                cairo::FontSlant::Normal,
                cairo::FontWeight::Bold,
                font_size,
            );

            cr.restore().ok();
            font_size + config.card_padding * 2.0
        }
        HeaderStyle::None => {
            cr.restore().ok();
            0.0
        }
    }
}

/// Draw a divider between groups
fn draw_divider(
    cr: &Context,
    config: &MaterialFrameConfig,
    x: f64,
    y: f64,
    length: f64,
    horizontal: bool,
) {
    // Resolve divider color through theme
    let divider_color = config.divider_color.resolve(&config.theme);

    match config.divider_style {
        DividerStyle::Space => {
            // No visible divider, just spacing (handled by layout)
        }
        DividerStyle::Line => {
            cr.save().ok();
            cr.set_source_rgba(
                divider_color.r,
                divider_color.g,
                divider_color.b,
                divider_color.a,
            );
            cr.set_line_width(1.0);

            if horizontal {
                cr.move_to(x, y);
                cr.line_to(x + length, y);
            } else {
                cr.move_to(x, y);
                cr.line_to(x, y + length);
            }
            cr.stroke().ok();
            cr.restore().ok();
        }
        DividerStyle::Fade => {
            cr.save().ok();

            if horizontal {
                let gradient = cairo::LinearGradient::new(x, y, x + length, y);
                gradient.add_color_stop_rgba(
                    0.0,
                    divider_color.r,
                    divider_color.g,
                    divider_color.b,
                    0.0,
                );
                gradient.add_color_stop_rgba(
                    0.3,
                    divider_color.r,
                    divider_color.g,
                    divider_color.b,
                    divider_color.a,
                );
                gradient.add_color_stop_rgba(
                    0.7,
                    divider_color.r,
                    divider_color.g,
                    divider_color.b,
                    divider_color.a,
                );
                gradient.add_color_stop_rgba(
                    1.0,
                    divider_color.r,
                    divider_color.g,
                    divider_color.b,
                    0.0,
                );

                cr.set_source(&gradient).ok();
                cr.set_line_width(1.0);
                cr.move_to(x, y);
                cr.line_to(x + length, y);
                cr.stroke().ok();
            } else {
                let gradient = cairo::LinearGradient::new(x, y, x, y + length);
                gradient.add_color_stop_rgba(
                    0.0,
                    divider_color.r,
                    divider_color.g,
                    divider_color.b,
                    0.0,
                );
                gradient.add_color_stop_rgba(
                    0.3,
                    divider_color.r,
                    divider_color.g,
                    divider_color.b,
                    divider_color.a,
                );
                gradient.add_color_stop_rgba(
                    0.7,
                    divider_color.r,
                    divider_color.g,
                    divider_color.b,
                    divider_color.a,
                );
                gradient.add_color_stop_rgba(
                    1.0,
                    divider_color.r,
                    divider_color.g,
                    divider_color.b,
                    0.0,
                );

                cr.set_source(&gradient).ok();
                cr.set_line_width(1.0);
                cr.move_to(x, y);
                cr.line_to(x, y + length);
                cr.stroke().ok();
            }

            cr.restore().ok();
        }
    }
}

/// Render the complete Material frame
/// Returns the content area bounds (x, y, width, height)
pub fn render_material_frame(
    cr: &Context,
    config: &MaterialFrameConfig,
    width: f64,
    height: f64,
) -> Result<(f64, f64, f64, f64)> {
    // Guard against invalid dimensions
    if width < 1.0 || height < 1.0 {
        return Ok((0.0, 0.0, 0.0, 0.0));
    }

    cr.save()?;

    // Draw overall background
    let bg = background_color(config);
    cr.set_source_rgba(bg.r, bg.g, bg.b, bg.a);
    cr.paint()?;

    // Calculate frame area with padding
    let padding = config.content_padding;
    let frame_x = padding;
    let frame_y = padding;
    let frame_w = (width - padding * 2.0).max(1.0);
    let frame_h = (height - padding * 2.0).max(1.0);

    // Draw the main card
    draw_shadow(cr, config, frame_x, frame_y, frame_w, frame_h);
    draw_card_surface(cr, config, frame_x, frame_y, frame_w, frame_h);

    // Draw main header if enabled
    let header_height = if config.show_header && !config.header_text.is_empty() {
        draw_main_header(cr, config, frame_x, frame_y, frame_w, frame_h)
    } else {
        0.0
    };

    cr.restore()?;

    // Calculate content area
    let content_x = frame_x + config.card_padding;
    let content_y = frame_y + header_height + config.card_padding;
    let content_w = frame_w - config.card_padding * 2.0;
    let content_h = frame_h - header_height - config.card_padding * 2.0;

    Ok((content_x, content_y, content_w, content_h))
}

/// Draw the main panel header
fn draw_main_header(
    cr: &Context,
    config: &MaterialFrameConfig,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) -> f64 {
    let header_h = config.header_height;
    let accent = config.accent_color.resolve(&config.theme);

    cr.save().ok();

    match config.header_style {
        HeaderStyle::ColorBar => {
            // Thin colored bar at top, clipped to card's rounded corners
            let bar_h = 4.0;
            cr.set_source_rgba(accent.r, accent.g, accent.b, accent.a);
            draw_header_bar_clipped(cr, x, y, w, h, bar_h, config.corner_radius);

            cr.restore().ok();

            // Text below bar
            cr.save().ok();
            let text_color = text_color(config);
            cr.set_source_rgba(text_color.r, text_color.g, text_color.b, 0.87);
            let (font_family, font_size) = config.header_font.resolve(&config.theme);
            let actual_font_size = font_size + 2.0;

            let text_y = y + bar_h + 12.0 + font_size;
            let text_x = calculate_aligned_text_x(
                cr,
                &config.header_text,
                x,
                w,
                config.card_padding,
                config.header_alignment,
                &font_family,
                actual_font_size,
            );
            cr.move_to(text_x, text_y);
            pango_show_text(
                cr,
                &config.header_text,
                &font_family,
                cairo::FontSlant::Normal,
                cairo::FontWeight::Bold,
                actual_font_size,
            );
            cr.restore().ok();

            bar_h + font_size + 24.0
        }
        HeaderStyle::Filled => {
            // Filled header, clipped to card's rounded corners
            cr.set_source_rgba(accent.r, accent.g, accent.b, accent.a);
            draw_header_bar_clipped(cr, x, y, w, h, header_h, config.corner_radius);

            // White text
            cr.set_source_rgba(1.0, 1.0, 1.0, 1.0);
            let (font_family, font_size) = config.header_font.resolve(&config.theme);
            let actual_font_size = font_size + 2.0;

            let text_y = y + header_h / 2.0 + font_size / 3.0;
            let text_x = calculate_aligned_text_x(
                cr,
                &config.header_text,
                x,
                w,
                config.card_padding,
                config.header_alignment,
                &font_family,
                actual_font_size,
            );
            cr.move_to(text_x, text_y);
            pango_show_text(
                cr,
                &config.header_text,
                &font_family,
                cairo::FontSlant::Normal,
                cairo::FontWeight::Bold,
                actual_font_size,
            );

            cr.restore().ok();
            header_h
        }
        HeaderStyle::TextOnly => {
            let text_color = text_color(config);
            cr.set_source_rgba(text_color.r, text_color.g, text_color.b, 0.87);
            let (font_family, font_size) = config.header_font.resolve(&config.theme);
            let actual_font_size = font_size + 2.0;

            let text_y = y + config.card_padding + font_size;
            let text_x = calculate_aligned_text_x(
                cr,
                &config.header_text,
                x,
                w,
                config.card_padding,
                config.header_alignment,
                &font_family,
                actual_font_size,
            );
            cr.move_to(text_x, text_y);
            pango_show_text(
                cr,
                &config.header_text,
                &font_family,
                cairo::FontSlant::Normal,
                cairo::FontWeight::Bold,
                actual_font_size,
            );

            cr.restore().ok();
            font_size + config.card_padding * 2.0
        }
        HeaderStyle::None => {
            cr.restore().ok();
            0.0
        }
    }
}

/// Calculate group layouts within content area
pub fn calculate_group_layouts(
    config: &MaterialFrameConfig,
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
    let divider_space = divider_count as f64 * config.divider_spacing;

    match config.split_orientation {
        SplitOrientation::Vertical => {
            // Groups stacked top to bottom
            let available_height = content_h - divider_space;
            let mut current_y = content_y;

            for (i, weight) in weights.iter().enumerate() {
                let group_h = available_height * (weight / total_weight);
                layouts.push((content_x, current_y, content_w, group_h));
                current_y += group_h;

                if i < divider_count {
                    current_y += config.divider_spacing;
                }
            }
        }
        SplitOrientation::Horizontal => {
            // Groups side by side
            let available_width = content_w - divider_space;
            let mut current_x = content_x;

            for (i, weight) in weights.iter().enumerate() {
                let group_w = available_width * (weight / total_weight);
                layouts.push((current_x, content_y, group_w, content_h));
                current_x += group_w;

                if i < divider_count {
                    current_x += config.divider_spacing;
                }
            }
        }
    }

    layouts
}

/// Draw dividers between groups
pub fn draw_group_dividers(
    cr: &Context,
    config: &MaterialFrameConfig,
    group_layouts: &[(f64, f64, f64, f64)],
) {
    if group_layouts.len() < 2 {
        return;
    }

    for &(x1, y1, w1, h1) in group_layouts.iter().take(group_layouts.len() - 1) {
        match config.split_orientation {
            SplitOrientation::Vertical => {
                let divider_y = y1 + h1 + config.divider_spacing / 2.0;
                draw_divider(cr, config, x1, divider_y, w1, true);
            }
            SplitOrientation::Horizontal => {
                let divider_x = x1 + w1 + config.divider_spacing / 2.0;
                draw_divider(cr, config, divider_x, y1, h1, false);
            }
        }
    }
}

/// Draw a group card with optional header
pub fn draw_group_card(
    cr: &Context,
    config: &MaterialFrameConfig,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    group_idx: usize,
) -> f64 {
    // Only draw inner cards if we have group headers
    let header_text = group_header(config, group_idx);
    if header_text.is_empty() {
        return 0.0;
    }

    draw_group_header(cr, config, x, y, w, h, group_idx)
}
