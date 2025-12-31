//! Art Deco display rendering
//!
//! Provides a 1920s-inspired Art Deco display with:
//! - Sunburst and fan corner decorations
//! - Stepped/ziggurat border patterns
//! - Chevron dividers and accents
//! - Gold, copper, brass metallic color schemes
//! - Geometric background patterns

use std::collections::HashMap;

use anyhow::Result;
use cairo::Context;
use serde::{Deserialize, Serialize};

use crate::ui::background::Color;
use crate::ui::lcars_display::{ContentItemConfig, SplitOrientation};
use crate::ui::theme::{ColorSource, FontSource, ComboThemeConfig, deserialize_color_or_source, deserialize_font_or_source};

// Re-export types we use
pub use crate::ui::lcars_display::{ContentDisplayType as ArtDecoContentType, ContentItemConfig as ArtDecoContentItemConfig};

/// Border style for the frame
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum BorderStyle {
    /// Sunburst radiating lines from corners
    #[default]
    Sunburst,
    /// V-pattern chevron border
    Chevron,
    /// Stepped ziggurat-style edges
    Stepped,
    /// Simple geometric lines
    Geometric,
    /// Full ornate frame with multiple elements
    Ornate,
}

/// Corner decoration style
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum CornerStyle {
    /// Radiating fan pattern
    #[default]
    Fan,
    /// Stepped pyramid/ziggurat
    Ziggurat,
    /// Diamond accent
    Diamond,
    /// Simple L-bracket
    Bracket,
    /// No corner decoration
    None,
}

/// Background pattern
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum BackgroundPattern {
    /// Solid color background
    #[default]
    Solid,
    /// Vertical pinstripes
    VerticalLines,
    /// Diamond grid pattern
    DiamondGrid,
    /// Radial sunburst from center
    Sunburst,
    /// Chevron/arrow pattern
    Chevrons,
}

/// Header display style
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum HeaderStyle {
    /// Centered with decorative side elements
    #[default]
    Centered,
    /// Full-width banner bar
    Banner,
    /// Stepped header with tiered effect
    Stepped,
    /// No header
    None,
}

/// Divider style between content groups
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum DividerStyle {
    /// Chevron/arrow pattern divider
    #[default]
    Chevron,
    /// Double line with diamond center
    DoubleLine,
    /// Simple solid line
    Line,
    /// Stepped zigzag pattern
    Stepped,
    /// No divider
    None,
}

fn default_border_width() -> f64 { 3.0 }
fn default_corner_size() -> f64 { 24.0 }
fn default_accent_width() -> f64 { 2.0 }
fn default_pattern_spacing() -> f64 { 16.0 }
fn default_content_padding() -> f64 { 12.0 }
fn default_divider_width() -> f64 { 2.0 }
fn default_divider_padding() -> f64 { 6.0 }
fn default_group_count() -> usize { 2 }
fn default_sunburst_rays() -> usize { 12 }

// ColorSource defaults for theme-aware fields
fn default_border_color_source() -> ColorSource {
    ColorSource::theme(1) // Gold
}

fn default_accent_color_source() -> ColorSource {
    ColorSource::theme(2) // Copper
}

fn default_background_color_source() -> ColorSource {
    ColorSource::theme(4) // Dark charcoal
}

fn default_pattern_color_source() -> ColorSource {
    ColorSource::theme(1) // Gold with low opacity handled in render
}

fn default_header_color_source() -> ColorSource {
    ColorSource::theme(1) // Gold
}

fn default_divider_color_source() -> ColorSource {
    ColorSource::theme(2) // Copper
}

fn default_header_font_source() -> FontSource {
    FontSource::theme(1, 16.0) // Theme font 1, 16pt
}

fn default_art_deco_theme() -> ComboThemeConfig {
    ComboThemeConfig::default_for_art_deco()
}

