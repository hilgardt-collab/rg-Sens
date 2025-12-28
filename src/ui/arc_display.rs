//! Arc gauge display for visualizing values as circular arcs

use gtk4::cairo;
use serde::{Deserialize, Serialize};

use crate::ui::background::{Color, ColorStop};
use crate::ui::theme::{
    deserialize_color_or_source, deserialize_color_stops_vec,
    ColorSource, ColorStopSource, ComboThemeConfig,
};
use crate::displayers::TextDisplayerConfig;

/// Arc end cap style
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[derive(Default)]
pub enum ArcCapStyle {
    #[serde(rename = "butt")]
    Butt,      // Square/flat end
    #[serde(rename = "round")]
    #[default]
    Round,     // Rounded end
    #[serde(rename = "pointed")]
    Pointed,   // Pointed/triangular end
}


/// Arc tapering style
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[derive(Default)]
pub enum ArcTaperStyle {
    #[serde(rename = "none")]
    #[default]
    None,           // No tapering, constant width
    #[serde(rename = "start")]
    Start,          // Narrower at start
    #[serde(rename = "end")]
    End,            // Narrower at end
    #[serde(rename = "both")]
    Both,           // Narrower at both ends (elliptical)
}


/// Color transition style
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[derive(Default)]
pub enum ColorTransitionStyle {
    #[serde(rename = "smooth")]
    Smooth,    // Smooth gradient fade between colors
    #[serde(rename = "abrupt")]
    #[default]
    Abrupt,    // Abrupt change at threshold
}


/// Color application mode
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[derive(Default)]
pub enum ColorApplicationMode {
    #[serde(rename = "progressive")]
    #[default]
    Progressive,   // Whole arc changes color based on value
    #[serde(rename = "segments")]
    Segments,      // Individual segments have fixed colors
}


/// Arc gauge configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ArcDisplayConfig {
    // Arc geometry
    #[serde(default = "default_start_angle")]
    pub start_angle: f64,  // Degrees (0 = right, 90 = down, 180 = left, 270 = up)
    #[serde(default = "default_end_angle")]
    pub end_angle: f64,    // Degrees
    #[serde(default = "default_arc_width")]
    pub arc_width: f64,    // Percentage of radius (0.0 to 1.0)
    #[serde(default = "default_radius_percent")]
    pub radius_percent: f64, // Percentage of available space (0.0 to 1.0)

    // Segmentation
    #[serde(default = "default_false")]
    pub segmented: bool,
    #[serde(default = "default_segment_count")]
    pub segment_count: u32,
    #[serde(default = "default_segment_spacing")]
    pub segment_spacing: f64, // Degrees

    // Style
    #[serde(default)]
    pub cap_style: ArcCapStyle,
    #[serde(default)]
    pub taper_style: ArcTaperStyle,
    #[serde(default = "default_taper_amount")]
    pub taper_amount: f64, // 0.0 to 1.0 (how much to taper)

    // Colors
    #[serde(default = "default_color_stops", deserialize_with = "deserialize_color_stops_vec")]
    pub color_stops: Vec<ColorStopSource>,
    #[serde(default)]
    pub color_transition: ColorTransitionStyle,
    #[serde(default)]
    pub color_mode: ColorApplicationMode,

    // Background arc (unfilled portion)
    #[serde(default = "default_show_background_arc")]
    pub show_background_arc: bool,
    #[serde(default = "default_background_color", deserialize_with = "deserialize_color_or_source")]
    pub background_color: ColorSource,
    #[serde(default = "default_false")]
    pub overlay_background: bool, // If true, draw full arc then overlay with background color

    // Animation
    #[serde(default = "default_false")]
    pub animate: bool,
    #[serde(default = "default_animation_duration")]
    pub animation_duration: f64, // Duration in seconds

    // Text overlay
    #[serde(default)]
    pub text_overlay: TextOverlayConfig,
}

/// Text overlay configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TextOverlayConfig {
    pub enabled: bool,
    #[serde(default)]
    pub text_config: TextDisplayerConfig,
}

