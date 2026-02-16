//! Shared rendering utilities for UI components

use gtk4::cairo::Context;

/// Render a checkerboard pattern to show transparency
///
/// This is commonly used as a background in color/gradient previews
/// to make transparent areas visible.
pub fn render_checkerboard(cr: &Context, width: f64, height: f64) {
    let square_size = 10.0;
    let light_gray = 0.8;
    let dark_gray = 0.6;

    for y in 0..((height / square_size).ceil() as i32) {
        for x in 0..((width / square_size).ceil() as i32) {
            let is_light = (x + y) % 2 == 0;
            let gray = if is_light { light_gray } else { dark_gray };

            cr.set_source_rgb(gray, gray, gray);
            cr.rectangle(
                x as f64 * square_size,
                y as f64 * square_size,
                square_size,
                square_size,
            );
            let _ = cr.fill();
        }
    }
}
