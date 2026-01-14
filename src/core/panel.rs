//! Panel - container for a data source and displayer pair

use super::constants::TRANSFORM_THRESHOLD;
use super::panel_data::{DisplayerConfig, PanelAppearance, PanelData, SourceConfig};
use super::shared_source_manager::{global_shared_source_manager, SharedSourceManager};
use super::{global_registry, BoxedDataSource, BoxedDisplayer, Registry};
use crate::ui::{BackgroundConfig, Color};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

/// Helper macro to extract and deserialize config from HashMap, reducing boilerplate.
/// Returns the deserialized config variant or a default if not found/invalid.
macro_rules! extract_config {
    ($config:expr, $key:expr, $variant:path, $default_type:expr) => {{
        let result = $config
            .get($key)
            .and_then(|val| serde_json::from_value(val.clone()).ok())
            .map(|cfg| $variant(cfg));
        match result {
            Some(cfg) => cfg,
            None => $default_type.unwrap_or_default(),
        }
    }};
}

/// Keys to sync from source values to panel.config for UI access
/// Used by alarm/timer displayers to persist state
const SYNC_KEYS: &[&str] = &[
    "alarms",
    "timers",
    "triggered_alarm_ids",
    "timer_state",
    "alarm_triggered",
    "alarm_enabled",
];

/// Position and size of a panel in the grid
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct PanelGeometry {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// Panel border configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelBorderConfig {
    pub enabled: bool,
    pub width: f64,
    pub color: Color,
}

impl Default for PanelBorderConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            width: 1.0,
            color: Color::new(1.0, 1.0, 1.0, 1.0), // White
        }
    }
}

/// A panel combines a data source and a displayer
///
/// # Dual Architecture (Transitional)
///
/// This struct maintains both new (`data: PanelData`) and legacy fields for backward
/// compatibility during the migration period. When `data` is `Some`, it is the
/// authoritative source of truth and the legacy fields are kept in sync for
/// rendering code that still accesses them directly.
///
/// ## Preferred API (use these):
/// - `Panel::from_data()` / `Panel::from_data_with_registry()` - create panels
/// - `panel.to_data()` - get current config as PanelData
/// - `panel.set_data()` / `panel.update_from_data()` - update config
/// - `panel.data.as_ref()` - access typed config directly
///
/// ## Legacy fields (synced for rendering, will be removed in future):
/// - `config` - use `data.source_config` / `data.displayer_config` instead
/// - `background` - use `data.appearance.background` instead
/// - `corner_radius` - use `data.appearance.corner_radius` instead
/// - `border` - use `data.appearance.border` instead
pub struct Panel {
    /// Unique ID for this panel instance
    pub id: String,
    /// Geometry in the grid
    pub geometry: PanelGeometry,
    /// The data source (kept for metadata/fields access, not for data polling)
    pub source: BoxedDataSource,
    /// Key to the shared source in SharedSourceManager
    /// When set, data is fetched from the shared source instead of polling directly
    pub source_key: Option<String>,
    /// The displayer
    pub displayer: BoxedDisplayer,
    /// Custom configuration
    /// **DEPRECATED**: Use `data.source_config` and `data.displayer_config` instead.
    /// This field is synced from PanelData for legacy code compatibility.
    pub config: HashMap<String, serde_json::Value>,
    /// Background configuration
    /// **DEPRECATED**: Use `data.appearance.background` instead.
    /// This field is synced from PanelData for rendering code compatibility.
    pub background: BackgroundConfig,
    /// Corner radius for panel edges
    /// **DEPRECATED**: Use `data.appearance.corner_radius` instead.
    /// This field is synced from PanelData for rendering code compatibility.
    pub corner_radius: f64,
    /// Border configuration
    /// **DEPRECATED**: Use `data.appearance.border` instead.
    /// This field is synced from PanelData for rendering code compatibility.
    pub border: PanelBorderConfig,
    /// Content scale factor (1.0 = normal)
    /// **DEPRECATED**: Use `data.appearance.scale` instead.
    pub scale: f64,
    /// Content translation X offset in pixels
    /// **DEPRECATED**: Use `data.appearance.translate_x` instead.
    pub translate_x: f64,
    /// Content translation Y offset in pixels
    /// **DEPRECATED**: Use `data.appearance.translate_y` instead.
    pub translate_y: f64,
    /// Z-index for layering (higher = in front, lower = behind)
    /// **DEPRECATED**: Use `data.appearance.z_index` instead.
    pub z_index: i32,
    /// If true, this panel ignores collision detection and can overlap other panels
    /// **DEPRECATED**: Use `data.appearance.ignore_collision` instead.
    pub ignore_collision: bool,
    /// Unified panel data - the single source of truth
    ///
    /// When `Some`, this is the authoritative source for all panel configuration.
    /// The legacy fields above are automatically synced from this data.
    pub data: Option<PanelData>,
}

