use gtk4::cairo;
use gtk4::prelude::GdkCairoContextExt;
use serde::{Deserialize, Serialize};

/// RGBA color with alpha channel
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

impl Color {
    pub fn new(r: f64, g: f64, b: f64, a: f64) -> Self {
        Self { r, g, b, a }
    }

    pub fn from_rgba8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            r: r as f64 / 255.0,
            g: g as f64 / 255.0,
            b: b as f64 / 255.0,
            a: a as f64 / 255.0,
        }
    }

    pub fn to_rgba8(&self) -> (u8, u8, u8, u8) {
        (
            (self.r * 255.0) as u8,
            (self.g * 255.0) as u8,
            (self.b * 255.0) as u8,
            (self.a * 255.0) as u8,
        )
    }

    /// Convert to GTK RGBA
    pub fn to_gdk_rgba(&self) -> gtk4::gdk::RGBA {
        gtk4::gdk::RGBA::new(self.r as f32, self.g as f32, self.b as f32, self.a as f32)
    }

    /// Create from GTK RGBA
    pub fn from_gdk_rgba(rgba: &gtk4::gdk::RGBA) -> Self {
        Self {
            r: rgba.red() as f64,
            g: rgba.green() as f64,
            b: rgba.blue() as f64,
            a: rgba.alpha() as f64,
        }
    }

    /// Apply to Cairo context
    pub fn apply_to_cairo(&self, cr: &cairo::Context) {
        cr.set_source_rgba(self.r, self.g, self.b, self.a);
    }
}

impl Default for Color {
    fn default() -> Self {
        Self::new(0.0, 0.0, 0.0, 1.0)
    }
}

/// Color stop for gradients
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ColorStop {
    pub position: f64, // 0.0 to 1.0
    pub color: Color,
}

impl ColorStop {
    pub fn new(position: f64, color: Color) -> Self {
        Self { position, color }
    }
}

/// Linear gradient configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LinearGradientConfig {
    pub angle: f64, // Angle in degrees (0 = left to right, 90 = top to bottom)
    pub stops: Vec<ColorStop>,
}

impl Default for LinearGradientConfig {
    fn default() -> Self {
        Self {
            angle: 90.0,
            stops: vec![
                ColorStop::new(0.0, Color::new(0.2, 0.2, 0.2, 1.0)),
                ColorStop::new(1.0, Color::new(0.1, 0.1, 0.1, 1.0)),
            ],
        }
    }
}

/// Radial gradient configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RadialGradientConfig {
    pub center_x: f64, // 0.0 to 1.0 (relative to width)
    pub center_y: f64, // 0.0 to 1.0 (relative to height)
    pub radius: f64,   // 0.0 to 1.0 (relative to diagonal)
    pub stops: Vec<ColorStop>,
}

impl Default for RadialGradientConfig {
    fn default() -> Self {
        Self {
            center_x: 0.5,
            center_y: 0.5,
            radius: 0.7,
            stops: vec![
                ColorStop::new(0.0, Color::new(0.3, 0.3, 0.3, 1.0)),
                ColorStop::new(1.0, Color::new(0.1, 0.1, 0.1, 1.0)),
            ],
        }
    }
}

/// Tiling polygons configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PolygonConfig {
    pub tile_size: u32,           // Size of each tile
    pub num_sides: u32,            // Number of sides (3=triangle, 4=square, 5=pentagon, 6=hexagon, etc.)
    pub rotation_angle: f64,       // Rotation angle in degrees
    pub colors: Vec<Color>,        // Colors that alternate for tiles
}

impl Default for PolygonConfig {
    fn default() -> Self {
        Self {
            tile_size: 60,
            num_sides: 6, // Hexagons by default
            rotation_angle: 0.0,
            colors: vec![
                Color::new(0.2, 0.2, 0.25, 1.0),
                Color::new(0.15, 0.15, 0.2, 1.0),
            ],
        }
    }
}

/// Image display mode
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[derive(Default)]
pub enum ImageDisplayMode {
    #[serde(rename = "fit")]
    #[default]
    Fit,       // Scale to fit (maintain aspect ratio, may have empty space)
    #[serde(rename = "stretch")]
    Stretch,   // Stretch to fill (may distort image)
    #[serde(rename = "zoom")]
    Zoom,      // Scale to fill (maintain aspect ratio, may crop)
    #[serde(rename = "tile")]
    Tile,      // Tile/repeat the image
}


