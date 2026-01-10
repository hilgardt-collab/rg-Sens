//! Fan speed data source implementation
//!
//! Provides access to all available fan speed sensors on the system,
//! including CPU fans, chassis fans, GPU fans, and other cooling fans.

use crate::core::{DataSource, FieldMetadata, FieldPurpose, FieldType, SourceMetadata};
use anyhow::Result;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;

/// Information about a discovered fan sensor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FanInfo {
    pub index: usize,
    pub label: String,
    pub category: FanCategory,
    /// Path to the fan input file (e.g., /sys/class/hwmon/hwmon0/fan1_input)
    #[serde(skip)]
    pub path: Option<std::path::PathBuf>,
}

/// Category of fan sensor
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum FanCategory {
    #[serde(rename = "cpu")]
    CPU,
    #[serde(rename = "gpu")]
    GPU,
    #[serde(rename = "chassis")]
    Chassis,
    #[serde(rename = "psu")]
    PSU,
    #[serde(rename = "other")]
    Other,
}

/// Configuration for fan speed source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FanSpeedConfig {
    #[serde(default)]
    pub sensor_index: usize,
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

impl Default for FanSpeedConfig {
    fn default() -> Self {
        Self {
            sensor_index: 0,
            update_interval_ms: default_update_interval(),
            custom_caption: None,
            min_limit: None,
            max_limit: None,
            auto_detect_limits: default_auto_detect_limits(),
        }
    }
}

/// Cached fan sensor information (discovered once at startup)
static FAN_SENSORS: Lazy<Vec<FanInfo>> = Lazy::new(|| {
    log::warn!("=== Discovering fan speed sensors (one-time initialization) ===");

    let fans = discover_all_fans();

    log::warn!("Fan speed discovery complete: {} sensors found", fans.len());

    fans
});

