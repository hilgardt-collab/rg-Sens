//! Material Design Cards display rendering
//!
//! Provides a clean, modern Material Design-inspired interface with:
//! - Clean white/dark cards with subtle drop shadows
//! - Large rounded corners
//! - Generous whitespace and padding
//! - Color-coded category headers
//! - Smooth animations

use std::collections::HashMap;

use anyhow::Result;
use cairo::Context;
use serde::{Deserialize, Serialize};

use crate::ui::background::Color;
use crate::ui::combo_config_base::{LayoutFrameConfig, ThemedFrameConfig};
use crate::ui::lcars_display::{ContentItemConfig, SplitOrientation};
use crate::ui::pango_text::{pango_show_text, pango_text_extents};
use crate::ui::theme::{ColorSource, ComboThemeConfig, FontSource, deserialize_color_or_source, deserialize_font_or_source};

// Re-export types we use
pub use crate::ui::lcars_display::{ContentDisplayType as MaterialContentType, ContentItemConfig as MaterialContentItemConfig};

/// Card elevation level (affects shadow intensity)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum CardElevation {
    /// Flat - no shadow
    Flat,
    /// Low elevation - subtle shadow
    #[default]
    Low,
    /// Medium elevation - moderate shadow
    Medium,
    /// High elevation - prominent shadow
    High,
}

/// Header style for cards
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum HeaderStyle {
    /// Colored bar at top
    #[default]
    ColorBar,
    /// Full colored background with white text
    Filled,
    /// Text only with colored text
    TextOnly,
    /// No header
    None,
}

/// Theme variant
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ThemeVariant {
    /// Light theme (white cards on light gray background)
    #[default]
    Light,
    /// Dark theme (dark gray cards on darker background)
    Dark,
}

/// Divider style between groups
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum DividerStyle {
    /// No visible divider, just spacing
    #[default]
    Space,
    /// Thin line divider
    Line,
    /// Subtle gradient fade
    Fade,
}

/// Header text alignment
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum HeaderAlignment {
    /// Left-aligned text (default)
    #[default]
    Left,
    /// Center-aligned text
    Center,
    /// Right-aligned text
    Right,
}

fn default_corner_radius() -> f64 { 12.0 }
fn default_card_padding() -> f64 { 16.0 }
fn default_content_padding() -> f64 { 20.0 }
fn default_item_spacing() -> f64 { 12.0 }
fn default_header_height() -> f64 { 40.0 }
fn default_shadow_blur() -> f64 { 8.0 }
fn default_shadow_offset_y() -> f64 { 2.0 }
fn default_divider_spacing() -> f64 { 16.0 }
fn default_group_count() -> usize { 2 }

fn default_accent_color() -> ColorSource {
    ColorSource::theme(3) // Theme color 3 (accent)
}

fn default_surface_color_light() -> Color {
    Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 } // White
}

fn default_surface_color_dark() -> Color {
    Color { r: 0.12, g: 0.12, b: 0.12, a: 1.0 } // Dark gray
}

fn default_background_color_light() -> Color {
    Color { r: 0.96, g: 0.96, b: 0.96, a: 1.0 } // Light gray
}

fn default_background_color_dark() -> Color {
    Color { r: 0.06, g: 0.06, b: 0.06, a: 1.0 } // Near black
}

fn default_text_color_light() -> Color {
    Color { r: 0.13, g: 0.13, b: 0.13, a: 1.0 } // Near black
}

fn default_text_color_dark() -> Color {
    Color { r: 0.93, g: 0.93, b: 0.93, a: 1.0 } // Near white
}

fn default_shadow_color() -> Color {
    Color { r: 0.0, g: 0.0, b: 0.0, a: 0.15 } // Subtle black shadow
}

fn default_divider_color() -> ColorSource {
    ColorSource::custom(Color { r: 0.5, g: 0.5, b: 0.5, a: 0.2 }) // Subtle gray
}

fn default_header_font_source() -> FontSource { FontSource::theme(1, 14.0) } // Theme font 1

