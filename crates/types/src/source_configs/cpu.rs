//! CPU source configuration types.

use serde::{Deserialize, Serialize};

/// CPU source field types
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum CpuField {
    Temperature,
    Usage,
    Frequency,
}

/// Temperature units
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TemperatureUnit {
    Celsius,
    Fahrenheit,
    Kelvin,
}

/// Frequency units
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub enum FrequencyUnit {
    #[default]
    MHz,
    GHz,
}

/// CPU core selection
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CoreSelection {
    Overall,
    Core(usize),
}

fn default_update_interval() -> u64 {
    1000
}

fn default_auto_detect_limits() -> bool {
    false
}

/// CPU source configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuSourceConfig {
    pub field: CpuField,
    pub temp_unit: TemperatureUnit,
    #[serde(default)]
    pub freq_unit: FrequencyUnit,
    pub sensor_index: usize,
    pub core_selection: CoreSelection,
    #[serde(default)]
    pub custom_caption: Option<String>,
    #[serde(default = "default_update_interval")]
    pub update_interval_ms: u64,
    #[serde(default)]
    pub min_limit: Option<f64>,
    #[serde(default)]
    pub max_limit: Option<f64>,
    #[serde(default = "default_auto_detect_limits")]
    pub auto_detect_limits: bool,
}

impl Default for CpuSourceConfig {
    fn default() -> Self {
        Self {
            field: CpuField::Usage,
            temp_unit: TemperatureUnit::Celsius,
            freq_unit: FrequencyUnit::MHz,
            sensor_index: 0,
            core_selection: CoreSelection::Overall,
            custom_caption: None,
            update_interval_ms: default_update_interval(),
            min_limit: None,
            max_limit: None,
            auto_detect_limits: default_auto_detect_limits(),
        }
    }
}
