//! CPU data source implementation

use crate::core::{DataSource, FieldMetadata, FieldPurpose, FieldType, SourceMetadata};
use crate::ui::{CpuField, CpuSourceConfig, CoreSelection, FrequencyUnit, TemperatureUnit};
use anyhow::Result;
use once_cell::sync::Lazy;
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;
use sysinfo::{CpuRefreshKind, RefreshKind, System};

use super::shared_sensors;

/// Information about a discovered CPU temperature sensor
#[derive(Debug, Clone)]
pub struct CpuSensor {
    pub index: usize,
    pub label: String,
}

/// Cached CPU hardware information (discovered once on first access)
struct CpuHardwareInfo {
    sensors: Vec<CpuSensor>,
    core_count: usize,
}

/// Global cache for CPU hardware info (discovered once at startup)
static CPU_HARDWARE_INFO: Lazy<CpuHardwareInfo> = Lazy::new(|| {
    log::info!("=== Discovering CPU hardware (one-time initialization) ===");

    // Get sensor info from shared components (already initialized)
    let all_temps = shared_sensors::get_refreshed_temperatures();
    let sensors = discover_cpu_sensors_from_list(&all_temps);

    // Create temporary system to get core count
    let system = System::new_with_specifics(
        RefreshKind::new().with_cpu(CpuRefreshKind::everything()),
    );
    let core_count = system.cpus().len();

    log::info!("CPU hardware discovery complete: {} sensors, {} cores", sensors.len(), core_count);

    CpuHardwareInfo {
        sensors,
        core_count,
    }
});

/// Discover all CPU temperature sensors from a list of (label, temperature) pairs
fn discover_cpu_sensors_from_list(temps: &[(String, f32)]) -> Vec<CpuSensor> {
    let mut sensors = Vec::new();
    let mut index = 0;

    // Log all available components for debugging
    log::info!("Discovering CPU temperature sensors...");
    log::info!("Total components found: {}", temps.len());

    for (label, temp) in temps {
        log::info!("  Component: {} = {}°C", label, temp);
    }

    // Collect all CPU-related sensors (don't exclude based on priority)
    for (label, _temp) in temps {
        let label_lower = label.to_lowercase();

        // Match AMD Ryzen sensors: Tctl, Tccd1, Tccd2, etc.
        // Match Intel sensors: Package, Core
        // Match generic: CPU
        if label_lower.contains("cpu")
            || label_lower.contains("package")
            || label_lower.contains("tctl")
            || label_lower.contains("tccd")
            || label_lower.contains("tdie")
            || label_lower.contains("core")
            || label_lower.starts_with("k10temp") {

            sensors.push(CpuSensor {
                index,
                label: label.clone(),
            });
            log::info!("  Added sensor {}: {}", index, label);
            index += 1;
        }
    }

    log::info!("Total CPU sensors discovered: {}", sensors.len());
    sensors
}

/// CPU data source
///
/// Provides comprehensive CPU information including usage, temperature, frequency,
/// and per-core statistics using sysinfo crate.
pub struct CpuSource {
    metadata: SourceMetadata,
    system: System,
    global_usage: f32,
    per_core_usage: Vec<f32>,
    cpu_sensors: Vec<CpuSensor>,
    cpu_temperature: Option<f32>,
    cpu_frequency: f64,
    config: CpuSourceConfig,
    detected_min: Option<f64>,
    detected_max: Option<f64>,
    update_count: usize,
}

impl CpuSource {
    pub fn new() -> Self {
        let metadata = SourceMetadata {
            id: "cpu".to_string(),
            name: "CPU Information".to_string(),
            description: "Comprehensive CPU metrics including usage, temperature, and frequency".to_string(),
            available_keys: vec![
                "caption".to_string(),
                "usage".to_string(),
                "unit".to_string(),
                "temperature".to_string(),
                "temp_unit".to_string(),
                "frequency".to_string(),
                "freq_unit".to_string(),
            ],
            default_interval: Duration::from_millis(1000),
        };

        // Initialize system with CPU refresh configuration
        let system = System::new_with_specifics(
            RefreshKind::new().with_cpu(CpuRefreshKind::everything()),
        );

        // Use cached sensor list from global hardware info
        // Note: Temperature readings use shared_sensors module, not a local Components instance
        let cpu_sensors = CPU_HARDWARE_INFO.sensors.clone();

        Self {
            metadata,
            system,
            global_usage: 0.0,
            per_core_usage: Vec::new(),
            cpu_sensors,
            cpu_temperature: None,
            cpu_frequency: 0.0,
            config: CpuSourceConfig::default(),
            detected_min: None,
            detected_max: None,
            update_count: 0,
        }
    }

