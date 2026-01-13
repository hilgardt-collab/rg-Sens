//! Displayer trait and related types

use super::constants::TRANSFORM_THRESHOLD;
use super::panel_data::DisplayerConfig;
use anyhow::Result;
use cairo::Context;
use gtk4::Widget;
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
    ///
    /// This is typically a DrawingArea or similar widget.
    fn create_widget(&self) -> Widget;

    /// Update the displayer with new data
    ///
    /// Called when data from the source changes.
    fn update_data(&mut self, data: &HashMap<String, Value>);

    /// Render the displayer
    ///
    /// Called when the widget needs to be redrawn.
    fn draw(&self, cr: &Context, width: f64, height: f64) -> Result<()>;

    /// Get configuration schema
    ///
    /// Describes what options this displayer supports.
    fn config_schema(&self) -> ConfigSchema;

    /// Apply configuration
    ///
    /// Update displayer settings based on user configuration.
    fn apply_config(&mut self, config: &HashMap<String, Value>) -> Result<()>;

    /// Check if the displayer needs to be redrawn
    ///
    /// Can be used to optimize rendering.
    fn needs_redraw(&self) -> bool {
        true
    }

    /// Apply typed configuration (preferred)
    ///
    /// This is the type-safe alternative to `apply_config()`. Displayers can
    /// implement this method to receive their specific config struct directly.
    /// The default implementation converts to HashMap and calls `apply_config()`.
    ///
    /// Displayers should override this method if they want to use typed configs
    /// for better type safety and cleaner code.
    fn apply_config_typed(&mut self, config: &DisplayerConfig) -> Result<()> {
        let map = config.to_hashmap();
        self.apply_config(&map)
    }

    /// Get the current typed configuration (if available)
    ///
    /// Displayers can implement this to return their current configuration
    /// as a typed DisplayerConfig enum variant. The default implementation
    /// returns None, indicating that the displayer doesn't support typed configs.
    fn get_typed_config(&self) -> Option<DisplayerConfig> {
        None
    }

    /// Get bounds of clickable icon/indicator area (if any)
    ///
    /// Returns (x, y, width, height) in widget coordinates.
    /// Used for hit testing to determine if a click should trigger
    /// icon-specific behavior (like opening alarm dialog).
    /// Default implementation returns None (no clickable icon).
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
            scale: values.get("_panel_scale")
                .and_then(|v| v.as_f64())
                .unwrap_or(1.0),
            translate_x: values.get("_panel_translate_x")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0),
            translate_y: values.get("_panel_translate_y")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0),
        }
    }

    /// Apply transform to Cairo context
    ///
    /// Call this at the start of drawing, then restore() when done.
    /// Returns the effective width and height after scaling.
    pub fn apply(&self, cr: &Context, width: f64, height: f64) -> (f64, f64) {
        cr.save().ok();

        // Validate translation values - skip if NaN or infinite
        let translate_x = if self.translate_x.is_finite() { self.translate_x } else { 0.0 };
        let translate_y = if self.translate_y.is_finite() { self.translate_y } else { 0.0 };

        // Apply translation first
        cr.translate(translate_x, translate_y);

        // Validate scale - must be positive and finite, with a minimum to prevent invalid matrix
        let scale = if self.scale.is_finite() && self.scale > TRANSFORM_THRESHOLD {
            self.scale
        } else if self.scale.is_finite() && self.scale > 0.0 {
            TRANSFORM_THRESHOLD // Use minimum valid scale
        } else {
            1.0 // Default to no scaling for invalid values
        };

        // Scale from center
        if (scale - 1.0).abs() > TRANSFORM_THRESHOLD {
            let center_x = width / 2.0;
            let center_y = height / 2.0;
            cr.translate(center_x, center_y);
            cr.scale(scale, scale);
            cr.translate(-center_x, -center_y);
        }

        // Return effective dimensions (for drawing at scale 1.0)
        (width, height)
    }

    /// Restore Cairo context after transform
    pub fn restore(&self, cr: &Context) {
        cr.restore().ok();
    }
}
