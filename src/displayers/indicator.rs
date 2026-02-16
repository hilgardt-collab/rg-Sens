//! Indicator displayer implementation
//!
//! Displays a color based on a value mapped to a gradient (0% -> 100%).
//! Can show full panel fill, circles, squares, or polygons.

use crate::core::{
    register_animation, ConfigOption, ConfigSchema, Displayer, DisplayerConfig, PanelTransform,
};
use crate::ui::background::{Color, ColorStop};
use crate::ui::render_cache::get_cached_color_at;
use anyhow::Result;
use cairo::Context;
use gtk4::{prelude::*, DrawingArea, Widget};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// Re-export indicator config types from rg-sens-types
pub use rg_sens_types::display_configs::indicator::{IndicatorConfig, IndicatorShape};

/// Get the color for a given value by interpolating the indicator's gradient
pub fn get_color_for_value(config: &IndicatorConfig, value: f64) -> Color {
    interpolate_gradient(&config.gradient_stops, value, config.min_value, config.max_value)
}

/// Interpolate a color from gradient stops based on a value
/// Uses cached LUT for O(1) lookup instead of sorting stops on every call
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

    // Use cached LUT for fast O(1) color lookup
    // The LUT handles sorting internally (once per unique gradient)
    get_cached_color_at(stops, normalized)
}

/// Render an indicator shape
pub fn render_indicator(
    cr: &Context,
    config: &IndicatorConfig,
    value: f64,
    width: f64,
    height: f64,
) -> Result<(), cairo::Error> {
    let color = get_color_for_value(config, value);

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
        let value = data
            .values
            .get(&data.config.value_field)
            .and_then(|v| match v {
                Value::Number(n) => n.as_f64(),
                Value::String(s) => s.parse::<f64>().ok(),
                _ => None,
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
            // Use try_lock to avoid blocking GTK main thread if update is in progress
            let Ok(data) = data_clone.try_lock() else {
                return; // Skip frame if lock contention
            };
            data.transform.apply(cr, width as f64, height as f64);
            Self::draw_internal(cr, width, height, &data);
            data.transform.restore(cr);
        });

        // Register with global animation manager - only redraws when dirty flag is set
        let data_for_animation = self.data.clone();
        register_animation(drawing_area.downgrade(), move || {
            // Use try_lock to avoid blocking UI thread if lock is held
            if let Ok(mut data) = data_for_animation.try_lock() {
                if data.dirty {
                    data.dirty = false;
                    true
                } else {
                    false
                }
            } else {
                false
            }
        });

        drawing_area.upcast()
    }

    fn update_data(&mut self, data: &HashMap<String, Value>) {
        if let Ok(mut display_data) = self.data.lock() {
            // Extract only needed values (avoids cloning entire HashMap)
            let mut values = super::extract_text_values(data, &display_data.config.text_config);
            // Also include the value_field for indicator rendering
            if let Some(v) = data.get(&display_data.config.value_field) {
                values.insert(display_data.config.value_field.clone(), v.clone());
            }
            display_data.values = values;
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
            if let Ok(indicator_config) =
                serde_json::from_value::<IndicatorConfig>(indicator_config_value.clone())
            {
                if let Ok(mut data) = self.data.lock() {
                    data.config = indicator_config;
                }
                return Ok(());
            }
        }

        Ok(())
    }

    fn needs_redraw(&self) -> bool {
        self.data.lock().map(|data| data.dirty).unwrap_or(false)
    }

    fn get_typed_config(&self) -> Option<DisplayerConfig> {
        if let Ok(data) = self.data.lock() {
            Some(DisplayerConfig::Indicator(data.config.clone()))
        } else {
            None
        }
    }
}
