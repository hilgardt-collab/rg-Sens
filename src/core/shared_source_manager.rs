//! Shared source manager - maintains single instances of data sources
//!
//! This module ensures that each unique source configuration has only ONE
//! data source instance that polls the hardware. Multiple panels can reference
//! the same shared source, avoiding duplicate sensor polling.

use super::{BoxedDataSource, Registry};
use super::panel_data::SourceConfig;
use anyhow::{Result, anyhow};
use log::{info, debug, warn};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;

/// Represents a shared source with its cached values and update tracking
pub struct SharedSource {
    /// The actual data source instance
    pub source: BoxedDataSource,
    /// Cached values from the last update
    pub cached_values: HashMap<String, serde_json::Value>,
    /// Number of panels using this source
    pub ref_count: usize,
    /// Minimum update interval requested by any panel using this source
    pub min_interval: Duration,
    /// List of panel IDs using this source (for debugging)
    pub panel_ids: Vec<String>,
}

impl SharedSource {
    fn new(source: BoxedDataSource, interval: Duration, panel_id: String) -> Self {
        Self {
            source,
            cached_values: HashMap::new(),
            ref_count: 1,
            min_interval: interval,
            panel_ids: vec![panel_id],
        }
    }

    /// Update the source and cache the values
    pub fn update(&mut self) -> Result<()> {
        self.source.update()?;
        self.cached_values = self.source.get_values();
        Ok(())
    }

    /// Get the cached values without polling hardware
    pub fn get_values(&self) -> &HashMap<String, serde_json::Value> {
        &self.cached_values
    }
}

/// Manages shared data source instances
///
/// Sources are keyed by a hash of their configuration, ensuring that
/// panels with identical source configs share the same source instance.
pub struct SharedSourceManager {
    /// Map from source key to shared source
    sources: RwLock<HashMap<String, SharedSource>>,
}

impl SharedSourceManager {
    pub fn new() -> Self {
        Self {
            sources: RwLock::new(HashMap::new()),
        }
    }

    /// Generate a unique key for a source configuration
    ///
    /// The key is based on source type and configuration parameters,
    /// but NOT the update interval (so panels with different intervals
    /// can share the same source).
    pub fn generate_source_key(source_config: &SourceConfig) -> String {
        use std::hash::{Hash, Hasher};
        use std::collections::hash_map::DefaultHasher;

        let mut hasher = DefaultHasher::new();

        // Hash the source type
        source_config.source_type().hash(&mut hasher);

        // Create a modified config without update_interval_ms for hashing
        // We serialize to JSON and remove the interval field to ensure
        // panels with different intervals can share the same source
        if let Ok(mut json_value) = serde_json::to_value(source_config) {
            // Remove update_interval_ms from the JSON before hashing
            if let Some(obj) = json_value.as_object_mut() {
                // The config is wrapped in an object like {"Cpu": {...}}
                for (_key, inner) in obj.iter_mut() {
                    if let Some(inner_obj) = inner.as_object_mut() {
                        inner_obj.remove("update_interval_ms");
                    }
                }
            }
            if let Ok(json_str) = serde_json::to_string(&json_value) {
                json_str.hash(&mut hasher);
            }
        }

        let hash = hasher.finish();
        format!("{}:{:016x}", source_config.source_type(), hash)
    }

    /// Get or create a shared source for the given configuration
    ///
    /// Returns the source key that can be used to retrieve values later.
    pub fn get_or_create_source(
        &self,
        source_config: &SourceConfig,
        panel_id: &str,
        registry: &Registry,
    ) -> Result<String> {
        let key = Self::generate_source_key(source_config);
        let interval = Duration::from_millis(source_config.update_interval_ms());

        let mut sources = self.sources.write().map_err(|e| anyhow!("Lock poisoned: {}", e))?;

        if let Some(shared) = sources.get_mut(&key) {
            // Source already exists, increment ref count and update interval if needed
            shared.ref_count += 1;
            shared.panel_ids.push(panel_id.to_string());
            if interval < shared.min_interval {
                info!(
                    "Updating min interval for source {} from {:?} to {:?} (panel {})",
                    key, shared.min_interval, interval, panel_id
                );
                shared.min_interval = interval;
            }
            debug!(
                "Reusing shared source {} for panel {} (ref_count: {})",
                key, panel_id, shared.ref_count
            );
        } else {
            // Create new source
            let mut source = registry.create_source(source_config.source_type())?;

            // Apply configuration to the source
            source.configure_typed(source_config)?;

            info!(
                "Created new shared source {} for panel {} (interval: {:?})",
                key, panel_id, interval
            );

            sources.insert(key.clone(), SharedSource::new(source, interval, panel_id.to_string()));
        }

        Ok(key)
    }