/// Main configuration for the Art Deco frame
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtDecoFrameConfig {
    // Frame styling
    #[serde(default)]
    pub border_style: BorderStyle,
    #[serde(default = "default_border_width")]
    pub border_width: f64,
    #[serde(default = "default_border_color_source", deserialize_with = "deserialize_color_or_source")]
    pub border_color: ColorSource,

    // Corner decorations
    #[serde(default)]
    pub corner_style: CornerStyle,
    #[serde(default = "default_corner_size")]
    pub corner_size: f64,
    #[serde(default = "default_accent_color_source", deserialize_with = "deserialize_color_or_source")]
    pub accent_color: ColorSource,
    #[serde(default = "default_accent_width")]
    pub accent_width: f64,

    // Background
    #[serde(default = "default_background_color_source", deserialize_with = "deserialize_color_or_source")]
    pub background_color: ColorSource,
    #[serde(default)]
    pub background_pattern: BackgroundPattern,
    #[serde(default = "default_pattern_color_source", deserialize_with = "deserialize_color_or_source")]
    pub pattern_color: ColorSource,
    #[serde(default = "default_pattern_spacing")]
    pub pattern_spacing: f64,
    #[serde(default = "default_sunburst_rays")]
    pub sunburst_rays: usize,

    // Header
    #[serde(default)]
    pub show_header: bool,
    #[serde(default)]
    pub header_text: String,
    #[serde(default = "default_header_font_source", deserialize_with = "deserialize_font_or_source")]
    pub header_font: FontSource,
    #[serde(default = "default_header_color_source", deserialize_with = "deserialize_color_or_source")]
    pub header_color: ColorSource,
    #[serde(default)]
    pub header_style: HeaderStyle,

    // Layout
    #[serde(default = "default_content_padding")]
    pub content_padding: f64,
    #[serde(default = "default_group_count")]
    pub group_count: usize,
    #[serde(default)]
    pub group_item_counts: Vec<usize>,
    #[serde(default)]
    pub group_size_weights: Vec<f64>,
    #[serde(default)]
    pub split_orientation: SplitOrientation,

    // Dividers
    #[serde(default)]
    pub divider_style: DividerStyle,
    #[serde(default = "default_divider_color_source", deserialize_with = "deserialize_color_or_source")]
    pub divider_color: ColorSource,
    #[serde(default = "default_divider_width")]
    pub divider_width: f64,
    #[serde(default = "default_divider_padding")]
    pub divider_padding: f64,

    // Content items (per slot)
    #[serde(default)]
    pub content_items: HashMap<String, ContentItemConfig>,

    /// Theme configuration
    #[serde(default = "default_art_deco_theme")]
    pub theme: ComboThemeConfig,
}

impl Default for ArtDecoFrameConfig {
    fn default() -> Self {
        Self {
            border_style: BorderStyle::default(),
            border_width: default_border_width(),
            border_color: default_border_color_source(),
            corner_style: CornerStyle::default(),
            corner_size: default_corner_size(),
            accent_color: default_accent_color_source(),
            accent_width: default_accent_width(),
            background_color: default_background_color_source(),
            background_pattern: BackgroundPattern::default(),
            pattern_color: default_pattern_color_source(),
            pattern_spacing: default_pattern_spacing(),
            sunburst_rays: default_sunburst_rays(),
            show_header: false,
            header_text: String::new(),
            header_font: default_header_font_source(),
            header_color: default_header_color_source(),
            header_style: HeaderStyle::default(),
            content_padding: default_content_padding(),
            group_count: default_group_count(),
            group_item_counts: vec![1, 1],
            group_size_weights: vec![1.0, 1.0],
            split_orientation: SplitOrientation::default(),
            divider_style: DividerStyle::default(),
            divider_color: default_divider_color_source(),
            divider_width: default_divider_width(),
            divider_padding: default_divider_padding(),
            content_items: HashMap::new(),
            theme: default_art_deco_theme(),
        }
    }
}

