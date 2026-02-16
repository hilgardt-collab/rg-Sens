//! Clock data source implementation
//!
//! Provides current time, date, alarm, and timer functionality.
//! Uses the global timer/alarm manager for timer and alarm state.

use crate::core::{
    global_timer_manager, DataSource, FieldMetadata, FieldPurpose, FieldType, SourceConfig,
    SourceMetadata, TimerState,
};
use anyhow::Result;
use chrono::{Datelike, Local, Timelike, Utc};
use chrono_tz::Tz;
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;

// Re-export types from core for backward compatibility
pub use crate::core::{AlarmConfig, TimerConfig};

// Re-export clock source config types from rg-sens-types
pub use rg_sens_types::source_configs::clock::{ClockSourceConfig, DateFormat, TimeFormat};

/// Clock data source
pub struct ClockSource {
    metadata: SourceMetadata,
    config: ClockSourceConfig,
    // Current time values
    hour: u32,
    minute: u32,
    second: u32,
    millisecond: u32,
    day: u32,
    month: u32,
    year: i32,
    day_of_week: u32,
    // Formatted strings
    time_string: String,
    date_string: String,
    day_name: String,
    month_name: String,

    /// Cached output values - updated in update(), returned by reference in values_ref()
    values: HashMap<String, Value>,
}

impl ClockSource {
    pub fn new() -> Self {
        Self {
            metadata: SourceMetadata {
                id: "clock".to_string(),
                name: "Clock".to_string(),
                description: "Current time, date, alarms, and timers".to_string(),
                available_keys: vec![
                    "hour".to_string(),
                    "minute".to_string(),
                    "second".to_string(),
                    "millisecond".to_string(),
                    "time".to_string(),
                    "date".to_string(),
                    "day".to_string(),
                    "month".to_string(),
                    "year".to_string(),
                    "day_of_week".to_string(),
                    "day_name".to_string(),
                    "month_name".to_string(),
                    "alarm_triggered".to_string(),
                    "alarm_enabled".to_string(),
                    "triggered_alarm_ids".to_string(),
                    "next_alarm_time".to_string(),
                    "timer_display".to_string(),
                    "timer_state".to_string(),
                    "timer_progress".to_string(),
                    "timezone".to_string(),
                ],
                default_interval: Duration::from_millis(100),
            },
            config: ClockSourceConfig::default(),
            hour: 0,
            minute: 0,
            second: 0,
            millisecond: 0,
            day: 1,
            month: 1,
            year: 2024,
            day_of_week: 0,
            time_string: String::new(),
            date_string: String::new(),
            day_name: String::new(),
            month_name: String::new(),
            values: HashMap::with_capacity(32),
        }
    }

    fn format_time(&self, hour: u32, minute: u32, second: u32) -> String {
        match self.config.time_format {
            TimeFormat::Hour24 => {
                if self.config.show_seconds {
                    format!("{:02}:{:02}:{:02}", hour, minute, second)
                } else {
                    format!("{:02}:{:02}", hour, minute)
                }
            }
            TimeFormat::Hour12 => {
                let (h12, ampm) = if hour == 0 {
                    (12, "AM")
                } else if hour < 12 {
                    (hour, "AM")
                } else if hour == 12 {
                    (12, "PM")
                } else {
                    (hour - 12, "PM")
                };
                if self.config.show_seconds {
                    format!("{:02}:{:02}:{:02} {}", h12, minute, second, ampm)
                } else {
                    format!("{:02}:{:02} {}", h12, minute, ampm)
                }
            }
        }
    }

    fn format_date(&self, year: i32, month: u32, day: u32) -> String {
        match self.config.date_format {
            DateFormat::YearMonthDay => format!("{}-{:02}-{:02}", year, month, day),
            DateFormat::DayMonthYear => format!("{:02}/{:02}/{}", day, month, year),
            DateFormat::MonthDayYear => format!("{:02}/{:02}/{}", month, day, year),
            DateFormat::LongFormat => {
                let month_name = Self::get_month_name(month);
                let day_name = Self::get_day_name(self.day_of_week);
                format!("{}, {} {}, {}", day_name, month_name, day, year)
            }
        }
    }

