//! CPU data source implementation

use crate::core::{DataSource, FieldMetadata, FieldPurpose, FieldType, SourceMetadata};
use crate::ui::{CpuField, CpuSourceConfig, CoreSelection, TemperatureUnit};
use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;
use sysinfo::{Components, CpuRefreshKind, RefreshKind, System};

/// Information about a discovered CPU temperature sensor
#[derive(Debug, Clone)]
pub struct CpuSensor {
    pub index: usize,
    pub label: String,
}

/// CPU data source
///
/// Provides comprehensive CPU information including usage, temperature, frequency,
/// and per-core statistics using sysinfo crate.
pub struct CpuSource {
    metadata: SourceMetadata,
    system: System,
    components: Components,
    global_usage: f32,
    per_core_usage: Vec<f32>,
    cpu_sensors: Vec<CpuSensor>,
    cpu_temperature: Option<f32>,
    cpu_frequency: f64,
    config: CpuSourceConfig,
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

        // Initialize components for temperature monitoring
        let components = Components::new_with_refreshed_list();

        // Discover CPU temperature sensors
        let cpu_sensors = Self::discover_cpu_sensors(&components);

        Self {
            metadata,
            system,
            components,
            global_usage: 0.0,
            per_core_usage: Vec::new(),
            cpu_sensors,
            cpu_temperature: None,
            cpu_frequency: 0.0,
            config: CpuSourceConfig::default(),
        }
    }

    /// Discover all CPU temperature sensors
    fn discover_cpu_sensors(components: &Components) -> Vec<CpuSensor> {
        let mut sensors = Vec::new();
        let mut index = 0;

        // First priority: CPU package sensors
        for component in components {
            let label = component.label();
            let label_lower = label.to_lowercase();
            if label_lower.contains("cpu") || label_lower.contains("package") || label_lower.contains("tctl") {
                sensors.push(CpuSensor {
                    index,
                    label: label.to_string(),
                });
                index += 1;
            }
        }

        // Second priority: Core sensors (if no package sensors found)
        if sensors.is_empty() {
            for component in components {
                let label = component.label();
                let label_lower = label.to_lowercase();
                if label_lower.contains("core") {
                    sensors.push(CpuSensor {
                        index,
                        label: label.to_string(),
                    });
                    index += 1;
                }
            }
        }

        sensors
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

    /// Find CPU temperature from components using configured sensor index
    fn find_cpu_temperature(&self) -> Option<f32> {
        // If no sensors discovered, return None
        if self.cpu_sensors.is_empty() {
            return None;
        }

        // Get the sensor label for the configured index
        let sensor_index = self.config.sensor_index;
        let target_label = if let Some(sensor) = self.cpu_sensors.get(sensor_index) {
            &sensor.label
        } else {
            // If configured index is out of bounds, use first sensor
            &self.cpu_sensors[0].label
        };

        // Find the component with matching label
        for component in &self.components {
            if component.label() == target_label {
                return Some(component.temperature());
            }
        }

        None
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
                "usage",
                "Usage",
                "Overall CPU usage percentage",
                FieldType::Percentage,
                FieldPurpose::Value,
            ),
            FieldMetadata::new(
                "unit",
                "Unit",
                "Unit of measurement for usage",
                FieldType::Text,
                FieldPurpose::Unit,
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
                &format!("core{}_usage", i),
                &format!("Core {} Usage", i),
                &format!("CPU core {} usage percentage", i),
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

        // Refresh and get temperature
        self.components.refresh();
        self.cpu_temperature = self.find_cpu_temperature();

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

        // Apply field configuration to determine what goes in the main value/unit fields
        match self.config.field {
            CpuField::Usage => {
                values.insert("caption".to_string(), Value::from("CPU"));
                values.insert("usage".to_string(), Value::from(usage_value));
                values.insert("unit".to_string(), Value::from("%"));
            }
            CpuField::Temperature => {
                values.insert("caption".to_string(), Value::from("CPU Temp"));
                if let Some(temp) = temperature_value {
                    values.insert("temperature".to_string(), Value::from(temp));
                    values.insert("unit".to_string(), Value::from(self.get_temperature_unit_string()));
                } else {
                    values.insert("temperature".to_string(), Value::from("N/A"));
                    values.insert("unit".to_string(), Value::from(""));
                }
            }
            CpuField::Frequency => {
                values.insert("caption".to_string(), Value::from("CPU Freq"));
                values.insert("frequency".to_string(), Value::from(frequency_value));
                values.insert("unit".to_string(), Value::from("MHz"));
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