/// Draw a sunburst/fan pattern from a corner
fn draw_sunburst_corner(
    cr: &Context,
    cx: f64,
    cy: f64,
    size: f64,
    ray_count: usize,
    start_angle: f64,
    sweep: f64,
    color: &Color,
    line_width: f64,
) {
    cr.save().ok();
    cr.set_source_rgba(color.r, color.g, color.b, color.a);
    cr.set_line_width(line_width);

    let rays = ray_count.max(3);
    let angle_step = sweep / (rays - 1) as f64;

    for i in 0..rays {
        let angle = start_angle + i as f64 * angle_step;
        let end_x = cx + size * angle.cos();
        let end_y = cy + size * angle.sin();

        cr.move_to(cx, cy);
        cr.line_to(end_x, end_y);
    }
    cr.stroke().ok();
    cr.restore().ok();
}

/// Draw a stepped/ziggurat corner decoration
fn draw_ziggurat_corner(
    cr: &Context,
    x: f64,
    y: f64,
    size: f64,
    steps: usize,
    top_left: bool,
    top_right: bool,
    bottom_left: bool,
    bottom_right: bool,
    color: &Color,
    line_width: f64,
) {
    cr.save().ok();
    cr.set_source_rgba(color.r, color.g, color.b, color.a);
    cr.set_line_width(line_width);

    let step_size = size / steps as f64;

    // Top-left corner
    if top_left {
        for i in 0..steps {
            let offset = i as f64 * step_size;
            cr.move_to(x + offset, y + size - offset);
            cr.line_to(x + offset, y + offset);
            cr.line_to(x + size - offset, y + offset);
        }
        cr.stroke().ok();
    }

    // Top-right corner
    if top_right {
        for i in 0..steps {
            let offset = i as f64 * step_size;
            cr.move_to(x + size - offset, y + size - offset);
            cr.line_to(x + size - offset, y + offset);
            cr.line_to(x + offset, y + offset);
        }
        cr.stroke().ok();
    }

    // Bottom-left corner (need to adjust coordinates)
    if bottom_left {
        for i in 0..steps {
            let offset = i as f64 * step_size;
            cr.move_to(x + offset, y + offset);
            cr.line_to(x + offset, y + size - offset);
            cr.line_to(x + size - offset, y + size - offset);
        }
        cr.stroke().ok();
    }

    // Bottom-right corner
    if bottom_right {
        for i in 0..steps {
            let offset = i as f64 * step_size;
            cr.move_to(x + size - offset, y + offset);
            cr.line_to(x + size - offset, y + size - offset);
            cr.line_to(x + offset, y + size - offset);
        }
        cr.stroke().ok();
    }

    cr.restore().ok();
}

/// Draw a diamond shape
fn draw_diamond(cr: &Context, cx: f64, cy: f64, size: f64, color: &Color, filled: bool) {
    cr.save().ok();
    cr.set_source_rgba(color.r, color.g, color.b, color.a);

    cr.move_to(cx, cy - size);
    cr.line_to(cx + size, cy);
    cr.line_to(cx, cy + size);
    cr.line_to(cx - size, cy);
    cr.close_path();

    if filled {
        cr.fill().ok();
    } else {
        cr.stroke().ok();
    }
    cr.restore().ok();
}

/// Draw chevron/arrow pattern
fn draw_chevron(cr: &Context, x: f64, y: f64, width: f64, height: f64, color: &Color, line_width: f64) {
    cr.save().ok();
    cr.set_source_rgba(color.r, color.g, color.b, color.a);
    cr.set_line_width(line_width);

    let mid_x = x + width / 2.0;

    cr.move_to(x, y + height);
    cr.line_to(mid_x, y);
    cr.line_to(x + width, y + height);
    cr.stroke().ok();

    cr.restore().ok();
}

/// Draw vertical lines background pattern
fn draw_vertical_lines_pattern(cr: &Context, x: f64, y: f64, w: f64, h: f64, spacing: f64, color: &Color) {
    cr.save().ok();
    cr.set_source_rgba(color.r, color.g, color.b, color.a * 0.15);
    cr.set_line_width(1.0);

    let mut px = x + spacing;
    while px < x + w {
        cr.move_to(px, y);
        cr.line_to(px, y + h);
        px += spacing;
    }
    cr.stroke().ok();
    cr.restore().ok();
}

