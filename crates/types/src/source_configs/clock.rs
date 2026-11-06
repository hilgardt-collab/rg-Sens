//! Clock source configuration types.

use serde::{Deserialize, Serialize};

/// Time format
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum TimeFormat {
    #[serde(rename = "24h")]
    #[default]
    Hour24,
    #[serde(rename = "12h")]
    Hour12,
}

/// Date format
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum DateFormat {
    #[serde(rename = "yyyy-mm-dd")]
    #[default]
    YearMonthDay,
    #[serde(rename = "dd/mm/yyyy")]
    DayMonthYear,
    #[serde(rename = "mm/dd/yyyy")]
    MonthDayYear,
    #[serde(rename = "day, month dd, yyyy")]
    LongFormat,
}

fn default_update_interval() -> u64 {
    100
}

fn default_timezone() -> String {
    "Local".to_string()
}

/// Clock source configuration
/// Note: Timer and alarm data is stored globally, not per-source.
/// Legacy alarm/timer fields are deserialized as raw JSON for migration only.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClockSourceConfig {
    #[serde(default = "default_update_interval")]
    pub update_interval_ms: u64,
    #[serde(default)]
    pub time_format: TimeFormat,
    #[serde(default)]
    pub date_format: DateFormat,
    #[serde(default)]
    pub show_seconds: bool,
    /// Timezone ID (e.g., "America/New_York", "Europe/London", "Local")
    #[serde(default = "default_timezone")]
    pub timezone: String,
    /// Legacy alarms field (for migration to global manager)
    #[serde(default, skip_serializing)]
    pub alarms: Vec<serde_json::Value>,
    /// Legacy timers field (for migration to global manager)
    #[serde(default, skip_serializing)]
    pub timers: Vec<serde_json::Value>,
}

impl Default for ClockSourceConfig {
    fn default() -> Self {
        Self {
            update_interval_ms: default_update_interval(),
            time_format: TimeFormat::Hour24,
            date_format: DateFormat::YearMonthDay,
            show_seconds: true,
            timezone: default_timezone(),
            alarms: Vec::new(),
            timers: Vec::new(),
        }
    }
}

impl crate::schema::Schema for ClockSourceConfig {
    fn props() -> Vec<crate::schema::Prop> {
        use crate::schema::{Prop, PropKind};
        vec![
            Prop::new(
                "time_format",
                "Time format",
                PropKind::Enum {
                    options: vec![("24h", "24 hour"), ("12h", "12 hour")],
                },
            ),
            Prop::new(
                "date_format",
                "Date format",
                PropKind::Enum {
                    options: vec![
                        ("yyyy-mm-dd", "YYYY-MM-DD"),
                        ("dd/mm/yyyy", "DD/MM/YYYY"),
                        ("mm/dd/yyyy", "MM/DD/YYYY"),
                        ("day, month dd, yyyy", "Day, Month DD, YYYY"),
                    ],
                },
            ),
            Prop::new("show_seconds", "Show seconds", PropKind::Bool),
            Prop::new(
                "timezone",
                "Timezone",
                PropKind::Custom { editor: "timezone" },
            )
            .help("IANA zone name, or Local"),
            Prop::new(
                "update_interval_ms",
                "Update interval (ms)",
                PropKind::Int {
                    min: 16,
                    max: 1000,
                    step: 10,
                },
            ),
            Prop::note("Tip: click the bell icon on the clock panel for alarm and timer controls."),
        ]
    }
}

#[cfg(test)]
mod schema_tests {
    use super::*;
    use crate::schema::{check_props_fit, Schema};

    #[test]
    fn props_fit_the_type() {
        check_props_fit::<ClockSourceConfig>(&ClockSourceConfig::props()).unwrap();
    }
}
