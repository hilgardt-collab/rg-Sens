//! Test source configuration types.

use serde::{Deserialize, Serialize};

/// Test signal mode
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TestMode {
    /// Manual static value
    #[default]
    Manual,
    /// Sine wave oscillation
    SineWave,
    /// Sawtooth wave (linear ramp)
    Sawtooth,
    /// Triangle wave
    Triangle,
    /// Square wave
    Square,
}

fn default_manual_value() -> f64 {
    50.0
}

fn default_max_value() -> f64 {
    100.0
}

fn default_period() -> f64 {
    5.0
}

fn default_update_interval() -> u64 {
    100
}

/// Test source configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestSourceConfig {
    /// Current mode
    #[serde(default)]
    pub mode: TestMode,
    /// Manual value (used in Manual mode)
    #[serde(default = "default_manual_value")]
    pub manual_value: f64,
    /// Minimum value
    #[serde(default)]
    pub min_value: f64,
    /// Maximum value
    #[serde(default = "default_max_value")]
    pub max_value: f64,
    /// Wave period in seconds (for oscillation modes)
    #[serde(default = "default_period")]
    pub period: f64,
    /// Update interval in milliseconds
    #[serde(default = "default_update_interval")]
    pub update_interval_ms: u64,
}

impl Default for TestSourceConfig {
    fn default() -> Self {
        Self {
            mode: TestMode::Manual,
            manual_value: default_manual_value(),
            min_value: 0.0,
            max_value: default_max_value(),
            period: default_period(),
            update_interval_ms: default_update_interval(),
        }
    }
}