impl Panel {
    /// Create a new panel (legacy constructor - use from_data for new code)
    pub fn new(
        id: String,
        geometry: PanelGeometry,
        source: BoxedDataSource,
        displayer: BoxedDisplayer,
    ) -> Self {
        Self {
            id,
            geometry,
            source,
            source_key: None, // Legacy panels don't use shared sources
            displayer,
            config: HashMap::new(),
            background: BackgroundConfig::default(),
            corner_radius: 8.0,
            border: PanelBorderConfig::default(),
            scale: 1.0,
            translate_x: 0.0,
            translate_y: 0.0,
            z_index: 0,
            ignore_collision: false,
            data: None, // Legacy panels don't have PanelData yet
        }
    }

    /// Create a new panel from PanelData (preferred constructor)
    ///
    /// This creates source and displayer instances from the registry based on
    /// the types specified in the PanelData, then applies the configurations.
    /// If a SharedSourceManager is available, the panel will use shared sources.
    pub fn from_data(data: PanelData) -> Result<Self> {
        Self::from_data_with_registry(data, global_registry())
    }

    /// Create a new panel from PanelData using a specific registry
    pub fn from_data_with_registry(data: PanelData, registry: &Registry) -> Result<Self> {
        Self::from_data_with_registry_and_source_manager(
            data,
            registry,
            global_shared_source_manager().cloned(),
        )
    }

    /// Create a new panel from PanelData with explicit source manager
    pub fn from_data_with_registry_and_source_manager(
        data: PanelData,
        registry: &Registry,
        source_manager: Option<Arc<SharedSourceManager>>,
    ) -> Result<Self> {
        // Create source from registry (for metadata/fields access)
        let source = registry.create_source(data.source_config.source_type())?;
        let displayer = registry.create_displayer(data.displayer_config.displayer_type())?;

        // Register with shared source manager if available
        let source_key = if let Some(ref manager) = source_manager {
            match manager.get_or_create_source(&data.source_config, &data.id, registry) {
                Ok(key) => {
                    log::debug!("Panel {} using shared source {}", data.id, key);
                    Some(key)
                }
                Err(e) => {
                    log::warn!("Failed to create shared source for panel {}: {}, falling back to direct polling", data.id, e);
                    None
                }
            }
        } else {
            None
        };

        // Build the combined config map for legacy interfaces
        let config = data.combined_config_map();

        let mut panel = Self {
            id: data.id.clone(),
            geometry: data.geometry,
            source,
            source_key,
            displayer,
            config,
            background: data.appearance.background.clone(),
            corner_radius: data.appearance.corner_radius,
            border: data.appearance.border.clone(),
            scale: data.appearance.scale,
            translate_x: data.appearance.translate_x,
            translate_y: data.appearance.translate_y,
            z_index: data.appearance.z_index,
            ignore_collision: data.appearance.ignore_collision,
            data: Some(data),
        };

        // Apply configurations to source and displayer
        panel.apply_configs_from_data()?;

        Ok(panel)
    }

