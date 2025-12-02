//! Registry for data sources and displayers

use super::{BoxedDataSource, BoxedDisplayer};
use anyhow::{anyhow, Result};
use std::collections::HashMap;

/// Function that creates a data source
pub type SourceFactory = fn() -> BoxedDataSource;

/// Function that creates a displayer
pub type DisplayerFactory = fn() -> BoxedDisplayer;

/// Registry for data sources and displayers
///
/// This allows for compile-time registration of built-in sources/displayers
/// and runtime registration of plugin-provided ones.
pub struct Registry {
    sources: HashMap<String, SourceFactory>,
    displayers: HashMap<String, DisplayerFactory>,
}

impl Registry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            sources: HashMap::new(),
            displayers: HashMap::new(),
        }
    }

    /// Register a data source
    pub fn register_source(&mut self, id: &str, factory: SourceFactory) {
        self.sources.insert(id.to_string(), factory);
    }

    /// Register a displayer
    pub fn register_displayer(&mut self, id: &str, factory: DisplayerFactory) {
        self.displayers.insert(id.to_string(), factory);
    }

    /// Create a data source by ID
    pub fn create_source(&self, id: &str) -> Result<BoxedDataSource> {
        let factory = self
            .sources
            .get(id)
            .ok_or_else(|| anyhow!("Unknown source: {}", id))?;
        Ok(factory())
    }

    /// Create a displayer by ID
    pub fn create_displayer(&self, id: &str) -> Result<BoxedDisplayer> {
        let factory = self
            .displayers
            .get(id)
            .ok_or_else(|| anyhow!("Unknown displayer: {}", id))?;
        Ok(factory())
    }

    /// List all registered source IDs
    pub fn list_sources(&self) -> Vec<String> {
        self.sources.keys().cloned().collect()
    }

    /// List all registered displayer IDs
    pub fn list_displayers(&self) -> Vec<String> {
        self.displayers.keys().cloned().collect()
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
static mut GLOBAL_REGISTRY: Option<Registry> = None;

/// Get the global registry
///
/// # Safety
/// This is safe as long as it's only called from the main thread
/// (which is the case for GTK applications).
pub fn global_registry() -> &'static mut Registry {
    unsafe {
        GLOBAL_REGISTRY.get_or_insert_with(Registry::new)
    }
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
