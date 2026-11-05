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

impl crate::schema::Schema for DiskSourceConfig {
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
                "disk_path",
                "Mount point",
                PropKind::DynamicChoice {
                    provider: "disk_mounts",
                },
            )
            .help("A mounted filesystem; the list is read once at startup"),
            Prop::new(
                "field",
                "Field",
                PropKind::Enum {
                    options: vec![
                        ("Used", "Used"),
                        ("Free", "Free"),
                        ("Total", "Total"),
                        ("Percent", "Percent used"),
                    ],
                },
            ),
            Prop::new(
                "disk_unit",
                "Unit",
                PropKind::Enum {
                    options: vec![("MB", "MB"), ("GB", "GB"), ("TB", "TB")],
                },
            )
            .visible_when(Condition::Equals {
                key: "field",
                values: vec!["Used", "Free", "Total"],
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
                .help("Derive the value range from the disk size"),
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
        check_props_fit::<DiskSourceConfig>(&DiskSourceConfig::props()).unwrap();
    }
}
