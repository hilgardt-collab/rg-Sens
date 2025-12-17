//! Unified panel data structure - single source of truth for panel configuration.
//!
//! This module defines the `PanelData` struct which contains ALL configuration
//! for a panel in one place. This replaces the fragmented approach where config
//! was spread across Panel.config HashMap, source internals, and displayer internals.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::core::PanelGeometry;
use crate::ui::BackgroundConfig;
use crate::core::PanelBorderConfig;

// Re-export source configs
pub use crate::ui::CpuSourceConfig;
pub use crate::ui::GpuSourceConfig;
pub use crate::ui::MemorySourceConfig;
pub use crate::ui::DiskSourceConfig;
pub use crate::sources::ClockSourceConfig;
pub use crate::sources::ComboSourceConfig;
pub use crate::sources::SystemTempConfig;
pub use crate::sources::FanSpeedConfig;
pub use crate::sources::TestSourceConfig;

// Re-export displayer configs
pub use crate::displayers::TextDisplayerConfig;
pub use crate::ui::BarDisplayConfig;
pub use crate::ui::ArcDisplayConfig;
pub use crate::ui::SpeedometerConfig;
pub use crate::ui::GraphDisplayConfig;
pub use crate::ui::AnalogClockConfig;
pub use crate::displayers::DigitalClockConfig;
pub use crate::displayers::LcarsDisplayConfig;
pub use crate::ui::CoreBarsConfig;
pub use crate::displayers::IndicatorConfig;

/// Type-safe enum for all source configurations.
/// Uses serde tag for JSON serialization: {"type": "cpu", ...}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "source_type")]
pub enum SourceConfig {
    #[serde(rename = "cpu")]
    Cpu(CpuSourceConfig),

    #[serde(rename = "gpu")]
    Gpu(GpuSourceConfig),

    #[serde(rename = "memory")]
    Memory(MemorySourceConfig),

    #[serde(rename = "disk")]
    Disk(DiskSourceConfig),

    #[serde(rename = "clock")]
    Clock(ClockSourceConfig),

    #[serde(rename = "combination")]
    Combo(ComboSourceConfig),

    #[serde(rename = "system_temp")]
    SystemTemp(SystemTempConfig),

    #[serde(rename = "fan_speed")]
    FanSpeed(FanSpeedConfig),

    #[serde(rename = "test")]
    Test(TestSourceConfig),
}

