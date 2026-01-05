//! Industrial/Gauge Panel display rendering
//!
//! Features:
//! - Brushed metal or carbon fiber textures (simulated with gradients)
//! - Physical gauge aesthetics (rivets, bezels, 3D effects)
//! - Warning stripe accents (yellow/black diagonal stripes)
//! - Pressure gauge-style circular displays
//! - Heavy bold typography

use cairo::Context;
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

use crate::ui::background::Color;
use crate::ui::combo_config_base::{LayoutFrameConfig, ThemedFrameConfig};
use crate::ui::lcars_display::ContentItemConfig;
use crate::ui::theme::ComboThemeConfig;
use std::collections::HashMap;
use crate::ui::lcars_display::SplitOrientation;

/// Surface texture style
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum SurfaceTexture {
    #[default]
    #[serde(rename = "brushed_metal")]
    BrushedMetal,
    #[serde(rename = "carbon_fiber")]
    CarbonFiber,
    #[serde(rename = "diamond_plate")]
    DiamondPlate,
    #[serde(rename = "solid")]
    Solid,
}

/// Rivet style for panel decoration
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum RivetStyle {
    #[default]
    #[serde(rename = "hex")]
    Hex,
    #[serde(rename = "phillips")]
    Phillips,
    #[serde(rename = "flat")]
    Flat,
    #[serde(rename = "none")]
    None,
}

/// Warning stripe position
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum WarningStripePosition {
    #[serde(rename = "none")]
    #[default]
    None,
    #[serde(rename = "top")]
    Top,
    #[serde(rename = "bottom")]
    Bottom,
    #[serde(rename = "left")]
    Left,
    #[serde(rename = "right")]
    Right,
    #[serde(rename = "all")]
    All,
}

/// Header style for the panel
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum HeaderStyle {
    #[default]
    #[serde(rename = "plate")]
    Plate,          // Metal plate with embossed text
    #[serde(rename = "stencil")]
    Stencil,        // Stenciled text
    #[serde(rename = "label")]
    Label,          // Label plate (like equipment labels)
    #[serde(rename = "none")]
    None,
}

/// Divider style between groups
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum DividerStyle {
    #[default]
    #[serde(rename = "groove")]
    Groove,         // Grooved metal divider
    #[serde(rename = "raised")]
    Raised,         // Raised metal bar
    #[serde(rename = "warning")]
    Warning,        // Warning stripes
    #[serde(rename = "none")]
    None,
}

/// Industrial frame configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndustrialFrameConfig {
    // Surface appearance
    pub surface_texture: SurfaceTexture,
    pub surface_color: Color,           // Base metal/surface color
    pub surface_color_dark: Color,      // For gradient/texture
    pub highlight_color: Color,         // Specular highlights

    // Border and frame
    pub show_border: bool,
    pub border_width: f64,
    pub border_color: Color,
    pub corner_radius: f64,
    pub show_beveled_edge: bool,
    pub bevel_width: f64,

    // Rivets
    pub rivet_style: RivetStyle,
    pub rivet_size: f64,
    pub rivet_color: Color,
    pub rivet_spacing: f64,             // Spacing between rivets
    pub show_corner_rivets: bool,
    pub show_edge_rivets: bool,

    // Warning stripes
    pub warning_stripe_position: WarningStripePosition,
    pub warning_stripe_width: f64,
    pub warning_color_1: Color,         // Usually yellow
    pub warning_color_2: Color,         // Usually black
    pub warning_stripe_angle: f64,      // Degrees

    // Header
    pub show_header: bool,
    pub header_text: String,
    pub header_style: HeaderStyle,
    pub header_height: f64,
    pub header_font: String,
    pub header_font_size: f64,
    pub header_color: Color,

    // Layout
    pub content_padding: f64,
    pub item_spacing: f64,
    pub group_count: usize,
    pub group_item_counts: Vec<usize>,
    pub group_size_weights: Vec<f64>,
    pub split_orientation: SplitOrientation,
    /// Item orientation within each group - defaults to same as split_orientation
    #[serde(default)]
    pub group_item_orientations: Vec<SplitOrientation>,

    // Dividers
    pub divider_style: DividerStyle,
    pub divider_width: f64,
    pub divider_color: Color,

    // Content items config
    pub content_items: std::collections::HashMap<String, crate::ui::lcars_display::ContentItemConfig>,

    /// Theme configuration
    pub theme: crate::ui::theme::ComboThemeConfig,
}

fn default_industrial_theme() -> crate::ui::theme::ComboThemeConfig {
    crate::ui::theme::ComboThemeConfig::default_for_industrial()
}