impl Default for TextOverlayConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            text_config: TextDisplayerConfig::default(),
        }
    }
}

fn default_start_angle() -> f64 {
    135.0 // Bottom-left
}

fn default_end_angle() -> f64 {
    45.0 // Bottom-right (goes through top)
}

fn default_arc_width() -> f64 {
    0.15 // 15% of radius
}

fn default_radius_percent() -> f64 {
    0.85 // 85% of available space
}

fn default_false() -> bool {
    false
}

fn default_segment_count() -> u32 {
    20
}

fn default_segment_spacing() -> f64 {
    2.0
}

fn default_taper_amount() -> f64 {
    0.5
}

fn default_color_stops() -> Vec<ColorStopSource> {
    vec![
        ColorStopSource::custom(0.0, Color::new(0.0, 0.8, 0.0, 1.0)),   // Green 0-60%
        ColorStopSource::custom(0.6, Color::new(0.0, 0.8, 0.0, 1.0)),   // Green at 60%
        ColorStopSource::custom(0.6, Color::new(1.0, 0.8, 0.0, 1.0)),   // Yellow at 60%
        ColorStopSource::custom(0.8, Color::new(1.0, 0.8, 0.0, 1.0)),   // Yellow at 80%
        ColorStopSource::custom(0.8, Color::new(1.0, 0.0, 0.0, 1.0)),   // Red at 80%
        ColorStopSource::custom(1.0, Color::new(1.0, 0.0, 0.0, 1.0)),   // Red at 100%
    ]
}

fn default_show_background_arc() -> bool {
    true
}

fn default_background_color() -> ColorSource {
    ColorSource::Custom { color: Color::new(0.2, 0.2, 0.2, 0.3) }
}

fn default_animation_duration() -> f64 {
    0.3 // 300ms
}

impl Default for ArcDisplayConfig {
    fn default() -> Self {
        Self {
            start_angle: default_start_angle(),
            end_angle: default_end_angle(),
            arc_width: default_arc_width(),
            radius_percent: default_radius_percent(),
            segmented: default_false(),
            segment_count: default_segment_count(),
            segment_spacing: default_segment_spacing(),
            cap_style: ArcCapStyle::default(),
            taper_style: ArcTaperStyle::default(),
            taper_amount: default_taper_amount(),
            color_stops: default_color_stops(),
            color_transition: ColorTransitionStyle::default(),
            color_mode: ColorApplicationMode::default(),
            show_background_arc: default_show_background_arc(),
            background_color: default_background_color(),
            overlay_background: default_false(),
            animate: default_false(),
            animation_duration: default_animation_duration(),
            text_overlay: TextOverlayConfig::default(),
        }
    }
}

/// Internal helper struct with resolved colors for rendering
struct ResolvedArcConfig<'a> {
    config: &'a ArcDisplayConfig,
    color_stops: Vec<ColorStop>,
    background_color: Color,
}