impl SourceConfig {
    /// Get the source type ID string
    pub fn source_type(&self) -> &'static str {
        match self {
            SourceConfig::Cpu(_) => "cpu",
            SourceConfig::Gpu(_) => "gpu",
            SourceConfig::Memory(_) => "memory",
            SourceConfig::Disk(_) => "disk",
            SourceConfig::Clock(_) => "clock",
            SourceConfig::Combo(_) => "combination",
            SourceConfig::SystemTemp(_) => "system_temp",
            SourceConfig::FanSpeed(_) => "fan_speed",
            SourceConfig::Test(_) => "test",
        }
    }

    /// Get the update interval in milliseconds from this source config
    pub fn update_interval_ms(&self) -> u64 {
        match self {
            SourceConfig::Cpu(cfg) => cfg.update_interval_ms,
            SourceConfig::Gpu(cfg) => cfg.update_interval_ms,
            SourceConfig::Memory(cfg) => cfg.update_interval_ms,
            SourceConfig::Disk(cfg) => cfg.update_interval_ms,
            SourceConfig::Clock(cfg) => cfg.update_interval_ms,
            SourceConfig::Combo(cfg) => cfg.update_interval_ms,
            SourceConfig::SystemTemp(cfg) => cfg.update_interval_ms,
            SourceConfig::FanSpeed(cfg) => cfg.update_interval_ms,
            SourceConfig::Test(cfg) => cfg.update_interval_ms,
        }
    }

    /// Convert to HashMap<String, Value> for legacy interface compatibility
    pub fn to_hashmap(&self) -> HashMap<String, serde_json::Value> {
        let mut map = HashMap::new();
        match self {
            SourceConfig::Cpu(cfg) => {
                if let Ok(val) = serde_json::to_value(cfg) {
                    map.insert("cpu_config".to_string(), val);
                }
            }
            SourceConfig::Gpu(cfg) => {
                if let Ok(val) = serde_json::to_value(cfg) {
                    map.insert("gpu_config".to_string(), val);
                }
            }
            SourceConfig::Memory(cfg) => {
                if let Ok(val) = serde_json::to_value(cfg) {
                    map.insert("memory_config".to_string(), val);
                }
            }
            SourceConfig::Disk(cfg) => {
                if let Ok(val) = serde_json::to_value(cfg) {
                    map.insert("disk_config".to_string(), val);
                }
            }
            SourceConfig::Clock(cfg) => {
                if let Ok(val) = serde_json::to_value(cfg) {
                    map.insert("clock_config".to_string(), val);
                }
            }
            SourceConfig::Combo(cfg) => {
                if let Ok(val) = serde_json::to_value(cfg) {
                    map.insert("combo_config".to_string(), val);
                }
            }
            SourceConfig::SystemTemp(cfg) => {
                if let Ok(val) = serde_json::to_value(cfg) {
                    map.insert("system_temp_config".to_string(), val);
                }
            }
            SourceConfig::FanSpeed(cfg) => {
                if let Ok(val) = serde_json::to_value(cfg) {
                    map.insert("fan_speed_config".to_string(), val);
                }
            }
            SourceConfig::Test(cfg) => {
                if let Ok(val) = serde_json::to_value(cfg) {
                    map.insert("test_config".to_string(), val);
                }
            }
        }
        map
    }

    /// Create a default SourceConfig for a given source type ID
    pub fn default_for_type(source_type: &str) -> Option<Self> {
        match source_type {
            "cpu" => Some(SourceConfig::Cpu(CpuSourceConfig::default())),
            "gpu" => Some(SourceConfig::Gpu(GpuSourceConfig::default())),
            "memory" => Some(SourceConfig::Memory(MemorySourceConfig::default())),
            "disk" => Some(SourceConfig::Disk(DiskSourceConfig::default())),
            "clock" => Some(SourceConfig::Clock(ClockSourceConfig::default())),
            "combination" => Some(SourceConfig::Combo(ComboSourceConfig::default())),
            "system_temp" => Some(SourceConfig::SystemTemp(SystemTempConfig::default())),
            "fan_speed" => Some(SourceConfig::FanSpeed(FanSpeedConfig::default())),
            "test" => Some(SourceConfig::Test(TestSourceConfig::default())),
            _ => None,
        }
    }
}

impl Default for SourceConfig {
    fn default() -> Self {
        SourceConfig::Cpu(CpuSourceConfig::default())
    }
}

/// Type-safe enum for all displayer configurations.
/// Uses serde tag for JSON serialization: {"type": "bar", ...}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "displayer_type")]
pub enum DisplayerConfig {
    #[serde(rename = "text")]
    Text(TextDisplayerConfig),

    #[serde(rename = "bar")]
    Bar(BarDisplayConfig),

    #[serde(rename = "arc")]
    Arc(ArcDisplayConfig),

    #[serde(rename = "speedometer")]
    Speedometer(SpeedometerConfig),

    #[serde(rename = "graph")]
    Graph(GraphDisplayConfig),

    #[serde(rename = "clock_analog")]
    ClockAnalog(AnalogClockConfig),

    #[serde(rename = "clock_digital")]
    ClockDigital(DigitalClockConfig),

    #[serde(rename = "lcars")]
    Lcars(LcarsDisplayConfig),

    #[serde(rename = "cpu_cores")]
    CpuCores(CoreBarsConfig),

    #[serde(rename = "indicator")]
    Indicator(IndicatorConfig),
}