/// Background type configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum BackgroundType {
    #[serde(rename = "solid")]
    Solid { color: Color },
    #[serde(rename = "linear_gradient")]
    LinearGradient(LinearGradientConfig),
    #[serde(rename = "radial_gradient")]
    RadialGradient(RadialGradientConfig),
    #[serde(rename = "image")]
    Image {
        path: String,
        #[serde(default)]
        display_mode: ImageDisplayMode,
        #[serde(default = "default_alpha")]
        alpha: f64, // 0.0 to 1.0
    },
    #[serde(rename = "polygons")]
    Polygons(PolygonConfig),
    #[serde(rename = "indicator")]
    Indicator(IndicatorBackgroundConfig),
}

/// Configuration for indicator background (value-based color from gradient)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IndicatorBackgroundConfig {
    /// Gradient stops defining the color mapping (position 0.0 = 0%, position 1.0 = 100%)
    #[serde(default = "default_indicator_gradient")]
    pub gradient_stops: Vec<ColorStop>,
    /// Shape to display
    #[serde(default)]
    pub shape: IndicatorBackgroundShape,
    /// Size of shape (0.0-1.0 relative to panel)
    #[serde(default = "default_indicator_size")]
    pub shape_size: f64,
    /// Rotation angle in degrees
    #[serde(default)]
    pub rotation_angle: f64,
    /// Border width
    #[serde(default)]
    pub border_width: f64,
    /// Border color
    #[serde(default = "default_indicator_border_color")]
    pub border_color: Color,
    /// Static value to use when no live data available (0-100)
    #[serde(default = "default_indicator_value")]
    pub static_value: f64,
    /// Field to bind to for live value updates
    #[serde(default)]
    pub value_field: String,
    /// Min value for mapping
    #[serde(default)]
    pub min_value: f64,
    /// Max value for mapping
    #[serde(default = "default_indicator_max")]
    pub max_value: f64,
}

fn default_indicator_gradient() -> Vec<ColorStop> {
    vec![
        ColorStop::new(0.0, Color::new(0.0, 0.5, 1.0, 1.0)),   // Blue at 0%
        ColorStop::new(0.4, Color::new(0.0, 1.0, 0.0, 1.0)),   // Green at 40%
        ColorStop::new(0.7, Color::new(1.0, 1.0, 0.0, 1.0)),   // Yellow at 70%
        ColorStop::new(1.0, Color::new(1.0, 0.0, 0.0, 1.0)),   // Red at 100%
    ]
}

fn default_indicator_size() -> f64 {
    0.8
}

fn default_indicator_border_color() -> Color {
    Color::new(1.0, 1.0, 1.0, 0.5)
}

fn default_indicator_value() -> f64 {
    50.0
}

fn default_indicator_max() -> f64 {
    100.0
}

/// Shape for indicator background
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum IndicatorBackgroundShape {
    #[default]
    Fill,
    Circle,
    Square,
    Polygon(u32),
}

impl Default for IndicatorBackgroundConfig {
    fn default() -> Self {
        Self {
            gradient_stops: default_indicator_gradient(),
            shape: IndicatorBackgroundShape::default(),
            shape_size: default_indicator_size(),
            rotation_angle: 0.0,
            border_width: 0.0,
            border_color: default_indicator_border_color(),
            static_value: default_indicator_value(),
            value_field: "value".to_string(),
            min_value: 0.0,
            max_value: 100.0,
        }
    }
}

fn default_alpha() -> f64 {
    1.0
}

impl Default for BackgroundType {
    fn default() -> Self {
        Self::Solid {
            color: Color::new(0.15, 0.15, 0.15, 1.0),
        }
    }
}

/// Background configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[derive(Default)]
pub struct BackgroundConfig {
    pub background: BackgroundType,
}


