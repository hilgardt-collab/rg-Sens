//! CPU data source implementation

use crate::core::{DataSource, FieldMetadata, FieldPurpose, FieldType, SourceMetadata};
use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;
use sysinfo::{Components, CpuRefreshKind, RefreshKind, System};

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
    cpu_temperature: Option<f32>,
    cpu_frequency: f64,
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

        Self {
            metadata,
            system,
            components,
            global_usage: 0.0,
            per_core_usage: Vec::new(),
            cpu_temperature: None,
            cpu_frequency: 0.0,
        }
    }

    /// Find CPU temperature from components
    fn find_cpu_temperature(&self) -> Option<f32> {
        // Look for components with "CPU" or "Core" in the label
        for component in &self.components {
            let label = component.label().to_lowercase();
            if label.contains("cpu") || label.contains("package") || label.contains("tctl") {
                return Some(component.temperature());
            }
        }

        // If no specific CPU temp found, try to find any core temperature
        for component in &self.components {
            let label = component.label().to_lowercase();
            if label.contains("core") {
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

        // Basic fields
        values.insert("caption".to_string(), Value::from("CPU"));
        values.insert("usage".to_string(), Value::from(self.global_usage));
        values.insert("unit".to_string(), Value::from("%"));

        // Temperature (if available)
        if let Some(temp) = self.cpu_temperature {
            values.insert("temperature".to_string(), Value::from(temp));
            values.insert("temp_unit".to_string(), Value::from("°C"));
        } else {
            values.insert("temperature".to_string(), Value::from("N/A"));
            values.insert("temp_unit".to_string(), Value::from(""));
        }

        // Frequency
        values.insert("frequency".to_string(), Value::from(self.cpu_frequency));
        values.insert("freq_unit".to_string(), Value::from("MHz"));

        // Per-core usage
        for (i, usage) in self.per_core_usage.iter().enumerate() {
            values.insert(format!("core{}_usage", i), Value::from(*usage));
        }

        values
    }

    fn is_available(&self) -> bool {
        // CPU info is always available
        true
    }
}
