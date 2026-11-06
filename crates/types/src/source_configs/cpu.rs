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
            // The displayers map a value without limits to 0 once it passes
            // 100, so a fixed range is the usable default while auto-detect
            // is off (the old config widgets always saved these).
            min_limit: Some(0.0),
            max_limit: Some(100.0),
            auto_detect_limits: default_auto_detect_limits(),
        }
    }
}

impl crate::schema::Schema for CpuSourceConfig {
    fn props() -> Vec<crate::schema::Prop> {
        use crate::schema::{Condition, Prop, PropKind};
        vec![
            Prop::new(
                "field",
                "Field",
                PropKind::Enum {
                    options: vec![
                        ("Usage", "Usage (%)"),
                        ("Temperature", "Temperature"),
                        ("Frequency", "Frequency"),
                    ],
                },
            ),
            Prop::new(
                "sensor_index",
                "Temperature sensor",
                PropKind::IndexChoice {
                    provider: "cpu_sensors",
                },
            )
            .visible_when(Condition::Equals {
                key: "field",
                values: vec!["Temperature"],
            }),
            Prop::new(
                "temp_unit",
                "Temperature unit",
                PropKind::Enum {
                    options: vec![
                        ("Celsius", "Celsius (°C)"),
                        ("Fahrenheit", "Fahrenheit (°F)"),
                        ("Kelvin", "Kelvin (K)"),
                    ],
                },
            )
            .visible_when(Condition::Equals {
                key: "field",
                values: vec!["Temperature"],
            }),
            Prop::new(
                "freq_unit",
                "Frequency unit",
                PropKind::Enum {
                    options: vec![("MHz", "MHz"), ("GHz", "GHz")],
                },
            )
            .visible_when(Condition::Equals {
                key: "field",
                values: vec!["Frequency"],
            }),
            Prop::new(
                "core_selection",
                "Core",
                PropKind::Custom {
                    editor: "cpu_core_selection",
                },
            )
            .help("Overall, or one core's usage / frequency"),
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
                    min: -1000.0,
                    max: 10000.0,
                    step: 1.0,
                    digits: 2,
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
                    min: -1000.0,
                    max: 10000.0,
                    step: 1.0,
                    digits: 2,
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
        check_props_fit::<CpuSourceConfig>(&CpuSourceConfig::props()).unwrap();
    }
}