    fn get_day_name(day_of_week: u32) -> &'static str {
        match day_of_week {
            0 => "Sunday",
            1 => "Monday",
            2 => "Tuesday",
            3 => "Wednesday",
            4 => "Thursday",
            5 => "Friday",
            6 => "Saturday",
            _ => "Unknown",
        }
    }

    fn get_month_name(month: u32) -> &'static str {
        match month {
            1 => "January",
            2 => "February",
            3 => "March",
            4 => "April",
            5 => "May",
            6 => "June",
            7 => "July",
            8 => "August",
            9 => "September",
            10 => "October",
            11 => "November",
            12 => "December",
            _ => "Unknown",
        }
    }

    /// Get the current alarms from global manager
    pub fn get_alarms(&self) -> Vec<AlarmConfig> {
        if let Ok(manager) = global_timer_manager().read() {
            manager.alarms.clone()
        } else {
            Vec::new()
        }
    }

    /// Get the current timers from global manager
    pub fn get_timers(&self) -> Vec<TimerConfig> {
        if let Ok(manager) = global_timer_manager().read() {
            manager.timers.clone()
        } else {
            Vec::new()
        }
    }

    /// Check if any alarm is currently triggered
    pub fn any_alarm_triggered(&self) -> bool {
        if let Ok(manager) = global_timer_manager().read() {
            manager.any_alarm_triggered()
        } else {
            false
        }
    }

    /// Get IDs of all triggered alarms
    pub fn get_triggered_alarm_ids(&self) -> Vec<String> {
        if let Ok(manager) = global_timer_manager().read() {
            manager.triggered_alarms.iter().cloned().collect()
        } else {
            Vec::new()
        }
    }

    /// Dismiss a specific alarm by ID
    pub fn dismiss_alarm(&self, alarm_id: &str) {
        if let Ok(mut manager) = global_timer_manager().write() {
            manager.dismiss_alarm(alarm_id);
        }
    }

    /// Dismiss all triggered alarms
    pub fn dismiss_all_alarms(&self) {
        if let Ok(mut manager) = global_timer_manager().write() {
            manager.dismiss_all_alarms();
        }
    }

    /// Start a timer by ID
    pub fn start_timer(&self, timer_id: &str) {
        if let Ok(mut manager) = global_timer_manager().write() {
            manager.start_timer(timer_id);
        }
    }

    /// Pause a timer by ID
    pub fn pause_timer(&self, timer_id: &str) {
        if let Ok(mut manager) = global_timer_manager().write() {
            manager.pause_timer(timer_id);
        }
    }

    /// Resume a timer by ID
    pub fn resume_timer(&self, timer_id: &str) {
        if let Ok(mut manager) = global_timer_manager().write() {
            manager.resume_timer(timer_id);
        }
    }

    /// Stop and reset a timer by ID
    pub fn stop_timer(&self, timer_id: &str) {
        if let Ok(mut manager) = global_timer_manager().write() {
            manager.stop_timer(timer_id);
        }
    }

    /// Dismiss all finished timers
    pub fn dismiss_finished_timers(&self) {
        if let Ok(mut manager) = global_timer_manager().write() {
            manager.dismiss_finished_timers();
        }
    }
}

impl Default for ClockSource {
    fn default() -> Self {
        Self::new()
    }
}

impl DataSource for ClockSource {
    fn metadata(&self) -> &SourceMetadata {
        &self.metadata
    }