/// Render a background to a Cairo context
pub fn render_background(
    cr: &cairo::Context,
    config: &BackgroundConfig,
    width: f64,
    height: f64,
) -> Result<(), cairo::Error> {
    match &config.background {
        BackgroundType::Solid { color } => {
            color.apply_to_cairo(cr);
            cr.rectangle(0.0, 0.0, width, height);
            cr.fill()?;
        }
        BackgroundType::LinearGradient(grad) => {
            render_linear_gradient(cr, grad, width, height)?;
        }
        BackgroundType::RadialGradient(grad) => {
            render_radial_gradient(cr, grad, width, height)?;
        }
        BackgroundType::Image { path, display_mode, alpha } => {
            render_image_background(cr, path, *display_mode, *alpha, width, height)?;
        }
        BackgroundType::Polygons(poly) => {
            render_polygon_background(cr, poly, width, height)?;
        }
        BackgroundType::Indicator(indicator) => {
            render_indicator_background(cr, indicator, width, height)?;
        }
    }
    Ok(())
}

/// Render a background with source values (for indicator backgrounds that use live data)
pub fn render_background_with_source(
    cr: &cairo::Context,
    config: &BackgroundConfig,
    width: f64,
    height: f64,
    source_values: &std::collections::HashMap<String, serde_json::Value>,
) -> Result<(), cairo::Error> {
    match &config.background {
        BackgroundType::Indicator(indicator) => {
            // Get value from source based on value_field
            let value = if !indicator.value_field.is_empty() {
                source_values
                    .get(&indicator.value_field)
                    .and_then(|v| v.as_f64())
                    .unwrap_or(indicator.static_value)
            } else {
                indicator.static_value
            };
            render_indicator_background_with_value(cr, indicator, value, width, height)?;
        }
        // For non-indicator backgrounds, just render normally
        _ => {
            render_background(cr, config, width, height)?;
        }
    }
    Ok(())
}

/// Render a linear gradient
fn render_linear_gradient(
    cr: &cairo::Context,
    config: &LinearGradientConfig,
    width: f64,
    height: f64,
) -> Result<(), cairo::Error> {
    // Convert angle to radians
    let angle_rad = config.angle.to_radians();

    // Calculate gradient vector
    let diagonal = (width * width + height * height).sqrt();
    let x1 = width / 2.0 - diagonal * angle_rad.cos() / 2.0;
    let y1 = height / 2.0 - diagonal * angle_rad.sin() / 2.0;
    let x2 = width / 2.0 + diagonal * angle_rad.cos() / 2.0;
    let y2 = height / 2.0 + diagonal * angle_rad.sin() / 2.0;

    let pattern = cairo::LinearGradient::new(x1, y1, x2, y2);

    for stop in &config.stops {
        pattern.add_color_stop_rgba(
            stop.position,
            stop.color.r,
            stop.color.g,
            stop.color.b,
            stop.color.a,
        );
    }

    cr.set_source(&pattern)?;
    cr.rectangle(0.0, 0.0, width, height);
    cr.fill()?;

    Ok(())
}

/// Render a radial gradient
fn render_radial_gradient(
    cr: &cairo::Context,
    config: &RadialGradientConfig,
    width: f64,
    height: f64,
) -> Result<(), cairo::Error> {
    let cx = width * config.center_x;
    let cy = height * config.center_y;
    let diagonal = (width * width + height * height).sqrt();
    let radius = diagonal * config.radius;

    let pattern = cairo::RadialGradient::new(cx, cy, 0.0, cx, cy, radius);

    for stop in &config.stops {
        pattern.add_color_stop_rgba(
            stop.position,
            stop.color.r,
            stop.color.g,
            stop.color.b,
            stop.color.a,
        );
    }

    cr.set_source(&pattern)?;
    cr.rectangle(0.0, 0.0, width, height);
    cr.fill()?;

    Ok(())
}

