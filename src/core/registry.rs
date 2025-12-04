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
        self.sources.write().unwrap().insert(id.to_string(), factory);
    }

    /// Register a displayer
    pub fn register_displayer(&self, id: &str, factory: DisplayerFactory) {
        self.displayers.write().unwrap().insert(id.to_string(), factory);
    }

    /// Create a data source by ID
    pub fn create_source(&self, id: &str) -> Result<BoxedDataSource> {
        let factory = *self
            .sources
            .read()
            .unwrap()
            .get(id)
            .ok_or_else(|| anyhow!("Unknown source: {}", id))?;
        Ok(factory())
    }

    /// Create a displayer by ID
    pub fn create_displayer(&self, id: &str) -> Result<BoxedDisplayer> {
        let factory = *self
            .displayers
            .read()
            .unwrap()
            .get(id)
            .ok_or_else(|| anyhow!("Unknown displayer: {}", id))?;
        Ok(factory())
    }

    /// List all registered source IDs
    pub fn list_sources(&self) -> Vec<String> {
        self.sources.read().unwrap().keys().cloned().collect()
    }

    /// List all registered displayer IDs
    pub fn list_displayers(&self) -> Vec<String> {
        self.displayers.read().unwrap().keys().cloned().collect()
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