    /// Update the data source and refresh the displayer
    ///
    /// If a shared source is being used (source_key is set), this fetches
    /// cached values from the SharedSourceManager. Otherwise, it polls directly.
    pub fn update(&mut self) -> Result<()> {
        // Get values - either from shared source or by polling directly
        // Use Cow to avoid cloning when transforms aren't needed (common case)
        // SharedSourceManager returns Arc<HashMap>, we store it to keep the borrow alive
        // The _arc_values variable keeps the Arc alive so Cow can borrow from it
        let _arc_values: Option<Arc<HashMap<String, serde_json::Value>>>;
        let values: Cow<'_, HashMap<String, serde_json::Value>> =
            if let Some(ref key) = self.source_key {
                // Use shared source - values are already updated by UpdateManager
                if let Some(manager) = global_shared_source_manager() {
                    _arc_values = manager.get_values(key);
                    if let Some(ref arc) = _arc_values {
                        // Borrow from Arc - no clone needed
                        Cow::Borrowed(arc.as_ref())
                    } else {
                        log::warn!(
                            "Shared source {} not found, falling back to direct poll",
                            key
                        );
                        self.source.update().ok();
                        Cow::Owned(self.source.get_values())
                    }
                } else {
                    // No manager available, fall back to direct poll
                    _arc_values = None;
                    self.source.update()?;
                    Cow::Owned(self.source.get_values())
                }
            } else {
                // No shared source, poll directly (legacy behavior)
                _arc_values = None;
                self.source.update()?;
                Cow::Owned(self.source.get_values())
            };

        // Only add transform values if panel has non-default transforms
        // Default: scale=1.0, translate_x=0.0, translate_y=0.0
        let has_transform = (self.scale - 1.0).abs() > TRANSFORM_THRESHOLD
            || self.translate_x.abs() > TRANSFORM_THRESHOLD
            || self.translate_y.abs() > TRANSFORM_THRESHOLD;

        // Add transform values if needed, then update displayer
        // Only clone when we actually need to modify the HashMap
        if has_transform {
            let mut values_owned = values.into_owned();
            values_owned.insert(
                "_panel_scale".to_string(),
                serde_json::Value::from(self.scale),
            );
            values_owned.insert(
                "_panel_translate_x".to_string(),
                serde_json::Value::from(self.translate_x),
            );
            values_owned.insert(
                "_panel_translate_y".to_string(),
                serde_json::Value::from(self.translate_y),
            );
            self.displayer.update_data(&values_owned);
            self.sync_keys_to_config(&values_owned);
        } else {
            self.displayer.update_data(&values);
            self.sync_keys_to_config(&values);
        }

        Ok(())
    }

    /// Sync certain live values from source to panel.config for UI access
    /// Used by alarm/timer displayers to persist state across updates
    ///
    /// Optimized to only insert when value differs, avoiding allocations
    /// when values haven't changed (common case for alarm/timer state).
    #[inline]
    fn sync_keys_to_config(&mut self, values: &HashMap<String, serde_json::Value>) {
        for key in SYNC_KEYS {
            if let Some(value) = values.get(*key) {
                // Only insert if value differs from current (avoids to_string + clone)
                let should_insert = self.config.get(*key) != Some(value);
                if should_insert {
                    self.config.insert((*key).to_string(), value.clone());
                }
            }
        }
    }

