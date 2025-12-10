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
        let panel_id = {
            let panel_guard = panel.read().await;
            panel_guard.id.clone()
        };

        let mut panels = self.panels.write().await;
        panels.insert(
            panel_id,
            PanelUpdateState {
                panel,
                last_update: Instant::now(),
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
        let panels = self.panels.read().await;

        // Collect panels that need updating
        let mut tasks = Vec::new();

        for (panel_id, state) in panels.iter() {
            // Get the panel's configured update interval
            let update_interval = {
                let panel_guard = state.panel.read().await;
                // Get update interval from CPU config if available
                if let Some(cpu_config_value) = panel_guard.config.get("cpu_config") {
                    match serde_json::from_value::<crate::ui::CpuSourceConfig>(cpu_config_value.clone()) {
                        Ok(cpu_config) => Duration::from_millis(cpu_config.update_interval_ms),
                        Err(e) => {
                            log::warn!("Failed to deserialize CPU config for panel {}: {}, using default interval", panel_id, e);
                            Duration::from_millis(1000)
                        }
                    }
                // Get update interval from GPU config if available
                } else if let Some(gpu_config_value) = panel_guard.config.get("gpu_config") {
                    match serde_json::from_value::<crate::ui::GpuSourceConfig>(gpu_config_value.clone()) {
                        Ok(gpu_config) => Duration::from_millis(gpu_config.update_interval_ms),
                        Err(e) => {
                            log::warn!("Failed to deserialize GPU config for panel {}: {}, using default interval", panel_id, e);
                            Duration::from_millis(1000)
                        }
                    }
                // Get update interval from Memory config if available
                } else if let Some(memory_config_value) = panel_guard.config.get("memory_config") {
                    match serde_json::from_value::<crate::ui::MemorySourceConfig>(memory_config_value.clone()) {
                        Ok(memory_config) => Duration::from_millis(memory_config.update_interval_ms),
                        Err(e) => {
                            log::warn!("Failed to deserialize Memory config for panel {}: {}, using default interval", panel_id, e);
                            Duration::from_millis(1000)
                        }
                    }
                // Get update interval from System Temp config if available
                } else if let Some(system_temp_config_value) = panel_guard.config.get("system_temp_config") {
                    match serde_json::from_value::<crate::sources::SystemTempConfig>(system_temp_config_value.clone()) {
                        Ok(system_temp_config) => Duration::from_millis(system_temp_config.update_interval_ms),
                        Err(e) => {
                            log::warn!("Failed to deserialize System Temp config for panel {}: {}, using default interval", panel_id, e);
                            Duration::from_millis(1000)
                        }
                    }
                // Get update interval from Fan Speed config if available
                } else if let Some(fan_speed_config_value) = panel_guard.config.get("fan_speed_config") {
                    match serde_json::from_value::<crate::sources::FanSpeedConfig>(fan_speed_config_value.clone()) {
                        Ok(fan_speed_config) => Duration::from_millis(fan_speed_config.update_interval_ms),
                        Err(e) => {
                            log::warn!("Failed to deserialize Fan Speed config for panel {}: {}, using default interval", panel_id, e);
                            Duration::from_millis(1000)
                        }
                    }
                } else {
                    Duration::from_millis(1000) // Default 1 second
                }
            };

            // Check if enough time has elapsed
            let elapsed = now.duration_since(state.last_update);
            if elapsed >= update_interval {
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

        drop(panels); // Release read lock

        // Update last_update times BEFORE waiting for tasks to complete
        // This ensures accurate interval timing regardless of task execution time
        let mut panels = self.panels.write().await;
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
