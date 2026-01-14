//! Synthwave/Outrun display rendering
//!
//! Provides a retro-futuristic 80s aesthetic with:
//! - Purple/pink/cyan gradient backgrounds
//! - Neon grid lines (classic 80s grid horizon)
//! - Chrome/metallic text effects
//! - Sunset gradient accents
//! - Retro-futuristic fonts

use std::collections::HashMap;

use anyhow::Result;
use cairo::Context;
use serde::{Deserialize, Serialize};

use crate::displayers::combo_displayer_base::{ComboFrameConfig, FrameRenderer};
use crate::ui::background::Color;
use crate::ui::combo_config_base::{LayoutFrameConfig, ThemedFrameConfig};
use crate::ui::lcars_display::{ContentItemConfig, SplitOrientation};
use crate::ui::pango_text::{pango_show_text, pango_text_extents};
use crate::ui::theme::ComboThemeConfig;

// Re-export types we use
pub use crate::ui::lcars_display::{
    ContentDisplayType as SynthwaveContentType, ContentItemConfig as SynthwaveContentItemConfig,
};

/// Color scheme presets
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SynthwaveColorScheme {
    /// Classic purple/pink/cyan
    #[default]
    Classic,
    /// Hot sunset (orange/pink/purple)
    Sunset,
    /// Cool blue/cyan/purple
    NightDrive,
    /// Neon green/cyan/blue (Miami Vice)
    Miami,
    /// Custom colors
    Custom {
        primary: Color,
        secondary: Color,
        accent: Color,
    },
}

impl SynthwaveColorScheme {
    /// Get primary color (usually for main elements)
    pub fn primary(&self) -> Color {
        match self {
            SynthwaveColorScheme::Classic => Color {
                r: 0.58,
                g: 0.0,
                b: 0.83,
                a: 1.0,
            }, // Purple
            SynthwaveColorScheme::Sunset => Color {
                r: 1.0,
                g: 0.4,
                b: 0.0,
                a: 1.0,
            }, // Orange
            SynthwaveColorScheme::NightDrive => Color {
                r: 0.1,
                g: 0.1,
                b: 0.4,
                a: 1.0,
            }, // Deep blue
            SynthwaveColorScheme::Miami => Color {
                r: 0.0,
                g: 0.9,
                b: 0.7,
                a: 1.0,
            }, // Teal
            SynthwaveColorScheme::Custom { primary, .. } => *primary,
        }
    }

    /// Get secondary color (for gradients)
    pub fn secondary(&self) -> Color {
        match self {
            SynthwaveColorScheme::Classic => Color {
                r: 1.0,
                g: 0.08,
                b: 0.58,
                a: 1.0,
            }, // Hot pink
            SynthwaveColorScheme::Sunset => Color {
                r: 1.0,
                g: 0.0,
                b: 0.5,
                a: 1.0,
            }, // Magenta
            SynthwaveColorScheme::NightDrive => Color {
                r: 0.4,
                g: 0.0,
                b: 0.6,
                a: 1.0,
            }, // Purple
            SynthwaveColorScheme::Miami => Color {
                r: 1.0,
                g: 0.4,
                b: 0.7,
                a: 1.0,
            }, // Pink
            SynthwaveColorScheme::Custom { secondary, .. } => *secondary,
        }
    }

    /// Get accent color (for highlights, neon effects)
    pub fn accent(&self) -> Color {
        match self {
            SynthwaveColorScheme::Classic => Color {
                r: 0.0,
                g: 1.0,
                b: 1.0,
                a: 1.0,
            }, // Cyan
            SynthwaveColorScheme::Sunset => Color {
                r: 0.5,
                g: 0.0,
                b: 0.5,
                a: 1.0,
            }, // Purple
            SynthwaveColorScheme::NightDrive => Color {
                r: 0.0,
                g: 0.9,
                b: 1.0,
                a: 1.0,
            }, // Cyan
            SynthwaveColorScheme::Miami => Color {
                r: 0.0,
                g: 0.5,
                b: 1.0,
                a: 1.0,
            }, // Blue
            SynthwaveColorScheme::Custom { accent, .. } => *accent,
        }
    }

    /// Get neon glow color (typically the brightest)
    pub fn neon(&self) -> Color {
        let accent = self.accent();
        Color {
            r: (accent.r * 1.2).min(1.0),
            g: (accent.g * 1.2).min(1.0),
            b: (accent.b * 1.2).min(1.0),
            a: 1.0,
        }
    }