    fn fields(&self) -> Vec<FieldMetadata> {
        vec![
            FieldMetadata::new(
                "hour",
                "Hour",
                "Current hour (0-23)",
                FieldType::Numerical,
                FieldPurpose::Value,
            ),
            FieldMetadata::new(
                "minute",
                "Minute",
                "Current minute (0-59)",
                FieldType::Numerical,
                FieldPurpose::Value,
            ),
            FieldMetadata::new(
                "second",
                "Second",
                "Current second (0-59)",
                FieldType::Numerical,
                FieldPurpose::Value,
            ),
            FieldMetadata::new(
                "millisecond",
                "Millisecond",
                "Current millisecond (0-999)",
                FieldType::Numerical,
                FieldPurpose::Value,
            ),
            FieldMetadata::new(
                "time",
                "Time",
                "Formatted time string",
                FieldType::Text,
                FieldPurpose::Caption,
            ),
            FieldMetadata::new(
                "date",
                "Date",
                "Formatted date string",
                FieldType::Text,
                FieldPurpose::Caption,
            ),
            FieldMetadata::new(
                "day_name",
                "Day Name",
                "Name of the day of the week",
                FieldType::Text,
                FieldPurpose::Caption,
            ),
            FieldMetadata::new(
                "month_name",
                "Month Name",
                "Name of the month",
                FieldType::Text,
                FieldPurpose::Caption,
            ),
            FieldMetadata::new(
                "alarm_triggered",
                "Alarm Triggered",
                "Whether the alarm is currently ringing",
                FieldType::Boolean,
                FieldPurpose::Status,
            ),
            FieldMetadata::new(
                "timer_display",
                "Timer",
                "Timer display string",
                FieldType::Text,
                FieldPurpose::Caption,
            ),
            FieldMetadata::new(
                "timer_progress",
                "Timer Progress",
                "Timer progress (0.0 to 1.0)",
                FieldType::Numerical,
                FieldPurpose::Value,
            ),
            FieldMetadata::new(
                "value",
                "Day Progress",
                "Percentage of 24-hour period elapsed (0.0 at midnight, 1.0 at end of day)",
                FieldType::Numerical,
                FieldPurpose::Value,
            ),
            FieldMetadata::new(
                "timezone",
                "Time Zone",
                "Configured timezone (e.g., 'Local', 'America/New_York')",
                FieldType::Text,
                FieldPurpose::Caption,
            ),
        ]
    }

