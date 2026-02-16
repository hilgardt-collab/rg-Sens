//! Fan speed source configuration types.

use serde::{Deserialize, Serialize};

/// Category of fan sensor
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum FanCategory {
    #[serde(rename = "cpu")]
    CPU,
    #[serde(rename = "gpu")]
    GPU,
    #[serde(rename = "chassis")]
    Chassis,
    #[serde(rename = "psu")]
    PSU,
    #[serde(rename = "other")]
    Other,
}

/// Information about a discovered fan sensor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FanInfo {
    pub index: usize,
    pub label: String,
    pub category: FanCategory,
    /// Path to the fan input file (e.g., /sys/class/hwmon/hwmon0/fan1_input)
    #[serde(skip)]
    pub path: Option<std::path::PathBuf>,
}

fn default_update_interval() -> u64 {
    1000
}

fn default_auto_detect_limits() -> bool {
    false
}

/// Configuration for fan speed source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FanSpeedConfig {
    #[serde(default)]
    pub sensor_index: usize,
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

impl Default for FanSpeedConfig {
    fn default() -> Self {
        Self {
            sensor_index: 0,
            update_interval_ms: default_update_interval(),
            custom_caption: None,
            min_limit: None,
            max_limit: None,
            auto_detect_limits: default_auto_detect_limits(),
        }
    }
}
