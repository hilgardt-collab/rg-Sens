//! System temperature data source implementation
//!
//! Provides access to all available temperature sensors on the system,
//! including motherboard, drives, GPU packages, and other hardware sensors.

use crate::core::{DataSource, FieldMetadata, FieldPurpose, FieldType, SourceMetadata};
use anyhow::Result;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;

use super::shared_sensors;

/// Temperature unit for display
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum TemperatureUnit {
    #[serde(rename = "celsius")]
    #[default]
    Celsius,
    #[serde(rename = "fahrenheit")]
    Fahrenheit,
    #[serde(rename = "kelvin")]
    Kelvin,
}

/// Information about a discovered temperature sensor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorInfo {
    pub index: usize,
    pub label: String,
    pub category: SensorCategory,
}

/// Category of temperature sensor
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum SensorCategory {
    #[serde(rename = "cpu")]
    CPU,
    #[serde(rename = "gpu")]
    GPU,
    #[serde(rename = "motherboard")]
    Motherboard,
    #[serde(rename = "storage")]
    Storage,
    #[serde(rename = "other")]
    Other,
}

/// Configuration for system temperature source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemTempConfig {
    #[serde(default)]
    pub sensor_index: usize,
    #[serde(default)]
    pub temp_unit: TemperatureUnit,
    #[serde(default = "default_update_interval")]
    pub update_interval_ms: u64,
    #[serde(default)]
    pub custom_caption: Option<String>,
    #[serde(default)]
    pub min_limit: Option<f64>,
    #[serde(default)]
    pub max_limit: Option<f64>,
    #[serde(default = "default_auto_detect_limits")]
    pub auto_detect_limits: bool,
}

fn default_update_interval() -> u64 {
    1000 // 1 second default
}

fn default_auto_detect_limits() -> bool {
    false
}

impl Default for SystemTempConfig {
    fn default() -> Self {
        Self {
            sensor_index: 0,
            temp_unit: TemperatureUnit::Celsius,
            update_interval_ms: default_update_interval(),
            custom_caption: None,
            min_limit: None,
            max_limit: None,
            auto_detect_limits: default_auto_detect_limits(),
        }
    }
}

/// Cached sensor information (discovered once at startup)
static SYSTEM_SENSORS: Lazy<Vec<SensorInfo>> = Lazy::new(|| {
    log::info!("=== Discovering system temperature sensors (one-time initialization) ===");

    // Use shared sensors instead of creating new Components
    let all_temps = shared_sensors::get_refreshed_temperatures();
    let sensors = discover_all_sensors_from_list(&all_temps);

    log::info!("System temperature discovery complete: {} sensors found", sensors.len());

    sensors
});

/// Discover all temperature sensors from a list of (label, temperature) pairs
fn discover_all_sensors_from_list(temps: &[(String, f32)]) -> Vec<SensorInfo> {
    let mut sensors = Vec::new();

    log::info!("Scanning for temperature sensors...");
    log::info!("Total components found: {}", temps.len());

    for (index, (label, temp)) in temps.iter().enumerate() {
        // Categorize sensor based on label
        let category = categorize_sensor(label);

        sensors.push(SensorInfo {
            index,
            label: label.clone(),
            category,
        });

        log::info!("  [{}] {:?}: {} = {:.1}°C", index, category, label, temp);
    }

    log::info!("Total temperature sensors discovered: {}", sensors.len());
    sensors
}

/// Categorize a sensor based on its label
fn categorize_sensor(label: &str) -> SensorCategory {
    let label_lower = label.to_lowercase();

    // CPU sensors
    if label_lower.contains("cpu")
        || label_lower.contains("package")
        || label_lower.contains("tctl")
        || label_lower.contains("tccd")
        || label_lower.contains("tdie")
        || label_lower.contains("core")
        || label_lower.starts_with("k10temp")
        || label_lower.contains("processor")
    {
        return SensorCategory::CPU;
    }

    // GPU sensors
    if label_lower.contains("gpu")
        || label_lower.contains("nvidia")
        || label_lower.contains("amdgpu")
        || label_lower.contains("radeon")
        || label_lower.contains("edge")
        || label_lower.contains("junction")
        || label_lower.contains("mem")
    {
        return SensorCategory::GPU;
    }

    // Storage sensors
    if label_lower.contains("nvme")
        || label_lower.contains("ssd")
        || label_lower.contains("hdd")
        || label_lower.contains("drive")
        || label_lower.contains("disk")
    {
        return SensorCategory::Storage;
    }

    // Motherboard sensors
    if label_lower.contains("motherboard")
        || label_lower.contains("acpi")
        || label_lower.contains("pch")
        || label_lower.contains("chipset")
        || label_lower.contains("system")
        || label_lower.contains("ambient")
    {
        return SensorCategory::Motherboard;
    }

    // Default to Other
    SensorCategory::Other
}

/// System temperature data source
pub struct SystemTempSource {
    metadata: SourceMetadata,
    config: SystemTempConfig,
    current_temp: f64,
    detected_min: Option<f64>,
    detected_max: Option<f64>,
}

impl Default for SystemTempSource {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemTempSource {
    pub fn new() -> Self {
        // Force initialization of sensor discovery
        let _ = &*SYSTEM_SENSORS;

        Self {
            metadata: SourceMetadata {
                id: "system_temp".to_string(),
                name: "System Temperature".to_string(),
                description: "Temperature from any system sensor (motherboard, drives, etc.)".to_string(),
                available_keys: vec![
                    "temperature".to_string(),
                    "sensor_label".to_string(),
                    "unit".to_string(),
                    "caption".to_string(),
                    "min_limit".to_string(),
                    "max_limit".to_string(),
                ],
                default_interval: Duration::from_millis(1000),
            },
            // Temperature readings use shared_sensors module, not a local Components instance
            config: SystemTempConfig::default(),
            current_temp: 0.0,
            detected_min: None,
            detected_max: None,
        }
    }

