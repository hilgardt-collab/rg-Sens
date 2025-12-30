//! Fighter Jet HUD display rendering
//!
//! Provides a military fighter jet heads-up display aesthetic with:
//! - Military green/amber monochrome color scheme
//! - Thin line frames with corner brackets [ ]
//! - Targeting reticle aesthetics for gauges
//! - Altitude/heading ladder-style scales
//! - Stencil military font styling

use std::collections::HashMap;

use anyhow::Result;
use cairo::Context;
use serde::{Deserialize, Serialize};

use crate::ui::background::Color;
use crate::ui::lcars_display::{ContentItemConfig, SplitOrientation};
use crate::ui::theme::{FontSource, deserialize_font_or_source};

// Re-export types we use
pub use crate::ui::lcars_display::{ContentDisplayType as FighterHudContentType, ContentItemConfig as FighterHudContentItemConfig};

/// HUD color presets (military display colors)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum HudColorPreset {
    /// Classic military green (night vision friendly)
    #[default]
    MilitaryGreen,
    /// Amber/orange (high contrast)
    Amber,
    /// Cyan (modern fighter jets)
    Cyan,
    /// White (daytime mode)
    White,
    /// Custom color
    Custom(Color),
}

impl HudColorPreset {
    /// Get the actual color value
    pub fn to_color(&self) -> Color {
        match self {
            HudColorPreset::MilitaryGreen => Color { r: 0.0, g: 0.9, b: 0.3, a: 1.0 },
            HudColorPreset::Amber => Color { r: 1.0, g: 0.75, b: 0.0, a: 1.0 },
            HudColorPreset::Cyan => Color { r: 0.0, g: 0.9, b: 1.0, a: 1.0 },
            HudColorPreset::White => Color { r: 0.95, g: 0.95, b: 0.95, a: 1.0 },
            HudColorPreset::Custom(c) => *c,
        }
    }

    /// Get a dimmed version for secondary elements
    pub fn to_dim_color(&self) -> Color {
        let c = self.to_color();
        Color { r: c.r * 0.5, g: c.g * 0.5, b: c.b * 0.5, a: c.a * 0.6 }
    }

    /// Get a bright version for emphasis
    pub fn to_bright_color(&self) -> Color {
        let c = self.to_color();
        Color {
            r: (c.r * 1.2).min(1.0),
            g: (c.g * 1.2).min(1.0),
            b: (c.b * 1.2).min(1.0),
            a: c.a
        }
    }
}

/// Frame style for the HUD
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum HudFrameStyle {
    /// Corner brackets [ ] with tick marks
    #[default]
    CornerBrackets,
    /// Targeting reticle corners
    TargetingReticle,
    /// Full box with corner accents
    TacticalBox,
    /// Minimal with just corner marks
    Minimal,
    /// No frame
    None,
}

/// Header style for the HUD
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum HudHeaderStyle {
    /// Status bar with designation
    #[default]
    StatusBar,
    /// Mission callout style
    MissionCallout,
    /// System ID style
    SystemId,
    /// No header
    None,
}

/// Divider style between content groups
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum HudDividerStyle {
    /// Tick mark ladder style
    #[default]
    TickLadder,
    /// Thin line with arrows
    ArrowLine,
    /// Dashed tactical line
    TacticalDash,
    /// Subtle gradient fade
    Fade,
    /// No divider
    None,
}

// Default value functions
fn default_line_width() -> f64 { 1.5 }
fn default_bracket_size() -> f64 { 20.0 }
fn default_bracket_thickness() -> f64 { 2.0 }
fn default_content_padding() -> f64 { 16.0 }
fn default_header_font_source() -> FontSource { FontSource::theme(1, 12.0) } // Theme font 1
fn default_header_height() -> f64 { 24.0 }
fn default_divider_padding() -> f64 { 6.0 }
fn default_group_count() -> usize { 1 }
fn default_glow_intensity() -> f64 { 0.3 }
fn default_tick_spacing() -> f64 { 8.0 }
fn default_reticle_size() -> f64 { 0.15 }

fn default_background_color() -> Color {
    Color { r: 0.0, g: 0.0, b: 0.0, a: 0.0 } // Transparent by default (HUD overlay)
}

/// Main configuration for the Fighter HUD frame
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FighterHudFrameConfig {
    // Color scheme
    #[serde(default)]
    pub hud_color: HudColorPreset,
    #[serde(default = "default_background_color")]
    pub background_color: Color,
    #[serde(default = "default_glow_intensity")]
    pub glow_intensity: f64,

    // Frame styling
    #[serde(default)]
    pub frame_style: HudFrameStyle,
    #[serde(default = "default_line_width")]
    pub line_width: f64,
    #[serde(default = "default_bracket_size")]
    pub bracket_size: f64,
    #[serde(default = "default_bracket_thickness")]
    pub bracket_thickness: f64,

    // Targeting reticle (optional center element)
    #[serde(default)]
    pub show_center_reticle: bool,
    #[serde(default = "default_reticle_size")]
    pub reticle_size: f64,

    // Header
    #[serde(default = "default_true")]
    pub show_header: bool,
    #[serde(default)]
    pub header_text: String,
    #[serde(default)]
    pub header_style: HudHeaderStyle,
    #[serde(default = "default_header_font_source", deserialize_with = "deserialize_font_or_source")]
    pub header_font: FontSource,
    #[serde(default = "default_header_height")]
    pub header_height: f64,

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
    pub divider_style: HudDividerStyle,
    #[serde(default = "default_divider_padding")]
    pub divider_padding: f64,
    #[serde(default = "default_tick_spacing")]
    pub tick_spacing: f64,

    // Content items (per slot)
    #[serde(default)]
    pub content_items: HashMap<String, ContentItemConfig>,

    // Animation
    #[serde(default = "default_true")]
    pub animation_enabled: bool,
    #[serde(default)]
    pub scan_line_effect: bool,

    /// Theme configuration
    #[serde(default = "default_fighter_hud_theme")]
    pub theme: crate::ui::theme::ComboThemeConfig,
}