/// Render an arc gauge display
pub fn render_arc(
    cr: &cairo::Context,
    config: &ArcDisplayConfig,
    theme: &ComboThemeConfig,
    value: f64, // 0.0 to 1.0
    values: &std::collections::HashMap<String, serde_json::Value>,
    width: f64,
    height: f64,
) -> Result<(), cairo::Error> {
    let value = value.clamp(0.0, 1.0);

    // Resolve theme colors to concrete colors
    let resolved_stops: Vec<ColorStop> = config.color_stops.iter()
        .map(|s| s.resolve(theme))
        .collect();
    let resolved_bg_color = config.background_color.resolve(theme);

    // Create a resolved config for internal functions
    let resolved_config = ResolvedArcConfig {
        config,
        color_stops: resolved_stops,
        background_color: resolved_bg_color,
    };

    // Calculate center and radius
    let center_x = width / 2.0;
    let center_y = height / 2.0;
    let max_radius = (width.min(height) / 2.0) * config.radius_percent;

    // Check if we should use overlay mode
    let use_overlay = config.overlay_background
        && config.show_background_arc
        && resolved_bg_color.a < 1.0; // Only overlay if background has transparency

    if use_overlay {
        // Draw full arc with colors first
        if config.segmented {
            render_full_segmented_arc(cr, &resolved_config, center_x, center_y, max_radius)?;
        } else {
            render_full_continuous_arc(cr, &resolved_config, center_x, center_y, max_radius)?;
        }

        // Then overlay the background arc over the unfilled portion
        render_overlay_arc(cr, &resolved_config, value, center_x, center_y, max_radius)?;
    } else {
        // Standard rendering: background first, then filled arc
        if config.show_background_arc {
            render_background_arc(cr, &resolved_config, center_x, center_y, max_radius)?;
        }

        // Draw filled arc
        if config.segmented {
            render_segmented_arc(cr, &resolved_config, value, center_x, center_y, max_radius)?;
        } else {
            render_continuous_arc(cr, &resolved_config, value, center_x, center_y, max_radius)?;
        }
    }

    // Render text overlay if enabled
    if config.text_overlay.enabled {
        crate::ui::text_renderer::render_text_lines(
            cr,
            width,
            height,
            &config.text_overlay.text_config,
            values,
        );
    }

    Ok(())
}

/// Render the background arc (unfilled portion)
fn render_background_arc(
    cr: &cairo::Context,
    resolved: &ResolvedArcConfig,
    cx: f64,
    cy: f64,
    radius: f64,
) -> Result<(), cairo::Error> {
    let config = resolved.config;
    let start_rad = config.start_angle.to_radians();
    let end_rad = config.end_angle.to_radians();
    let width = radius * config.arc_width;

    cr.save()?;
    resolved.background_color.apply_to_cairo(cr);

    // Set line cap based on config
    cr.set_line_cap(match config.cap_style {
        ArcCapStyle::Butt => cairo::LineCap::Butt,
        ArcCapStyle::Round => cairo::LineCap::Round,
        ArcCapStyle::Pointed => cairo::LineCap::Butt,
    });

    if config.segmented {
        // Draw segmented background
        let total_angle = normalize_angle_range(config.start_angle, config.end_angle);
        let segment_angle = (total_angle - (config.segment_count - 1) as f64 * config.segment_spacing) / config.segment_count as f64;

        for i in 0..config.segment_count {
            let seg_start = start_rad + (i as f64 * (segment_angle + config.segment_spacing)).to_radians();
            let seg_end = seg_start + segment_angle.to_radians();

            if config.taper_style != ArcTaperStyle::None {
                // Draw tapered segment
                let seg_steps = 10;
                let seg_angle_step = (seg_end - seg_start) / seg_steps as f64;

                for j in 0..seg_steps {
                    let t = (i as f64 + j as f64 / seg_steps as f64) / config.segment_count as f64;
                    let step_start = seg_start + j as f64 * seg_angle_step;
                    let step_end = step_start + seg_angle_step;
                    let step_width = calculate_tapered_width(width, t, config.taper_style, config.taper_amount);

                    cr.set_line_width(step_width);
                    cr.new_path();
                    cr.arc(cx, cy, radius - step_width / 2.0, step_start, step_end);
                    cr.stroke()?;
                }
            } else {
                cr.set_line_width(width);
                cr.new_path();
                cr.arc(cx, cy, radius - width / 2.0, seg_start, seg_end);
                cr.stroke()?;
            }
        }
    } else {
        // Draw continuous background with tapering if enabled
        if config.taper_style != ArcTaperStyle::None {
            let total_angle = normalize_angle_range(config.start_angle, config.end_angle);
            let num_segments = 50;
            let angle_step = total_angle / num_segments as f64;

            for i in 0..num_segments {
                let t = i as f64 / num_segments as f64;
                let seg_start = start_rad + (i as f64 * angle_step).to_radians();
                let seg_end = seg_start + angle_step.to_radians();
                let seg_width = calculate_tapered_width(width, t, config.taper_style, config.taper_amount);

                cr.set_line_width(seg_width);
                cr.new_path();
                cr.arc(cx, cy, radius - seg_width / 2.0, seg_start, seg_end);
                cr.stroke()?;
            }
        } else {
            cr.set_line_width(width);
            cr.new_path();
            cr.arc(cx, cy, radius - width / 2.0, start_rad, end_rad);
            cr.stroke()?;
        }
    }

    cr.restore()?;
    Ok(())
}