    /// Get list of available CPU temperature sensors from cache (no instance needed)
    pub fn get_cached_sensors() -> &'static [CpuSensor] {
        &CPU_HARDWARE_INFO.sensors
    }

    /// Get CPU core count from cache (no instance needed)
    pub fn get_cached_core_count() -> usize {
        CPU_HARDWARE_INFO.core_count
    }

    /// Get list of available CPU temperature sensors
    pub fn get_available_sensors(&self) -> &[CpuSensor] {
        &self.cpu_sensors
    }

    /// Get number of CPU cores
    pub fn get_core_count(&self) -> usize {
        self.system.cpus().len()
    }

    /// Set configuration for this CPU source
    ///
    /// This determines which data field to expose (temperature/usage/frequency),
    /// temperature units, and which core to monitor.
    pub fn set_config(&mut self, config: CpuSourceConfig) {
        self.config = config;

        // Reset auto-detection when config changes
        self.update_count = 0;
        self.detected_min = None;
        self.detected_max = None;
    }

    /// Get current configuration
    pub fn get_config(&self) -> &CpuSourceConfig {
        &self.config
    }

    /// Convert temperature from Celsius to the configured unit
    fn convert_temperature(&self, celsius: f32) -> f32 {
        match self.config.temp_unit {
            TemperatureUnit::Celsius => celsius,
            TemperatureUnit::Fahrenheit => celsius * 9.0 / 5.0 + 32.0,
            TemperatureUnit::Kelvin => celsius + 273.15,
        }
    }

    /// Get temperature unit string
    fn get_temperature_unit_string(&self) -> &str {
        match self.config.temp_unit {
            TemperatureUnit::Celsius => "°C",
            TemperatureUnit::Fahrenheit => "°F",
            TemperatureUnit::Kelvin => "K",
        }
    }

    /// Convert frequency from MHz to the configured unit
    fn convert_frequency(&self, mhz: f64) -> f64 {
        match self.config.freq_unit {
            FrequencyUnit::MHz => mhz,
            FrequencyUnit::GHz => mhz / 1000.0,
        }
    }

    /// Get frequency unit string
    fn get_frequency_unit_string(&self) -> &str {
        match self.config.freq_unit {
            FrequencyUnit::MHz => "MHz",
            FrequencyUnit::GHz => "GHz",
        }
    }

    /// Generate automatic caption based on configuration
    fn generate_auto_caption(&self) -> String {
        // Core prefix
        let core_prefix = match &self.config.core_selection {
            CoreSelection::Overall => String::new(),
            CoreSelection::Core(idx) => format!("Core {} ", idx),
        };

        // Field type
        let field_type = match self.config.field {
            CpuField::Usage => "CPU",
            CpuField::Temperature => "Temp",
            CpuField::Frequency => "Freq",
        };

        format!("{}{}", core_prefix, field_type)
    }

    /// Find CPU temperature from shared sensors using configured sensor index
    fn find_cpu_temperature(&self) -> Option<f32> {
        // If no sensors discovered, return None
        if self.cpu_sensors.is_empty() {
            log::warn!("No CPU sensors discovered, cannot read temperature");
            return None;
        }

        // Get the sensor label for the configured index
        let sensor_index = self.config.sensor_index;
        let target_label = if let Some(sensor) = self.cpu_sensors.get(sensor_index) {
            sensor.label.clone()
        } else if let Some(first_sensor) = self.cpu_sensors.first() {
            // If configured index is out of bounds, use first sensor
            log::warn!("Sensor index {} out of bounds (max: {}), using first sensor",
                      sensor_index, self.cpu_sensors.len().saturating_sub(1));
            first_sensor.label.clone()
        } else {
            // No sensors available at all (should be caught by earlier check, but be defensive)
            log::error!("No CPU temperature sensors available after check - this should not happen");
            return None;
        };

        log::debug!("Looking for temperature sensor: {}", target_label);

        // Use shared sensors to get the temperature
        shared_sensors::get_temperature_by_label(&target_label)
    }
}

