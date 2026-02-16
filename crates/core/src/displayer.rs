//! Displayer trait and related types

use crate::constants::TRANSFORM_THRESHOLD;
use anyhow::Result;
use cairo::Context;
use gtk4::Widget;
use rg_sens_types::panel::DisplayerConfig;
use serde_json::Value;
use std::collections::HashMap;

/// Configuration schema for a displayer option
#[derive(Debug, Clone)]
pub struct ConfigOption {
    /// Option key
    pub key: String,
    /// Human-readable name
    pub name: String,
    /// Description
    pub description: String,
    /// Value type (e.g., "color", "number", "string", "boolean")
    pub value_type: String,
    /// Default value
    pub default: Value,
}

/// Configuration schema for a displayer
#[derive(Debug, Clone)]
pub struct ConfigSchema {
    /// Available configuration options
    pub options: Vec<ConfigOption>,
}

/// Trait for all displayers
///
/// Displayers are responsible for rendering data visually.
/// They receive data from data sources and draw it using Cairo.
pub trait Displayer: Send + Sync {
    /// Unique identifier for this displayer type
    fn id(&self) -> &str;

    /// Human-readable name
    fn name(&self) -> &str;

    /// Create the GTK widget for this displayer
    fn create_widget(&self) -> Widget;

    /// Update the displayer with new data
    fn update_data(&mut self, data: &HashMap<String, Value>);

    /// Render the displayer
    fn draw(&self, cr: &Context, width: f64, height: f64) -> Result<()>;

    /// Get configuration schema
    fn config_schema(&self) -> ConfigSchema;

    /// Apply configuration
    fn apply_config(&mut self, config: &HashMap<String, Value>) -> Result<()>;

    /// Check if the displayer needs to be redrawn
    fn needs_redraw(&self) -> bool {
        true
    }

    /// Apply typed configuration (preferred)
    fn apply_config_typed(&mut self, config: &DisplayerConfig) -> Result<()> {
        let map = config.to_hashmap();
        self.apply_config(&map)
    }

    /// Get the current typed configuration (if available)
    fn get_typed_config(&self) -> Option<DisplayerConfig> {
        None
    }

    /// Get bounds of clickable icon/indicator area (if any)
    fn get_icon_bounds(&self) -> Option<(f64, f64, f64, f64)> {
        None
    }
}

/// Type-erased displayer for dynamic dispatch
pub type BoxedDisplayer = Box<dyn Displayer>;

/// Panel transform configuration for displayers
#[derive(Debug, Clone, Copy, Default)]
pub struct PanelTransform {
    /// Scale factor (1.0 = normal size)
    pub scale: f64,
    /// X translation in pixels
    pub translate_x: f64,
    /// Y translation in pixels
    pub translate_y: f64,
}

impl PanelTransform {
    /// Extract transform from data values
    pub fn from_values(values: &HashMap<String, Value>) -> Self {
        Self {
            scale: values
                .get("_panel_scale")
                .and_then(|v| v.as_f64())
                .unwrap_or(1.0),
            translate_x: values
                .get("_panel_translate_x")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0),
            translate_y: values
                .get("_panel_translate_y")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0),
        }
    }

    /// Apply transform to Cairo context
    pub fn apply(&self, cr: &Context, width: f64, height: f64) -> (f64, f64) {
        cr.save().ok();

        let translate_x = if self.translate_x.is_finite() {
            self.translate_x
        } else {
            0.0
        };
        let translate_y = if self.translate_y.is_finite() {
            self.translate_y
        } else {
            0.0
        };

        cr.translate(translate_x, translate_y);

        let scale = if self.scale.is_finite() && self.scale > TRANSFORM_THRESHOLD {
            self.scale
        } else if self.scale.is_finite() && self.scale > 0.0 {
            TRANSFORM_THRESHOLD
        } else {
            1.0
        };

        if (scale - 1.0).abs() > TRANSFORM_THRESHOLD {
            let center_x = width / 2.0;
            let center_y = height / 2.0;
            cr.translate(center_x, center_y);
            cr.scale(scale, scale);
            cr.translate(-center_x, -center_y);
        }

        (width, height)
    }

    /// Restore Cairo context after transform
    pub fn restore(&self, cr: &Context) {
        cr.restore().ok();
    }
}