impl Default for IndustrialFrameConfig {
    fn default() -> Self {
        Self {
            // Surface - brushed steel look
            surface_texture: SurfaceTexture::BrushedMetal,
            surface_color: Color { r: 0.55, g: 0.57, b: 0.58, a: 1.0 },      // Steel gray
            surface_color_dark: Color { r: 0.40, g: 0.42, b: 0.43, a: 1.0 }, // Darker steel
            highlight_color: Color { r: 0.75, g: 0.77, b: 0.78, a: 1.0 },    // Highlight

            // Border
            show_border: true,
            border_width: 3.0,
            border_color: Color { r: 0.25, g: 0.25, b: 0.25, a: 1.0 },
            corner_radius: 8.0,
            show_beveled_edge: true,
            bevel_width: 4.0,

            // Rivets
            rivet_style: RivetStyle::Hex,
            rivet_size: 8.0,
            rivet_color: Color { r: 0.35, g: 0.35, b: 0.35, a: 1.0 },
            rivet_spacing: 60.0,
            show_corner_rivets: true,
            show_edge_rivets: false,

            // Warning stripes
            warning_stripe_position: WarningStripePosition::None,
            warning_stripe_width: 20.0,
            warning_color_1: Color { r: 1.0, g: 0.8, b: 0.0, a: 1.0 },   // Yellow
            warning_color_2: Color { r: 0.1, g: 0.1, b: 0.1, a: 1.0 },   // Black
            warning_stripe_angle: 45.0,

            // Header
            show_header: true,
            header_text: "SYSTEM MONITOR".to_string(),
            header_style: HeaderStyle::Plate,
            header_height: 36.0,
            header_font: "Sans Bold".to_string(),
            header_font_size: 16.0,
            header_color: Color { r: 0.1, g: 0.1, b: 0.1, a: 1.0 },

            // Layout
            content_padding: 12.0,
            item_spacing: 8.0,
            group_count: 1,
            group_item_counts: vec![3],
            group_size_weights: vec![1.0],
            split_orientation: SplitOrientation::Horizontal,
            group_item_orientations: Vec::new(),

            // Dividers
            divider_style: DividerStyle::Groove,
            divider_width: 4.0,
            divider_color: Color { r: 0.3, g: 0.3, b: 0.3, a: 1.0 },

            content_items: std::collections::HashMap::new(),
            theme: default_industrial_theme(),
        }
    }
}

impl LayoutFrameConfig for IndustrialFrameConfig {
    fn group_count(&self) -> usize {
        self.group_count
    }

    fn group_size_weights(&self) -> &Vec<f64> {
        &self.group_size_weights
    }

    fn group_size_weights_mut(&mut self) -> &mut Vec<f64> {
        &mut self.group_size_weights
    }

    fn group_item_orientations(&self) -> &Vec<SplitOrientation> {
        &self.group_item_orientations
    }

    fn group_item_orientations_mut(&mut self) -> &mut Vec<SplitOrientation> {
        &mut self.group_item_orientations
    }

    fn split_orientation(&self) -> SplitOrientation {
        self.split_orientation
    }
}

impl ThemedFrameConfig for IndustrialFrameConfig {
    fn theme(&self) -> &ComboThemeConfig {
        &self.theme
    }

    fn theme_mut(&mut self) -> &mut ComboThemeConfig {
        &mut self.theme
    }

    fn content_items(&self) -> &HashMap<String, ContentItemConfig> {
        &self.content_items
    }

    fn content_items_mut(&mut self) -> &mut HashMap<String, ContentItemConfig> {
        &mut self.content_items
    }
}

impl IndustrialFrameConfig {
    /// Get text color based on background
    pub fn text_color(&self) -> Color {
        // Dark text on metal background
        Color { r: 0.1, g: 0.1, b: 0.1, a: 1.0 }
    }
}

/// Render the complete Industrial frame
/// Returns the content area bounds (x, y, width, height)
pub fn render_industrial_frame(
    cr: &Context,
    config: &IndustrialFrameConfig,
    width: f64,
    height: f64,
) -> Result<(f64, f64, f64, f64), cairo::Error> {
    if width < 1.0 || height < 1.0 {
        return Ok((0.0, 0.0, 0.0, 0.0));
    }

    cr.save()?;

    // Draw base surface with texture
    draw_surface(cr, config, 0.0, 0.0, width, height)?;

    // Draw beveled edge
    if config.show_beveled_edge {
        draw_bevel(cr, config, 0.0, 0.0, width, height)?;
    }

    // Draw border
    if config.show_border {
        draw_border(cr, config, 0.0, 0.0, width, height)?;
    }

    // Draw warning stripes
    draw_warning_stripes(cr, config, 0.0, 0.0, width, height)?;

    // Draw rivets
    draw_rivets(cr, config, 0.0, 0.0, width, height)?;

    // Draw header
    let header_height = if config.show_header && !config.header_text.is_empty() {
        draw_header(cr, config, config.content_padding, config.content_padding,
                   width - config.content_padding * 2.0)?
    } else {
        0.0
    };

    cr.restore()?;

    // Calculate content area
    let content_x = config.content_padding;
    let content_y = config.content_padding + header_height;
    let content_w = width - config.content_padding * 2.0;
    let content_h = height - config.content_padding * 2.0 - header_height;

    Ok((content_x, content_y, content_w, content_h))
}

