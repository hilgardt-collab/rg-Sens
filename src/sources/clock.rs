//! Clock data source implementation
//!
//! Provides current time, date, alarm, and timer functionality.

use crate::core::{DataSource, FieldMetadata, FieldPurpose, FieldType, SourceMetadata};
use anyhow::Result;
use chrono::{Local, Timelike, Datelike, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Alarm sound configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AlarmSoundConfig {
    /// Enable audible alarm
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Path to custom sound file (None = system bell)
    #[serde(default)]
    pub custom_sound_path: Option<String>,
    /// Enable visual flash effect
    #[serde(default = "default_true")]
    pub visual_enabled: bool,
}

fn default_true() -> bool {
    true
}

impl Default for AlarmSoundConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            custom_sound_path: None,
            visual_enabled: true,
        }
    }
}

/// Alarm configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AlarmConfig {
    pub enabled: bool,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
    /// Days of week the alarm is active (0=Sunday, 6=Saturday)
    pub days: Vec<u32>,
    /// Whether the alarm is currently triggered (ringing)
    #[serde(skip)]
    pub triggered: bool,
    /// Custom alarm label
    pub label: Option<String>,
    /// Sound configuration for this alarm
    #[serde(default)]
    pub sound: AlarmSoundConfig,
}

impl Default for AlarmConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            hour: 7,
            minute: 0,
            second: 0,
            days: vec![1, 2, 3, 4, 5], // Weekdays by default
            triggered: false,
            label: None,
            sound: AlarmSoundConfig::default(),
        }
    }
}

/// Timer mode
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum TimerMode {
    #[serde(rename = "countdown")]
    Countdown,
    #[serde(rename = "stopwatch")]
    Stopwatch,
}

impl Default for TimerMode {
    fn default() -> Self {
        Self::Countdown
    }
}

/// Timer state
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum TimerState {
    #[serde(rename = "stopped")]
    Stopped,
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "paused")]
    Paused,
    #[serde(rename = "finished")]
    Finished,
}

impl Default for TimerState {
    fn default() -> Self {
        Self::Stopped
    }
}

/// Timer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimerConfig {
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

/// Time format
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum TimeFormat {
    #[serde(rename = "24h")]
    Hour24,
    #[serde(rename = "12h")]
    Hour12,
}

impl Default for TimeFormat {
    fn default() -> Self {
        Self::Hour24
    }
}

/// Date format
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum DateFormat {
    #[serde(rename = "yyyy-mm-dd")]
    YearMonthDay,
    #[serde(rename = "dd/mm/yyyy")]
    DayMonthYear,
    #[serde(rename = "mm/dd/yyyy")]
    MonthDayYear,
    #[serde(rename = "day, month dd, yyyy")]
    LongFormat,
}

impl Default for DateFormat {
    fn default() -> Self {
        Self::YearMonthDay
    }
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
    #[serde(default)]
    pub alarm: AlarmConfig,
    #[serde(default)]
    pub timer: TimerConfig,
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
            alarm: AlarmConfig::default(),
            timer: TimerConfig::default(),
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
    // Alarm state
    alarm_triggered: bool,
    last_alarm_check: Option<(u32, u32, u32)>,
    alarm_sound_played: bool,
    // Timer values
    timer_display: String,
    timer_progress: f64,
    timer_sound_played: bool,
}

