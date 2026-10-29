//! Fan speed data source implementation
//!
//! Provides access to all available fan speed sensors on the system,
//! including CPU fans, chassis fans, GPU fans, and other cooling fans.
//!
//! A fan is identified in saved configs by its unique label (see
//! `super::hwmon` for the identity rules). The numeric index is only used for
//! configs written before labels existed and is interpreted against the raw
//! discovery order those configs were saved with.

use anyhow::Result;
use once_cell::sync::Lazy;
use rg_sens_core::{
    DataSource, FieldMetadata, FieldPurpose, FieldType, SourceConfig, SourceMetadata,
};
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;

use super::hwmon::{self, HwmonSensor, SensorIdentity};

// Re-export fan speed source config types from rg-sens-types
pub use rg_sens_types::source_configs::fan_speed::{FanCategory, FanInfo, FanSpeedConfig};

impl SensorIdentity for FanInfo {
    fn label(&self) -> &str {
        &self.label
    }
    fn base_label(&self) -> &str {
        &self.base_label
    }
}

struct FanDiscovery {
    /// Fans sorted by (driver name, device, fan number), with unique labels
    sensors: Vec<FanInfo>,
    /// Raw discovery position -> index into `sensors` (legacy index-only configs)
    raw_order: Vec<usize>,
}

/// Cached fan sensor information (discovered once at startup)
static FAN_DISCOVERY: Lazy<FanDiscovery> = Lazy::new(|| {
    log::warn!("=== Discovering fan speed sensors (one-time initialization) ===");

    let (sorted, raw_order) = hwmon::finalize(discover_all_fans());
    let sensors: Vec<FanInfo> = sorted
        .into_iter()
        .enumerate()
        .map(|(index, s)| {
            let category = categorize_fan(&s.base_label);
            log::info!(
                "  [{}] {:?}: {} ({})",
                index,
                category,
                s.label,
                s.path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default()
            );
            FanInfo {
                index,
                label: s.label,
                category,
                base_label: s.base_label,
                path: s.path,
            }
        })
        .collect();

    log::warn!(
        "Fan speed discovery complete: {} sensors found",
        sensors.len()
    );

    FanDiscovery { sensors, raw_order }
});

/// Resolve the fan a config refers to and normalize the config to it
/// (index set to this run's position, label filled in / upgraded to the
/// unique form). Returns `None` when the saved fan is not present; the
/// config is left untouched in that case so the user's choice is preserved.
pub fn resolve_fan_index(config: &mut FanSpeedConfig) -> Option<usize> {
    let discovery = &*FAN_DISCOVERY;
    let idx = hwmon::resolve_sensor(
        &discovery.sensors,
        config.sensor_label.as_deref(),
        config.sensor_index,
        Some(&discovery.raw_order),
    )?;
    set_fan_by_index(config, idx);
    Some(idx)
}

/// Select a fan by its position in `available_sensors()`, storing both the
/// index and the label (the label is what survives restarts).
pub fn set_fan_by_index(config: &mut FanSpeedConfig, index: usize) {
    config.sensor_index = index;
    config.sensor_label = FAN_DISCOVERY.sensors.get(index).map(|f| f.label.clone());
}

/// Discover all fan speed sensors on the system, in raw sysfs order
#[cfg(target_os = "linux")]
fn discover_all_fans() -> Vec<HwmonSensor> {
    log::info!("Scanning for fan speed sensors...");

    // Keep every fan that exposes a readable input file regardless of the
    // current speed: zero-RPM-idle fans (GPU fan-stop, PSU hybrid mode) are
    // legitimate sensors.
    let fans: Vec<HwmonSensor> = hwmon::scan("fan", get_fan_label)
        .into_iter()
        .filter(|fan| {
            let readable = fan.path.as_deref().and_then(hwmon::read_i64).is_some();
            if !readable {
                log::debug!("Skipping unreadable fan input {:?}", fan.path);
            }
            readable
        })
        .collect();

    log::info!("Total fan speed sensors discovered: {}", fans.len());
    fans
}

#[cfg(not(target_os = "linux"))]
fn discover_all_fans() -> Vec<HwmonSensor> {
    log::warn!("Fan speed monitoring is currently only supported on Linux");
    Vec::new()
}