/// Draw the surface texture
fn draw_surface(
    cr: &Context,
    config: &IndustrialFrameConfig,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) -> Result<(), cairo::Error> {
    cr.save()?;

    // Create rounded rectangle path
    draw_rounded_rect(cr, x, y, w, h, config.corner_radius);
    cr.clip();

    match config.surface_texture {
        SurfaceTexture::BrushedMetal => {
            draw_brushed_metal(cr, config, x, y, w, h)?;
        }
        SurfaceTexture::CarbonFiber => {
            draw_carbon_fiber(cr, config, x, y, w, h)?;
        }
        SurfaceTexture::DiamondPlate => {
            draw_diamond_plate(cr, config, x, y, w, h)?;
        }
        SurfaceTexture::Solid => {
            let c = &config.surface_color;
            cr.set_source_rgba(c.r, c.g, c.b, c.a);
            cr.paint()?;
        }
    }

    cr.restore()?;
    Ok(())
}

/// Draw brushed metal texture
fn draw_brushed_metal(
    cr: &Context,
    config: &IndustrialFrameConfig,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) -> Result<(), cairo::Error> {
    // Base gradient (vertical)
    let gradient = cairo::LinearGradient::new(x, y, x, y + h);
    let c1 = &config.surface_color;
    let c2 = &config.surface_color_dark;
    let ch = &config.highlight_color;

    gradient.add_color_stop_rgba(0.0, ch.r, ch.g, ch.b, 0.3);
    gradient.add_color_stop_rgba(0.1, c1.r, c1.g, c1.b, c1.a);
    gradient.add_color_stop_rgba(0.5, c2.r, c2.g, c2.b, c2.a);
    gradient.add_color_stop_rgba(0.9, c1.r, c1.g, c1.b, c1.a);
    gradient.add_color_stop_rgba(1.0, c2.r, c2.g, c2.b, c2.a);

    cr.set_source(&gradient)?;
    cr.paint()?;

    // Add horizontal brush strokes
    cr.set_line_width(0.5);
    let stroke_spacing = 2.0;
    let mut stroke_y = y;
    while stroke_y < y + h {
        let alpha = 0.05 + (stroke_y * 0.1).sin().abs() * 0.05;
        cr.set_source_rgba(1.0, 1.0, 1.0, alpha);
        cr.move_to(x, stroke_y);
        cr.line_to(x + w, stroke_y);
        cr.stroke()?;
        stroke_y += stroke_spacing;
    }

    Ok(())
}

/// Draw carbon fiber texture
fn draw_carbon_fiber(
    cr: &Context,
    _config: &IndustrialFrameConfig,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) -> Result<(), cairo::Error> {
    // Dark base
    cr.set_source_rgba(0.1, 0.1, 0.1, 1.0);
    cr.paint()?;

    // Draw weave pattern
    let cell_size = 8.0;
    cr.set_line_width(1.0);

    let mut cy = y;
    let mut row = 0;
    while cy < y + h {
        let mut cx = x;
        let mut col = 0;
        while cx < x + w {
            let _offset = if (row + col) % 2 == 0 { 0.0 } else { cell_size / 2.0 };

            // Draw diagonal lines for weave effect
            if (row + col) % 2 == 0 {
                cr.set_source_rgba(0.2, 0.2, 0.2, 1.0);
            } else {
                cr.set_source_rgba(0.15, 0.15, 0.15, 1.0);
            }

            cr.rectangle(cx, cy, cell_size, cell_size);
            cr.fill()?;

            // Add subtle highlight
            if (row + col) % 2 == 0 {
                cr.set_source_rgba(1.0, 1.0, 1.0, 0.05);
                cr.move_to(cx, cy);
                cr.line_to(cx + cell_size, cy + cell_size);
                cr.stroke()?;
            }

            cx += cell_size;
            col += 1;
        }
        cy += cell_size;
        row += 1;
    }

    // Add overall sheen
    let gradient = cairo::LinearGradient::new(x, y, x + w, y + h);
    gradient.add_color_stop_rgba(0.0, 1.0, 1.0, 1.0, 0.1);
    gradient.add_color_stop_rgba(0.5, 1.0, 1.0, 1.0, 0.0);
    gradient.add_color_stop_rgba(1.0, 1.0, 1.0, 1.0, 0.05);
    cr.set_source(&gradient)?;
    cr.paint()?;

    Ok(())
}