impl ClockSource {
    pub fn new() -> Self {
        Self {
            metadata: SourceMetadata {
                id: "clock".to_string(),
                name: "Clock".to_string(),
                description: "Current time, date, alarm, and timer".to_string(),
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
                    "alarm_time".to_string(),
                    "timer_display".to_string(),
                    "timer_state".to_string(),
                    "timer_progress".to_string(),
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
            alarm_triggered: false,
            last_alarm_check: None,
            alarm_sound_played: false,
            timer_display: "00:00".to_string(),
            timer_progress: 0.0,
            timer_sound_played: false,
        }
    }

    /// Play alarm/timer sound
    fn play_alarm_sound(sound_config: &AlarmSoundConfig) {
        if !sound_config.enabled {
            return;
        }

        // Spawn a thread to play the sound to avoid blocking
        let custom_path = sound_config.custom_sound_path.clone();
        std::thread::spawn(move || {
            if let Some(path) = custom_path {
                // Try to play custom sound file using system player
                #[cfg(target_os = "linux")]
                {
                    let _ = std::process::Command::new("paplay")
                        .arg(&path)
                        .spawn();
                }
                #[cfg(target_os = "macos")]
                {
                    let _ = std::process::Command::new("afplay")
                        .arg(&path)
                        .spawn();
                }
                #[cfg(target_os = "windows")]
                {
                    // Windows: use PowerShell to play sound
                    let _ = std::process::Command::new("powershell")
                        .args(["-c", &format!("(New-Object Media.SoundPlayer '{}').PlaySync()", path)])
                        .spawn();
                }
            } else {
                // System bell/beep
                #[cfg(target_os = "linux")]
                {
                    // Try paplay first (PulseAudio), then fall back to other methods
                    let result = std::process::Command::new("paplay")
                        .arg("/usr/share/sounds/freedesktop/stereo/alarm-clock-elapsed.oga")
                        .status();
                    if result.is_err() || !result.unwrap().success() {
                        // Try canberra-gtk-play (GNOME)
                        let result = std::process::Command::new("canberra-gtk-play")
                            .arg("-i")
                            .arg("alarm-clock-elapsed")
                            .status();
                        if result.is_err() || !result.unwrap().success() {
                            // Fall back to terminal bell
                            print!("\x07");
                            let _ = std::io::Write::flush(&mut std::io::stdout());
                        }
                    }
                }
                #[cfg(target_os = "macos")]
                {
                    let _ = std::process::Command::new("afplay")
                        .arg("/System/Library/Sounds/Glass.aiff")
                        .spawn();
                }
                #[cfg(target_os = "windows")]
                {
                    // Windows system beep
                    let _ = std::process::Command::new("powershell")
                        .args(["-c", "[console]::beep(800, 500); [console]::beep(800, 500)"])
                        .spawn();
                }
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
        if !self.config.alarm.enabled {
            self.alarm_triggered = false;
            self.alarm_sound_played = false;
            return;
        }

        let current = (self.hour, self.minute, self.second);
        let alarm = (
            self.config.alarm.hour,
            self.config.alarm.minute,
            self.config.alarm.second,
        );

        // Check if current day is in alarm days
        let day_matches = self.config.alarm.days.is_empty()
            || self.config.alarm.days.contains(&self.day_of_week);

        if day_matches && current == alarm {
            // Only trigger once per second
            if self.last_alarm_check != Some(current) {
                self.alarm_triggered = true;
                self.last_alarm_check = Some(current);
                // Play alarm sound once when triggered
                if !self.alarm_sound_played {
                    Self::play_alarm_sound(&self.config.alarm.sound);
                    self.alarm_sound_played = true;
                }
            }
        } else {
            // Reset trigger after the alarm second passes
            if self.last_alarm_check.is_some() && self.last_alarm_check != Some(current) {
                self.alarm_triggered = false;
                self.alarm_sound_played = false;
            }
        }
    }

    fn update_timer(&mut self) {
        match self.config.timer.state {
            TimerState::Running => {
                if let Some(start) = self.config.timer.start_instant {
                    let elapsed_since_start = start.elapsed().as_millis() as u64;

                    match self.config.timer.mode {
                        TimerMode::Stopwatch => {
                            self.config.timer.elapsed_ms += elapsed_since_start;
                            self.config.timer.start_instant = Some(Instant::now());
                            self.timer_progress = 0.0; // Stopwatch has no progress
                        }
                        TimerMode::Countdown => {
                            let total_ms = self.config.timer.countdown_duration * 1000;
                            if self.config.timer.elapsed_ms >= elapsed_since_start {
                                self.config.timer.elapsed_ms -= elapsed_since_start;
                                self.config.timer.start_instant = Some(Instant::now());
                            } else {
                                self.config.timer.elapsed_ms = 0;
                                self.config.timer.state = TimerState::Finished;
                                self.config.timer.start_instant = None;
                                // Play timer finished sound once
                                if !self.timer_sound_played {
                                    Self::play_alarm_sound(&self.config.timer.sound);
                                    self.timer_sound_played = true;
                                }
                            }
                            self.timer_progress = if total_ms > 0 {
                                1.0 - (self.config.timer.elapsed_ms as f64 / total_ms as f64)
                            } else {
                                1.0
                            };
                        }
                    }
                }
            }
            TimerState::Stopped => {
                match self.config.timer.mode {
                    TimerMode::Countdown => {
                        self.config.timer.elapsed_ms = self.config.timer.countdown_duration * 1000;
                        self.timer_progress = 0.0;
                    }
                    TimerMode::Stopwatch => {
                        self.config.timer.elapsed_ms = 0;
                        self.timer_progress = 0.0;
                    }
                }
            }
            _ => {}
        }

        // Format timer display
        let total_seconds = self.config.timer.elapsed_ms / 1000;
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let seconds = total_seconds % 60;

        self.timer_display = if hours > 0 {
            format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
        } else {
            format!("{:02}:{:02}", minutes, seconds)
        };
    }

    /// Start the timer
    pub fn start_timer(&mut self) {
        if self.config.timer.state == TimerState::Stopped || self.config.timer.state == TimerState::Finished {
            // Initialize elapsed time for countdown
            if self.config.timer.mode == TimerMode::Countdown {
                self.config.timer.elapsed_ms = self.config.timer.countdown_duration * 1000;
            }
            self.timer_sound_played = false; // Reset sound flag when starting fresh
        }
        self.config.timer.state = TimerState::Running;
        self.config.timer.start_instant = Some(Instant::now());
    }

    /// Pause the timer
    pub fn pause_timer(&mut self) {
        self.config.timer.state = TimerState::Paused;
        self.config.timer.start_instant = None;
    }

    /// Resume the timer
    pub fn resume_timer(&mut self) {
        self.config.timer.state = TimerState::Running;
        self.config.timer.start_instant = Some(Instant::now());
    }

    /// Stop and reset the timer
    pub fn stop_timer(&mut self) {
        self.config.timer.state = TimerState::Stopped;
        self.config.timer.start_instant = None;
        self.config.timer.elapsed_ms = 0;
        self.timer_sound_played = false; // Reset sound flag
    }

    /// Dismiss the alarm
    pub fn dismiss_alarm(&mut self) {
        self.alarm_triggered = false;
        self.alarm_sound_played = false; // Reset sound flag
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

        // Alarm
        values.insert("alarm_triggered".to_string(), Value::from(self.alarm_triggered));
        values.insert("alarm_enabled".to_string(), Value::from(self.config.alarm.enabled));
        let alarm_time = format!(
            "{:02}:{:02}:{:02}",
            self.config.alarm.hour,
            self.config.alarm.minute,
            self.config.alarm.second
        );
        values.insert("alarm_time".to_string(), Value::from(alarm_time));

        // Timer
        values.insert("timer_display".to_string(), Value::from(self.timer_display.clone()));
        values.insert("timer_progress".to_string(), Value::from(self.timer_progress));
        let timer_state = match self.config.timer.state {
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

        values
    }

    fn configure(&mut self, config: &HashMap<String, Value>) -> Result<()> {
        if let Some(config_value) = config.get("clock_config") {
            let new_config: ClockSourceConfig = serde_json::from_value(config_value.clone())?;

            // Preserve timer runtime state if not changing timer settings
            let preserve_timer = self.config.timer.countdown_duration == new_config.timer.countdown_duration
                && self.config.timer.mode == new_config.timer.mode;

            let old_timer_state = self.config.timer.state;
            let old_timer_elapsed = self.config.timer.elapsed_ms;
            let old_timer_instant = self.config.timer.start_instant;

            self.config = new_config;

            if preserve_timer {
                self.config.timer.state = old_timer_state;
                self.config.timer.elapsed_ms = old_timer_elapsed;
                self.config.timer.start_instant = old_timer_instant;
            }
        }

        // Handle timer commands
        if let Some(cmd) = config.get("timer_command") {
            if let Some(cmd_str) = cmd.as_str() {
                match cmd_str {
                    "start" => self.start_timer(),
                    "pause" => self.pause_timer(),
                    "resume" => self.resume_timer(),
                    "stop" => self.stop_timer(),
                    _ => {}
                }
            }
        }

        // Handle alarm dismiss
        if let Some(dismiss) = config.get("dismiss_alarm") {
            if dismiss.as_bool().unwrap_or(false) {
                self.dismiss_alarm();
            }
        }

        Ok(())
    }
}
