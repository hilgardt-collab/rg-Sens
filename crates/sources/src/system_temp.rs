//! System temperature data source implementation
//!
//! Provides access to all available temperature sensors on the system,
//! including motherboard, drives, GPU packages, and other hardware sensors.
//!
//! Sensors are read straight from hwmon (`tempN_input`) so that the list is
//! deterministic and each sensor has a unique label; see `super::hwmon` for
//! the identity rules. Labels follow sysinfo's format exactly, because that
//! is what configs saved by earlier versions contain.

use anyhow::Result;
use once_cell::sync::Lazy;
use rg_sens_core::{
    DataSource, FieldMetadata, FieldPurpose, FieldType, SourceConfig, SourceMetadata,
};
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;

use super::hwmon::{self, HwmonSensor, SensorIdentity};

// Re-export system temp source config types from rg-sens-types
pub use rg_sens_types::source_configs::system_temp::{
    SensorCategory, SensorInfo, SystemTempConfig, TemperatureUnit,
};

impl SensorIdentity for SensorInfo {
    fn label(&self) -> &str {
        &self.label
    }
    fn base_label(&self) -> &str {
        &self.base_label
    }
}

/// Cached sensor information (discovered once at startup), sorted by
/// (driver name, device, channel) with unique labels
static SYSTEM_SENSORS: Lazy<Vec<SensorInfo>> = Lazy::new(|| {
    log::warn!("=== Discovering system temperature sensors (one-time initialization) ===");

    let (sorted, _raw_order) = hwmon::finalize(discover_all_temps());
    let sensors: Vec<SensorInfo> = sorted
        .into_iter()
        .enumerate()
        .map(|(index, s)| {
            let category = categorize_sensor(&s.base_label);
            log::info!("  [{}] {:?}: {}", index, category, s.label);
            SensorInfo {
                index,
                label: s.label,
                category,
                base_label: s.base_label,
                path: s.path,
            }
        })
        .collect();

    log::warn!(
        "System temperature discovery complete: {} sensors found",
        sensors.len()
    );

    sensors
});

/// Resolve the sensor a config refers to and normalize the config to it
/// (index set to this run's position, label filled in / upgraded to the
/// unique form). Returns `None` when the saved sensor is not present; the
/// config is left untouched in that case so the user's choice is preserved.
pub fn resolve_sensor_index(config: &mut SystemTempConfig) -> Option<usize> {
    // No legacy order: before this, indices came from sysinfo's per-process
    // random ordering, so an index-only config never meant anything stable.
    let idx = hwmon::resolve_sensor(
        &SYSTEM_SENSORS,
        config.sensor_label.as_deref(),
        config.sensor_index,
        None,
    )?;
    set_sensor_by_index(config, idx);
    Some(idx)
}

/// Select a sensor by its position in `available_sensors()`, storing both the
/// index and the label (the label is what survives restarts).
pub fn set_sensor_by_index(config: &mut SystemTempConfig, index: usize) {
    config.sensor_index = index;
    config.sensor_label = SYSTEM_SENSORS.get(index).map(|s| s.label.clone());
}

/// Discover all temperature sensors, in raw sysfs order
#[cfg(target_os = "linux")]
fn discover_all_temps() -> Vec<HwmonSensor> {
    log::info!("Scanning for temperature sensors...");

    let mut sensors = hwmon::scan("temp", |dir, name, n| {
        let label = hwmon::read_trimmed(&dir.join(format!("temp{}_label", n))).unwrap_or_default();
        let model = hwmon::read_trimmed(&dir.join("device/model"));
        format_temp_label(name, &label, model.as_deref(), n)
    });

    if sensors.is_empty() {
        // Boards without hwmon temperature drivers (Raspberry Pi) expose the
        // SoC temperature through the thermal zone instead
        sensors = hwmon::scan_thermal_zones();
    }

    log::info!("Total temperature sensors discovered: {}", sensors.len());
    sensors
}

/// Non-Linux: discover through sysinfo (no device identity available, so no
/// collision handling; sensors are read back by label)
#[cfg(not(target_os = "linux"))]
fn discover_all_temps() -> Vec<HwmonSensor> {
    super::shared_sensors::get_refreshed_temperatures()
        .into_iter()
        .enumerate()
        .map(|(i, (label, _temp))| HwmonSensor {
            name: label.clone(),
            device_id: String::new(),
            number: i as u32,
            base_label: label.clone(),
            label,
            path: None,
            raw_pos: 0,
        })
        .collect()
}

/// Build the label the way sysinfo 0.31 does (`Component::label` on Linux),
/// so sensors keep the names that existing saved configs refer to:
///
/// | `tempN_label` | `device/model` | label                                |
/// |---------------|----------------|--------------------------------------|
/// | yes           | yes            | `{name} {label} {model} tempN`       |
/// | yes           | no             | `{name} {label}`                     |
/// | no            | yes            | `{name} {model}`                     |
/// | no            | no             | `{name} tempN`                       |
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn format_temp_label(name: &str, label: &str, model: Option<&str>, n: u32) -> String {
    match (label.is_empty(), model) {
        (false, Some(model)) => format!("{} {} {} temp{}", name, label, model, n),
        (false, None) => format!("{} {}", name, label),
        (true, Some(model)) => format!("{} {}", name, model),
        (true, None) => format!("{} temp{}", name, n),
    }
}

