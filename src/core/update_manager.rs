//! Update manager for scheduling and coordinating source updates

use super::Panel;
use anyhow::Result;
use log::{error, trace};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::Instant;

/// Tracks update timing for a panel
struct PanelUpdateState {
    panel: Arc<RwLock<Panel>>,
    last_update: Instant,
    /// Cached update interval to avoid deserializing config every cycle
    cached_interval: Duration,
    /// Hash of config to detect changes
    config_hash: u64,
}

/// Compute a simple hash of the config keys that affect update interval
fn compute_config_hash(config: &HashMap<String, serde_json::Value>) -> u64 {
    use std::hash::{Hash, Hasher};
    use std::collections::hash_map::DefaultHasher;

    let mut hasher = DefaultHasher::new();

    // Hash the relevant config keys (the ones that contain update_interval_ms)
    for key in ["cpu_config", "gpu_config", "memory_config", "system_temp_config",
                "fan_speed_config", "disk_config", "clock_config"] {
        if let Some(value) = config.get(key) {
            key.hash(&mut hasher);
            // Hash the string representation for simplicity
            value.to_string().hash(&mut hasher);
        }
    }

    hasher.finish()
}

/// Extract update interval from panel config
fn extract_update_interval(config: &HashMap<String, serde_json::Value>, panel_id: &str) -> Duration {
    if let Some(cpu_config_value) = config.get("cpu_config") {
        match serde_json::from_value::<crate::ui::CpuSourceConfig>(cpu_config_value.clone()) {
            Ok(cpu_config) => return Duration::from_millis(cpu_config.update_interval_ms),
            Err(e) => log::warn!("Failed to deserialize CPU config for panel {}: {}", panel_id, e),
        }
    }
    if let Some(gpu_config_value) = config.get("gpu_config") {
        match serde_json::from_value::<crate::ui::GpuSourceConfig>(gpu_config_value.clone()) {
            Ok(gpu_config) => return Duration::from_millis(gpu_config.update_interval_ms),
            Err(e) => log::warn!("Failed to deserialize GPU config for panel {}: {}", panel_id, e),
        }
    }
    if let Some(memory_config_value) = config.get("memory_config") {
        match serde_json::from_value::<crate::ui::MemorySourceConfig>(memory_config_value.clone()) {
            Ok(memory_config) => return Duration::from_millis(memory_config.update_interval_ms),
            Err(e) => log::warn!("Failed to deserialize Memory config for panel {}: {}", panel_id, e),
        }
    }
    if let Some(system_temp_config_value) = config.get("system_temp_config") {
        match serde_json::from_value::<crate::sources::SystemTempConfig>(system_temp_config_value.clone()) {
            Ok(system_temp_config) => return Duration::from_millis(system_temp_config.update_interval_ms),
            Err(e) => log::warn!("Failed to deserialize System Temp config for panel {}: {}", panel_id, e),
        }
    }
    if let Some(fan_speed_config_value) = config.get("fan_speed_config") {
        match serde_json::from_value::<crate::sources::FanSpeedConfig>(fan_speed_config_value.clone()) {
            Ok(fan_speed_config) => return Duration::from_millis(fan_speed_config.update_interval_ms),
            Err(e) => log::warn!("Failed to deserialize Fan Speed config for panel {}: {}", panel_id, e),
        }
    }
    if let Some(disk_config_value) = config.get("disk_config") {
        match serde_json::from_value::<crate::ui::DiskSourceConfig>(disk_config_value.clone()) {
            Ok(disk_config) => return Duration::from_millis(disk_config.update_interval_ms),
            Err(e) => log::warn!("Failed to deserialize Disk config for panel {}: {}", panel_id, e),
        }
    }
    if let Some(clock_config_value) = config.get("clock_config") {
        match serde_json::from_value::<crate::sources::ClockSourceConfig>(clock_config_value.clone()) {
            Ok(clock_config) => return Duration::from_millis(clock_config.update_interval_ms),
            Err(e) => log::warn!("Failed to deserialize Clock config for panel {}: {}", panel_id, e),
        }
    }

    Duration::from_millis(1000) // Default 1 second
}

