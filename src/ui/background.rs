use gtk4::cairo;

use crate::ui::theme::ComboThemeConfig;

// Re-export all background/color types from rg-sens-types for backward compatibility
pub use rg_sens_types::background::{
    BackgroundConfig, BackgroundType, ImageDisplayMode, IndicatorBackgroundConfig,
    IndicatorBackgroundShape, PolygonConfig,
};
pub use rg_sens_types::color::{Color, ColorStop, LinearGradientConfig, RadialGradientConfig};
pub use rg_sens_types::theme::ColorSource;

/// Render a background to a Cairo context
pub fn render_background(
    cr: &cairo::Context,
    config: &BackgroundConfig,
    width: f64,
    height: f64,
) -> Result<(), cairo::Error> {
    render_background_with_theme(cr, config, width, height, None)
}

/// Render a background to a Cairo context with theme support for polygon colors
pub fn render_background_with_theme(
    cr: &cairo::Context,
    config: &BackgroundConfig,
    width: f64,
    height: f64,
    theme: Option<&ComboThemeConfig>,
) -> Result<(), cairo::Error> {
    match &config.background {
        BackgroundType::Solid { color } => {
            let resolved_color = if let Some(t) = theme {
                color.resolve(t)
            } else {
                color.resolve(&ComboThemeConfig::default())
            };
            resolved_color.apply_to_cairo(cr);
            cr.rectangle(0.0, 0.0, width, height);
            cr.fill()?;
        }
        BackgroundType::LinearGradient(grad) => {
            render_linear_gradient(cr, grad, width, height)?;
        }
        BackgroundType::RadialGradient(grad) => {
            render_radial_gradient(cr, grad, width, height)?;
        }
        BackgroundType::Image {
            path,
            display_mode,
            alpha,
        } => {
            render_image_background(cr, path, *display_mode, *alpha, width, height)?;
        }
        BackgroundType::Polygons(poly) => {
            render_polygon_background(cr, poly, width, height, theme)?;
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

/// Render a background with source values and theme support
pub fn render_background_with_source_and_theme(
    cr: &cairo::Context,
    config: &BackgroundConfig,
    width: f64,
    height: f64,
    source_values: &std::collections::HashMap<String, serde_json::Value>,
    theme: Option<&ComboThemeConfig>,
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
        // For non-indicator backgrounds, render with theme
        _ => {
            render_background_with_theme(cr, config, width, height, theme)?;
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
    use crate::ui::render_cache::{get_cached_scaled_surface, get_cached_tile_surface};

    let target_width = width as i32;
    let target_height = height as i32;

    // For Tile mode, use the tile surface cache
    if matches!(display_mode, ImageDisplayMode::Tile) {
        if let Some(surface) = get_cached_tile_surface(path) {
            cr.save()?;
            cr.rectangle(0.0, 0.0, width, height);
            cr.clip();

            let pattern = cairo::SurfacePattern::create(&surface);
            pattern.set_extend(cairo::Extend::Repeat);
            cr.set_source(&pattern)?;
            cr.paint_with_alpha(alpha)?;

            cr.restore()?;
        } else {
            // Fallback to solid color if image can't be loaded
            cr.set_source_rgb(0.2, 0.2, 0.2);
            cr.rectangle(0.0, 0.0, width, height);
            cr.fill()?;
        }
        return Ok(());
    }

    // For Fit/Stretch/Zoom modes, use pre-scaled surface cache
    // This avoids expensive set_source_pixbuf + scale on every frame
    let mode_code = match display_mode {
        ImageDisplayMode::Fit => 0,
        ImageDisplayMode::Stretch => 1,
        ImageDisplayMode::Zoom => 2,
        ImageDisplayMode::Tile => unreachable!(),
    };

    if let Some(scaled_surface) =
        get_cached_scaled_surface(path, target_width, target_height, mode_code, alpha)
    {
        // Fast path: just paint the pre-scaled, pre-alpha'd surface
        cr.set_source_surface(&scaled_surface, 0.0, 0.0)?;
        cr.paint()?;
        // Clear source reference to prevent GL texture memory leak
        cr.set_source_rgba(0.0, 0.0, 0.0, 0.0);
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
    theme: Option<&ComboThemeConfig>,
) -> Result<(), cairo::Error> {
    if config.colors.is_empty() {
        return Ok(());
    }

    // Resolve colors from ColorSource using theme (or default if no theme provided)
    let default_theme = ComboThemeConfig::default();
    let theme = theme.unwrap_or(&default_theme);
    let resolved_colors: Vec<Color> = config.colors.iter().map(|cs| cs.resolve(theme)).collect();

    // Fill background color first (fills gaps between polygons)
    let bg_color = config.background_color.resolve(theme);
    bg_color.apply_to_cairo(cr);
    cr.rectangle(0.0, 0.0, width, height);
    cr.fill()?;

    let size = config.tile_size as f64;
    let sides = config.num_sides.max(3); // Minimum 3 sides
    let angle = config.rotation_angle.to_radians();

    match sides {
        3 => render_triangle_tiling(cr, size, angle, &resolved_colors, width, height)?,
        4 => render_square_tiling(cr, size, angle, &resolved_colors, width, height)?,
        6 => render_hexagon_tiling(cr, size, angle, &resolved_colors, width, height)?,
        _ => {
            render_generic_polygon_tiling(cr, size, sides, angle, &resolved_colors, width, height)?
        }
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
/// Uses cached LUT for efficient repeated lookups
fn interpolate_indicator_gradient(stops: &[ColorStop], value: f64, min: f64, max: f64) -> Color {
    use crate::ui::render_cache::get_cached_color_at;

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

    // Use cached LUT for fast color lookup
    // The LUT handles unsorted stops internally (sorts once during construction)
    get_cached_color_at(stops, normalized)
}