/// Draw diamond plate texture
fn draw_diamond_plate(
    cr: &Context,
    config: &IndustrialFrameConfig,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) -> Result<(), cairo::Error> {
    // Base metal color
    let c = &config.surface_color;
    cr.set_source_rgba(c.r, c.g, c.b, c.a);
    cr.paint()?;

    // Draw raised diamond pattern
    let diamond_w = 16.0;
    let diamond_h = 8.0;
    let spacing_x = diamond_w + 4.0;
    let spacing_y = diamond_h + 4.0;

    cr.set_line_width(1.0);

    let mut row = 0;
    let mut dy = y;
    while dy < y + h {
        let offset = if row % 2 == 0 { 0.0 } else { spacing_x / 2.0 };
        let mut dx = x + offset;
        while dx < x + w {
            // Draw raised diamond
            cr.save()?;

            // Shadow
            cr.set_source_rgba(0.0, 0.0, 0.0, 0.3);
            cr.move_to(dx + diamond_w / 2.0 + 1.0, dy + 1.0);
            cr.line_to(dx + diamond_w + 1.0, dy + diamond_h / 2.0 + 1.0);
            cr.line_to(dx + diamond_w / 2.0 + 1.0, dy + diamond_h + 1.0);
            cr.line_to(dx + 1.0, dy + diamond_h / 2.0 + 1.0);
            cr.close_path();
            cr.fill()?;

            // Highlight
            let ch = &config.highlight_color;
            cr.set_source_rgba(ch.r, ch.g, ch.b, 0.6);
            cr.move_to(dx + diamond_w / 2.0, dy);
            cr.line_to(dx + diamond_w, dy + diamond_h / 2.0);
            cr.line_to(dx + diamond_w / 2.0, dy + diamond_h);
            cr.line_to(dx, dy + diamond_h / 2.0);
            cr.close_path();
            cr.fill()?;

            cr.restore()?;
            dx += spacing_x;
        }
        dy += spacing_y;
        row += 1;
    }

    Ok(())
}

/// Draw beveled edge effect
fn draw_bevel(
    cr: &Context,
    config: &IndustrialFrameConfig,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) -> Result<(), cairo::Error> {
    let bevel = config.bevel_width;
    let radius = config.corner_radius;

    cr.save()?;

    // Top-left highlight (light)
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.3);
    cr.set_line_width(bevel);
    cr.new_path();
    cr.move_to(x + radius, y + bevel / 2.0);
    cr.line_to(x + w - radius, y + bevel / 2.0);
    cr.stroke()?;

    cr.move_to(x + bevel / 2.0, y + radius);
    cr.line_to(x + bevel / 2.0, y + h - radius);
    cr.stroke()?;

    // Bottom-right shadow (dark)
    cr.set_source_rgba(0.0, 0.0, 0.0, 0.4);
    cr.new_path();
    cr.move_to(x + radius, y + h - bevel / 2.0);
    cr.line_to(x + w - radius, y + h - bevel / 2.0);
    cr.stroke()?;

    cr.move_to(x + w - bevel / 2.0, y + radius);
    cr.line_to(x + w - bevel / 2.0, y + h - radius);
    cr.stroke()?;

    cr.restore()?;
    Ok(())
}

/// Draw border
fn draw_border(
    cr: &Context,
    config: &IndustrialFrameConfig,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) -> Result<(), cairo::Error> {
    cr.save()?;

    let c = &config.border_color;
    cr.set_source_rgba(c.r, c.g, c.b, c.a);
    cr.set_line_width(config.border_width);

    draw_rounded_rect(cr,
        x + config.border_width / 2.0,
        y + config.border_width / 2.0,
        w - config.border_width,
        h - config.border_width,
        config.corner_radius);
    cr.stroke()?;

    cr.restore()?;
    Ok(())
}

