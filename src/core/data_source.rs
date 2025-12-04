//! Data source trait and related types

use super::field_metadata::FieldMetadata;
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

    /// Get a specific value by key
    fn get_value(&self, key: &str) -> Option<Value> {
        self.get_values().get(key).cloned()
    }

    /// Check if this source is available on the current system
    ///
    /// For example, GPU sources might not be available if no GPU is present.
    fn is_available(&self) -> bool {
        true
    }
}

/// Type-erased data source for dynamic dispatch
pub type BoxedDataSource = Box<dyn DataSource>;
