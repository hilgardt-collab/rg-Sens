//! Plugin ABI definitions
//!
//! This module defines the ABI for dynamic plugins.
//! Currently a placeholder for future implementation.

use serde::{Deserialize, Serialize};

/// Metadata about a plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    /// Plugin name
    pub name: String,
    /// Plugin version
    pub version: String,
    /// Plugin author
    pub author: String,
    /// Plugin description
    pub description: String,
}

/// Plugin API trait
///
/// Future plugins will implement this trait and expose it via C ABI.
pub trait PluginApi {
    /// Get plugin metadata
    fn metadata(&self) -> PluginMetadata;

    /// Initialize the plugin
    fn initialize(&mut self) -> Result<(), String>;

    /// Shutdown the plugin
    fn shutdown(&mut self);
}

// Future: C ABI functions for dynamic loading
// #[no_mangle]
// pub extern "C" fn plugin_create() -> *mut c_void { ... }