    /// Get background gradient colors (top, bottom)
    pub fn background_gradient(&self) -> (Color, Color) {
        match self {
            SynthwaveColorScheme::Classic => (
                Color {
                    r: 0.05,
                    g: 0.0,
                    b: 0.15,
                    a: 1.0,
                }, // Dark purple
                Color {
                    r: 0.0,
                    g: 0.0,
                    b: 0.05,
                    a: 1.0,
                }, // Near black
            ),
            SynthwaveColorScheme::Sunset => (
                Color {
                    r: 0.3,
                    g: 0.05,
                    b: 0.15,
                    a: 1.0,
                }, // Dark red
                Color {
                    r: 0.05,
                    g: 0.0,
                    b: 0.1,
                    a: 1.0,
                }, // Dark purple
            ),
            SynthwaveColorScheme::NightDrive => (
                Color {
                    r: 0.0,
                    g: 0.02,
                    b: 0.1,
                    a: 1.0,
                }, // Dark blue
                Color {
                    r: 0.0,
                    g: 0.0,
                    b: 0.02,
                    a: 1.0,
                }, // Near black
            ),
            SynthwaveColorScheme::Miami => (
                Color {
                    r: 0.0,
                    g: 0.1,
                    b: 0.15,
                    a: 1.0,
                }, // Dark teal
                Color {
                    r: 0.05,
                    g: 0.0,
                    b: 0.1,
                    a: 1.0,
                }, // Dark purple
            ),
            SynthwaveColorScheme::Custom {
                primary, secondary, ..
            } => (
                Color {
                    r: primary.r * 0.2,
                    g: primary.g * 0.2,
                    b: primary.b * 0.2,
                    a: 1.0,
                },
                Color {
                    r: secondary.r * 0.1,
                    g: secondary.g * 0.1,
                    b: secondary.b * 0.1,
                    a: 1.0,
                },
            ),
        }
    }
}

/// Frame style for the synthwave display
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SynthwaveFrameStyle {
    /// Neon border with glow
    #[default]
    NeonBorder,
    /// Chrome/metallic frame
    Chrome,
    /// Minimal corner accents
    Minimal,
    /// Double-line retro frame
    RetroDouble,
    /// No frame
    None,
}

/// Grid style for the background
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum GridStyle {
    /// Classic perspective grid (horizon effect)
    #[default]
    Perspective,
    /// Flat grid pattern
    Flat,
    /// Hexagonal grid
    Hexagon,
    /// Scanlines only
    Scanlines,
    /// No grid
    None,
}

/// Header style
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SynthwaveHeaderStyle {
    /// Chrome text with reflection
    #[default]
    Chrome,
    /// Neon glow text
    Neon,
    /// Outlined text
    Outline,
    /// Simple text
    Simple,
    /// No header
    None,
}

/// Divider style between groups
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SynthwaveDividerStyle {
    /// Neon line with glow
    #[default]
    NeonLine,
    /// Gradient fade
    Gradient,
    /// Dotted neon
    NeonDots,
    /// Minimal line
    Line,
    /// No divider
    None,
}

// Default value functions
fn default_grid_spacing() -> f64 {
    30.0
}
fn default_grid_line_width() -> f64 {
    1.0
}
fn default_grid_horizon() -> f64 {
    0.4
}
fn default_grid_perspective() -> f64 {
    0.8
}
fn default_neon_glow() -> f64 {
    0.6
}
fn default_content_padding() -> f64 {
    16.0
}
fn default_header_font() -> String {
    "sans-serif".to_string()
}
fn default_header_font_size() -> f64 {
    16.0
}
fn default_header_height() -> f64 {
    32.0
}
fn default_divider_padding() -> f64 {
    8.0
}
fn default_group_count() -> usize {
    1
}
fn default_frame_width() -> f64 {
    2.0
}
fn default_corner_radius() -> f64 {
    8.0
}
fn default_true() -> bool {
    true
}

/// Main configuration for the Synthwave frame
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthwaveFrameConfig {
    // Color scheme
    #[serde(default)]
    pub color_scheme: SynthwaveColorScheme,
    #[serde(default = "default_neon_glow")]
    pub neon_glow_intensity: f64,

    // Frame styling
    #[serde(default)]
    pub frame_style: SynthwaveFrameStyle,
    #[serde(default = "default_frame_width")]
    pub frame_width: f64,
    #[serde(default = "default_corner_radius")]
    pub corner_radius: f64,

    // Grid background
    #[serde(default)]
    pub grid_style: GridStyle,
    #[serde(default = "default_true")]
    pub show_grid: bool,
    #[serde(default = "default_grid_spacing")]
    pub grid_spacing: f64,
    #[serde(default = "default_grid_line_width")]
    pub grid_line_width: f64,
    #[serde(default = "default_grid_horizon")]
    pub grid_horizon: f64,
    #[serde(default = "default_grid_perspective")]
    pub grid_perspective: f64,

    // Sun/sunset effect
    #[serde(default)]
    pub show_sun: bool,
    #[serde(default)]
    pub sun_position: f64, // 0.0 = bottom, 1.0 = horizon

    // Header
    #[serde(default = "default_true")]
    pub show_header: bool,
    #[serde(default)]
    pub header_text: String,
    #[serde(default)]
    pub header_style: SynthwaveHeaderStyle,
    #[serde(default = "default_header_font")]
    pub header_font: String,
    #[serde(default = "default_header_font_size")]
    pub header_font_size: f64,
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
    /// Item orientation within each group - defaults to same as split_orientation
    #[serde(default)]
    pub group_item_orientations: Vec<SplitOrientation>,
    #[serde(default)]
    pub item_spacing: f64,

    // Dividers
    #[serde(default)]
    pub divider_style: SynthwaveDividerStyle,
    #[serde(default = "default_divider_padding")]
    pub divider_padding: f64,

    // Content items (per slot)
    #[serde(default)]
    pub content_items: HashMap<String, ContentItemConfig>,

    // Animation
    #[serde(default = "default_true")]
    pub animation_enabled: bool,
    #[serde(default = "default_animation_speed")]
    pub animation_speed: f64,
    #[serde(default)]
    pub scanline_effect: bool,

    // Theme configuration
    #[serde(default = "default_synthwave_theme")]
    pub theme: crate::ui::theme::ComboThemeConfig,
}

