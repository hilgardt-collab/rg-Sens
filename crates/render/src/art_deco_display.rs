//! Art Deco display rendering
//!
//! A 1920s-inspired frame: polished-metal rules, stepped (ziggurat)
//! silhouettes, sunburst fans, chevron and zigzag bands, and strict
//! bilateral symmetry. Every ornament is filled or double-ruled rather than
//! hairline, sized from the configured widths and the panel size, and
//! coloured from the resolved theme colours (the "metal" gradients are
//! derived from the configured colour, so a copper or silver palette gets
//! copper or silver metal).
//!
//! Layout contract (unchanged across redesigns): the frame rectangle is
//! inset by `border_width + 4`, the content area by `content_padding`
//! inside that plus the header height, and groups are separated by
//! `divider_width + 2 * divider_padding`.

use anyhow::Result;
use cairo::Context;
use std::f64::consts::{FRAC_PI_2, PI};

use crate::background::Color;
use crate::combo_traits::FrameRenderer;
use crate::pango_text::get_font_description;
use pangocairo::functions::{create_layout, show_layout};

pub use rg_sens_types::display_configs::art_deco::*;

pub struct ArtDecoRenderer;

impl FrameRenderer for ArtDecoRenderer {
    type Config = ArtDecoFrameConfig;

    fn theme_id(&self) -> &'static str {
        "art_deco"
    }

    fn theme_name(&self) -> &'static str {
        "Art Deco"
    }

    fn default_config(&self) -> Self::Config {
        ArtDecoFrameConfig::default()
    }

    fn render_frame(
        &self,
        cr: &Context,
        config: &Self::Config,
        width: f64,
        height: f64,
    ) -> anyhow::Result<(f64, f64, f64, f64)> {
        render_art_deco_frame(cr, config, width, height).map_err(|e| anyhow::anyhow!("{}", e))
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

// ---------------------------------------------------------------------------
// Colour and geometry helpers
// ---------------------------------------------------------------------------

/// Detail unit for ornaments that have no configured size: a 48th of the
/// panel's shorter side, kept legible.
fn unit(w: f64, h: f64) -> f64 {
    (w.min(h) / 48.0).clamp(4.0, 12.0)
}

fn set_color(cr: &Context, c: &Color) {
    cr.set_source_rgba(c.r, c.g, c.b, c.a);
}

fn set_color_alpha(cr: &Context, c: &Color, alpha: f64) {
    cr.set_source_rgba(c.r, c.g, c.b, c.a * alpha);
}

fn lighten(c: &Color, k: f64) -> Color {
    Color::new(
        c.r + (1.0 - c.r) * k,
        c.g + (1.0 - c.g) * k,
        c.b + (1.0 - c.b) * k,
        c.a,
    )
}

fn darken(c: &Color, k: f64) -> Color {
    Color::new(c.r * (1.0 - k), c.g * (1.0 - k), c.b * (1.0 - k), c.a)
}

/// Polished metal: dark – bright – dark along the given axis, derived from
/// the colour itself so every palette keeps its hue.
fn metal_gradient(c: &Color, x0: f64, y0: f64, x1: f64, y1: f64) -> cairo::LinearGradient {
    let g = cairo::LinearGradient::new(x0, y0, x1, y1);
    let d = darken(c, 0.30);
    let b = lighten(c, 0.40);
    g.add_color_stop_rgba(0.0, d.r, d.g, d.b, c.a);
    g.add_color_stop_rgba(0.5, b.r, b.g, b.b, c.a);
    g.add_color_stop_rgba(1.0, d.r, d.g, d.b, c.a);
    g
}

/// A rule that fades to transparent at both ends.
fn fading_gradient(
    c: &Color,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    alpha: f64,
) -> cairo::LinearGradient {
    let g = cairo::LinearGradient::new(x0, y0, x1, y1);
    g.add_color_stop_rgba(0.0, c.r, c.g, c.b, 0.0);
    g.add_color_stop_rgba(0.18, c.r, c.g, c.b, c.a * alpha);
    g.add_color_stop_rgba(0.82, c.r, c.g, c.b, c.a * alpha);
    g.add_color_stop_rgba(1.0, c.r, c.g, c.b, 0.0);
    g
}

/// A rule that is transparent at (x0, y0) and solid at (x1, y1).
fn fade_in_gradient(
    c: &Color,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    alpha: f64,
) -> cairo::LinearGradient {
    let g = cairo::LinearGradient::new(x0, y0, x1, y1);
    g.add_color_stop_rgba(0.0, c.r, c.g, c.b, 0.0);
    g.add_color_stop_rgba(1.0, c.r, c.g, c.b, c.a * alpha);
    g
}

fn diamond_path(cr: &Context, cx: f64, cy: f64, r: f64) {
    cr.new_sub_path();
    cr.move_to(cx, cy - r);
    cr.line_to(cx + r, cy);
    cr.line_to(cx, cy + r);
    cr.line_to(cx - r, cy);
    cr.close_path();
}

/// A filled metal diamond with a thin dark edge: the "jewel" of the style.
fn jewel(cr: &Context, cx: f64, cy: f64, r: f64, c: &Color) {
    cr.save().ok();
    diamond_path(cr, cx, cy, r);
    let g = metal_gradient(c, cx - r, cy, cx + r, cy);
    cr.set_source(&g).ok();
    cr.fill_preserve().ok();
    let d = darken(c, 0.45);
    set_color(cr, &d);
    cr.set_line_width(1.0);
    cr.stroke().ok();
    cr.restore().ok();
}

/// Rectangle with staircase corners (`steps` steps over depth `d`).
fn stepped_rect_path(cr: &Context, x: f64, y: f64, w: f64, h: f64, d: f64, steps: usize) {
    let steps = steps.max(1);
    let s = d / steps as f64;
    let n = steps as f64;
    cr.new_sub_path();
    cr.move_to(x + d, y);
    cr.line_to(x + w - d, y);
    for i in 0..steps {
        let (px, py) = (x + w - d + i as f64 * s, y + i as f64 * s);
        cr.line_to(px + s, py);
        cr.line_to(px + s, py + s);
    }
    cr.line_to(x + w, y + h - d);
    for i in 0..steps {
        let (px, py) = (x + w - i as f64 * s, y + h - d + i as f64 * s);
        cr.line_to(px, py + s);
        cr.line_to(px - s, py + s);
    }
    cr.line_to(x + d, y + h);
    for i in 0..steps {
        let (px, py) = (x + d - i as f64 * s, y + h - i as f64 * s);
        cr.line_to(px - s, py);
        cr.line_to(px - s, py - s);
    }
    cr.line_to(x, y + d);
    for i in 0..steps {
        let (px, py) = (x + i as f64 * s, y + d - i as f64 * s);
        cr.line_to(px, py - s);
        cr.line_to(px + s, py - s);
    }
    let _ = n;
    cr.close_path();
}

/// Depth of the staircase corners of a `Stepped` border: as much as the
/// configured corner size allows without reaching into the content area.
fn step_depth(config: &ArtDecoFrameConfig) -> f64 {
    config.corner_size.min(config.content_padding).max(6.0)
}

/// The frame silhouette: a rectangle, or a stepped one for `Stepped`.
fn frame_path(cr: &Context, config: &ArtDecoFrameConfig, x: f64, y: f64, w: f64, h: f64) {
    match config.border_style {
        BorderStyle::Stepped => stepped_rect_path(cr, x, y, w, h, step_depth(config), 3),
        _ => {
            cr.new_sub_path();
            cr.rectangle(x, y, w, h);
        }
    }
}

/// Zigzag polyline from (x0, y) to (x1, y) with the given period and
/// amplitude; starts and ends on the centre line.
fn zigzag_path(cr: &Context, x0: f64, x1: f64, y: f64, period: f64, amplitude: f64) {
    let len = x1 - x0;
    if len <= 0.0 || period <= 0.0 {
        return;
    }
    let n = (len / period).round().max(1.0);
    let p = len / n;
    cr.new_sub_path();
    cr.move_to(x0, y);
    let mut px = x0;
    for i in 0..n as usize {
        let _ = i;
        cr.line_to(px + p * 0.25, y - amplitude);
        cr.line_to(px + p * 0.75, y + amplitude);
        px += p;
    }
    cr.line_to(x1, y);
}

// ---------------------------------------------------------------------------
// Text
// ---------------------------------------------------------------------------

/// Uppercase, letter-spaced title layout (the "reads as Deco" typography).
fn title_layout(cr: &Context, text: &str, family: &str, size: f64) -> pango::Layout {
    let desc = get_font_description(
        family,
        cairo::FontSlant::Normal,
        cairo::FontWeight::Bold,
        size,
    );
    let layout = create_layout(cr);
    layout.set_font_description(Some(&desc));
    layout.set_text(&text.to_uppercase());
    let attrs = pango::AttrList::new();
    let tracking = (size * 0.22 * pango::SCALE as f64) as i32;
    attrs.insert(pango::AttrInt::new_letter_spacing(tracking));
    layout.set_attributes(Some(&attrs));
    layout
}

// ---------------------------------------------------------------------------
// Background
// ---------------------------------------------------------------------------

fn draw_background(cr: &Context, config: &ArtDecoFrameConfig, x: f64, y: f64, w: f64, h: f64) {
    let bg = config.background_color.resolve(&config.theme);
    cr.save().ok();
    frame_path(cr, config, x, y, w, h);
    cr.clip();
    set_color(cr, &bg);
    cr.paint().ok();
    // Vertical vignette: deeper at the top and bottom edges.
    let vig = cairo::LinearGradient::new(0.0, y, 0.0, y + h);
    vig.add_color_stop_rgba(0.0, 0.0, 0.0, 0.0, 0.30);
    vig.add_color_stop_rgba(0.16, 0.0, 0.0, 0.0, 0.0);
    vig.add_color_stop_rgba(0.84, 0.0, 0.0, 0.0, 0.0);
    vig.add_color_stop_rgba(1.0, 0.0, 0.0, 0.0, 0.30);
    cr.set_source(&vig).ok();
    cr.paint().ok();
    draw_background_pattern(cr, config, x, y, w, h);
    cr.restore().ok();
}

fn draw_background_pattern(
    cr: &Context,
    config: &ArtDecoFrameConfig,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) {
    let c = config.pattern_color.resolve(&config.theme);
    let spacing = config.pattern_spacing.max(4.0);
    cr.save().ok();
    match config.background_pattern {
        BackgroundPattern::Solid => {}
        BackgroundPattern::VerticalLines => {
            set_color_alpha(cr, &c, 0.14);
            cr.set_line_width(1.0);
            let mut px = x + spacing;
            while px < x + w {
                cr.move_to(px.round() + 0.5, y);
                cr.line_to(px.round() + 0.5, y + h);
                px += spacing;
            }
            cr.stroke().ok();
        }
        BackgroundPattern::DiamondGrid => {
            // 45° lattice.
            set_color_alpha(cr, &c, 0.10);
            cr.set_line_width(1.0);
            let step = spacing * 2.0;
            let mut d = -h;
            while d < w {
                cr.move_to(x + d, y);
                cr.line_to(x + d + h, y + h);
                cr.move_to(x + d + h, y);
                cr.line_to(x + d, y + h);
                d += step;
            }
            cr.stroke().ok();
        }
        BackgroundPattern::Sunburst => {
            // Rays from the bottom centre: a rising sun behind the content.
            set_color_alpha(cr, &c, 0.09);
            cr.set_line_width(1.0);
            let rays = config.sunburst_rays.max(4);
            let (cx, cy) = (x + w / 2.0, y + h);
            let r = (w * w + h * h).sqrt();
            for i in 0..=rays {
                let a = PI + PI * i as f64 / rays as f64;
                cr.move_to(cx, cy);
                cr.line_to(cx + r * a.cos(), cy + r * a.sin());
            }
            cr.stroke().ok();
        }
        BackgroundPattern::Chevrons => {
            set_color_alpha(cr, &c, 0.10);
            cr.set_line_width(1.0);
            let period = spacing * 2.0;
            let mut py = y + spacing;
            while py < y + h + spacing {
                zigzag_path(cr, x, x + w, py, period, spacing * 0.4);
                py += spacing;
            }
            cr.stroke().ok();
        }
    }
    cr.restore().ok();
}

// ---------------------------------------------------------------------------
// Border
// ---------------------------------------------------------------------------

fn draw_border(cr: &Context, config: &ArtDecoFrameConfig, x: f64, y: f64, w: f64, h: f64, u: f64) {
    let c = config.border_color.resolve(&config.theme);
    let bw = config.border_width.max(0.5);
    cr.save().ok();

    // Outer rule: polished metal.
    frame_path(cr, config, x, y, w, h);
    let metal = metal_gradient(&c, 0.0, y, 0.0, y + h);
    cr.set_source(&metal).ok();
    cr.set_line_width(bw);
    cr.stroke().ok();

    let inner_w = (bw * 0.4).max(1.0);
    let inset = bw * 1.5 + 2.0;
    match config.border_style {
        BorderStyle::Geometric | BorderStyle::Ornate | BorderStyle::Stepped => {
            // Inner rule.
            set_color_alpha(cr, &c, 0.75);
            cr.set_line_width(inner_w);
            let (ix, iy, iw, ih) = (x + inset, y + inset, w - inset * 2.0, h - inset * 2.0);
            match config.border_style {
                BorderStyle::Stepped => {
                    let d = (step_depth(config) - inset).max(3.0);
                    stepped_rect_path(cr, ix, iy, iw, ih, d, 3);
                }
                _ => {
                    cr.new_sub_path();
                    cr.rectangle(ix, iy, iw, ih);
                }
            }
            cr.stroke().ok();
        }
        BorderStyle::Chevron => {
            // A zigzag band inside the rule, clear of the corners.
            let amp = (u * 0.35).max(1.5);
            let cy_off = inset + amp;
            let skip = config.corner_size.max(u * 2.0);
            let period = (u * 1.6).max(6.0);
            set_color_alpha(cr, &c, 0.85);
            cr.set_line_width(config.accent_width.max(1.0));
            zigzag_path(cr, x + skip, x + w - skip, y + cy_off, period, amp);
            zigzag_path(cr, x + skip, x + w - skip, y + h - cy_off, period, amp);
            cr.save().ok();
            cr.translate(x + cy_off, y);
            cr.rotate(FRAC_PI_2);
            zigzag_path(cr, skip, h - skip, 0.0, period, amp);
            cr.restore().ok();
            cr.save().ok();
            cr.translate(x + w - cy_off, y);
            cr.rotate(FRAC_PI_2);
            zigzag_path(cr, skip, h - skip, 0.0, period, amp);
            cr.restore().ok();
            cr.stroke().ok();
        }
        BorderStyle::Sunburst => {
            // A halo of ray ticks around each corner, inside the rule.
            let r0 = config.corner_size.max(u * 2.0) * 1.25;
            let len = u * 1.6;
            set_color_alpha(cr, &c, 0.8);
            cr.set_line_width(config.accent_width.max(1.0));
            for (cx, cy, a0) in [
                (x + bw, y + bw, 0.0),
                (x + w - bw, y + bw, FRAC_PI_2),
                (x + w - bw, y + h - bw, PI),
                (x + bw, y + h - bw, PI + FRAC_PI_2),
            ] {
                for i in 0..9 {
                    let a = a0 + FRAC_PI_2 * (i as f64 + 0.5) / 9.0;
                    cr.move_to(cx + r0 * a.cos(), cy + r0 * a.sin());
                    cr.line_to(cx + (r0 + len) * a.cos(), cy + (r0 + len) * a.sin());
                }
            }
            cr.stroke().ok();
        }
    }

    if matches!(config.border_style, BorderStyle::Ornate) {
        // Jewels at the edge midpoints, sitting on the outer rule.
        let r = (u * 0.8).max(bw * 1.5);
        for (cx, cy) in [
            (x + w / 2.0, y),
            (x + w / 2.0, y + h),
            (x, y + h / 2.0),
            (x + w, y + h / 2.0),
        ] {
            jewel(cr, cx, cy, r, &c);
        }
    }
    cr.restore().ok();
}

// ---------------------------------------------------------------------------
// Corners
// ---------------------------------------------------------------------------

/// Draw `f` at each corner. `f` draws the top-left ornament with the origin
/// just inside the rule and +x, +y pointing into the panel; the other
/// corners are mirrored.
fn for_each_corner<F: Fn(&Context)>(
    cr: &Context,
    config: &ArtDecoFrameConfig,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    f: F,
) {
    let bw = config.border_width.max(0.5);
    let corners = [
        (x + bw, y + bw, 1.0, 1.0),
        (x + w - bw, y + bw, -1.0, 1.0),
        (x + w - bw, y + h - bw, -1.0, -1.0),
        (x + bw, y + h - bw, 1.0, -1.0),
    ];
    for (cx, cy, sx, sy) in corners {
        cr.save().ok();
        // Keep ornaments inside the frame silhouette (stepped corners cut
        // them like everything else).
        frame_path(cr, config, x, y, w, h);
        cr.clip();
        cr.translate(cx, cy);
        cr.scale(sx, sy);
        f(cr);
        cr.restore().ok();
    }
}

fn draw_corner_decorations(
    cr: &Context,
    config: &ArtDecoFrameConfig,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    u: f64,
) {
    let c = config.accent_color.resolve(&config.theme);
    let r = config.corner_size.max(4.0);
    let lw = config.accent_width.max(1.0);
    let bw = config.border_width.max(0.5);

    match config.corner_style {
        CornerStyle::None => {}
        CornerStyle::Fan => for_each_corner(cr, config, x, y, w, h, |cr| {
            // Quarter sunburst: alternating metal wedges, a rim and a hub.
            let wedges = 5;
            let g = metal_gradient(&c, 0.0, 0.0, r, 0.0);
            for i in 0..wedges {
                let a0 = FRAC_PI_2 * i as f64 / wedges as f64;
                let a1 = FRAC_PI_2 * (i + 1) as f64 / wedges as f64;
                cr.new_sub_path();
                cr.move_to(0.0, 0.0);
                cr.arc(0.0, 0.0, r, a0, a1);
                cr.close_path();
                if i % 2 == 0 {
                    cr.set_source(&g).ok();
                } else {
                    set_color_alpha(cr, &c, 0.14);
                }
                cr.fill().ok();
            }
            set_color_alpha(cr, &c, 0.95);
            cr.set_line_width(lw * 1.2);
            cr.new_sub_path();
            cr.arc(0.0, 0.0, r, 0.0, FRAC_PI_2);
            cr.stroke().ok();
            cr.new_sub_path();
            cr.move_to(0.0, 0.0);
            cr.arc(0.0, 0.0, r * 0.26, 0.0, FRAC_PI_2);
            cr.close_path();
            let hub = darken(&c, 0.35);
            set_color(cr, &hub);
            cr.fill().ok();
        }),
        CornerStyle::Ziggurat => for_each_corner(cr, config, x, y, w, h, |cr| {
            // The staircase from the reference: five steps between the two
            // edge runs, a small square at the end of the top run.
            let s = r / 8.0;
            let g = metal_gradient(&c, 0.0, r, r, 0.0);
            cr.set_source(&g).ok();
            cr.set_line_width(lw * 1.4);
            cr.new_sub_path();
            cr.move_to(0.0, r);
            cr.line_to(0.0, r * 0.625);
            for i in 0..5 {
                let px = i as f64 * s;
                let py = r * 0.625 - i as f64 * s;
                cr.line_to(px + s, py);
                cr.line_to(px + s, py - s);
            }
            cr.line_to(r, 0.0);
            cr.stroke().ok();
            cr.rectangle(r * 0.75, 0.0, s * 0.8, s * 0.8);
            cr.fill().ok();
            // Inner echo.
            set_color_alpha(cr, &c, 0.45);
            cr.set_line_width(lw * 0.8);
            let o = s * 1.6;
            cr.new_sub_path();
            cr.move_to(o, r * 0.75);
            cr.line_to(o, r * 0.5 + o * 0.5);
            for i in 0..3 {
                let px = o + i as f64 * s;
                let py = r * 0.5 + o * 0.5 - i as f64 * s;
                cr.line_to(px + s, py);
                cr.line_to(px + s, py - s);
            }
            cr.line_to(r * 0.75, o);
            cr.stroke().ok();
        }),
        CornerStyle::Diamond => for_each_corner(cr, config, x, y, w, h, |cr| {
            let d = r * 0.32;
            jewel(cr, d + lw, d + lw, d, &c);
            set_color_alpha(cr, &c, 0.8);
            cr.set_line_width(lw);
            cr.move_to(d * 2.0 + lw * 2.0, d + lw);
            cr.line_to(r, d + lw);
            cr.move_to(d + lw, d * 2.0 + lw * 2.0);
            cr.line_to(d + lw, r);
            cr.stroke().ok();
            cr.arc(r, d + lw, lw, 0.0, 2.0 * PI);
            cr.fill().ok();
            cr.arc(d + lw, r, lw, 0.0, 2.0 * PI);
            cr.fill().ok();
        }),
        CornerStyle::Bracket => for_each_corner(cr, config, x, y, w, h, |cr| {
            let t = (bw + 1.0).max(lw);
            let g = metal_gradient(&c, 0.0, r, r, 0.0);
            cr.set_source(&g).ok();
            cr.set_line_width(t);
            cr.move_to(t / 2.0, r);
            cr.line_to(t / 2.0, t / 2.0);
            cr.line_to(r, t / 2.0);
            cr.stroke().ok();
            set_color_alpha(cr, &c, 0.6);
            cr.set_line_width(lw);
            let o = t + u * 0.6;
            cr.move_to(o, r * 0.6);
            cr.line_to(o, o);
            cr.line_to(r * 0.6, o);
            cr.stroke().ok();
        }),
        CornerStyle::DoubleBracket => for_each_corner(cr, config, x, y, w, h, |cr| {
            set_color_alpha(cr, &c, 0.9);
            cr.set_line_width(lw);
            for (o, len) in [(lw, r), (lw + u * 0.7, r * 0.62)] {
                cr.move_to(o, len);
                cr.line_to(o, o);
                cr.line_to(len, o);
            }
            cr.stroke().ok();
            // Step between the two brackets.
            let o = lw + u * 0.35;
            cr.set_line_width(lw * 0.8);
            cr.move_to(o, r * 0.8);
            cr.line_to(o, o);
            cr.line_to(r * 0.8, o);
            cr.stroke().ok();
        }),
        CornerStyle::Hexagon | CornerStyle::Octagon | CornerStyle::Circle => {
            let sides = match config.corner_style {
                CornerStyle::Hexagon => 6,
                CornerStyle::Octagon => 8,
                _ => 0,
            };
            for_each_corner(cr, config, x, y, w, h, |cr| {
                let m = r * 0.42;
                let rad = r * 0.3;
                let bgc = config.background_color.resolve(&config.theme);
                // Lines from the medallion along both edges.
                set_color_alpha(cr, &c, 0.8);
                cr.set_line_width(lw);
                cr.move_to(m, m);
                cr.line_to(r * 1.4, m);
                cr.move_to(m, m);
                cr.line_to(m, r * 1.4);
                cr.stroke().ok();
                // Medallion: dark disc, metal ring, hub.
                medallion_path(cr, m, m, rad, sides);
                set_color(cr, &bgc);
                cr.fill_preserve().ok();
                let g = metal_gradient(&c, m - rad, m, m + rad, m);
                cr.set_source(&g).ok();
                cr.set_line_width(lw * 1.5);
                cr.stroke().ok();
                medallion_path(cr, m, m, rad * 0.45, sides);
                cr.set_source(&g).ok();
                cr.fill().ok();
            });
        }
        CornerStyle::GeometricStack => for_each_corner(cr, config, x, y, w, h, |cr| {
            jewel(cr, r * 0.3, r * 0.3, r * 0.2, &c);
            set_color_alpha(cr, &c, 0.9);
            cr.set_line_width(lw);
            cr.new_sub_path();
            cr.arc(r * 0.62, r * 0.62, r * 0.12, 0.0, 2.0 * PI);
            cr.stroke().ok();
            cr.move_to(r * 0.52, r * 0.3);
            cr.line_to(r * 1.2, r * 0.3);
            cr.move_to(r * 0.3, r * 0.52);
            cr.line_to(r * 0.3, r * 1.2);
            cr.stroke().ok();
            set_color_alpha(cr, &c, 0.5);
            cr.move_to(r * 0.72, r * 0.62);
            cr.line_to(r * 1.2, r * 0.62);
            cr.move_to(r * 0.62, r * 0.72);
            cr.line_to(r * 0.62, r * 1.2);
            cr.stroke().ok();
        }),
    }
}

fn medallion_path(cr: &Context, cx: f64, cy: f64, r: f64, sides: usize) {
    cr.new_sub_path();
    if sides == 0 {
        cr.arc(cx, cy, r, 0.0, 2.0 * PI);
        return;
    }
    for i in 0..sides {
        let a = -FRAC_PI_2 + 2.0 * PI * i as f64 / sides as f64;
        let (px, py) = (cx + r * a.cos(), cy + r * a.sin());
        if i == 0 {
            cr.move_to(px, py);
        } else {
            cr.line_to(px, py);
        }
    }
    cr.close_path();
}

// ---------------------------------------------------------------------------
// Header
// ---------------------------------------------------------------------------

/// Draw the header; returns its height (0 when hidden).
fn draw_header(cr: &Context, config: &ArtDecoFrameConfig, x: f64, y: f64, w: f64, u: f64) -> f64 {
    if !config.show_header || config.header_text.is_empty() {
        return 0.0;
    }
    let (family, size) = config.header_font.resolve(&config.theme);
    let hc = config.header_color.resolve(&config.theme);
    let ac = config.accent_color.resolve(&config.theme);
    let hh = size + 20.0;
    let clear = config.corner_size.max(u * 2.0) + u;

    cr.save().ok();
    let layout = title_layout(cr, &config.header_text, &family, size);
    let (tw, th) = layout.pixel_size();
    let (tw, th) = (tw as f64, th as f64);
    let tx = x + (w - tw) / 2.0;
    let ty = y + (hh - th) / 2.0;
    let mid = y + hh / 2.0;

    match config.header_style {
        HeaderStyle::None => {}
        HeaderStyle::Centered => {
            // Rules fading in towards the title, jewel at each end.
            let gap = u * 1.6;
            let jr = (u * 0.55).max(3.0);
            let left_end = tx - gap;
            let right_start = tx + tw + gap;
            cr.set_line_width(1.5);
            if left_end - jr * 2.0 > x + clear {
                let g = fade_in_gradient(&ac, x + clear, 0.0, left_end, 0.0, 0.9);
                cr.set_source(&g).ok();
                cr.move_to(x + clear, mid);
                cr.line_to(left_end - jr * 2.0, mid);
                cr.stroke().ok();
                jewel(cr, left_end - jr, mid, jr, &ac);
            }
            if right_start + jr * 2.0 < x + w - clear {
                let g = fade_in_gradient(&ac, x + w - clear, 0.0, right_start, 0.0, 0.9);
                cr.set_source(&g).ok();
                cr.move_to(right_start + jr * 2.0, mid);
                cr.line_to(x + w - clear, mid);
                cr.stroke().ok();
                jewel(cr, right_start + jr, mid, jr, &ac);
            }
        }
        HeaderStyle::Banner => {
            // A tinted band closed by a double rule with a centre jewel.
            set_color_alpha(cr, &ac, 0.14);
            cr.rectangle(x, y, w, hh);
            cr.fill().ok();
            let rule_y = y + hh - 1.0;
            let g = metal_gradient(&ac, x, 0.0, x + w, 0.0);
            cr.set_source(&g).ok();
            cr.set_line_width(2.0);
            cr.move_to(x, rule_y);
            cr.line_to(x + w, rule_y);
            cr.stroke().ok();
            set_color_alpha(cr, &ac, 0.5);
            cr.set_line_width(1.0);
            cr.move_to(x, rule_y - 3.5);
            cr.line_to(x + w, rule_y - 3.5);
            cr.stroke().ok();
            jewel(cr, x + w / 2.0, rule_y - 1.0, (u * 0.6).max(3.0), &ac);
            // Chevron notches at the ends of the band.
            set_color_alpha(cr, &ac, 0.55);
            let n = u * 0.9;
            cr.move_to(x + clear, mid - n);
            cr.line_to(x + clear + n, mid);
            cr.line_to(x + clear, mid + n);
            cr.move_to(x + w - clear, mid - n);
            cr.line_to(x + w - clear - n, mid);
            cr.line_to(x + w - clear, mid + n);
            cr.set_line_width(1.5);
            cr.stroke().ok();
        }
        HeaderStyle::Stepped => {
            // A ziggurat rising behind the title: three tiers, widest at
            // the base, with a metal outline on the steps.
            let tiers = 3;
            let tier_h = hh / tiers as f64;
            let inset = ((w - tw) / 2.0 - u * 2.0).max(0.0) / tiers as f64;
            for i in 0..tiers {
                let ins = (tiers - 1 - i) as f64 * inset;
                set_color_alpha(cr, &ac, 0.08 + 0.06 * i as f64);
                cr.rectangle(x + ins, y + i as f64 * tier_h, w - ins * 2.0, tier_h);
                cr.fill().ok();
            }
            let g = metal_gradient(&ac, x, 0.0, x + w, 0.0);
            cr.set_source(&g).ok();
            cr.set_line_width(1.5);
            cr.move_to(x, y + hh);
            for i in 0..tiers {
                let ins = i as f64 * inset;
                let top = y + hh - (i + 1) as f64 * tier_h;
                cr.line_to(x + ins, top + tier_h);
                cr.line_to(x + ins, top);
            }
            let ins = tiers as f64 * inset;
            cr.line_to(x + ins, y);
            cr.line_to(x + w - ins, y);
            for i in (0..tiers).rev() {
                let ins = i as f64 * inset;
                let top = y + hh - (i + 1) as f64 * tier_h;
                cr.line_to(x + w - ins, top);
                cr.line_to(x + w - ins, top + tier_h);
            }
            cr.line_to(x + w, y + hh);
            cr.stroke().ok();
        }
    }

    // Title in polished metal.
    let g = metal_gradient(&hc, 0.0, ty, 0.0, ty + th);
    cr.set_source(&g).ok();
    cr.move_to(tx, ty);
    show_layout(cr, &layout);
    cr.restore().ok();
    hh
}

// ---------------------------------------------------------------------------
// Dividers
// ---------------------------------------------------------------------------

/// Space reserved between two groups; the motif must fit inside it.
fn divider_extent(config: &ArtDecoFrameConfig) -> f64 {
    config.divider_width + config.divider_padding * 2.0
}

/// Draw a divider along the centre line starting at (x, y): horizontal to
/// the right, or vertical downwards. Vertical dividers are the horizontal
/// motif rotated, so every motif is written once.
fn draw_divider(
    cr: &Context,
    config: &ArtDecoFrameConfig,
    x: f64,
    y: f64,
    length: f64,
    horizontal: bool,
) {
    if matches!(config.divider_style, DividerStyle::None) || length <= 0.0 {
        return;
    }
    cr.save().ok();
    cr.translate(x, y);
    if !horizontal {
        cr.rotate(FRAC_PI_2);
    }
    draw_divider_motif(cr, config, length);
    cr.restore().ok();
}

fn draw_divider_motif(cr: &Context, config: &ArtDecoFrameConfig, len: f64) {
    let c = config.divider_color.resolve(&config.theme);
    let lw = config.divider_width.max(1.0);
    // Band the motif may occupy, centred on y = 0.
    let hb = (divider_extent(config) - 2.0).max(4.0);
    let half = hb / 2.0;
    let mid = len / 2.0;
    let jr = (half * 0.9).max(3.0);

    let rule = |cr: &Context, x0: f64, x1: f64, yy: f64, alpha: f64| {
        if x1 <= x0 {
            return;
        }
        let g = fading_gradient(&c, x0, 0.0, x1, 0.0, alpha);
        cr.set_source(&g).ok();
        cr.set_line_width(lw);
        cr.move_to(x0, yy);
        cr.line_to(x1, yy);
        cr.stroke().ok();
    };
    // A rule that fades at the outer end and is solid at the centre.
    let rule_to_centre =
        |cr: &Context, x0: f64, x1: f64, yy: f64, alpha: f64, outer_is_x0: bool| {
            if x1 <= x0 {
                return;
            }
            let g = if outer_is_x0 {
                fade_in_gradient(&c, x0, 0.0, x1, 0.0, alpha)
            } else {
                fade_in_gradient(&c, x1, 0.0, x0, 0.0, alpha)
            };
            cr.set_source(&g).ok();
            cr.set_line_width(lw);
            cr.move_to(x0, yy);
            cr.line_to(x1, yy);
            cr.stroke().ok();
        };

    match config.divider_style {
        DividerStyle::None => {}
        DividerStyle::Line => rule(cr, 0.0, len, 0.0, 0.9),
        DividerStyle::DoubleLine => {
            let off = (half * 0.7).max(lw * 1.5);
            rule(cr, 0.0, len, -off, 0.85);
            rule(cr, 0.0, len, off, 0.85);
            jewel(cr, mid, 0.0, half.max(3.0), &c);
        }
        DividerStyle::Chevron => {
            let amp = (half * 0.8).max(1.5);
            let period = (hb * 1.6).max(10.0);
            let gap = jr * 1.6;
            set_color_alpha(cr, &c, 0.9);
            cr.set_line_width(lw);
            zigzag_path(cr, 0.0, mid - gap, 0.0, period, amp);
            zigzag_path(cr, mid + gap, len, 0.0, period, amp);
            cr.stroke().ok();
            jewel(cr, mid, 0.0, jr, &c);
        }
        DividerStyle::Stepped => {
            // Steps climbing towards the centre, a jewel on the summit.
            let n = 3;
            let gap = jr * 1.4;
            let seg = ((mid - gap) / n as f64).max(1.0);
            let rise = (hb * 1.2) / n as f64;
            set_color_alpha(cr, &c, 0.9);
            cr.set_line_width(lw);
            cr.new_sub_path();
            cr.move_to(0.0, half * 0.9);
            for i in 0..n {
                let yy = half * 0.9 - i as f64 * rise;
                cr.line_to((i + 1) as f64 * seg, yy);
                cr.line_to((i + 1) as f64 * seg, yy - rise);
            }
            cr.stroke().ok();
            cr.new_sub_path();
            cr.move_to(len, half * 0.9);
            for i in 0..n {
                let yy = half * 0.9 - i as f64 * rise;
                cr.line_to(len - (i + 1) as f64 * seg, yy);
                cr.line_to(len - (i + 1) as f64 * seg, yy - rise);
            }
            cr.stroke().ok();
            jewel(
                cr,
                mid,
                half * 0.9 - n as f64 * rise + jr * 0.3,
                jr * 0.9,
                &c,
            );
        }
        DividerStyle::DiamondCluster => {
            let sp = jr * 2.6;
            rule(cr, 0.0, mid - sp - jr, 0.0, 0.8);
            rule(cr, mid + sp + jr, len, 0.0, 0.8);
            jewel(cr, mid - sp, 0.0, jr * 0.6, &c);
            jewel(cr, mid, 0.0, jr, &c);
            jewel(cr, mid + sp, 0.0, jr * 0.6, &c);
        }
        DividerStyle::Crescent => {
            let r = jr;
            rule(cr, 0.0, mid - r * 3.0, 0.0, 0.8);
            rule(cr, mid + r * 3.0, len, 0.0, 0.8);
            cr.new_sub_path();
            cr.arc(mid, 0.0, r, 0.0, 2.0 * PI);
            let g = metal_gradient(&c, mid - r, 0.0, mid + r, 0.0);
            cr.set_source(&g).ok();
            cr.fill().ok();
            // Bite out of the disc makes the crescent.
            let bgc = config.background_color.resolve(&config.theme);
            set_color(cr, &bgc);
            cr.new_sub_path();
            cr.arc(mid + r * 0.45, -r * 0.1, r * 0.8, 0.0, 2.0 * PI);
            cr.fill().ok();
            set_color_alpha(cr, &c, 0.9);
            for dx in [-2.2, 2.2] {
                cr.new_sub_path();
                cr.arc(mid + r * dx, 0.0, lw, 0.0, 2.0 * PI);
                cr.fill().ok();
            }
        }
        DividerStyle::ArrowDiamond => {
            let gap = jr * 2.2;
            rule_to_centre(cr, 0.0, mid - gap, 0.0, 0.9, true);
            rule_to_centre(cr, mid + gap, len, 0.0, 0.9, false);
            set_color_alpha(cr, &c, 0.9);
            cr.set_line_width(lw);
            let a = half * 0.7;
            cr.move_to(mid - gap - a, -a);
            cr.line_to(mid - gap, 0.0);
            cr.line_to(mid - gap - a, a);
            cr.move_to(mid + gap + a, -a);
            cr.line_to(mid + gap, 0.0);
            cr.line_to(mid + gap + a, a);
            cr.stroke().ok();
            jewel(cr, mid, 0.0, jr, &c);
        }
        DividerStyle::CircleChain => {
            let r = jr * 0.7;
            let sp = r * 2.6;
            rule(cr, 0.0, mid - sp - r, 0.0, 0.8);
            rule(cr, mid + sp + r, len, 0.0, 0.8);
            set_color_alpha(cr, &c, 0.9);
            cr.set_line_width(lw);
            cr.move_to(mid - sp + r, 0.0);
            cr.line_to(mid - r * 1.3, 0.0);
            cr.move_to(mid + r * 1.3, 0.0);
            cr.line_to(mid + sp - r, 0.0);
            cr.stroke().ok();
            for (cx, rr) in [(mid - sp, r), (mid, r * 1.3), (mid + sp, r)] {
                cr.new_sub_path();
                cr.arc(cx, 0.0, rr, 0.0, 2.0 * PI);
                cr.stroke().ok();
                cr.new_sub_path();
                cr.arc(cx, 0.0, rr * 0.35, 0.0, 2.0 * PI);
                cr.fill().ok();
            }
        }
        DividerStyle::CrossedLines => {
            let reach = hb * 2.5;
            rule(cr, 0.0, mid - reach, 0.0, 0.8);
            rule(cr, mid + reach, len, 0.0, 0.8);
            set_color_alpha(cr, &c, 0.9);
            cr.set_line_width(lw);
            cr.move_to(mid - reach, -half);
            cr.line_to(mid + reach, half);
            cr.move_to(mid - reach, half);
            cr.line_to(mid + reach, -half);
            cr.stroke().ok();
            jewel(cr, mid, 0.0, jr * 0.8, &c);
        }
        DividerStyle::FleurDeLis => {
            let r = jr;
            rule(cr, 0.0, mid - r * 3.0, 0.0, 0.8);
            rule(cr, mid + r * 3.0, len, 0.0, 0.8);
            set_color_alpha(cr, &c, 0.9);
            cr.set_line_width(lw);
            // Central lobe and two curled side lobes.
            cr.new_sub_path();
            cr.arc(mid, 0.0, r, PI * 0.75, PI * 2.25);
            cr.stroke().ok();
            cr.new_sub_path();
            cr.arc(mid - r * 1.4, r * 0.1, r * 0.7, PI * 1.25, PI * 2.6);
            cr.stroke().ok();
            cr.new_sub_path();
            cr.arc_negative(mid + r * 1.4, r * 0.1, r * 0.7, -PI * 0.25, -PI * 1.6);
            cr.stroke().ok();
            jewel(cr, mid, r * 0.5, r * 0.45, &c);
        }
        DividerStyle::Heartbeat => {
            let reach = hb * 2.2;
            rule(cr, 0.0, mid - reach - jr, 0.0, 0.8);
            rule(cr, mid + reach + jr, len, 0.0, 0.8);
            set_color_alpha(cr, &c, 0.9);
            cr.set_line_width(lw);
            zigzag_path(cr, mid - reach, mid + reach, 0.0, reach, half * 0.8);
            cr.stroke().ok();
            jewel(cr, mid - reach - jr * 0.5, 0.0, jr * 0.6, &c);
            jewel(cr, mid + reach + jr * 0.5, 0.0, jr * 0.6, &c);
        }
        DividerStyle::DiamondGrid => {
            let r = jr * 0.65;
            let n = 5;
            let span = r * 2.0 * n as f64;
            rule(cr, 0.0, mid - span / 2.0 - r, 0.0, 0.8);
            rule(cr, mid + span / 2.0 + r, len, 0.0, 0.8);
            for i in 0..n {
                let cx = mid - span / 2.0 + r + i as f64 * r * 2.0;
                let rr = if i == n / 2 { jr } else { r };
                jewel(cr, cx, 0.0, rr, &c);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Public entry points
// ---------------------------------------------------------------------------

/// Render the Art Deco frame and return the content area `(x, y, w, h)`.
pub fn render_art_deco_frame(
    cr: &Context,
    config: &ArtDecoFrameConfig,
    width: f64,
    height: f64,
) -> Result<(f64, f64, f64, f64)> {
    if width < 1.0 || height < 1.0 {
        return Ok((0.0, 0.0, 0.0, 0.0));
    }

    cr.save()?;
    let u = unit(width, height);
    let margin = config.border_width + 4.0;
    let frame_x = margin;
    let frame_y = margin;
    let frame_w = (width - margin * 2.0).max(1.0);
    let frame_h = (height - margin * 2.0).max(1.0);

    draw_background(cr, config, frame_x, frame_y, frame_w, frame_h);
    draw_border(cr, config, frame_x, frame_y, frame_w, frame_h, u);
    draw_corner_decorations(cr, config, frame_x, frame_y, frame_w, frame_h, u);
    let header_height = draw_header(cr, config, frame_x, frame_y, frame_w, u);
    cr.restore()?;

    let content_x = frame_x + config.content_padding;
    let content_y = frame_y + header_height + config.content_padding;
    let content_w = frame_w - config.content_padding * 2.0;
    let content_h = frame_h - header_height - config.content_padding * 2.0;

    Ok((content_x, content_y, content_w, content_h))
}

/// Calculate group layouts within content area
pub fn calculate_group_layouts(
    config: &ArtDecoFrameConfig,
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
    let extent = divider_extent(config);
    let divider_space = divider_count as f64 * extent;

    match config.split_orientation {
        SplitOrientation::Vertical => {
            let available_height = content_h - divider_space;
            let mut current_y = content_y;

            for (i, weight) in weights.iter().enumerate() {
                let group_h = available_height * (weight / total_weight);
                layouts.push((content_x, current_y, content_w, group_h));
                current_y += group_h;

                if i < divider_count {
                    current_y += extent;
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
                    current_x += extent;
                }
            }
        }
    }

    layouts
}

/// Draw dividers between groups
pub fn draw_group_dividers(
    cr: &Context,
    config: &ArtDecoFrameConfig,
    group_layouts: &[(f64, f64, f64, f64)],
) {
    if group_layouts.len() < 2 {
        return;
    }
    let centre = divider_extent(config) / 2.0;

    for &(x1, y1, w1, h1) in group_layouts.iter().take(group_layouts.len() - 1) {
        match config.split_orientation {
            SplitOrientation::Vertical => {
                draw_divider(cr, config, x1, y1 + h1 + centre, w1, true);
            }
            SplitOrientation::Horizontal => {
                draw_divider(cr, config, x1 + w1 + centre, y1, h1, false);
            }
        }
    }
}