/// Manages periodic updates for panels
pub struct UpdateManager {
    panels: Arc<RwLock<HashMap<String, PanelUpdateState>>>,
}

impl UpdateManager {
    /// Create a new update manager
    pub fn new() -> Self {
        Self {
            panels: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a panel to be updated
    pub async fn add_panel(&self, panel: Arc<RwLock<Panel>>) {
        let (panel_id, config_hash, cached_interval) = {
            let panel_guard = panel.read().await;
            let hash = compute_config_hash(&panel_guard.config);
            let interval = extract_update_interval(&panel_guard.config, &panel_guard.id);
            (panel_guard.id.clone(), hash, interval)
        };

        let mut panels = self.panels.write().await;
        panels.insert(
            panel_id,
            PanelUpdateState {
                panel,
                last_update: Instant::now(),
                cached_interval,
                config_hash,
            },
        );
    }

    /// Start the update loop
    ///
    /// This runs indefinitely, updating each panel at its configured interval.
    pub async fn run(&self, base_interval: Duration) {
        let mut interval = tokio::time::interval(base_interval);

        loop {
            interval.tick().await;

            let start = Instant::now();
            if let Err(e) = self.update_all().await {
                error!("Error updating panels: {}", e);
            }

            let elapsed = start.elapsed();
            trace!("Update cycle took {:?}", elapsed);
        }
    }

    /// Update all panels that are due for an update
    async fn update_all(&self) -> Result<()> {
        let now = Instant::now();

        // First pass: collect panels that need updating and check for config changes
        // We need write lock to potentially update cached intervals
        let mut panels = self.panels.write().await;
        let mut tasks = Vec::new();
        let mut config_updates = Vec::new();

        for (panel_id, state) in panels.iter() {
            // Quick check using cached interval (no deserialization!)
            let elapsed = now.duration_since(state.last_update);
            if elapsed >= state.cached_interval {
                // Check if config has changed (need to re-parse interval)
                let current_hash = {
                    if let Ok(panel_guard) = state.panel.try_read() {
                        compute_config_hash(&panel_guard.config)
                    } else {
                        state.config_hash // Keep old hash if can't read
                    }
                };

                if current_hash != state.config_hash {
                    // Config changed, need to update cached interval
                    config_updates.push((panel_id.clone(), current_hash));
                }

                let panel = state.panel.clone();
                let panel_id_owned = panel_id.clone();
                let panel_id_for_task = panel_id_owned.clone();
                let task = tokio::spawn(async move {
                    let mut panel_guard = panel.write().await;
                    if let Err(e) = panel_guard.update() {
                        error!("Error updating panel {}: {}", panel_id_for_task, e);
                    }
                });
                tasks.push((panel_id_owned, task));
            }
        }

        // Update cached intervals for panels with config changes
        for (panel_id, new_hash) in config_updates {
            if let Some(state) = panels.get_mut(&panel_id) {
                state.config_hash = new_hash;
                // Re-parse interval only when config actually changed
                if let Ok(panel_guard) = state.panel.try_read() {
                    state.cached_interval = extract_update_interval(&panel_guard.config, &panel_id);
                    trace!("Updated cached interval for panel {}: {:?}", panel_id, state.cached_interval);
                }
            }
        }

        // Update last_update times
        for (panel_id, _) in &tasks {
            if let Some(state) = panels.get_mut(panel_id) {
                state.last_update = now;
            }
        }

        drop(panels); // Release write lock before awaiting tasks

        // Wait for all updates to complete
        for (panel_id, task) in tasks {
            if let Err(e) = task.await {
                error!("Panel update task failed for {}: {}", panel_id, e);
            }
        }

        Ok(())
    }
}

impl Default for UpdateManager {
    fn default() -> Self {
        Self::new()
    }
}
