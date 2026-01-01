//! Data source trait and related types

use super::field_metadata::FieldMetadata;
use super::panel_data::SourceConfig;
use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;

/// Metadata about a data source
#[derive(Debug, Clone)]
pub struct SourceMetadata {
    /// Unique identifier for this source type
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Description of what this source provides
    pub description: String,
    /// Available data keys this source provides
    pub available_keys: Vec<String>,
    /// Recommended update interval
    pub default_interval: Duration,
}

/// Trait for all data sources
///
/// Data sources are responsible for collecting system metrics
/// (CPU, memory, GPU, etc.) and providing them to displayers.
pub trait DataSource: Send + Sync {
    /// Get metadata about this source
    fn metadata(&self) -> &SourceMetadata;

    /// Get field metadata describing available data fields
    ///
    /// Returns a list of fields that this source can provide, including
    /// their types, purposes, and descriptions. This allows displayers
    /// to present appropriate UI for configuring which fields to show.
    fn fields(&self) -> Vec<FieldMetadata>;

    /// Update internal state
    ///
    /// This is called periodically based on the update interval.
    /// Should be relatively quick (<10ms ideally).
    fn update(&mut self) -> Result<()>;

    /// Get current data values
    ///
    /// Returns a map of key -> value pairs. Keys should match
    /// those listed in metadata().available_keys.
    fn get_values(&self) -> HashMap<String, Value>;

    /// Get a reference to the internal values HashMap (if available)
    ///
    /// This is an optimization for sources that maintain an internal HashMap.
    /// It avoids cloning on every access. Sources that don't maintain an
    /// internal HashMap can use the default implementation which clones.
    fn values_ref(&self) -> Option<&HashMap<String, Value>> {
        None
    }

    /// Get a specific value by key
    fn get_value(&self, key: &str) -> Option<Value> {
        // Use values_ref if available to avoid full HashMap clone
        if let Some(values) = self.values_ref() {
            values.get(key).cloned()
        } else {
            self.get_values().get(key).cloned()
        }
    }

    /// Check if this source is available on the current system
    ///
    /// For example, GPU sources might not be available if no GPU is present.
    fn is_available(&self) -> bool {
        true
    }

    /// Configure the data source with source-specific settings
    ///
    /// This allows sources to receive configuration that affects what data
    /// they collect or how they present it. The configuration is passed as
    /// a JSON value to allow flexibility. Sources that don't need configuration
    /// can use the default implementation which does nothing.
    fn configure(&mut self, _config: &HashMap<String, Value>) -> Result<()> {
        Ok(())
    }

    /// Configure the data source with typed configuration (preferred)
    ///
    /// This is the type-safe alternative to `configure()`. Sources can implement
    /// this method to receive their specific config struct directly. The default
    /// implementation converts to HashMap and calls `configure()`.
    ///
    /// Sources should override this method if they want to use typed configs
    /// for better type safety and cleaner code.
    fn configure_typed(&mut self, config: &SourceConfig) -> Result<()> {
        let map = config.to_hashmap();
        self.configure(&map)
    }

    /// Get the current typed configuration (if available)
    ///
    /// Sources can implement this to return their current configuration
    /// as a typed SourceConfig enum variant. The default implementation
    /// returns None, indicating that the source doesn't support typed configs.
    fn get_typed_config(&self) -> Option<SourceConfig> {
        None
    }
}

/// Type-erased data source for dynamic dispatch
pub type BoxedDataSource = Box<dyn DataSource>;
