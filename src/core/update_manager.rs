//! Update manager for scheduling and coordinating source updates
//!
//! The update flow is:
//! 1. Collect all unique shared sources that need updating
//! 2. Update each shared source ONCE (regardless of how many panels use it)
//! 3. Update each panel's displayer with the cached values from its shared source

use super::Panel;
use super::panel_data::SourceConfig;
use super::shared_source_manager::global_shared_source_manager;
use anyhow::Result;
use log::{error, trace, info, debug};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::sync::mpsc;
use tokio::time::Instant;

/// Minimum interval between config hash checks (500ms)
/// Config changes are rare (user must open dialog, change settings, click save)
/// so checking every tick is wasteful
const CONFIG_CHECK_INTERVAL: Duration = Duration::from_millis(500);

/// Tracks update timing for a panel
struct PanelUpdateState {
    panel: Arc<RwLock<Panel>>,
    last_update: Instant,
    /// Last time we checked for config changes
    last_config_check: Instant,
    /// Cached update interval to avoid deserializing config every cycle
    cached_interval: Duration,
    /// Hash of config to detect changes
    config_hash: u64,
    /// Key to the shared source (if using shared sources)
    source_key: Option<String>,
}

/// Tracks update timing for a shared source
struct SharedSourceUpdateState {
    last_update: Instant,
    interval: Duration,
}

/// Compute a hash from PanelData's source config (preferred method)
fn compute_config_hash_from_data(source_config: &SourceConfig) -> u64 {
    use std::hash::{Hash, Hasher};
    use std::collections::hash_map::DefaultHasher;

    let mut hasher = DefaultHasher::new();

    // Hash the source type and update interval
    source_config.source_type().hash(&mut hasher);
    source_config.update_interval_ms().hash(&mut hasher);

    // Hash the serialized config for a complete picture
    if let Ok(json) = serde_json::to_string(source_config) {
        json.hash(&mut hasher);
    }

    hasher.finish()
}

/// Compute a simple hash of the config keys that affect update interval (legacy fallback)
fn compute_config_hash(config: &HashMap<String, serde_json::Value>) -> u64 {
    use std::hash::{Hash, Hasher};
    use std::collections::hash_map::DefaultHasher;

    let mut hasher = DefaultHasher::new();

    // Hash the relevant config keys (the ones that contain update_interval_ms)
    for key in ["cpu_config", "gpu_config", "memory_config", "system_temp_config",
                "fan_speed_config", "disk_config", "clock_config"] {
        if let Some(value) = config.get(key) {
            key.hash(&mut hasher);
            // Use to_vec() instead of to_string() - avoids UTF-8 validation overhead
            if let Ok(bytes) = serde_json::to_vec(value) {
                bytes.hash(&mut hasher);
            }
        }
    }

    hasher.finish()
}

/// Extract update interval from panel config
///
/// Optimized to directly extract `update_interval_ms` field from JSON without
/// deserializing the entire config struct.
fn extract_update_interval(config: &HashMap<String, serde_json::Value>, _panel_id: &str) -> Duration {
    // All source configs have update_interval_ms as a field
    // Instead of deserializing the full struct, just extract the field directly
    const CONFIG_KEYS: &[&str] = &[
        "cpu_config", "gpu_config", "memory_config", "system_temp_config",
        "fan_speed_config", "disk_config", "clock_config", "combo_config",
    ];

    for key in CONFIG_KEYS {
        if let Some(config_value) = config.get(*key) {
            if let Some(obj) = config_value.as_object() {
                if let Some(interval) = obj.get("update_interval_ms") {
                    if let Some(ms) = interval.as_u64() {
                        return Duration::from_millis(ms);
                    }
                }
            }
        }
    }

    Duration::from_millis(1000) // Default 1 second
}

/// Channel message for adding panels
enum UpdateManagerMessage {
    AddPanel(Arc<RwLock<Panel>>),
    RemovePanel(String),
}

/// Manages periodic updates for panels
pub struct UpdateManager {
    panels: Arc<RwLock<HashMap<String, PanelUpdateState>>>,
    /// Tracks shared source update timing
    shared_sources: Arc<RwLock<HashMap<String, SharedSourceUpdateState>>>,
    /// Sender for adding panels from sync code (GTK main thread)
    /// Uses bounded channel (capacity 256) to provide backpressure
    sender: mpsc::Sender<UpdateManagerMessage>,
    /// Receiver for processing panel additions (used in the update loop)
    receiver: Arc<tokio::sync::Mutex<mpsc::Receiver<UpdateManagerMessage>>>,
}