/// Render an image background
fn render_image_background(
    cr: &cairo::Context,
    path: &str,
    display_mode: ImageDisplayMode,
    alpha: f64,
    width: f64,
    height: f64,
) -> Result<(), cairo::Error> {
    use crate::ui::render_cache::{get_cached_pixbuf, get_cached_tile_surface};

    // Use cached image loading
    if let Some(pixbuf) = get_cached_pixbuf(path) {
        let img_width = pixbuf.width() as f64;
        let img_height = pixbuf.height() as f64;

        cr.save()?;

        match display_mode {
            ImageDisplayMode::Fit => {
                // Scale to fit (maintain aspect ratio, may have empty space)
                let scale = (width / img_width).min(height / img_height);
                cr.scale(scale, scale);
                cr.translate(
                    (width / scale - img_width) / 2.0,
                    (height / scale - img_height) / 2.0,
                );
                cr.set_source_pixbuf(&pixbuf, 0.0, 0.0);
                cr.paint_with_alpha(alpha)?;
            }
            ImageDisplayMode::Stretch => {
                // Stretch to fill (may distort)
                cr.scale(width / img_width, height / img_height);
                cr.set_source_pixbuf(&pixbuf, 0.0, 0.0);
                cr.paint_with_alpha(alpha)?;
            }
            ImageDisplayMode::Zoom => {
                // Scale to fill (maintain aspect ratio, may crop)
                let scale = (width / img_width).max(height / img_height);
                cr.scale(scale, scale);
                cr.translate(
                    (width / scale - img_width) / 2.0,
                    (height / scale - img_height) / 2.0,
                );
                cr.set_source_pixbuf(&pixbuf, 0.0, 0.0);
                cr.paint_with_alpha(alpha)?;
            }
            ImageDisplayMode::Tile => {
                // Tile the image - use cached tile surface
                if let Some(surface) = get_cached_tile_surface(path) {
                    let pattern = cairo::SurfacePattern::create(&surface);
                    pattern.set_extend(cairo::Extend::Repeat);
                    cr.set_source(&pattern)?;
                    cr.paint_with_alpha(alpha)?;
                } else {
                    // Fallback: create surface on the fly
                    let surface = cairo::ImageSurface::create(
                        cairo::Format::ARgb32,
                        img_width as i32,
                        img_height as i32,
                    )?;
                    let tmp_cr = cairo::Context::new(&surface)?;
                    tmp_cr.set_source_pixbuf(&pixbuf, 0.0, 0.0);
                    tmp_cr.paint()?;

                    let pattern = cairo::SurfacePattern::create(&surface);
                    pattern.set_extend(cairo::Extend::Repeat);
                    cr.set_source(&pattern)?;
                    cr.paint_with_alpha(alpha)?;
                }
            }
        }

        cr.restore()?;
    } else {
        // Fallback to solid color if image can't be loaded
        cr.set_source_rgb(0.2, 0.2, 0.2);
        cr.rectangle(0.0, 0.0, width, height);
        cr.fill()?;
    }

    Ok(())
}

/// Render tiling polygon background
fn render_polygon_background(
    cr: &cairo::Context,
    config: &PolygonConfig,
    width: f64,
    height: f64,
) -> Result<(), cairo::Error> {
    if config.colors.is_empty() {
        return Ok(());
    }

    let size = config.tile_size as f64;
    let sides = config.num_sides.max(3); // Minimum 3 sides
    let angle = config.rotation_angle.to_radians();

    match sides {
        3 => render_triangle_tiling(cr, size, angle, &config.colors, width, height)?,
        4 => render_square_tiling(cr, size, angle, &config.colors, width, height)?,
        6 => render_hexagon_tiling(cr, size, angle, &config.colors, width, height)?,
        _ => render_generic_polygon_tiling(cr, size, sides, angle, &config.colors, width, height)?,
    }

    Ok(())
}

/// Draw a regular polygon
fn draw_polygon(cr: &cairo::Context, cx: f64, cy: f64, radius: f64, sides: u32, rotation: f64) {
    let angle_step = 2.0 * std::f64::consts::PI / sides as f64;

    for i in 0..sides {
        let angle = rotation + i as f64 * angle_step;
        let x = cx + radius * angle.cos();
        let y = cy + radius * angle.sin();

        if i == 0 {
            cr.move_to(x, y);
        } else {
            cr.line_to(x, y);
        }
    }
    cr.close_path();
}

