//! Steampunk display rendering
//!
//! Provides a Victorian-era steampunk display with:
//! - Brass, copper, and bronze metallic color schemes
//! - Decorative gears and cogs
//! - Ornate rivets and bolts
//! - Victorian flourishes and filigree
//! - Steam pipe and gauge aesthetics
//! - Weathered patina textures

use std::f64::consts::PI;

use anyhow::Result;
use cairo::Context;

use crate::combo_traits::FrameRenderer;
use crate::background::Color;
use crate::pango_text::{pango_show_text, pango_text_extents};

pub use rg_sens_types::display_configs::steampunk::*;

pub struct SteampunkRenderer;

impl FrameRenderer for SteampunkRenderer {
    type Config = SteampunkFrameConfig;

    fn theme_id(&self) -> &'static str {
        "steampunk"
    }

    fn theme_name(&self) -> &'static str {
        "Steampunk"
    }

    fn default_config(&self) -> Self::Config {
        SteampunkFrameConfig::default()
    }

    fn render_frame(
        &self,
        cr: &Context,
        config: &Self::Config,
        width: f64,
        height: f64,
    ) -> anyhow::Result<(f64, f64, f64, f64)> {
        render_steampunk_frame(cr, config, width, height).map_err(|e| anyhow::anyhow!("{}", e))
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

/// Draw a decorative gear
fn draw_gear(
    cr: &Context,
    cx: f64,
    cy: f64,
    outer_radius: f64,
    inner_radius: f64,
    teeth: usize,
    color: &Color,
    highlight_color: &Color,
) {
    cr.save().ok();

    let _tooth_depth = (outer_radius - inner_radius) * 0.6;
    let mid_radius = inner_radius + (outer_radius - inner_radius) * 0.5;
    let angle_step = 2.0 * PI / teeth as f64;
    let half_tooth = angle_step * 0.3;

    // Draw gear body with teeth
    cr.new_path();
    for i in 0..teeth {
        let base_angle = i as f64 * angle_step;

        // Inner edge (valley)
        let a1 = base_angle - half_tooth;
        cr.line_to(cx + mid_radius * a1.cos(), cy + mid_radius * a1.sin());

        // Tooth rise
        let a2 = base_angle - half_tooth * 0.5;
        cr.line_to(cx + outer_radius * a2.cos(), cy + outer_radius * a2.sin());

        // Tooth top
        let a3 = base_angle + half_tooth * 0.5;
        cr.line_to(cx + outer_radius * a3.cos(), cy + outer_radius * a3.sin());

        // Tooth fall
        let a4 = base_angle + half_tooth;
        cr.line_to(cx + mid_radius * a4.cos(), cy + mid_radius * a4.sin());
    }
    cr.close_path();

    // Fill with gradient for metallic look
    let gradient = cairo::RadialGradient::new(
        cx - outer_radius * 0.3,
        cy - outer_radius * 0.3,
        0.0,
        cx,
        cy,
        outer_radius,
    );
    gradient.add_color_stop_rgba(
        0.0,
        highlight_color.r,
        highlight_color.g,
        highlight_color.b,
        highlight_color.a,
    );
    gradient.add_color_stop_rgba(0.5, color.r, color.g, color.b, color.a);
    gradient.add_color_stop_rgba(1.0, color.r * 0.6, color.g * 0.6, color.b * 0.6, color.a);
    cr.set_source(&gradient).ok();
    cr.fill_preserve().ok();

    // Outline
    cr.set_source_rgba(0.0, 0.0, 0.0, 0.5);
    cr.set_line_width(1.0);
    cr.stroke().ok();

    // Center hole
    let hole_radius = inner_radius * 0.4;
    cr.arc(cx, cy, hole_radius, 0.0, 2.0 * PI);
    cr.set_source_rgba(0.1, 0.08, 0.05, 1.0);
    cr.fill().ok();

    // Inner ring
    cr.arc(cx, cy, hole_radius + 2.0, 0.0, 2.0 * PI);
    cr.set_source_rgba(color.r * 0.8, color.g * 0.8, color.b * 0.8, color.a);
    cr.set_line_width(2.0);
    cr.stroke().ok();

    cr.restore().ok();
}

/// Draw a rivet/bolt
fn draw_rivet(cr: &Context, cx: f64, cy: f64, size: f64, color: &Color) {
    cr.save().ok();

    // Outer ring (shadow)
    cr.arc(cx + 1.0, cy + 1.0, size, 0.0, 2.0 * PI);
    cr.set_source_rgba(0.0, 0.0, 0.0, 0.3);
    cr.fill().ok();

    // Main rivet body
    let gradient = cairo::RadialGradient::new(cx - size * 0.3, cy - size * 0.3, 0.0, cx, cy, size);
    gradient.add_color_stop_rgba(0.0, color.r + 0.3, color.g + 0.25, color.b + 0.2, color.a);
    gradient.add_color_stop_rgba(0.5, color.r, color.g, color.b, color.a);
    gradient.add_color_stop_rgba(1.0, color.r * 0.6, color.g * 0.5, color.b * 0.4, color.a);

    cr.arc(cx, cy, size, 0.0, 2.0 * PI);
    cr.set_source(&gradient).ok();
    cr.fill().ok();

    // Slot (Phillips style for steampunk)
    cr.set_source_rgba(0.0, 0.0, 0.0, 0.5);
    cr.set_line_width(size * 0.25);
    let slot_size = size * 0.5;
    cr.move_to(cx - slot_size, cy);
    cr.line_to(cx + slot_size, cy);
    cr.stroke().ok();
    cr.move_to(cx, cy - slot_size);
    cr.line_to(cx, cy + slot_size);
    cr.stroke().ok();

    cr.restore().ok();
}

/// Draw a Victorian flourish
fn draw_flourish(
    cr: &Context,
    x: f64,
    y: f64,
    size: f64,
    flip_h: bool,
    flip_v: bool,
    color: &Color,
) {
    cr.save().ok();
    cr.translate(x, y);
    if flip_h {
        cr.scale(-1.0, 1.0);
    }
    if flip_v {
        cr.scale(1.0, -1.0);
    }

    cr.set_source_rgba(color.r, color.g, color.b, color.a);
    cr.set_line_width(2.0);

    // Main curl
    cr.new_path();
    cr.move_to(0.0, 0.0);
    cr.curve_to(
        size * 0.3,
        0.0,
        size * 0.5,
        size * 0.2,
        size * 0.6,
        size * 0.4,
    );
    cr.curve_to(
        size * 0.7,
        size * 0.6,
        size * 0.5,
        size * 0.7,
        size * 0.3,
        size * 0.6,
    );
    cr.curve_to(
        size * 0.1,
        size * 0.5,
        size * 0.15,
        size * 0.3,
        size * 0.25,
        size * 0.25,
    );
    cr.stroke().ok();

    // Secondary curl
    cr.move_to(size * 0.2, size * 0.1);
    cr.curve_to(
        size * 0.35,
        size * 0.15,
        size * 0.4,
        size * 0.3,
        size * 0.35,
        size * 0.4,
    );
    cr.stroke().ok();

    // Leaf accent
    cr.move_to(size * 0.5, size * 0.3);
    cr.curve_to(
        size * 0.7,
        size * 0.2,
        size * 0.8,
        size * 0.25,
        size * 0.85,
        size * 0.35,
    );
    cr.curve_to(
        size * 0.8,
        size * 0.4,
        size * 0.6,
        size * 0.45,
        size * 0.5,
        size * 0.4,
    );
    cr.fill().ok();

    cr.restore().ok();
}

/// Draw a pipe elbow/joint
fn draw_pipe_joint(cr: &Context, cx: f64, cy: f64, size: f64, color: &Color) {
    cr.save().ok();

    let pipe_width = size * 0.4;

    // Horizontal pipe segment
    let gradient_h = cairo::LinearGradient::new(
        cx - size,
        cy - pipe_width / 2.0,
        cx - size,
        cy + pipe_width / 2.0,
    );
    gradient_h.add_color_stop_rgba(0.0, color.r + 0.2, color.g + 0.15, color.b + 0.1, color.a);
    gradient_h.add_color_stop_rgba(0.5, color.r, color.g, color.b, color.a);
    gradient_h.add_color_stop_rgba(1.0, color.r * 0.6, color.g * 0.5, color.b * 0.4, color.a);

    cr.rectangle(cx - size, cy - pipe_width / 2.0, size, pipe_width);
    cr.set_source(&gradient_h).ok();
    cr.fill().ok();

    // Vertical pipe segment
    let gradient_v =
        cairo::LinearGradient::new(cx - pipe_width / 2.0, cy, cx + pipe_width / 2.0, cy);
    gradient_v.add_color_stop_rgba(0.0, color.r + 0.2, color.g + 0.15, color.b + 0.1, color.a);
    gradient_v.add_color_stop_rgba(0.5, color.r, color.g, color.b, color.a);
    gradient_v.add_color_stop_rgba(1.0, color.r * 0.6, color.g * 0.5, color.b * 0.4, color.a);

    cr.rectangle(cx - pipe_width / 2.0, cy, pipe_width, size);
    cr.set_source(&gradient_v).ok();
    cr.fill().ok();

    // Joint coupling (circular)
    let coupling_radius = pipe_width * 0.7;
    let gradient_c = cairo::RadialGradient::new(
        cx - coupling_radius * 0.3,
        cy - coupling_radius * 0.3,
        0.0,
        cx,
        cy,
        coupling_radius,
    );
    gradient_c.add_color_stop_rgba(0.0, color.r + 0.3, color.g + 0.25, color.b + 0.2, color.a);
    gradient_c.add_color_stop_rgba(0.7, color.r, color.g, color.b, color.a);
    gradient_c.add_color_stop_rgba(1.0, color.r * 0.5, color.g * 0.4, color.b * 0.3, color.a);

    cr.arc(cx, cy, coupling_radius, 0.0, 2.0 * PI);
    cr.set_source(&gradient_c).ok();
    cr.fill().ok();

    // Coupling ring
    cr.arc(cx, cy, coupling_radius, 0.0, 2.0 * PI);
    cr.set_source_rgba(0.0, 0.0, 0.0, 0.4);
    cr.set_line_width(1.5);
    cr.stroke().ok();

    cr.restore().ok();
}

/// Draw brushed brass texture
fn draw_brushed_brass(cr: &Context, x: f64, y: f64, w: f64, h: f64, color: &Color) {
    cr.save().ok();

    // Base gradient
    let gradient = cairo::LinearGradient::new(x, y, x, y + h);
    gradient.add_color_stop_rgba(0.0, color.r + 0.15, color.g + 0.12, color.b + 0.05, color.a);
    gradient.add_color_stop_rgba(0.3, color.r, color.g, color.b, color.a);
    gradient.add_color_stop_rgba(0.7, color.r * 0.9, color.g * 0.85, color.b * 0.75, color.a);
    gradient.add_color_stop_rgba(1.0, color.r * 0.8, color.g * 0.75, color.b * 0.6, color.a);

    cr.rectangle(x, y, w, h);
    cr.set_source(&gradient).ok();
    cr.fill().ok();

    // Horizontal brush strokes - batch into single stroke call for performance
    cr.set_line_width(0.5);
    cr.set_source_rgba(1.0, 0.95, 0.8, 0.05); // Use average alpha
    let stroke_spacing = 4.0; // Increase spacing for performance
    let mut stroke_y = y;
    while stroke_y < y + h {
        cr.move_to(x, stroke_y);
        cr.line_to(x + w, stroke_y);
        stroke_y += stroke_spacing;
    }
    cr.stroke().ok();

    cr.restore().ok();
}

/// Draw weathered patina texture
fn draw_patina_texture(
    cr: &Context,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    base_color: &Color,
    patina_color: &Color,
    intensity: f64,
) {
    cr.save().ok();

    // Clip to bounds once at the start (not per-spot)
    cr.rectangle(x, y, w, h);
    cr.clip();

    // Base copper color
    cr.rectangle(x, y, w, h);
    cr.set_source_rgba(base_color.r, base_color.g, base_color.b, base_color.a);
    cr.fill().ok();

    // Random patina spots - cap at 50 spots maximum for performance
    let spot_count = (((w * h) / 800.0) as usize).min(50);
    for i in 0..spot_count {
        let seed = (i as f64 * 7.3 + x * 0.1 + y * 0.13) % 1.0;
        let px = x + (seed * w * 3.7) % w;
        let py = y + ((seed * 2.3).sin().abs() * h);
        let radius = 8.0 + seed * 20.0;

        let gradient = cairo::RadialGradient::new(px, py, 0.0, px, py, radius);
        gradient.add_color_stop_rgba(
            0.0,
            patina_color.r,
            patina_color.g,
            patina_color.b,
            patina_color.a * intensity,
        );
        gradient.add_color_stop_rgba(1.0, patina_color.r, patina_color.g, patina_color.b, 0.0);

        // Draw spot without per-spot clipping (already clipped to bounds)
        cr.set_source(&gradient).ok();
        cr.paint().ok();
    }

    cr.restore().ok();
}

/// Draw leather texture with stitching
fn draw_leather_texture(cr: &Context, x: f64, y: f64, w: f64, h: f64, color: &Color) {
    cr.save().ok();

    // Base leather color with subtle gradient
    let gradient = cairo::LinearGradient::new(x, y, x + w * 0.7, y + h * 0.7);
    gradient.add_color_stop_rgba(0.0, color.r * 1.1, color.g * 1.05, color.b, color.a);
    gradient.add_color_stop_rgba(0.5, color.r, color.g, color.b, color.a);
    gradient.add_color_stop_rgba(1.0, color.r * 0.85, color.g * 0.8, color.b * 0.75, color.a);

    cr.rectangle(x, y, w, h);
    cr.set_source(&gradient).ok();
    cr.fill().ok();

    // Grain texture (subtle noise pattern) - cap at 200 dots for performance
    // Batch all grain lines into a single path per alpha level for efficiency
    cr.set_line_width(0.5);
    let grain_count = ((w * h / 200.0) as usize).min(200);

    // Use a single average alpha and batch all grain lines
    cr.set_source_rgba(0.0, 0.0, 0.0, 0.07);
    for i in 0..grain_count {
        let seed = i as f64 * 3.7;
        let px = x + (seed * 1.7) % w;
        let py = y + (seed * 2.3) % h;
        cr.move_to(px, py);
        cr.line_to(px + 1.0, py + 1.0);
    }
    cr.stroke().ok();

    // Stitching along edges
    let stitch_spacing = 8.0;
    let stitch_length = 4.0;
    let margin = 10.0;

    cr.set_source_rgba(0.9, 0.85, 0.7, 0.8);
    cr.set_line_width(1.0);

    // Top edge stitching
    let mut sx = x + margin;
    while sx < x + w - margin {
        cr.move_to(sx, y + margin);
        cr.line_to(sx + stitch_length, y + margin);
        sx += stitch_spacing;
    }

    // Bottom edge stitching
    sx = x + margin;
    while sx < x + w - margin {
        cr.move_to(sx, y + h - margin);
        cr.line_to(sx + stitch_length, y + h - margin);
        sx += stitch_spacing;
    }

    cr.stroke().ok();

    cr.restore().ok();
}

/// Draw metal plate texture with rivets
fn draw_metal_plate_texture(
    cr: &Context,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    color: &Color,
    rivet_color: &Color,
    rivet_spacing: f64,
) {
    cr.save().ok();

    // Base metal with beveled edge effect
    let gradient = cairo::LinearGradient::new(x, y, x, y + h);
    gradient.add_color_stop_rgba(0.0, color.r + 0.15, color.g + 0.12, color.b + 0.1, color.a);
    gradient.add_color_stop_rgba(0.05, color.r, color.g, color.b, color.a);
    gradient.add_color_stop_rgba(0.95, color.r * 0.9, color.g * 0.88, color.b * 0.85, color.a);
    gradient.add_color_stop_rgba(1.0, color.r * 0.7, color.g * 0.68, color.b * 0.65, color.a);

    cr.rectangle(x, y, w, h);
    cr.set_source(&gradient).ok();
    cr.fill().ok();

    // Draw rivets along edges
    let margin = 12.0;
    let rivet_size = 4.0;

    // Top edge
    let mut rx = x + margin;
    while rx < x + w - margin {
        draw_rivet(cr, rx, y + margin, rivet_size, rivet_color);
        rx += rivet_spacing;
    }

    // Bottom edge
    rx = x + margin;
    while rx < x + w - margin {
        draw_rivet(cr, rx, y + h - margin, rivet_size, rivet_color);
        rx += rivet_spacing;
    }

    // Left edge
    let mut ry = y + margin + rivet_spacing;
    while ry < y + h - margin - rivet_spacing {
        draw_rivet(cr, x + margin, ry, rivet_size, rivet_color);
        ry += rivet_spacing;
    }

    // Right edge
    ry = y + margin + rivet_spacing;
    while ry < y + h - margin - rivet_spacing {
        draw_rivet(cr, x + w - margin, ry, rivet_size, rivet_color);
        ry += rivet_spacing;
    }

    cr.restore().ok();
}

/// Draw corner decorations based on style
fn draw_corner_decorations(
    cr: &Context,
    config: &SteampunkFrameConfig,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) {
    let accent_color = config.accent_color.resolve(&config.theme);
    let highlight_color = Color::new(
        (accent_color.r + 0.3).min(1.0),
        (accent_color.g + 0.25).min(1.0),
        (accent_color.b + 0.15).min(1.0),
        accent_color.a,
    );
    let size = config.corner_size;

    match config.corner_style {
        CornerStyle::Gear => {
            let outer_r = size / 2.0;
            let inner_r = size / 3.0;
            draw_gear(
                cr,
                x + size / 2.0,
                y + size / 2.0,
                outer_r,
                inner_r,
                config.gear_teeth,
                &accent_color,
                &highlight_color,
            );
            draw_gear(
                cr,
                x + w - size / 2.0,
                y + size / 2.0,
                outer_r,
                inner_r,
                config.gear_teeth,
                &accent_color,
                &highlight_color,
            );
            draw_gear(
                cr,
                x + w - size / 2.0,
                y + h - size / 2.0,
                outer_r,
                inner_r,
                config.gear_teeth,
                &accent_color,
                &highlight_color,
            );
            draw_gear(
                cr,
                x + size / 2.0,
                y + h - size / 2.0,
                outer_r,
                inner_r,
                config.gear_teeth,
                &accent_color,
                &highlight_color,
            );
        }
        CornerStyle::Flourish => {
            draw_flourish(
                cr,
                x + 4.0,
                y + 4.0,
                size * 0.8,
                false,
                false,
                &accent_color,
            );
            draw_flourish(
                cr,
                x + w - 4.0,
                y + 4.0,
                size * 0.8,
                true,
                false,
                &accent_color,
            );
            draw_flourish(
                cr,
                x + w - 4.0,
                y + h - 4.0,
                size * 0.8,
                true,
                true,
                &accent_color,
            );
            draw_flourish(
                cr,
                x + 4.0,
                y + h - 4.0,
                size * 0.8,
                false,
                true,
                &accent_color,
            );
        }
        CornerStyle::Rivet => {
            let rivet_color = config.rivet_color.resolve(&config.theme);
            let rivet_r = size / 3.0;
            draw_rivet(cr, x + size / 2.0, y + size / 2.0, rivet_r, &rivet_color);
            draw_rivet(
                cr,
                x + w - size / 2.0,
                y + size / 2.0,
                rivet_r,
                &rivet_color,
            );
            draw_rivet(
                cr,
                x + w - size / 2.0,
                y + h - size / 2.0,
                rivet_r,
                &rivet_color,
            );
            draw_rivet(
                cr,
                x + size / 2.0,
                y + h - size / 2.0,
                rivet_r,
                &rivet_color,
            );
        }
        CornerStyle::PipeJoint => {
            draw_pipe_joint(
                cr,
                x + size * 0.6,
                y + size * 0.6,
                size * 0.5,
                &accent_color,
            );
            cr.save().ok();
            cr.translate(x + w, y);
            cr.scale(-1.0, 1.0);
            draw_pipe_joint(cr, size * 0.6, size * 0.6, size * 0.5, &accent_color);
            cr.restore().ok();
            cr.save().ok();
            cr.translate(x + w, y + h);
            cr.scale(-1.0, -1.0);
            draw_pipe_joint(cr, size * 0.6, size * 0.6, size * 0.5, &accent_color);
            cr.restore().ok();
            cr.save().ok();
            cr.translate(x, y + h);
            cr.scale(1.0, -1.0);
            draw_pipe_joint(cr, size * 0.6, size * 0.6, size * 0.5, &accent_color);
            cr.restore().ok();
        }
        CornerStyle::None => {}
    }
}

/// Draw border based on style
fn draw_border(cr: &Context, config: &SteampunkFrameConfig, x: f64, y: f64, w: f64, h: f64) {
    let border_color = config.border_color.resolve(&config.theme);
    let accent_color = config.accent_color.resolve(&config.theme);

    cr.save().ok();

    match config.border_style {
        BorderStyle::Victorian => {
            // Ornate double border with flourishes
            cr.set_source_rgba(
                border_color.r,
                border_color.g,
                border_color.b,
                border_color.a,
            );
            cr.set_line_width(config.border_width);
            cr.rectangle(x, y, w, h);
            cr.stroke().ok();

            // Inner decorative line
            cr.set_line_width(config.border_width / 2.0);
            cr.rectangle(x + 6.0, y + 6.0, w - 12.0, h - 12.0);
            cr.stroke().ok();

            // Small decorative dots at midpoints
            let dot_size = 3.0;
            cr.set_source_rgba(
                accent_color.r,
                accent_color.g,
                accent_color.b,
                accent_color.a,
            );
            cr.arc(x + w / 2.0, y, dot_size, 0.0, 2.0 * PI);
            cr.fill().ok();
            cr.arc(x + w / 2.0, y + h, dot_size, 0.0, 2.0 * PI);
            cr.fill().ok();
            cr.arc(x, y + h / 2.0, dot_size, 0.0, 2.0 * PI);
            cr.fill().ok();
            cr.arc(x + w, y + h / 2.0, dot_size, 0.0, 2.0 * PI);
            cr.fill().ok();
        }
        BorderStyle::PipeFrame => {
            // Draw pipe-like border with 3D effect
            let pipe_width = config.border_width * 2.0;

            // Top pipe
            let gradient_t = cairo::LinearGradient::new(x, y, x, y + pipe_width);
            gradient_t.add_color_stop_rgba(
                0.0,
                border_color.r + 0.2,
                border_color.g + 0.15,
                border_color.b + 0.1,
                border_color.a,
            );
            gradient_t.add_color_stop_rgba(
                0.5,
                border_color.r,
                border_color.g,
                border_color.b,
                border_color.a,
            );
            gradient_t.add_color_stop_rgba(
                1.0,
                border_color.r * 0.6,
                border_color.g * 0.5,
                border_color.b * 0.4,
                border_color.a,
            );
            cr.rectangle(x, y, w, pipe_width);
            cr.set_source(&gradient_t).ok();
            cr.fill().ok();

            // Bottom pipe
            cr.rectangle(x, y + h - pipe_width, w, pipe_width);
            cr.set_source(&gradient_t).ok();
            cr.fill().ok();

            // Left pipe
            let gradient_l = cairo::LinearGradient::new(x, y, x + pipe_width, y);
            gradient_l.add_color_stop_rgba(
                0.0,
                border_color.r + 0.2,
                border_color.g + 0.15,
                border_color.b + 0.1,
                border_color.a,
            );
            gradient_l.add_color_stop_rgba(
                0.5,
                border_color.r,
                border_color.g,
                border_color.b,
                border_color.a,
            );
            gradient_l.add_color_stop_rgba(
                1.0,
                border_color.r * 0.6,
                border_color.g * 0.5,
                border_color.b * 0.4,
                border_color.a,
            );
            cr.rectangle(x, y + pipe_width, pipe_width, h - pipe_width * 2.0);
            cr.set_source(&gradient_l).ok();
            cr.fill().ok();

            // Right pipe
            cr.rectangle(
                x + w - pipe_width,
                y + pipe_width,
                pipe_width,
                h - pipe_width * 2.0,
            );
            cr.set_source(&gradient_l).ok();
            cr.fill().ok();
        }
        BorderStyle::Riveted => {
            // Simple border with rivets
            cr.set_source_rgba(
                border_color.r,
                border_color.g,
                border_color.b,
                border_color.a,
            );
            cr.set_line_width(config.border_width);
            cr.rectangle(x, y, w, h);
            cr.stroke().ok();

            // Draw rivets along border if enabled
            if config.show_rivets {
                let rivet_color = config.rivet_color.resolve(&config.theme);
                let margin = config.border_width + config.rivet_size;

                // Top and bottom
                let mut rx = x + margin;
                while rx < x + w - margin {
                    draw_rivet(cr, rx, y + margin, config.rivet_size, &rivet_color);
                    draw_rivet(cr, rx, y + h - margin, config.rivet_size, &rivet_color);
                    rx += config.rivet_spacing;
                }

                // Left and right (skip corners)
                let mut ry = y + margin + config.rivet_spacing;
                while ry < y + h - margin - config.rivet_spacing {
                    draw_rivet(cr, x + margin, ry, config.rivet_size, &rivet_color);
                    draw_rivet(cr, x + w - margin, ry, config.rivet_size, &rivet_color);
                    ry += config.rivet_spacing;
                }
            }
        }
        BorderStyle::Brass => {
            // Clean brass border with beveled effect
            let bevel = 3.0;

            // Outer highlight
            cr.set_source_rgba(
                border_color.r + 0.2,
                border_color.g + 0.15,
                border_color.b + 0.1,
                border_color.a,
            );
            cr.set_line_width(config.border_width);
            cr.move_to(x, y + h);
            cr.line_to(x, y);
            cr.line_to(x + w, y);
            cr.stroke().ok();

            // Outer shadow
            cr.set_source_rgba(
                border_color.r * 0.6,
                border_color.g * 0.5,
                border_color.b * 0.4,
                border_color.a,
            );
            cr.move_to(x + w, y);
            cr.line_to(x + w, y + h);
            cr.line_to(x, y + h);
            cr.stroke().ok();

            // Inner line
            cr.set_source_rgba(
                border_color.r,
                border_color.g,
                border_color.b,
                border_color.a,
            );
            cr.set_line_width(config.border_width / 2.0);
            cr.rectangle(x + bevel, y + bevel, w - bevel * 2.0, h - bevel * 2.0);
            cr.stroke().ok();
        }
        BorderStyle::GearBorder => {
            // Border with small gears at intervals
            cr.set_source_rgba(
                border_color.r,
                border_color.g,
                border_color.b,
                border_color.a,
            );
            cr.set_line_width(config.border_width);
            cr.rectangle(x, y, w, h);
            cr.stroke().ok();

            // Draw small gears at intervals
            let gear_spacing = 60.0;
            let gear_size = 8.0;
            let highlight = Color::new(
                (border_color.r + 0.2).min(1.0),
                (border_color.g + 0.15).min(1.0),
                (border_color.b + 0.1).min(1.0),
                border_color.a,
            );

            // Top and bottom edges
            let mut gx = x + gear_spacing;
            while gx < x + w - gear_spacing {
                draw_gear(
                    cr,
                    gx,
                    y,
                    gear_size,
                    gear_size * 0.6,
                    8,
                    &accent_color,
                    &highlight,
                );
                draw_gear(
                    cr,
                    gx,
                    y + h,
                    gear_size,
                    gear_size * 0.6,
                    8,
                    &accent_color,
                    &highlight,
                );
                gx += gear_spacing;
            }
        }
    }

    cr.restore().ok();
}

/// Draw background texture
fn draw_background_texture(
    cr: &Context,
    config: &SteampunkFrameConfig,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) {
    let bg_color = config.background_color.resolve(&config.theme);
    let patina_color = config.patina_color.resolve(&config.theme);
    let rivet_color = config.rivet_color.resolve(&config.theme);

    match config.background_texture {
        BackgroundTexture::BrushedBrass => {
            draw_brushed_brass(cr, x, y, w, h, &bg_color);
        }
        BackgroundTexture::Patina => {
            draw_patina_texture(
                cr,
                x,
                y,
                w,
                h,
                &bg_color,
                &patina_color,
                config.patina_intensity,
            );
        }
        BackgroundTexture::Leather => {
            draw_leather_texture(cr, x, y, w, h, &bg_color);
        }
        BackgroundTexture::MetalPlate => {
            draw_metal_plate_texture(
                cr,
                x,
                y,
                w,
                h,
                &bg_color,
                &rivet_color,
                config.rivet_spacing,
            );
        }
        BackgroundTexture::Solid => {
            cr.rectangle(x, y, w, h);
            cr.set_source_rgba(bg_color.r, bg_color.g, bg_color.b, bg_color.a);
            cr.fill().ok();
        }
    }
}

/// Draw the header
fn draw_header(cr: &Context, config: &SteampunkFrameConfig, x: f64, y: f64, w: f64) -> f64 {
    if !config.show_header || config.header_text.is_empty() {
        return 0.0;
    }

    let (font_family, font_size) = config.header_font.resolve(&config.theme);
    let header_color = config.header_color.resolve(&config.theme);
    let accent_color = config.accent_color.resolve(&config.theme);
    let rivet_color = config.rivet_color.resolve(&config.theme);

    let header_height = font_size + 24.0;
    let padding = 12.0;

    cr.save().ok();

    let text_extents = pango_text_extents(
        cr,
        &config.header_text,
        &font_family,
        cairo::FontSlant::Normal,
        cairo::FontWeight::Bold,
        font_size,
    );
    let (text_width, text_height) = (text_extents.width(), text_extents.height());
    let text_x = x + (w - text_width) / 2.0;
    let text_y = y + header_height / 2.0 + text_height / 2.0;

    match config.header_style {
        HeaderStyle::Nameplate => {
            // Brass nameplate with beveled edges
            let plate_w = text_width + 40.0;
            let plate_h = header_height - 8.0;
            let plate_x = x + (w - plate_w) / 2.0;
            let plate_y = y + 4.0;

            // Plate background with gradient
            let gradient = cairo::LinearGradient::new(plate_x, plate_y, plate_x, plate_y + plate_h);
            gradient.add_color_stop_rgba(
                0.0,
                accent_color.r + 0.2,
                accent_color.g + 0.15,
                accent_color.b + 0.1,
                accent_color.a,
            );
            gradient.add_color_stop_rgba(
                0.3,
                accent_color.r,
                accent_color.g,
                accent_color.b,
                accent_color.a,
            );
            gradient.add_color_stop_rgba(
                0.7,
                accent_color.r * 0.9,
                accent_color.g * 0.85,
                accent_color.b * 0.75,
                accent_color.a,
            );
            gradient.add_color_stop_rgba(
                1.0,
                accent_color.r * 0.7,
                accent_color.g * 0.65,
                accent_color.b * 0.55,
                accent_color.a,
            );

            cr.rectangle(plate_x, plate_y, plate_w, plate_h);
            cr.set_source(&gradient).ok();
            cr.fill().ok();

            // Beveled border
            cr.set_line_width(1.5);
            cr.set_source_rgba(1.0, 0.95, 0.85, 0.5);
            cr.move_to(plate_x, plate_y + plate_h);
            cr.line_to(plate_x, plate_y);
            cr.line_to(plate_x + plate_w, plate_y);
            cr.stroke().ok();

            cr.set_source_rgba(0.0, 0.0, 0.0, 0.4);
            cr.move_to(plate_x + plate_w, plate_y);
            cr.line_to(plate_x + plate_w, plate_y + plate_h);
            cr.line_to(plate_x, plate_y + plate_h);
            cr.stroke().ok();

            // Corner rivets
            let rivet_margin = 8.0;
            draw_rivet(
                cr,
                plate_x + rivet_margin,
                plate_y + plate_h / 2.0,
                4.0,
                &rivet_color,
            );
            draw_rivet(
                cr,
                plate_x + plate_w - rivet_margin,
                plate_y + plate_h / 2.0,
                4.0,
                &rivet_color,
            );
        }
        HeaderStyle::Banner => {
            // Full-width Victorian banner
            let gradient = cairo::LinearGradient::new(x, y, x, y + header_height);
            gradient.add_color_stop_rgba(
                0.0,
                accent_color.r * 0.3,
                accent_color.g * 0.25,
                accent_color.b * 0.2,
                0.8,
            );
            gradient.add_color_stop_rgba(
                0.5,
                accent_color.r * 0.2,
                accent_color.g * 0.15,
                accent_color.b * 0.1,
                0.6,
            );
            gradient.add_color_stop_rgba(
                1.0,
                accent_color.r * 0.15,
                accent_color.g * 0.1,
                accent_color.b * 0.08,
                0.4,
            );

            cr.rectangle(x + padding, y, w - padding * 2.0, header_height);
            cr.set_source(&gradient).ok();
            cr.fill().ok();

            // Decorative line below
            cr.set_source_rgba(
                accent_color.r,
                accent_color.g,
                accent_color.b,
                accent_color.a,
            );
            cr.set_line_width(2.0);
            cr.move_to(x + padding, y + header_height);
            cr.line_to(x + w - padding, y + header_height);
            cr.stroke().ok();

            // Small flourishes at ends
            draw_flourish(
                cr,
                x + padding + 10.0,
                y + header_height - 8.0,
                16.0,
                false,
                true,
                &accent_color,
            );
            draw_flourish(
                cr,
                x + w - padding - 10.0,
                y + header_height - 8.0,
                16.0,
                true,
                true,
                &accent_color,
            );
        }
        HeaderStyle::Industrial => {
            // Industrial label plate style
            let plate_w = text_width + 30.0;
            let plate_h = header_height - 10.0;
            let plate_x = x + (w - plate_w) / 2.0;
            let plate_y = y + 5.0;

            // Dark background
            cr.rectangle(plate_x, plate_y, plate_w, plate_h);
            cr.set_source_rgba(0.15, 0.12, 0.1, 1.0);
            cr.fill().ok();

            // Border
            cr.set_source_rgba(
                accent_color.r,
                accent_color.g,
                accent_color.b,
                accent_color.a,
            );
            cr.set_line_width(2.0);
            cr.rectangle(plate_x, plate_y, plate_w, plate_h);
            cr.stroke().ok();
        }
        HeaderStyle::None => {}
    }

    // Draw header text with emboss effect
    // Shadow
    cr.set_source_rgba(0.0, 0.0, 0.0, 0.5);
    cr.move_to(text_x + 1.0, text_y + 1.0);
    pango_show_text(
        cr,
        &config.header_text,
        &font_family,
        cairo::FontSlant::Normal,
        cairo::FontWeight::Bold,
        font_size,
    );

    // Main text
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
    config: &SteampunkFrameConfig,
    x: f64,
    y: f64,
    length: f64,
    horizontal: bool,
) {
    if matches!(config.divider_style, DividerStyle::None) {
        return;
    }

    let divider_color = config.divider_color.resolve(&config.theme);
    let accent_color = config.accent_color.resolve(&config.theme);

    cr.save().ok();

    match config.divider_style {
        DividerStyle::Pipe => {
            let pipe_width = config.divider_width;

            if horizontal {
                // Horizontal pipe
                let gradient =
                    cairo::LinearGradient::new(x, y - pipe_width / 2.0, x, y + pipe_width / 2.0);
                gradient.add_color_stop_rgba(
                    0.0,
                    divider_color.r + 0.2,
                    divider_color.g + 0.15,
                    divider_color.b + 0.1,
                    divider_color.a,
                );
                gradient.add_color_stop_rgba(
                    0.5,
                    divider_color.r,
                    divider_color.g,
                    divider_color.b,
                    divider_color.a,
                );
                gradient.add_color_stop_rgba(
                    1.0,
                    divider_color.r * 0.6,
                    divider_color.g * 0.5,
                    divider_color.b * 0.4,
                    divider_color.a,
                );

                cr.rectangle(x, y - pipe_width / 2.0, length, pipe_width);
                cr.set_source(&gradient).ok();
                cr.fill().ok();

                // Pressure gauge in center
                let gauge_radius = pipe_width * 1.2;
                let gauge_cx = x + length / 2.0;
                let gauge_gradient = cairo::RadialGradient::new(
                    gauge_cx - gauge_radius * 0.3,
                    y - gauge_radius * 0.3,
                    0.0,
                    gauge_cx,
                    y,
                    gauge_radius,
                );
                gauge_gradient.add_color_stop_rgba(
                    0.0,
                    accent_color.r + 0.3,
                    accent_color.g + 0.25,
                    accent_color.b + 0.15,
                    accent_color.a,
                );
                gauge_gradient.add_color_stop_rgba(
                    0.7,
                    accent_color.r,
                    accent_color.g,
                    accent_color.b,
                    accent_color.a,
                );
                gauge_gradient.add_color_stop_rgba(
                    1.0,
                    accent_color.r * 0.5,
                    accent_color.g * 0.4,
                    accent_color.b * 0.3,
                    accent_color.a,
                );

                cr.arc(gauge_cx, y, gauge_radius, 0.0, 2.0 * PI);
                cr.set_source(&gauge_gradient).ok();
                cr.fill().ok();

                // Gauge glass
                cr.arc(gauge_cx, y, gauge_radius * 0.7, 0.0, 2.0 * PI);
                cr.set_source_rgba(0.1, 0.1, 0.1, 0.9);
                cr.fill().ok();

                // Gauge needle
                cr.set_source_rgba(1.0, 0.3, 0.2, 1.0);
                cr.set_line_width(1.5);
                let needle_angle: f64 = -0.3; // Slightly above center
                cr.move_to(gauge_cx, y);
                cr.line_to(
                    gauge_cx + gauge_radius * 0.5 * needle_angle.cos(),
                    y + gauge_radius * 0.5 * needle_angle.sin(),
                );
                cr.stroke().ok();
            } else {
                // Vertical pipe
                let gradient =
                    cairo::LinearGradient::new(x - pipe_width / 2.0, y, x + pipe_width / 2.0, y);
                gradient.add_color_stop_rgba(
                    0.0,
                    divider_color.r + 0.2,
                    divider_color.g + 0.15,
                    divider_color.b + 0.1,
                    divider_color.a,
                );
                gradient.add_color_stop_rgba(
                    0.5,
                    divider_color.r,
                    divider_color.g,
                    divider_color.b,
                    divider_color.a,
                );
                gradient.add_color_stop_rgba(
                    1.0,
                    divider_color.r * 0.6,
                    divider_color.g * 0.5,
                    divider_color.b * 0.4,
                    divider_color.a,
                );

                cr.rectangle(x - pipe_width / 2.0, y, pipe_width, length);
                cr.set_source(&gradient).ok();
                cr.fill().ok();

                // Pressure gauge in center
                let gauge_radius = pipe_width * 1.2;
                let gauge_cy = y + length / 2.0;
                let gauge_gradient = cairo::RadialGradient::new(
                    x - gauge_radius * 0.3,
                    gauge_cy - gauge_radius * 0.3,
                    0.0,
                    x,
                    gauge_cy,
                    gauge_radius,
                );
                gauge_gradient.add_color_stop_rgba(
                    0.0,
                    accent_color.r + 0.3,
                    accent_color.g + 0.25,
                    accent_color.b + 0.15,
                    accent_color.a,
                );
                gauge_gradient.add_color_stop_rgba(
                    0.7,
                    accent_color.r,
                    accent_color.g,
                    accent_color.b,
                    accent_color.a,
                );
                gauge_gradient.add_color_stop_rgba(
                    1.0,
                    accent_color.r * 0.5,
                    accent_color.g * 0.4,
                    accent_color.b * 0.3,
                    accent_color.a,
                );

                cr.arc(x, gauge_cy, gauge_radius, 0.0, 2.0 * PI);
                cr.set_source(&gauge_gradient).ok();
                cr.fill().ok();

                cr.arc(x, gauge_cy, gauge_radius * 0.7, 0.0, 2.0 * PI);
                cr.set_source_rgba(0.1, 0.1, 0.1, 0.9);
                cr.fill().ok();
            }
        }
        DividerStyle::GearChain => {
            let gear_size = config.divider_width;
            let gear_spacing = gear_size * 2.5;
            let highlight = Color::new(
                (divider_color.r + 0.2).min(1.0),
                (divider_color.g + 0.15).min(1.0),
                (divider_color.b + 0.1).min(1.0),
                divider_color.a,
            );

            if horizontal {
                let mut gx = x + gear_spacing / 2.0;
                while gx < x + length - gear_spacing / 2.0 {
                    draw_gear(
                        cr,
                        gx,
                        y,
                        gear_size,
                        gear_size * 0.6,
                        8,
                        &divider_color,
                        &highlight,
                    );
                    gx += gear_spacing;
                }
            } else {
                let mut gy = y + gear_spacing / 2.0;
                while gy < y + length - gear_spacing / 2.0 {
                    draw_gear(
                        cr,
                        x,
                        gy,
                        gear_size,
                        gear_size * 0.6,
                        8,
                        &divider_color,
                        &highlight,
                    );
                    gy += gear_spacing;
                }
            }
        }
        DividerStyle::RivetedBar => {
            let bar_thickness = config.divider_width * 0.6;
            let rivet_color = config.rivet_color.resolve(&config.theme);

            cr.set_source_rgba(
                divider_color.r,
                divider_color.g,
                divider_color.b,
                divider_color.a,
            );

            if horizontal {
                cr.rectangle(x, y - bar_thickness / 2.0, length, bar_thickness);
                cr.fill().ok();

                // Rivets along bar
                let mut rx = x + 15.0;
                while rx < x + length - 15.0 {
                    draw_rivet(cr, rx, y, 3.0, &rivet_color);
                    rx += 20.0;
                }
            } else {
                cr.rectangle(x - bar_thickness / 2.0, y, bar_thickness, length);
                cr.fill().ok();

                let mut ry = y + 15.0;
                while ry < y + length - 15.0 {
                    draw_rivet(cr, x, ry, 3.0, &rivet_color);
                    ry += 20.0;
                }
            }
        }
        DividerStyle::Ornament => {
            cr.set_source_rgba(
                divider_color.r,
                divider_color.g,
                divider_color.b,
                divider_color.a,
            );
            cr.set_line_width(1.5);

            if horizontal {
                // Center line
                cr.move_to(x, y);
                cr.line_to(x + length, y);
                cr.stroke().ok();

                // Center ornament
                let cx = x + length / 2.0;
                draw_flourish(cr, cx - 12.0, y - 6.0, 12.0, false, false, &divider_color);
                draw_flourish(cr, cx + 12.0, y - 6.0, 12.0, true, false, &divider_color);
            } else {
                cr.move_to(x, y);
                cr.line_to(x, y + length);
                cr.stroke().ok();

                let cy = y + length / 2.0;
                draw_flourish(cr, x - 6.0, cy - 12.0, 12.0, false, false, &divider_color);
                draw_flourish(cr, x - 6.0, cy + 12.0, 12.0, false, true, &divider_color);
            }
        }
        DividerStyle::None => {}
    }

    cr.restore().ok();
}

/// Render the complete Steampunk frame
/// Returns the content area bounds (x, y, width, height)
pub fn render_steampunk_frame(
    cr: &Context,
    config: &SteampunkFrameConfig,
    width: f64,
    height: f64,
) -> Result<(f64, f64, f64, f64)> {
    if width < 1.0 || height < 1.0 {
        return Ok((0.0, 0.0, 0.0, 0.0));
    }

    cr.save()?;

    let margin = config.border_width + 4.0;
    let frame_x = margin;
    let frame_y = margin;
    let frame_w = (width - margin * 2.0).max(1.0);
    let frame_h = (height - margin * 2.0).max(1.0);

    // Draw background texture
    draw_background_texture(cr, config, frame_x, frame_y, frame_w, frame_h);

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
    config: &SteampunkFrameConfig,
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
    config: &SteampunkFrameConfig,
    group_layouts: &[(f64, f64, f64, f64)],
) {
    if group_layouts.len() < 2 {
        return;
    }

    for &(x1, y1, w1, h1) in group_layouts.iter().take(group_layouts.len() - 1) {
        match config.split_orientation {
            SplitOrientation::Vertical => {
                let divider_y = y1 + h1 + config.divider_padding + config.divider_width / 2.0;
                draw_divider(cr, config, x1, divider_y, w1, true);
            }
            SplitOrientation::Horizontal => {
                let divider_x = x1 + w1 + config.divider_padding + config.divider_width / 2.0;
                draw_divider(cr, config, divider_x, y1, h1, false);
            }
        }
    }
}
