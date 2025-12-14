//! Clock data source implementation
//!
//! Provides current time, date, alarm, and timer functionality.
//! Supports multiple concurrent alarms and timers with individual sound configuration.

use crate::audio::{AlarmSoundConfig, AudioPlayer};
use crate::core::{DataSource, FieldMetadata, FieldPurpose, FieldType, SourceMetadata};
use anyhow::Result;
use chrono::{Local, Timelike, Datelike, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};
use uuid::Uuid;

/// Alarm configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AlarmConfig {
    /// Unique identifier for this alarm
    #[serde(default = "generate_uuid")]
    pub id: String,
    pub enabled: bool,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
    /// Days of week the alarm is active (0=Sunday, 6=Saturday)
    /// Empty = every day, otherwise specific days
    pub days: Vec<u32>,
    /// Custom alarm label
    pub label: Option<String>,
    /// Sound configuration for this alarm
    #[serde(default)]
    pub sound: AlarmSoundConfig,
}

fn generate_uuid() -> String {
    Uuid::new_v4().to_string()
}

impl Default for AlarmConfig {
    fn default() -> Self {
        Self {
            id: generate_uuid(),
            enabled: false,
            hour: 7,
            minute: 0,
            second: 0,
            days: vec![1, 2, 3, 4, 5], // Weekdays by default
            label: None,
            sound: AlarmSoundConfig::default(),
        }
    }
}

impl AlarmConfig {
    /// Create a new alarm with a unique ID
    pub fn new() -> Self {
        Self::default()
    }

    /// Create an alarm with specific time
    pub fn with_time(hour: u32, minute: u32, second: u32) -> Self {
        Self {
            hour,
            minute,
            second,
            enabled: true,
            ..Self::default()
        }
    }
}

/// Timer mode
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum TimerMode {
    #[serde(rename = "countdown")]
    #[default]
    Countdown,
    #[serde(rename = "stopwatch")]
    Stopwatch,
}

/// Timer state
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum TimerState {
    #[serde(rename = "stopped")]
    #[default]
    Stopped,
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "paused")]
    Paused,
    #[serde(rename = "finished")]
    Finished,
}

/// Timer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimerConfig {
    /// Unique identifier for this timer
    #[serde(default = "generate_uuid")]
    pub id: String,
    pub mode: TimerMode,
    /// Countdown duration in seconds
    pub countdown_duration: u64,
    /// Current state
    #[serde(skip)]
    pub state: TimerState,
    /// Elapsed time in milliseconds (for stopwatch) or remaining time (for countdown)
    #[serde(skip)]
    pub elapsed_ms: u64,
    /// When the timer was last started/resumed
    #[serde(skip)]
    pub start_instant: Option<Instant>,
    /// Custom timer label
    pub label: Option<String>,
    /// Sound configuration for timer
    #[serde(default)]
    pub sound: AlarmSoundConfig,
}

impl Default for TimerConfig {
    fn default() -> Self {
        Self {
            id: generate_uuid(),
            mode: TimerMode::Countdown,
            countdown_duration: 300, // 5 minutes
            state: TimerState::Stopped,
            elapsed_ms: 0,
            start_instant: None,
            label: None,
            sound: AlarmSoundConfig::default(),
        }
    }
}

impl TimerConfig {
    /// Create a new timer with a unique ID
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a countdown timer with specific duration
    pub fn countdown(duration_secs: u64) -> Self {
        Self {
            mode: TimerMode::Countdown,
            countdown_duration: duration_secs,
            ..Self::default()
        }
    }

    /// Create a stopwatch timer
    pub fn stopwatch() -> Self {
        Self {
            mode: TimerMode::Stopwatch,
            ..Self::default()
        }
    }
}

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

