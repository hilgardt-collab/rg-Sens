//! Network source configuration types.

use serde::{Deserialize, Serialize};

/// Network source field types
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub enum NetworkField {
    #[default]
    DownloadSpeed,
    UploadSpeed,
    TotalDownload,
    TotalUpload,
}

/// Network speed unit types
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub enum NetworkSpeedUnit {
    BytesPerSec,
    #[default]
    KBPerSec,
    MBPerSec,
    GBPerSec,
}

/// Network total data unit types
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub enum NetworkTotalUnit {
    Bytes,
    KB,
    #[default]
    MB,
    GB,
}

fn default_update_interval() -> u64 {
    1000
}

fn default_auto_detect_limits() -> bool {
    true
}

/// Network source configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSourceConfig {
    pub field: NetworkField,
    #[serde(default)]
    pub speed_unit: NetworkSpeedUnit,
    #[serde(default)]
    pub total_unit: NetworkTotalUnit,
    pub interface: String,
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

impl Default for NetworkSourceConfig {
    fn default() -> Self {
        Self {
            field: NetworkField::DownloadSpeed,
            speed_unit: NetworkSpeedUnit::KBPerSec,
            total_unit: NetworkTotalUnit::MB,
            interface: "".to_string(),
            custom_caption: None,
            update_interval_ms: default_update_interval(),
            min_limit: None,
            max_limit: Some(100.0),
            auto_detect_limits: default_auto_detect_limits(),
        }
    }
}
