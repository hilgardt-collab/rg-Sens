//! Indicator displayer implementation
//!
//! Displays a color based on a value mapped to a gradient (0% -> 100%).
//! Can show full panel fill, circles, squares, or polygons.

use crate::core::{ConfigOption, ConfigSchema, Displayer, DisplayerConfig, PanelTransform};
use crate::displayers::TextDisplayerConfig;
use crate::ui::background::{Color, ColorStop};
use anyhow::Result;
use cairo::Context;
use gtk4::{glib, prelude::*, DrawingArea, Widget};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Shape type for the indicator
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum IndicatorShape {
    /// Fill the entire panel with the color
    #[default]
    Fill,
    /// Circle shape
    Circle,
    /// Square shape
    Square,
    /// Regular polygon with N sides
    Polygon(u32),
}

impl IndicatorShape {
    /// Get display name for UI
    pub fn display_name(&self) -> String {
        match self {
            IndicatorShape::Fill => "Fill".to_string(),
            IndicatorShape::Circle => "Circle".to_string(),
            IndicatorShape::Square => "Square".to_string(),
            IndicatorShape::Polygon(n) => format!("{}-gon", n),
        }
    }

    /// Get common shapes for UI dropdown
    pub fn common_shapes() -> Vec<IndicatorShape> {
        vec![
            IndicatorShape::Fill,
            IndicatorShape::Circle,
            IndicatorShape::Square,
            IndicatorShape::Polygon(3),  // Triangle
            IndicatorShape::Polygon(5),  // Pentagon
            IndicatorShape::Polygon(6),  // Hexagon
            IndicatorShape::Polygon(7),  // Heptagon
            IndicatorShape::Polygon(8),  // Octagon
        ]
    }
}

/// Configuration for the indicator displayer
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IndicatorConfig {
    /// The field to use for the value (should be 0-100 range)
    #[serde(default = "default_value_field")]
    pub value_field: String,

    /// Gradient stops defining the color mapping
    /// Position 0.0 = value 0%, position 1.0 = value 100%
    #[serde(default = "default_gradient")]
    pub gradient_stops: Vec<ColorStop>,

    /// Shape to display
    #[serde(default)]
    pub shape: IndicatorShape,

    /// Size of the shape as percentage of panel (0.0-1.0)
    /// Only applies to non-Fill shapes
    #[serde(default = "default_shape_size")]
    pub shape_size: f64,

    /// Rotation angle in degrees for the shape
    #[serde(default)]
    pub rotation_angle: f64,

    /// Whether to show text overlay
    #[serde(default)]
    pub show_text: bool,

    /// Text overlay configuration
    #[serde(default)]
    pub text_config: TextDisplayerConfig,

    /// Border width (0 for no border)
    #[serde(default)]
    pub border_width: f64,

    /// Border color
    #[serde(default = "default_border_color")]
    pub border_color: Color,

    /// Minimum value (for mapping to gradient)
    #[serde(default)]
    pub min_value: f64,

    /// Maximum value (for mapping to gradient)
    #[serde(default = "default_max_value")]
    pub max_value: f64,
}

fn default_value_field() -> String {
    "value".to_string()
}

fn default_shape_size() -> f64 {
    0.8
}

fn default_max_value() -> f64 {
    100.0
}

fn default_border_color() -> Color {
    Color::new(1.0, 1.0, 1.0, 0.5)
}

fn default_gradient() -> Vec<ColorStop> {
    vec![
        ColorStop::new(0.0, Color::new(0.0, 0.5, 1.0, 1.0)),   // Blue at 0%
        ColorStop::new(0.4, Color::new(0.0, 1.0, 0.0, 1.0)),   // Green at 40%
        ColorStop::new(0.7, Color::new(1.0, 1.0, 0.0, 1.0)),   // Yellow at 70%
        ColorStop::new(1.0, Color::new(1.0, 0.0, 0.0, 1.0)),   // Red at 100%
    ]
}