impl UpdateManager {
    /// Create a new update manager
    pub fn new() -> Self {
        // Use bounded channel with capacity 256 to prevent unbounded memory growth
        let (sender, receiver) = mpsc::channel(256);
        Self {
            panels: Arc::new(RwLock::new(HashMap::new())),
            shared_sources: Arc::new(RwLock::new(HashMap::new())),
            sender,
            receiver: Arc::new(tokio::sync::Mutex::new(receiver)),
        }
    }

    /// Queue a panel to be added (can be called from sync code like GTK main thread)
    /// This is the recommended way to add panels after the update loop has started.
    pub fn queue_add_panel(&self, panel: Arc<RwLock<Panel>>) {
        // Use try_send for non-blocking send from sync code
        if let Err(e) = self.sender.try_send(UpdateManagerMessage::AddPanel(panel)) {
            error!("Failed to queue panel for addition: {}", e);
        }
    }

    /// Queue a panel to be removed by ID (can be called from sync code)
    pub fn queue_remove_panel(&self, panel_id: String) {
        // Use try_send for non-blocking send from sync code
        if let Err(e) = self.sender.try_send(UpdateManagerMessage::RemovePanel(panel_id)) {
            error!("Failed to queue panel for removal: {}", e);
        }
    }

    /// Process any pending messages (add/remove panels)
    async fn process_messages(&self) {
        let mut receiver = self.receiver.lock().await;
        while let Ok(msg) = receiver.try_recv() {
            match msg {
                UpdateManagerMessage::AddPanel(panel) => {
                    self.add_panel(panel).await;
                }
                UpdateManagerMessage::RemovePanel(panel_id) => {
                    let mut panels = self.panels.write().await;
                    if panels.remove(&panel_id).is_some() {
                        info!("Removed panel {} from update manager", panel_id);
                    }
                }
            }
        }
    }

    /// Add a panel to be updated
    pub async fn add_panel(&self, panel: Arc<RwLock<Panel>>) {
        let (panel_id, config_hash, cached_interval, source_key) = {
            let panel_guard = panel.read().await;
            // Prefer PanelData if available (new typed config system)
            if let Some(ref data) = panel_guard.data {
                let hash = compute_config_hash_from_data(&data.source_config);
                let interval = Duration::from_millis(data.source_config.update_interval_ms());
                (panel_guard.id.clone(), hash, interval, panel_guard.source_key.clone())
            } else {
                // Fall back to legacy HashMap config
                let hash = compute_config_hash(&panel_guard.config);
                let interval = extract_update_interval(&panel_guard.config, &panel_guard.id);
                (panel_guard.id.clone(), hash, interval, None)
            }
        };

        // Register shared source if panel uses one
        if let Some(ref key) = source_key {
            let mut shared_sources = self.shared_sources.write().await;
            if !shared_sources.contains_key(key) {
                // Get interval from SharedSourceManager
                let interval = global_shared_source_manager()
                    .and_then(|m| m.get_interval(key))
                    .unwrap_or(cached_interval);

                shared_sources.insert(
                    key.clone(),
                    SharedSourceUpdateState {
                        last_update: Instant::now() - interval, // Force immediate update
                        interval,
                    },
                );
                debug!("Registered shared source {} with interval {:?}", key, interval);
            }
        }

        let mut panels = self.panels.write().await;
        panels.insert(
            panel_id.clone(),
            PanelUpdateState {
                panel,
                last_update: Instant::now(),
                last_config_check: Instant::now(),
                cached_interval,
                config_hash,
                source_key,
            },
        );
        debug!("Added panel {} to update manager", panel_id);
    }

    /// Start the update loop
    ///
    /// This runs indefinitely, updating each panel at its configured interval.
    pub async fn run(&self, base_interval: Duration) {
        let mut interval = tokio::time::interval(base_interval);

        loop {
            interval.tick().await;

            // Process any pending add/remove messages first
            self.process_messages().await;

            let start = Instant::now();
            if let Err(e) = self.update_all().await {
                error!("Error updating panels: {}", e);
            }

            let elapsed = start.elapsed();
            trace!("Update cycle took {:?}", elapsed);
        }
    }