/// Label for a fan: the driver's `fanN_label` when present, otherwise
/// "{driver name} Fan N"
#[cfg(target_os = "linux")]
fn get_fan_label(hwmon_dir: &std::path::Path, name: &str, fan_num: u32) -> String {
    if let Some(label) = hwmon::read_trimmed(&hwmon_dir.join(format!("fan{}_label", fan_num))) {
        if !label.is_empty() {
            return label;
        }
    }
    format!("{} Fan {}", name, fan_num)
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
    if label_lower.contains("psu") || label_lower.contains("power supply") {
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
    /// Index into `FAN_DISCOVERY.sensors`; `None` when the configured fan is
    /// not present on this system (published as "N/A")
    resolved: Option<usize>,
    /// `None` when the sensor could not be read (published as "N/A")
    current_rpm: Option<f64>,
    detected_min: Option<f64>,
    detected_max: Option<f64>,

    /// Cached output values - updated in update(), returned by reference in values_ref()
    values: HashMap<String, Value>,
}

impl FanSpeedSource {
    pub fn new() -> Self {
        // Resolving the default config also forces sensor discovery
        let mut config = FanSpeedConfig::default();
        let resolved = resolve_fan_index(&mut config);

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
            config,
            resolved,
            current_rpm: None,
            detected_min: None,
            detected_max: None,
            values: HashMap::with_capacity(8),
        }
    }

    /// Get list of all available fan sensors
    pub fn available_sensors() -> &'static [FanInfo] {
        &FAN_DISCOVERY.sensors
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
        let fan = self.resolved.and_then(|idx| FAN_DISCOVERY.sensors.get(idx));

        // Read the selected fan through the path captured at discovery
        self.current_rpm = fan.and_then(|f| f.path.as_deref()).and_then(|path| {
            let rpm = hwmon::read_i64(path);
            if rpm.is_none() {
                log::warn!("Failed to read fan speed from {:?}", path);
            }
            rpm.map(|v| v as f64)
        });

        // Update detected limits if auto-detect is enabled (only from real reads)
        if let Some(rpm) = self.current_rpm {
            if self.config.auto_detect_limits {
                // Update min
                self.detected_min = Some(self.detected_min.map(|min| min.min(rpm)).unwrap_or(rpm));

                // Update max
                self.detected_max = Some(self.detected_max.map(|max| max.max(rpm)).unwrap_or(rpm));
            }
        }

        // Build values HashMap (reuse allocation, just clear and refill)
        self.values.clear();

        // Fan speed value - MUST provide "value" key for displayers.
        // On read failure (or a fan that is not present) publish "N/A" (same
        // convention as cpu/gpu/disk) instead of a misleading 0.
        match self.current_rpm {
            Some(rpm) => {
                self.values.insert("value".to_string(), Value::from(rpm));
                self.values.insert("rpm".to_string(), Value::from(rpm)); // Keep for compatibility
            }
            None => {
                self.values.insert("value".to_string(), Value::from("N/A"));
                self.values.insert("rpm".to_string(), Value::from("N/A"));
            }
        }

        // Sensor label: the discovered label, or the saved one when the fan
        // is missing so the panel still says which fan it is waiting for
        let sensor_label = fan
            .map(|f| f.label.clone())
            .or_else(|| self.config.sensor_label.clone())
            .unwrap_or_else(|| "Unknown".to_string());
        self.values.insert(
            "sensor_label".to_string(),
            Value::from(sensor_label.as_str()),
        );

        // Unit (empty when the value is N/A, matching cpu/gpu/disk)
        let unit = if self.current_rpm.is_some() {
            "RPM"
        } else {
            ""
        };
        self.values.insert("unit".to_string(), Value::from(unit));

        // Caption (custom or auto-generated)
        let caption = self
            .config
            .custom_caption
            .clone()
            .unwrap_or_else(|| sensor_label.clone());
        self.values
            .insert("caption".to_string(), Value::from(caption));

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
                self.values
                    .insert("min_limit".to_string(), Value::from(min));
                self.values
                    .insert("max_limit".to_string(), Value::from(max));
            } else if !self.config.auto_detect_limits {
                // For manual limits, always provide them even if equal
                // (user explicitly set them, might be intentional)
                self.values
                    .insert("min_limit".to_string(), Value::from(min));
                self.values
                    .insert("max_limit".to_string(), Value::from(max));
            }
            // For auto-detect with min == max, don't provide limits yet
            // Let the displayer use fallback logic (percentage mode)
        } else if min_limit.is_some() || max_limit.is_some() {
            // Provide partial limits if available (one but not both)
            if let Some(min) = min_limit {
                self.values
                    .insert("min_limit".to_string(), Value::from(min));
            }
            if let Some(max) = max_limit {
                self.values
                    .insert("max_limit".to_string(), Value::from(max));
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
            let mut new_config: FanSpeedConfig = serde_json::from_value(config_value.clone())?;

            // Label first, legacy index only for label-less configs. The
            // config widget always writes both, so there is no need to guess
            // which one the user changed.
            self.resolved = resolve_fan_index(&mut new_config);
            if self.resolved.is_none() {
                log::warn!(
                    "Fan {} is not present on this system; publishing N/A until another fan is selected",
                    describe_selection(&new_config)
                );
            }
            self.config = new_config;

            // Reset detected limits when configuration changes
            if self.config.auto_detect_limits {
                self.detected_min = None;
                self.detected_max = None;
            }
        }
        Ok(())
    }

    fn get_typed_config(&self) -> Option<SourceConfig> {
        Some(SourceConfig::FanSpeed(self.config.clone()))
    }
}

/// "'label'" or "#index" for log/UI messages about a saved selection
pub fn describe_selection(config: &FanSpeedConfig) -> String {
    match config.sensor_label.as_deref() {
        Some(label) if !label.is_empty() => format!("'{}'", label),
        _ => format!("#{}", config.sensor_index),
    }
}

impl Default for FanSpeedSource {
    fn default() -> Self {
        Self::new()
    }
}