    /// Get list of all available sensors
    pub fn available_sensors() -> &'static [SensorInfo] {
        &SYSTEM_SENSORS
    }

    /// Convert temperature to the configured unit
    fn convert_temperature(&self, celsius: f64) -> f64 {
        match self.config.temp_unit {
            TemperatureUnit::Celsius => celsius,
            TemperatureUnit::Fahrenheit => celsius * 9.0 / 5.0 + 32.0,
            TemperatureUnit::Kelvin => celsius + 273.15,
        }
    }

    /// Get unit suffix for display
    fn unit_suffix(&self) -> &str {
        match self.config.temp_unit {
            TemperatureUnit::Celsius => "°C",
            TemperatureUnit::Fahrenheit => "°F",
            TemperatureUnit::Kelvin => "K",
        }
    }
}

impl DataSource for SystemTempSource {
    fn metadata(&self) -> &SourceMetadata {
        &self.metadata
    }

    fn fields(&self) -> Vec<FieldMetadata> {
        vec![
            FieldMetadata::new(
                "value",
                "Value",
                format!("Current temperature value in {}", self.unit_suffix()),
                FieldType::Numerical,
                FieldPurpose::Value,
            ),
            FieldMetadata::new(
                "caption",
                "Caption",
                "Display caption for the temperature sensor",
                FieldType::Text,
                FieldPurpose::Caption,
            ),
            FieldMetadata::new(
                "unit",
                "Unit",
                "Temperature unit (°C, °F, or K)",
                FieldType::Text,
                FieldPurpose::Unit,
            ),
            FieldMetadata::new(
                "sensor_label",
                "Sensor",
                "Name of the selected sensor",
                FieldType::Text,
                FieldPurpose::Caption,
            ),
            FieldMetadata::new(
                "min_limit",
                "Min Limit",
                "Minimum temperature limit for visualization",
                FieldType::Numerical,
                FieldPurpose::SecondaryValue,
            ),
            FieldMetadata::new(
                "max_limit",
                "Max Limit",
                "Maximum temperature limit for visualization",
                FieldType::Numerical,
                FieldPurpose::SecondaryValue,
            ),
        ]
    }

    fn update(&mut self) -> Result<()> {
        // Get temperature from shared sensors (handles refresh internally)
        if let Some(temp_celsius) = shared_sensors::get_temperature_by_index(self.config.sensor_index) {
            self.current_temp = self.convert_temperature(temp_celsius as f64);

            // Update detected limits if auto-detect is enabled
            if self.config.auto_detect_limits {
                // Update min
                self.detected_min = Some(
                    self.detected_min
                        .map(|min| min.min(self.current_temp))
                        .unwrap_or(self.current_temp)
                );

                // Update max
                self.detected_max = Some(
                    self.detected_max
                        .map(|max| max.max(self.current_temp))
                        .unwrap_or(self.current_temp)
                );
            }
        } else {
            log::warn!("Selected sensor index {} not found", self.config.sensor_index);
            self.current_temp = 0.0;
        }

        Ok(())
    }

    fn get_values(&self) -> HashMap<String, Value> {
        let mut values = HashMap::new();

        // Temperature value - MUST provide "value" key for displayers
        values.insert("value".to_string(), Value::from(self.current_temp));
        values.insert("temperature".to_string(), Value::from(self.current_temp)); // Keep for compatibility

        // Sensor label
        let sensor_label = SYSTEM_SENSORS
            .get(self.config.sensor_index)
            .map(|s| s.label.as_str())
            .unwrap_or("Unknown");
        values.insert("sensor_label".to_string(), Value::from(sensor_label));

        // Unit
        values.insert("unit".to_string(), Value::from(self.unit_suffix()));

        // Caption (custom or auto-generated)
        let caption = self.config.custom_caption.clone().unwrap_or_else(|| {
            format!("{} Temp", sensor_label)
        });
        values.insert("caption".to_string(), Value::from(caption));

        // Limits
        let min_limit = if self.config.auto_detect_limits {
            self.detected_min
        } else {
            self.config.min_limit
        };

        let max_limit = if self.config.auto_detect_limits {
            self.detected_max
        } else {
            self.config.max_limit
        };

        // Only provide limits if we have a valid range (min < max)
        // This prevents the arc displayer from showing 0 when min == max
        if let (Some(min), Some(max)) = (min_limit, max_limit) {
            if max > min {
                values.insert("min_limit".to_string(), Value::from(min));
                values.insert("max_limit".to_string(), Value::from(max));
            } else if !self.config.auto_detect_limits {
                // For manual limits, always provide them even if equal
                // (user explicitly set them, might be intentional)
                values.insert("min_limit".to_string(), Value::from(min));
                values.insert("max_limit".to_string(), Value::from(max));
            }
            // For auto-detect with min == max, don't provide limits yet
            // Let the displayer use fallback logic (percentage mode)
        } else if min_limit.is_some() || max_limit.is_some() {
            // Provide partial limits if available (one but not both)
            if let Some(min) = min_limit {
                values.insert("min_limit".to_string(), Value::from(min));
            }
            if let Some(max) = max_limit {
                values.insert("max_limit".to_string(), Value::from(max));
            }
        }

        values
    }

    fn configure(&mut self, config: &HashMap<String, Value>) -> Result<()> {
        if let Some(config_value) = config.get("system_temp_config") {
            self.config = serde_json::from_value(config_value.clone())?;

            // Reset detected limits when configuration changes
            if self.config.auto_detect_limits {
                self.detected_min = None;
                self.detected_max = None;
            }
        }
        Ok(())
    }
}
