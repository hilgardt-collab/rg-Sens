//! Update manager for scheduling and coordinating source updates
//!
//! The update flow is:
//! 1. Collect all unique shared sources that need updating
//! 2. Update each shared source ONCE (regardless of how many panels use it)
//! 3. Update each panel's displayer with the cached values from its shared source

use super::panel_data::SourceConfig;
use super::shared_source_manager::global_shared_source_manager;
use super::Panel;
use anyhow::Result;
use log::{debug, error, info, trace};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::sync::RwLock;
use tokio::time::Instant;

/// Minimum interval between config hash checks (1 second)
/// Config changes are rare (user must open dialog, change settings, click save)
/// so checking frequently is wasteful
const CONFIG_CHECK_INTERVAL: Duration = Duration::from_secs(1);

/// Maximum number of concurrent panel update tasks
/// This prevents spawning too many tasks at once if many panels are due
const MAX_CONCURRENT_UPDATES: usize = 16;

/// Circuit breaker: number of consecutive failures before opening
const CIRCUIT_BREAKER_THRESHOLD: u32 = 3;

/// Circuit breaker: initial retry interval when circuit opens
const CIRCUIT_BREAKER_BASE_RETRY: Duration = Duration::from_secs(10);

/// Circuit breaker: maximum retry interval (caps exponential backoff)
const CIRCUIT_BREAKER_MAX_RETRY: Duration = Duration::from_secs(300); // 5 minutes

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

/// Circuit breaker for handling repeated source failures
///
/// Prevents wasting resources on sources that consistently fail (e.g., disconnected GPU,
/// unavailable sensor). After `failure_threshold` consecutive failures, the circuit opens
/// and retries are delayed with exponential backoff.
struct CircuitBreaker {
    /// Number of consecutive failures
    consecutive_failures: u32,
    /// Time when circuit was opened (None = circuit closed)
    opened_at: Option<Instant>,
    /// Current retry interval (increases with backoff)
    current_retry_interval: Duration,
}

impl CircuitBreaker {
    fn new() -> Self {
        Self {
            consecutive_failures: 0,
            opened_at: None,
            current_retry_interval: CIRCUIT_BREAKER_BASE_RETRY,
        }
    }

    /// Check if circuit is open and should skip update
    fn should_skip(&self, now: Instant) -> bool {
        if let Some(opened_at) = self.opened_at {
            // Circuit is open - check if retry time has passed
            now.duration_since(opened_at) < self.current_retry_interval
        } else {
            false
        }
    }

    /// Record a successful update - closes the circuit
    fn record_success(&mut self) {
        if self.opened_at.is_some() {
            debug!("Circuit breaker closed after successful update");
        }
        self.consecutive_failures = 0;
        self.opened_at = None;
        self.current_retry_interval = CIRCUIT_BREAKER_BASE_RETRY;
    }

    /// Record a failed update - may open the circuit
    fn record_failure(&mut self, now: Instant, source_name: &str) {
        self.consecutive_failures += 1;

        if self.consecutive_failures >= CIRCUIT_BREAKER_THRESHOLD {
            if self.opened_at.is_some() {
                // Already open - apply exponential backoff
                self.current_retry_interval =
                    (self.current_retry_interval * 2).min(CIRCUIT_BREAKER_MAX_RETRY);
                debug!(
                    "Circuit breaker for {} still open, backoff increased to {:?}",
                    source_name, self.current_retry_interval
                );
            } else {
                // First time opening
                info!(
                    "Circuit breaker opened for {} after {} consecutive failures, retry in {:?}",
                    source_name, self.consecutive_failures, self.current_retry_interval
                );
            }
            self.opened_at = Some(now);
        }
    }

    /// Check if circuit is in half-open state (ready to retry after timeout)
    fn is_half_open(&self, now: Instant) -> bool {
        if let Some(opened_at) = self.opened_at {
            now.duration_since(opened_at) >= self.current_retry_interval
        } else {
            false
        }
    }
}

/// Tracks update timing for a shared source
struct SharedSourceUpdateState {
    last_update: Instant,
    interval: Duration,
    /// Circuit breaker for handling repeated failures
    circuit_breaker: CircuitBreaker,
}

/// Compute a hash from PanelData's source config (preferred method)
///
/// OPTIMIZATION: Only hash fields that affect update scheduling:
/// - source_type: determines which source to poll
/// - update_interval_ms: determines polling frequency
///
/// Other config fields (colors, field selections, etc.) are applied directly
/// when config is saved and don't need to be detected by the update manager.
/// This avoids expensive serde serialization every 500ms.
fn compute_config_hash_from_data(source_config: &SourceConfig) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();

    // Hash only the fields that affect scheduling (fast - no serde)
    source_config.source_type().hash(&mut hasher);
    source_config.update_interval_ms().hash(&mut hasher);

    hasher.finish()
}

