//! Test data source for debugging and demonstration
//!
//! Provides a configurable value source with manual control or automatic oscillation

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use std::time::Duration;
use crate::core::{DataSource, FieldMetadata, FieldPurpose, FieldType, SourceMetadata};

/// Test value generation mode
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

/// Configuration for the test source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestSourceConfig {
    /// Current mode
    pub mode: TestMode,
    /// Manual value (used in Manual mode)
    pub manual_value: f64,
    /// Minimum value
    pub min_value: f64,
    /// Maximum value
    pub max_value: f64,
    /// Wave period in seconds (for oscillation modes)
    pub period: f64,
    /// Update interval in milliseconds
    #[serde(default = "default_update_interval")]
    pub update_interval_ms: u64,
}

fn default_update_interval() -> u64 {
    100
}

impl Default for TestSourceConfig {
    fn default() -> Self {
        Self {
            mode: TestMode::Manual,
            manual_value: 50.0,
            min_value: 0.0,
            max_value: 100.0,
            period: 5.0,
            update_interval_ms: 100,
        }
    }
}

/// Shared state for the test source that can be modified from UI
#[derive(Debug)]
pub struct TestSourceState {
    pub config: TestSourceConfig,
    pub start_time: Instant,
}

impl Default for TestSourceState {
    fn default() -> Self {
        Self {
            config: TestSourceConfig::default(),
            start_time: Instant::now(),
        }
    }
}

/// Global test source state for UI access
pub static TEST_SOURCE_STATE: once_cell::sync::Lazy<Arc<Mutex<TestSourceState>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(TestSourceState::default())));

/// Test data source
pub struct TestSource {
    metadata: SourceMetadata,
    /// Cached output values - updated in update(), returned by reference in values_ref()
    values: HashMap<String, Value>,
}

impl TestSource {
    pub fn new() -> Self {
        Self {
            metadata: SourceMetadata {
                id: "test".to_string(),
                name: "Test".to_string(),
                description: "Test source for debugging and demonstration".to_string(),
                available_keys: vec![
                    "caption".to_string(),
                    "value".to_string(),
                    "unit".to_string(),
                    "normalized".to_string(),
                    "numerical_value".to_string(),
                    "min".to_string(),
                    "max".to_string(),
                    "min_limit".to_string(),
                    "max_limit".to_string(),
                ],
                default_interval: Duration::from_millis(100),
            },
            values: HashMap::with_capacity(12),
        }
    }

    /// Calculate value based on current mode and time
    fn calculate_value(state: &TestSourceState) -> f64 {
        let config = &state.config;
        let range = config.max_value - config.min_value;

        match config.mode {
            TestMode::Manual => config.manual_value,
            TestMode::SineWave => {
                let elapsed = state.start_time.elapsed().as_secs_f64();
                let phase = (elapsed / config.period) * std::f64::consts::TAU;
                let normalized = (phase.sin() + 1.0) / 2.0; // 0.0 to 1.0
                config.min_value + normalized * range
            }
            TestMode::Sawtooth => {
                let elapsed = state.start_time.elapsed().as_secs_f64();
                let normalized = (elapsed / config.period).fract(); // 0.0 to 1.0
                config.min_value + normalized * range
            }
            TestMode::Triangle => {
                let elapsed = state.start_time.elapsed().as_secs_f64();
                let phase = (elapsed / config.period).fract() * 2.0; // 0.0 to 2.0
                let normalized = if phase <= 1.0 { phase } else { 2.0 - phase }; // 0.0 to 1.0 to 0.0
                config.min_value + normalized * range
            }
            TestMode::Square => {
                let elapsed = state.start_time.elapsed().as_secs_f64();
                let phase = (elapsed / config.period).fract();
                if phase < 0.5 {
                    config.min_value
                } else {
                    config.max_value
                }
            }
        }
    }
}

impl Default for TestSource {
    fn default() -> Self {
        Self::new()
    }
}

impl DataSource for TestSource {
    fn metadata(&self) -> &SourceMetadata {
        &self.metadata
    }