/// Draw warning stripes
fn draw_warning_stripes(
    cr: &Context,
    config: &IndustrialFrameConfig,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) -> Result<(), cairo::Error> {
    if matches!(config.warning_stripe_position, WarningStripePosition::None) {
        return Ok(());
    }

    cr.save()?;

    let stripe_w = config.warning_stripe_width;
    let c1 = &config.warning_color_1;
    let c2 = &config.warning_color_2;
    let angle = config.warning_stripe_angle.to_radians();

    let draw_stripe_area = |cr: &Context, sx: f64, sy: f64, sw: f64, sh: f64| -> Result<(), cairo::Error> {
        cr.save()?;

        // Clip to stripe area
        cr.rectangle(sx, sy, sw, sh);
        cr.clip();

        // Draw diagonal stripes
        let stripe_width = 10.0;
        let diagonal = (sw * sw + sh * sh).sqrt();
        let num_stripes = (diagonal / stripe_width) as i32 + 4;

        cr.translate(sx + sw / 2.0, sy + sh / 2.0);
        cr.rotate(angle);
        cr.translate(-(sw / 2.0), -(sh / 2.0));

        for i in -num_stripes..num_stripes {
            let stripe_x = i as f64 * stripe_width * 2.0 - diagonal;
            if i % 2 == 0 {
                cr.set_source_rgba(c1.r, c1.g, c1.b, c1.a);
            } else {
                cr.set_source_rgba(c2.r, c2.g, c2.b, c2.a);
            }
            cr.rectangle(stripe_x, -diagonal, stripe_width, diagonal * 3.0);
            cr.fill()?;
        }

        cr.restore()?;
        Ok(())
    };

    match config.warning_stripe_position {
        WarningStripePosition::Top => {
            draw_stripe_area(cr, x, y, w, stripe_w)?;
        }
        WarningStripePosition::Bottom => {
            draw_stripe_area(cr, x, y + h - stripe_w, w, stripe_w)?;
        }
        WarningStripePosition::Left => {
            draw_stripe_area(cr, x, y, stripe_w, h)?;
        }
        WarningStripePosition::Right => {
            draw_stripe_area(cr, x + w - stripe_w, y, stripe_w, h)?;
        }
        WarningStripePosition::All => {
            draw_stripe_area(cr, x, y, w, stripe_w)?;
            draw_stripe_area(cr, x, y + h - stripe_w, w, stripe_w)?;
            draw_stripe_area(cr, x, y + stripe_w, stripe_w, h - stripe_w * 2.0)?;
            draw_stripe_area(cr, x + w - stripe_w, y + stripe_w, stripe_w, h - stripe_w * 2.0)?;
        }
        WarningStripePosition::None => {}
    }

    cr.restore()?;
    Ok(())
}

/// Draw rivets
fn draw_rivets(
    cr: &Context,
    config: &IndustrialFrameConfig,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) -> Result<(), cairo::Error> {
    if matches!(config.rivet_style, RivetStyle::None) {
        return Ok(());
    }

    let size = config.rivet_size;
    let margin = config.border_width + config.bevel_width + size;

    // Draw corner rivets
    if config.show_corner_rivets {
        draw_rivet(cr, config, x + margin, y + margin)?;
        draw_rivet(cr, config, x + w - margin, y + margin)?;
        draw_rivet(cr, config, x + margin, y + h - margin)?;
        draw_rivet(cr, config, x + w - margin, y + h - margin)?;
    }

    // Draw edge rivets
    if config.show_edge_rivets {
        let spacing = config.rivet_spacing;

        // Top and bottom edges
        let mut rx = x + margin + spacing;
        while rx < x + w - margin - spacing {
            draw_rivet(cr, config, rx, y + margin)?;
            draw_rivet(cr, config, rx, y + h - margin)?;
            rx += spacing;
        }

        // Left and right edges
        let mut ry = y + margin + spacing;
        while ry < y + h - margin - spacing {
            draw_rivet(cr, config, x + margin, ry)?;
            draw_rivet(cr, config, x + w - margin, ry)?;
            ry += spacing;
        }
    }

    Ok(())
}

/// Draw a single rivet
fn draw_rivet(
    cr: &Context,
    config: &IndustrialFrameConfig,
    cx: f64,
    cy: f64,
) -> Result<(), cairo::Error> {
    let size = config.rivet_size;
    let c = &config.rivet_color;

    cr.save()?;

    match config.rivet_style {
        RivetStyle::Hex => {
            // Hexagonal bolt head
            let radius = size / 2.0;
            cr.new_path();
            for i in 0..6 {
                let angle = (i as f64 * 60.0 - 30.0).to_radians();
                let px = cx + radius * angle.cos();
                let py = cy + radius * angle.sin();
                if i == 0 {
                    cr.move_to(px, py);
                } else {
                    cr.line_to(px, py);
                }
            }
            cr.close_path();

            // Fill with gradient for 3D effect
            let gradient = cairo::RadialGradient::new(cx - size / 4.0, cy - size / 4.0, 0.0,
                                                       cx, cy, size);
            gradient.add_color_stop_rgba(0.0, c.r + 0.3, c.g + 0.3, c.b + 0.3, c.a);
            gradient.add_color_stop_rgba(1.0, c.r - 0.1, c.g - 0.1, c.b - 0.1, c.a);
            cr.set_source(&gradient)?;
            cr.fill_preserve()?;

            cr.set_source_rgba(0.0, 0.0, 0.0, 0.5);
            cr.set_line_width(0.5);
            cr.stroke()?;
        }
        RivetStyle::Phillips => {
            // Round screw with Phillips head
            cr.arc(cx, cy, size / 2.0, 0.0, 2.0 * PI);

            let gradient = cairo::RadialGradient::new(cx - size / 4.0, cy - size / 4.0, 0.0,
                                                       cx, cy, size);
            gradient.add_color_stop_rgba(0.0, c.r + 0.2, c.g + 0.2, c.b + 0.2, c.a);
            gradient.add_color_stop_rgba(1.0, c.r - 0.1, c.g - 0.1, c.b - 0.1, c.a);
            cr.set_source(&gradient)?;
            cr.fill()?;

            // Draw Phillips cross
            cr.set_source_rgba(0.0, 0.0, 0.0, 0.6);
            cr.set_line_width(1.5);
            let cross_size = size * 0.3;
            cr.move_to(cx - cross_size, cy);
            cr.line_to(cx + cross_size, cy);
            cr.stroke()?;
            cr.move_to(cx, cy - cross_size);
            cr.line_to(cx, cy + cross_size);
            cr.stroke()?;
        }
        RivetStyle::Flat => {
            // Simple flat rivet
            cr.arc(cx, cy, size / 2.0, 0.0, 2.0 * PI);
            cr.set_source_rgba(c.r, c.g, c.b, c.a);
            cr.fill_preserve()?;

            cr.set_source_rgba(0.0, 0.0, 0.0, 0.3);
            cr.set_line_width(0.5);
            cr.stroke()?;
        }
        RivetStyle::None => {}
    }

    cr.restore()?;
    Ok(())
}

