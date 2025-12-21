//! Text displayer implementation

use crate::core::{ConfigOption, ConfigSchema, Displayer, DisplayerConfig, PanelTransform};
use crate::displayers::TextDisplayerConfig;
use anyhow::Result;
use cairo::Context;
use gtk4::{glib, prelude::*, DrawingArea, Widget};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Text displayer
///
/// Displays data values as text using Cairo and Pango.
pub struct TextDisplayer {
    id: String,
    name: String,
    data: Arc<Mutex<DisplayData>>,
}

#[derive(Clone)]
struct DisplayData {
    /// Current data values from the source
    values: HashMap<String, Value>,
    /// Text display configuration
    config: TextDisplayerConfig,
    /// Panel transform (scale and translate)
    transform: PanelTransform,
    /// Flag to indicate data has changed and needs redraw
    dirty: bool,
}

impl TextDisplayer {
    pub fn new() -> Self {
        let data = Arc::new(Mutex::new(DisplayData {
            values: HashMap::new(),
            config: TextDisplayerConfig::default(),
            transform: PanelTransform::default(),
            dirty: true,
        }));

        Self {
            id: "text".to_string(),
            name: "Text Display".to_string(),
            data,
        }
    }

    fn draw_internal(cr: &Context, width: i32, height: i32, data: &DisplayData) {
        // Don't clear background - let the custom panel background show through

        // Apply panel transform (scale and translate)
        data.transform.apply(cr, width as f64, height as f64);

        // Use shared text renderer
        crate::ui::text_renderer::render_text_lines(
            cr,
            width as f64,
            height as f64,
            &data.config,
            &data.values,
        );

        // Restore transform
        data.transform.restore(cr);
    }
}

impl Default for TextDisplayer {
    fn default() -> Self {
        Self::new()
    }
}

impl Displayer for TextDisplayer {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn create_widget(&self) -> Widget {
        let drawing_area = DrawingArea::new();

        // Set minimum size
        drawing_area.set_size_request(200, 100);

        // Set up draw function
        let data_clone = self.data.clone();
        drawing_area.set_draw_func(move |_widget, cr, width, height| {
            if let Ok(data) = data_clone.lock() {
                Self::draw_internal(cr, width, height, &data);
            }
        });

        // Set up periodic redraw using timeout - only redraw when data has changed
        glib::timeout_add_local(std::time::Duration::from_millis(100), {
            let drawing_area_weak = drawing_area.downgrade();
            let data_for_timer = self.data.clone();
            move || {
                // Check if widget still exists - this automatically stops the timeout
                let Some(drawing_area) = drawing_area_weak.upgrade() else {
                    return glib::ControlFlow::Break;
                };

                // Skip updates when widget is not visible (saves CPU)
                if !drawing_area.is_mapped() {
                    return glib::ControlFlow::Continue;
                }

                // Only redraw if data changed
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
        // Store only needed data values and extract transform
        if let Ok(mut display_data) = self.data.lock() {
            display_data.transform = PanelTransform::from_values(data);
            // Extract only needed values for text lines (avoids cloning entire HashMap)
            display_data.values = super::extract_text_values(data, &display_data.config);
            display_data.dirty = true;
        }
    }

    fn draw(&self, cr: &Context, width: f64, height: f64) -> Result<()> {
        if let Ok(data) = self.data.lock() {
            Self::draw_internal(cr, width as i32, height as i32, &data);
        }
        Ok(())
    }

    fn config_schema(&self) -> ConfigSchema {
        ConfigSchema {
            options: vec![
                ConfigOption {
                    key: "font_size".to_string(),
                    name: "Font Size".to_string(),
                    description: "Size of the text in pixels".to_string(),
                    value_type: "number".to_string(),
                    default: Value::from(24.0),
                },
                ConfigOption {
                    key: "color".to_string(),
                    name: "Text Color".to_string(),
                    description: "RGB color for the text".to_string(),
                    value_type: "color".to_string(),
                    default: Value::from("#FFFFFF"),
                },
            ],
        }
    }

    fn apply_config(&mut self, config: &HashMap<String, Value>) -> Result<()> {
        // Check for full text_config first (new format from PanelData)
        if let Some(text_config_value) = config.get("text_config") {
            if let Ok(text_config) = serde_json::from_value::<TextDisplayerConfig>(text_config_value.clone()) {
                if let Ok(mut data) = self.data.lock() {
                    data.config = text_config;
                }
                return Ok(());
            }
        }

        // Fallback: Try legacy format with "lines" key
        if let Some(lines_value) = config.get("lines") {
            if let Ok(text_config) = serde_json::from_value::<TextDisplayerConfig>(
                serde_json::json!({ "lines": lines_value })
            ) {
                if let Ok(mut data) = self.data.lock() {
                    data.config = text_config;
                }
            }
        }

        Ok(())
    }

    fn needs_redraw(&self) -> bool {
        self.data.lock().map(|data| data.dirty).unwrap_or(false)
    }

    fn get_typed_config(&self) -> Option<DisplayerConfig> {
        if let Ok(data) = self.data.lock() {
            Some(DisplayerConfig::Text(data.config.clone()))
        } else {
            None
        }
    }
}