impl Default for IndicatorConfig {
    fn default() -> Self {
        Self {
            value_field: default_value_field(),
            gradient_stops: default_gradient(),
            shape: IndicatorShape::default(),
            shape_size: default_shape_size(),
            rotation_angle: 0.0,
            show_text: false,
            text_config: TextDisplayerConfig::default(),
            border_width: 0.0,
            border_color: default_border_color(),
            min_value: 0.0,
            max_value: 100.0,
        }
    }
}

impl IndicatorConfig {
    /// Get the color for a given value by interpolating the gradient
    pub fn get_color_for_value(&self, value: f64) -> Color {
        interpolate_gradient(&self.gradient_stops, value, self.min_value, self.max_value)
    }
}

/// Interpolate a color from gradient stops based on a value
pub fn interpolate_gradient(stops: &[ColorStop], value: f64, min: f64, max: f64) -> Color {
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
    sorted_stops.sort_by(|a, b| {
        a.position.partial_cmp(&b.position).unwrap_or(std::cmp::Ordering::Equal)
    });

    // Handle edge cases - use first/last references to avoid repeated lookups
    let first = &sorted_stops[0];
    let last = sorted_stops.last().unwrap();
    if normalized <= first.position {
        return first.color;
    }
    if normalized >= last.position {
        return last.color;
    }

    // Find surrounding stops
    for i in 0..sorted_stops.len() - 1 {
        let start = sorted_stops[i];
        let end = sorted_stops[i + 1];

        if normalized >= start.position && normalized <= end.position {
            // Interpolate between these two stops
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

    // Fallback
    sorted_stops[0].color
}

/// Render an indicator shape
pub fn render_indicator(
    cr: &Context,
    config: &IndicatorConfig,
    value: f64,
    width: f64,
    height: f64,
) -> Result<(), cairo::Error> {
    let color = config.get_color_for_value(value);

    cr.save()?;

    match config.shape {
        IndicatorShape::Fill => {
            color.apply_to_cairo(cr);
            cr.rectangle(0.0, 0.0, width, height);
            cr.fill()?;
        }
        IndicatorShape::Circle => {
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
        IndicatorShape::Square => {
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
        IndicatorShape::Polygon(sides) => {
            let center_x = width / 2.0;
            let center_y = height / 2.0;
            let radius = (width.min(height) / 2.0) * config.shape_size;
            let sides = sides.max(3);

            cr.translate(center_x, center_y);
            cr.rotate(config.rotation_angle.to_radians());

            draw_polygon(cr, 0.0, 0.0, radius, sides);

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

/// Draw a regular polygon path
fn draw_polygon(cr: &Context, cx: f64, cy: f64, radius: f64, sides: u32) {
    let angle_step = std::f64::consts::TAU / sides as f64;
    // Start at the top
    let start_angle = -std::f64::consts::FRAC_PI_2;

    for i in 0..sides {
        let angle = start_angle + i as f64 * angle_step;
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

/// Indicator displayer
pub struct IndicatorDisplayer {
    id: String,
    name: String,
    data: Arc<Mutex<DisplayData>>,
}

#[derive(Clone)]
struct DisplayData {
    values: HashMap<String, Value>,
    config: IndicatorConfig,
    transform: PanelTransform,
    dirty: bool,
}

impl IndicatorDisplayer {
    pub fn new() -> Self {
        Self {
            id: "indicator".to_string(),
            name: "Indicator".to_string(),
            data: Arc::new(Mutex::new(DisplayData {
                values: HashMap::new(),
                config: IndicatorConfig::default(),
                transform: PanelTransform::default(),
                dirty: true,
            })),
        }
    }

    fn draw_internal(cr: &Context, width: i32, height: i32, data: &DisplayData) {
        let w = width as f64;
        let h = height as f64;

        // Get the value from data
        let value = data.values.get(&data.config.value_field)
            .and_then(|v| {
                match v {
                    Value::Number(n) => n.as_f64(),
                    Value::String(s) => s.parse::<f64>().ok(),
                    _ => None,
                }
            })
            .unwrap_or(0.0);

        // Render the indicator shape
        let _ = render_indicator(cr, &data.config, value, w, h);

        // Render text overlay if enabled
        if data.config.show_text {
            crate::ui::text_renderer::render_text_lines(
                cr,
                w,
                h,
                &data.config.text_config,
                &data.values,
            );
        }
    }
}

impl Default for IndicatorDisplayer {
    fn default() -> Self {
        Self::new()
    }
}

impl Displayer for IndicatorDisplayer {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn create_widget(&self) -> Widget {
        let drawing_area = DrawingArea::new();
        drawing_area.set_size_request(100, 100);

        let data_clone = self.data.clone();
        drawing_area.set_draw_func(move |_widget, cr, width, height| {
            if let Ok(data) = data_clone.lock() {
                data.transform.apply(cr, width as f64, height as f64);
                Self::draw_internal(cr, width, height, &data);
                data.transform.restore(cr);
            }
        });

        // Periodic redraw when dirty
        glib::timeout_add_local(std::time::Duration::from_millis(100), {
            let drawing_area_weak = drawing_area.downgrade();
            let data_for_timer = self.data.clone();
            move || {
                let Some(drawing_area) = drawing_area_weak.upgrade() else {
                    return glib::ControlFlow::Break;
                };

                // Skip updates when widget is not visible (saves CPU)
                if !drawing_area.is_mapped() {
                    return glib::ControlFlow::Continue;
                }

                // Use try_lock to avoid blocking UI thread if lock is held
                let needs_redraw = if let Ok(mut data) = data_for_timer.try_lock() {
                    if data.dirty {
                        data.dirty = false;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                };

                if needs_redraw {
                    drawing_area.queue_draw();
                }
                glib::ControlFlow::Continue
            }
        });

        drawing_area.upcast()
    }

    fn update_data(&mut self, data: &HashMap<String, Value>) {
        if let Ok(mut display_data) = self.data.lock() {
            display_data.values = data.clone();
            display_data.transform = PanelTransform::from_values(data);
            display_data.dirty = true;
        }
    }

    fn draw(&self, cr: &Context, width: f64, height: f64) -> Result<()> {
        if let Ok(data) = self.data.lock() {
            data.transform.apply(cr, width, height);
            Self::draw_internal(cr, width as i32, height as i32, &data);
            data.transform.restore(cr);
        }
        Ok(())
    }

    fn config_schema(&self) -> ConfigSchema {
        ConfigSchema {
            options: vec![
                ConfigOption {
                    key: "value_field".to_string(),
                    name: "Value Field".to_string(),
                    description: "Field to use for the indicator value".to_string(),
                    value_type: "string".to_string(),
                    default: Value::from("value"),
                },
                ConfigOption {
                    key: "shape".to_string(),
                    name: "Shape".to_string(),
                    description: "Shape of the indicator".to_string(),
                    value_type: "string".to_string(),
                    default: Value::from("fill"),
                },
            ],
        }
    }

    fn apply_config(&mut self, config: &HashMap<String, Value>) -> Result<()> {
        if let Some(indicator_config_value) = config.get("indicator_config") {
            if let Ok(indicator_config) = serde_json::from_value::<IndicatorConfig>(indicator_config_value.clone()) {
                if let Ok(mut data) = self.data.lock() {
                    data.config = indicator_config;
                }
                return Ok(());
            }
        }

        Ok(())
    }

    fn needs_redraw(&self) -> bool {
        true
    }

    fn get_typed_config(&self) -> Option<DisplayerConfig> {
        if let Ok(data) = self.data.lock() {
            Some(DisplayerConfig::Indicator(data.config.clone()))
        } else {
            None
        }
    }
}
