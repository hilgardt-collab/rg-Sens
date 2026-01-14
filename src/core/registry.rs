//! Registry for data sources and displayers

use super::{BoxedDataSource, BoxedDisplayer};
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

/// Function that creates a data source
pub type SourceFactory = fn() -> BoxedDataSource;

/// Function that creates a displayer
pub type DisplayerFactory = fn() -> BoxedDisplayer;

/// Information about a registered source
#[derive(Clone)]
pub struct SourceInfo {
    pub id: String,
    pub display_name: String,
    pub compatible_displayers: Vec<String>,
    pub factory: SourceFactory,
}

/// Information about a registered displayer
#[derive(Clone)]
pub struct DisplayerInfo {
    pub id: String,
    pub display_name: String,
    pub factory: DisplayerFactory,
}

/// Registry for data sources and displayers
///
/// This allows for compile-time registration of built-in sources/displayers
/// and runtime registration of plugin-provided ones.
pub struct Registry {
    sources: RwLock<HashMap<String, SourceFactory>>,
    displayers: RwLock<HashMap<String, DisplayerFactory>>,
    source_info: RwLock<HashMap<String, SourceInfo>>,
    displayer_info: RwLock<HashMap<String, DisplayerInfo>>,
}

impl Registry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            sources: RwLock::new(HashMap::new()),
            displayers: RwLock::new(HashMap::new()),
            source_info: RwLock::new(HashMap::new()),
            displayer_info: RwLock::new(HashMap::new()),
        }
    }

    /// Register a data source (legacy API, kept for compatibility)
    pub fn register_source(&self, id: &str, factory: SourceFactory) {
        // Use capitalized ID as default display name
        let display_name = capitalize_id(id);
        self.register_source_with_info(id, &display_name, &[], factory);
    }

    /// Register a data source with full info
    pub fn register_source_with_info(
        &self,
        id: &str,
        display_name: &str,
        compatible_displayers: &[&str],
        factory: SourceFactory,
    ) {
        match self.sources.write() {
            Ok(mut sources) => {
                sources.insert(id.to_string(), factory);
            }
            Err(e) => {
                log::error!("Failed to register source '{}': lock poisoned: {}", id, e);
                return;
            }
        }
        match self.source_info.write() {
            Ok(mut info) => {
                info.insert(
                    id.to_string(),
                    SourceInfo {
                        id: id.to_string(),
                        display_name: display_name.to_string(),
                        compatible_displayers: compatible_displayers
                            .iter()
                            .map(|s| s.to_string())
                            .collect(),
                        factory,
                    },
                );
            }
            Err(e) => {
                log::error!("Failed to store source info '{}': lock poisoned: {}", id, e);
            }
        }
    }

    /// Register a displayer (legacy API, kept for compatibility)
    pub fn register_displayer(&self, id: &str, factory: DisplayerFactory) {
        // Use capitalized ID as default display name
        let display_name = capitalize_id(id);
        self.register_displayer_with_info(id, &display_name, factory);
    }

    /// Register a displayer with full info
    pub fn register_displayer_with_info(
        &self,
        id: &str,
        display_name: &str,
        factory: DisplayerFactory,
    ) {
        match self.displayers.write() {
            Ok(mut displayers) => {
                displayers.insert(id.to_string(), factory);
            }
            Err(e) => {
                log::error!(
                    "Failed to register displayer '{}': lock poisoned: {}",
                    id,
                    e
                );
                return;
            }
        }
        match self.displayer_info.write() {
            Ok(mut info) => {
                info.insert(
                    id.to_string(),
                    DisplayerInfo {
                        id: id.to_string(),
                        display_name: display_name.to_string(),
                        factory,
                    },
                );
            }
            Err(e) => {
                log::error!(
                    "Failed to store displayer info '{}': lock poisoned: {}",
                    id,
                    e
                );
            }
        }
    }

    /// Create a data source by ID
    pub fn create_source(&self, id: &str) -> Result<BoxedDataSource> {
        let sources = self
            .sources
            .read()
            .map_err(|e| anyhow!("Registry lock poisoned: {}", e))?;
        let factory = *sources
            .get(id)
            .ok_or_else(|| anyhow!("Unknown source: {}", id))?;
        Ok(factory())
    }

    /// Create a displayer by ID
    pub fn create_displayer(&self, id: &str) -> Result<BoxedDisplayer> {
        let displayers = self
            .displayers
            .read()
            .map_err(|e| anyhow!("Registry lock poisoned: {}", e))?;
        let factory = *displayers
            .get(id)
            .ok_or_else(|| anyhow!("Unknown displayer: {}", id))?;
        Ok(factory())
    }

    /// List all registered source IDs
    pub fn list_sources(&self) -> Vec<String> {
        match self.sources.read() {
            Ok(sources) => sources.keys().cloned().collect(),
            Err(e) => {
                log::error!("Failed to list sources: lock poisoned: {}", e);
                Vec::new()
            }
        }
    }

    /// List all registered displayer IDs
    pub fn list_displayers(&self) -> Vec<String> {
        match self.displayers.read() {
            Ok(displayers) => displayers.keys().cloned().collect(),
            Err(e) => {
                log::error!("Failed to list displayers: lock poisoned: {}", e);
                Vec::new()
            }
        }
    }

    /// Get source info by ID
    pub fn get_source_info(&self, id: &str) -> Option<SourceInfo> {
        match self.source_info.read() {
            Ok(info) => info.get(id).cloned(),
            Err(e) => {
                log::error!("Failed to get source info: lock poisoned: {}", e);
                None
            }
        }
    }

    /// Get displayer info by ID
    pub fn get_displayer_info(&self, id: &str) -> Option<DisplayerInfo> {
        match self.displayer_info.read() {
            Ok(info) => info.get(id).cloned(),
            Err(e) => {
                log::error!("Failed to get displayer info: lock poisoned: {}", e);
                None
            }
        }
    }

    /// List all sources with their info, sorted by display name
    pub fn list_sources_with_info(&self) -> Vec<SourceInfo> {
        match self.source_info.read() {
            Ok(info) => {
                let mut sources = Vec::with_capacity(info.len());
                sources.extend(info.values().cloned());
                sources.sort_by(|a, b| a.display_name.cmp(&b.display_name));
                sources
            }
            Err(e) => {
                log::error!("Failed to list sources with info: lock poisoned: {}", e);
                Vec::new()
            }
        }
    }

    /// List all displayers with their info, sorted by display name
    pub fn list_displayers_with_info(&self) -> Vec<DisplayerInfo> {
        match self.displayer_info.read() {
            Ok(info) => {
                let mut displayers = Vec::with_capacity(info.len());
                displayers.extend(info.values().cloned());
                displayers.sort_by(|a, b| a.display_name.cmp(&b.display_name));
                displayers
            }
            Err(e) => {
                log::error!("Failed to list displayers with info: lock poisoned: {}", e);
                Vec::new()
            }
        }
    }

    /// Get compatible displayers for a source
    pub fn get_compatible_displayers(&self, source_id: &str) -> Vec<DisplayerInfo> {
        let source_info = match self.get_source_info(source_id) {
            Some(info) => info,
            None => return self.list_displayers_with_info(), // fallback to all (already sorted)
        };

        // If no compatible displayers specified, return all (except special ones)
        if source_info.compatible_displayers.is_empty() {
            return self
                .list_displayers_with_info()
                .into_iter()
                .filter(|d| !["clock_analog", "clock_digital", "lcars"].contains(&d.id.as_str()))
                .collect();
        }

        // Return only compatible displayers, sorted alphabetically by display name
        let mut displayers: Vec<DisplayerInfo> = source_info
            .compatible_displayers
            .iter()
            .filter_map(|id| self.get_displayer_info(id))
            .collect();
        displayers.sort_by(|a, b| a.display_name.cmp(&b.display_name));
        displayers
    }
}

/// Helper to capitalize an ID like "cpu" -> "Cpu", "clock_analog" -> "Clock Analog"
fn capitalize_id(id: &str) -> String {
    // Convert snake_case to Title Case (first letter of each word capitalized)
    id.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().chain(chars).collect(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

/// Global registry instance
///
/// In the future, this might be replaced with a more sophisticated
/// plugin system, but for now we use a static registry.
static GLOBAL_REGISTRY: OnceLock<Registry> = OnceLock::new();

/// Get the global registry
pub fn global_registry() -> &'static Registry {
    GLOBAL_REGISTRY.get_or_init(Registry::new)
}

/// Macro to register a data source
#[macro_export]
macro_rules! register_source {
    ($id:expr, $type:ty) => {
        $crate::core::global_registry().register_source($id, || Box::new(<$type>::default()));
    };
}

/// Macro to register a displayer
#[macro_export]
macro_rules! register_displayer {
    ($id:expr, $type:ty) => {
        $crate::core::global_registry().register_displayer($id, || Box::new(<$type>::default()));
    };
}