    /// Update all panels that are due for an update
    ///
    /// The update flow is:
    /// 1. Update shared sources that are due (each source polled only once)
    /// 2. Update panels that are due (using cached values from shared sources)
    async fn update_all(&self) -> Result<()> {
        let now = Instant::now();

        // === PHASE 1: Update shared sources ===
        let updated_sources = self.update_shared_sources(now).await;

        // === PHASE 2: Update panels ===
        let mut panels = self.panels.write().await;
        let mut tasks = Vec::new();
        let mut config_updates: Vec<(String, u64, Duration, Option<String>)> = Vec::new();

        // Track panels that had their config checked (for updating last_config_check)
        let mut config_checked: Vec<String> = Vec::new();

        for (panel_id, state) in panels.iter() {
            // Only check config hash if enough time has elapsed (throttle to avoid CPU waste)
            let should_check_config = now.duration_since(state.last_config_check) >= CONFIG_CHECK_INTERVAL;

            let (current_hash, new_interval, new_source_key) = if should_check_config {
                config_checked.push(panel_id.clone());
                if let Ok(panel_guard) = state.panel.try_read() {
                    if let Some(ref data) = panel_guard.data {
                        let hash = compute_config_hash_from_data(&data.source_config);
                        let interval = Duration::from_millis(data.source_config.update_interval_ms());
                        (hash, Some(interval), panel_guard.source_key.clone())
                    } else {
                        let hash = compute_config_hash(&panel_guard.config);
                        (hash, None, None)
                    }
                } else {
                    (state.config_hash, None, state.source_key.clone())
                }
            } else {
                // Skip hash computation, use cached values
                (state.config_hash, None, state.source_key.clone())
            };

            if current_hash != state.config_hash {
                let interval = new_interval.unwrap_or_else(|| {
                    if let Ok(panel_guard) = state.panel.try_read() {
                        extract_update_interval(&panel_guard.config, panel_id)
                    } else {
                        state.cached_interval
                    }
                });
                config_updates.push((panel_id.clone(), current_hash, interval, new_source_key.clone()));
            }

            // Check if update is due
            let effective_interval = config_updates
                .iter()
                .find(|(id, _, _, _)| id == panel_id)
                .map(|(_, _, interval, _)| *interval)
                .unwrap_or(state.cached_interval);

            let elapsed = now.duration_since(state.last_update);
            if elapsed >= effective_interval {
                let panel = state.panel.clone();
                let panel_id_owned = panel_id.clone();
                let panel_id_for_task = panel_id_owned.clone();
                let source_key = state.source_key.clone();

                // Check if this panel's source was updated in phase 1
                // (currently unused but may be useful for future optimizations)
                let _source_was_updated = source_key
                    .as_ref()
                    .map(|k| updated_sources.contains(k))
                    .unwrap_or(false);

                let task = tokio::spawn(async move {
                    let mut panel_guard = panel.write().await;

                    // If using shared source and it was updated, panel.update() will use cached values
                    // If not using shared source, panel.update() will poll directly
                    if let Err(e) = panel_guard.update() {
                        error!("Error updating panel {}: {}", panel_id_for_task, e);
                    }
                });
                tasks.push((panel_id_owned, task));
            }
        }

        // Update cached state for panels with config changes
        // Also track old source keys for cleanup
        let mut old_source_keys: Vec<String> = Vec::new();
        for (panel_id, new_hash, new_interval, new_source_key) in config_updates {
            if let Some(state) = panels.get_mut(&panel_id) {
                // Check if source_key changed and register new shared source if needed
                if state.source_key != new_source_key {
                    // Track old key for potential cleanup
                    if let Some(ref old_key) = state.source_key {
                        old_source_keys.push(old_key.clone());
                    }

                    if let Some(ref new_key) = new_source_key {
                        let mut shared_sources = self.shared_sources.write().await;
                        if !shared_sources.contains_key(new_key) {
                            // Get interval from SharedSourceManager
                            let interval = global_shared_source_manager()
                                .and_then(|m| m.get_interval(new_key))
                                .unwrap_or(new_interval);

                            shared_sources.insert(
                                new_key.clone(),
                                SharedSourceUpdateState {
                                    last_update: Instant::now() - interval, // Force immediate update
                                    interval,
                                },
                            );
                            debug!("Registered new shared source {} for panel {} with interval {:?}", new_key, panel_id, interval);
                        }
                    }
                }

                state.config_hash = new_hash;
                state.cached_interval = new_interval;
                state.source_key = new_source_key;
                info!("Updated cached interval for panel {}: {:?}", panel_id, state.cached_interval);
            }
        }

        // Update last_config_check for all panels that had their config checked
        for panel_id in config_checked {
            if let Some(state) = panels.get_mut(&panel_id) {
                state.last_config_check = now;
            }
        }

        // Clean up old shared sources that are no longer referenced by any panel
        // Use linear search instead of HashSet - old_source_keys is typically 0-1 items
        if !old_source_keys.is_empty() {
            let mut shared_sources = self.shared_sources.write().await;
            for old_key in old_source_keys {
                let is_still_active = panels.values().any(|s| s.source_key.as_ref() == Some(&old_key));
                if !is_still_active {
                    shared_sources.remove(&old_key);
                    debug!("Removed unused shared source {} from update manager", old_key);
                }
            }
        }

        // Update last_update times
        for (panel_id, _) in &tasks {
            if let Some(state) = panels.get_mut(panel_id) {
                state.last_update = now;
            }
        }

        drop(panels);

        // Wait for all panel updates to complete
        for (panel_id, task) in tasks {
            if let Err(e) = task.await {
                error!("Panel update task failed for {}: {}", panel_id, e);
            }
        }

        Ok(())
    }

