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

use crate::displayers::combo_displayer_base::{ComboFrameConfig, FrameRenderer};
use crate::ui::background::Color;
use crate::ui::combo_config_base::{LayoutFrameConfig, ThemedFrameConfig};
use crate::ui::lcars_display::{ContentItemConfig, SplitOrientation};
use crate::ui::pango_text::{pango_show_text, pango_text_extents};
use crate::ui::theme::{
    deserialize_color_or_source, deserialize_font_or_source, ColorSource, ComboThemeConfig,
    FontSource,
};

// Re-export types we use
pub use crate::ui::lcars_display::{
    ContentDisplayType as ArtDecoContentType, ContentItemConfig as ArtDecoContentItemConfig,
};

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
    /// Hexagon medallion with extending lines
    Hexagon,
    /// Octagon medallion with extending lines
    Octagon,
    /// Circle medallion with extending lines
    Circle,
    /// Double-line L bracket with inner step
    DoubleBracket,
    /// Stacked geometric shapes (diamond, circle, lines)
    GeometricStack,
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
    /// Stacked/overlapping diamond cluster
    DiamondCluster,
    /// Crescent moon with dots
    Crescent,
    /// Arrows pointing to central diamond
    ArrowDiamond,
    /// Three circles connected by lines
    CircleChain,
    /// Crossed/woven lines pattern
    CrossedLines,
    /// Fleur-de-lis / leaf ornament
    FleurDeLis,
    /// Zigzag heartbeat pattern with diamond accents
    Heartbeat,
    /// Interlocked diamond grid pattern
    DiamondGrid,
    /// No divider
    None,
}

fn default_border_width() -> f64 {
    3.0
}
fn default_corner_size() -> f64 {
    24.0
}
fn default_accent_width() -> f64 {
    2.0
}
fn default_pattern_spacing() -> f64 {
    16.0
}
fn default_content_padding() -> f64 {
    12.0
}
fn default_divider_width() -> f64 {
    2.0
}
fn default_divider_padding() -> f64 {
    6.0
}
fn default_group_count() -> usize {
    2
}
fn default_sunburst_rays() -> usize {
    12
}

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
    #[serde(
        default = "default_border_color_source",
        deserialize_with = "deserialize_color_or_source"
    )]
    pub border_color: ColorSource,

    // Corner decorations
    #[serde(default)]
    pub corner_style: CornerStyle,
    #[serde(default = "default_corner_size")]
    pub corner_size: f64,
    #[serde(
        default = "default_accent_color_source",
        deserialize_with = "deserialize_color_or_source"
    )]
    pub accent_color: ColorSource,
    #[serde(default = "default_accent_width")]
    pub accent_width: f64,

    // Background
    #[serde(
        default = "default_background_color_source",
        deserialize_with = "deserialize_color_or_source"
    )]
    pub background_color: ColorSource,
    #[serde(default)]
    pub background_pattern: BackgroundPattern,
    #[serde(
        default = "default_pattern_color_source",
        deserialize_with = "deserialize_color_or_source"
    )]
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
    #[serde(
        default = "default_header_font_source",
        deserialize_with = "deserialize_font_or_source"
    )]
    pub header_font: FontSource,
    #[serde(
        default = "default_header_color_source",
        deserialize_with = "deserialize_color_or_source"
    )]
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
    /// Item orientation within each group - defaults to same as split_orientation
    #[serde(default)]
    pub group_item_orientations: Vec<SplitOrientation>,

    // Dividers
    #[serde(default)]
    pub divider_style: DividerStyle,
    #[serde(
        default = "default_divider_color_source",
        deserialize_with = "deserialize_color_or_source"
    )]
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

    /// Animation enabled
    #[serde(default = "default_animation_enabled")]
    pub animation_enabled: bool,

    /// Animation speed
    #[serde(default = "default_animation_speed")]
    pub animation_speed: f64,
}

fn default_animation_enabled() -> bool {
    true
}

fn default_animation_speed() -> f64 {
    8.0
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
            group_item_orientations: Vec::new(),
            divider_style: DividerStyle::default(),
            divider_color: default_divider_color_source(),
            divider_width: default_divider_width(),
            divider_padding: default_divider_padding(),
            content_items: HashMap::new(),
            theme: default_art_deco_theme(),
            animation_enabled: default_animation_enabled(),
            animation_speed: default_animation_speed(),
        }
    }
}

impl LayoutFrameConfig for ArtDecoFrameConfig {
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

impl ThemedFrameConfig for ArtDecoFrameConfig {
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

impl ComboFrameConfig for ArtDecoFrameConfig {
    fn animation_enabled(&self) -> bool {
        self.animation_enabled
    }

    fn set_animation_enabled(&mut self, enabled: bool) {
        self.animation_enabled = enabled;
    }

    fn animation_speed(&self) -> f64 {
        self.animation_speed
    }

    fn set_animation_speed(&mut self, speed: f64) {
        self.animation_speed = speed;
    }

    fn group_item_counts(&self) -> &[usize] {
        &self.group_item_counts
    }