fn default_synthwave_theme() -> crate::ui::theme::ComboThemeConfig {
    crate::ui::theme::ComboThemeConfig::default_for_synthwave()
}

fn default_animation_speed() -> f64 {
    8.0
}

impl Default for SynthwaveFrameConfig {
    fn default() -> Self {
        Self {
            color_scheme: SynthwaveColorScheme::Classic,
            neon_glow_intensity: default_neon_glow(),

            frame_style: SynthwaveFrameStyle::NeonBorder,
            frame_width: default_frame_width(),
            corner_radius: default_corner_radius(),

            grid_style: GridStyle::Perspective,
            show_grid: true,
            grid_spacing: default_grid_spacing(),
            grid_line_width: default_grid_line_width(),
            grid_horizon: default_grid_horizon(),
            grid_perspective: default_grid_perspective(),

            show_sun: true,
            sun_position: 0.3,

            show_header: true,
            header_text: "SYNTHWAVE".to_string(),
            header_style: SynthwaveHeaderStyle::Chrome,
            header_font: default_header_font(),
            header_font_size: default_header_font_size(),
            header_height: default_header_height(),

            content_padding: default_content_padding(),
            group_count: default_group_count(),
            group_item_counts: vec![4],
            group_size_weights: vec![1.0],
            split_orientation: SplitOrientation::Vertical,
            group_item_orientations: Vec::new(),
            item_spacing: 8.0,

            divider_style: SynthwaveDividerStyle::NeonLine,
            divider_padding: default_divider_padding(),

            content_items: HashMap::new(),

            animation_enabled: true,
            animation_speed: default_animation_speed(),
            scanline_effect: false,

            theme: default_synthwave_theme(),
        }
    }
}

impl LayoutFrameConfig for SynthwaveFrameConfig {
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

impl ThemedFrameConfig for SynthwaveFrameConfig {
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

impl ComboFrameConfig for SynthwaveFrameConfig {
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

/// Frame renderer for Synthwave theme
pub struct SynthwaveRenderer;

impl FrameRenderer for SynthwaveRenderer {
    type Config = SynthwaveFrameConfig;

    fn theme_id(&self) -> &'static str {
        "synthwave"
    }

    fn theme_name(&self) -> &'static str {
        "Synthwave"
    }

    fn default_config(&self) -> Self::Config {
        SynthwaveFrameConfig::default()
    }