/// Render continuous arc
fn render_continuous_arc(
    cr: &cairo::Context,
    resolved: &ResolvedArcConfig,
    value: f64,
    cx: f64,
    cy: f64,
    radius: f64,
) -> Result<(), cairo::Error> {
    let config = resolved.config;
    let start_rad = config.start_angle.to_radians();
    let total_angle = normalize_angle_range(config.start_angle, config.end_angle);
    let filled_angle = total_angle * value;
    let end_rad = start_rad + filled_angle.to_radians();
    let width = radius * config.arc_width;

    cr.save()?;

    // Set line cap
    cr.set_line_cap(match config.cap_style {
        ArcCapStyle::Butt => cairo::LineCap::Butt,
        ArcCapStyle::Round => cairo::LineCap::Round,
        ArcCapStyle::Pointed => cairo::LineCap::Butt,
    });

    // Determine if we need to draw with segments (either tapered or segment color mode)
    let needs_segments = config.taper_style != ArcTaperStyle::None || config.color_mode == ColorApplicationMode::Segments;

    if !needs_segments {
        // Simple constant-width arc with progressive color
        let color = get_color_at_value(value, &resolved.color_stops, config.color_transition);
        color.apply_to_cairo(cr);
        cr.set_line_width(width);
        cr.new_path();
        cr.arc(cx, cy, radius - width / 2.0, start_rad, end_rad);
        cr.stroke()?;
    } else {
        // Draw with multiple segments for tapering or segment-based colors
        // Use small segments along the TOTAL arc, but only draw the filled portion
        let total_num_segments = 50;
        let total_angle_step = total_angle / total_num_segments as f64;
        let num_filled_segments = (value * total_num_segments as f64).ceil() as u32;

        for i in 0..num_filled_segments {
            // t is position along the TOTAL arc (0.0 to 1.0)
            let t = i as f64 / total_num_segments as f64;
            let seg_start = start_rad + (i as f64 * total_angle_step).to_radians();
            let seg_end = seg_start + total_angle_step.to_radians();

            let seg_width = calculate_tapered_width(width, t, config.taper_style, config.taper_amount);

            // Color based on mode
            let seg_color = if config.color_mode == ColorApplicationMode::Progressive {
                get_color_at_value(value, &resolved.color_stops, config.color_transition)
            } else {
                // Segments mode: color based on position in total arc
                get_color_at_value(t, &resolved.color_stops, config.color_transition)
            };

            seg_color.apply_to_cairo(cr);
            cr.set_line_width(seg_width);
            cr.new_path();
            cr.arc(cx, cy, radius - seg_width / 2.0, seg_start, seg_end);
            cr.stroke()?;
        }
    }

    // Draw pointed caps if needed
    if config.cap_style == ArcCapStyle::Pointed {
        let start_color = if config.color_mode == ColorApplicationMode::Progressive {
            get_color_at_value(value, &resolved.color_stops, config.color_transition)
        } else {
            get_color_at_value(0.0, &resolved.color_stops, config.color_transition)
        };
        let end_color = if config.color_mode == ColorApplicationMode::Progressive {
            get_color_at_value(value, &resolved.color_stops, config.color_transition)
        } else {
            get_color_at_value(value, &resolved.color_stops, config.color_transition)
        };
        draw_pointed_cap(cr, cx, cy, radius, width, start_rad, true, &start_color)?;
        draw_pointed_cap(cr, cx, cy, radius, width, end_rad, false, &end_color)?;
    }

    cr.restore()?;
    Ok(())
}