    fn group_item_counts_mut(&mut self) -> &mut Vec<usize> {
        &mut self.group_item_counts
    }
}

/// Frame renderer for Art Deco theme
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

/// Draw an Art Deco stepped/ziggurat corner decoration with stair-step pattern
fn draw_art_deco_ziggurat_corner(
    cr: &Context,
    corner_x: f64,
    corner_y: f64,
    size: f64,
    flip_h: bool,
    flip_v: bool,
    color: &Color,
    line_width: f64,
) {
    cr.save().ok();
    cr.set_source_rgba(color.r, color.g, color.b, color.a);
    cr.set_line_width(line_width);

    let h_dir = if flip_h { -1.0 } else { 1.0 };
    let v_dir = if flip_v { -1.0 } else { 1.0 };

    let steps = 4;
    let step_size = size / (steps as f64 + 1.0);

    // Draw stair-step pattern
    // Start from outer corner, step inward diagonally
    cr.move_to(corner_x, corner_y + v_dir * size);

    for i in 0..steps {
        let step_x = corner_x + h_dir * (i as f64 * step_size);
        let step_y = corner_y + v_dir * ((steps - i) as f64 * step_size);
        let next_step_x = corner_x + h_dir * ((i + 1) as f64 * step_size);

        cr.line_to(step_x, step_y);
        cr.line_to(next_step_x, step_y);
    }

    // Final segment to end
    cr.line_to(corner_x + h_dir * size, corner_y);
    cr.stroke().ok();

    // Draw a second inner stair pattern for more detail
    let inner_offset = step_size * 0.5;
    cr.move_to(
        corner_x + h_dir * inner_offset,
        corner_y + v_dir * (size - inner_offset),
    );

    for i in 0..(steps - 1) {
        let step_x = corner_x + h_dir * (inner_offset + i as f64 * step_size);
        let step_y = corner_y + v_dir * ((steps - 1 - i) as f64 * step_size);
        let next_step_x = corner_x + h_dir * (inner_offset + (i + 1) as f64 * step_size);

        cr.line_to(step_x, step_y);
        cr.line_to(next_step_x, step_y);
    }

    cr.line_to(
        corner_x + h_dir * (size - inner_offset),
        corner_y + v_dir * inner_offset,
    );
    cr.stroke().ok();

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

/// Draw Art Deco L-bracket corner with hexagon medallion
/// This draws an L-shaped bracket with multiple parallel lines and a hexagon at the corner
fn draw_art_deco_hexagon_corner(
    cr: &Context,
    corner_x: f64,
    corner_y: f64,
    size: f64,
    flip_h: bool,
    flip_v: bool,
    color: &Color,
    line_width: f64,
) {
    cr.save().ok();
    cr.set_source_rgba(color.r, color.g, color.b, color.a);
    cr.set_line_width(line_width);

    let h_dir = if flip_h { -1.0 } else { 1.0 };
    let v_dir = if flip_v { -1.0 } else { 1.0 };

    let hex_size = size * 0.22;
    let hex_cx = corner_x + h_dir * size * 0.35;
    let hex_cy = corner_y + v_dir * size * 0.35;
    let line_spacing = size * 0.08;

    // Draw outer hexagon (flat-topped)
    cr.new_path();
    for i in 0..6 {
        let angle = std::f64::consts::PI / 6.0 + i as f64 * std::f64::consts::PI / 3.0;
        let px = hex_cx + hex_size * angle.cos();
        let py = hex_cy + hex_size * angle.sin();
        if i == 0 {
            cr.move_to(px, py);
        } else {
            cr.line_to(px, py);
        }
    }
    cr.close_path();
    cr.stroke().ok();

    // Draw inner hexagon
    let inner_hex = hex_size * 0.55;
    cr.new_path();
    for i in 0..6 {
        let angle = std::f64::consts::PI / 6.0 + i as f64 * std::f64::consts::PI / 3.0;
        let px = hex_cx + inner_hex * angle.cos();
        let py = hex_cy + inner_hex * angle.sin();
        if i == 0 {
            cr.move_to(px, py);
        } else {
            cr.line_to(px, py);
        }
    }
    cr.close_path();
    cr.stroke().ok();

    // Draw horizontal lines extending from corner
    for i in 0..3 {
        let y_offset = corner_y + v_dir * (i as f64 * line_spacing);
        cr.move_to(corner_x, y_offset);
        cr.line_to(corner_x + h_dir * size, y_offset);
    }
    cr.stroke().ok();

    // Draw vertical lines extending from corner
    for i in 0..3 {
        let x_offset = corner_x + h_dir * (i as f64 * line_spacing);
        cr.move_to(x_offset, corner_y);
        cr.line_to(x_offset, corner_y + v_dir * size);
    }
    cr.stroke().ok();

    cr.restore().ok();
}

/// Draw Art Deco L-bracket corner with octagon medallion
fn draw_art_deco_octagon_corner(
    cr: &Context,
    corner_x: f64,
    corner_y: f64,
    size: f64,
    flip_h: bool,
    flip_v: bool,
    color: &Color,
    line_width: f64,
) {
    cr.save().ok();
    cr.set_source_rgba(color.r, color.g, color.b, color.a);
    cr.set_line_width(line_width);

    let h_dir = if flip_h { -1.0 } else { 1.0 };
    let v_dir = if flip_v { -1.0 } else { 1.0 };

    let oct_size = size * 0.22;
    let oct_cx = corner_x + h_dir * size * 0.35;
    let oct_cy = corner_y + v_dir * size * 0.35;
    let line_spacing = size * 0.08;

    // Draw octagon
    cr.new_path();
    for i in 0..8 {
        let angle = std::f64::consts::PI / 8.0 + i as f64 * std::f64::consts::PI / 4.0;
        let px = oct_cx + oct_size * angle.cos();
        let py = oct_cy + oct_size * angle.sin();
        if i == 0 {
            cr.move_to(px, py);
        } else {
            cr.line_to(px, py);
        }
    }
    cr.close_path();
    cr.stroke().ok();

    // Draw horizontal lines extending from corner
    for i in 0..3 {
        let y_offset = corner_y + v_dir * (i as f64 * line_spacing);
        cr.move_to(corner_x, y_offset);
        cr.line_to(corner_x + h_dir * size, y_offset);
    }
    cr.stroke().ok();

    // Draw vertical lines extending from corner
    for i in 0..3 {
        let x_offset = corner_x + h_dir * (i as f64 * line_spacing);
        cr.move_to(x_offset, corner_y);
        cr.line_to(x_offset, corner_y + v_dir * size);
    }
    cr.stroke().ok();

    cr.restore().ok();
}

/// Draw Art Deco L-bracket corner with concentric circles
fn draw_art_deco_circle_corner(
    cr: &Context,
    corner_x: f64,
    corner_y: f64,
    size: f64,
    flip_h: bool,
    flip_v: bool,
    color: &Color,
    line_width: f64,
) {
    cr.save().ok();
    cr.set_source_rgba(color.r, color.g, color.b, color.a);
    cr.set_line_width(line_width);

    let h_dir = if flip_h { -1.0 } else { 1.0 };
    let v_dir = if flip_v { -1.0 } else { 1.0 };

    let circle_radius = size * 0.18;
    let circle_cx = corner_x + h_dir * size * 0.35;
    let circle_cy = corner_y + v_dir * size * 0.35;
    let line_spacing = size * 0.08;

    // Draw concentric circles
    cr.arc(
        circle_cx,
        circle_cy,
        circle_radius,
        0.0,
        std::f64::consts::TAU,
    );
    cr.stroke().ok();
    cr.arc(
        circle_cx,
        circle_cy,
        circle_radius * 0.6,
        0.0,
        std::f64::consts::TAU,
    );
    cr.stroke().ok();
    // Center dot
    cr.arc(
        circle_cx,
        circle_cy,
        line_width * 1.5,
        0.0,
        std::f64::consts::TAU,
    );
    cr.fill().ok();

    // Draw horizontal lines extending from corner
    for i in 0..3 {
        let y_offset = corner_y + v_dir * (i as f64 * line_spacing);
        cr.move_to(corner_x, y_offset);
        cr.line_to(corner_x + h_dir * size, y_offset);
    }
    cr.stroke().ok();

    // Draw vertical lines extending from corner
    for i in 0..3 {
        let x_offset = corner_x + h_dir * (i as f64 * line_spacing);
        cr.move_to(x_offset, corner_y);
        cr.line_to(x_offset, corner_y + v_dir * size);
    }
    cr.stroke().ok();

    cr.restore().ok();
}

/// Draw Art Deco L-bracket corner with stacked diamonds
fn draw_art_deco_diamond_corner(
    cr: &Context,
    corner_x: f64,
    corner_y: f64,
    size: f64,
    flip_h: bool,
    flip_v: bool,
    color: &Color,
    line_width: f64,
) {
    cr.save().ok();
    cr.set_source_rgba(color.r, color.g, color.b, color.a);
    cr.set_line_width(line_width);

    let h_dir = if flip_h { -1.0 } else { 1.0 };
    let v_dir = if flip_v { -1.0 } else { 1.0 };

    let diamond_size = size * 0.15;
    let diamond_cx = corner_x + h_dir * size * 0.35;
    let diamond_cy = corner_y + v_dir * size * 0.35;
    let line_spacing = size * 0.08;

    // Draw stacked diamonds (large outer, smaller inner)
    // Outer diamond
    cr.move_to(diamond_cx, diamond_cy - diamond_size);
    cr.line_to(diamond_cx + diamond_size, diamond_cy);
    cr.line_to(diamond_cx, diamond_cy + diamond_size);
    cr.line_to(diamond_cx - diamond_size, diamond_cy);
    cr.close_path();
    cr.stroke().ok();

    // Inner diamond
    let inner_size = diamond_size * 0.5;
    cr.move_to(diamond_cx, diamond_cy - inner_size);
    cr.line_to(diamond_cx + inner_size, diamond_cy);
    cr.line_to(diamond_cx, diamond_cy + inner_size);
    cr.line_to(diamond_cx - inner_size, diamond_cy);
    cr.close_path();
    cr.stroke().ok();

    // Small diamonds at the tips
    let tip_size = diamond_size * 0.3;
    let tip_dist = diamond_size + tip_size + 2.0;

    // Diamond above/below
    cr.move_to(diamond_cx, diamond_cy - tip_dist);
    cr.line_to(diamond_cx + tip_size, diamond_cy - tip_dist + tip_size);
    cr.line_to(diamond_cx, diamond_cy - tip_dist + tip_size * 2.0);
    cr.line_to(diamond_cx - tip_size, diamond_cy - tip_dist + tip_size);
    cr.close_path();
    cr.fill().ok();

    cr.move_to(diamond_cx, diamond_cy + tip_dist);
    cr.line_to(diamond_cx + tip_size, diamond_cy + tip_dist - tip_size);
    cr.line_to(diamond_cx, diamond_cy + tip_dist - tip_size * 2.0);
    cr.line_to(diamond_cx - tip_size, diamond_cy + tip_dist - tip_size);
    cr.close_path();
    cr.fill().ok();

    // Draw horizontal lines extending from corner
    for i in 0..3 {
        let y_offset = corner_y + v_dir * (i as f64 * line_spacing);
        cr.move_to(corner_x, y_offset);
        cr.line_to(corner_x + h_dir * size, y_offset);
    }
    cr.stroke().ok();

    // Draw vertical lines extending from corner
    for i in 0..3 {
        let x_offset = corner_x + h_dir * (i as f64 * line_spacing);
        cr.move_to(x_offset, corner_y);
        cr.line_to(x_offset, corner_y + v_dir * size);
    }
    cr.stroke().ok();

    cr.restore().ok();
}

/// Draw Art Deco double-line L bracket corner
fn draw_art_deco_double_bracket(
    cr: &Context,
    corner_x: f64,
    corner_y: f64,
    size: f64,
    flip_h: bool,
    flip_v: bool,
    color: &Color,
    line_width: f64,
) {
    cr.save().ok();
    cr.set_source_rgba(color.r, color.g, color.b, color.a);
    cr.set_line_width(line_width);

    let h_dir = if flip_h { -1.0 } else { 1.0 };
    let v_dir = if flip_v { -1.0 } else { 1.0 };

    let line_spacing = size * 0.12;

    // Draw outer L bracket
    cr.move_to(corner_x + h_dir * size, corner_y);
    cr.line_to(corner_x, corner_y);
    cr.line_to(corner_x, corner_y + v_dir * size);
    cr.stroke().ok();

    // Draw inner L bracket (offset)
    cr.move_to(
        corner_x + h_dir * (size - line_spacing * 2.0),
        corner_y + v_dir * line_spacing,
    );
    cr.line_to(
        corner_x + h_dir * line_spacing,
        corner_y + v_dir * line_spacing,
    );
    cr.line_to(
        corner_x + h_dir * line_spacing,
        corner_y + v_dir * (size - line_spacing * 2.0),
    );
    cr.stroke().ok();

    cr.restore().ok();
}

/// Draw Art Deco geometric stack corner (nested squares with lines)
fn draw_art_deco_geometric_stack(
    cr: &Context,
    corner_x: f64,
    corner_y: f64,
    size: f64,
    flip_h: bool,
    flip_v: bool,
    color: &Color,
    line_width: f64,
) {
    cr.save().ok();
    cr.set_source_rgba(color.r, color.g, color.b, color.a);
    cr.set_line_width(line_width);

    let h_dir = if flip_h { -1.0 } else { 1.0 };
    let v_dir = if flip_v { -1.0 } else { 1.0 };

    let sq_size = size * 0.18;
    let sq_cx = corner_x + h_dir * size * 0.35;
    let sq_cy = corner_y + v_dir * size * 0.35;
    let line_spacing = size * 0.08;

    // Draw outer square (rotated 45 degrees - diamond orientation)
    cr.move_to(sq_cx, sq_cy - sq_size);
    cr.line_to(sq_cx + sq_size, sq_cy);
    cr.line_to(sq_cx, sq_cy + sq_size);
    cr.line_to(sq_cx - sq_size, sq_cy);
    cr.close_path();
    cr.stroke().ok();

    // Draw inner square
    let inner_sq = sq_size * 0.55;
    cr.move_to(sq_cx, sq_cy - inner_sq);
    cr.line_to(sq_cx + inner_sq, sq_cy);
    cr.line_to(sq_cx, sq_cy + inner_sq);
    cr.line_to(sq_cx - inner_sq, sq_cy);
    cr.close_path();
    cr.stroke().ok();

    // Draw horizontal lines extending from corner
    for i in 0..3 {
        let y_offset = corner_y + v_dir * (i as f64 * line_spacing);
        cr.move_to(corner_x, y_offset);
        cr.line_to(corner_x + h_dir * size, y_offset);
    }
    cr.stroke().ok();

    // Draw vertical lines extending from corner
    for i in 0..3 {
        let x_offset = corner_x + h_dir * (i as f64 * line_spacing);
        cr.move_to(x_offset, corner_y);
        cr.line_to(x_offset, corner_y + v_dir * size);
    }
    cr.stroke().ok();

    cr.restore().ok();
}

/// Draw chevron/arrow pattern
fn draw_chevron(
    cr: &Context,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    color: &Color,
    line_width: f64,
) {
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
fn draw_vertical_lines_pattern(
    cr: &Context,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    spacing: f64,
    color: &Color,
) {
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
fn draw_diamond_grid_pattern(
    cr: &Context,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    spacing: f64,
    color: &Color,
) {
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
fn draw_sunburst_background(
    cr: &Context,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    rays: usize,
    color: &Color,
) {
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
fn draw_chevron_background(
    cr: &Context,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    spacing: f64,
    color: &Color,
) {
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
fn draw_corner_decorations(
    cr: &Context,
    config: &ArtDecoFrameConfig,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) {
    let accent_color = config.accent_color.resolve(&config.theme);
    let size = config.corner_size;

    match config.corner_style {
        CornerStyle::Fan => {
            // Top-left
            draw_sunburst_corner(
                cr,
                x,
                y,
                size,
                8,
                0.0,
                std::f64::consts::FRAC_PI_2,
                &accent_color,
                config.accent_width,
            );
            // Top-right
            draw_sunburst_corner(
                cr,
                x + w,
                y,
                size,
                8,
                std::f64::consts::FRAC_PI_2,
                std::f64::consts::FRAC_PI_2,
                &accent_color,
                config.accent_width,
            );
            // Bottom-right
            draw_sunburst_corner(
                cr,
                x + w,
                y + h,
                size,
                8,
                std::f64::consts::PI,
                std::f64::consts::FRAC_PI_2,
                &accent_color,
                config.accent_width,
            );
            // Bottom-left
            draw_sunburst_corner(
                cr,
                x,
                y + h,
                size,
                8,
                -std::f64::consts::FRAC_PI_2,
                std::f64::consts::FRAC_PI_2,
                &accent_color,
                config.accent_width,
            );
        }
        CornerStyle::Ziggurat => {
            // Top-left: stair steps going right and down
            draw_art_deco_ziggurat_corner(
                cr,
                x,
                y,
                size,
                false,
                false,
                &accent_color,
                config.accent_width,
            );
            // Top-right: stair steps going left and down
            draw_art_deco_ziggurat_corner(
                cr,
                x + w,
                y,
                size,
                true,
                false,
                &accent_color,
                config.accent_width,
            );
            // Bottom-right: stair steps going left and up
            draw_art_deco_ziggurat_corner(
                cr,
                x + w,
                y + h,
                size,
                true,
                true,
                &accent_color,
                config.accent_width,
            );
            // Bottom-left: stair steps going right and up
            draw_art_deco_ziggurat_corner(
                cr,
                x,
                y + h,
                size,
                false,
                true,
                &accent_color,
                config.accent_width,
            );
        }
        CornerStyle::Diamond => {
            // Top-left
            draw_art_deco_diamond_corner(
                cr,
                x,
                y,
                size,
                false,
                false,
                &accent_color,
                config.accent_width,
            );
            // Top-right
            draw_art_deco_diamond_corner(
                cr,
                x + w,
                y,
                size,
                true,
                false,
                &accent_color,
                config.accent_width,
            );
            // Bottom-right
            draw_art_deco_diamond_corner(
                cr,
                x + w,
                y + h,
                size,
                true,
                true,
                &accent_color,
                config.accent_width,
            );
            // Bottom-left
            draw_art_deco_diamond_corner(
                cr,
                x,
                y + h,
                size,
                false,
                true,
                &accent_color,
                config.accent_width,
            );
        }
        CornerStyle::Bracket => {
            cr.save().ok();
            cr.set_source_rgba(
                accent_color.r,
                accent_color.g,
                accent_color.b,
                accent_color.a,
            );
            cr.set_line_width(config.accent_width);

            let line_spacing = size * 0.12;
            let num_lines = 4;

            // Top-left - multiple parallel L-brackets
            for i in 0..num_lines {
                let offset = i as f64 * line_spacing;
                let arm_len = size - offset * 1.5;
                if arm_len > 0.0 {
                    cr.move_to(x + offset, y + arm_len);
                    cr.line_to(x + offset, y + offset);
                    cr.line_to(x + arm_len, y + offset);
                }
            }
            cr.stroke().ok();

            // Top-right
            for i in 0..num_lines {
                let offset = i as f64 * line_spacing;
                let arm_len = size - offset * 1.5;
                if arm_len > 0.0 {
                    cr.move_to(x + w - arm_len, y + offset);
                    cr.line_to(x + w - offset, y + offset);
                    cr.line_to(x + w - offset, y + arm_len);
                }
            }
            cr.stroke().ok();

            // Bottom-right
            for i in 0..num_lines {
                let offset = i as f64 * line_spacing;
                let arm_len = size - offset * 1.5;
                if arm_len > 0.0 {
                    cr.move_to(x + w - offset, y + h - arm_len);
                    cr.line_to(x + w - offset, y + h - offset);
                    cr.line_to(x + w - arm_len, y + h - offset);
                }
            }
            cr.stroke().ok();

            // Bottom-left
            for i in 0..num_lines {
                let offset = i as f64 * line_spacing;
                let arm_len = size - offset * 1.5;
                if arm_len > 0.0 {
                    cr.move_to(x + arm_len, y + h - offset);
                    cr.line_to(x + offset, y + h - offset);
                    cr.line_to(x + offset, y + h - arm_len);
                }
            }
            cr.stroke().ok();

            cr.restore().ok();
        }
        CornerStyle::Hexagon => {
            // Top-left: lines go right and down
            draw_art_deco_hexagon_corner(
                cr,
                x,
                y,
                size,
                false,
                false,
                &accent_color,
                config.accent_width,
            );
            // Top-right: lines go left and down
            draw_art_deco_hexagon_corner(
                cr,
                x + w,
                y,
                size,
                true,
                false,
                &accent_color,
                config.accent_width,
            );
            // Bottom-right: lines go left and up
            draw_art_deco_hexagon_corner(
                cr,
                x + w,
                y + h,
                size,
                true,
                true,
                &accent_color,
                config.accent_width,
            );
            // Bottom-left: lines go right and up
            draw_art_deco_hexagon_corner(
                cr,
                x,
                y + h,
                size,
                false,
                true,
                &accent_color,
                config.accent_width,
            );
        }
        CornerStyle::Octagon => {
            // Top-left
            draw_art_deco_octagon_corner(
                cr,
                x,
                y,
                size,
                false,
                false,
                &accent_color,
                config.accent_width,
            );
            // Top-right
            draw_art_deco_octagon_corner(
                cr,
                x + w,
                y,
                size,
                true,
                false,
                &accent_color,
                config.accent_width,
            );
            // Bottom-right
            draw_art_deco_octagon_corner(
                cr,
                x + w,
                y + h,
                size,
                true,
                true,
                &accent_color,
                config.accent_width,
            );
            // Bottom-left
            draw_art_deco_octagon_corner(
                cr,
                x,
                y + h,
                size,
                false,
                true,
                &accent_color,
                config.accent_width,
            );
        }
        CornerStyle::Circle => {
            // Top-left
            draw_art_deco_circle_corner(
                cr,
                x,
                y,
                size,
                false,
                false,
                &accent_color,
                config.accent_width,
            );
            // Top-right
            draw_art_deco_circle_corner(
                cr,
                x + w,
                y,
                size,
                true,
                false,
                &accent_color,
                config.accent_width,
            );
            // Bottom-right
            draw_art_deco_circle_corner(
                cr,
                x + w,
                y + h,
                size,
                true,
                true,
                &accent_color,
                config.accent_width,
            );
            // Bottom-left
            draw_art_deco_circle_corner(
                cr,
                x,
                y + h,
                size,
                false,
                true,
                &accent_color,
                config.accent_width,
            );
        }
        CornerStyle::DoubleBracket => {
            // Top-left
            draw_art_deco_double_bracket(
                cr,
                x,
                y,
                size,
                false,
                false,
                &accent_color,
                config.accent_width,
            );
            // Top-right
            draw_art_deco_double_bracket(
                cr,
                x + w,
                y,
                size,
                true,
                false,
                &accent_color,
                config.accent_width,
            );
            // Bottom-right
            draw_art_deco_double_bracket(
                cr,
                x + w,
                y + h,
                size,
                true,
                true,
                &accent_color,
                config.accent_width,
            );
            // Bottom-left
            draw_art_deco_double_bracket(
                cr,
                x,
                y + h,
                size,
                false,
                true,
                &accent_color,
                config.accent_width,
            );
        }
        CornerStyle::GeometricStack => {
            // Top-left
            draw_art_deco_geometric_stack(
                cr,
                x,
                y,
                size,
                false,
                false,
                &accent_color,
                config.accent_width,
            );
            // Top-right
            draw_art_deco_geometric_stack(
                cr,
                x + w,
                y,
                size,
                true,
                false,
                &accent_color,
                config.accent_width,
            );
            // Bottom-right
            draw_art_deco_geometric_stack(
                cr,
                x + w,
                y + h,
                size,
                true,
                true,
                &accent_color,
                config.accent_width,
            );
            // Bottom-left
            draw_art_deco_geometric_stack(
                cr,
                x,
                y + h,
                size,
                false,
                true,
                &accent_color,
                config.accent_width,
            );
        }
        CornerStyle::None => {}
    }
}

/// Draw border based on style
fn draw_border(cr: &Context, config: &ArtDecoFrameConfig, x: f64, y: f64, w: f64, h: f64) {
    let border_color = config.border_color.resolve(&config.theme);

    cr.save().ok();
    cr.set_source_rgba(
        border_color.r,
        border_color.g,
        border_color.b,
        border_color.a,
    );
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
            draw_chevron(
                cr,
                x + w / 2.0 - chevron_size / 2.0,
                y - chevron_size / 2.0,
                chevron_size,
                chevron_size / 2.0,
                &border_color,
                config.border_width,
            );
            draw_chevron(
                cr,
                x + w / 2.0 - chevron_size / 2.0,
                y + h - chevron_size / 2.0,
                chevron_size,
                chevron_size / 2.0,
                &border_color,
                config.border_width,
            );
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
fn draw_background_pattern(
    cr: &Context,
    config: &ArtDecoFrameConfig,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) {
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
        HeaderStyle::Centered => {
            // Draw decorative lines on sides
            cr.set_source_rgba(
                accent_color.r,
                accent_color.g,
                accent_color.b,
                accent_color.a,
            );
            cr.set_line_width(2.0);

            // Left side decoration
            let left_end = text_x - 20.0;
            if left_end > x + padding {
                cr.move_to(x + padding, y + header_height / 2.0);
                cr.line_to(left_end - 10.0, y + header_height / 2.0);
                cr.stroke().ok();
                draw_diamond(
                    cr,
                    left_end - 5.0,
                    y + header_height / 2.0,
                    4.0,
                    &accent_color,
                    true,
                );
            }

            // Right side decoration
            let right_start = text_x + text_width + 20.0;
            if right_start < x + w - padding {
                draw_diamond(
                    cr,
                    right_start + 5.0,
                    y + header_height / 2.0,
                    4.0,
                    &accent_color,
                    true,
                );
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
            cr.set_source_rgba(
                accent_color.r,
                accent_color.g,
                accent_color.b,
                accent_color.a,
            );
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
    config: &ArtDecoFrameConfig,
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

    match config.divider_style {
        DividerStyle::Chevron => {
            let chevron_count = (length / 16.0).floor() as usize;
            let chevron_width = length / chevron_count as f64;

            if horizontal {
                for i in 0..chevron_count {
                    let cx = x + i as f64 * chevron_width + chevron_width / 2.0;
                    draw_chevron(
                        cr,
                        cx - 4.0,
                        y - 3.0,
                        8.0,
                        6.0,
                        &divider_color,
                        config.divider_width,
                    );
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
        DividerStyle::DiamondCluster => {
            let cx = if horizontal { x + length / 2.0 } else { x };
            let cy = if horizontal { y } else { y + length / 2.0 };
            let size = 8.0;

            // Draw stacked diamonds (3 overlapping)
            draw_diamond(cr, cx, cy, size, &divider_color, false);
            draw_diamond(cr, cx - size * 0.7, cy, size * 0.6, &divider_color, false);
            draw_diamond(cr, cx + size * 0.7, cy, size * 0.6, &divider_color, false);

            // Extending lines
            if horizontal {
                cr.move_to(x, y);
                cr.line_to(cx - size * 1.5, y);
                cr.move_to(cx + size * 1.5, y);
                cr.line_to(x + length, y);
            } else {
                cr.move_to(x, y);
                cr.line_to(x, cy - size * 1.5);
                cr.move_to(x, cy + size * 1.5);
                cr.line_to(x, y + length);
            }
            cr.stroke().ok();
        }
        DividerStyle::Crescent => {
            let cx = if horizontal { x + length / 2.0 } else { x };
            let cy = if horizontal { y } else { y + length / 2.0 };
            let radius = 6.0;

            // Draw crescent moon
            cr.arc(cx, cy, radius, 0.2, std::f64::consts::PI - 0.2);
            cr.stroke().ok();

            // Inner arc to create crescent effect
            cr.arc(cx, cy - 2.0, radius * 0.7, 0.3, std::f64::consts::PI - 0.3);
            cr.stroke().ok();

            // Dots on sides
            let dot_dist = radius + 8.0;
            cr.arc(cx - dot_dist, cy, 2.0, 0.0, std::f64::consts::TAU);
            cr.fill().ok();
            cr.arc(cx + dot_dist, cy, 2.0, 0.0, std::f64::consts::TAU);
            cr.fill().ok();

            // Extending lines
            if horizontal {
                cr.move_to(x, y);
                cr.line_to(cx - dot_dist - 6.0, y);
                cr.move_to(cx + dot_dist + 6.0, y);
                cr.line_to(x + length, y);
            } else {
                cr.move_to(x, y);
                cr.line_to(x, cy - dot_dist - 6.0);
                cr.move_to(x, cy + dot_dist + 6.0);
                cr.line_to(x, y + length);
            }
            cr.stroke().ok();
        }
        DividerStyle::ArrowDiamond => {
            let cx = if horizontal { x + length / 2.0 } else { x };
            let cy = if horizontal { y } else { y + length / 2.0 };
            let size = 6.0;

            // Center diamond
            draw_diamond(cr, cx, cy, size, &divider_color, false);

            if horizontal {
                // Left arrow
                cr.move_to(x, y);
                cr.line_to(cx - size - 10.0, y);
                cr.line_to(cx - size - 5.0, y - 4.0);
                cr.move_to(cx - size - 10.0, y);
                cr.line_to(cx - size - 5.0, y + 4.0);

                // Right arrow
                cr.move_to(x + length, y);
                cr.line_to(cx + size + 10.0, y);
                cr.line_to(cx + size + 5.0, y - 4.0);
                cr.move_to(cx + size + 10.0, y);
                cr.line_to(cx + size + 5.0, y + 4.0);
            } else {
                // Top arrow
                cr.move_to(x, y);
                cr.line_to(x, cy - size - 10.0);
                cr.line_to(x - 4.0, cy - size - 5.0);
                cr.move_to(x, cy - size - 10.0);
                cr.line_to(x + 4.0, cy - size - 5.0);

                // Bottom arrow
                cr.move_to(x, y + length);
                cr.line_to(x, cy + size + 10.0);
                cr.line_to(x - 4.0, cy + size + 5.0);
                cr.move_to(x, cy + size + 10.0);
                cr.line_to(x + 4.0, cy + size + 5.0);
            }
            cr.stroke().ok();
        }
        DividerStyle::CircleChain => {
            let cx = if horizontal { x + length / 2.0 } else { x };
            let cy = if horizontal { y } else { y + length / 2.0 };
            let radius = 4.0;
            let spacing = 12.0;

            // Three circles
            cr.arc(cx - spacing, cy, radius, 0.0, std::f64::consts::TAU);
            cr.stroke().ok();
            cr.arc(cx, cy, radius, 0.0, std::f64::consts::TAU);
            cr.stroke().ok();
            cr.arc(cx + spacing, cy, radius, 0.0, std::f64::consts::TAU);
            cr.stroke().ok();

            // Center dots
            cr.arc(cx - spacing, cy, 1.5, 0.0, std::f64::consts::TAU);
            cr.fill().ok();
            cr.arc(cx, cy, 1.5, 0.0, std::f64::consts::TAU);
            cr.fill().ok();
            cr.arc(cx + spacing, cy, 1.5, 0.0, std::f64::consts::TAU);
            cr.fill().ok();

            // Extending lines
            if horizontal {
                cr.move_to(x, y);
                cr.line_to(cx - spacing - radius - 4.0, y);
                cr.move_to(cx + spacing + radius + 4.0, y);
                cr.line_to(x + length, y);
            } else {
                cr.move_to(x, y);
                cr.line_to(x, cy - spacing - radius - 4.0);
                cr.move_to(x, cy + spacing + radius + 4.0);
                cr.line_to(x, y + length);
            }
            cr.stroke().ok();
        }
        DividerStyle::CrossedLines => {
            let cx = if horizontal { x + length / 2.0 } else { x };
            let cy = if horizontal { y } else { y + length / 2.0 };
            let cross_len = 20.0;
            let gap = 3.0;

            if horizontal {
                // Crossed pattern in center
                cr.move_to(cx - cross_len, y - gap);
                cr.line_to(cx + cross_len, y + gap);
                cr.move_to(cx - cross_len, y + gap);
                cr.line_to(cx + cross_len, y - gap);
                cr.stroke().ok();

                // Extending lines (double)
                cr.move_to(x, y - gap);
                cr.line_to(cx - cross_len - 4.0, y - gap);
                cr.move_to(x, y + gap);
                cr.line_to(cx - cross_len - 4.0, y + gap);
                cr.move_to(cx + cross_len + 4.0, y - gap);
                cr.line_to(x + length, y - gap);
                cr.move_to(cx + cross_len + 4.0, y + gap);
                cr.line_to(x + length, y + gap);
            } else {
                cr.move_to(x - gap, cy - cross_len);
                cr.line_to(x + gap, cy + cross_len);
                cr.move_to(x + gap, cy - cross_len);
                cr.line_to(x - gap, cy + cross_len);
                cr.stroke().ok();

                cr.move_to(x - gap, y);
                cr.line_to(x - gap, cy - cross_len - 4.0);
                cr.move_to(x + gap, y);
                cr.line_to(x + gap, cy - cross_len - 4.0);
                cr.move_to(x - gap, cy + cross_len + 4.0);
                cr.line_to(x - gap, y + length);
                cr.move_to(x + gap, cy + cross_len + 4.0);
                cr.line_to(x + gap, y + length);
            }
            cr.stroke().ok();
        }
        DividerStyle::FleurDeLis => {
            let cx = if horizontal { x + length / 2.0 } else { x };
            let cy = if horizontal { y } else { y + length / 2.0 };
            let size = 8.0;

            // Fleur-de-lis shape (simplified leaf/teardrop)
            cr.move_to(cx, cy - size);
            cr.curve_to(
                cx + size * 0.6,
                cy - size * 0.3,
                cx + size * 0.4,
                cy + size * 0.5,
                cx,
                cy + size,
            );
            cr.curve_to(
                cx - size * 0.4,
                cy + size * 0.5,
                cx - size * 0.6,
                cy - size * 0.3,
                cx,
                cy - size,
            );
            cr.stroke().ok();

            // Small curls on sides
            cr.arc(cx - size * 0.8, cy, 3.0, -0.5, std::f64::consts::PI + 0.5);
            cr.stroke().ok();
            cr.arc(cx + size * 0.8, cy, 3.0, -std::f64::consts::PI - 0.5, 0.5);
            cr.stroke().ok();

            // Extending lines
            if horizontal {
                cr.move_to(x, y);
                cr.line_to(cx - size - 8.0, y);
                cr.move_to(cx + size + 8.0, y);
                cr.line_to(x + length, y);
            } else {
                cr.move_to(x, y);
                cr.line_to(x, cy - size - 8.0);
                cr.move_to(x, cy + size + 8.0);
                cr.line_to(x, y + length);
            }
            cr.stroke().ok();
        }
        DividerStyle::Heartbeat => {
            let cx = if horizontal { x + length / 2.0 } else { x };
            let cy = if horizontal { y } else { y + length / 2.0 };
            let peak = 10.0;
            let width = 30.0;

            if horizontal {
                // Zigzag heartbeat pattern
                cr.move_to(cx - width, y);
                cr.line_to(cx - width * 0.6, y);
                cr.line_to(cx - width * 0.3, y - peak);
                cr.line_to(cx, y + peak * 0.5);
                cr.line_to(cx + width * 0.3, y - peak);
                cr.line_to(cx + width * 0.6, y);
                cr.line_to(cx + width, y);
                cr.stroke().ok();

                // Small diamonds at ends
                draw_diamond(cr, cx - width - 6.0, y, 4.0, &divider_color, true);
                draw_diamond(cr, cx + width + 6.0, y, 4.0, &divider_color, true);

                // Extending lines
                cr.move_to(x, y);
                cr.line_to(cx - width - 12.0, y);
                cr.move_to(cx + width + 12.0, y);
                cr.line_to(x + length, y);
            } else {
                cr.move_to(x, cy - width);
                cr.line_to(x, cy - width * 0.6);
                cr.line_to(x - peak, cy - width * 0.3);
                cr.line_to(x + peak * 0.5, cy);
                cr.line_to(x - peak, cy + width * 0.3);
                cr.line_to(x, cy + width * 0.6);
                cr.line_to(x, cy + width);
                cr.stroke().ok();

                draw_diamond(cr, x, cy - width - 6.0, 4.0, &divider_color, true);
                draw_diamond(cr, x, cy + width + 6.0, 4.0, &divider_color, true);

                cr.move_to(x, y);
                cr.line_to(x, cy - width - 12.0);
                cr.move_to(x, cy + width + 12.0);
                cr.line_to(x, y + length);
            }
            cr.stroke().ok();
        }
        DividerStyle::DiamondGrid => {
            let cx = if horizontal { x + length / 2.0 } else { x };
            let cy = if horizontal { y } else { y + length / 2.0 };
            let size = 10.0;

            // Large central diamond
            draw_diamond(cr, cx, cy, size, &divider_color, false);
            // Inner diamond
            draw_diamond(cr, cx, cy, size * 0.5, &divider_color, false);

            // Corner diamonds
            draw_diamond(cr, cx - size, cy, size * 0.4, &divider_color, true);
            draw_diamond(cr, cx + size, cy, size * 0.4, &divider_color, true);
            draw_diamond(cr, cx, cy - size, size * 0.4, &divider_color, true);
            draw_diamond(cr, cx, cy + size, size * 0.4, &divider_color, true);

            // Circles at outer positions
            let circle_dist = size + 8.0;
            cr.arc(cx - circle_dist, cy, 2.5, 0.0, std::f64::consts::TAU);
            cr.stroke().ok();
            cr.arc(cx + circle_dist, cy, 2.5, 0.0, std::f64::consts::TAU);
            cr.stroke().ok();

            // Extending lines
            if horizontal {
                cr.move_to(x, y);
                cr.line_to(cx - circle_dist - 6.0, y);
                cr.move_to(cx + circle_dist + 6.0, y);
                cr.line_to(x + length, y);
            } else {
                cr.move_to(x, y);
                cr.line_to(x, cy - circle_dist - 6.0);
                cr.move_to(x, cy + circle_dist + 6.0);
                cr.line_to(x, y + length);
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