/// Main configuration for the Material Cards frame
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialFrameConfig {
    // Theme variant (light/dark)
    #[serde(default)]
    pub theme_variant: ThemeVariant,
    #[serde(default = "default_accent_color", deserialize_with = "deserialize_color_or_source")]
    pub accent_color: ColorSource,

    // Card styling
    #[serde(default)]
    pub elevation: CardElevation,
    #[serde(default = "default_corner_radius")]
    pub corner_radius: f64,
    #[serde(default = "default_card_padding")]
    pub card_padding: f64,
    #[serde(default = "default_shadow_blur")]
    pub shadow_blur: f64,
    #[serde(default = "default_shadow_offset_y")]
    pub shadow_offset_y: f64,
    #[serde(default = "default_shadow_color")]
    pub shadow_color: Color,

    // Surface colors (card background)
    #[serde(default = "default_surface_color_light")]
    pub surface_color_light: Color,
    #[serde(default = "default_surface_color_dark")]
    pub surface_color_dark: Color,

    // Background colors (behind cards)
    #[serde(default = "default_background_color_light")]
    pub background_color_light: Color,
    #[serde(default = "default_background_color_dark")]
    pub background_color_dark: Color,

    // Text colors
    #[serde(default = "default_text_color_light")]
    pub text_color_light: Color,
    #[serde(default = "default_text_color_dark")]
    pub text_color_dark: Color,

    // Header
    #[serde(default)]
    pub show_header: bool,
    #[serde(default)]
    pub header_text: String,
    #[serde(default)]
    pub header_style: HeaderStyle,
    #[serde(default = "default_header_font_source", deserialize_with = "deserialize_font_or_source")]
    pub header_font: FontSource,
    #[serde(default = "default_header_height")]
    pub header_height: f64,
    #[serde(default)]
    pub header_alignment: HeaderAlignment,

    // Layout
    #[serde(default = "default_content_padding")]
    pub content_padding: f64,
    #[serde(default = "default_item_spacing")]
    pub item_spacing: f64,
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

    /// Per-group accent colors for headers
    #[serde(default)]
    pub group_accent_colors: Vec<Color>,

    /// Per-group header labels
    #[serde(default)]
    pub group_headers: Vec<String>,

    // Dividers
    #[serde(default)]
    pub divider_style: DividerStyle,
    #[serde(default = "default_divider_color", deserialize_with = "deserialize_color_or_source")]
    pub divider_color: ColorSource,
    #[serde(default = "default_divider_spacing")]
    pub divider_spacing: f64,

    // Content items (per slot)
    #[serde(default)]
    pub content_items: HashMap<String, ContentItemConfig>,

    /// Theme configuration
    #[serde(default = "default_material_theme")]
    pub theme: crate::ui::theme::ComboThemeConfig,
}

fn default_material_theme() -> crate::ui::theme::ComboThemeConfig {
    crate::ui::theme::ComboThemeConfig::default_for_material()
}

impl Default for MaterialFrameConfig {
    fn default() -> Self {
        Self {
            theme_variant: ThemeVariant::default(),
            accent_color: default_accent_color(),
            elevation: CardElevation::default(),
            corner_radius: default_corner_radius(),
            card_padding: default_card_padding(),
            shadow_blur: default_shadow_blur(),
            shadow_offset_y: default_shadow_offset_y(),
            shadow_color: default_shadow_color(),
            surface_color_light: default_surface_color_light(),
            surface_color_dark: default_surface_color_dark(),
            background_color_light: default_background_color_light(),
            background_color_dark: default_background_color_dark(),
            text_color_light: default_text_color_light(),
            text_color_dark: default_text_color_dark(),
            show_header: false,
            header_text: String::new(),
            header_style: HeaderStyle::default(),
            header_font: default_header_font_source(),
            header_height: default_header_height(),
            header_alignment: HeaderAlignment::default(),
            content_padding: default_content_padding(),
            item_spacing: default_item_spacing(),
            group_count: default_group_count(),
            group_item_counts: vec![1, 1],
            group_size_weights: vec![1.0, 1.0],
            split_orientation: SplitOrientation::default(),
            group_item_orientations: Vec::new(),
            group_accent_colors: Vec::new(),
            group_headers: Vec::new(),
            divider_style: DividerStyle::default(),
            divider_color: default_divider_color(),
            divider_spacing: default_divider_spacing(),
            content_items: HashMap::new(),
            theme: default_material_theme(),
        }
    }
}

impl LayoutFrameConfig for MaterialFrameConfig {
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

impl ThemedFrameConfig for MaterialFrameConfig {
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

impl MaterialFrameConfig {
    /// Get the surface color based on current theme variant
    pub fn surface_color(&self) -> Color {
        match self.theme_variant {
            ThemeVariant::Light => self.surface_color_light,
            ThemeVariant::Dark => self.surface_color_dark,
        }
    }

    /// Get the background color based on current theme variant
    pub fn background_color(&self) -> Color {
        match self.theme_variant {
            ThemeVariant::Light => self.background_color_light,
            ThemeVariant::Dark => self.background_color_dark,
        }
    }

    /// Get the text color based on current theme variant
    pub fn text_color(&self) -> Color {
        match self.theme_variant {
            ThemeVariant::Light => self.text_color_light,
            ThemeVariant::Dark => self.text_color_dark,
        }
    }

    /// Get accent color for a specific group (resolved through theme)
    pub fn group_accent(&self, group_idx: usize) -> Color {
        self.group_accent_colors
            .get(group_idx)
            .copied()
            .unwrap_or_else(|| self.accent_color.resolve(&self.theme))
    }