    fn update(&mut self) -> Result<()> {
        // Get time in the configured timezone
        let (hour, minute, second, millisecond, day, month, year, day_of_week) =
            if self.config.timezone == "Local" {
                let now = Local::now();
                (
                    now.hour(),
                    now.minute(),
                    now.second(),
                    now.timestamp_subsec_millis(),
                    now.day(),
                    now.month(),
                    now.year(),
                    now.weekday().num_days_from_sunday(),
                )
            } else if let Ok(tz) = self.config.timezone.parse::<Tz>() {
                let now = Utc::now().with_timezone(&tz);
                (
                    now.hour(),
                    now.minute(),
                    now.second(),
                    now.timestamp_subsec_millis(),
                    now.day(),
                    now.month(),
                    now.year(),
                    now.weekday().num_days_from_sunday(),
                )
            } else {
                // Fallback to local time if timezone parsing fails
                let now = Local::now();
                (
                    now.hour(),
                    now.minute(),
                    now.second(),
                    now.timestamp_subsec_millis(),
                    now.day(),
                    now.month(),
                    now.year(),
                    now.weekday().num_days_from_sunday(),
                )
            };

        self.hour = hour;
        self.minute = minute;
        self.second = second;
        self.millisecond = millisecond;
        self.day = day;
        self.month = month;
        self.year = year;
        self.day_of_week = day_of_week;

        self.time_string = self.format_time(self.hour, self.minute, self.second);
        self.date_string = self.format_date(self.year, self.month, self.day);
        self.day_name = Self::get_day_name(self.day_of_week).to_string();
        self.month_name = Self::get_month_name(self.month).to_string();

        // Update global timer manager with current time
        if let Ok(mut manager) = global_timer_manager().write() {
            manager.update(self.hour, self.minute, self.second, self.day_of_week);
        }

        // Build values HashMap (reuse allocation, just clear and refill)
        self.values.clear();

        // Time components
        self.values
            .insert("hour".to_string(), Value::from(self.hour));
        self.values
            .insert("minute".to_string(), Value::from(self.minute));
        self.values
            .insert("second".to_string(), Value::from(self.second));
        self.values
            .insert("millisecond".to_string(), Value::from(self.millisecond));

        // Formatted strings
        self.values
            .insert("time".to_string(), Value::from(self.time_string.clone()));
        self.values
            .insert("date".to_string(), Value::from(self.date_string.clone()));
        self.values
            .insert("day_name".to_string(), Value::from(self.day_name.clone()));
        self.values.insert(
            "month_name".to_string(),
            Value::from(self.month_name.clone()),
        );

        // Date components
        self.values.insert("day".to_string(), Value::from(self.day));
        self.values
            .insert("month".to_string(), Value::from(self.month));
        self.values
            .insert("year".to_string(), Value::from(self.year));
        self.values
            .insert("day_of_week".to_string(), Value::from(self.day_of_week));

        // Get alarm/timer info from global manager
        if let Ok(manager) = global_timer_manager().read() {
            // Alarm state
            let any_triggered = manager.any_alarm_triggered();
            self.values
                .insert("alarm_triggered".to_string(), Value::from(any_triggered));
            let any_alarm_enabled = manager.alarms.iter().any(|a| a.enabled);
            self.values
                .insert("alarm_enabled".to_string(), Value::from(any_alarm_enabled));

            // Next alarm info
            if let Some(ref next_time) = manager.next_alarm_time {
                self.values.insert(
                    "next_alarm_time".to_string(),
                    Value::from(next_time.clone()),
                );
            }

            // Triggered alarm IDs
            let triggered_ids: Vec<_> = manager.triggered_alarms.iter().cloned().collect();
            self.values.insert(
                "triggered_alarm_ids".to_string(),
                serde_json::to_value(&triggered_ids).unwrap_or(Value::Array(vec![])),
            );

            // Timer info
            if let Some(timer) = manager.get_display_timer() {
                self.values.insert(
                    "timer_display".to_string(),
                    Value::from(timer.display_string()),
                );
                self.values
                    .insert("timer_progress".to_string(), Value::from(timer.progress()));

                let timer_state = match timer.state {
                    TimerState::Stopped => "stopped",
                    TimerState::Running => "running",
                    TimerState::Paused => "paused",
                    TimerState::Finished => "finished",
                };
                self.values
                    .insert("timer_state".to_string(), Value::from(timer_state));
            } else {
                self.values
                    .insert("timer_display".to_string(), Value::from(""));
                self.values
                    .insert("timer_progress".to_string(), Value::from(0.0));
                self.values
                    .insert("timer_state".to_string(), Value::from("stopped"));
            }

            // Expose all alarms and timers for UI
            self.values.insert(
                "alarms".to_string(),
                serde_json::to_value(&manager.alarms).unwrap_or(Value::Array(vec![])),
            );
            self.values.insert(
                "timers".to_string(),
                serde_json::to_value(&manager.timers).unwrap_or(Value::Array(vec![])),
            );

            // Check if needs attention (for visual cue)
            self.values.insert(
                "needs_attention".to_string(),
                Value::from(manager.needs_attention()),
            );
        } else {
            // Fallback values if manager lock fails
            self.values
                .insert("alarm_triggered".to_string(), Value::from(false));
            self.values
                .insert("alarm_enabled".to_string(), Value::from(false));
            self.values
                .insert("timer_display".to_string(), Value::from(""));
            self.values
                .insert("timer_progress".to_string(), Value::from(0.0));
            self.values
                .insert("timer_state".to_string(), Value::from("stopped"));
            self.values
                .insert("needs_attention".to_string(), Value::from(false));
        }

        // Normalized value for analog displays (based on 12-hour clock)
        let hour_12 = (self.hour % 12) as f64;
        let minute_frac = self.minute as f64 / 60.0;
        let second_frac = self.second as f64 / 60.0;
        let ms_frac = self.millisecond as f64 / 1000.0;

        // Hour hand position (0-1 for full rotation)
        let hour_value = (hour_12 + minute_frac) / 12.0;
        self.values
            .insert("hour_value".to_string(), Value::from(hour_value));

        // Minute hand position (0-1 for full rotation)
        let minute_value = (self.minute as f64 + second_frac) / 60.0;
        self.values
            .insert("minute_value".to_string(), Value::from(minute_value));

        // Second hand position (0-1 for full rotation, with millisecond smoothing)
        let second_value = (self.second as f64 + ms_frac) / 60.0;
        self.values
            .insert("second_value".to_string(), Value::from(second_value));

        // Day progress value (0-1 representing percentage of 24-hour period elapsed)
        // 24 hours = 86400 seconds
        let total_seconds =
            self.hour as f64 * 3600.0 + self.minute as f64 * 60.0 + self.second as f64 + ms_frac;
        let day_progress = total_seconds / 86400.0;
        self.values
            .insert("value".to_string(), Value::from(day_progress));

        // Caption for text displays
        self.values
            .insert("caption".to_string(), Value::from(self.time_string.clone()));
        self.values.insert("unit".to_string(), Value::from(""));

        // Timezone
        self.values.insert(
            "timezone".to_string(),
            Value::from(self.config.timezone.clone()),
        );

        Ok(())
    }

