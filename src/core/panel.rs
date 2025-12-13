//! Panel - container for a data source and displayer pair

use super::{BoxedDataSource, BoxedDisplayer, Registry, global_registry};
use super::panel_data::{PanelData, PanelAppearance, SourceConfig, DisplayerConfig};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::ui::{BackgroundConfig, Color};

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
    /// The data source
    pub source: BoxedDataSource,
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
            displayer,
            config: HashMap::new(),
            background: BackgroundConfig::default(),
            corner_radius: 8.0,
            border: PanelBorderConfig::default(),
            data: None, // Legacy panels don't have PanelData yet
        }
    }

    /// Create a new panel from PanelData (preferred constructor)
    ///
    /// This creates source and displayer instances from the registry based on
    /// the types specified in the PanelData, then applies the configurations.
    pub fn from_data(data: PanelData) -> Result<Self> {
        Self::from_data_with_registry(data, global_registry())
    }

    /// Create a new panel from PanelData using a specific registry
    pub fn from_data_with_registry(data: PanelData, registry: &Registry) -> Result<Self> {
        // Create source and displayer from registry
        let source = registry.create_source(data.source_config.source_type())?;
        let displayer = registry.create_displayer(data.displayer_config.displayer_type())?;

        // Build the combined config map for legacy interfaces
        let config = data.combined_config_map();

        let mut panel = Self {
            id: data.id.clone(),
            geometry: data.geometry,
            source,
            displayer,
            config,
            background: data.appearance.background.clone(),
            corner_radius: data.appearance.corner_radius,
            border: data.appearance.border.clone(),
            data: Some(data),
        };

        // Apply configurations to source and displayer
        panel.apply_configs_from_data()?;

        Ok(panel)
    }

    /// Update the data source and refresh the displayer
    pub fn update(&mut self) -> Result<()> {
        // Update source
        self.source.update()?;

        // Get data and update displayer
        let values = self.source.get_values();
        self.displayer.update_data(&values);

        Ok(())
    }

    /// Apply configuration to the source and displayer (legacy method)
    pub fn apply_config(&mut self, config: HashMap<String, serde_json::Value>) -> Result<()> {
        self.config = config.clone();

        // Configure the data source
        self.source.configure(&config)?;

        // Configure the displayer
        self.displayer.apply_config(&config)
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
        let source_config = self.source.get_typed_config()
            .unwrap_or_else(|| self.extract_source_config(source_type));

        // Prefer typed displayer config if available, otherwise extract from HashMap
        let displayer_config = self.displayer.get_typed_config()
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
            },
        }
    }

    /// Extract source config from the legacy config HashMap
    fn extract_source_config(&self, source_type: &str) -> SourceConfig {
        match source_type {
            "cpu" => {
                if let Some(val) = self.config.get("cpu_config") {
                    if let Ok(cfg) = serde_json::from_value(val.clone()) {
                        return SourceConfig::Cpu(cfg);
                    }
                }
                SourceConfig::default_for_type("cpu").unwrap_or_default()
            }
            "gpu" => {
                if let Some(val) = self.config.get("gpu_config") {
                    if let Ok(cfg) = serde_json::from_value(val.clone()) {
                        return SourceConfig::Gpu(cfg);
                    }
                }
                SourceConfig::default_for_type("gpu").unwrap_or_default()
            }
            "memory" => {
                if let Some(val) = self.config.get("memory_config") {
                    if let Ok(cfg) = serde_json::from_value(val.clone()) {
                        return SourceConfig::Memory(cfg);
                    }
                }
                SourceConfig::default_for_type("memory").unwrap_or_default()
            }
            "disk" => {
                if let Some(val) = self.config.get("disk_config") {
                    if let Ok(cfg) = serde_json::from_value(val.clone()) {
                        return SourceConfig::Disk(cfg);
                    }
                }
                SourceConfig::default_for_type("disk").unwrap_or_default()
            }
            "clock" => {
                if let Some(val) = self.config.get("clock_config") {
                    if let Ok(cfg) = serde_json::from_value(val.clone()) {
                        return SourceConfig::Clock(cfg);
                    }
                }
                SourceConfig::default_for_type("clock").unwrap_or_default()
            }
            "combination" => {
                if let Some(val) = self.config.get("combo_config") {
                    if let Ok(cfg) = serde_json::from_value(val.clone()) {
                        return SourceConfig::Combo(cfg);
                    }
                }
                SourceConfig::default_for_type("combination").unwrap_or_default()
            }
            "system_temp" => {
                if let Some(val) = self.config.get("system_temp_config") {
                    if let Ok(cfg) = serde_json::from_value(val.clone()) {
                        return SourceConfig::SystemTemp(cfg);
                    }
                }
                SourceConfig::default_for_type("system_temp").unwrap_or_default()
            }
            "fan_speed" => {
                if let Some(val) = self.config.get("fan_speed_config") {
                    if let Ok(cfg) = serde_json::from_value(val.clone()) {
                        return SourceConfig::FanSpeed(cfg);
                    }
                }
                SourceConfig::default_for_type("fan_speed").unwrap_or_default()
            }
            _ => SourceConfig::default()
        }
    }

    /// Extract displayer config from the legacy config HashMap
    fn extract_displayer_config(&self, displayer_type: &str) -> DisplayerConfig {
        match displayer_type {
            "text" => {
                if let Some(val) = self.config.get("text_config") {
                    if let Ok(cfg) = serde_json::from_value(val.clone()) {
                        return DisplayerConfig::Text(cfg);
                    }
                }
                DisplayerConfig::default_for_type("text").unwrap_or_default()
            }
            "bar" => {
                if let Some(val) = self.config.get("bar_config") {
                    if let Ok(cfg) = serde_json::from_value(val.clone()) {
                        return DisplayerConfig::Bar(cfg);
                    }
                }
                DisplayerConfig::default_for_type("bar").unwrap_or_default()
            }
            "arc" => {
                if let Some(val) = self.config.get("arc_config") {
                    if let Ok(cfg) = serde_json::from_value(val.clone()) {
                        return DisplayerConfig::Arc(cfg);
                    }
                }
                DisplayerConfig::default_for_type("arc").unwrap_or_default()
            }
            "speedometer" => {
                if let Some(val) = self.config.get("speedometer_config") {
                    if let Ok(cfg) = serde_json::from_value(val.clone()) {
                        return DisplayerConfig::Speedometer(cfg);
                    }
                }
                DisplayerConfig::default_for_type("speedometer").unwrap_or_default()
            }
            "graph" => {
                if let Some(val) = self.config.get("graph_config") {
                    if let Ok(cfg) = serde_json::from_value(val.clone()) {
                        return DisplayerConfig::Graph(cfg);
                    }
                }
                DisplayerConfig::default_for_type("graph").unwrap_or_default()
            }
            "clock_analog" => {
                if let Some(val) = self.config.get("clock_analog_config") {
                    if let Ok(cfg) = serde_json::from_value(val.clone()) {
                        return DisplayerConfig::ClockAnalog(cfg);
                    }
                }
                DisplayerConfig::default_for_type("clock_analog").unwrap_or_default()
            }
            "clock_digital" => {
                if let Some(val) = self.config.get("clock_digital_config") {
                    if let Ok(cfg) = serde_json::from_value(val.clone()) {
                        return DisplayerConfig::ClockDigital(cfg);
                    }
                }
                DisplayerConfig::default_for_type("clock_digital").unwrap_or_default()
            }
            "lcars" => {
                if let Some(val) = self.config.get("lcars_config") {
                    if let Ok(cfg) = serde_json::from_value(val.clone()) {
                        return DisplayerConfig::Lcars(cfg);
                    }
                }
                DisplayerConfig::default_for_type("lcars").unwrap_or_default()
            }
            "cpu_cores" => {
                if let Some(val) = self.config.get("core_bars_config") {
                    if let Ok(cfg) = serde_json::from_value(val.clone()) {
                        return DisplayerConfig::CpuCores(cfg);
                    }
                }
                DisplayerConfig::default_for_type("cpu_cores").unwrap_or_default()
            }
            _ => DisplayerConfig::default()
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
    pub fn update_from_data_with_registry(&mut self, new_data: PanelData, registry: &Registry) -> Result<()> {
        let old_source_type = self.data.as_ref()
            .map(|d| d.source_config.source_type())
            .unwrap_or_else(|| self.source.metadata().id.as_str());
        let old_displayer_type = self.data.as_ref()
            .map(|d| d.displayer_config.displayer_type())
            .unwrap_or_else(|| self.displayer.id());

        let new_source_type = new_data.source_config.source_type();
        let new_displayer_type = new_data.displayer_config.displayer_type();

        // Recreate source if type changed
        if old_source_type != new_source_type {
            self.source = registry.create_source(new_source_type)?;
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
        self.data.as_ref()
            .map(|d| d.source_config.source_type())
            .unwrap_or_else(|| self.source.metadata().id.as_str())
    }

    /// Get the displayer type ID string
    pub fn displayer_type(&self) -> &str {
        self.data.as_ref()
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
        self.data.as_ref()
            .map(|d| d.source_config.update_interval_ms())
            .unwrap_or(1000) // Default 1 second
    }
}
