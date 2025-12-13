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