/// Clock source configuration
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
    /// Multiple alarms (replaces single alarm)
    #[serde(default)]
    pub alarms: Vec<AlarmConfig>,
    /// Multiple timers (replaces single timer)
    #[serde(default)]
    pub timers: Vec<TimerConfig>,
    /// Legacy single alarm (for backward compatibility, migrated to alarms)
    #[serde(default, skip_serializing)]
    alarm: Option<AlarmConfig>,
    /// Legacy single timer (for backward compatibility, migrated to timers)
    #[serde(default, skip_serializing)]
    timer: Option<TimerConfig>,
}

fn default_timezone() -> String {
    "Local".to_string()
}

fn default_update_interval() -> u64 {
    100 // 100ms for smooth second hand movement
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
            alarm: None,
            timer: None,
        }
    }
}

impl ClockSourceConfig {
    /// Migrate legacy single alarm/timer to vectors
    pub fn migrate_legacy(&mut self) {
        // Migrate single alarm to alarms vector
        if let Some(mut legacy_alarm) = self.alarm.take() {
            if legacy_alarm.enabled || legacy_alarm.hour != 7 || legacy_alarm.minute != 0 {
                // Only migrate if it was actually configured
                if legacy_alarm.id.is_empty() {
                    legacy_alarm.id = generate_uuid();
                }
                self.alarms.push(legacy_alarm);
            }
        }

        // Migrate single timer to timers vector
        if let Some(mut legacy_timer) = self.timer.take() {
            if legacy_timer.countdown_duration != 300 || legacy_timer.label.is_some() {
                // Only migrate if it was actually configured
                if legacy_timer.id.is_empty() {
                    legacy_timer.id = generate_uuid();
                }
                self.timers.push(legacy_timer);
            }
        }
    }
}

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
    // Alarm state - IDs of currently triggered alarms
    triggered_alarms: HashSet<String>,
    // Alarms that have already played their sound this trigger cycle
    alarm_sound_played: HashSet<String>,
    // Last check time for each alarm (to prevent re-triggering within same second)
    last_alarm_check: HashMap<String, (u32, u32, u32)>,
    // Timers that have already played their sound
    timer_sound_played: HashSet<String>,
    // Timer display for the "next" timer
    timer_display: String,
    timer_progress: f64,
    // Next alarm info for display
    next_alarm_time: Option<String>,
    next_alarm_id: Option<String>,
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
            triggered_alarms: HashSet::new(),
            alarm_sound_played: HashSet::new(),
            last_alarm_check: HashMap::new(),
            timer_sound_played: HashSet::new(),
            timer_display: String::new(),
            timer_progress: 0.0,
            next_alarm_time: None,
            next_alarm_id: None,
        }
    }

    /// Play alarm/timer sound using spawn-and-forget
    /// Note: Sounds play to completion and cannot be stopped mid-play
    fn play_sound_for_id(&self, _id: &str, sound_config: &AlarmSoundConfig) {
        if !sound_config.enabled {
            return;
        }

        let custom_path = sound_config.custom_sound_path.clone();
        let volume = sound_config.volume;

        // Spawn a thread to play the sound to avoid blocking
        std::thread::spawn(move || {
            if let Ok(player) = AudioPlayer::new() {
                player.set_volume(volume);

                let result = if let Some(ref path) = custom_path {
                    player.play(path)
                } else {
                    player.play_system_alert()
                };

                if let Err(e) = result {
                    log::warn!("Failed to play alarm sound: {:?}", e);
                }

                // Keep thread alive while sound plays
                // For non-looped sounds, wait a reasonable time
                std::thread::sleep(std::time::Duration::from_secs(5));
            }
        });
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

    fn check_alarm(&mut self) {
        let current = (self.hour, self.minute, self.second);

        // Collect alarm checks first to avoid borrow issues
        let alarm_checks: Vec<_> = self.config.alarms.iter()
            .filter(|a| a.enabled)
            .map(|alarm| {
                let alarm_time = (alarm.hour, alarm.minute, alarm.second);
                let day_matches = alarm.days.is_empty() || alarm.days.contains(&self.day_of_week);
                (alarm.id.clone(), alarm_time, day_matches, alarm.sound.clone())
            })
            .collect();

        for (alarm_id, alarm_time, day_matches, sound_config) in alarm_checks {
            let last_check = self.last_alarm_check.get(&alarm_id).cloned();

            if day_matches && current == alarm_time {
                // Only trigger once per second
                if last_check != Some(current) {
                    self.triggered_alarms.insert(alarm_id.clone());
                    self.last_alarm_check.insert(alarm_id.clone(), current);

                    // Play alarm sound once when triggered
                    if !self.alarm_sound_played.contains(&alarm_id) {
                        self.play_sound_for_id(&alarm_id, &sound_config);
                        self.alarm_sound_played.insert(alarm_id);
                    }
                }
            } else if last_check.is_some() && last_check != Some(current) {
                // Time has passed - auto re-arm for repeating alarms
                self.triggered_alarms.remove(&alarm_id);
                self.alarm_sound_played.remove(&alarm_id);
                self.last_alarm_check.remove(&alarm_id);
            }
        }

        // Update next alarm info
        self.update_next_alarm();
    }

    /// Find and set the next upcoming alarm
    fn update_next_alarm(&mut self) {
        let mut next_alarm: Option<(&AlarmConfig, u64)> = None;

        for alarm in self.config.alarms.iter().filter(|a| a.enabled) {
            let minutes_until = self.minutes_until_alarm(alarm);
            if let Some((_, current_min)) = next_alarm {
                if minutes_until < current_min {
                    next_alarm = Some((alarm, minutes_until));
                }
            } else {
                next_alarm = Some((alarm, minutes_until));
            }
        }

        if let Some((alarm, _)) = next_alarm {
            self.next_alarm_time = Some(format!("{:02}:{:02}", alarm.hour, alarm.minute));
            self.next_alarm_id = Some(alarm.id.clone());
        } else {
            self.next_alarm_time = None;
            self.next_alarm_id = None;
        }
    }

    /// Calculate minutes until an alarm fires (accounting for days)
    fn minutes_until_alarm(&self, alarm: &AlarmConfig) -> u64 {
        let current_minutes = (self.hour * 60 + self.minute) as i64;
        let alarm_minutes = (alarm.hour * 60 + alarm.minute) as i64;

        let mut diff = alarm_minutes - current_minutes;

        // If alarm is today but already passed, or not on this day, calculate next occurrence
        if diff <= 0 || (!alarm.days.is_empty() && !alarm.days.contains(&self.day_of_week)) {
            // Find next valid day
            if alarm.days.is_empty() {
                // Every day - if passed today, it's tomorrow
                diff += 24 * 60;
            } else {
                // Find next day in the list
                let mut days_until = 1u64;
                for i in 1..=7 {
                    let check_day = (self.day_of_week + i) % 7;
                    if alarm.days.contains(&check_day) {
                        days_until = i as u64;
                        break;
                    }
                }
                diff = (days_until as i64 * 24 * 60) + (alarm_minutes - current_minutes);
                if diff < 0 {
                    diff += 24 * 60; // Wrap around
                }
            }
        }

        diff.max(0) as u64
    }

    fn update_timer(&mut self) {
        // Update all timers
        for timer in &mut self.config.timers {
            match timer.state {
                TimerState::Running => {
                    if let Some(start) = timer.start_instant {
                        let elapsed_since_start = start.elapsed().as_millis() as u64;

                        match timer.mode {
                            TimerMode::Stopwatch => {
                                timer.elapsed_ms += elapsed_since_start;
                                timer.start_instant = Some(Instant::now());
                            }
                            TimerMode::Countdown => {
                                if timer.elapsed_ms >= elapsed_since_start {
                                    timer.elapsed_ms -= elapsed_since_start;
                                    timer.start_instant = Some(Instant::now());
                                } else {
                                    timer.elapsed_ms = 0;
                                    timer.state = TimerState::Finished;
                                    timer.start_instant = None;
                                }
                            }
                        }
                    }
                }
                TimerState::Stopped => {
                    match timer.mode {
                        TimerMode::Countdown => {
                            timer.elapsed_ms = timer.countdown_duration * 1000;
                        }
                        TimerMode::Stopwatch => {
                            timer.elapsed_ms = 0;
                        }
                    }
                }
                _ => {}
            }
        }

        // Check for finished timers and play sounds
        let finished_timers: Vec<_> = self.config.timers.iter()
            .filter(|t| t.state == TimerState::Finished)
            .map(|t| (t.id.clone(), t.sound.clone()))
            .collect();

        for (timer_id, sound_config) in finished_timers {
            if !self.timer_sound_played.contains(&timer_id) {
                self.play_sound_for_id(&timer_id, &sound_config);
                self.timer_sound_played.insert(timer_id);
            }
        }

        // Update timer display for the "active" timer (first running/paused/finished, or first countdown)
        self.update_timer_display();
    }

    /// Update timer display string for the most relevant timer
    fn update_timer_display(&mut self) {
        // Priority: finished > running > paused > first countdown
        let display_timer = self.config.timers.iter()
            .find(|t| t.state == TimerState::Finished)
            .or_else(|| self.config.timers.iter().find(|t| t.state == TimerState::Running))
            .or_else(|| self.config.timers.iter().find(|t| t.state == TimerState::Paused))
            .or_else(|| self.config.timers.iter().find(|t| t.mode == TimerMode::Countdown));

        if let Some(timer) = display_timer {
            let total_seconds = timer.elapsed_ms / 1000;
            let hours = total_seconds / 3600;
            let minutes = (total_seconds % 3600) / 60;
            let seconds = total_seconds % 60;

            self.timer_display = if hours > 0 {
                format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
            } else {
                format!("{:02}:{:02}", minutes, seconds)
            };

            // Calculate progress for countdown timers
            if timer.mode == TimerMode::Countdown {
                let total_ms = timer.countdown_duration * 1000;
                self.timer_progress = if total_ms > 0 {
                    1.0 - (timer.elapsed_ms as f64 / total_ms as f64)
                } else {
                    1.0
                };
            } else {
                self.timer_progress = 0.0;
            }
        } else {
            self.timer_display.clear();
            self.timer_progress = 0.0;
        }
    }

    /// Start a timer by ID
    pub fn start_timer(&mut self, timer_id: &str) {
        if let Some(timer) = self.config.timers.iter_mut().find(|t| t.id == timer_id) {
            if timer.state == TimerState::Stopped || timer.state == TimerState::Finished {
                // Initialize elapsed time for countdown
                if timer.mode == TimerMode::Countdown {
                    timer.elapsed_ms = timer.countdown_duration * 1000;
                }
                self.timer_sound_played.remove(timer_id);
            }
            timer.state = TimerState::Running;
            timer.start_instant = Some(Instant::now());
        }
    }

    /// Pause a timer by ID
    pub fn pause_timer(&mut self, timer_id: &str) {
        if let Some(timer) = self.config.timers.iter_mut().find(|t| t.id == timer_id) {
            timer.state = TimerState::Paused;
            timer.start_instant = None;
        }
    }

    /// Resume a timer by ID
    pub fn resume_timer(&mut self, timer_id: &str) {
        if let Some(timer) = self.config.timers.iter_mut().find(|t| t.id == timer_id) {
            timer.state = TimerState::Running;
            timer.start_instant = Some(Instant::now());
        }
    }

    /// Stop and reset a timer by ID
    pub fn stop_timer(&mut self, timer_id: &str) {
        if let Some(timer) = self.config.timers.iter_mut().find(|t| t.id == timer_id) {
            timer.state = TimerState::Stopped;
            timer.start_instant = None;
            timer.elapsed_ms = 0;
        }
        self.timer_sound_played.remove(timer_id);
    }

    /// Dismiss a specific alarm by ID
    pub fn dismiss_alarm(&mut self, alarm_id: &str) {
        self.triggered_alarms.remove(alarm_id);
        self.alarm_sound_played.remove(alarm_id);
        self.last_alarm_check.remove(alarm_id);
    }

    /// Dismiss all triggered alarms
    pub fn dismiss_all_alarms(&mut self) {
        let ids: Vec<_> = self.triggered_alarms.iter().cloned().collect();
        for id in ids {
            self.dismiss_alarm(&id);
        }
    }

    /// Dismiss all finished timers (reset them to stopped state)
    pub fn dismiss_finished_timers(&mut self) {
        let finished_ids: Vec<_> = self.config.timers.iter()
            .filter(|t| t.state == TimerState::Finished)
            .map(|t| t.id.clone())
            .collect();
        for id in finished_ids {
            self.stop_timer(&id);
        }
    }

    /// Add a new alarm
    pub fn add_alarm(&mut self, alarm: AlarmConfig) {
        self.config.alarms.push(alarm);
    }

    /// Remove an alarm by ID
    pub fn remove_alarm(&mut self, alarm_id: &str) {
        self.dismiss_alarm(alarm_id);
        self.config.alarms.retain(|a| a.id != alarm_id);
    }

    /// Add a new timer
    pub fn add_timer(&mut self, timer: TimerConfig) {
        self.config.timers.push(timer);
    }

    /// Remove a timer by ID
    pub fn remove_timer(&mut self, timer_id: &str) {
        self.stop_timer(timer_id);
        self.config.timers.retain(|t| t.id != timer_id);
    }

    /// Get the current alarms configuration
    pub fn get_alarms(&self) -> &[AlarmConfig] {
        &self.config.alarms
    }

    /// Get the current timers configuration
    pub fn get_timers(&self) -> &[TimerConfig] {
        &self.config.timers
    }

    /// Check if any alarm is currently triggered
    pub fn any_alarm_triggered(&self) -> bool {
        !self.triggered_alarms.is_empty()
    }

    /// Get IDs of all triggered alarms
    pub fn get_triggered_alarm_ids(&self) -> Vec<String> {
        self.triggered_alarms.iter().cloned().collect()
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
                "Value",
                "Normalized time value for analog display (0.0-1.0 based on seconds)",
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

        self.check_alarm();
        self.update_timer();

        Ok(())
    }

    fn get_values(&self) -> HashMap<String, Value> {
        let mut values = HashMap::new();

        // Time components
        values.insert("hour".to_string(), Value::from(self.hour));
        values.insert("minute".to_string(), Value::from(self.minute));
        values.insert("second".to_string(), Value::from(self.second));
        values.insert("millisecond".to_string(), Value::from(self.millisecond));

        // Formatted strings
        values.insert("time".to_string(), Value::from(self.time_string.clone()));
        values.insert("date".to_string(), Value::from(self.date_string.clone()));
        values.insert("day_name".to_string(), Value::from(self.day_name.clone()));
        values.insert("month_name".to_string(), Value::from(self.month_name.clone()));

        // Date components
        values.insert("day".to_string(), Value::from(self.day));
        values.insert("month".to_string(), Value::from(self.month));
        values.insert("year".to_string(), Value::from(self.year));
        values.insert("day_of_week".to_string(), Value::from(self.day_of_week));

        // Alarm - backward compatible fields
        let any_triggered = !self.triggered_alarms.is_empty();
        values.insert("alarm_triggered".to_string(), Value::from(any_triggered));
        let any_alarm_enabled = self.config.alarms.iter().any(|a| a.enabled);
        values.insert("alarm_enabled".to_string(), Value::from(any_alarm_enabled));

        // Next alarm info for display
        if let Some(ref next_time) = self.next_alarm_time {
            values.insert("next_alarm_time".to_string(), Value::from(next_time.clone()));
        }

        // Expose triggered alarm IDs for dismiss handling
        let triggered_ids: Vec<_> = self.triggered_alarms.iter().cloned().collect();
        values.insert("triggered_alarm_ids".to_string(),
            serde_json::to_value(&triggered_ids).unwrap_or(Value::Array(vec![])));

        // Expose all alarms and timers for UI
        values.insert("alarms".to_string(),
            serde_json::to_value(&self.config.alarms).unwrap_or(Value::Array(vec![])));
        values.insert("timers".to_string(),
            serde_json::to_value(&self.config.timers).unwrap_or(Value::Array(vec![])));

        // Timer display (most relevant timer)
        values.insert("timer_display".to_string(), Value::from(self.timer_display.clone()));
        values.insert("timer_progress".to_string(), Value::from(self.timer_progress));

        // Timer state - find most relevant timer
        let display_timer_state = self.config.timers.iter()
            .find(|t| t.state == TimerState::Finished)
            .or_else(|| self.config.timers.iter().find(|t| t.state == TimerState::Running))
            .or_else(|| self.config.timers.iter().find(|t| t.state == TimerState::Paused))
            .map(|t| t.state)
            .unwrap_or(TimerState::Stopped);

        let timer_state = match display_timer_state {
            TimerState::Stopped => "stopped",
            TimerState::Running => "running",
            TimerState::Paused => "paused",
            TimerState::Finished => "finished",
        };
        values.insert("timer_state".to_string(), Value::from(timer_state));

        // Normalized value for analog displays (based on 12-hour clock)
        let hour_12 = (self.hour % 12) as f64;
        let minute_frac = self.minute as f64 / 60.0;
        let second_frac = self.second as f64 / 60.0;
        let ms_frac = self.millisecond as f64 / 1000.0;

        // Hour hand position (0-1 for full rotation)
        let hour_value = (hour_12 + minute_frac) / 12.0;
        values.insert("hour_value".to_string(), Value::from(hour_value));

        // Minute hand position (0-1 for full rotation)
        let minute_value = (self.minute as f64 + second_frac) / 60.0;
        values.insert("minute_value".to_string(), Value::from(minute_value));

        // Second hand position (0-1 for full rotation, with millisecond smoothing)
        let second_value = (self.second as f64 + ms_frac) / 60.0;
        values.insert("second_value".to_string(), Value::from(second_value));

        // Generic value for compatibility
        values.insert("value".to_string(), Value::from(second_value));

        // Caption for text displays
        values.insert("caption".to_string(), Value::from(self.time_string.clone()));
        values.insert("unit".to_string(), Value::from(""));

        // Timezone
        values.insert("timezone".to_string(), Value::from(self.config.timezone.clone()));

        values
    }

    fn configure(&mut self, config: &HashMap<String, Value>) -> Result<()> {
        if let Some(config_value) = config.get("clock_config") {
            let mut new_config: ClockSourceConfig = serde_json::from_value(config_value.clone())?;

            // Migrate legacy single alarm/timer if present
            new_config.migrate_legacy();

            // Preserve timer runtime state for existing timers
            let old_timer_states: HashMap<String, (TimerState, u64, Option<Instant>)> =
                self.config.timers.iter()
                    .map(|t| (t.id.clone(), (t.state, t.elapsed_ms, t.start_instant)))
                    .collect();

            self.config = new_config;

            // Restore timer runtime state for timers that still exist
            for timer in &mut self.config.timers {
                if let Some((state, elapsed, instant)) = old_timer_states.get(&timer.id) {
                    timer.state = *state;
                    timer.elapsed_ms = *elapsed;
                    timer.start_instant = *instant;
                }
            }
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
}