/// Draw header
fn draw_header(
    cr: &Context,
    config: &IndustrialFrameConfig,
    x: f64,
    y: f64,
    w: f64,
) -> Result<f64, cairo::Error> {
    let h = config.header_height;

    cr.save()?;

    match config.header_style {
        HeaderStyle::Plate => {
            // Metal plate with embossed look
            let plate_w = w.min(config.header_text.len() as f64 * config.header_font_size * 0.6 + 40.0);
            let plate_x = x + (w - plate_w) / 2.0;

            // Plate background
            cr.rectangle(plate_x, y, plate_w, h);

            let gradient = cairo::LinearGradient::new(plate_x, y, plate_x, y + h);
            gradient.add_color_stop_rgba(0.0, 0.5, 0.52, 0.53, 1.0);
            gradient.add_color_stop_rgba(0.5, 0.45, 0.47, 0.48, 1.0);
            gradient.add_color_stop_rgba(1.0, 0.4, 0.42, 0.43, 1.0);
            cr.set_source(&gradient)?;
            cr.fill()?;

            // Embossed border
            cr.set_line_width(1.0);
            cr.set_source_rgba(1.0, 1.0, 1.0, 0.3);
            cr.move_to(plate_x + 1.0, y + h - 1.0);
            cr.line_to(plate_x + 1.0, y + 1.0);
            cr.line_to(plate_x + plate_w - 1.0, y + 1.0);
            cr.stroke()?;

            cr.set_source_rgba(0.0, 0.0, 0.0, 0.4);
            cr.move_to(plate_x + plate_w - 1.0, y + 1.0);
            cr.line_to(plate_x + plate_w - 1.0, y + h - 1.0);
            cr.line_to(plate_x + 1.0, y + h - 1.0);
            cr.stroke()?;

            // Text
            let c = &config.header_color;
            cr.set_source_rgba(c.r, c.g, c.b, c.a);
            crate::ui::render_cache::apply_cached_font(cr, &config.header_font, cairo::FontSlant::Normal, cairo::FontWeight::Bold, config.header_font_size);

            let extents = cr.text_extents(&config.header_text)?;
            let text_x = plate_x + (plate_w - extents.width()) / 2.0;
            let text_y = y + (h + extents.height()) / 2.0;

            // Shadow
            cr.set_source_rgba(0.0, 0.0, 0.0, 0.3);
            cr.move_to(text_x + 1.0, text_y + 1.0);
            cr.show_text(&config.header_text)?;

            // Main text
            cr.set_source_rgba(c.r, c.g, c.b, c.a);
            cr.move_to(text_x, text_y);
            cr.show_text(&config.header_text)?;
        }
        HeaderStyle::Stencil => {
            // Stenciled text look
            crate::ui::render_cache::apply_cached_font(cr, &config.header_font, cairo::FontSlant::Normal, cairo::FontWeight::Bold, config.header_font_size);

            let extents = cr.text_extents(&config.header_text)?;
            let text_x = x + (w - extents.width()) / 2.0;
            let text_y = y + (h + extents.height()) / 2.0;

            // Spray paint effect
            cr.set_source_rgba(0.1, 0.1, 0.1, 0.9);
            cr.move_to(text_x, text_y);
            cr.show_text(&config.header_text)?;
        }
        HeaderStyle::Label => {
            // Equipment label style
            let label_w = config.header_text.len() as f64 * config.header_font_size * 0.6 + 20.0;
            let label_x = x + (w - label_w) / 2.0;
            let label_h = h - 8.0;
            let label_y = y + 4.0;

            // Yellow label background
            cr.rectangle(label_x, label_y, label_w, label_h);
            cr.set_source_rgba(1.0, 0.9, 0.3, 1.0);
            cr.fill()?;

            // Black border
            cr.rectangle(label_x, label_y, label_w, label_h);
            cr.set_source_rgba(0.0, 0.0, 0.0, 1.0);
            cr.set_line_width(2.0);
            cr.stroke()?;

            // Text
            cr.set_source_rgba(0.0, 0.0, 0.0, 1.0);
            crate::ui::render_cache::apply_cached_font(cr, &config.header_font, cairo::FontSlant::Normal, cairo::FontWeight::Bold, config.header_font_size);

            let extents = cr.text_extents(&config.header_text)?;
            let text_x = label_x + (label_w - extents.width()) / 2.0;
            let text_y = label_y + (label_h + extents.height()) / 2.0;
            cr.move_to(text_x, text_y);
            cr.show_text(&config.header_text)?;
        }
        HeaderStyle::None => {
            cr.restore()?;
            return Ok(0.0);
        }
    }

    cr.restore()?;
    Ok(h + 8.0)  // Header height plus spacing
}