fn default_fighter_hud_theme() -> crate::ui::theme::ComboThemeConfig {
    crate::ui::theme::ComboThemeConfig::default_for_fighter_hud()
}

fn default_true() -> bool { true }

impl Default for FighterHudFrameConfig {
    fn default() -> Self {
        Self {
            hud_color: HudColorPreset::MilitaryGreen,
            background_color: default_background_color(),
            glow_intensity: default_glow_intensity(),

            frame_style: HudFrameStyle::CornerBrackets,
            line_width: default_line_width(),
            bracket_size: default_bracket_size(),
            bracket_thickness: default_bracket_thickness(),

            show_center_reticle: false,
            reticle_size: default_reticle_size(),

            show_header: true,
            header_text: "SYS MONITOR".to_string(),
            header_style: HudHeaderStyle::StatusBar,
            header_font: default_header_font_source(),
            header_height: default_header_height(),

            content_padding: default_content_padding(),
            group_count: default_group_count(),
            group_item_counts: vec![4],
            group_size_weights: vec![1.0],
            split_orientation: SplitOrientation::Vertical,

            divider_style: HudDividerStyle::TickLadder,
            divider_padding: default_divider_padding(),
            tick_spacing: default_tick_spacing(),

            content_items: HashMap::new(),

            animation_enabled: true,
            scan_line_effect: false,

            theme: default_fighter_hud_theme(),
        }
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

            // Small targeting pips at corners
            let pip_size = 3.0;
            cr.arc(x + corner_gap / 2.0, y + corner_gap / 2.0, pip_size, 0.0, std::f64::consts::TAU);
            cr.arc(x + w - corner_gap / 2.0, y + corner_gap / 2.0, pip_size, 0.0, std::f64::consts::TAU);
            cr.arc(x + corner_gap / 2.0, y + h - corner_gap / 2.0, pip_size, 0.0, std::f64::consts::TAU);
            cr.arc(x + w - corner_gap / 2.0, y + h - corner_gap / 2.0, pip_size, 0.0, std::f64::consts::TAU);
            cr.fill().ok();
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

    let color = config.hud_color.to_color();
    let cx = x + w / 2.0;
    let cy = y + h / 2.0;
    let size = w.min(h) * config.reticle_size;

    cr.save().ok();
    cr.set_source_rgba(color.r, color.g, color.b, color.a * 0.8);
    cr.set_line_width(config.line_width);

    // Center circle
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

    // Corner brackets around reticle
    let bracket_offset = size * 0.6;
    let bracket_len = size * 0.2;
    let dim_color = config.hud_color.to_dim_color();
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
fn draw_header(
    cr: &Context,
    config: &FighterHudFrameConfig,
    x: f64,
    y: f64,
    w: f64,
) -> f64 {
    if !config.show_header || matches!(config.header_style, HudHeaderStyle::None) {
        return 0.0;
    }

    let header_h = config.header_height;
    let color = config.hud_color.to_color();
    let dim_color = config.hud_color.to_dim_color();

    cr.save().ok();

    // Resolve header font from FontSource using theme
    let (header_font_family, header_font_size) = config.header_font.resolve(&config.theme);
    cr.select_font_face(
        &header_font_family,
        cairo::FontSlant::Normal,
        cairo::FontWeight::Bold,
    );
    cr.set_font_size(header_font_size);

    let text = if config.header_text.is_empty() {
        "HUD"
    } else {
        &config.header_text
    };

    let text_extents = cr.text_extents(text).ok();
    let (_text_width, text_height) = text_extents.map(|e| (e.width(), e.height())).unwrap_or((0.0, 0.0));

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
                cr.show_text(&status_text).ok();
            }

            cr.set_source_rgba(color.r, color.g, color.b, color.a);
            cr.move_to(text_x, text_y);
            cr.show_text(&status_text).ok();

            // Right side status indicators
            let status_right = "ONLINE";
            let right_extents = cr.text_extents(status_right).ok();
            let right_width = right_extents.map(|e| e.width()).unwrap_or(0.0);

            cr.set_source_rgba(color.r, color.g, color.b, color.a * 0.7);
            cr.move_to(x + w - right_width - 4.0, text_y);
            cr.show_text(status_right).ok();
        }
        HudHeaderStyle::MissionCallout => {
            // Centered mission-style callout
            let callout_text = format!("<<< {} >>>", text.to_uppercase());
            let callout_extents = cr.text_extents(&callout_text).ok();
            let callout_width = callout_extents.map(|e| e.width()).unwrap_or(0.0);

            let text_x = x + (w - callout_width) / 2.0;
            let text_y = y + header_h / 2.0 + text_height / 3.0;

            // Glow
            if config.glow_intensity > 0.0 {
                cr.set_source_rgba(color.r, color.g, color.b, config.glow_intensity * 0.4);
                cr.move_to(text_x, text_y);
                cr.show_text(&callout_text).ok();
            }

            cr.set_source_rgba(color.r, color.g, color.b, color.a);
            cr.move_to(text_x, text_y);
            cr.show_text(&callout_text).ok();

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
            cr.show_text(&id_text).ok();
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
    draw_frame_corners(cr, config, frame_margin, frame_margin, width - frame_margin * 2.0, height - frame_margin * 2.0);

    // Draw header
    let content_x = config.content_padding;
    let header_height = draw_header(cr, config, content_x, frame_margin, width - config.content_padding * 2.0);

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