    /// Update the displayer with pre-fetched values from a shared source
    ///
    /// This is called by UpdateManager after it has updated all shared sources.
    /// It avoids the need to look up values again.
    pub fn update_with_values(&mut self, values: &HashMap<String, serde_json::Value>) {
        // Only clone HashMap if we need to add transform values (non-default transforms)
        // Default: scale=1.0, translate_x=0.0, translate_y=0.0
        let has_transform = (self.scale - 1.0).abs() > TRANSFORM_THRESHOLD
            || self.translate_x.abs() > TRANSFORM_THRESHOLD
            || self.translate_y.abs() > TRANSFORM_THRESHOLD;

        if has_transform {
            // Clone and add transform values
            let mut values_with_transform = values.clone();
            values_with_transform.insert(
                "_panel_scale".to_string(),
                serde_json::Value::from(self.scale),
            );
            values_with_transform.insert(
                "_panel_translate_x".to_string(),
                serde_json::Value::from(self.translate_x),
            );
            values_with_transform.insert(
                "_panel_translate_y".to_string(),
                serde_json::Value::from(self.translate_y),
            );
            self.displayer.update_data(&values_with_transform);
        } else {
            // No transform needed, pass reference directly without cloning
            self.displayer.update_data(values);
        }

        self.sync_keys_to_config(values);
    }

    /// Apply configuration to the source and displayer (legacy method)
    pub fn apply_config(&mut self, config: HashMap<String, serde_json::Value>) -> Result<()> {
        // Configure the data source (local copy for metadata)
        self.source.configure(&config)?;

        // Handle shared source re-registration when config changes
        // Each panel should have its own source instance when configs differ
        if let Some(typed_config) =
            SourceConfig::extract_from_hashmap(&config, self.source.metadata().id.as_str())
        {
            use super::shared_source_manager::SharedSourceManager;

            let new_key = SharedSourceManager::generate_source_key(&typed_config);

            if let Some(manager) = global_shared_source_manager() {
                // Check if the source key has changed (config is now different)
                let key_changed = self
                    .source_key
                    .as_ref()
                    .map(|k| k != &new_key)
                    .unwrap_or(true);

                if key_changed {
                    // Release the old source reference if we had one
                    if let Some(ref old_key) = self.source_key {
                        manager.release_source(old_key, &self.id);
                        log::debug!("Panel {} released old source {}", self.id, old_key);
                    }

                    // Get or create a new source with the new configuration
                    let registry = global_registry();
                    match manager.get_or_create_source(&typed_config, &self.id, registry) {
                        Ok(key) => {
                            log::debug!("Panel {} now using source {}", self.id, key);
                            self.source_key = Some(key);
                        }
                        Err(e) => {
                            log::warn!(
                                "Failed to create shared source for panel {}: {}",
                                self.id,
                                e
                            );
                            self.source_key = None;
                        }
                    }
                }
            }
        }

        // Configure the displayer
        self.displayer.apply_config(&config)?;

        // Store config by moving ownership (no clone needed)
        self.config = config;
        Ok(())
    }