    fn render_frame(
        &self,
        cr: &Context,
        config: &Self::Config,
        width: f64,
        height: f64,
    ) -> anyhow::Result<(f64, f64, f64, f64)> {
        render_synthwave_frame(cr, config, width, height).map_err(|e| anyhow::anyhow!("{}", e))
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

/// Draw the background gradient
fn draw_background(cr: &Context, config: &SynthwaveFrameConfig, width: f64, height: f64) {
    let (top_color, bottom_color) = config.color_scheme.background_gradient();

    let gradient = cairo::LinearGradient::new(0.0, 0.0, 0.0, height);
    gradient.add_color_stop_rgba(0.0, top_color.r, top_color.g, top_color.b, top_color.a);
    gradient.add_color_stop_rgba(
        1.0,
        bottom_color.r,
        bottom_color.g,
        bottom_color.b,
        bottom_color.a,
    );

    cr.set_source(&gradient).ok();
    cr.rectangle(0.0, 0.0, width, height);
    cr.fill().ok();
}

/// Draw the sun/sunset effect
fn draw_sun(cr: &Context, config: &SynthwaveFrameConfig, width: f64, height: f64) {
    if !config.show_sun {
        return;
    }

    let primary = config.color_scheme.primary();
    let secondary = config.color_scheme.secondary();

    let horizon_y = height * config.grid_horizon;
    let sun_y = horizon_y - height * config.sun_position * 0.3;
    let sun_x = width / 2.0;
    let sun_radius = width.min(height) * 0.15;

    cr.save().ok();

    // Create radial gradient for sun
    let sun_gradient =
        cairo::RadialGradient::new(sun_x, sun_y, 0.0, sun_x, sun_y, sun_radius * 1.5);
    sun_gradient.add_color_stop_rgba(0.0, secondary.r, secondary.g, secondary.b, 1.0);
    sun_gradient.add_color_stop_rgba(0.5, primary.r, primary.g, primary.b, 0.8);
    sun_gradient.add_color_stop_rgba(1.0, primary.r, primary.g, primary.b, 0.0);

    cr.set_source(&sun_gradient).ok();
    cr.arc(sun_x, sun_y, sun_radius * 1.5, 0.0, std::f64::consts::TAU);
    cr.fill().ok();

    // Sun body with horizontal stripes
    let stripe_count = 5;
    let stripe_height = sun_radius * 2.0 / stripe_count as f64;

    for i in 0..stripe_count {
        if i % 2 == 0 {
            let y_top = sun_y - sun_radius + i as f64 * stripe_height;

            // Draw stripe clipped to sun circle
            cr.set_source_rgba(secondary.r, secondary.g, secondary.b, 1.0);
            cr.arc(sun_x, sun_y, sun_radius, 0.0, std::f64::consts::TAU);
            cr.clip();

            cr.rectangle(sun_x - sun_radius, y_top, sun_radius * 2.0, stripe_height);
            cr.fill().ok();

            cr.reset_clip();
        }
    }

    cr.restore().ok();
}

/// Draw the perspective grid
fn draw_grid(cr: &Context, config: &SynthwaveFrameConfig, width: f64, height: f64) {
    if !config.show_grid {
        return;
    }

    let accent = config.color_scheme.accent();
    let horizon_y = height * config.grid_horizon;

    cr.save().ok();
    cr.set_line_width(config.grid_line_width);

    match config.grid_style {
        GridStyle::Perspective => {
            // Calculate aspect ratio to adjust grid proportions
            let aspect_ratio = width / height;
            let grid_height = height - horizon_y;

            // For tall panels, use width-based spacing to maintain visual consistency
            // For wide panels, use height-based spacing
            let base_spacing = if aspect_ratio < 1.0 {
                // Tall panel: base spacing on width
                width * 0.08
            } else {
                // Wide panel: base spacing on grid height
                grid_height * 0.1
            };

            // Horizontal grid lines with perspective (from horizon to bottom)
            // Use consistent visual spacing that works for any aspect ratio
            let min_spacing = base_spacing * 0.15; // Minimum spacing at horizon
            let max_spacing = base_spacing * 1.5; // Maximum spacing at bottom
            let perspective = config.grid_perspective;

            let mut y = horizon_y + min_spacing;
            let mut spacing = min_spacing;
            while y < height {
                // Fade lines closer to horizon
                let t = (y - horizon_y) / grid_height;
                let alpha = (0.2 + t * 0.4).min(0.6);
                cr.set_source_rgba(accent.r, accent.g, accent.b, alpha);

                cr.move_to(0.0, y);
                cr.line_to(width, y);
                cr.stroke().ok();

                // Increase spacing as we move away from horizon (perspective effect)
                spacing = (spacing * (1.0 + 0.15 * perspective)).min(max_spacing);
                y += spacing;
            }

            // Vertical grid lines converging at vanishing point
            let center_x = width / 2.0;
            // Place vanishing point at a fixed visual distance above horizon
            let vanishing_distance = width * 0.3; // Based on width for consistency
            let vanishing_y = horizon_y - vanishing_distance;

            // Clip to panel bounds for vertical lines
            cr.save().ok();
            cr.rectangle(0.0, horizon_y, width, height - horizon_y);
            cr.clip();

            // Adjust vertical line count based on width
            let vertical_count = ((width / 25.0) as usize).clamp(8, 30);

            // Distribute lines evenly AT THE HORIZON, then trace down to bottom
            // This ensures lines extend to panel edges at the horizon
            for i in 0..vertical_count {
                // Position at horizon spans full width
                let x_horizon = (i as f64 / (vertical_count - 1) as f64) * width;

                // Calculate where this line would be at the bottom
                // Line goes from vanishing point through x_horizon at horizon_y
                let dx_to_vanish = center_x - x_horizon;
                let dy_to_vanish = vanishing_y - horizon_y;

                // Avoid division issues
                if dy_to_vanish.abs() < 0.001 {
                    continue;
                }

                // Extend line from horizon down to bottom of panel
                let t_bottom = (height - horizon_y) / (-dy_to_vanish);
                let x_bottom = x_horizon - dx_to_vanish * t_bottom;

                let distance_from_center = (x_horizon - center_x).abs() / (width / 2.0);
                let alpha = (1.0 - distance_from_center * 0.5) * 0.4;
                cr.set_source_rgba(accent.r, accent.g, accent.b, alpha);

                cr.move_to(x_bottom, height);
                cr.line_to(x_horizon, horizon_y);
                cr.stroke().ok();
            }

            cr.restore().ok();
        }
        GridStyle::Flat => {
            // Simple flat grid
            let spacing = config.grid_spacing;
            cr.set_source_rgba(accent.r, accent.g, accent.b, 0.3);

            // Horizontal lines
            let mut y = 0.0;
            while y < height {
                cr.move_to(0.0, y);
                cr.line_to(width, y);
                y += spacing;
            }

            // Vertical lines
            let mut x = 0.0;
            while x < width {
                cr.move_to(x, 0.0);
                cr.line_to(x, height);
                x += spacing;
            }
            cr.stroke().ok();
        }
        GridStyle::Hexagon => {
            // Hexagonal pattern
            let size = config.grid_spacing;
            let hex_height = size * 3.0_f64.sqrt();
            let hex_width = size * 2.0;

            cr.set_source_rgba(accent.r, accent.g, accent.b, 0.25);

            let mut row = 0;
            let mut y = 0.0;
            while y < height + hex_height {
                let offset = if row % 2 == 0 { 0.0 } else { hex_width * 0.75 };
                let mut x = offset;
                while x < width + hex_width {
                    draw_hexagon(cr, x, y, size);
                    x += hex_width * 1.5;
                }
                y += hex_height / 2.0;
                row += 1;
            }
            cr.stroke().ok();
        }
        GridStyle::Scanlines => {
            // Horizontal scanlines
            cr.set_source_rgba(accent.r, accent.g, accent.b, 0.1);
            let spacing = 2.0;
            let mut y = 0.0;
            while y < height {
                cr.move_to(0.0, y);
                cr.line_to(width, y);
                y += spacing;
            }
            cr.stroke().ok();
        }
        GridStyle::None => {}
    }

    cr.restore().ok();
}

/// Draw a hexagon at the given position
fn draw_hexagon(cr: &Context, cx: f64, cy: f64, size: f64) {
    let angle_offset = std::f64::consts::FRAC_PI_6;
    for i in 0..6 {
        let angle = angle_offset + i as f64 * std::f64::consts::FRAC_PI_3;
        let x = cx + size * angle.cos();
        let y = cy + size * angle.sin();
        if i == 0 {
            cr.move_to(x, y);
        } else {
            cr.line_to(x, y);
        }
    }
    cr.close_path();
}

/// Draw the neon frame
fn draw_frame(cr: &Context, config: &SynthwaveFrameConfig, width: f64, height: f64) {
    if matches!(config.frame_style, SynthwaveFrameStyle::None) {
        return;
    }

    let accent = config.color_scheme.accent();
    let neon = config.color_scheme.neon();
    let secondary = config.color_scheme.secondary();
    let radius = config.corner_radius;
    let line_width = config.frame_width;
    let glow = config.neon_glow_intensity;

    cr.save().ok();

    match config.frame_style {
        SynthwaveFrameStyle::NeonBorder => {
            // Glow effect (multiple passes with decreasing alpha)
            if glow > 0.0 {
                for i in (1..=4).rev() {
                    let alpha = glow * 0.15 * (5.0 - i as f64) / 4.0;
                    let width_mult = 1.0 + i as f64 * 0.8;
                    cr.set_source_rgba(neon.r, neon.g, neon.b, alpha);
                    cr.set_line_width(line_width * width_mult);
                    draw_rounded_rect(
                        cr,
                        line_width,
                        line_width,
                        width - line_width * 2.0,
                        height - line_width * 2.0,
                        radius,
                    );
                    cr.stroke().ok();
                }
            }

            // Main neon line
            cr.set_source_rgba(accent.r, accent.g, accent.b, 1.0);
            cr.set_line_width(line_width);
            draw_rounded_rect(
                cr,
                line_width,
                line_width,
                width - line_width * 2.0,
                height - line_width * 2.0,
                radius,
            );
            cr.stroke().ok();

            // Bright center line
            cr.set_source_rgba(1.0, 1.0, 1.0, 0.8);
            cr.set_line_width(line_width * 0.3);
            draw_rounded_rect(
                cr,
                line_width,
                line_width,
                width - line_width * 2.0,
                height - line_width * 2.0,
                radius,
            );
            cr.stroke().ok();
        }
        SynthwaveFrameStyle::Chrome => {
            // Chrome/metallic gradient frame
            let gradient = cairo::LinearGradient::new(0.0, 0.0, 0.0, height);
            gradient.add_color_stop_rgba(0.0, 0.9, 0.9, 0.95, 1.0);
            gradient.add_color_stop_rgba(0.3, 0.5, 0.5, 0.6, 1.0);
            gradient.add_color_stop_rgba(0.5, 0.2, 0.2, 0.25, 1.0);
            gradient.add_color_stop_rgba(0.7, 0.5, 0.5, 0.6, 1.0);
            gradient.add_color_stop_rgba(1.0, 0.9, 0.9, 0.95, 1.0);

            cr.set_source(&gradient).ok();
            cr.set_line_width(line_width * 2.0);
            draw_rounded_rect(
                cr,
                line_width,
                line_width,
                width - line_width * 2.0,
                height - line_width * 2.0,
                radius,
            );
            cr.stroke().ok();

            // Inner highlight
            cr.set_source_rgba(1.0, 1.0, 1.0, 0.5);
            cr.set_line_width(1.0);
            draw_rounded_rect(
                cr,
                line_width + 2.0,
                line_width + 2.0,
                width - line_width * 2.0 - 4.0,
                height - line_width * 2.0 - 4.0,
                radius - 2.0,
            );
            cr.stroke().ok();
        }
        SynthwaveFrameStyle::Minimal => {
            // Just corner accents
            let corner_size = 20.0;
            cr.set_source_rgba(accent.r, accent.g, accent.b, 0.8);
            cr.set_line_width(line_width);

            // Top-left
            cr.move_to(line_width, line_width + corner_size);
            cr.line_to(line_width, line_width);
            cr.line_to(line_width + corner_size, line_width);

            // Top-right
            cr.move_to(width - line_width - corner_size, line_width);
            cr.line_to(width - line_width, line_width);
            cr.line_to(width - line_width, line_width + corner_size);

            // Bottom-left
            cr.move_to(line_width, height - line_width - corner_size);
            cr.line_to(line_width, height - line_width);
            cr.line_to(line_width + corner_size, height - line_width);

            // Bottom-right
            cr.move_to(width - line_width - corner_size, height - line_width);
            cr.line_to(width - line_width, height - line_width);
            cr.line_to(width - line_width, height - line_width - corner_size);

            cr.stroke().ok();
        }
        SynthwaveFrameStyle::RetroDouble => {
            // Double-line retro frame
            let gap = 4.0;

            // Outer line
            cr.set_source_rgba(accent.r, accent.g, accent.b, 0.8);
            cr.set_line_width(line_width);
            draw_rounded_rect(
                cr,
                line_width,
                line_width,
                width - line_width * 2.0,
                height - line_width * 2.0,
                radius,
            );
            cr.stroke().ok();

            // Inner line (different color)
            cr.set_source_rgba(secondary.r, secondary.g, secondary.b, 0.6);
            cr.set_line_width(line_width * 0.75);
            draw_rounded_rect(
                cr,
                line_width + gap,
                line_width + gap,
                width - (line_width + gap) * 2.0,
                height - (line_width + gap) * 2.0,
                radius.max(gap) - gap,
            );
            cr.stroke().ok();
        }
        SynthwaveFrameStyle::None => {}
    }

    cr.restore().ok();
}

/// Helper to draw a rounded rectangle
fn draw_rounded_rect(cr: &Context, x: f64, y: f64, w: f64, h: f64, r: f64) {
    let r = r.min(w / 2.0).min(h / 2.0);
    cr.new_path();
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

/// Draw the header
fn draw_header(cr: &Context, config: &SynthwaveFrameConfig, x: f64, y: f64, w: f64) -> f64 {
    if !config.show_header || matches!(config.header_style, SynthwaveHeaderStyle::None) {
        return 0.0;
    }

    let header_h = config.header_height;
    let accent = config.color_scheme.accent();
    let secondary = config.color_scheme.secondary();
    let neon = config.color_scheme.neon();

    cr.save().ok();

    let text = if config.header_text.is_empty() {
        "SYNTHWAVE"
    } else {
        &config.header_text
    };

    let text_extents = pango_text_extents(
        cr,
        text,
        &config.header_font,
        cairo::FontSlant::Normal,
        cairo::FontWeight::Bold,
        config.header_font_size,
    );
    let text_width = text_extents.width();
    let text_height = text_extents.height();

    let text_x = x + (w - text_width) / 2.0;
    let text_y = y + header_h / 2.0 + text_height / 3.0;

    match config.header_style {
        SynthwaveHeaderStyle::Chrome => {
            // Chrome text effect with gradient
            let gradient =
                cairo::LinearGradient::new(text_x, text_y - text_height, text_x, text_y + 2.0);
            gradient.add_color_stop_rgba(0.0, 0.95, 0.95, 1.0, 1.0);
            gradient.add_color_stop_rgba(0.4, 0.6, 0.6, 0.7, 1.0);
            gradient.add_color_stop_rgba(0.5, 0.3, 0.3, 0.4, 1.0);
            gradient.add_color_stop_rgba(0.6, 0.7, 0.7, 0.8, 1.0);
            gradient.add_color_stop_rgba(1.0, 0.95, 0.95, 1.0, 1.0);

            // Shadow
            cr.set_source_rgba(0.0, 0.0, 0.0, 0.5);
            cr.move_to(text_x + 2.0, text_y + 2.0);
            pango_show_text(
                cr,
                text,
                &config.header_font,
                cairo::FontSlant::Normal,
                cairo::FontWeight::Bold,
                config.header_font_size,
            );

            // Chrome gradient text
            cr.set_source(&gradient).ok();
            cr.move_to(text_x, text_y);
            pango_show_text(
                cr,
                text,
                &config.header_font,
                cairo::FontSlant::Normal,
                cairo::FontWeight::Bold,
                config.header_font_size,
            );

            // Underline with neon glow
            if config.neon_glow_intensity > 0.0 {
                cr.set_source_rgba(neon.r, neon.g, neon.b, config.neon_glow_intensity * 0.5);
                cr.set_line_width(4.0);
                cr.move_to(text_x - 10.0, text_y + 4.0);
                cr.line_to(text_x + text_width + 10.0, text_y + 4.0);
                cr.stroke().ok();
            }

            cr.set_source_rgba(accent.r, accent.g, accent.b, 0.9);
            cr.set_line_width(1.5);
            cr.move_to(text_x - 10.0, text_y + 4.0);
            cr.line_to(text_x + text_width + 10.0, text_y + 4.0);
            cr.stroke().ok();
        }
        SynthwaveHeaderStyle::Neon => {
            // Neon glow effect
            if config.neon_glow_intensity > 0.0 {
                for i in (1..=3).rev() {
                    let alpha = config.neon_glow_intensity * 0.3 * (4.0 - i as f64) / 3.0;
                    cr.set_source_rgba(neon.r, neon.g, neon.b, alpha);
                    cr.move_to(text_x, text_y);
                    pango_show_text(
                        cr,
                        text,
                        &config.header_font,
                        cairo::FontSlant::Normal,
                        cairo::FontWeight::Bold,
                        config.header_font_size,
                    );
                }
            }

            // Main text
            cr.set_source_rgba(accent.r, accent.g, accent.b, 1.0);
            cr.move_to(text_x, text_y);
            pango_show_text(
                cr,
                text,
                &config.header_font,
                cairo::FontSlant::Normal,
                cairo::FontWeight::Bold,
                config.header_font_size,
            );

            // Bright center
            cr.set_source_rgba(1.0, 1.0, 1.0, 0.6);
            cr.move_to(text_x, text_y);
            pango_show_text(
                cr,
                text,
                &config.header_font,
                cairo::FontSlant::Normal,
                cairo::FontWeight::Bold,
                config.header_font_size,
            );
        }
        SynthwaveHeaderStyle::Outline => {
            // Outlined text using Pango layout path
            cr.set_source_rgba(accent.r, accent.g, accent.b, 1.0);
            let font_desc = crate::ui::pango_text::get_font_description(
                &config.header_font,
                cairo::FontSlant::Normal,
                cairo::FontWeight::Bold,
                config.header_font_size,
            );
            let layout = pangocairo::functions::create_layout(cr);
            layout.set_font_description(Some(&font_desc));
            layout.set_text(text);
            let baseline = layout.baseline() as f64 / pango::SCALE as f64;
            cr.move_to(text_x, text_y - baseline);
            pangocairo::functions::layout_path(cr, &layout);
            cr.set_line_width(2.0);
            cr.stroke_preserve().ok();
            cr.set_source_rgba(0.0, 0.0, 0.0, 0.8);
            cr.fill().ok();
        }
        SynthwaveHeaderStyle::Simple => {
            cr.set_source_rgba(secondary.r, secondary.g, secondary.b, 1.0);
            cr.move_to(text_x, text_y);
            pango_show_text(
                cr,
                text,
                &config.header_font,
                cairo::FontSlant::Normal,
                cairo::FontWeight::Bold,
                config.header_font_size,
            );
        }
        SynthwaveHeaderStyle::None => {}
    }

    cr.restore().ok();

    header_h
}

/// Draw a divider between groups
fn draw_divider(
    cr: &Context,
    config: &SynthwaveFrameConfig,
    x: f64,
    y: f64,
    length: f64,
    horizontal: bool,
) {
    if matches!(config.divider_style, SynthwaveDividerStyle::None) {
        return;
    }

    let accent = config.color_scheme.accent();
    let neon = config.color_scheme.neon();
    let glow = config.neon_glow_intensity;

    cr.save().ok();

    match config.divider_style {
        SynthwaveDividerStyle::NeonLine => {
            // Glow effect
            if glow > 0.0 {
                cr.set_source_rgba(neon.r, neon.g, neon.b, glow * 0.3);
                cr.set_line_width(6.0);
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
            cr.set_source_rgba(accent.r, accent.g, accent.b, 0.8);
            cr.set_line_width(1.5);
            if horizontal {
                cr.move_to(x, y);
                cr.line_to(x + length, y);
            } else {
                cr.move_to(x, y);
                cr.line_to(x, y + length);
            }
            cr.stroke().ok();
        }
        SynthwaveDividerStyle::Gradient => {
            let gradient = if horizontal {
                cairo::LinearGradient::new(x, y, x + length, y)
            } else {
                cairo::LinearGradient::new(x, y, x, y + length)
            };
            gradient.add_color_stop_rgba(0.0, accent.r, accent.g, accent.b, 0.0);
            gradient.add_color_stop_rgba(0.5, accent.r, accent.g, accent.b, 0.6);
            gradient.add_color_stop_rgba(1.0, accent.r, accent.g, accent.b, 0.0);

            cr.set_source(&gradient).ok();
            cr.set_line_width(2.0);
            if horizontal {
                cr.move_to(x, y);
                cr.line_to(x + length, y);
            } else {
                cr.move_to(x, y);
                cr.line_to(x, y + length);
            }
            cr.stroke().ok();
        }
        SynthwaveDividerStyle::NeonDots => {
            let dot_spacing = 8.0;
            let dot_radius = 2.0;
            cr.set_source_rgba(accent.r, accent.g, accent.b, 0.7);

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
        SynthwaveDividerStyle::Line => {
            cr.set_source_rgba(accent.r, accent.g, accent.b, 0.4);
            cr.set_line_width(1.0);
            if horizontal {
                cr.move_to(x, y);
                cr.line_to(x + length, y);
            } else {
                cr.move_to(x, y);
                cr.line_to(x, y + length);
            }
            cr.stroke().ok();
        }
        SynthwaveDividerStyle::None => {}
    }

    cr.restore().ok();
}

/// Get colors for content rendering
pub fn get_synthwave_colors(config: &SynthwaveFrameConfig) -> (Color, Color, Color) {
    (
        config.color_scheme.primary(),
        config.color_scheme.secondary(),
        config.color_scheme.accent(),
    )
}

/// Render animated scanline overlay effect (CRT monitor style)
///
/// `scanline_offset` should be a value that increases over time (0.0 to 100.0, wrapping)
/// to create the animated scrolling effect.
pub fn render_scanline_overlay(
    cr: &Context,
    config: &SynthwaveFrameConfig,
    width: f64,
    height: f64,
    scanline_offset: f64,
) {
    if !config.scanline_effect {
        return;
    }

    cr.save().ok();

    let accent = config.color_scheme.accent();

    // Draw moving scanline band (bright line that moves down the screen)
    let band_height = 3.0;
    let band_y = (scanline_offset / 100.0 * height) % height;

    // Create gradient for the scanline band (fades at edges)
    let gradient = cairo::LinearGradient::new(0.0, band_y - band_height, 0.0, band_y + band_height);
    gradient.add_color_stop_rgba(0.0, accent.r, accent.g, accent.b, 0.0);
    gradient.add_color_stop_rgba(0.5, accent.r, accent.g, accent.b, 0.15);
    gradient.add_color_stop_rgba(1.0, accent.r, accent.g, accent.b, 0.0);

    cr.set_source(&gradient).ok();
    cr.rectangle(0.0, band_y - band_height, width, band_height * 2.0);
    cr.fill().ok();

    // Draw subtle static scanlines (every other pixel row)
    cr.set_source_rgba(0.0, 0.0, 0.0, 0.08);
    let mut y = 0.0;
    while y < height {
        cr.rectangle(0.0, y, width, 1.0);
        y += 2.0;
    }
    cr.fill().ok();

    cr.restore().ok();
}

/// Render the complete Synthwave frame
/// Returns the content area bounds (x, y, width, height)
pub fn render_synthwave_frame(
    cr: &Context,
    config: &SynthwaveFrameConfig,
    width: f64,
    height: f64,
) -> Result<(f64, f64, f64, f64)> {
    // Guard against invalid dimensions
    if width < 1.0 || height < 1.0 {
        return Ok((0.0, 0.0, 0.0, 0.0));
    }

    cr.save()?;

    // Draw background gradient
    draw_background(cr, config, width, height);

    // Draw sun (behind grid)
    draw_sun(cr, config, width, height);

    // Draw grid
    draw_grid(cr, config, width, height);

    // Draw frame
    draw_frame(cr, config, width, height);

    // Draw header
    let frame_margin = config.frame_width + 4.0;
    let header_height = draw_header(
        cr,
        config,
        frame_margin,
        frame_margin,
        width - frame_margin * 2.0,
    );

    cr.restore()?;

    // Calculate content area
    let content_x = config.content_padding;
    let content_y = frame_margin + header_height + config.content_padding * 0.5;
    let content_w = width - config.content_padding * 2.0;
    let content_h = height - content_y - config.content_padding;

    Ok((content_x, content_y, content_w.max(0.0), content_h.max(0.0)))
}

/// Calculate group layouts within content area
pub fn calculate_group_layouts(
    config: &SynthwaveFrameConfig,
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
    config: &SynthwaveFrameConfig,
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
