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

/// The middle of the step of a `count`-segment display that `value` is on:
/// it lights the segments `value` lights (`(value * count).ceil()` of them,
/// as every segmented renderer counts), and so do all values on that step.
///
/// A glide moves a value a little on each frame, but a segmented display
/// only changes when another segment lights. Drawing with the step instead of
/// the value gives the same picture, and lets a combo panel see that an item
/// has not changed (the step is what goes into its layer key, see
/// `layer_canvas` in the main crate). The middle is used so that rounding in
/// `value * count` cannot tip it onto the next step.
pub fn segment_step(value: f64, count: u32) -> f64 {
    let count = count.max(1) as f64;
    let lit = (value * count).ceil();
    if lit.is_nan() || lit <= 0.0 {
        return 0.0;
    }
    (lit - 0.5) / count
}

#[cfg(test)]
mod segment_step_tests {
    use super::segment_step;

    #[test]
    fn a_step_lights_what_the_value_lights() {
        for count in [1u32, 3, 10, 24, 25, 50] {
            for i in 0..=2000 {
                let value = i as f64 / 1000.0 - 0.25; // below 0 and above 1 too
                let lit = |v: f64| (v * count.max(1) as f64).ceil() as u32;
                let step = segment_step(value, count);
                assert_eq!(lit(step), lit(value), "count {count}, value {value}");
                assert_eq!(segment_step(step, count), step);
            }
        }
        assert_eq!(segment_step(f64::NAN, 10), 0.0);
    }
}