/// Render segmented arc
fn render_segmented_arc(
    cr: &cairo::Context,
    resolved: &ResolvedArcConfig,
    value: f64,
    cx: f64,
    cy: f64,
    radius: f64,
) -> Result<(), cairo::Error> {
    let config = resolved.config;
    let start_rad = config.start_angle.to_radians();
    let total_angle = normalize_angle_range(config.start_angle, config.end_angle);
    let segment_angle = (total_angle - (config.segment_count - 1) as f64 * config.segment_spacing) / config.segment_count as f64;
    let width = radius * config.arc_width;

    let filled_segments = (value * config.segment_count as f64).ceil() as u32;

    cr.save()?;
    cr.set_line_cap(match config.cap_style {
        ArcCapStyle::Butt => cairo::LineCap::Butt,
        ArcCapStyle::Round => cairo::LineCap::Round,
        ArcCapStyle::Pointed => cairo::LineCap::Butt,
    });

    // In Segments mode with background arc, draw all segments (filled and unfilled)
    // In Segments mode without background arc, only draw filled segments
    // In Progressive mode, only draw filled segments
    let segments_to_draw = if config.color_mode == ColorApplicationMode::Segments && config.show_background_arc {
        config.segment_count
    } else {
        filled_segments
    };

    for i in 0..segments_to_draw {
        let seg_start = start_rad + (i as f64 * (segment_angle + config.segment_spacing)).to_radians();
        let seg_end = seg_start + segment_angle.to_radians();
        let seg_value = (i as f64 + 0.5) / config.segment_count as f64;
        let is_filled = i < filled_segments;

        // Determine color based on mode
        let color = match config.color_mode {
            ColorApplicationMode::Progressive => {
                // All filled segments have the same color based on current value
                get_color_at_value(value, &resolved.color_stops, config.color_transition)
            }
            ColorApplicationMode::Segments => {
                // Each segment has its own color based on position
                if is_filled {
                    // Filled segments show their position color
                    get_color_at_value(seg_value, &resolved.color_stops, config.color_transition)
                } else {
                    // Unfilled segments use background arc color
                    resolved.background_color
                }
            }
        };

        color.apply_to_cairo(cr);

        // Apply tapering if enabled
        if config.taper_style != ArcTaperStyle::None {
            let seg_steps = 10;
            let seg_angle_step = (seg_end - seg_start) / seg_steps as f64;

            for j in 0..seg_steps {
                let t = (i as f64 + j as f64 / seg_steps as f64) / config.segment_count as f64;
                let step_start = seg_start + j as f64 * seg_angle_step;
                let step_end = step_start + seg_angle_step;
                let step_width = calculate_tapered_width(width, t, config.taper_style, config.taper_amount);

                cr.set_line_width(step_width);
                cr.new_path();
                cr.arc(cx, cy, radius - step_width / 2.0, step_start, step_end);
                cr.stroke()?;
            }
        } else {
            cr.set_line_width(width);
            cr.new_path();
            cr.arc(cx, cy, radius - width / 2.0, seg_start, seg_end);
            cr.stroke()?;
        }

        // Draw pointed caps for segments if needed
        if config.cap_style == ArcCapStyle::Pointed && is_filled {
            draw_pointed_cap(cr, cx, cy, radius, width, seg_start, true, &color)?;
            draw_pointed_cap(cr, cx, cy, radius, width, seg_end, false, &color)?;
        }
    }

    cr.restore()?;
    Ok(())
}

/// Calculate tapered width at position t (0.0 to 1.0)
fn calculate_tapered_width(base_width: f64, t: f64, style: ArcTaperStyle, amount: f64) -> f64 {
    match style {
        ArcTaperStyle::None => base_width,
        ArcTaperStyle::Start => {
            // Narrower at start (t=0)
            let factor = 1.0 - amount * (1.0 - t);
            base_width * factor
        }
        ArcTaperStyle::End => {
            // Narrower at end (t=1)
            let factor = 1.0 - amount * t;
            base_width * factor
        }
        ArcTaperStyle::Both => {
            // Narrower at both ends (elliptical)
            let factor = 1.0 - amount * (2.0 * (t - 0.5)).abs();
            base_width * factor
        }
    }
}