/// Calculate group layouts
/// Returns Vec of (x, y, width, height, item_count) for each group
pub fn calculate_group_layouts(
    content_x: f64,
    content_y: f64,
    content_w: f64,
    content_h: f64,
    config: &IndustrialFrameConfig,
) -> Vec<(f64, f64, f64, f64, usize)> {
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
    let divider_space = divider_count as f64 * (config.divider_width + 8.0);

    match config.split_orientation {
        SplitOrientation::Horizontal => {
            // Horizontal layout (groups side by side)
            let available_width = content_w - divider_space;
            let mut current_x = content_x;

            for (i, weight) in weights.iter().enumerate() {
                let group_w = available_width * (weight / total_weight);
                let item_count = config.group_item_counts.get(i).copied().unwrap_or(3);
                layouts.push((current_x, content_y, group_w, content_h, item_count));
                current_x += group_w;
                if i < divider_count {
                    current_x += config.divider_width + 8.0;
                }
            }
        }
        SplitOrientation::Vertical => {
            // Vertical layout (groups stacked)
            let available_height = content_h - divider_space;
            let mut current_y = content_y;

            for (i, weight) in weights.iter().enumerate() {
                let group_h = available_height * (weight / total_weight);
                let item_count = config.group_item_counts.get(i).copied().unwrap_or(3);
                layouts.push((content_x, current_y, content_w, group_h, item_count));
                current_y += group_h;
                if i < divider_count {
                    current_y += config.divider_width + 8.0;
                }
            }
        }
    }

    layouts
}

