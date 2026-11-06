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
    /// Unique, human-readable label; this is the sensor's identity in saved configs
    pub label: String,
    pub category: SensorCategory,
    /// Label before collision disambiguation (equal to `label` unless another
    /// sensor shared it). Matches configs saved before disambiguation existed.
    #[serde(skip)]
    pub base_label: String,
    /// Path to the `tempN_input` file (None when read through sysinfo)
    #[serde(skip)]
    pub path: Option<std::path::PathBuf>,
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
    /// Sensor label - the stable identifier. A set label that matches no
    /// sensor means "not found"; the index is never used as a substitute.
    #[serde(default)]
    pub sensor_label: Option<String>,
    /// Sensor index - only consulted for configs saved before labels existed
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
            // The displayers map a value without limits to 0 once it passes
            // 100, so a fixed range is the usable default while auto-detect
            // is off (the old config widgets always saved these).
            min_limit: Some(0.0),
            max_limit: Some(100.0),
            auto_detect_limits: default_auto_detect_limits(),
        }
    }
}

impl crate::schema::Schema for SystemTempConfig {
    fn props() -> Vec<crate::schema::Prop> {
        use crate::schema::{Condition, Prop, PropKind};
        vec![
            Prop::new(
                "sensor_label",
                "Temperature sensor",
                PropKind::Custom {
                    editor: "temp_sensor",
                },
            )
            .help("The sensor's label is its identity; a saved sensor that is missing shows N/A"),
            Prop::new(
                "temp_unit",
                "Unit",
                PropKind::Enum {
                    options: vec![
                        ("celsius", "Celsius (°C)"),
                        ("fahrenheit", "Fahrenheit (°F)"),
                        ("kelvin", "Kelvin (K)"),
                    ],
                },
            ),
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
                    min: -50.0,
                    max: 200.0,
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
                    min: -50.0,
                    max: 200.0,
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
        check_props_fit::<SystemTempConfig>(&SystemTempConfig::props()).unwrap();
    }
}