/// Draw diamond grid background pattern
fn draw_diamond_grid_pattern(cr: &Context, x: f64, y: f64, w: f64, h: f64, spacing: f64, color: &Color) {
    cr.save().ok();
    cr.set_source_rgba(color.r, color.g, color.b, color.a * 0.1);
    cr.set_line_width(0.5);

    // Draw diagonal lines in both directions
    let diagonal_spacing = spacing;

    // Top-left to bottom-right diagonals
    let mut start = x;
    while start < x + w + h {
        cr.move_to(start, y);
        cr.line_to(start - h, y + h);
        start += diagonal_spacing;
    }

    // Top-right to bottom-left diagonals
    start = x;
    while start < x + w + h {
        cr.move_to(start - h, y);
        cr.line_to(start, y + h);
        start += diagonal_spacing;
    }

    cr.stroke().ok();
    cr.restore().ok();
}

/// Draw sunburst background pattern from center
fn draw_sunburst_background(cr: &Context, x: f64, y: f64, w: f64, h: f64, rays: usize, color: &Color) {
    cr.save().ok();
    cr.rectangle(x, y, w, h);
    cr.clip();

    let cx = x + w / 2.0;
    let cy = y + h / 2.0;
    let max_radius = (w.powi(2) + h.powi(2)).sqrt();

    cr.set_source_rgba(color.r, color.g, color.b, color.a * 0.08);
    cr.set_line_width(1.0);

    let angle_step = std::f64::consts::TAU / rays as f64;
    for i in 0..rays {
        let angle = i as f64 * angle_step;
        cr.move_to(cx, cy);
        cr.line_to(cx + max_radius * angle.cos(), cy + max_radius * angle.sin());
    }
    cr.stroke().ok();

    cr.restore().ok();
}

/// Draw chevron background pattern
fn draw_chevron_background(cr: &Context, x: f64, y: f64, w: f64, h: f64, spacing: f64, color: &Color) {
    cr.save().ok();
    cr.rectangle(x, y, w, h);
    cr.clip();

    cr.set_source_rgba(color.r, color.g, color.b, color.a * 0.1);
    cr.set_line_width(1.0);

    let chevron_height = spacing;
    let mut py = y;
    while py < y + h + chevron_height {
        let mid_x = x + w / 2.0;
        cr.move_to(x, py + chevron_height);
        cr.line_to(mid_x, py);
        cr.line_to(x + w, py + chevron_height);
        py += chevron_height * 2.0;
    }
    cr.stroke().ok();

    cr.restore().ok();
}

