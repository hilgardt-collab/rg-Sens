//! CPU data source implementation

use crate::core::{DataSource, FieldMetadata, FieldPurpose, FieldType, SourceMetadata};
use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;
use sysinfo::{CpuRefreshKind, RefreshKind, System};

/// CPU data source
///
/// Provides CPU usage information using sysinfo crate.
pub struct CpuSource {
    metadata: SourceMetadata,
    system: System,
    global_usage: f32,
}

impl CpuSource {
    pub fn new() -> Self {
        let metadata = SourceMetadata {
            id: "cpu".to_string(),
            name: "CPU Usage".to_string(),
            description: "Global CPU usage percentage".to_string(),
            available_keys: vec!["caption".to_string(), "usage".to_string(), "unit".to_string()],
            default_interval: Duration::from_millis(1000),
        };

        // Initialize system with CPU refresh configuration
        let system = System::new_with_specifics(
            RefreshKind::new().with_cpu(CpuRefreshKind::everything()),
        );

        Self {
            metadata,
            system,
            global_usage: 0.0,
        }
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
        vec![
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
                "Current CPU usage percentage",
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
        ]
    }

    fn update(&mut self) -> Result<()> {
        // Refresh CPU information
        self.system.refresh_cpu_all();

        // Calculate global CPU usage
        self.global_usage = self.system.global_cpu_usage();

        Ok(())
    }

    fn get_values(&self) -> HashMap<String, Value> {
        let mut values = HashMap::new();
        values.insert("caption".to_string(), Value::from("CPU"));
        values.insert("usage".to_string(), Value::from(self.global_usage));
        values.insert("unit".to_string(), Value::from("%"));
        values
    }

    fn is_available(&self) -> bool {
        // CPU info is always available
        true
    }
}
