//! Update manager for scheduling and coordinating source updates

use super::Panel;
use anyhow::Result;
use log::{error, trace};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::Instant;

/// Manages periodic updates for panels
pub struct UpdateManager {
    panels: Arc<RwLock<Vec<Arc<RwLock<Panel>>>>>,
}

impl UpdateManager {
    /// Create a new update manager
    pub fn new() -> Self {
        Self {
            panels: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Add a panel to be updated
    pub async fn add_panel(&self, panel: Arc<RwLock<Panel>>) {
        let mut panels = self.panels.write().await;
        panels.push(panel);
    }

    /// Start the update loop
    ///
    /// This runs indefinitely, updating all panels at their specified intervals.
    pub async fn run(&self, update_interval: Duration) {
        let mut interval = tokio::time::interval(update_interval);

        loop {
            interval.tick().await;

            let start = Instant::now();
            if let Err(e) = self.update_all().await {
                error!("Error updating panels: {}", e);
            }

            let elapsed = start.elapsed();
            trace!("Update cycle took {:?}", elapsed);

            // Warn if updates are taking too long
            if elapsed > update_interval {
                log::warn!(
                    "Update cycle took {:?}, which exceeds interval {:?}",
                    elapsed,
                    update_interval
                );
            }
        }
    }

    /// Update all panels
    async fn update_all(&self) -> Result<()> {
        let panels = self.panels.read().await;

        // Update panels in parallel
        let mut tasks = Vec::new();

        for panel in panels.iter() {
            let panel = panel.clone();
            let task = tokio::spawn(async move {
                let mut panel = panel.write().await;
                if let Err(e) = panel.update() {
                    error!("Error updating panel {}: {}", panel.id, e);
                }
            });
            tasks.push(task);
        }

        // Wait for all updates to complete
        for task in tasks {
            task.await?;
        }

        Ok(())
    }
}

impl Default for UpdateManager {
    fn default() -> Self {
        Self::new()
    }
}