impl DisplayerConfig {
    /// Get the displayer type ID string
    pub fn displayer_type(&self) -> &'static str {
        match self {
            DisplayerConfig::Text(_) => "text",
            DisplayerConfig::Bar(_) => "bar",
            DisplayerConfig::Arc(_) => "arc",
            DisplayerConfig::Speedometer(_) => "speedometer",
            DisplayerConfig::Graph(_) => "graph",
            DisplayerConfig::ClockAnalog(_) => "clock_analog",
            DisplayerConfig::ClockDigital(_) => "clock_digital",
            DisplayerConfig::Lcars(_) => "lcars",
            DisplayerConfig::CpuCores(_) => "cpu_cores",
            DisplayerConfig::Indicator(_) => "indicator",
        }
    }

    /// Convert to HashMap<String, Value> for legacy interface compatibility
    pub fn to_hashmap(&self) -> HashMap<String, serde_json::Value> {
        let mut map = HashMap::new();
        match self {
            DisplayerConfig::Text(cfg) => {
                if let Ok(val) = serde_json::to_value(cfg) {
                    map.insert("text_config".to_string(), val);
                }
            }
            DisplayerConfig::Bar(cfg) => {
                if let Ok(val) = serde_json::to_value(cfg) {
                    map.insert("bar_config".to_string(), val);
                }
            }
            DisplayerConfig::Arc(cfg) => {
                if let Ok(val) = serde_json::to_value(cfg) {
                    map.insert("arc_config".to_string(), val);
                }
            }
            DisplayerConfig::Speedometer(cfg) => {
                if let Ok(val) = serde_json::to_value(cfg) {
                    map.insert("speedometer_config".to_string(), val);
                }
            }
            DisplayerConfig::Graph(cfg) => {
                if let Ok(val) = serde_json::to_value(cfg) {
                    map.insert("graph_config".to_string(), val);
                }
            }
            DisplayerConfig::ClockAnalog(cfg) => {
                if let Ok(val) = serde_json::to_value(cfg) {
                    map.insert("clock_analog_config".to_string(), val);
                }
            }
            DisplayerConfig::ClockDigital(cfg) => {
                if let Ok(val) = serde_json::to_value(cfg) {
                    map.insert("clock_digital_config".to_string(), val);
                }
            }
            DisplayerConfig::Lcars(cfg) => {
                if let Ok(val) = serde_json::to_value(cfg) {
                    map.insert("lcars_config".to_string(), val);
                }
            }
            DisplayerConfig::CpuCores(cfg) => {
                if let Ok(val) = serde_json::to_value(cfg) {
                    map.insert("core_bars_config".to_string(), val);
                }
            }
            DisplayerConfig::Indicator(cfg) => {
                if let Ok(val) = serde_json::to_value(cfg) {
                    map.insert("indicator_config".to_string(), val);
                }
            }
        }
        map
    }

    /// Create a default DisplayerConfig for a given displayer type ID
    pub fn default_for_type(displayer_type: &str) -> Option<Self> {
        match displayer_type {
            "text" => Some(DisplayerConfig::Text(TextDisplayerConfig::default())),
            "bar" => Some(DisplayerConfig::Bar(BarDisplayConfig::default())),
            "arc" => Some(DisplayerConfig::Arc(ArcDisplayConfig::default())),
            "speedometer" => Some(DisplayerConfig::Speedometer(SpeedometerConfig::default())),
            "graph" => Some(DisplayerConfig::Graph(GraphDisplayConfig::default())),
            "clock_analog" => Some(DisplayerConfig::ClockAnalog(AnalogClockConfig::default())),
            "clock_digital" => Some(DisplayerConfig::ClockDigital(DigitalClockConfig::default())),
            "lcars" => Some(DisplayerConfig::Lcars(LcarsDisplayConfig::default())),
            "cpu_cores" => Some(DisplayerConfig::CpuCores(CoreBarsConfig::default())),
            "indicator" => Some(DisplayerConfig::Indicator(IndicatorConfig::default())),
            _ => None,
        }
    }
}

impl Default for DisplayerConfig {
    fn default() -> Self {
        DisplayerConfig::Text(TextDisplayerConfig::default())
    }
}

/// Panel appearance settings (background, border, corner radius)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelAppearance {
    pub background: BackgroundConfig,
    pub corner_radius: f64,
    pub border: PanelBorderConfig,
}

