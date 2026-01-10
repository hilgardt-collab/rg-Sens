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
use std::sync::{Arc, RwLock, Mutex};
use std::time::Duration;

/// Represents a shared source with its cached values and update tracking
///
/// Wrapped in Arc<Mutex<>> to allow updating individual sources without
/// holding the collection lock during I/O operations.
pub struct SharedSource {
    /// The actual data source instance
    pub source: BoxedDataSource,
    /// Cached values from the last update (Arc for cheap cloning)
    pub cached_values: Arc<HashMap<String, serde_json::Value>>,
    /// Number of panels using this source
    pub ref_count: usize,
    /// Minimum update interval requested by any panel using this source
    pub min_interval: Duration,
    /// Map of panel IDs to their requested intervals (for recalculating min)
    pub panel_intervals: HashMap<String, Duration>,
}

impl SharedSource {
    fn new(source: BoxedDataSource, interval: Duration, panel_id: String) -> Self {
        let mut panel_intervals = HashMap::new();
        panel_intervals.insert(panel_id, interval);
        Self {
            source,
            cached_values: Arc::new(HashMap::new()),
            ref_count: 1,
            min_interval: interval,
            panel_intervals,
        }
    }

    /// Recalculate min_interval from panel_intervals
    fn recalculate_min_interval(&mut self) {
        self.min_interval = self.panel_intervals
            .values()
            .copied()
            .min()
            .unwrap_or(Duration::from_millis(1000));
    }

    /// Update the source and cache the values
    pub fn update(&mut self) -> Result<()> {
        self.source.update()?;
        // Use values_ref if available to avoid cloning (e.g., for ComboSource)
        self.cached_values = if let Some(values) = self.source.values_ref() {
            Arc::new(values.clone())
        } else {
            Arc::new(self.source.get_values())
        };
        Ok(())
    }

    /// Get the cached values without polling hardware (cheap Arc clone)
    pub fn get_values(&self) -> Arc<HashMap<String, serde_json::Value>> {
        Arc::clone(&self.cached_values)
    }
}

/// Thread-safe wrapper for SharedSource
type SharedSourceHandle = Arc<Mutex<SharedSource>>;

/// Manages shared data source instances
///
/// Sources are keyed by a hash of their configuration, ensuring that
/// panels with identical source configs share the same source instance.
///
/// Each SharedSource is wrapped in Arc<Mutex<>> to allow updating individual
/// sources without holding the collection lock during I/O operations.
pub struct SharedSourceManager {
    /// Map from source key to shared source handle
    sources: RwLock<HashMap<String, SharedSourceHandle>>,
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
    ///
    /// # Performance Note
    /// This method is optimized to minimize lock contention. Source creation
    /// and initial updates happen outside the write lock to avoid blocking
    /// the update loop.
    pub fn get_or_create_source(
        &self,
        source_config: &SourceConfig,
        panel_id: &str,
        registry: &Registry,
    ) -> Result<String> {
        let key = Self::generate_source_key(source_config);
        let interval = Duration::from_millis(source_config.update_interval_ms());

        // Phase 1: Check if source exists (quick read lock)
        // Clone the Arc handle while holding the lock, then release lock before acquiring Mutex
        // This avoids potential deadlock from nested RwLock -> Mutex acquisition
        let existing_handle = {
            let sources = self.sources.read().map_err(|e| anyhow!("Lock poisoned: {}", e))?;
            sources.get(&key).cloned()
        };
        // Read lock released here - now safe to acquire Mutex

        if let Some(handle) = existing_handle {
            // Source already exists, increment ref count and track panel interval
            if let Ok(mut shared) = handle.lock() {
                shared.ref_count += 1;
                shared.panel_intervals.insert(panel_id.to_string(), interval);
                shared.recalculate_min_interval();
                debug!(
                    "Reusing shared source {} for panel {} (ref_count: {}, min_interval: {:?})",
                    key, panel_id, shared.ref_count, shared.min_interval
                );
            }
            return Ok(key);
        }

        // Phase 2: Create new source OUTSIDE the lock (slow I/O)
        let mut source = registry.create_source(source_config.source_type())?;
        source.configure_typed(source_config)?;

        info!(
            "Created new shared source {} for panel {} (interval: {:?})",
            key, panel_id, interval
        );

        // Create the shared source and do initial update BEFORE acquiring write lock
        let mut shared = SharedSource::new(source, interval, panel_id.to_string());
        if let Err(e) = shared.update() {
            warn!("Initial update failed for source {}: {}", key, e);
        }

        // Phase 3: Insert into map (quick write lock)
        // Re-check in case another thread created it while we were doing I/O
        // Clone handle while holding lock to avoid nested RwLock -> Mutex deadlock
        let existing_handle = {
            let sources = self.sources.read().map_err(|e| anyhow!("Lock poisoned: {}", e))?;
            sources.get(&key).cloned()
        };

        if let Some(handle) = existing_handle {
            // Another thread created it - just increment ref count and discard ours
            if let Ok(mut existing) = handle.lock() {
                existing.ref_count += 1;
                existing.panel_intervals.insert(panel_id.to_string(), interval);
                existing.recalculate_min_interval();
                debug!(
                    "Source {} was created by another thread, reusing (ref_count: {})",
                    key, existing.ref_count
                );
            }
        } else {
            // No existing source, insert new one
            let mut sources = self.sources.write().map_err(|e| anyhow!("Lock poisoned: {}", e))?;
            // Double-check after acquiring write lock (another thread may have inserted)
            sources.entry(key.clone()).or_insert_with(|| Arc::new(Mutex::new(shared)));
        }

        Ok(key)
    }

