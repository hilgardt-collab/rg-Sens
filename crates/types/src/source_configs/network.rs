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

impl crate::schema::Schema for NetworkSourceConfig {
    fn props() -> Vec<crate::schema::Prop> {
        use crate::schema::{Condition, Prop, PropKind};
        vec![
            Prop::new(
                "custom_caption",
                "Caption",
                PropKind::OptionalText {
                    placeholder: "Auto-generated if empty",
                },
            ),
            Prop::new(
                "interface",
                "Interface",
                PropKind::DynamicChoice {
                    provider: "network_interfaces",
                },
            ),
            Prop::new(
                "field",
                "Field",
                PropKind::Enum {
                    options: vec![
                        ("DownloadSpeed", "Download speed"),
                        ("UploadSpeed", "Upload speed"),
                        ("TotalDownload", "Total downloaded"),
                        ("TotalUpload", "Total uploaded"),
                    ],
                },
            ),
            Prop::new(
                "speed_unit",
                "Speed unit",
                PropKind::Enum {
                    options: vec![
                        ("BytesPerSec", "B/s"),
                        ("KBPerSec", "KB/s"),
                        ("MBPerSec", "MB/s"),
                        ("GBPerSec", "GB/s"),
                    ],
                },
            )
            .visible_when(Condition::Equals {
                key: "field",
                values: vec!["DownloadSpeed", "UploadSpeed"],
            }),
            Prop::new(
                "total_unit",
                "Total unit",
                PropKind::Enum {
                    options: vec![("Bytes", "Bytes"), ("KB", "KB"), ("MB", "MB"), ("GB", "GB")],
                },
            )
            .visible_when(Condition::Equals {
                key: "field",
                values: vec!["TotalDownload", "TotalUpload"],
            }),
            Prop::new(
                "update_interval_ms",
                "Update interval (ms)",
                PropKind::Int {
                    min: 100,
                    max: 60_000,
                    step: 100,
                },
            ),
            Prop::new("auto_detect_limits", "Auto-detect limits", PropKind::Bool)
                .section("Limits")
                .help("Scale the value range to the highest rate seen"),
            Prop::new(
                "min_limit",
                "Minimum",
                PropKind::OptionalFloat {
                    min: 0.0,
                    max: 1.0e9,
                    step: 1.0,
                    digits: 1,
                },
            )
            .section("Limits")
            .visible_when(Condition::IsFalse {
                key: "auto_detect_limits",
            }),
            Prop::new(
                "max_limit",
                "Maximum",
                PropKind::OptionalFloat {
                    min: 0.0,
                    max: 1.0e9,
                    step: 1.0,
                    digits: 1,
                },
            )
            .section("Limits")
            .visible_when(Condition::IsFalse {
                key: "auto_detect_limits",
            }),
        ]
    }
}

#[cfg(test)]
mod schema_tests {
    use super::*;
    use crate::schema::{check_props_fit, Schema};

    #[test]
    fn props_fit_the_type() {
        check_props_fit::<NetworkSourceConfig>(&NetworkSourceConfig::props()).unwrap();
    }
}