    /// Release a reference to a shared source
    ///
    /// When ref_count reaches 0, the source is removed.
    pub fn release_source(&self, key: &str, panel_id: &str) {
        if let Ok(mut sources) = self.sources.write() {
            if let Some(shared) = sources.get_mut(key) {
                shared.ref_count = shared.ref_count.saturating_sub(1);
                shared.panel_ids.retain(|id| id != panel_id);

                debug!(
                    "Released source {} for panel {} (ref_count: {})",
                    key, panel_id, shared.ref_count
                );

                if shared.ref_count == 0 {
                    info!("Removing unused shared source {}", key);
                    sources.remove(key);
                }
            }
        }
    }

    /// Update a specific source and return its values
    pub fn update_source(&self, key: &str) -> Result<HashMap<String, serde_json::Value>> {
        let mut sources = self.sources.write().map_err(|e| anyhow!("Lock poisoned: {}", e))?;

        if let Some(shared) = sources.get_mut(key) {
            shared.update()?;
            Ok(shared.cached_values.clone())
        } else {
            Err(anyhow!("Source not found: {}", key))
        }
    }

    /// Get cached values for a source (without updating)
    pub fn get_values(&self, key: &str) -> Option<HashMap<String, serde_json::Value>> {
        self.sources.read().ok()?.get(key).map(|s| s.cached_values.clone())
    }

    /// Get the minimum update interval for a source
    pub fn get_interval(&self, key: &str) -> Option<Duration> {
        self.sources.read().ok()?.get(key).map(|s| s.min_interval)
    }

    /// Get all source keys and their intervals for the update loop
    pub fn get_all_sources(&self) -> Vec<(String, Duration)> {
        self.sources
            .read()
            .map(|sources| {
                sources
                    .iter()
                    .map(|(key, shared)| (key.clone(), shared.min_interval))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Update the interval for a source (e.g., when a panel's config changes)
    pub fn update_interval(&self, key: &str, panel_id: &str, new_interval: Duration) {
        if let Ok(mut sources) = self.sources.write() {
            if let Some(shared) = sources.get_mut(key) {
                // Recalculate minimum interval across all panels
                // For now, just update if the new interval is smaller
                if new_interval < shared.min_interval {
                    info!(
                        "Panel {} updated interval for source {} to {:?}",
                        panel_id, key, new_interval
                    );
                    shared.min_interval = new_interval;
                }
            }
        }
    }

    /// Get a mutable reference to the source for configuration updates
    pub fn configure_source(&self, key: &str, config: &SourceConfig) -> Result<()> {
        let mut sources = self.sources.write().map_err(|e| anyhow!("Lock poisoned: {}", e))?;

        if let Some(shared) = sources.get_mut(key) {
            shared.source.configure_typed(config)?;
            Ok(())
        } else {
            Err(anyhow!("Source not found: {}", key))
        }
    }

    /// Get source metadata for UI purposes
    pub fn get_source_metadata(&self, key: &str) -> Option<super::SourceMetadata> {
        self.sources.read().ok()?.get(key).map(|s| s.source.metadata().clone())
    }

    /// Get the field list for a source
    pub fn get_source_fields(&self, key: &str) -> Option<Vec<super::FieldMetadata>> {
        self.sources.read().ok()?.get(key).map(|s| s.source.fields())
    }

    /// Debug: print all sources and their ref counts
    pub fn debug_print_sources(&self) {
        if let Ok(sources) = self.sources.read() {
            info!("=== Shared Sources ({} total) ===", sources.len());
            for (key, shared) in sources.iter() {
                info!(
                    "  {} : ref_count={}, interval={:?}, panels={:?}",
                    key, shared.ref_count, shared.min_interval, shared.panel_ids
                );
            }
        }
    }
}

impl Default for SharedSourceManager {
    fn default() -> Self {
        Self::new()
    }
}

// Global shared source manager instance
use once_cell::sync::OnceCell;
static GLOBAL_SHARED_SOURCE_MANAGER: OnceCell<Arc<SharedSourceManager>> = OnceCell::new();

/// Initialize the global shared source manager. Call this once at startup.
pub fn init_global_shared_source_manager(manager: Arc<SharedSourceManager>) {
    if GLOBAL_SHARED_SOURCE_MANAGER.set(manager).is_err() {
        warn!("Global shared source manager already initialized");
    }
}

/// Get the global shared source manager. Returns None if not initialized.
pub fn global_shared_source_manager() -> Option<&'static Arc<SharedSourceManager>> {
    GLOBAL_SHARED_SOURCE_MANAGER.get()
}