    /// Get header text for a specific group
    pub fn group_header(&self, group_idx: usize) -> &str {
        self.group_headers
            .get(group_idx)
            .map(|s| s.as_str())
            .unwrap_or("")
    }
}

/// Draw a rounded rectangle path
fn draw_rounded_rect(cr: &Context, x: f64, y: f64, w: f64, h: f64, radius: f64) {
    let r = radius.min(w / 2.0).min(h / 2.0);

    cr.new_sub_path();
    cr.arc(x + w - r, y + r, r, -std::f64::consts::FRAC_PI_2, 0.0);
    cr.arc(x + w - r, y + h - r, r, 0.0, std::f64::consts::FRAC_PI_2);
    cr.arc(x + r, y + h - r, r, std::f64::consts::FRAC_PI_2, std::f64::consts::PI);
    cr.arc(x + r, y + r, r, std::f64::consts::PI, 3.0 * std::f64::consts::FRAC_PI_2);
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
                cr, text, font_family,
                cairo::FontSlant::Normal, cairo::FontWeight::Bold, font_size
            );
            area_x + (area_w - extents.width()) / 2.0
        }
        HeaderAlignment::Right => {
            let extents = pango_text_extents(
                cr, text, font_family,
                cairo::FontSlant::Normal, cairo::FontWeight::Bold, font_size
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
    let surface = config.surface_color();

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
    let header_text = config.group_header(group_idx);
    if header_text.is_empty() && !matches!(config.header_style, HeaderStyle::ColorBar) {
        return 0.0;
    }

    let accent = config.group_accent(group_idx);
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
                let text_color = config.text_color();
                cr.set_source_rgba(text_color.r, text_color.g, text_color.b, 0.87);
                let (font_family, font_size) = config.header_font.resolve(&config.theme);

                let text_y = y + bar_h + 8.0 + font_size;
                cr.move_to(x + config.card_padding, text_y);
                pango_show_text(cr, header_text, &font_family, cairo::FontSlant::Normal, cairo::FontWeight::Bold, font_size);
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
            pango_show_text(cr, header_text, &font_family, cairo::FontSlant::Normal, cairo::FontWeight::Bold, font_size);

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
            pango_show_text(cr, header_text, &font_family, cairo::FontSlant::Normal, cairo::FontWeight::Bold, font_size);

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
                gradient.add_color_stop_rgba(0.0, divider_color.r, divider_color.g, divider_color.b, 0.0);
                gradient.add_color_stop_rgba(0.3, divider_color.r, divider_color.g, divider_color.b, divider_color.a);
                gradient.add_color_stop_rgba(0.7, divider_color.r, divider_color.g, divider_color.b, divider_color.a);
                gradient.add_color_stop_rgba(1.0, divider_color.r, divider_color.g, divider_color.b, 0.0);

                cr.set_source(&gradient).ok();
                cr.set_line_width(1.0);
                cr.move_to(x, y);
                cr.line_to(x + length, y);
                cr.stroke().ok();
            } else {
                let gradient = cairo::LinearGradient::new(x, y, x, y + length);
                gradient.add_color_stop_rgba(0.0, divider_color.r, divider_color.g, divider_color.b, 0.0);
                gradient.add_color_stop_rgba(0.3, divider_color.r, divider_color.g, divider_color.b, divider_color.a);
                gradient.add_color_stop_rgba(0.7, divider_color.r, divider_color.g, divider_color.b, divider_color.a);
                gradient.add_color_stop_rgba(1.0, divider_color.r, divider_color.g, divider_color.b, 0.0);

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
    let bg = config.background_color();
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
            let text_color = config.text_color();
            cr.set_source_rgba(text_color.r, text_color.g, text_color.b, 0.87);
            let (font_family, font_size) = config.header_font.resolve(&config.theme);
            let actual_font_size = font_size + 2.0;

            let text_y = y + bar_h + 12.0 + font_size;
            let text_x = calculate_aligned_text_x(cr, &config.header_text, x, w, config.card_padding, config.header_alignment, &font_family, actual_font_size);
            cr.move_to(text_x, text_y);
            pango_show_text(cr, &config.header_text, &font_family, cairo::FontSlant::Normal, cairo::FontWeight::Bold, actual_font_size);
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
            let text_x = calculate_aligned_text_x(cr, &config.header_text, x, w, config.card_padding, config.header_alignment, &font_family, actual_font_size);
            cr.move_to(text_x, text_y);
            pango_show_text(cr, &config.header_text, &font_family, cairo::FontSlant::Normal, cairo::FontWeight::Bold, actual_font_size);

            cr.restore().ok();
            header_h
        }
        HeaderStyle::TextOnly => {
            let text_color = config.text_color();
            cr.set_source_rgba(text_color.r, text_color.g, text_color.b, 0.87);
            let (font_family, font_size) = config.header_font.resolve(&config.theme);
            let actual_font_size = font_size + 2.0;

            let text_y = y + config.card_padding + font_size;
            let text_x = calculate_aligned_text_x(cr, &config.header_text, x, w, config.card_padding, config.header_alignment, &font_family, actual_font_size);
            cr.move_to(text_x, text_y);
            pango_show_text(cr, &config.header_text, &font_family, cairo::FontSlant::Normal, cairo::FontWeight::Bold, actual_font_size);

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
    let header_text = config.group_header(group_idx);
    if header_text.is_empty() {
        return 0.0;
    }

    draw_group_header(cr, config, x, y, w, h, group_idx)
}
