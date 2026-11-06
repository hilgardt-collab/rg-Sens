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

impl crate::schema::Schema for GpuSourceConfig {
    fn props() -> Vec<crate::schema::Prop> {
        use crate::schema::{Condition, Prop, PropKind};
        vec![
            Prop::new(
                "gpu_index",
                "GPU",
                PropKind::IndexChoice { provider: "gpus" },
            ),
            Prop::new(
                "field",
                "Field",
                PropKind::Enum {
                    options: vec![
                        ("Temperature", "Temperature"),
                        ("Utilization", "Utilization (%)"),
                        ("MemoryUsed", "Memory used"),
                        ("MemoryTotal", "Memory total"),
                        ("MemoryPercent", "Memory used (%)"),
                        ("PowerUsage", "Power usage (W)"),
                        ("FanSpeed", "Fan speed (%)"),
                        ("ClockCore", "Core clock"),
                        ("ClockMemory", "Memory clock"),
                    ],
                },
            ),
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
                "memory_unit",
                "Memory unit",
                PropKind::Enum {
                    options: vec![("MB", "MB"), ("GB", "GB")],
                },
            )
            .visible_when(Condition::Equals {
                key: "field",
                values: vec!["MemoryUsed", "MemoryTotal"],
            }),
            Prop::new(
                "frequency_unit",
                "Frequency unit",
                PropKind::Enum {
                    options: vec![("MHz", "MHz"), ("GHz", "GHz")],
                },
            )
            .visible_when(Condition::Equals {
                key: "field",
                values: vec!["ClockCore", "ClockMemory"],
            }),
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
                    max: 100000.0,
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
                    max: 100000.0,
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
        check_props_fit::<GpuSourceConfig>(&GpuSourceConfig::props()).unwrap();
    }
}