/// Render triangle tiling
fn render_triangle_tiling(
    cr: &cairo::Context,
    size: f64,
    rotation: f64,
    colors: &[Color],
    width: f64,
    height: f64,
) -> Result<(), cairo::Error> {
    let height_step = size * 0.866; // sqrt(3)/2
    let rows = (height / height_step).ceil() as i32 + 2;
    let cols = (width / size).ceil() as i32 + 2;

    let mut color_index = 0;
    for row in -1..rows {
        for col in -1..cols {
            let x = col as f64 * size + if row % 2 == 1 { size / 2.0 } else { 0.0 };
            let y = row as f64 * height_step;

            colors[color_index % colors.len()].apply_to_cairo(cr);
            draw_polygon(cr, x, y, size * 0.577, 3, rotation); // 0.577 ≈ 1/sqrt(3)
            cr.fill()?;

            color_index += 1;
        }
    }
    Ok(())
}

/// Render square tiling
fn render_square_tiling(
    cr: &cairo::Context,
    size: f64,
    rotation: f64,
    colors: &[Color],
    width: f64,
    height: f64,
) -> Result<(), cairo::Error> {
    let rows = (height / size).ceil() as i32 + 2;
    let cols = (width / size).ceil() as i32 + 2;

    let mut color_index = 0;
    for row in -1..rows {
        for col in -1..cols {
            let x = col as f64 * size + size / 2.0;
            let y = row as f64 * size + size / 2.0;

            colors[color_index % colors.len()].apply_to_cairo(cr);
            draw_polygon(cr, x, y, size * 0.707, 4, rotation); // 0.707 ≈ 1/sqrt(2)
            cr.fill()?;

            color_index += 1;
        }
    }
    Ok(())
}

/// Render hexagon tiling (honeycomb pattern)
fn render_hexagon_tiling(
    cr: &cairo::Context,
    size: f64,
    rotation: f64,
    colors: &[Color],
    width: f64,
    height: f64,
) -> Result<(), cairo::Error> {
    let hex_width = size * 1.5;
    let hex_height = size * 0.866 * 2.0; // sqrt(3) * size

    let rows = (height / (hex_height * 0.75)).ceil() as i32 + 2;
    let cols = (width / hex_width).ceil() as i32 + 2;

    let mut color_index = 0;
    for row in -1..rows {
        for col in -1..cols {
            let x = col as f64 * hex_width + if row % 2 == 1 { hex_width / 2.0 } else { 0.0 };
            let y = row as f64 * hex_height * 0.75;

            colors[color_index % colors.len()].apply_to_cairo(cr);
            draw_polygon(cr, x, y, size, 6, rotation);
            cr.fill()?;

            color_index += 1;
        }
    }
    Ok(())
}

/// Render generic polygon tiling (best effort, may have gaps)
fn render_generic_polygon_tiling(
    cr: &cairo::Context,
    size: f64,
    sides: u32,
    rotation: f64,
    colors: &[Color],
    width: f64,
    height: f64,
) -> Result<(), cairo::Error> {
    let spacing = size * 1.2;
    let rows = (height / spacing).ceil() as i32 + 2;
    let cols = (width / spacing).ceil() as i32 + 2;

    let mut color_index = 0;
    for row in -1..rows {
        for col in -1..cols {
            let x = col as f64 * spacing + if row % 2 == 1 { spacing / 2.0 } else { 0.0 };
            let y = row as f64 * spacing;

            colors[color_index % colors.len()].apply_to_cairo(cr);
            draw_polygon(cr, x, y, size * 0.5, sides, rotation);
            cr.fill()?;

            color_index += 1;
        }
    }
    Ok(())
}