/// Get color at a specific value (0.0 to 1.0) using color stops
fn get_color_at_value(value: f64, stops: &[ColorStop], transition: ColorTransitionStyle) -> Color {
    use crate::ui::render_cache::{get_abrupt_color, get_cached_color_at};

    match transition {
        ColorTransitionStyle::Abrupt => get_abrupt_color(stops, value),
        ColorTransitionStyle::Smooth => get_cached_color_at(stops, value),
    }
}

/// Draw a pointed end cap
fn draw_pointed_cap(
    cr: &cairo::Context,
    cx: f64,
    cy: f64,
    radius: f64,
    width: f64,
    angle: f64,
    is_start: bool,
    color: &Color,
) -> Result<(), cairo::Error> {
    let inner_radius = radius - width;
    let outer_radius = radius;

    // Calculate points
    let cos_a = angle.cos();
    let sin_a = angle.sin();

    let inner_x = cx + inner_radius * cos_a;
    let inner_y = cy + inner_radius * sin_a;
    let outer_x = cx + outer_radius * cos_a;
    let outer_y = cy + outer_radius * sin_a;

    // Point extends beyond the arc
    let point_length = width * 0.5;
    let point_x = cx + (radius - width / 2.0) * cos_a + point_length * cos_a * (if is_start { -1.0 } else { 1.0 });
    let point_y = cy + (radius - width / 2.0) * sin_a + point_length * sin_a * (if is_start { -1.0 } else { 1.0 });

    color.apply_to_cairo(cr);
    cr.new_path();
    cr.move_to(inner_x, inner_y);
    cr.line_to(point_x, point_y);
    cr.line_to(outer_x, outer_y);
    cr.close_path();
    cr.fill()?;

    Ok(())
}

/// Render full segmented arc (all segments with their colors) for overlay mode
fn render_full_segmented_arc(
    cr: &cairo::Context,
    resolved: &ResolvedArcConfig,
    cx: f64,
    cy: f64,
    radius: f64,
) -> Result<(), cairo::Error> {
    let config = resolved.config;
    let start_rad = config.start_angle.to_radians();
    let total_angle = normalize_angle_range(config.start_angle, config.end_angle);
    let segment_angle = (total_angle - (config.segment_count - 1) as f64 * config.segment_spacing) / config.segment_count as f64;
    let width = radius * config.arc_width;

    cr.save()?;
    cr.set_line_cap(match config.cap_style {
        ArcCapStyle::Butt => cairo::LineCap::Butt,
        ArcCapStyle::Round => cairo::LineCap::Round,
        ArcCapStyle::Pointed => cairo::LineCap::Butt,
    });

    for i in 0..config.segment_count {
        let seg_start = start_rad + (i as f64 * (segment_angle + config.segment_spacing)).to_radians();
        let seg_end = seg_start + segment_angle.to_radians();
        let seg_value = (i as f64 + 0.5) / config.segment_count as f64;

        let color = get_color_at_value(seg_value, &resolved.color_stops, config.color_transition);
        color.apply_to_cairo(cr);

        // Apply tapering if enabled
        if config.taper_style != ArcTaperStyle::None {
            let seg_steps = 10;
            let seg_angle_step = (seg_end - seg_start) / seg_steps as f64;

            for j in 0..seg_steps {
                let t = (i as f64 + j as f64 / seg_steps as f64) / config.segment_count as f64;
                let step_start = seg_start + j as f64 * seg_angle_step;
                let step_end = step_start + seg_angle_step;
                let step_width = calculate_tapered_width(width, t, config.taper_style, config.taper_amount);

                cr.set_line_width(step_width);
                cr.new_path();
                cr.arc(cx, cy, radius - step_width / 2.0, step_start, step_end);
                cr.stroke()?;
            }
        } else {
            cr.set_line_width(width);
            cr.new_path();
            cr.arc(cx, cy, radius - width / 2.0, seg_start, seg_end);
            cr.stroke()?;
        }
    }

    cr.restore()?;
    Ok(())
}

