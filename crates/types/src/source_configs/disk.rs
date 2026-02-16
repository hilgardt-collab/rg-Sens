//! Disk source configuration types.

use serde::{Deserialize, Serialize};

/// Disk source field types
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum DiskField {
    Used,
    Free,
    Total,
    Percent,
}

/// Disk capacity unit types
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub enum DiskUnit {
    MB,
    #[default]
    GB,
    TB,
}

fn default_update_interval() -> u64 {
    2000
}

fn default_auto_detect_limits() -> bool {
    false
}

/// Disk source configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskSourceConfig {
    pub field: DiskField,
    #[serde(default)]
    pub disk_unit: DiskUnit,
    pub disk_path: String,
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

impl Default for DiskSourceConfig {
    fn default() -> Self {
        Self {
            field: DiskField::Percent,
            disk_unit: DiskUnit::GB,
            disk_path: "/".to_string(),
            custom_caption: None,
            update_interval_ms: default_update_interval(),
            min_limit: None,
            max_limit: None,
            auto_detect_limits: default_auto_detect_limits(),
        }
    }
}
