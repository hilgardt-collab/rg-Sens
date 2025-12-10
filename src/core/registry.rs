//! Registry for data sources and displayers

use super::{BoxedDataSource, BoxedDisplayer};
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

/// Function that creates a data source
pub type SourceFactory = fn() -> BoxedDataSource;

/// Function that creates a displayer
pub type DisplayerFactory = fn() -> BoxedDisplayer;

/// Registry for data sources and displayers
///
/// This allows for compile-time registration of built-in sources/displayers
/// and runtime registration of plugin-provided ones.
pub struct Registry {
    sources: RwLock<HashMap<String, SourceFactory>>,
    displayers: RwLock<HashMap<String, DisplayerFactory>>,
}

impl Registry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            sources: RwLock::new(HashMap::new()),
            displayers: RwLock::new(HashMap::new()),
        }
    }

    /// Register a data source
    pub fn register_source(&self, id: &str, factory: SourceFactory) {
        match self.sources.write() {
            Ok(mut sources) => {
                sources.insert(id.to_string(), factory);
            }
            Err(e) => {
                log::error!("Failed to register source '{}': lock poisoned: {}", id, e);
            }
        }
    }

    /// Register a displayer
    pub fn register_displayer(&self, id: &str, factory: DisplayerFactory) {
        match self.displayers.write() {
            Ok(mut displayers) => {
                displayers.insert(id.to_string(), factory);
            }
            Err(e) => {
                log::error!("Failed to register displayer '{}': lock poisoned: {}", id, e);
            }
        }
    }

    /// Create a data source by ID
    pub fn create_source(&self, id: &str) -> Result<BoxedDataSource> {
        let sources = self.sources.read()
            .map_err(|e| anyhow!("Registry lock poisoned: {}", e))?;
        let factory = *sources
            .get(id)
            .ok_or_else(|| anyhow!("Unknown source: {}", id))?;
        Ok(factory())
    }

    /// Create a displayer by ID
    pub fn create_displayer(&self, id: &str) -> Result<BoxedDisplayer> {
        let displayers = self.displayers.read()
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
        $crate::core::global_registry().register_source($id, || {
            Box::new(<$type>::default())
        });
    };
}

/// Macro to register a displayer
#[macro_export]
macro_rules! register_displayer {
    ($id:expr, $type:ty) => {
        $crate::core::global_registry().register_displayer($id, || {
            Box::new(<$type>::default())
        });
    };
}
