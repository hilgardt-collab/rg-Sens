//! Update manager for scheduling and coordinating source updates

use super::Panel;
use super::panel_data::SourceConfig;
use anyhow::Result;
use log::{error, trace, info};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::sync::mpsc;
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
    if let Some(combo_config_value) = config.get("combo_config") {
        match serde_json::from_value::<crate::sources::ComboSourceConfig>(combo_config_value.clone()) {
            Ok(combo_config) => return Duration::from_millis(combo_config.update_interval_ms),
            Err(e) => log::warn!("Failed to deserialize Combo config for panel {}: {}", panel_id, e),
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
    /// Sender for adding panels from sync code (GTK main thread)
    sender: mpsc::UnboundedSender<UpdateManagerMessage>,
    /// Receiver for processing panel additions (used in the update loop)
    receiver: Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<UpdateManagerMessage>>>,
}

impl UpdateManager {
    /// Create a new update manager
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        Self {
            panels: Arc::new(RwLock::new(HashMap::new())),
            sender,
            receiver: Arc::new(tokio::sync::Mutex::new(receiver)),
        }
    }

    /// Queue a panel to be added (can be called from sync code like GTK main thread)
    /// This is the recommended way to add panels after the update loop has started.
    pub fn queue_add_panel(&self, panel: Arc<RwLock<Panel>>) {
        if let Err(e) = self.sender.send(UpdateManagerMessage::AddPanel(panel)) {
            error!("Failed to queue panel for addition: {}", e);
        }
    }

    /// Queue a panel to be removed by ID (can be called from sync code)
    pub fn queue_remove_panel(&self, panel_id: String) {
        if let Err(e) = self.sender.send(UpdateManagerMessage::RemovePanel(panel_id)) {
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
        let (panel_id, config_hash, cached_interval) = {
            let panel_guard = panel.read().await;
            // Prefer PanelData if available (new typed config system)
            if let Some(ref data) = panel_guard.data {
                let hash = compute_config_hash_from_data(&data.source_config);
                let interval = Duration::from_millis(data.source_config.update_interval_ms());
                (panel_guard.id.clone(), hash, interval)
            } else {
                // Fall back to legacy HashMap config
                let hash = compute_config_hash(&panel_guard.config);
                let interval = extract_update_interval(&panel_guard.config, &panel_guard.id);
                (panel_guard.id.clone(), hash, interval)
            }
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
    async fn update_all(&self) -> Result<()> {
        let now = Instant::now();

        // First pass: collect panels that need updating and check for config changes
        // We need write lock to potentially update cached intervals
        let mut panels = self.panels.write().await;
        let mut tasks = Vec::new();
        let mut config_updates: Vec<(String, u64, Duration)> = Vec::new();

        for (panel_id, state) in panels.iter() {
            // Check if config has changed on EVERY tick (not just when update is due)
            // This ensures interval changes are detected immediately
            let (current_hash, new_interval) = {
                if let Ok(panel_guard) = state.panel.try_read() {
                    // Prefer PanelData if available
                    if let Some(ref data) = panel_guard.data {
                        let hash = compute_config_hash_from_data(&data.source_config);
                        let interval = Duration::from_millis(data.source_config.update_interval_ms());
                        (hash, Some(interval))
                    } else {
                        // Fall back to legacy HashMap config
                        let hash = compute_config_hash(&panel_guard.config);
                        (hash, None)
                    }
                } else {
                    (state.config_hash, None) // Keep old hash if can't read
                }
            };

            if current_hash != state.config_hash {
                // Config changed, need to update cached interval
                let interval = new_interval.unwrap_or_else(|| {
                    if let Ok(panel_guard) = state.panel.try_read() {
                        extract_update_interval(&panel_guard.config, panel_id)
                    } else {
                        state.cached_interval
                    }
                });
                config_updates.push((panel_id.clone(), current_hash, interval));
            }

            // Now check if update is due using (potentially updated) cached interval
            // Use the new interval if config changed, otherwise use cached
            let effective_interval = config_updates
                .iter()
                .find(|(id, _, _)| id == panel_id)
                .map(|(_, _, interval)| *interval)
                .unwrap_or(state.cached_interval);

            let elapsed = now.duration_since(state.last_update);
            if elapsed >= effective_interval {
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
        for (panel_id, new_hash, new_interval) in config_updates {
            if let Some(state) = panels.get_mut(&panel_id) {
                state.config_hash = new_hash;
                state.cached_interval = new_interval;
                info!("Updated cached interval for panel {}: {:?}", panel_id, state.cached_interval);
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
