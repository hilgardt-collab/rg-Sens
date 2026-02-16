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
