use gtk4::cairo;
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

/// Tessellated polygons configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PolygonConfig {
    pub polygon_size: u32,    // Approximate size of each polygon
    pub color_variation: f64, // How much colors vary (0.0 to 1.0)
    pub base_colors: Vec<Color>,
    pub seed: u32, // Random seed for reproducibility
}

impl Default for PolygonConfig {
    fn default() -> Self {
        Self {
            polygon_size: 100,
            color_variation: 0.2,
            base_colors: vec![
                Color::new(0.15, 0.15, 0.2, 1.0),
                Color::new(0.1, 0.1, 0.15, 1.0),
            ],
            seed: 42,
        }
    }
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
    Image { path: String, stretch: bool },
    #[serde(rename = "polygons")]
    Polygons(PolygonConfig),
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
pub struct BackgroundConfig {
    pub background: BackgroundType,
}

impl Default for BackgroundConfig {
    fn default() -> Self {
        Self {
            background: BackgroundType::default(),
        }
    }
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
        BackgroundType::Image { path, stretch } => {
            render_image_background(cr, path, *stretch, width, height)?;
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
    stretch: bool,
    width: f64,
    height: f64,
) -> Result<(), cairo::Error> {
    use gtk4::gdk_pixbuf::Pixbuf;

    if let Ok(pixbuf) = Pixbuf::from_file(path) {
        let img_width = pixbuf.width() as f64;
        let img_height = pixbuf.height() as f64;

        cr.save()?;

        if stretch {
            // Stretch to fill
            cr.scale(width / img_width, height / img_height);
        } else {
            // Scale to fit (maintain aspect ratio)
            let scale = (width / img_width).min(height / img_height);
            cr.scale(scale, scale);
            cr.translate(
                (width / scale - img_width) / 2.0,
                (height / scale - img_height) / 2.0,
            );
        }

        cr.set_source_pixbuf(&pixbuf, 0.0, 0.0);
        cr.paint()?;
        cr.restore()?;
    } else {
        // Fallback to solid color if image can't be loaded
        cr.set_source_rgb(0.2, 0.2, 0.2);
        cr.rectangle(0.0, 0.0, width, height);
        cr.fill()?;
    }

    Ok(())
}

/// Render tessellated polygon background using Voronoi-like pattern
fn render_polygon_background(
    cr: &cairo::Context,
    config: &PolygonConfig,
    width: f64,
    height: f64,
) -> Result<(), cairo::Error> {
    use rand::{Rng, SeedableRng};
    use rand::rngs::StdRng;

    let mut rng = StdRng::seed_from_u64(config.seed as u64);

    // Generate points for Voronoi cells
    let cols = ((width / config.polygon_size as f64).ceil() as usize).max(2);
    let rows = ((height / config.polygon_size as f64).ceil() as usize).max(2);

    let mut points = Vec::new();
    for row in 0..rows {
        for col in 0..cols {
            let base_x = (col as f64 + 0.5) * width / cols as f64;
            let base_y = (row as f64 + 0.5) * height / rows as f64;

            // Add some randomness
            let jitter_x = rng.gen_range(-width / cols as f64 * 0.4..width / cols as f64 * 0.4);
            let jitter_y = rng.gen_range(-height / rows as f64 * 0.4..height / rows as f64 * 0.4);

            points.push((base_x + jitter_x, base_y + jitter_y));
        }
    }

    // For simplicity, draw rectangles for now (full Voronoi would be complex)
    // We can enhance this later
    for (i, (px, py)) in points.iter().enumerate() {
        // Pick a base color
        let base_color = &config.base_colors[i % config.base_colors.len()];

        // Add variation
        let variation = config.color_variation;
        let r = (base_color.r + rng.gen_range(-variation..variation)).clamp(0.0, 1.0);
        let g = (base_color.g + rng.gen_range(-variation..variation)).clamp(0.0, 1.0);
        let b = (base_color.b + rng.gen_range(-variation..variation)).clamp(0.0, 1.0);

        cr.set_source_rgba(r, g, b, base_color.a);

        // Draw a polygon (simplified as rectangle for now)
        let size = config.polygon_size as f64 * 0.8;
        cr.rectangle(px - size / 2.0, py - size / 2.0, size, size);
        cr.fill()?;
    }

    Ok(())
}