/// Render indicator background with value-based coloring
fn render_indicator_background(
    cr: &cairo::Context,
    config: &IndicatorBackgroundConfig,
    width: f64,
    height: f64,
) -> Result<(), cairo::Error> {
    let color = interpolate_indicator_gradient(
        &config.gradient_stops,
        config.static_value,
        config.min_value,
        config.max_value,
    );

    cr.save()?;

    match config.shape {
        IndicatorBackgroundShape::Fill => {
            color.apply_to_cairo(cr);
            cr.rectangle(0.0, 0.0, width, height);
            cr.fill()?;
        }
        IndicatorBackgroundShape::Circle => {
            let center_x = width / 2.0;
            let center_y = height / 2.0;
            let radius = (width.min(height) / 2.0) * config.shape_size;

            color.apply_to_cairo(cr);
            cr.arc(center_x, center_y, radius, 0.0, std::f64::consts::TAU);
            cr.fill()?;

            if config.border_width > 0.0 {
                config.border_color.apply_to_cairo(cr);
                cr.set_line_width(config.border_width);
                cr.arc(center_x, center_y, radius, 0.0, std::f64::consts::TAU);
                cr.stroke()?;
            }
        }
        IndicatorBackgroundShape::Square => {
            let center_x = width / 2.0;
            let center_y = height / 2.0;
            let size = width.min(height) * config.shape_size;

            cr.translate(center_x, center_y);
            cr.rotate(config.rotation_angle.to_radians());

            color.apply_to_cairo(cr);
            cr.rectangle(-size / 2.0, -size / 2.0, size, size);
            cr.fill()?;

            if config.border_width > 0.0 {
                config.border_color.apply_to_cairo(cr);
                cr.set_line_width(config.border_width);
                cr.rectangle(-size / 2.0, -size / 2.0, size, size);
                cr.stroke()?;
            }
        }
        IndicatorBackgroundShape::Polygon(sides) => {
            let center_x = width / 2.0;
            let center_y = height / 2.0;
            let radius = (width.min(height) / 2.0) * config.shape_size;
            let sides = sides.max(3);

            cr.translate(center_x, center_y);
            cr.rotate(config.rotation_angle.to_radians());

            // Draw polygon centered at origin
            let angle_step = std::f64::consts::TAU / sides as f64;
            let start_angle = -std::f64::consts::FRAC_PI_2;

            for i in 0..sides {
                let angle = start_angle + i as f64 * angle_step;
                let x = radius * angle.cos();
                let y = radius * angle.sin();

                if i == 0 {
                    cr.move_to(x, y);
                } else {
                    cr.line_to(x, y);
                }
            }
            cr.close_path();

            color.apply_to_cairo(cr);
            cr.fill_preserve()?;

            if config.border_width > 0.0 {
                config.border_color.apply_to_cairo(cr);
                cr.set_line_width(config.border_width);
                cr.stroke()?;
            } else {
                cr.new_path();
            }
        }
    }

    cr.restore()?;
    Ok(())
}

/// Render indicator background with a dynamic value (for panels that update)
pub fn render_indicator_background_with_value(
    cr: &cairo::Context,
    config: &IndicatorBackgroundConfig,
    value: f64,
    width: f64,
    height: f64,
) -> Result<(), cairo::Error> {
    let mut config_copy = config.clone();
    config_copy.static_value = value;
    render_indicator_background(cr, &config_copy, width, height)
}

/// Interpolate gradient for indicator background
fn interpolate_indicator_gradient(stops: &[ColorStop], value: f64, min: f64, max: f64) -> Color {
    if stops.is_empty() {
        return Color::new(0.5, 0.5, 0.5, 1.0);
    }

    if stops.len() == 1 {
        return stops[0].color;
    }

    // Normalize value to 0.0-1.0 range
    let range = max - min;
    let normalized = if range > 0.0 {
        ((value - min) / range).clamp(0.0, 1.0)
    } else {
        0.5
    };

    // Find the two stops to interpolate between
    let mut sorted_stops: Vec<&ColorStop> = stops.iter().collect();
    // Use unwrap_or to handle NaN values safely (NaN positions sort as equal)
    sorted_stops.sort_by(|a, b| a.position.partial_cmp(&b.position).unwrap_or(std::cmp::Ordering::Equal));

    // Handle edge cases - use first/last references to avoid repeated lookups
    let first = &sorted_stops[0];
    let last = sorted_stops.last().unwrap();
    if normalized <= first.position {
        return first.color;
    }
    if normalized >= last.position {
        return last.color;
    }

    // Find surrounding stops and interpolate
    for i in 0..sorted_stops.len() - 1 {
        let start = sorted_stops[i];
        let end = sorted_stops[i + 1];

        if normalized >= start.position && normalized <= end.position {
            let segment_range = end.position - start.position;
            let t = if segment_range > 0.0 {
                (normalized - start.position) / segment_range
            } else {
                0.0
            };

            return Color::new(
                start.color.r + (end.color.r - start.color.r) * t,
                start.color.g + (end.color.g - start.color.g) * t,
                start.color.b + (end.color.b - start.color.b) * t,
                start.color.a + (end.color.a - start.color.a) * t,
            );
        }
    }

    sorted_stops[0].color
}