/// Draw corner decorations based on style
fn draw_corner_decorations(cr: &Context, config: &ArtDecoFrameConfig, x: f64, y: f64, w: f64, h: f64) {
    let accent_color = config.accent_color.resolve(&config.theme);
    let size = config.corner_size;

    match config.corner_style {
        CornerStyle::Fan => {
            // Top-left
            draw_sunburst_corner(cr, x, y, size, 8, 0.0, std::f64::consts::FRAC_PI_2, &accent_color, config.accent_width);
            // Top-right
            draw_sunburst_corner(cr, x + w, y, size, 8, std::f64::consts::FRAC_PI_2, std::f64::consts::FRAC_PI_2, &accent_color, config.accent_width);
            // Bottom-right
            draw_sunburst_corner(cr, x + w, y + h, size, 8, std::f64::consts::PI, std::f64::consts::FRAC_PI_2, &accent_color, config.accent_width);
            // Bottom-left
            draw_sunburst_corner(cr, x, y + h, size, 8, -std::f64::consts::FRAC_PI_2, std::f64::consts::FRAC_PI_2, &accent_color, config.accent_width);
        }
        CornerStyle::Ziggurat => {
            // Draw stepped corners
            draw_ziggurat_corner(cr, x, y, size, 4, true, false, false, false, &accent_color, config.accent_width);
            draw_ziggurat_corner(cr, x + w - size, y, size, 4, false, true, false, false, &accent_color, config.accent_width);
            draw_ziggurat_corner(cr, x + w - size, y + h - size, size, 4, false, false, false, true, &accent_color, config.accent_width);
            draw_ziggurat_corner(cr, x, y + h - size, size, 4, false, false, true, false, &accent_color, config.accent_width);
        }
        CornerStyle::Diamond => {
            let diamond_size = size / 3.0;
            // Corner diamonds
            draw_diamond(cr, x + size / 2.0, y + size / 2.0, diamond_size, &accent_color, true);
            draw_diamond(cr, x + w - size / 2.0, y + size / 2.0, diamond_size, &accent_color, true);
            draw_diamond(cr, x + w - size / 2.0, y + h - size / 2.0, diamond_size, &accent_color, true);
            draw_diamond(cr, x + size / 2.0, y + h - size / 2.0, diamond_size, &accent_color, true);
        }
        CornerStyle::Bracket => {
            cr.save().ok();
            cr.set_source_rgba(accent_color.r, accent_color.g, accent_color.b, accent_color.a);
            cr.set_line_width(config.accent_width);

            // Top-left
            cr.move_to(x, y + size);
            cr.line_to(x, y);
            cr.line_to(x + size, y);
            cr.stroke().ok();

            // Top-right
            cr.move_to(x + w - size, y);
            cr.line_to(x + w, y);
            cr.line_to(x + w, y + size);
            cr.stroke().ok();

            // Bottom-right
            cr.move_to(x + w, y + h - size);
            cr.line_to(x + w, y + h);
            cr.line_to(x + w - size, y + h);
            cr.stroke().ok();

            // Bottom-left
            cr.move_to(x + size, y + h);
            cr.line_to(x, y + h);
            cr.line_to(x, y + h - size);
            cr.stroke().ok();

            cr.restore().ok();
        }
        CornerStyle::None => {}
    }
}

/// Draw border based on style
fn draw_border(cr: &Context, config: &ArtDecoFrameConfig, x: f64, y: f64, w: f64, h: f64) {
    let border_color = config.border_color.resolve(&config.theme);

    cr.save().ok();
    cr.set_source_rgba(border_color.r, border_color.g, border_color.b, border_color.a);
    cr.set_line_width(config.border_width);

    match config.border_style {
        BorderStyle::Sunburst => {
            // Simple rectangle with sunburst corners handled separately
            cr.rectangle(x, y, w, h);
            cr.stroke().ok();
        }
        BorderStyle::Chevron => {
            // Draw border with chevron accents at midpoints
            cr.rectangle(x, y, w, h);
            cr.stroke().ok();

            // Add small chevrons at edge midpoints
            let chevron_size = 8.0;
            draw_chevron(cr, x + w / 2.0 - chevron_size / 2.0, y - chevron_size / 2.0, chevron_size, chevron_size / 2.0, &border_color, config.border_width);
            draw_chevron(cr, x + w / 2.0 - chevron_size / 2.0, y + h - chevron_size / 2.0, chevron_size, chevron_size / 2.0, &border_color, config.border_width);
        }
        BorderStyle::Stepped => {
            // Draw stepped border with insets
            let step = 4.0;
            cr.rectangle(x, y, w, h);
            cr.stroke().ok();
            cr.rectangle(x + step, y + step, w - step * 2.0, h - step * 2.0);
            cr.stroke().ok();
        }
        BorderStyle::Geometric => {
            // Clean double-line border
            cr.rectangle(x, y, w, h);
            cr.stroke().ok();
            cr.set_line_width(config.border_width / 2.0);
            cr.rectangle(x + 4.0, y + 4.0, w - 8.0, h - 8.0);
            cr.stroke().ok();
        }
        BorderStyle::Ornate => {
            // Full ornate border with multiple elements
            cr.rectangle(x, y, w, h);
            cr.stroke().ok();

            // Inner decorative line
            cr.set_line_width(config.border_width / 2.0);
            cr.rectangle(x + 6.0, y + 6.0, w - 12.0, h - 12.0);
            cr.stroke().ok();

            // Diamond accents at midpoints
            let diamond_size = 4.0;
            draw_diamond(cr, x + w / 2.0, y, diamond_size, &border_color, true);
            draw_diamond(cr, x + w / 2.0, y + h, diamond_size, &border_color, true);
            draw_diamond(cr, x, y + h / 2.0, diamond_size, &border_color, true);
            draw_diamond(cr, x + w, y + h / 2.0, diamond_size, &border_color, true);
        }
    }

    cr.restore().ok();
}