impl Default for CpuSource {
    fn default() -> Self {
        Self::new()
    }
}

impl DataSource for CpuSource {
    fn metadata(&self) -> &SourceMetadata {
        &self.metadata
    }

    fn fields(&self) -> Vec<FieldMetadata> {
        let mut fields = vec![
            FieldMetadata::new(
                "caption",
                "Caption",
                "Label identifying this as CPU data",
                FieldType::Text,
                FieldPurpose::Caption,
            ),
            FieldMetadata::new(
                "value",
                "Value (Configured)",
                "The configured value (temperature/usage/frequency based on Data Source settings)",
                FieldType::Numerical,
                FieldPurpose::Value,
            ),
            FieldMetadata::new(
                "unit",
                "Unit",
                "Unit of measurement for the configured value",
                FieldType::Text,
                FieldPurpose::Unit,
            ),
            FieldMetadata::new(
                "usage",
                "Usage (Always)",
                "Overall CPU usage percentage",
                FieldType::Percentage,
                FieldPurpose::Value,
            ),
            FieldMetadata::new(
                "temperature",
                "Temperature",
                "CPU temperature",
                FieldType::Numerical,
                FieldPurpose::Value,
            ),
            FieldMetadata::new(
                "temp_unit",
                "Temperature Unit",
                "Unit of measurement for temperature",
                FieldType::Text,
                FieldPurpose::Unit,
            ),
            FieldMetadata::new(
                "frequency",
                "Frequency",
                "CPU frequency",
                FieldType::Numerical,
                FieldPurpose::Value,
            ),
            FieldMetadata::new(
                "freq_unit",
                "Frequency Unit",
                "Unit of measurement for frequency",
                FieldType::Text,
                FieldPurpose::Unit,
            ),
        ];

        // Add per-core usage fields
        for i in 0..self.per_core_usage.len() {
            fields.push(FieldMetadata::new(
                format!("core{}_usage", i),
                format!("Core {} Usage", i),
                format!("CPU core {} usage percentage", i),
                FieldType::Percentage,
                FieldPurpose::Value,
            ));
        }

        fields
    }

    fn update(&mut self) -> Result<()> {
        // Refresh CPU information
        self.system.refresh_cpu_all();

        // Update global CPU usage
        self.global_usage = self.system.global_cpu_usage();

        // Update per-core usage
        self.per_core_usage.clear();
        for cpu in self.system.cpus() {
            self.per_core_usage.push(cpu.cpu_usage());
        }

        // Get CPU frequency (from first CPU)
        if let Some(cpu) = self.system.cpus().first() {
            self.cpu_frequency = cpu.frequency() as f64;
        }

        // Get temperature from shared sensors (they handle refresh internally)
        self.cpu_temperature = self.find_cpu_temperature();

        log::debug!("CPU update complete - cores: {}, temp: {:?}, freq: {}",
                   self.per_core_usage.len(), self.cpu_temperature, self.cpu_frequency);

        // Auto-detect limits if enabled (track for first 10 updates)
        if self.config.auto_detect_limits && self.update_count < 10 {
            self.update_count += 1;

            // Get the current value based on configuration
            let current_value: Option<f64> = match self.config.field {
                CpuField::Usage => {
                    let usage = match &self.config.core_selection {
                        CoreSelection::Overall => self.global_usage,
                        CoreSelection::Core(core_idx) => {
                            self.per_core_usage.get(*core_idx).copied().unwrap_or(0.0)
                        }
                    };
                    Some(usage as f64)
                }
                CpuField::Temperature => {
                    self.cpu_temperature.map(|t| self.convert_temperature(t) as f64)
                }
                CpuField::Frequency => {
                    Some(self.convert_frequency(self.cpu_frequency))
                }
            };

            // Update detected min/max
            if let Some(value) = current_value {
                self.detected_min = Some(self.detected_min.map_or(value, |min| min.min(value)));
                self.detected_max = Some(self.detected_max.map_or(value, |max| max.max(value)));

                log::debug!("Auto-detect limits update {}: value={:.2}, min={:.2}, max={:.2}",
                           self.update_count, value,
                           self.detected_min.unwrap(), self.detected_max.unwrap());
            }
        }

        Ok(())
    }

