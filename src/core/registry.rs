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
    /// Cache for source field metadata to avoid creating full source instances
    /// just to read static field definitions. Key is source ID, value is field list.
    source_fields_cache: RwLock<HashMap<String, Vec<super::FieldMetadata>>>,
}

impl Registry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            sources: RwLock::new(HashMap::new()),
            displayers: RwLock::new(HashMap::new()),
            source_info: RwLock::new(HashMap::new()),
            displayer_info: RwLock::new(HashMap::new()),
            source_fields_cache: RwLock::new(HashMap::new()),
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

    /// Get cached field metadata for a source type.
    ///
    /// This avoids creating expensive source instances just to read static field metadata.
    /// Fields are cached on first access per source type.
    ///
    /// Note: This should NOT be used for "combination" source since its fields depend
    /// on the configured slots. Use `get_source_fields_for_combo_slot` for that.
    pub fn get_source_fields_cached(&self, source_id: &str) -> Vec<super::FieldMetadata> {
        // Skip caching for combination source - its fields are dynamic
        if source_id == "combination" {
            log::warn!("get_source_fields_cached called for 'combination' - use get_source_fields_for_combo_slot instead");
            return Vec::new();
        }

        // Check cache first
        if let Ok(cache) = self.source_fields_cache.read() {
            if let Some(fields) = cache.get(source_id) {
                return fields.clone();
            }
        }

        // Not in cache - create a source instance to get fields, then cache them
        log::info!(
            "Caching field metadata for source '{}' (one-time cost)",
            source_id
        );

        let fields = match self.create_source(source_id) {
            Ok(source) => {
                let fields = source.fields();
                // Cache the fields for future use
                if let Ok(mut cache) = self.source_fields_cache.write() {
                    cache.insert(source_id.to_string(), fields.clone());
                }
                fields
                // source is dropped here, freeing memory
            }
            Err(e) => {
                log::error!("Failed to get fields for source '{}': {}", source_id, e);
                Vec::new()
            }
        };

        fields
    }

    /// Get field metadata for a combo slot by constructing prefixed fields from cached source fields.
    ///
    /// This is much cheaper than creating a full ComboSource with all its child sources,
    /// because it uses cached field metadata for each individual source type.
    pub fn get_source_fields_for_combo_slot(
        &self,
        slot_name: &str,
        source_id: &str,
    ) -> Vec<super::FieldMetadata> {
        if source_id.is_empty() || source_id == "none" {
            return Vec::new();
        }

        let base_fields = self.get_source_fields_cached(source_id);

        // Prefix field IDs and names with slot name (matching ComboSource::fields() behavior)
        base_fields
            .into_iter()
            .map(|field| {
                super::FieldMetadata::new(
                    format!("{}_{}", slot_name, field.id),
                    format!("{} {}", slot_name, field.name),
                    &field.description,
                    field.field_type,
                    field.purpose,
                )
            })
            .collect()
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