/// Draw group dividers
pub fn draw_group_dividers(
    cr: &Context,
    layouts: &[(f64, f64, f64, f64, usize)],
    config: &IndustrialFrameConfig,
) -> Result<(), cairo::Error> {
    if layouts.len() <= 1 || matches!(config.divider_style, DividerStyle::None) {
        return Ok(());
    }

    cr.save()?;

    let is_horizontal = matches!(config.split_orientation, SplitOrientation::Horizontal);

    for i in 0..layouts.len() - 1 {
        let (_x1, _y1, _w1, _h1, _) = layouts[i];
        let (x2, y2, w2, h2, _) = layouts[i + 1];

        // Calculate divider position based on orientation
        let (div_x, div_y, div_w, div_h) = if is_horizontal {
            // Vertical divider between horizontal groups
            let dx = x2 - config.divider_width / 2.0 - 2.0;
            (dx, y2, config.divider_width, h2)
        } else {
            // Horizontal divider between vertical groups
            let dy = y2 - config.divider_width / 2.0 - 2.0;
            (x2, dy, w2, config.divider_width)
        };

        match config.divider_style {
            DividerStyle::Groove => {
                cr.set_line_width(1.0);

                if is_horizontal {
                    // Vertical groove
                    cr.set_source_rgba(0.0, 0.0, 0.0, 0.5);
                    cr.move_to(div_x, div_y);
                    cr.line_to(div_x, div_y + div_h);
                    cr.stroke()?;

                    cr.set_source_rgba(1.0, 1.0, 1.0, 0.3);
                    cr.move_to(div_x + 2.0, div_y);
                    cr.line_to(div_x + 2.0, div_y + div_h);
                    cr.stroke()?;
                } else {
                    // Horizontal groove
                    cr.set_source_rgba(0.0, 0.0, 0.0, 0.5);
                    cr.move_to(div_x, div_y);
                    cr.line_to(div_x + div_w, div_y);
                    cr.stroke()?;

                    cr.set_source_rgba(1.0, 1.0, 1.0, 0.3);
                    cr.move_to(div_x, div_y + 2.0);
                    cr.line_to(div_x + div_w, div_y + 2.0);
                    cr.stroke()?;
                }
            }
            DividerStyle::Raised => {
                let c = &config.divider_color;
                cr.set_source_rgba(c.r, c.g, c.b, c.a);

                if is_horizontal {
                    cr.rectangle(div_x, div_y, config.divider_width, div_h);
                    cr.fill()?;

                    cr.set_source_rgba(1.0, 1.0, 1.0, 0.3);
                    cr.move_to(div_x, div_y);
                    cr.line_to(div_x, div_y + div_h);
                    cr.stroke()?;

                    cr.set_source_rgba(0.0, 0.0, 0.0, 0.3);
                    cr.move_to(div_x + config.divider_width, div_y);
                    cr.line_to(div_x + config.divider_width, div_y + div_h);
                    cr.stroke()?;
                } else {
                    cr.rectangle(div_x, div_y, div_w, config.divider_width);
                    cr.fill()?;

                    cr.set_source_rgba(1.0, 1.0, 1.0, 0.3);
                    cr.move_to(div_x, div_y);
                    cr.line_to(div_x + div_w, div_y);
                    cr.stroke()?;

                    cr.set_source_rgba(0.0, 0.0, 0.0, 0.3);
                    cr.move_to(div_x, div_y + config.divider_width);
                    cr.line_to(div_x + div_w, div_y + config.divider_width);
                    cr.stroke()?;
                }
            }
            DividerStyle::Warning => {
                let c1 = &config.warning_color_1;
                let c2 = &config.warning_color_2;
                let stripe_size = 8.0;

                cr.save()?;

                if is_horizontal {
                    cr.rectangle(div_x - 2.0, div_y, config.divider_width + 4.0, div_h);
                    cr.clip();

                    let mut sy = div_y;
                    let mut stripe_idx = 0;
                    while sy < div_y + div_h {
                        if stripe_idx % 2 == 0 {
                            cr.set_source_rgba(c1.r, c1.g, c1.b, c1.a);
                        } else {
                            cr.set_source_rgba(c2.r, c2.g, c2.b, c2.a);
                        }
                        cr.rectangle(div_x - 2.0, sy, config.divider_width + 4.0, stripe_size);
                        cr.fill()?;
                        sy += stripe_size;
                        stripe_idx += 1;
                    }
                } else {
                    cr.rectangle(div_x, div_y - 2.0, div_w, config.divider_width + 4.0);
                    cr.clip();

                    let mut sx = div_x;
                    let mut stripe_idx = 0;
                    while sx < div_x + div_w {
                        if stripe_idx % 2 == 0 {
                            cr.set_source_rgba(c1.r, c1.g, c1.b, c1.a);
                        } else {
                            cr.set_source_rgba(c2.r, c2.g, c2.b, c2.a);
                        }
                        cr.rectangle(sx, div_y - 2.0, stripe_size, config.divider_width + 4.0);
                        cr.fill()?;
                        sx += stripe_size;
                        stripe_idx += 1;
                    }
                }

                cr.restore()?;
            }
            DividerStyle::None => {}
        }
    }

    cr.restore()?;
    Ok(())
}

/// Draw group panel (optional raised panel effect)
pub fn draw_group_panel(
    cr: &Context,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    _config: &IndustrialFrameConfig,
) -> Result<(), cairo::Error> {
    cr.save()?;

    // Subtle inset panel effect
    cr.set_source_rgba(0.0, 0.0, 0.0, 0.1);
    cr.rectangle(x, y, w, h);
    cr.fill()?;

    // Inner highlight (top-left)
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.1);
    cr.set_line_width(1.0);
    cr.move_to(x, y + h);
    cr.line_to(x, y);
    cr.line_to(x + w, y);
    cr.stroke()?;

    // Inner shadow (bottom-right)
    cr.set_source_rgba(0.0, 0.0, 0.0, 0.2);
    cr.move_to(x + w, y);
    cr.line_to(x + w, y + h);
    cr.line_to(x, y + h);
    cr.stroke()?;

    cr.restore()?;
    Ok(())
}

/// Helper function to draw a rounded rectangle path
fn draw_rounded_rect(cr: &Context, x: f64, y: f64, w: f64, h: f64, r: f64) {
    let r = r.min(w / 2.0).min(h / 2.0);
    cr.new_path();
    cr.arc(x + w - r, y + r, r, -PI / 2.0, 0.0);
    cr.arc(x + w - r, y + h - r, r, 0.0, PI / 2.0);
    cr.arc(x + r, y + h - r, r, PI / 2.0, PI);
    cr.arc(x + r, y + r, r, PI, 3.0 * PI / 2.0);
    cr.close_path();
}