    fn fields(&self) -> Vec<FieldMetadata> {
        vec![
            FieldMetadata::new(
                "caption",
                "Caption",
                "Display caption",
                FieldType::Text,
                FieldPurpose::Caption,
            ),
            FieldMetadata::new(
                "value",
                "Value",
                "Test value (configurable)",
                FieldType::Numerical,
                FieldPurpose::Value,
            ),
            FieldMetadata::new(
                "unit",
                "Unit",
                "Display unit",
                FieldType::Text,
                FieldPurpose::Unit,
            ),
            FieldMetadata::new(
                "normalized",
                "Normalized",
                "Normalized value (0-100)",
                FieldType::Numerical,
                FieldPurpose::Value,
            ),
            FieldMetadata::new(
                "min",
                "Min",
                "Minimum value",
                FieldType::Numerical,
                FieldPurpose::SecondaryValue,
            ),
            FieldMetadata::new(
                "max",
                "Max",
                "Maximum value",
                FieldType::Numerical,
                FieldPurpose::SecondaryValue,
            ),
        ]
    }

    fn update(&mut self) -> anyhow::Result<()> {
        // Build values HashMap (reuse allocation, just clear and refill)
        self.values.clear();

        // Use blocking lock - handlers only hold the lock briefly so this is safe
        // try_lock was causing values to reset to defaults during any lock contention
        let Ok(state) = TEST_SOURCE_STATE.lock() else {
            // Return default values only if lock is poisoned (thread panic)
            self.values.insert("caption".to_string(), Value::from("Test"));
            self.values.insert("value".to_string(), Value::from(50.0));
            self.values.insert("unit".to_string(), Value::from(""));
            self.values.insert("normalized".to_string(), Value::from(50.0));
            self.values.insert("numerical_value".to_string(), Value::from(50.0));
            self.values.insert("min".to_string(), Value::from(0.0));
            self.values.insert("max".to_string(), Value::from(100.0));
            self.values.insert("min_limit".to_string(), Value::from(0.0));
            self.values.insert("max_limit".to_string(), Value::from(100.0));
            return Ok(());
        };
        let value = Self::calculate_value(&state);
        let config = &state.config;

        // Calculate normalized value (0-100)
        let range = config.max_value - config.min_value;
        let normalized = if range > 0.0 {
            ((value - config.min_value) / range * 100.0).clamp(0.0, 100.0)
        } else {
            50.0
        };

        // Caption based on mode
        let caption = match config.mode {
            TestMode::Manual => "Test",
            TestMode::SineWave => "Sine",
            TestMode::Sawtooth => "Saw",
            TestMode::Triangle => "Tri",
            TestMode::Square => "Sqr",
        };
        self.values.insert("caption".to_string(), Value::from(caption));
        self.values.insert("value".to_string(), Value::from(value));
        self.values.insert("unit".to_string(), Value::from(""));
        self.values.insert("normalized".to_string(), Value::from(normalized));
        self.values.insert("min".to_string(), Value::from(config.min_value));
        self.values.insert("max".to_string(), Value::from(config.max_value));
        // Also provide min_limit/max_limit for displayers that expect these names
        self.values.insert("min_limit".to_string(), Value::from(config.min_value));
        self.values.insert("max_limit".to_string(), Value::from(config.max_value));
        // Provide numerical_value for LCARS compatibility
        self.values.insert("numerical_value".to_string(), Value::from(value));

        Ok(())
    }

    fn get_values(&self) -> HashMap<String, Value> {
        self.values.clone()
    }

    fn values_ref(&self) -> Option<&HashMap<String, Value>> {
        Some(&self.values)
    }

    fn get_typed_config(&self) -> Option<crate::core::SourceConfig> {
        if let Ok(state) = TEST_SOURCE_STATE.lock() {
            Some(crate::core::SourceConfig::Test(state.config.clone()))
        } else {
            None
        }
    }

    fn configure(&mut self, config: &HashMap<String, Value>) -> anyhow::Result<()> {
        // DO NOT modify the global TEST_SOURCE_STATE here!
        // The global state is controlled exclusively by the Test Source Dialog.
        // This method is called when config dialogs open/create sources for preview,
        // and we don't want that to reset the running test source state.
        //
        // Only update the update_interval_ms which is a per-panel setting
        // that doesn't affect the running value generation.

        if let Some(test_config_value) = config.get("test_config") {
            if let Ok(test_config) = serde_json::from_value::<TestSourceConfig>(test_config_value.clone()) {
                // Only update update_interval_ms, not mode/values
                if let Ok(mut state) = TEST_SOURCE_STATE.lock() {
                    state.config.update_interval_ms = test_config.update_interval_ms;
                }
            }
        }

        // Check for individual update_interval key
        if let Some(interval) = config.get("update_interval_ms").and_then(|v| v.as_u64()) {
            if let Ok(mut state) = TEST_SOURCE_STATE.lock() {
                state.config.update_interval_ms = interval;
            }
        }

        Ok(())
    }
}
