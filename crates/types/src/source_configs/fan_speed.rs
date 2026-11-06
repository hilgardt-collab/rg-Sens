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
    /// Unique, human-readable label; this is the fan's identity in saved configs
    pub label: String,
    pub category: FanCategory,
    /// Label before collision disambiguation (equal to `label` unless another
    /// fan shared it). Matches configs saved before disambiguation existed.
    #[serde(skip)]
    pub base_label: String,
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
    /// Fan label - the stable identifier (hwmon ordering changes across
    /// boots). A set label that matches no fan means "not found"; the index
    /// is never used as a substitute.
    #[serde(default)]
    pub sensor_label: Option<String>,
    /// Fan index - only consulted for configs saved before labels existed,
    /// interpreted against the raw discovery order those configs were saved with
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
            sensor_label: None,
            sensor_index: 0,
            update_interval_ms: default_update_interval(),
            custom_caption: None,
            // The displayers map a value without limits to 0 once it passes
            // 100, so a fixed range is the usable default while auto-detect
            // is off (the old config widgets always saved these).
            min_limit: Some(0.0),
            max_limit: Some(3000.0),
            auto_detect_limits: default_auto_detect_limits(),
        }
    }
}

impl crate::schema::Schema for FanSpeedConfig {
    fn props() -> Vec<crate::schema::Prop> {
        use crate::schema::{Condition, Prop, PropKind};
        vec![
            Prop::new(
                "sensor_label",
                "Fan sensor",
                PropKind::Custom {
                    editor: "fan_sensor",
                },
            )
            .help("The fan's label is its identity; a saved fan that is missing shows N/A"),
            Prop::new(
                "custom_caption",
                "Caption",
                PropKind::OptionalText {
                    placeholder: "Auto-generated if empty",
                },
            ),
            Prop::new(
                "update_interval_ms",
                "Update interval (ms)",
                PropKind::Int {
                    min: 100,
                    max: 60000,
                    step: 100,
                },
            ),
            Prop::new("auto_detect_limits", "Auto-detect limits", PropKind::Bool)
                .section("Limits")
                .help("Track the observed range instead of fixed limits"),
            Prop::new(
                "min_limit",
                "Minimum",
                PropKind::OptionalFloat {
                    min: 0.0,
                    max: 10000.0,
                    step: 50.0,
                    digits: 0,
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
                    max: 10000.0,
                    step: 50.0,
                    digits: 0,
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
        check_props_fit::<FanSpeedConfig>(&FanSpeedConfig::props()).unwrap();
    }
}