/// Render full continuous arc (entire arc with gradient) for overlay mode
fn render_full_continuous_arc(
    cr: &cairo::Context,
    resolved: &ResolvedArcConfig,
    cx: f64,
    cy: f64,
    radius: f64,
) -> Result<(), cairo::Error> {
    // Just render the continuous arc at full value (1.0)
    render_continuous_arc(cr, resolved, 1.0, cx, cy, radius)
}

/// Render overlay arc (background color over unfilled portion) for overlay mode
fn render_overlay_arc(
    cr: &cairo::Context,
    resolved: &ResolvedArcConfig,
    value: f64,
    cx: f64,
    cy: f64,
    radius: f64,
) -> Result<(), cairo::Error> {
    let config = resolved.config;
    let start_rad = config.start_angle.to_radians();
    let total_angle = normalize_angle_range(config.start_angle, config.end_angle);
    let width = radius * config.arc_width;

    cr.save()?;
    resolved.background_color.apply_to_cairo(cr);

    cr.set_line_cap(match config.cap_style {
        ArcCapStyle::Butt => cairo::LineCap::Butt,
        ArcCapStyle::Round => cairo::LineCap::Round,
        ArcCapStyle::Pointed => cairo::LineCap::Butt,
    });

    if config.segmented {
        // Overlay on unfilled segments
        let segment_angle = (total_angle - (config.segment_count - 1) as f64 * config.segment_spacing) / config.segment_count as f64;
        let filled_segments = (value * config.segment_count as f64).ceil() as u32;

        for i in filled_segments..config.segment_count {
            let seg_start = start_rad + (i as f64 * (segment_angle + config.segment_spacing)).to_radians();
            let seg_end = seg_start + segment_angle.to_radians();

            // Apply tapering if enabled
            if config.taper_style != ArcTaperStyle::None {
                let seg_steps = 10;
                let seg_angle_step = (seg_end - seg_start) / seg_steps as f64;

                for j in 0..seg_steps {
                    let t = (i as f64 + j as f64 / seg_steps as f64) / config.segment_count as f64;
                    let step_start = seg_start + j as f64 * seg_angle_step;
                    let step_end = step_start + seg_angle_step;
                    let step_width = calculate_tapered_width(width, t, config.taper_style, config.taper_amount);

                    cr.set_line_width(step_width);
                    cr.new_path();
                    cr.arc(cx, cy, radius - step_width / 2.0, step_start, step_end);
                    cr.stroke()?;
                }
            } else {
                cr.set_line_width(width);
                cr.new_path();
                cr.arc(cx, cy, radius - width / 2.0, seg_start, seg_end);
                cr.stroke()?;
            }
        }
    } else {
        // Overlay on unfilled continuous arc
        let filled_angle = total_angle * value;
        let overlay_start = start_rad + filled_angle.to_radians();
        let overlay_end = start_rad + total_angle.to_radians();

        if config.taper_style != ArcTaperStyle::None {
            let overlay_angle = total_angle - filled_angle;
            let num_segments = 50;
            let angle_step = overlay_angle / num_segments as f64;

            for i in 0..num_segments {
                let t = (filled_angle + i as f64 * angle_step) / total_angle;
                let seg_start = overlay_start + (i as f64 * angle_step).to_radians();
                let seg_end = seg_start + angle_step.to_radians();
                let seg_width = calculate_tapered_width(width, t, config.taper_style, config.taper_amount);

                cr.set_line_width(seg_width);
                cr.new_path();
                cr.arc(cx, cy, radius - seg_width / 2.0, seg_start, seg_end);
                cr.stroke()?;
            }
        } else {
            cr.set_line_width(width);
            cr.new_path();
            cr.arc(cx, cy, radius - width / 2.0, overlay_start, overlay_end);
            cr.stroke()?;
        }
    }

    cr.restore()?;
    Ok(())
}

/// Normalize angle range to always be positive
fn normalize_angle_range(start: f64, end: f64) -> f64 {
    if end > start {
        end - start
    } else {
        360.0 - start + end
    }
}
