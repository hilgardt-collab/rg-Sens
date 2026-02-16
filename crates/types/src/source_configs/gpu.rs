//! GPU source configuration types.

use serde::{Deserialize, Serialize};

use super::cpu::TemperatureUnit;

/// GPU source field types
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum GpuField {
    Temperature,
    Utilization,
    MemoryUsed,
    MemoryTotal,
    MemoryPercent,
    PowerUsage,
    FanSpeed,
    ClockCore,
    ClockMemory,
}

/// Memory unit types (shared with memory source)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub enum MemoryUnit {
    MB,
    #[default]
    GB,
}

/// GPU frequency unit types
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub enum FrequencyUnit {
    #[default]
    MHz,
    GHz,
}

fn default_update_interval() -> u64 {
    1000
}

fn default_auto_detect_limits() -> bool {
    false
}

/// GPU source configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuSourceConfig {
    pub field: GpuField,
    pub temp_unit: TemperatureUnit,
    #[serde(default)]
    pub memory_unit: MemoryUnit,
    #[serde(default)]
    pub frequency_unit: FrequencyUnit,
    pub gpu_index: u32,
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

impl Default for GpuSourceConfig {
    fn default() -> Self {
        Self {
            field: GpuField::Temperature,
            temp_unit: TemperatureUnit::Celsius,
            memory_unit: MemoryUnit::GB,
            frequency_unit: FrequencyUnit::MHz,
            gpu_index: 0,
            custom_caption: None,
            update_interval_ms: default_update_interval(),
            min_limit: None,
            max_limit: None,
            auto_detect_limits: default_auto_detect_limits(),
        }
    }
}
