//! Art Nouveau display rendering
//!
//! Provides an organic, nature-inspired Art Nouveau display with:
//! - Flowing vine and whiplash curve borders
//! - Floral and leaf corner decorations
//! - Wave and tendril dividers
//! - Earthy color schemes (olive, gold, cream)
//! - Organic background patterns

use std::f64::consts::PI;

use anyhow::Result;
use cairo::Context;

use crate::displayers::combo_displayer_base::FrameRenderer;
use crate::ui::background::Color;
use crate::ui::pango_text::{pango_show_text, pango_text_extents};

pub use rg_sens_types::display_configs::art_nouveau::*;

pub struct ArtNouveauRenderer;

impl FrameRenderer for ArtNouveauRenderer {
    type Config = ArtNouveauFrameConfig;

    fn theme_id(&self) -> &'static str {
        "art_nouveau"
    }

    fn theme_name(&self) -> &'static str {
        "Art Nouveau"
    }

    fn default_config(&self) -> Self::Config {
        ArtNouveauFrameConfig::default()
    }

    fn render_frame(
        &self,
        cr: &Context,
        config: &Self::Config,
        width: f64,
        height: f64,
    ) -> anyhow::Result<(f64, f64, f64, f64)> {
        render_art_nouveau_frame(cr, config, width, height).map_err(|e| anyhow::anyhow!("{}", e))
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

/// Draw a flowing flourish/swirl corner decoration
fn draw_flourish_corner(
    cr: &Context,
    cx: f64,
    cy: f64,
    size: f64,
    rotation: f64,
    color: &Color,
    line_width: f64,
) {
    cr.save().ok();
    cr.translate(cx, cy);
    cr.rotate(rotation);
    cr.set_source_rgba(color.r, color.g, color.b, color.a);
    cr.set_line_width(line_width);
    cr.set_line_cap(cairo::LineCap::Round);

    // Main flourish spiral
    cr.move_to(0.0, 0.0);
    cr.curve_to(
        size * 0.3,
        -size * 0.1,
        size * 0.5,
        -size * 0.4,
        size * 0.7,
        -size * 0.3,
    );
    cr.curve_to(
        size * 0.85,
        -size * 0.2,
        size * 0.9,
        0.0,
        size * 0.7,
        size * 0.15,
    );
    cr.curve_to(size * 0.5, size * 0.3, size * 0.2, size * 0.2, 0.0, 0.0);
    cr.stroke().ok();

    // Secondary tendril
    cr.move_to(size * 0.2, -size * 0.05);
    cr.curve_to(
        size * 0.35,
        -size * 0.25,
        size * 0.5,
        -size * 0.35,
        size * 0.45,
        -size * 0.5,
    );
    cr.stroke().ok();

    cr.restore().ok();
}

/// Draw a leaf corner decoration
fn draw_leaf_corner(
    cr: &Context,
    cx: f64,
    cy: f64,
    size: f64,
    rotation: f64,
    color: &Color,
    line_width: f64,
) {
    cr.save().ok();
    cr.translate(cx, cy);
    cr.rotate(rotation);
    cr.set_source_rgba(color.r, color.g, color.b, color.a);
    cr.set_line_width(line_width);

    // Draw a stylized leaf
    cr.move_to(0.0, 0.0);
    cr.curve_to(
        size * 0.3,
        -size * 0.2,
        size * 0.5,
        -size * 0.5,
        size * 0.7,
        -size * 0.4,
    );
    cr.curve_to(size * 0.6, -size * 0.2, size * 0.4, 0.0, 0.0, 0.0);
    cr.close_path();
    cr.fill().ok();

    // Leaf vein
    cr.set_source_rgba(color.r * 0.7, color.g * 0.7, color.b * 0.7, color.a);
    cr.set_line_width(line_width * 0.5);
    cr.move_to(0.0, 0.0);
    cr.curve_to(
        size * 0.2,
        -size * 0.15,
        size * 0.4,
        -size * 0.3,
        size * 0.55,
        -size * 0.35,
    );
    cr.stroke().ok();

    cr.restore().ok();
}

/// Draw a spiral tendril corner decoration
fn draw_spiral_corner(
    cr: &Context,
    cx: f64,
    cy: f64,
    size: f64,
    rotation: f64,
    color: &Color,
    line_width: f64,
) {
    cr.save().ok();
    cr.translate(cx, cy);
    cr.rotate(rotation);
    cr.set_source_rgba(color.r, color.g, color.b, color.a);
    cr.set_line_width(line_width);
    cr.set_line_cap(cairo::LineCap::Round);

    // Draw a spiral tendril
    let turns = 1.5;
    let points = 30;
    cr.move_to(0.0, 0.0);

    for i in 1..=points {
        let t = i as f64 / points as f64;
        let angle = t * turns * 2.0 * PI;
        let radius = size * 0.7 * t;
        let x = radius * angle.cos();
        let y = -radius * angle.sin();
        cr.line_to(x, y);
    }
    cr.stroke().ok();

    cr.restore().ok();
}

/// Draw a simple curved bracket corner
fn draw_bracket_corner(
    cr: &Context,
    cx: f64,
    cy: f64,
    size: f64,
    rotation: f64,
    color: &Color,
    line_width: f64,
) {
    cr.save().ok();
    cr.translate(cx, cy);
    cr.rotate(rotation);
    cr.set_source_rgba(color.r, color.g, color.b, color.a);
    cr.set_line_width(line_width);
    cr.set_line_cap(cairo::LineCap::Round);

    // Curved L-bracket
    cr.move_to(0.0, -size * 0.6);
    cr.curve_to(0.0, -size * 0.2, size * 0.1, 0.0, size * 0.6, 0.0);
    cr.stroke().ok();

    cr.restore().ok();
}

/// Draw corner decorations for all four corners
fn draw_corner_decorations(
    cr: &Context,
    config: &ArtNouveauFrameConfig,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) {
    let accent_color = config.accent_color.resolve(&config.theme);
    let size = config.corner_size;
    let line_width = config.accent_width;

    match config.corner_style {
        CornerStyle::Flourish => {
            draw_flourish_corner(cr, x + 8.0, y + 8.0, size, 0.0, &accent_color, line_width);
            draw_flourish_corner(
                cr,
                x + w - 8.0,
                y + 8.0,
                size,
                PI / 2.0,
                &accent_color,
                line_width,
            );
            draw_flourish_corner(
                cr,
                x + w - 8.0,
                y + h - 8.0,
                size,
                PI,
                &accent_color,
                line_width,
            );
            draw_flourish_corner(
                cr,
                x + 8.0,
                y + h - 8.0,
                size,
                -PI / 2.0,
                &accent_color,
                line_width,
            );
        }
        CornerStyle::Leaf => {
            draw_leaf_corner(cr, x + 8.0, y + 8.0, size, 0.0, &accent_color, line_width);
            draw_leaf_corner(
                cr,
                x + w - 8.0,
                y + 8.0,
                size,
                PI / 2.0,
                &accent_color,
                line_width,
            );
            draw_leaf_corner(
                cr,
                x + w - 8.0,
                y + h - 8.0,
                size,
                PI,
                &accent_color,
                line_width,
            );
            draw_leaf_corner(
                cr,
                x + 8.0,
                y + h - 8.0,
                size,
                -PI / 2.0,
                &accent_color,
                line_width,
            );
        }
        CornerStyle::Spiral => {
            draw_spiral_corner(cr, x + 8.0, y + 8.0, size, 0.0, &accent_color, line_width);
            draw_spiral_corner(
                cr,
                x + w - 8.0,
                y + 8.0,
                size,
                PI / 2.0,
                &accent_color,
                line_width,
            );
            draw_spiral_corner(
                cr,
                x + w - 8.0,
                y + h - 8.0,
                size,
                PI,
                &accent_color,
                line_width,
            );
            draw_spiral_corner(
                cr,
                x + 8.0,
                y + h - 8.0,
                size,
                -PI / 2.0,
                &accent_color,
                line_width,
            );
        }
        CornerStyle::Bracket => {
            draw_bracket_corner(cr, x + 8.0, y + 8.0, size, 0.0, &accent_color, line_width);
            draw_bracket_corner(
                cr,
                x + w - 8.0,
                y + 8.0,
                size,
                PI / 2.0,
                &accent_color,
                line_width,
            );
            draw_bracket_corner(
                cr,
                x + w - 8.0,
                y + h - 8.0,
                size,
                PI,
                &accent_color,
                line_width,
            );
            draw_bracket_corner(
                cr,
                x + 8.0,
                y + h - 8.0,
                size,
                -PI / 2.0,
                &accent_color,
                line_width,
            );
        }
        CornerStyle::None => {}
    }
}

/// Draw a flowing wave along an edge
fn draw_wave_edge(
    cr: &Context,
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    amplitude: f64,
    frequency: f64,
    color: &Color,
    line_width: f64,
) {
    cr.save().ok();
    cr.set_source_rgba(color.r, color.g, color.b, color.a);
    cr.set_line_width(line_width);
    cr.set_line_cap(cairo::LineCap::Round);

    let length = ((x2 - x1).powi(2) + (y2 - y1).powi(2)).sqrt();
    let angle = (y2 - y1).atan2(x2 - x1);
    let segments = (length / 10.0).max(10.0) as usize;

    cr.move_to(x1, y1);

    for i in 1..=segments {
        let t = i as f64 / segments as f64;
        let wave = (t * frequency * 2.0 * PI).sin() * amplitude;

        let px = x1 + t * (x2 - x1);
        let py = y1 + t * (y2 - y1);

        // Offset perpendicular to the line
        let offset_x = -wave * angle.sin();
        let offset_y = wave * angle.cos();

        cr.line_to(px + offset_x, py + offset_y);
    }
    cr.stroke().ok();
    cr.restore().ok();
}

/// Draw the frame border with organic curves
fn draw_border(cr: &Context, config: &ArtNouveauFrameConfig, x: f64, y: f64, w: f64, h: f64) {
    let border_color = config.border_color.resolve(&config.theme);
    let line_width = config.border_width;

    cr.save().ok();
    cr.set_source_rgba(
        border_color.r,
        border_color.g,
        border_color.b,
        border_color.a,
    );
    cr.set_line_width(line_width);
    cr.set_line_cap(cairo::LineCap::Round);
    cr.set_line_join(cairo::LineJoin::Round);

    match config.border_style {
        BorderStyle::Vine => {
            // Flowing vine border with subtle waves
            let amplitude = 3.0;
            let freq = config.wave_frequency;
            draw_wave_edge(
                cr,
                x,
                y,
                x + w,
                y,
                amplitude,
                freq,
                &border_color,
                line_width,
            );
            draw_wave_edge(
                cr,
                x + w,
                y,
                x + w,
                y + h,
                amplitude,
                freq,
                &border_color,
                line_width,
            );
            draw_wave_edge(
                cr,
                x + w,
                y + h,
                x,
                y + h,
                amplitude,
                freq,
                &border_color,
                line_width,
            );
            draw_wave_edge(
                cr,
                x,
                y + h,
                x,
                y,
                amplitude,
                freq,
                &border_color,
                line_width,
            );
        }
        BorderStyle::Whiplash => {
            // Classic whiplash S-curves at corners
            let curve_size = 20.0;

            // Top edge with whiplash curves
            cr.move_to(x + curve_size, y);
            cr.curve_to(x + curve_size / 2.0, y - 5.0, x + 5.0, y, x, y + curve_size);
            cr.move_to(x + curve_size, y);
            cr.line_to(x + w - curve_size, y);
            cr.curve_to(
                x + w - curve_size / 2.0,
                y - 5.0,
                x + w - 5.0,
                y,
                x + w,
                y + curve_size,
            );

            // Bottom edge
            cr.move_to(x + curve_size, y + h);
            cr.curve_to(
                x + curve_size / 2.0,
                y + h + 5.0,
                x + 5.0,
                y + h,
                x,
                y + h - curve_size,
            );
            cr.move_to(x + curve_size, y + h);
            cr.line_to(x + w - curve_size, y + h);
            cr.curve_to(
                x + w - curve_size / 2.0,
                y + h + 5.0,
                x + w - 5.0,
                y + h,
                x + w,
                y + h - curve_size,
            );

            // Side edges
            cr.move_to(x, y + curve_size);
            cr.line_to(x, y + h - curve_size);
            cr.move_to(x + w, y + curve_size);
            cr.line_to(x + w, y + h - curve_size);

            cr.stroke().ok();
        }
        BorderStyle::Floral => {
            // Simple border with floral accents at midpoints
            cr.rectangle(x, y, w, h);
            cr.stroke().ok();

            // Add small leaf accents at midpoints
            let accent_color = config.accent_color.resolve(&config.theme);
            let leaf_size = 8.0;
            draw_leaf_corner(
                cr,
                x + w / 2.0,
                y,
                leaf_size,
                PI / 2.0,
                &accent_color,
                line_width * 0.8,
            );
            draw_leaf_corner(
                cr,
                x + w / 2.0,
                y + h,
                leaf_size,
                -PI / 2.0,
                &accent_color,
                line_width * 0.8,
            );
            draw_leaf_corner(
                cr,
                x,
                y + h / 2.0,
                leaf_size,
                0.0,
                &accent_color,
                line_width * 0.8,
            );
            draw_leaf_corner(
                cr,
                x + w,
                y + h / 2.0,
                leaf_size,
                PI,
                &accent_color,
                line_width * 0.8,
            );
        }
        BorderStyle::Organic => {
            // Slightly curved organic border
            let bulge = 2.0;

            cr.move_to(x, y);
            cr.curve_to(
                x + w / 3.0,
                y - bulge,
                x + 2.0 * w / 3.0,
                y + bulge,
                x + w,
                y,
            );
            cr.curve_to(
                x + w + bulge,
                y + h / 3.0,
                x + w - bulge,
                y + 2.0 * h / 3.0,
                x + w,
                y + h,
            );
            cr.curve_to(
                x + 2.0 * w / 3.0,
                y + h + bulge,
                x + w / 3.0,
                y + h - bulge,
                x,
                y + h,
            );
            cr.curve_to(x - bulge, y + 2.0 * h / 3.0, x + bulge, y + h / 3.0, x, y);
            cr.stroke().ok();
        }
        BorderStyle::Peacock => {
            // Border with peacock feather "eye" accents
            cr.rectangle(x, y, w, h);
            cr.stroke().ok();

            // Draw peacock eye at each corner area
            let accent_color = config.accent_color.resolve(&config.theme);
            let eye_size = 10.0;
            let inset = 15.0;

            for &(ex, ey) in &[
                (x + inset, y + inset),
                (x + w - inset, y + inset),
                (x + w - inset, y + h - inset),
                (x + inset, y + h - inset),
            ] {
                // Outer ring
                cr.set_source_rgba(accent_color.r, accent_color.g, accent_color.b, 0.6);
                cr.arc(ex, ey, eye_size, 0.0, 2.0 * PI);
                cr.stroke().ok();

                // Inner dot
                cr.set_source_rgba(
                    accent_color.r,
                    accent_color.g,
                    accent_color.b,
                    accent_color.a,
                );
                cr.arc(ex, ey, eye_size * 0.4, 0.0, 2.0 * PI);
                cr.fill().ok();
            }
        }
    }

    cr.restore().ok();
}

/// Draw background pattern
fn draw_background_pattern(
    cr: &Context,
    config: &ArtNouveauFrameConfig,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) {
    let pattern_color = config.pattern_color.resolve(&config.theme);
    let faint_color = Color::new(pattern_color.r, pattern_color.g, pattern_color.b, 0.1);

    cr.save().ok();
    cr.rectangle(x, y, w, h);
    cr.clip();

    match config.background_pattern {
        BackgroundPattern::Solid => {
            // No pattern overlay
        }
        BackgroundPattern::Vines => {
            // Subtle vine pattern
            cr.set_source_rgba(faint_color.r, faint_color.g, faint_color.b, faint_color.a);
            cr.set_line_width(1.0);

            let spacing = config.pattern_spacing;
            let mut yy = y;
            while yy < y + h {
                // Draw a wavy vine line
                cr.move_to(x, yy);
                let mut xx = x;
                while xx < x + w {
                    let wave = 8.0 * ((xx - x) * 0.05).sin();
                    cr.line_to(xx, yy + wave);
                    xx += 5.0;
                }
                cr.stroke().ok();
                yy += spacing;
            }
        }
        BackgroundPattern::Leaves => {
            // Scattered small leaves
            cr.set_source_rgba(
                faint_color.r,
                faint_color.g,
                faint_color.b,
                faint_color.a * 1.5,
            );
            let spacing = config.pattern_spacing;

            let mut yy = y + spacing / 2.0;
            let mut row = 0;
            while yy < y + h {
                let mut xx = x + if row % 2 == 0 { spacing / 2.0 } else { spacing };
                while xx < x + w {
                    // Draw a tiny leaf
                    cr.save().ok();
                    cr.translate(xx, yy);
                    cr.rotate((row + (xx as usize)) as f64 * 0.7);
                    cr.move_to(0.0, 0.0);
                    cr.curve_to(2.0, -1.0, 4.0, -3.0, 5.0, -2.0);
                    cr.curve_to(4.0, -1.0, 2.0, 0.0, 0.0, 0.0);
                    cr.fill().ok();
                    cr.restore().ok();
                    xx += spacing;
                }
                yy += spacing;
                row += 1;
            }
        }
        BackgroundPattern::Waves => {
            // Horizontal flowing waves
            cr.set_source_rgba(faint_color.r, faint_color.g, faint_color.b, faint_color.a);
            cr.set_line_width(1.0);

            let spacing = config.pattern_spacing;
            let amplitude = 6.0;
            let freq = config.wave_frequency * 0.1;

            let mut yy = y + spacing / 2.0;
            while yy < y + h {
                cr.move_to(x, yy);
                let mut xx = x;
                while xx < x + w {
                    let wave = amplitude * (xx * freq).sin();
                    cr.line_to(xx, yy + wave);
                    xx += 3.0;
                }
                cr.stroke().ok();
                yy += spacing;
            }
        }
        BackgroundPattern::Peacock => {
            // Peacock feather eye pattern
            cr.set_source_rgba(
                faint_color.r,
                faint_color.g,
                faint_color.b,
                faint_color.a * 2.0,
            );

            let spacing = config.pattern_spacing * 1.5;
            let eye_size = 8.0;

            let mut yy = y + spacing / 2.0;
            let mut row = 0;
            while yy < y + h {
                let offset = if row % 2 == 0 { 0.0 } else { spacing / 2.0 };
                let mut xx = x + spacing / 2.0 + offset;
                while xx < x + w {
                    // Draw concentric circles
                    cr.set_line_width(0.5);
                    cr.arc(xx, yy, eye_size, 0.0, 2.0 * PI);
                    cr.stroke().ok();
                    cr.arc(xx, yy, eye_size * 0.5, 0.0, 2.0 * PI);
                    cr.stroke().ok();
                    cr.arc(xx, yy, eye_size * 0.2, 0.0, 2.0 * PI);
                    cr.fill().ok();

                    xx += spacing;
                }
                yy += spacing;
                row += 1;
            }
        }
    }

    cr.restore().ok();
}

/// Draw the header area
fn draw_header(cr: &Context, config: &ArtNouveauFrameConfig, x: f64, y: f64, w: f64) -> f64 {
    if !config.show_header || matches!(config.header_style, HeaderStyle::None) {
        return 0.0;
    }

    let header_color = config.header_color.resolve(&config.theme);
    let accent_color = config.accent_color.resolve(&config.theme);
    let (font_family, font_size) = config.header_font.resolve(&config.theme);

    cr.save().ok();

    // Measure text
    let header_height = font_size + 20.0;

    let text_extents = pango_text_extents(
        cr,
        &config.header_text,
        &font_family,
        cairo::FontSlant::Normal,
        cairo::FontWeight::Normal,
        font_size,
    );
    let (text_width, text_height) = (text_extents.width(), text_extents.height().max(font_size));

    let text_x = x + (w - text_width) / 2.0;
    let text_y = y + header_height / 2.0 + text_height / 2.0;

    match config.header_style {
        HeaderStyle::Banner => {
            // Flowing banner with curved bottom
            cr.set_source_rgba(accent_color.r, accent_color.g, accent_color.b, 0.15);
            cr.move_to(x, y);
            cr.line_to(x + w, y);
            cr.line_to(x + w, y + header_height - 5.0);
            cr.curve_to(
                x + 2.0 * w / 3.0,
                y + header_height + 3.0,
                x + w / 3.0,
                y + header_height - 3.0,
                x,
                y + header_height - 5.0,
            );
            cr.close_path();
            cr.fill().ok();
        }
        HeaderStyle::Arch => {
            // Organic arch header
            cr.set_source_rgba(accent_color.r, accent_color.g, accent_color.b, 0.15);
            cr.move_to(x, y + header_height);
            cr.curve_to(
                x + w / 4.0,
                y,
                x + 3.0 * w / 4.0,
                y,
                x + w,
                y + header_height,
            );
            cr.close_path();
            cr.fill().ok();
        }
        HeaderStyle::Flourish => {
            // Header with flourish decorations on sides
            cr.set_source_rgba(accent_color.r, accent_color.g, accent_color.b, 0.1);
            cr.rectangle(x, y, w, header_height);
            cr.fill().ok();

            // Draw flourishes on either side of text
            let flourish_width = (text_x - x - 20.0).max(0.0);
            if flourish_width > 10.0 {
                cr.set_source_rgba(accent_color.r, accent_color.g, accent_color.b, 0.4);
                cr.set_line_width(1.5);

                // Left flourish
                cr.move_to(x + 10.0, text_y);
                cr.curve_to(
                    x + flourish_width / 3.0,
                    text_y - 5.0,
                    x + 2.0 * flourish_width / 3.0,
                    text_y + 3.0,
                    text_x - 10.0,
                    text_y,
                );
                cr.stroke().ok();

                // Right flourish
                let right_start = text_x + text_width + 10.0;
                cr.move_to(right_start, text_y);
                cr.curve_to(
                    right_start + flourish_width / 3.0,
                    text_y + 3.0,
                    right_start + 2.0 * flourish_width / 3.0,
                    text_y - 5.0,
                    x + w - 10.0,
                    text_y,
                );
                cr.stroke().ok();
            }
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
        cairo::FontWeight::Normal,
        font_size,
    );

    cr.restore().ok();

    header_height
}

/// Draw a divider between content groups
fn draw_divider(
    cr: &Context,
    config: &ArtNouveauFrameConfig,
    x: f64,
    y: f64,
    length: f64,
    horizontal: bool,
) {
    if matches!(config.divider_style, DividerStyle::None) {
        return;
    }

    let divider_color = config.divider_color.resolve(&config.theme);

    cr.save().ok();
    cr.set_source_rgba(
        divider_color.r,
        divider_color.g,
        divider_color.b,
        divider_color.a,
    );
    cr.set_line_width(config.divider_width);
    cr.set_line_cap(cairo::LineCap::Round);

    match config.divider_style {
        DividerStyle::Vine => {
            // Vine with small leaf offshoots
            let segments = (length / 30.0).max(3.0) as usize;

            if horizontal {
                cr.move_to(x, y);
                for i in 1..=segments {
                    let t = i as f64 / segments as f64;
                    let px = x + t * length;
                    let wave = 3.0 * (t * 4.0 * PI).sin();
                    cr.line_to(px, y + wave);
                }
                cr.stroke().ok();

                // Add leaf offshoots
                for i in 1..segments {
                    let t = i as f64 / segments as f64;
                    let px = x + t * length;
                    let wave = 3.0 * (t * 4.0 * PI).sin();
                    let direction = if i % 2 == 0 { 1.0 } else { -1.0 };

                    cr.move_to(px, y + wave);
                    cr.curve_to(
                        px + 3.0,
                        y + wave + direction * 3.0,
                        px + 6.0,
                        y + wave + direction * 5.0,
                        px + 5.0,
                        y + wave + direction * 8.0,
                    );
                    cr.stroke().ok();
                }
            } else {
                cr.move_to(x, y);
                for i in 1..=segments {
                    let t = i as f64 / segments as f64;
                    let py = y + t * length;
                    let wave = 3.0 * (t * 4.0 * PI).sin();
                    cr.line_to(x + wave, py);
                }
                cr.stroke().ok();

                for i in 1..segments {
                    let t = i as f64 / segments as f64;
                    let py = y + t * length;
                    let wave = 3.0 * (t * 4.0 * PI).sin();
                    let direction = if i % 2 == 0 { 1.0 } else { -1.0 };

                    cr.move_to(x + wave, py);
                    cr.curve_to(
                        x + wave + direction * 3.0,
                        py + 3.0,
                        x + wave + direction * 5.0,
                        py + 6.0,
                        x + wave + direction * 8.0,
                        py + 5.0,
                    );
                    cr.stroke().ok();
                }
            }
        }
        DividerStyle::Wave => {
            // Flowing sine wave
            let amplitude = 4.0;
            let freq = config.wave_frequency;

            if horizontal {
                draw_wave_edge(
                    cr,
                    x,
                    y,
                    x + length,
                    y,
                    amplitude,
                    freq,
                    &divider_color,
                    config.divider_width,
                );
            } else {
                draw_wave_edge(
                    cr,
                    x,
                    y,
                    x,
                    y + length,
                    amplitude,
                    freq,
                    &divider_color,
                    config.divider_width,
                );
            }
        }
        DividerStyle::Tendril => {
            // Curling tendril divider
            if horizontal {
                cr.move_to(x, y);
                cr.curve_to(
                    x + length * 0.25,
                    y - 5.0,
                    x + length * 0.5,
                    y + 5.0,
                    x + length * 0.75,
                    y - 3.0,
                );
                cr.curve_to(
                    x + length * 0.85,
                    y - 5.0,
                    x + length * 0.95,
                    y,
                    x + length,
                    y,
                );
                cr.stroke().ok();

                // Add a small curl at center
                let cx = x + length / 2.0;
                cr.arc(cx, y + 2.0, 4.0, PI, 2.0 * PI);
                cr.stroke().ok();
            } else {
                cr.move_to(x, y);
                cr.curve_to(
                    x - 5.0,
                    y + length * 0.25,
                    x + 5.0,
                    y + length * 0.5,
                    x - 3.0,
                    y + length * 0.75,
                );
                cr.curve_to(
                    x - 5.0,
                    y + length * 0.85,
                    x,
                    y + length * 0.95,
                    x,
                    y + length,
                );
                cr.stroke().ok();

                let cy = y + length / 2.0;
                cr.arc(x + 2.0, cy, 4.0, PI / 2.0, 3.0 * PI / 2.0);
                cr.stroke().ok();
            }
        }
        DividerStyle::Line => {
            // Simple curved line
            if horizontal {
                cr.move_to(x, y);
                cr.curve_to(
                    x + length / 3.0,
                    y - 2.0,
                    x + 2.0 * length / 3.0,
                    y + 2.0,
                    x + length,
                    y,
                );
            } else {
                cr.move_to(x, y);
                cr.curve_to(
                    x - 2.0,
                    y + length / 3.0,
                    x + 2.0,
                    y + 2.0 * length / 3.0,
                    x,
                    y + length,
                );
            }
            cr.stroke().ok();
        }
        DividerStyle::None => {}
    }

    cr.restore().ok();
}

/// Render the complete Art Nouveau frame
/// Returns the content area bounds (x, y, width, height)
pub fn render_art_nouveau_frame(
    cr: &Context,
    config: &ArtNouveauFrameConfig,
    width: f64,
    height: f64,
) -> Result<(f64, f64, f64, f64)> {
    if width < 1.0 || height < 1.0 {
        return Ok((0.0, 0.0, 0.0, 0.0));
    }

    cr.save()?;

    let background_color = config.background_color.resolve(&config.theme);

    let margin = config.border_width + 4.0;
    let frame_x = margin;
    let frame_y = margin;
    let frame_w = (width - margin * 2.0).max(1.0);
    let frame_h = (height - margin * 2.0).max(1.0);

    // Draw background fill
    cr.set_source_rgba(
        background_color.r,
        background_color.g,
        background_color.b,
        background_color.a,
    );
    cr.rectangle(frame_x, frame_y, frame_w, frame_h);
    cr.fill()?;

    // Draw background pattern
    draw_background_pattern(cr, config, frame_x, frame_y, frame_w, frame_h);

    // Draw border
    draw_border(cr, config, frame_x, frame_y, frame_w, frame_h);

    // Draw corner decorations
    draw_corner_decorations(cr, config, frame_x, frame_y, frame_w, frame_h);

    // Draw header and get its height
    let header_height = draw_header(cr, config, frame_x, frame_y, frame_w);

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
    config: &ArtNouveauFrameConfig,
    content_x: f64,
    content_y: f64,
    content_w: f64,
    content_h: f64,
) -> Vec<(f64, f64, f64, f64)> {
    let group_count = config.group_count.max(1);
    let mut layouts = Vec::with_capacity(group_count);

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
    config: &ArtNouveauFrameConfig,
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