    fn get_values(&self) -> HashMap<String, Value> {
        self.values.clone()
    }

    fn values_ref(&self) -> Option<&HashMap<String, Value>> {
        Some(&self.values)
    }

    fn configure(&mut self, config: &HashMap<String, Value>) -> Result<()> {
        if let Some(config_value) = config.get("clock_config") {
            let new_config: ClockSourceConfig = serde_json::from_value(config_value.clone())?;

            // Migrate any legacy alarms/timers to global manager
            // (legacy fields are now serde_json::Value, deserialize to concrete types)
            if !new_config.alarms.is_empty() || !new_config.timers.is_empty() {
                if let Ok(mut manager) = global_timer_manager().write() {
                    // Only add if global manager is empty (first load)
                    if manager.alarms.is_empty() && manager.timers.is_empty() {
                        let timers: Vec<TimerConfig> = new_config
                            .timers
                            .iter()
                            .filter_map(|v| serde_json::from_value(v.clone()).ok())
                            .collect();
                        let alarms: Vec<AlarmConfig> = new_config
                            .alarms
                            .iter()
                            .filter_map(|v| serde_json::from_value(v.clone()).ok())
                            .collect();
                        manager.load_config(timers, alarms);
                    }
                }
            }

            self.config = new_config;
        }

        // Handle timer commands with ID
        if let Some(timer_id) = config.get("timer_id").and_then(|v| v.as_str()) {
            if let Some(cmd) = config.get("timer_command").and_then(|v| v.as_str()) {
                match cmd {
                    "start" => self.start_timer(timer_id),
                    "pause" => self.pause_timer(timer_id),
                    "resume" => self.resume_timer(timer_id),
                    "stop" => self.stop_timer(timer_id),
                    _ => {}
                }
            }
        }

        // Handle alarm dismiss by ID
        if let Some(alarm_id) = config.get("dismiss_alarm_id").and_then(|v| v.as_str()) {
            self.dismiss_alarm(alarm_id);
        }

        // Handle dismiss all alarms
        if let Some(dismiss) = config.get("dismiss_all_alarms") {
            if dismiss.as_bool().unwrap_or(false) {
                self.dismiss_all_alarms();
            }
        }

        // Handle dismiss finished timers
        if let Some(dismiss) = config.get("dismiss_finished_timers") {
            if dismiss.as_bool().unwrap_or(false) {
                self.dismiss_finished_timers();
            }
        }

        Ok(())
    }

    fn get_typed_config(&self) -> Option<SourceConfig> {
        Some(SourceConfig::Clock(self.config.clone()))
    }
}