impl Default for PanelAppearance {
    fn default() -> Self {
        Self {
            background: BackgroundConfig::default(),
            corner_radius: 8.0,
            border: PanelBorderConfig::default(),
        }
    }
}

/// The unified panel data structure - single source of truth for all panel configuration.
///
/// This struct contains everything needed to:
/// 1. Create and configure a panel at runtime
/// 2. Serialize/deserialize to JSON for persistence
/// 3. Build the config dialog UI
/// 4. Update sources and displayers when config changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelData {
    /// Unique panel identifier
    pub id: String,

    /// Grid position and size
    pub geometry: PanelGeometry,

    /// Type-safe source configuration (includes source type in the enum variant)
    pub source_config: SourceConfig,

    /// Type-safe displayer configuration (includes displayer type in the enum variant)
    pub displayer_config: DisplayerConfig,

    /// Visual appearance (background, border, corners)
    pub appearance: PanelAppearance,
}

impl PanelData {
    /// Create a new PanelData with default configurations
    pub fn new(id: String, geometry: PanelGeometry) -> Self {
        Self {
            id,
            geometry,
            source_config: SourceConfig::default(),
            displayer_config: DisplayerConfig::default(),
            appearance: PanelAppearance::default(),
        }
    }

    /// Create PanelData with specific source and displayer types (using defaults for those types)
    pub fn with_types(
        id: String,
        geometry: PanelGeometry,
        source_type: &str,
        displayer_type: &str,
    ) -> Self {
        Self {
            id,
            geometry,
            source_config: SourceConfig::default_for_type(source_type)
                .unwrap_or_default(),
            displayer_config: DisplayerConfig::default_for_type(displayer_type)
                .unwrap_or_default(),
            appearance: PanelAppearance::default(),
        }
    }

    /// Get the source type ID string
    pub fn source_type(&self) -> &'static str {
        self.source_config.source_type()
    }

    /// Get the displayer type ID string
    pub fn displayer_type(&self) -> &'static str {
        self.displayer_config.displayer_type()
    }

    /// Convert source config to legacy HashMap format
    pub fn source_config_map(&self) -> HashMap<String, serde_json::Value> {
        self.source_config.to_hashmap()
    }

    /// Convert displayer config to legacy HashMap format
    pub fn displayer_config_map(&self) -> HashMap<String, serde_json::Value> {
        self.displayer_config.to_hashmap()
    }

    /// Get combined config map (source + displayer) for legacy interfaces
    pub fn combined_config_map(&self) -> HashMap<String, serde_json::Value> {
        let mut map = self.source_config.to_hashmap();
        map.extend(self.displayer_config.to_hashmap());
        map
    }
}

impl Default for PanelData {
    fn default() -> Self {
        Self::new("default".to_string(), PanelGeometry::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_config_serialization() {
        let config = SourceConfig::Cpu(CpuSourceConfig::default());
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"source_type\":\"cpu\""));

        let deserialized: SourceConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.source_type(), "cpu");
    }

    #[test]
    fn test_displayer_config_serialization() {
        let config = DisplayerConfig::Bar(BarDisplayConfig::default());
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"displayer_type\":\"bar\""));

        let deserialized: DisplayerConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.displayer_type(), "bar");
    }

    #[test]
    fn test_panel_data_serialization() {
        let data = PanelData::with_types(
            "test-panel".to_string(),
            PanelGeometry { x: 0, y: 0, width: 2, height: 1 },
            "cpu",
            "bar",
        );

        let json = serde_json::to_string_pretty(&data).unwrap();
        let deserialized: PanelData = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, "test-panel");
        assert_eq!(deserialized.source_type(), "cpu");
        assert_eq!(deserialized.displayer_type(), "bar");
    }

    #[test]
    fn test_to_hashmap_compatibility() {
        let config = SourceConfig::Cpu(CpuSourceConfig::default());
        let map = config.to_hashmap();

        assert!(map.contains_key("cpu_config"));
    }
}
