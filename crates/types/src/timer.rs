//! Timer, alarm, and audio configuration types

use crate::color::Color;
use serde::{Deserialize, Serialize};
use std::time::Instant;
use uuid::Uuid;

/// Configuration for alarm/timer sounds
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AlarmSoundConfig {
    /// Whether sound is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Custom sound file path (None = use system alert)
    #[serde(default)]
    pub custom_sound_path: Option<String>,

    /// Whether to loop the sound until dismissed
    #[serde(default = "default_true")]
    pub loop_sound: bool,

    /// Volume level (0.0 to 1.0)
    #[serde(default = "default_volume")]
    pub volume: f32,

    /// Whether visual flash effect is enabled
    #[serde(default = "default_true")]
    pub visual_enabled: bool,
}

fn default_true() -> bool {
    true
}

fn default_volume() -> f32 {
    0.8
}

impl Default for AlarmSoundConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            custom_sound_path: None,
            loop_sound: true,
            volume: 0.8,
            visual_enabled: true,
        }
    }
}

/// Timer mode - kept for backward compatibility but always Countdown
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum TimerMode {
    #[serde(rename = "countdown")]
    #[default]
    Countdown,
    #[serde(rename = "stopwatch")]
    Stopwatch, // Deprecated, treated as Countdown
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

/// Timer display configuration (font, color, location)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TimerDisplayConfig {
    pub font_family: String,
    pub font_size: f64,
    pub font_bold: bool,
    pub font_italic: bool,
    pub color: Color,
    pub finished_color: Color,
    /// Horizontal position: "left", "center", "right"
    pub horizontal_position: String,
    /// Vertical position: "top", "center", "bottom"
    pub vertical_position: String,
    /// Offset from edge (percentage of panel size)
    pub offset_x: f64,
    pub offset_y: f64,
}

impl Default for TimerDisplayConfig {
    fn default() -> Self {
        Self {
            font_family: "Sans".to_string(),
            font_size: 14.0,
            font_bold: true,
            font_italic: false,
            color: Color::new(0.2, 0.8, 0.2, 1.0), // Green
            finished_color: Color::new(1.0, 0.3, 0.3, 1.0), // Red
            horizontal_position: "right".to_string(),
            vertical_position: "bottom".to_string(),
            offset_x: 5.0,
            offset_y: 5.0,
        }
    }
}

fn generate_uuid() -> String {
    Uuid::new_v4().to_string()
}

/// Timer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimerConfig {
    /// Unique identifier for this timer
    #[serde(default = "generate_uuid")]
    pub id: String,
    /// Mode - kept for backwards compatibility, always treated as Countdown
    #[serde(default)]
    pub mode: TimerMode,
    /// Countdown duration in seconds
    pub countdown_duration: u64,
    /// Current state (not serialized - runtime only)
    #[serde(skip)]
    pub state: TimerState,
    /// Elapsed time in milliseconds
    #[serde(skip)]
    pub elapsed_ms: u64,
    /// When the timer was last started/resumed
    #[serde(skip)]
    pub start_instant: Option<Instant>,
    /// Custom timer label
    pub label: Option<String>,
    /// Sound configuration - deprecated, use global_timer_sound instead
    #[serde(default, skip_serializing)]
    pub sound: AlarmSoundConfig,
    /// Display configuration
    #[serde(default)]
    pub display: TimerDisplayConfig,
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
            display: TimerDisplayConfig::default(),
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

    /// Get formatted display string
    pub fn display_string(&self) -> String {
        let total_seconds = self.elapsed_ms / 1000;
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let seconds = total_seconds % 60;

        if hours > 0 {
            format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
        } else {
            format!("{:02}:{:02}", minutes, seconds)
        }
    }

    /// Get progress (0.0 to 1.0) for countdown timers
    pub fn progress(&self) -> f64 {
        if self.mode == TimerMode::Countdown {
            let total_ms = self.countdown_duration * 1000;
            if total_ms > 0 {
                1.0 - (self.elapsed_ms as f64 / total_ms as f64)
            } else {
                1.0
            }
        } else {
            0.0
        }
    }
}

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
            ..Self::default()
        }
    }
}
