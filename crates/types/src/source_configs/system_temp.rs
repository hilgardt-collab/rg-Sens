//! System temperature source configuration types.
//!
//! Note: This module's TemperatureUnit uses lowercase serde names
//! ("celsius", "fahrenheit", "kelvin"), distinct from cpu::TemperatureUnit.

use serde::{Deserialize, Serialize};

/// Temperature unit for display (with lowercase serde names)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum TemperatureUnit {
    #[serde(rename = "celsius")]
    #[default]
    Celsius,
    #[serde(rename = "fahrenheit")]
    Fahrenheit,
    #[serde(rename = "kelvin")]
    Kelvin,
}

/// Category of temperature sensor
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum SensorCategory {
    #[serde(rename = "cpu")]
    CPU,
    #[serde(rename = "gpu")]
    GPU,
    #[serde(rename = "motherboard")]
    Motherboard,
    #[serde(rename = "storage")]
    Storage,
    #[serde(rename = "other")]
    Other,
}

/// Information about a discovered temperature sensor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorInfo {
    pub index: usize,
    pub label: String,
    pub category: SensorCategory,
}

fn default_update_interval() -> u64 {
    1000
}

fn default_auto_detect_limits() -> bool {
    false
}

/// Configuration for system temperature source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemTempConfig {
    /// Sensor label - stable identifier (preferred over index)
    #[serde(default)]
    pub sensor_label: Option<String>,
    /// Sensor index - deprecated, kept for backward compatibility
    #[serde(default)]
    pub sensor_index: usize,
    #[serde(default)]
    pub temp_unit: TemperatureUnit,
    #[serde(default = "default_update_interval")]
    pub update_interval_ms: u64,
    #[serde(default)]
    pub custom_caption: Option<String>,
    #[serde(default)]
    pub min_limit: Option<f64>,
    #[serde(default)]
    pub max_limit: Option<f64>,
    #[serde(default = "default_auto_detect_limits")]
    pub auto_detect_limits: bool,
}

impl Default for SystemTempConfig {
    fn default() -> Self {
        Self {
            sensor_label: None,
            sensor_index: 0,
            temp_unit: TemperatureUnit::Celsius,
            update_interval_ms: default_update_interval(),
            custom_caption: None,
            min_limit: None,
            max_limit: None,
            auto_detect_limits: default_auto_detect_limits(),
        }
    }
}