/// Draw background pattern
fn draw_background_pattern(cr: &Context, config: &ArtDecoFrameConfig, x: f64, y: f64, w: f64, h: f64) {
    let pattern_color = config.pattern_color.resolve(&config.theme);

    match config.background_pattern {
        BackgroundPattern::Solid => {} // No pattern
        BackgroundPattern::VerticalLines => {
            draw_vertical_lines_pattern(cr, x, y, w, h, config.pattern_spacing, &pattern_color);
        }
        BackgroundPattern::DiamondGrid => {
            draw_diamond_grid_pattern(cr, x, y, w, h, config.pattern_spacing, &pattern_color);
        }
        BackgroundPattern::Sunburst => {
            draw_sunburst_background(cr, x, y, w, h, config.sunburst_rays, &pattern_color);
        }
        BackgroundPattern::Chevrons => {
            draw_chevron_background(cr, x, y, w, h, config.pattern_spacing, &pattern_color);
        }
    }
}

/// Draw the header
fn draw_header(cr: &Context, config: &ArtDecoFrameConfig, x: f64, y: f64, w: f64) -> f64 {
    if !config.show_header || config.header_text.is_empty() {
        return 0.0;
    }

    let (font_family, font_size) = config.header_font.resolve(&config.theme);
    let header_color = config.header_color.resolve(&config.theme);
    let accent_color = config.accent_color.resolve(&config.theme);

    let header_height = font_size + 20.0;
    let padding = 12.0;

    cr.save().ok();

    cr.select_font_face(
        &font_family,
        cairo::FontSlant::Normal,
        cairo::FontWeight::Bold,
    );
    cr.set_font_size(font_size);

    let text_extents = cr.text_extents(&config.header_text).ok();
    let (text_width, text_height) = text_extents.map(|e| (e.width(), e.height())).unwrap_or((0.0, 0.0));
    let text_x = x + (w - text_width) / 2.0;
    let text_y = y + header_height / 2.0 + text_height / 2.0;

    match config.header_style {
        HeaderStyle::Centered => {
            // Draw decorative lines on sides
            cr.set_source_rgba(accent_color.r, accent_color.g, accent_color.b, accent_color.a);
            cr.set_line_width(2.0);

            // Left side decoration
            let left_end = text_x - 20.0;
            if left_end > x + padding {
                cr.move_to(x + padding, y + header_height / 2.0);
                cr.line_to(left_end - 10.0, y + header_height / 2.0);
                cr.stroke().ok();
                draw_diamond(cr, left_end - 5.0, y + header_height / 2.0, 4.0, &accent_color, true);
            }

            // Right side decoration
            let right_start = text_x + text_width + 20.0;
            if right_start < x + w - padding {
                draw_diamond(cr, right_start + 5.0, y + header_height / 2.0, 4.0, &accent_color, true);
                cr.move_to(right_start + 10.0, y + header_height / 2.0);
                cr.line_to(x + w - padding, y + header_height / 2.0);
                cr.stroke().ok();
            }
        }
        HeaderStyle::Banner => {
            // Full-width banner background
            cr.set_source_rgba(accent_color.r, accent_color.g, accent_color.b, 0.2);
            cr.rectangle(x, y, w, header_height);
            cr.fill().ok();

            // Banner border
            cr.set_source_rgba(accent_color.r, accent_color.g, accent_color.b, accent_color.a);
            cr.set_line_width(1.0);
            cr.move_to(x, y + header_height);
            cr.line_to(x + w, y + header_height);
            cr.stroke().ok();
        }
        HeaderStyle::Stepped => {
            // Stepped header with tiered effect
            let step_height = header_height / 3.0;
            cr.set_source_rgba(accent_color.r, accent_color.g, accent_color.b, 0.15);

            // Three tiers getting narrower
            for i in 0..3 {
                let tier_inset = i as f64 * 20.0;
                let tier_y = y + i as f64 * step_height;
                cr.rectangle(x + tier_inset, tier_y, w - tier_inset * 2.0, step_height);
            }
            cr.fill().ok();
        }
        HeaderStyle::None => {}
    }

    // Draw header text
    cr.set_source_rgba(header_color.r, header_color.g, header_color.b, header_color.a);
    cr.move_to(text_x, text_y);
    cr.show_text(&config.header_text).ok();

    cr.restore().ok();

    header_height
}