/// Current reading of a discovered sensor in degrees Celsius
fn read_sensor(info: &SensorInfo) -> Option<f32> {
    match &info.path {
        // hwmon reports millidegrees
        Some(path) => hwmon::read_i64(path).map(|v| v as f32 / 1000.0),
        None => super::shared_sensors::get_temperature_by_label(&info.base_label),
    }
    .filter(|t| t.is_finite())
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
    /// Index into `SYSTEM_SENSORS`; `None` when the configured sensor is not
    /// present on this system (published as "N/A")
    resolved: Option<usize>,
    /// `None` when the sensor could not be read (published as "N/A")
    current_temp: Option<f64>,
    detected_min: Option<f64>,
    detected_max: Option<f64>,

    /// Cached output values - updated in update(), returned by reference in values_ref()
    values: HashMap<String, Value>,
}

impl Default for SystemTempSource {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemTempSource {
    pub fn new() -> Self {
        // Resolving the default config also forces sensor discovery
        let mut config = SystemTempConfig::default();
        let resolved = resolve_sensor_index(&mut config);

        Self {
            metadata: SourceMetadata {
                id: "system_temp".to_string(),
                name: "System Temperature".to_string(),
                description: "Temperature from any system sensor (motherboard, drives, etc.)"
                    .to_string(),
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
            config,
            resolved,
            current_temp: None,
            detected_min: None,
            detected_max: None,
            values: HashMap::with_capacity(8),
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
        let sensor = self.resolved.and_then(|idx| SYSTEM_SENSORS.get(idx));

        let celsius = sensor.and_then(read_sensor);
        if let (Some(s), None) = (sensor, celsius) {
            log::warn!("Failed to read temperature sensor '{}'", s.label);
        }
        self.current_temp = celsius.map(|c| self.convert_temperature(c as f64));

        // Update detected limits if auto-detect is enabled (only from real reads)
        if let Some(temp) = self.current_temp {
            if self.config.auto_detect_limits {
                // Update min
                self.detected_min =
                    Some(self.detected_min.map(|min| min.min(temp)).unwrap_or(temp));

                // Update max
                self.detected_max =
                    Some(self.detected_max.map(|max| max.max(temp)).unwrap_or(temp));
            }
        }

        // Build values HashMap (reuse allocation, just clear and refill)
        self.values.clear();

        // Temperature value - MUST provide "value" key for displayers.
        // On read failure (or a sensor that is not present) publish "N/A"
        // (same convention as cpu/gpu/disk) instead of a misleading 0.
        match self.current_temp {
            Some(temp) => {
                self.values.insert("value".to_string(), Value::from(temp));
                self.values
                    .insert("temperature".to_string(), Value::from(temp)); // Keep for compatibility
            }
            None => {
                self.values.insert("value".to_string(), Value::from("N/A"));
                self.values
                    .insert("temperature".to_string(), Value::from("N/A"));
            }
        }

        // Sensor label: the discovered label, or the saved one when the sensor
        // is missing so the panel still says which sensor it is waiting for
        let sensor_label = sensor
            .map(|s| s.label.clone())
            .or_else(|| self.config.sensor_label.clone())
            .unwrap_or_else(|| "Unknown".to_string());
        self.values.insert(
            "sensor_label".to_string(),
            Value::from(sensor_label.as_str()),
        );

        // Unit (empty when the value is N/A, matching cpu/gpu/disk)
        let unit = if self.current_temp.is_some() {
            self.unit_suffix()
        } else {
            ""
        };
        self.values.insert("unit".to_string(), Value::from(unit));

        // Caption (custom or auto-generated)
        let caption = self
            .config
            .custom_caption
            .clone()
            .unwrap_or_else(|| format!("{} Temp", sensor_label));
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
        if let Some(config_value) = config.get("system_temp_config") {
            let mut new_config: SystemTempConfig = serde_json::from_value(config_value.clone())?;

            // Label first; a missing sensor stays missing (N/A) rather than
            // silently becoming whichever sensor sits at the saved index
            self.resolved = resolve_sensor_index(&mut new_config);
            if self.resolved.is_none() {
                log::warn!(
                    "Temperature sensor {} is not present on this system; publishing N/A until another sensor is selected",
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
        Some(SourceConfig::SystemTemp(self.config.clone()))
    }
}

/// "'label'" or "#index" for log/UI messages about a saved selection
pub fn describe_selection(config: &SystemTempConfig) -> String {
    match config.sensor_label.as_deref() {
        Some(label) if !label.is_empty() => format!("'{}'", label),
        _ => format!("#{}", config.sensor_index),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temp_labels_match_sysinfo_format() {
        // Values seen in real saved configs
        assert_eq!(
            format_temp_label("nvme", "Composite", Some("Samsung SSD 9100 PRO 4TB"), 1),
            "nvme Composite Samsung SSD 9100 PRO 4TB temp1"
        );
        assert_eq!(format_temp_label("it87952", "", None, 3), "it87952 temp3");
        assert_eq!(
            format_temp_label("r8169_0_e00:00", "", None, 1),
            "r8169_0_e00:00 temp1"
        );
        assert_eq!(
            format_temp_label("k10temp", "Tctl", None, 1),
            "k10temp Tctl"
        );
        assert_eq!(
            format_temp_label("drivetemp", "", Some("WDC WD40EFRX"), 1),
            "drivetemp WDC WD40EFRX"
        );
    }

    #[test]
    fn describe_selection_prefers_label() {
        let mut cfg = SystemTempConfig::default();
        cfg.sensor_index = 4;
        assert_eq!(describe_selection(&cfg), "#4");
        cfg.sensor_label = Some(String::new());
        assert_eq!(describe_selection(&cfg), "#4");
        cfg.sensor_label = Some("k10temp Tctl".into());
        assert_eq!(describe_selection(&cfg), "'k10temp Tctl'");
    }
}
