//! Shared temperature sensor components cache
//!
//! This module provides a single global instance of sysinfo::Components that is
//! initialized once at startup and shared by all sources that need temperature data.
//! This avoids the expensive sensor discovery happening every time a source is created.

use once_cell::sync::Lazy;
use std::sync::Mutex;
use sysinfo::Components;

/// Global shared components instance
///
/// The Components are wrapped in a Mutex to allow thread-safe refresh operations.
/// The Lazy ensures one-time initialization on first access.
static SHARED_COMPONENTS: Lazy<Mutex<Components>> = Lazy::new(|| {
    log::info!("=== Initializing shared temperature sensors (one-time) ===");
    let components = Components::new_with_refreshed_list();
    log::info!("Shared temperature sensors initialized: {} components", components.len());
    Mutex::new(components)
});

/// Refresh the shared components and get current temperature readings
///
/// This function refreshes the global components and returns the current
/// temperatures as a Vec of (label, temperature_celsius) pairs.
pub fn get_refreshed_temperatures() -> Vec<(String, f32)> {
    if let Ok(mut components) = SHARED_COMPONENTS.lock() {
        components.refresh();
        components
            .iter()
            .map(|c| (c.label().to_string(), c.temperature()))
            .collect()
    } else {
        log::error!("Failed to lock shared components for temperature reading");
        Vec::new()
    }
}

/// Get temperature for a specific sensor label (after refreshing)
pub fn get_temperature_by_label(label: &str) -> Option<f32> {
    if let Ok(mut components) = SHARED_COMPONENTS.lock() {
        components.refresh();
        for component in components.iter() {
            if component.label() == label {
                return Some(component.temperature());
            }
        }
    }
    None
}

/// Get temperature by index (after refreshing)
pub fn get_temperature_by_index(index: usize) -> Option<f32> {
    if let Ok(mut components) = SHARED_COMPONENTS.lock() {
        components.refresh();
        components.get(index).map(|c| c.temperature())
    } else {
        None
    }
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
    if let Ok(components) = SHARED_COMPONENTS.lock() {
        components.len()
    } else {
        0
    }
}