    /// Apply configurations from the PanelData to source and displayer
    ///
    /// Uses the typed config methods (configure_typed/apply_config_typed) which
    /// internally fall back to HashMap-based methods if sources/displayers
    /// haven't implemented the typed versions yet.
    fn apply_configs_from_data(&mut self) -> Result<()> {
        if let Some(ref data) = self.data {
            // Use typed config methods - they internally convert to HashMap if needed
            self.source.configure_typed(&data.source_config)?;
            self.displayer.apply_config_typed(&data.displayer_config)?;

            // Handle shared source re-registration when config changes
            use super::shared_source_manager::SharedSourceManager;

            let new_key = SharedSourceManager::generate_source_key(&data.source_config);

            if let Some(manager) = global_shared_source_manager() {
                let key_changed = self
                    .source_key
                    .as_ref()
                    .map(|k| k != &new_key)
                    .unwrap_or(true);

                if key_changed {
                    // Release the old source reference if we had one
                    if let Some(ref old_key) = self.source_key {
                        manager.release_source(old_key, &self.id);
                        log::debug!("Panel {} released old source {}", self.id, old_key);
                    }

                    // Get or create a new source with the new configuration
                    let registry = global_registry();
                    match manager.get_or_create_source(&data.source_config, &self.id, registry) {
                        Ok(key) => {
                            log::debug!("Panel {} now using source {}", self.id, key);
                            self.source_key = Some(key);
                        }
                        Err(e) => {
                            log::warn!(
                                "Failed to create shared source for panel {}: {}",
                                self.id,
                                e
                            );
                            self.source_key = None;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Convert the current panel state to PanelData
    ///
    /// Prefers getting typed config from source/displayer if they support it (via get_typed_config),
    /// otherwise falls back to extracting from the legacy config HashMap.
    pub fn to_data(&self) -> PanelData {
        let source_type = self.source.metadata().id.as_str();
        let displayer_type = self.displayer.id();

        // Prefer typed source config if available, otherwise extract from HashMap
        let source_config = self
            .source
            .get_typed_config()
            .unwrap_or_else(|| self.extract_source_config(source_type));

        // Prefer typed displayer config if available, otherwise extract from HashMap
        let displayer_config = self
            .displayer
            .get_typed_config()
            .unwrap_or_else(|| self.extract_displayer_config(displayer_type));

        PanelData {
            id: self.id.clone(),
            geometry: self.geometry,
            source_config,
            displayer_config,
            appearance: PanelAppearance {
                background: self.background.clone(),
                corner_radius: self.corner_radius,
                border: self.border.clone(),
                scale: self.scale,
                translate_x: self.translate_x,
                translate_y: self.translate_y,
                z_index: self.z_index,
                ignore_collision: self.ignore_collision,
            },
        }
    }

    /// Extract source config from the legacy config HashMap
    fn extract_source_config(&self, source_type: &str) -> SourceConfig {
        match source_type {
            "cpu" => extract_config!(
                self.config,
                "cpu_config",
                SourceConfig::Cpu,
                SourceConfig::default_for_type("cpu")
            ),
            "gpu" => extract_config!(
                self.config,
                "gpu_config",
                SourceConfig::Gpu,
                SourceConfig::default_for_type("gpu")
            ),
            "memory" => extract_config!(
                self.config,
                "memory_config",
                SourceConfig::Memory,
                SourceConfig::default_for_type("memory")
            ),
            "disk" => extract_config!(
                self.config,
                "disk_config",
                SourceConfig::Disk,
                SourceConfig::default_for_type("disk")
            ),
            "clock" => extract_config!(
                self.config,
                "clock_config",
                SourceConfig::Clock,
                SourceConfig::default_for_type("clock")
            ),
            "combination" => extract_config!(
                self.config,
                "combo_config",
                SourceConfig::Combo,
                SourceConfig::default_for_type("combination")
            ),
            "system_temp" => extract_config!(
                self.config,
                "system_temp_config",
                SourceConfig::SystemTemp,
                SourceConfig::default_for_type("system_temp")
            ),
            "fan_speed" => extract_config!(
                self.config,
                "fan_speed_config",
                SourceConfig::FanSpeed,
                SourceConfig::default_for_type("fan_speed")
            ),
            "test" => extract_config!(
                self.config,
                "test_config",
                SourceConfig::Test,
                SourceConfig::default_for_type("test")
            ),
            "static_text" => extract_config!(
                self.config,
                "static_text_config",
                SourceConfig::StaticText,
                SourceConfig::default_for_type("static_text")
            ),
            _ => SourceConfig::default(),
        }
    }

    /// Extract displayer config from the legacy config HashMap
    fn extract_displayer_config(&self, displayer_type: &str) -> DisplayerConfig {
        match displayer_type {
            "text" => extract_config!(
                self.config,
                "text_config",
                DisplayerConfig::Text,
                DisplayerConfig::default_for_type("text")
            ),
            "bar" => extract_config!(
                self.config,
                "bar_config",
                DisplayerConfig::Bar,
                DisplayerConfig::default_for_type("bar")
            ),
            "arc" => extract_config!(
                self.config,
                "arc_config",
                DisplayerConfig::Arc,
                DisplayerConfig::default_for_type("arc")
            ),
            "speedometer" => extract_config!(
                self.config,
                "speedometer_config",
                DisplayerConfig::Speedometer,
                DisplayerConfig::default_for_type("speedometer")
            ),
            "graph" => extract_config!(
                self.config,
                "graph_config",
                DisplayerConfig::Graph,
                DisplayerConfig::default_for_type("graph")
            ),
            "clock_analog" => extract_config!(
                self.config,
                "clock_analog_config",
                DisplayerConfig::ClockAnalog,
                DisplayerConfig::default_for_type("clock_analog")
            ),
            "clock_digital" => extract_config!(
                self.config,
                "clock_digital_config",
                DisplayerConfig::ClockDigital,
                DisplayerConfig::default_for_type("clock_digital")
            ),
            "lcars" => extract_config!(
                self.config,
                "lcars_config",
                DisplayerConfig::Lcars,
                DisplayerConfig::default_for_type("lcars")
            ),
            "cpu_cores" => extract_config!(
                self.config,
                "core_bars_config",
                DisplayerConfig::CpuCores,
                DisplayerConfig::default_for_type("cpu_cores")
            ),
            "indicator" => extract_config!(
                self.config,
                "indicator_config",
                DisplayerConfig::Indicator,
                DisplayerConfig::default_for_type("indicator")
            ),
            _ => DisplayerConfig::default(),
        }
    }

    /// Update the panel from new PanelData
    ///
    /// This will:
    /// 1. Update the stored PanelData
    /// 2. Recreate source/displayer if types changed
    /// 3. Apply new configurations
    /// 4. Sync legacy fields for backward compatibility
    pub fn update_from_data(&mut self, new_data: PanelData) -> Result<()> {
        self.update_from_data_with_registry(new_data, global_registry())
    }

    /// Update the panel from new PanelData using a specific registry
    pub fn update_from_data_with_registry(
        &mut self,
        new_data: PanelData,
        registry: &Registry,
    ) -> Result<()> {
        let old_source_key = SharedSourceManager::generate_source_key(
            self.data
                .as_ref()
                .map(|d| &d.source_config)
                .unwrap_or(&SourceConfig::default()),
        );
        let new_source_key = SharedSourceManager::generate_source_key(&new_data.source_config);

        let old_displayer_type = self
            .data
            .as_ref()
            .map(|d| d.displayer_config.displayer_type())
            .unwrap_or_else(|| self.displayer.id());

        let new_source_type = new_data.source_config.source_type();
        let new_displayer_type = new_data.displayer_config.displayer_type();

        // Handle shared source changes
        if old_source_key != new_source_key {
            // Release old shared source
            if let Some(ref old_key) = self.source_key {
                if let Some(manager) = global_shared_source_manager() {
                    manager.release_source(old_key, &self.id);
                }
            }

            // Create new source instance (for metadata/fields)
            self.source = registry.create_source(new_source_type)?;

            // Register with shared source manager
            if let Some(manager) = global_shared_source_manager() {
                match manager.get_or_create_source(&new_data.source_config, &self.id, registry) {
                    Ok(key) => {
                        log::debug!("Panel {} updated to shared source {}", self.id, key);
                        self.source_key = Some(key);
                    }
                    Err(e) => {
                        log::warn!(
                            "Failed to create shared source for panel {}: {}",
                            self.id,
                            e
                        );
                        self.source_key = None;
                    }
                }
            }
        } else if let Some(ref key) = self.source_key {
            // Source key unchanged but config might have - update the shared source config
            if let Some(manager) = global_shared_source_manager() {
                let _ = manager.configure_source(key, &new_data.source_config);
            }
        }

        // Recreate displayer if type changed
        if old_displayer_type != new_displayer_type {
            self.displayer = registry.create_displayer(new_displayer_type)?;
        }

        // Update all fields
        self.id = new_data.id.clone();
        self.geometry = new_data.geometry;
        self.background = new_data.appearance.background.clone();
        self.corner_radius = new_data.appearance.corner_radius;
        self.border = new_data.appearance.border.clone();
        self.scale = new_data.appearance.scale;
        self.translate_x = new_data.appearance.translate_x;
        self.translate_y = new_data.appearance.translate_y;
        self.z_index = new_data.appearance.z_index;
        self.ignore_collision = new_data.appearance.ignore_collision;
        self.config = new_data.combined_config_map();
        self.data = Some(new_data);

        // Apply configurations
        self.apply_configs_from_data()
    }

    /// Set the PanelData without recreating source/displayer
    ///
    /// Use this when you know the types haven't changed and just want to
    /// update the configuration. For type changes, use update_from_data().
    pub fn set_data(&mut self, data: PanelData) -> Result<()> {
        // Sync legacy fields
        self.id = data.id.clone();
        self.geometry = data.geometry;
        self.background = data.appearance.background.clone();
        self.corner_radius = data.appearance.corner_radius;
        self.border = data.appearance.border.clone();
        self.scale = data.appearance.scale;
        self.translate_x = data.appearance.translate_x;
        self.translate_y = data.appearance.translate_y;
        self.z_index = data.appearance.z_index;
        self.ignore_collision = data.appearance.ignore_collision;
        self.config = data.combined_config_map();
        self.data = Some(data);

        // Apply configurations
        self.apply_configs_from_data()
    }

    /// Get a reference to the PanelData if available
    pub fn get_data(&self) -> Option<&PanelData> {
        self.data.as_ref()
    }

    /// Get a mutable reference to the PanelData if available
    pub fn get_data_mut(&mut self) -> Option<&mut PanelData> {
        self.data.as_mut()
    }

    /// Check if this panel has PanelData (vs legacy config)
    pub fn has_data(&self) -> bool {
        self.data.is_some()
    }

    // =====================================================
    // Convenience accessors for PanelData (preferred API)
    // =====================================================

    /// Get the source type ID string
    pub fn source_type(&self) -> &str {
        self.data
            .as_ref()
            .map(|d| d.source_config.source_type())
            .unwrap_or_else(|| self.source.metadata().id.as_str())
    }

    /// Get the displayer type ID string
    pub fn displayer_type(&self) -> &str {
        self.data
            .as_ref()
            .map(|d| d.displayer_config.displayer_type())
            .unwrap_or_else(|| self.displayer.id())
    }

    /// Get the source config (if PanelData is available)
    pub fn source_config(&self) -> Option<&SourceConfig> {
        self.data.as_ref().map(|d| &d.source_config)
    }

    /// Get the displayer config (if PanelData is available)
    pub fn displayer_config(&self) -> Option<&DisplayerConfig> {
        self.data.as_ref().map(|d| &d.displayer_config)
    }

    /// Get the appearance config (if PanelData is available)
    pub fn appearance(&self) -> Option<&PanelAppearance> {
        self.data.as_ref().map(|d| &d.appearance)
    }

    /// Get the update interval from the source config
    pub fn update_interval_ms(&self) -> u64 {
        self.data
            .as_ref()
            .map(|d| d.source_config.update_interval_ms())
            .unwrap_or(1000) // Default 1 second
    }
}

impl Drop for Panel {
    fn drop(&mut self) {
        // Release the shared source reference when the panel is dropped.
        // This ensures the SharedSourceManager can clean up unused sources.
        if let Some(ref source_key) = self.source_key {
            if let Some(manager) = global_shared_source_manager() {
                manager.release_source(source_key, &self.id);
                log::debug!(
                    "Panel {} dropped, released shared source {}",
                    self.id,
                    source_key
                );
            }
        }
    }
}