/// Discover all fan speed sensors on the system
fn discover_all_fans() -> Vec<FanInfo> {
    let mut fans = Vec::new();

    log::info!("Scanning for fan speed sensors...");

    #[cfg(target_os = "linux")]
    {
        // On Linux, read from /sys/class/hwmon
        if let Ok(entries) = std::fs::read_dir("/sys/class/hwmon") {
            for entry in entries.flatten() {
                let path = entry.path();

                // Look for fan input files
                if let Ok(files) = std::fs::read_dir(&path) {
                    for file in files.flatten() {
                        let filename = file.file_name();
                        let filename_str = filename.to_string_lossy();

                        // Match fan*_input files
                        if filename_str.starts_with("fan") && filename_str.ends_with("_input") {
                            let fan_path = file.path();

                            // Try to read the fan speed
                            if let Ok(speed_str) = std::fs::read_to_string(&fan_path) {
                                if let Ok(speed) = speed_str.trim().parse::<i32>() {
                                    if speed > 0 {
                                        // Get the label for this fan
                                        let label = get_fan_label(&path, &filename_str);
                                        let category = categorize_fan(&label);

                                        let index = fans.len();
                                        fans.push(FanInfo {
                                            index,
                                            label: label.clone(),
                                            category,
                                            path: Some(fan_path.clone()),
                                        });

                                        log::info!("  [{}] {:?}: {} = {} RPM ({})", index, category, label, speed, fan_path.display());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        log::warn!("Fan speed monitoring is currently only supported on Linux");
    }

    log::info!("Total fan speed sensors discovered: {}", fans.len());
    fans
}

/// Get the label for a fan sensor
#[cfg(target_os = "linux")]
fn get_fan_label(hwmon_path: &std::path::Path, fan_filename: &str) -> String {
    // Try to read the fan label file
    let fan_num = fan_filename
        .trim_start_matches("fan")
        .trim_end_matches("_input");

    let label_path = hwmon_path.join(format!("fan{}_label", fan_num));
    if let Ok(label) = std::fs::read_to_string(&label_path) {
        return label.trim().to_string();
    }

    // Try to read the device name
    let name_path = hwmon_path.join("name");
    if let Ok(name) = std::fs::read_to_string(&name_path) {
        return format!("{} Fan {}", name.trim(), fan_num);
    }

    // Fallback to hwmon name and fan number
    if let Some(hwmon_name) = hwmon_path.file_name() {
        return format!("{} Fan {}", hwmon_name.to_string_lossy(), fan_num);
    }

    format!("Fan {}", fan_num)
}

/// Categorize a fan based on its label
fn categorize_fan(label: &str) -> FanCategory {
    let label_lower = label.to_lowercase();

    // CPU fans
    if label_lower.contains("cpu")
        || label_lower.contains("processor")
        || label_lower.contains("cpu_fan")
    {
        return FanCategory::CPU;
    }

    // GPU fans
    if label_lower.contains("gpu")
        || label_lower.contains("nvidia")
        || label_lower.contains("amdgpu")
        || label_lower.contains("radeon")
        || label_lower.contains("vga")
        || label_lower.contains("video")
    {
        return FanCategory::GPU;
    }

    // PSU fans
    if label_lower.contains("psu")
        || label_lower.contains("power supply")
    {
        return FanCategory::PSU;
    }

    // Chassis/Case fans
    if label_lower.contains("chassis")
        || label_lower.contains("case")
        || label_lower.contains("sys")
        || label_lower.contains("intake")
        || label_lower.contains("exhaust")
    {
        return FanCategory::Chassis;
    }

    // Default to Other
    FanCategory::Other
}

/// Fan speed data source
pub struct FanSpeedSource {
    metadata: SourceMetadata,
    config: FanSpeedConfig,
    current_rpm: f64,
    detected_min: Option<f64>,
    detected_max: Option<f64>,

    /// Cached output values - updated in update(), returned by reference in values_ref()
    values: HashMap<String, Value>,
}

impl FanSpeedSource {
    pub fn new() -> Self {
        // Force initialization of sensor discovery
        let _ = &*FAN_SENSORS;

        Self {
            metadata: SourceMetadata {
                id: "fan_speed".to_string(),
                name: "Fan Speed".to_string(),
                description: "RPM from any system fan sensor (CPU, GPU, chassis, etc.)".to_string(),
                available_keys: vec![
                    "rpm".to_string(),
                    "sensor_label".to_string(),
                    "unit".to_string(),
                    "caption".to_string(),
                    "min_limit".to_string(),
                    "max_limit".to_string(),
                ],
                default_interval: Duration::from_millis(1000),
            },
            config: FanSpeedConfig::default(),
            current_rpm: 0.0,
            detected_min: None,
            detected_max: None,
            values: HashMap::with_capacity(8),
        }
    }

    /// Get list of all available fan sensors
    pub fn available_sensors() -> &'static [FanInfo] {
        &FAN_SENSORS
    }
}


impl DataSource for FanSpeedSource {
    fn metadata(&self) -> &SourceMetadata {
        &self.metadata
    }

    fn fields(&self) -> Vec<FieldMetadata> {
        vec![
            FieldMetadata::new(
                "value",
                "Value",
                "Current fan speed in RPM",
                FieldType::Numerical,
                FieldPurpose::Value,
            ),
            FieldMetadata::new(
                "caption",
                "Caption",
                "Display caption for the fan sensor",
                FieldType::Text,
                FieldPurpose::Caption,
            ),
            FieldMetadata::new(
                "unit",
                "Unit",
                "Fan speed unit (RPM)",
                FieldType::Text,
                FieldPurpose::Unit,
            ),
            FieldMetadata::new(
                "sensor_label",
                "Sensor",
                "Name of the selected fan sensor",
                FieldType::Text,
                FieldPurpose::Caption,
            ),
            FieldMetadata::new(
                "min_limit",
                "Min Limit",
                "Minimum RPM limit for visualization",
                FieldType::Numerical,
                FieldPurpose::SecondaryValue,
            ),
            FieldMetadata::new(
                "max_limit",
                "Max Limit",
                "Maximum RPM limit for visualization",
                FieldType::Numerical,
                FieldPurpose::SecondaryValue,
            ),
        ]
    }

    fn update(&mut self) -> Result<()> {
        // Read the selected fan sensor using the path stored in FAN_SENSORS
        // This ensures we always read from the correct fan file
        if let Some(fan_info) = FAN_SENSORS.get(self.config.sensor_index) {
            if let Some(ref fan_path) = fan_info.path {
                if let Ok(speed_str) = std::fs::read_to_string(fan_path) {
                    if let Ok(speed) = speed_str.trim().parse::<i32>() {
                        self.current_rpm = speed as f64;

                        // Update detected limits if auto-detect is enabled
                        if self.config.auto_detect_limits {
                            // Update min
                            self.detected_min = Some(
                                self.detected_min
                                    .map(|min| min.min(self.current_rpm))
                                    .unwrap_or(self.current_rpm)
                            );

                            // Update max
                            self.detected_max = Some(
                                self.detected_max
                                    .map(|max| max.max(self.current_rpm))
                                    .unwrap_or(self.current_rpm)
                            );
                        }
                    } else {
                        log::warn!("Failed to parse fan speed from {:?}", fan_path);
                        self.current_rpm = 0.0;
                    }
                } else {
                    log::warn!("Failed to read fan speed from {:?}", fan_path);
                    self.current_rpm = 0.0;
                }
            } else {
                log::warn!("Fan sensor {} has no path stored", self.config.sensor_index);
                self.current_rpm = 0.0;
            }
        } else {
            log::warn!("Selected fan sensor index {} not found in FAN_SENSORS", self.config.sensor_index);
            self.current_rpm = 0.0;
        }

        // Build values HashMap (reuse allocation, just clear and refill)
        self.values.clear();

        // Fan speed value - MUST provide "value" key for displayers
        self.values.insert("value".to_string(), Value::from(self.current_rpm));
        self.values.insert("rpm".to_string(), Value::from(self.current_rpm)); // Keep for compatibility

        // Sensor label
        let sensor_label = FAN_SENSORS
            .get(self.config.sensor_index)
            .map(|s| s.label.as_str())
            .unwrap_or("Unknown");
        self.values.insert("sensor_label".to_string(), Value::from(sensor_label));

        // Unit
        self.values.insert("unit".to_string(), Value::from("RPM"));

        // Caption (custom or auto-generated)
        let caption = self.config.custom_caption.clone().unwrap_or_else(|| {
            sensor_label.to_string()
        });
        self.values.insert("caption".to_string(), Value::from(caption));

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
                self.values.insert("min_limit".to_string(), Value::from(min));
                self.values.insert("max_limit".to_string(), Value::from(max));
            } else if !self.config.auto_detect_limits {
                // For manual limits, always provide them even if equal
                // (user explicitly set them, might be intentional)
                self.values.insert("min_limit".to_string(), Value::from(min));
                self.values.insert("max_limit".to_string(), Value::from(max));
            }
            // For auto-detect with min == max, don't provide limits yet
            // Let the displayer use fallback logic (percentage mode)
        } else if min_limit.is_some() || max_limit.is_some() {
            // Provide partial limits if available (one but not both)
            if let Some(min) = min_limit {
                self.values.insert("min_limit".to_string(), Value::from(min));
            }
            if let Some(max) = max_limit {
                self.values.insert("max_limit".to_string(), Value::from(max));
            }
        }

        Ok(())
    }

    fn get_values(&self) -> HashMap<String, Value> {
        self.values.clone()
    }

    fn values_ref(&self) -> Option<&HashMap<String, Value>> {
        Some(&self.values)
    }

    fn configure(&mut self, config: &HashMap<String, Value>) -> Result<()> {
        if let Some(config_value) = config.get("fan_speed_config") {
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

impl Default for FanSpeedSource {
    fn default() -> Self {
        Self::new()
    }
}