    fn get_values(&self) -> HashMap<String, Value> {
        let mut values = HashMap::new();

        // Determine which data to use based on core selection
        let usage_value = match &self.config.core_selection {
            CoreSelection::Overall => self.global_usage,
            CoreSelection::Core(core_idx) => {
                self.per_core_usage.get(*core_idx).copied().unwrap_or(0.0)
            }
        };

        // Get frequency (could be per-core in future, for now just overall)
        let frequency_value = self.cpu_frequency;

        // Get temperature (apply conversion)
        let temperature_value = self.cpu_temperature.map(|t| self.convert_temperature(t));

        // Generate caption (use custom if provided, otherwise auto-generate)
        let caption = self.config.custom_caption
            .as_ref()
            .cloned()
            .unwrap_or_else(|| self.generate_auto_caption());

        // Apply field configuration to determine what goes in the main value/unit fields
        // Use consistent field names ("caption", "value", "unit") for easier text displayer config
        match self.config.field {
            CpuField::Usage => {
                values.insert("caption".to_string(), Value::from(caption));
                values.insert("value".to_string(), Value::from(usage_value));
                values.insert("usage".to_string(), Value::from(usage_value)); // Keep for compatibility
                values.insert("unit".to_string(), Value::from("%"));
            }
            CpuField::Temperature => {
                values.insert("caption".to_string(), Value::from(caption));
                if let Some(temp) = temperature_value {
                    values.insert("value".to_string(), Value::from(temp));
                    values.insert("temperature".to_string(), Value::from(temp)); // Keep for compatibility
                    values.insert("unit".to_string(), Value::from(self.get_temperature_unit_string()));
                } else {
                    values.insert("value".to_string(), Value::from("N/A"));
                    values.insert("temperature".to_string(), Value::from("N/A")); // Keep for compatibility
                    values.insert("unit".to_string(), Value::from(""));
                }
            }
            CpuField::Frequency => {
                let converted_freq = self.convert_frequency(frequency_value);
                values.insert("caption".to_string(), Value::from(caption));
                values.insert("value".to_string(), Value::from(converted_freq));
                values.insert("frequency".to_string(), Value::from(converted_freq)); // Keep for compatibility
                values.insert("unit".to_string(), Value::from(self.get_frequency_unit_string()));
            }
        }

        // Also provide all raw data for advanced use cases
        values.insert("raw_usage".to_string(), Value::from(self.global_usage));

        if let Some(temp) = self.cpu_temperature {
            values.insert("raw_temperature_celsius".to_string(), Value::from(temp));
        }

        values.insert("raw_frequency".to_string(), Value::from(self.cpu_frequency));

        // Per-core data (always available)
        for (i, usage) in self.per_core_usage.iter().enumerate() {
            values.insert(format!("core{}_usage", i), Value::from(*usage));
        }

        // Add limits (either manual or auto-detected)
        // Note: Both manual and auto-detected limits are already in the display unit
        // - Manual limits: User enters values in the displayed unit (e.g., "6" means 6 GHz if unit is GHz)
        // - Auto-detected limits: Converted during detection (line 407)

        // For frequency, ALWAYS use 0 as min (whether auto-detect or manual)
        // This ensures frequency is shown as a percentage of max (0 to max)
        let min_limit = if self.config.field == CpuField::Frequency {
            Some(0.0)
        } else if self.config.auto_detect_limits {
            self.detected_min
        } else {
            self.config.min_limit
        };

        let max_limit = if self.config.auto_detect_limits {
            self.detected_max
        } else {
            self.config.max_limit
        };

        if let Some(min) = min_limit {
            values.insert("min_limit".to_string(), Value::from(min));
        }

        if let Some(max) = max_limit {
            values.insert("max_limit".to_string(), Value::from(max));
        }

        values
    }

    fn is_available(&self) -> bool {
        // CPU info is always available
        true
    }

    fn configure(&mut self, config: &HashMap<String, Value>) -> Result<()> {
        // Look for cpu_config in the configuration
        if let Some(cpu_config_value) = config.get("cpu_config") {
            // Try to deserialize it into CpuSourceConfig
            if let Ok(cpu_config) = serde_json::from_value::<CpuSourceConfig>(cpu_config_value.clone()) {
                self.set_config(cpu_config);
            }
        }
        Ok(())
    }
}