    /// Release a reference to a shared source
    ///
    /// When ref_count reaches 0, the source is removed.
    /// When a panel is removed, the min_interval is recalculated.
    ///
    /// # Performance Note
    /// Uses read lock for the common case (decrementing ref count) and only
    /// acquires write lock when the source needs to be removed.
    pub fn release_source(&self, key: &str, panel_id: &str) {
        // Phase 1: Clone handle while holding read lock to avoid nested RwLock -> Mutex deadlock
        let handle = {
            let sources = match self.sources.read() {
                Ok(s) => s,
                Err(_) => return,
            };
            sources.get(key).cloned()
        };
        // Read lock released here - now safe to acquire Mutex

        let should_remove = if let Some(handle) = handle {
            if let Ok(mut shared) = handle.lock() {
                shared.ref_count = shared.ref_count.saturating_sub(1);
                shared.panel_intervals.remove(panel_id);
                shared.recalculate_min_interval();

                debug!(
                    "Released source {} for panel {} (ref_count: {}, min_interval: {:?})",
                    key, panel_id, shared.ref_count, shared.min_interval
                );

                shared.ref_count == 0
            } else {
                false
            }
        } else {
            false
        };

        // Phase 2: Remove with write lock only if needed
        if should_remove {
            // Clone handle to re-check ref_count without holding RwLock
            let handle_for_recheck = {
                let sources = match self.sources.read() {
                    Ok(s) => s,
                    Err(_) => return,
                };
                sources.get(key).cloned()
            };

            // Check ref_count without holding any RwLock (avoids deadlock)
            let still_empty = if let Some(ref h) = handle_for_recheck {
                h.lock().ok().map(|s| s.ref_count == 0).unwrap_or(false)
            } else {
                false
            };

            if still_empty {
                // Safe to acquire write lock now - no mutex held
                if let Ok(mut sources) = self.sources.write() {
                    // Use remove_entry pattern to avoid nested lock during final check
                    // If ref_count was incremented by another thread, they have their own Arc
                    if let Some((_, handle)) = sources.remove_entry(key) {
                        // Final check after removal - if ref_count > 0, put it back
                        if let Ok(shared) = handle.lock() {
                            if shared.ref_count > 0 {
                                // Another thread added a reference, put it back
                                sources.insert(key.to_string(), handle.clone());
                                debug!("Source {} was re-referenced, keeping", key);
                            } else {
                                info!("Removed unused shared source {}", key);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Update a specific source and return its values (cheap Arc clone)
    ///
    /// This method is optimized to release the collection lock before performing
    /// I/O operations. Only the individual source mutex is held during hardware polling.
    pub fn update_source(&self, key: &str) -> Result<Arc<HashMap<String, serde_json::Value>>> {
        // Phase 1: Get the handle with a quick read lock
        let handle = {
            let sources = self.sources.read().map_err(|e| anyhow!("Lock poisoned: {}", e))?;
            sources.get(key).cloned()
        };
        // Collection lock released here before I/O

        // Phase 2: Update the source (only source mutex held, not collection lock)
        if let Some(handle) = handle {
            let mut shared = handle.lock().map_err(|e| anyhow!("Source lock poisoned: {}", e))?;
            shared.update()?;
            Ok(Arc::clone(&shared.cached_values))
        } else {
            Err(anyhow!("Source not found: {}", key))
        }
    }

    /// Get cached values for a source (without updating) - cheap Arc clone
    pub fn get_values(&self, key: &str) -> Option<Arc<HashMap<String, serde_json::Value>>> {
        let sources = self.sources.read().ok()?;
        let handle = sources.get(key)?;
        let shared = handle.lock().ok()?;
        Some(Arc::clone(&shared.cached_values))
    }

    /// Get the minimum update interval for a source
    pub fn get_interval(&self, key: &str) -> Option<Duration> {
        let sources = self.sources.read().ok()?;
        let handle = sources.get(key)?;
        let shared = handle.lock().ok()?;
        Some(shared.min_interval)
    }

    /// Get all source keys and their intervals for the update loop
    pub fn get_all_sources(&self) -> Vec<(String, Duration)> {
        self.sources
            .read()
            .map(|sources| {
                sources
                    .iter()
                    .filter_map(|(key, handle)| {
                        handle.lock().ok().map(|shared| (key.clone(), shared.min_interval))
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Update the interval for a source (e.g., when a panel's config changes)
    pub fn update_interval(&self, key: &str, panel_id: &str, new_interval: Duration) {
        if let Ok(sources) = self.sources.read() {
            if let Some(handle) = sources.get(key) {
                if let Ok(mut shared) = handle.lock() {
                    // Update this panel's interval and recalculate minimum
                    let old_min = shared.min_interval;
                    shared.panel_intervals.insert(panel_id.to_string(), new_interval);
                    shared.recalculate_min_interval();

                    if shared.min_interval != old_min {
                        info!(
                            "Panel {} updated interval for source {} from {:?} to {:?}",
                            panel_id, key, old_min, shared.min_interval
                        );
                    }
                }
            }
        }
    }

    /// Configure a source (e.g., when a panel's config changes)
    pub fn configure_source(&self, key: &str, config: &SourceConfig) -> Result<()> {
        // Get handle with read lock (quick)
        let handle = {
            let sources = self.sources.read().map_err(|e| anyhow!("Lock poisoned: {}", e))?;
            sources.get(key).cloned()
        };

        // Configure with only source lock held
        if let Some(handle) = handle {
            let mut shared = handle.lock().map_err(|e| anyhow!("Source lock poisoned: {}", e))?;
            shared.source.configure_typed(config)?;
            Ok(())
        } else {
            Err(anyhow!("Source not found: {}", key))
        }
    }

    /// Get source metadata for UI purposes
    pub fn get_source_metadata(&self, key: &str) -> Option<super::SourceMetadata> {
        let sources = self.sources.read().ok()?;
        let handle = sources.get(key)?;
        let shared = handle.lock().ok()?;
        Some(shared.source.metadata().clone())
    }

    /// Get the field list for a source
    pub fn get_source_fields(&self, key: &str) -> Option<Vec<super::FieldMetadata>> {
        let sources = self.sources.read().ok()?;
        let handle = sources.get(key)?;
        let shared = handle.lock().ok()?;
        Some(shared.source.fields())
    }

    /// Debug: print all sources and their ref counts
    pub fn debug_print_sources(&self) {
        if let Ok(sources) = self.sources.read() {
            info!("=== Shared Sources ({} total) ===", sources.len());
            for (key, handle) in sources.iter() {
                if let Ok(shared) = handle.lock() {
                    let panel_ids: Vec<_> = shared.panel_intervals.keys().collect();
                    info!(
                        "  {} : ref_count={}, interval={:?}, panels={:?}",
                        key, shared.ref_count, shared.min_interval, panel_ids
                    );
                }
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