    /// Update shared sources that are due for an update
    ///
    /// Returns a set of source keys that were updated
    async fn update_shared_sources(&self, now: Instant) -> std::collections::HashSet<String> {
        let mut updated = std::collections::HashSet::new();

        let manager = match global_shared_source_manager() {
            Some(m) => m,
            None => return updated,
        };

        let mut shared_sources = self.shared_sources.write().await;

        // Collect sources that need updating
        let sources_to_update: Vec<String> = shared_sources
            .iter()
            .filter(|(_, state)| now.duration_since(state.last_update) >= state.interval)
            .map(|(key, _)| key.clone())
            .collect();

        // Track sources that should be removed (no longer exist in SharedSourceManager)
        let mut sources_to_remove: Vec<String> = Vec::new();

        // Update each source
        for key in sources_to_update {
            match manager.update_source(&key) {
                Ok(_) => {
                    trace!("Updated shared source {}", key);
                    updated.insert(key.clone());

                    // Update last_update time
                    if let Some(state) = shared_sources.get_mut(&key) {
                        state.last_update = now;

                        // Also refresh interval from manager in case it changed
                        if let Some(new_interval) = manager.get_interval(&key) {
                            if new_interval != state.interval {
                                debug!(
                                    "Shared source {} interval changed from {:?} to {:?}",
                                    key, state.interval, new_interval
                                );
                                state.interval = new_interval;
                            }
                        }
                    }
                }
                Err(e) => {
                    // Source not found means it was released (e.g., panel changed source)
                    // Remove it from our tracking to prevent repeated errors
                    if e.to_string().contains("Source not found") {
                        debug!("Shared source {} no longer exists, removing from update tracking", key);
                        sources_to_remove.push(key);
                    } else {
                        error!("Error updating shared source {}: {}", key, e);
                    }
                }
            }
        }

        // Remove stale sources from tracking
        for key in sources_to_remove {
            shared_sources.remove(&key);
        }

        updated
    }
}

impl Default for UpdateManager {
    fn default() -> Self {
        Self::new()
    }
}

// Global update manager instance
use once_cell::sync::OnceCell;
static GLOBAL_UPDATE_MANAGER: OnceCell<Arc<UpdateManager>> = OnceCell::new();

/// Initialize the global update manager. Call this once at startup.
pub fn init_global_update_manager(manager: Arc<UpdateManager>) {
    if GLOBAL_UPDATE_MANAGER.set(manager).is_err() {
        log::warn!("Global update manager already initialized");
    }
}

/// Get the global update manager. Returns None if not initialized.
pub fn global_update_manager() -> Option<&'static Arc<UpdateManager>> {
    GLOBAL_UPDATE_MANAGER.get()
}