/// Compute a simple hash of the config keys that affect update interval (legacy fallback)
///
/// OPTIMIZATION: Only extract and hash update_interval_ms from config structs.
/// Avoids full JSON serialization. Iterates through actual config keys instead of
/// searching for each possible key.
fn compute_config_hash(config: &HashMap<String, serde_json::Value>) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();

    // Iterate through actual keys in config (typically just 1-2 keys)
    for (key, value) in config.iter() {
        if key.ends_with("_config") {
            key.hash(&mut hasher);
            // Only hash the update_interval_ms field, not the entire config
            if let Some(interval) = value.get("update_interval_ms").and_then(|v| v.as_u64()) {
                interval.hash(&mut hasher);
            }
        }
    }

    hasher.finish()
}

/// Extract update interval from panel config
///
/// Optimized to directly extract `update_interval_ms` field from JSON without
/// deserializing the entire config struct. Iterates through actual config keys
/// instead of searching for each possible key.
fn extract_update_interval(
    config: &HashMap<String, serde_json::Value>,
    _panel_id: &str,
) -> Duration {
    // Iterate through actual keys in config (typically just 1-2 keys)
    for (key, config_value) in config.iter() {
        if key.ends_with("_config") {
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
    /// Semaphore to limit concurrent panel update tasks
    update_semaphore: Arc<tokio::sync::Semaphore>,
    /// Flag to signal graceful shutdown
    should_stop: Arc<std::sync::atomic::AtomicBool>,
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
            update_semaphore: Arc::new(tokio::sync::Semaphore::new(MAX_CONCURRENT_UPDATES)),
            should_stop: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Signal the update manager to stop
    pub fn stop(&self) {
        self.should_stop
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }

    /// Check if the update manager has been signaled to stop
    pub fn is_stopped(&self) -> bool {
        self.should_stop.load(std::sync::atomic::Ordering::Relaxed)
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
        if let Err(e) = self
            .sender
            .try_send(UpdateManagerMessage::RemovePanel(panel_id))
        {
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
                (
                    panel_guard.id.clone(),
                    hash,
                    interval,
                    panel_guard.source_key.clone(),
                )
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
                        circuit_breaker: CircuitBreaker::new(),
                    },
                );
                debug!(
                    "Registered shared source {} with interval {:?}",
                    key, interval
                );
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
    /// This runs until stop() is called, updating each panel at its configured interval.
    pub async fn run(&self, base_interval: Duration) {
        let mut interval = tokio::time::interval(base_interval);

        loop {
            // Check if we should stop before waiting for the next tick
            if self.should_stop.load(std::sync::atomic::Ordering::Relaxed) {
                info!("Update manager stopping gracefully");
                break;
            }

            interval.tick().await;

            // Check again after waking up
            if self.should_stop.load(std::sync::atomic::Ordering::Relaxed) {
                info!("Update manager stopping gracefully");
                break;
            }

            // Process any pending add/remove messages first
            self.process_messages().await;

            let start = Instant::now();
            if let Err(e) = self.update_all().await {
                error!("Error updating panels: {}", e);
            }

            let elapsed = start.elapsed();
            trace!("Update cycle took {:?}", elapsed);

            // Periodic diagnostic logging (every ~60 seconds)
            static CYCLE_COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let cycle = CYCLE_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if cycle > 0 && cycle % 60 == 0 {
                let panel_count = self.panels.read().await.len();
                let shared_source_count = self.shared_sources.read().await.len();
                log::info!(
                    "UpdateManager diagnostics [cycle {}]: {} panels, {} shared sources, last cycle took {:?}",
                    cycle,
                    panel_count,
                    shared_source_count,
                    elapsed
                );
                // Warn if update cycle is taking too long
                if elapsed > std::time::Duration::from_millis(500) {
                    log::warn!(
                        "Update cycle took {:?} - this may cause UI lag",
                        elapsed
                    );
                }
            }
        }

        info!("Update manager stopped");
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
        let panel_count = panels.len();
        // Use Arc<str> for tasks to avoid cloning String for each task spawn
        let mut tasks: Vec<(Arc<str>, tokio::task::JoinHandle<()>)> =
            Vec::with_capacity(panel_count);
        // Use HashMap for O(1) lookup instead of Vec with O(n) linear search
        let mut config_updates: HashMap<String, (u64, Duration, Option<String>)> = HashMap::new();

        // Track panels that had their config checked (for updating last_config_check)
        // Use Arc<str> to share panel_id with config_updates without extra clones
        let mut config_checked: Vec<Arc<str>> = Vec::with_capacity(panel_count);

        for (panel_id, state) in panels.iter() {
            // Only check config hash if enough time has elapsed (throttle to avoid CPU waste)
            let should_check_config =
                now.duration_since(state.last_config_check) >= CONFIG_CHECK_INTERVAL;

            // Create Arc<str> lazily when first needed (avoids allocation if not needed)
            let mut panel_id_arc: Option<Arc<str>> = None;
            let get_arc = |arc: &mut Option<Arc<str>>| -> Arc<str> {
                arc.get_or_insert_with(|| Arc::from(panel_id.as_str()))
                    .clone()
            };

            let (current_hash, new_interval, new_source_key) = if should_check_config {
                config_checked.push(get_arc(&mut panel_id_arc));
                if let Ok(panel_guard) = state.panel.try_read() {
                    if let Some(ref data) = panel_guard.data {
                        let hash = compute_config_hash_from_data(&data.source_config);
                        let interval =
                            Duration::from_millis(data.source_config.update_interval_ms());
                        (hash, Some(interval), panel_guard.source_key.clone())
                    } else {
                        // Legacy config - still need to track source_key for shared source updates
                        let hash = compute_config_hash(&panel_guard.config);
                        (hash, None, panel_guard.source_key.clone())
                    }
                } else {
                    (state.config_hash, None, state.source_key.clone())
                }
            } else {
                // Skip hash computation, use cached values
                (state.config_hash, None, state.source_key.clone())
            };

            // Check if config hash changed OR source_key changed
            // Source_key can change without hash changing (e.g., combo source slot config changes)
            let source_key_changed = new_source_key != state.source_key;
            if current_hash != state.config_hash || source_key_changed {
                let interval = new_interval.unwrap_or_else(|| {
                    if let Ok(panel_guard) = state.panel.try_read() {
                        extract_update_interval(&panel_guard.config, panel_id)
                    } else {
                        state.cached_interval
                    }
                });
                config_updates.insert(
                    panel_id.clone(),
                    (current_hash, interval, new_source_key.clone()),
                );
                if source_key_changed {
                    debug!(
                        "Panel {} source_key changed from {:?} to {:?}",
                        panel_id, state.source_key, new_source_key
                    );
                }
            }

            // Check if update is due - O(1) HashMap lookup instead of O(n) Vec search
            let effective_interval = config_updates
                .get(panel_id)
                .map(|(_, interval, _)| *interval)
                .unwrap_or(state.cached_interval);

            let elapsed = now.duration_since(state.last_update);
            if elapsed >= effective_interval {
                let panel = state.panel.clone();
                let source_key = state.source_key.clone();

                // Check if this panel's source was updated in phase 1
                // (currently unused but may be useful for future optimizations)
                let _source_was_updated = source_key
                    .as_ref()
                    .map(|k| updated_sources.contains(k))
                    .unwrap_or(false);

                // Clone semaphore for the task - limits concurrent updates
                let semaphore = self.update_semaphore.clone();

                // Get Arc<str> for panel_id (reuses existing Arc if already created, or creates one)
                // Arc clone is cheap (just atomic increment) compared to String clone
                let panel_id_arc_for_tasks = get_arc(&mut panel_id_arc);
                let panel_id_for_task = Arc::clone(&panel_id_arc_for_tasks);
                let task = tokio::spawn(async move {
                    // Acquire permit to limit concurrent updates (automatically released on drop)
                    let _permit = semaphore.acquire().await;

                    let mut panel_guard = panel.write().await;

                    // If using shared source and it was updated, panel.update() will use cached values
                    // If not using shared source, panel.update() will poll directly
                    if let Err(e) = panel_guard.update() {
                        error!("Error updating panel {}: {}", panel_id_for_task, e);
                    }
                });
                tasks.push((panel_id_arc_for_tasks, task));
            }
        }

        // Update cached state for panels with config changes
        // Also track old source keys for cleanup
        let mut old_source_keys: Vec<String> = Vec::new();
        for (panel_id, (new_hash, new_interval, new_source_key)) in config_updates {
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
                                    circuit_breaker: CircuitBreaker::new(),
                                },
                            );
                            debug!(
                                "Registered new shared source {} for panel {} with interval {:?}",
                                new_key, panel_id, interval
                            );
                        }
                    }
                }

                state.config_hash = new_hash;
                state.cached_interval = new_interval;
                state.source_key = new_source_key;
                info!(
                    "Updated cached interval for panel {}: {:?}",
                    panel_id, state.cached_interval
                );
            }
        }

        // Update last_config_check for all panels that had their config checked
        for panel_id in config_checked {
            // Use &*panel_id to deref Arc<str> to &str for HashMap lookup
            if let Some(state) = panels.get_mut(&*panel_id) {
                state.last_config_check = now;
            }
        }

        // Clean up old shared sources that are no longer referenced by any panel
        // Use linear search instead of HashSet - old_source_keys is typically 0-1 items
        if !old_source_keys.is_empty() {
            let mut shared_sources = self.shared_sources.write().await;
            for old_key in old_source_keys {
                let is_still_active = panels
                    .values()
                    .any(|s| s.source_key.as_ref() == Some(&old_key));
                if !is_still_active {
                    shared_sources.remove(&old_key);
                    debug!(
                        "Removed unused shared source {} from update manager",
                        old_key
                    );
                }
            }
        }

        // Update last_update times
        for (panel_id, _) in &tasks {
            // Use &**panel_id to deref &Arc<str> to &str for HashMap lookup
            if let Some(state) = panels.get_mut(&**panel_id) {
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
    /// Returns a set of source keys that were updated.
    /// Uses circuit breaker pattern to avoid hammering failing sources.
    async fn update_shared_sources(&self, now: Instant) -> std::collections::HashSet<String> {
        let mut updated = std::collections::HashSet::new();

        let manager = match global_shared_source_manager() {
            Some(m) => m,
            None => return updated,
        };

        // Phase 1: Collect sources that need updating (read lock only)
        // Skip sources with open circuit breakers (unless ready to retry)
        let sources_to_update: Vec<(String, Duration, bool)> = {
            let shared_sources = self.shared_sources.read().await;
            shared_sources
                .iter()
                .filter(|(key, state)| {
                    // Check if update is due based on interval
                    let interval_due = now.duration_since(state.last_update) >= state.interval;
                    if !interval_due {
                        return false;
                    }

                    // Check circuit breaker state
                    if state.circuit_breaker.should_skip(now) {
                        trace!("Skipping source {} - circuit breaker open", key);
                        return false;
                    }

                    true
                })
                .map(|(key, state)| {
                    let is_half_open = state.circuit_breaker.is_half_open(now);
                    (key.clone(), state.interval, is_half_open)
                })
                .collect()
        };
        // Lock released here before I/O

        // Phase 2: Perform I/O WITHOUT holding the lock
        // This prevents blocking readers during hardware polling
        let mut update_results: Vec<(String, Result<(), String>, Option<Duration>, bool)> =
            Vec::with_capacity(sources_to_update.len());

        for (key, _interval, is_half_open) in sources_to_update {
            if is_half_open {
                trace!("Retrying source {} (circuit breaker half-open)", key);
            }

            let result = manager.update_source(&key);
            let new_interval = manager.get_interval(&key);

            match &result {
                Ok(_) => {
                    trace!("Updated shared source {}", key);
                    updated.insert(key.clone());
                    update_results.push((key, Ok(()), new_interval, is_half_open));
                }
                Err(e) => {
                    let err_str = e.to_string();
                    if err_str.contains("Source not found") {
                        debug!(
                            "Shared source {} no longer exists, removing from update tracking",
                            key
                        );
                    } else if !is_half_open {
                        // Only log errors for non-retry attempts to reduce log spam
                        error!("Error updating shared source {}: {}", key, e);
                    } else {
                        debug!("Retry failed for shared source {}: {}", key, e);
                    }
                    update_results.push((key, Err(err_str), None, is_half_open));
                }
            }
        }

        // Phase 3: Update state with results (write lock)
        {
            let mut shared_sources = self.shared_sources.write().await;

            for (key, result, new_interval, _is_half_open) in update_results {
                match result {
                    Ok(()) => {
                        if let Some(state) = shared_sources.get_mut(&key) {
                            state.last_update = now;
                            state.circuit_breaker.record_success();

                            // Also refresh interval from manager in case it changed
                            if let Some(new_int) = new_interval {
                                if new_int != state.interval {
                                    debug!(
                                        "Shared source {} interval changed from {:?} to {:?}",
                                        key, state.interval, new_int
                                    );
                                    state.interval = new_int;
                                }
                            }
                        }
                    }
                    Err(ref err_str) if err_str.contains("Source not found") => {
                        shared_sources.remove(&key);
                    }
                    Err(_) => {
                        // Record failure in circuit breaker
                        if let Some(state) = shared_sources.get_mut(&key) {
                            state.circuit_breaker.record_failure(now, &key);
                        }
                    }
                }
            }
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
