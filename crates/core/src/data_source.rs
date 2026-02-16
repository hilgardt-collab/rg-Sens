//! Data source trait and related types

use anyhow::Result;
use rg_sens_types::panel::SourceConfig;
use rg_sens_types::FieldMetadata;
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
    fn fields(&self) -> Vec<FieldMetadata>;

    /// Update internal state
    fn update(&mut self) -> Result<()>;

    /// Get current data values
    fn get_values(&self) -> HashMap<String, Value>;

    /// Get a reference to the internal values HashMap (if available)
    fn values_ref(&self) -> Option<&HashMap<String, Value>> {
        None
    }

    /// Get a specific value by key
    fn get_value(&self, key: &str) -> Option<Value> {
        if let Some(values) = self.values_ref() {
            values.get(key).cloned()
        } else {
            self.get_values().get(key).cloned()
        }
    }

    /// Check if this source is available on the current system
    fn is_available(&self) -> bool {
        true
    }

    /// Configure the data source with source-specific settings
    fn configure(&mut self, _config: &HashMap<String, Value>) -> Result<()> {
        Ok(())
    }

    /// Configure the data source with typed configuration (preferred)
    fn configure_typed(&mut self, config: &SourceConfig) -> Result<()> {
        let map = config.to_hashmap();
        self.configure(&map)
    }

    /// Get the current typed configuration (if available)
    fn get_typed_config(&self) -> Option<SourceConfig> {
        None
    }
}

/// Type-erased data source for dynamic dispatch
pub type BoxedDataSource = Box<dyn DataSource>;