/// Draw a divider between content groups
fn draw_divider(cr: &Context, config: &ArtDecoFrameConfig, x: f64, y: f64, length: f64, horizontal: bool) {
    if matches!(config.divider_style, DividerStyle::None) {
        return;
    }

    let divider_color = config.divider_color.resolve(&config.theme);

    cr.save().ok();
    cr.set_source_rgba(divider_color.r, divider_color.g, divider_color.b, divider_color.a);
    cr.set_line_width(config.divider_width);

    match config.divider_style {
        DividerStyle::Chevron => {
            let chevron_count = (length / 16.0).floor() as usize;
            let chevron_width = length / chevron_count as f64;

            if horizontal {
                for i in 0..chevron_count {
                    let cx = x + i as f64 * chevron_width + chevron_width / 2.0;
                    draw_chevron(cr, cx - 4.0, y - 3.0, 8.0, 6.0, &divider_color, config.divider_width);
                }
            } else {
                // Rotated chevrons for vertical divider
                for i in 0..chevron_count {
                    let cy = y + i as f64 * chevron_width + chevron_width / 2.0;
                    cr.move_to(x - 3.0, cy);
                    cr.line_to(x + 3.0, cy - 4.0);
                    cr.line_to(x + 3.0, cy + 4.0);
                    cr.close_path();
                    cr.stroke().ok();
                }
            }
        }
        DividerStyle::DoubleLine => {
            let gap = 4.0;
            if horizontal {
                cr.move_to(x, y - gap / 2.0);
                cr.line_to(x + length, y - gap / 2.0);
                cr.move_to(x, y + gap / 2.0);
                cr.line_to(x + length, y + gap / 2.0);
                cr.stroke().ok();

                // Center diamond
                draw_diamond(cr, x + length / 2.0, y, 5.0, &divider_color, true);
            } else {
                cr.move_to(x - gap / 2.0, y);
                cr.line_to(x - gap / 2.0, y + length);
                cr.move_to(x + gap / 2.0, y);
                cr.line_to(x + gap / 2.0, y + length);
                cr.stroke().ok();

                draw_diamond(cr, x, y + length / 2.0, 5.0, &divider_color, true);
            }
        }
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
        DividerStyle::Stepped => {
            let step_count = 5;
            let step_size = length / step_count as f64;

            if horizontal {
                for i in 0..step_count {
                    let sx = x + i as f64 * step_size;
                    let offset = if i % 2 == 0 { -2.0 } else { 2.0 };
                    cr.move_to(sx, y + offset);
                    cr.line_to(sx + step_size, y + offset);
                }
            } else {
                for i in 0..step_count {
                    let sy = y + i as f64 * step_size;
                    let offset = if i % 2 == 0 { -2.0 } else { 2.0 };
                    cr.move_to(x + offset, sy);
                    cr.line_to(x + offset, sy + step_size);
                }
            }
            cr.stroke().ok();
        }
        DividerStyle::None => {}
    }

    cr.restore().ok();
}

/// Render the complete Art Deco frame
/// Returns the content area bounds (x, y, width, height)
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
    let divider_space = divider_count as f64 * (config.divider_width + config.divider_padding * 2.0);

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
    config: &ArtDecoFrameConfig,
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
