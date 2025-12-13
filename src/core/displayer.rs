//! Displayer trait and related types

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
}

/// Type-erased displayer for dynamic dispatch
pub type BoxedDisplayer = Box<dyn Displayer>;
