//! Shared temperature sensor components cache
//!
//! This module provides a single global instance of sysinfo::Components that is
//! initialized once at startup and shared by all sources that need temperature data.
//! This avoids the expensive sensor discovery happening every time a source is created.

use once_cell::sync::Lazy;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use sysinfo::Components;

/// Minimum interval between sensor refreshes (250ms)
/// This prevents redundant refreshes when multiple sources read temperatures
const MIN_REFRESH_INTERVAL: Duration = Duration::from_millis(250);

/// Shared components with refresh timestamp
struct SharedSensors {
    components: Components,
    last_refresh: Instant,
}

impl SharedSensors {
    fn new() -> Self {
        Self {
            components: Components::new_with_refreshed_list(),
            last_refresh: Instant::now(),
        }
    }

    /// Refresh only if enough time has passed since last refresh
    fn refresh_if_needed(&mut self) {
        if self.last_refresh.elapsed() >= MIN_REFRESH_INTERVAL {
            self.components.refresh();
            self.last_refresh = Instant::now();
        }
    }
}

/// Global shared components instance
///
/// The Components are wrapped in a Mutex to allow thread-safe refresh operations.
/// The Lazy ensures one-time initialization on first access.
static SHARED_COMPONENTS: Lazy<Mutex<SharedSensors>> = Lazy::new(|| {
    log::warn!("=== Initializing shared temperature sensors (one-time) ===");
    let sensors = SharedSensors::new();
    log::info!("Shared temperature sensors initialized: {} components", sensors.components.len());
    Mutex::new(sensors)
});

/// Refresh the shared components and get current temperature readings
///
/// This function refreshes the global components (if needed) and returns the current
/// temperatures as a Vec of (label, temperature_celsius) pairs.
/// Uses cached refresh to avoid redundant sensor polling.
pub fn get_refreshed_temperatures() -> Vec<(String, f32)> {
    // Use unwrap_or_else to recover from poisoned mutex - data may still be valid
    let mut sensors = SHARED_COMPONENTS.lock().unwrap_or_else(|poisoned| {
        log::warn!("Shared sensors mutex was poisoned, recovering");
        poisoned.into_inner()
    });
    sensors.refresh_if_needed();
    sensors.components
        .iter()
        .map(|c| (c.label().to_string(), c.temperature()))
        .collect()
}

/// Get temperature for a specific sensor label (refreshes if needed)
pub fn get_temperature_by_label(label: &str) -> Option<f32> {
    // Use unwrap_or_else to recover from poisoned mutex - data may still be valid
    let mut sensors = SHARED_COMPONENTS.lock().unwrap_or_else(|poisoned| {
        log::warn!("Shared sensors mutex was poisoned, recovering");
        poisoned.into_inner()
    });
    sensors.refresh_if_needed();
    sensors.components
        .iter()
        .find(|c| c.label() == label)
        .map(|c| c.temperature())
}

/// Get temperature by index (refreshes if needed)
pub fn get_temperature_by_index(index: usize) -> Option<f32> {
    // Use unwrap_or_else to recover from poisoned mutex - data may still be valid
    let mut sensors = SHARED_COMPONENTS.lock().unwrap_or_else(|poisoned| {
        log::warn!("Shared sensors mutex was poisoned, recovering");
        poisoned.into_inner()
    });
    sensors.refresh_if_needed();
    sensors.components.get(index).map(|c| c.temperature())
}

/// Force initialization of the shared components
///
/// Call this at application startup to ensure sensor discovery happens
/// before any config dialogs are opened.
pub fn initialize() {
    // Accessing the lazy static triggers initialization
    let _ = &*SHARED_COMPONENTS;
}

/// Get the total number of temperature components
#[allow(dead_code)]
pub fn component_count() -> usize {
    // Use unwrap_or_else to recover from poisoned mutex - data may still be valid
    let sensors = SHARED_COMPONENTS.lock().unwrap_or_else(|poisoned| {
        log::warn!("Shared sensors mutex was poisoned, recovering");
        poisoned.into_inner()
    });
    sensors.components.len()
}
