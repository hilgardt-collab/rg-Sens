//! Memory (RAM) data source implementation

use crate::core::{DataSource, FieldMetadata, FieldPurpose, FieldType, SourceMetadata};
use crate::core::constants::{BYTES_PER_MB, BYTES_PER_GB};
use crate::ui::{MemoryField, MemorySourceConfig, MemoryUnit};
use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;
use sysinfo::System;

/// Memory data source
///
/// Provides comprehensive memory information including RAM and swap usage.
pub struct MemorySource {
    metadata: SourceMetadata,
    system: System,
    config: MemorySourceConfig,

    // Cached values (in bytes)
    total_memory: u64,
    used_memory: u64,
    available_memory: u64,
    total_swap: u64,
    used_swap: u64,
}

impl MemorySource {
    pub fn new() -> Self {
        let metadata = SourceMetadata {
            id: "memory".to_string(),
            name: "Memory (RAM)".to_string(),
            description: "System memory (RAM) and swap usage monitoring".to_string(),
            available_keys: vec![
                "caption".to_string(),
                "value".to_string(),
                "unit".to_string(),
                "used".to_string(),
                "free".to_string(),
                "available".to_string(),
                "total".to_string(),
                "percent".to_string(),
                "swap_used".to_string(),
                "swap_free".to_string(),
                "swap_total".to_string(),
                "swap_percent".to_string(),
            ],
            default_interval: Duration::from_millis(1000),
        };

        // Use empty System - we only need memory info, not CPU/disk/etc.
        // refresh_memory() will be called in update() to populate memory data
        let system = System::new();

        Self {
            metadata,
            system,
            config: MemorySourceConfig::default(),
            total_memory: 0,
            used_memory: 0,
            available_memory: 0,
            total_swap: 0,
            used_swap: 0,
        }
    }

    /// Set configuration
    pub fn set_config(&mut self, config: MemorySourceConfig) {
        self.config = config;
    }

    /// Get current configuration
    pub fn get_config(&self) -> &MemorySourceConfig {
        &self.config
    }

    /// Convert memory from bytes to configured unit
    fn convert_memory(&self, bytes: u64) -> f64 {
        match self.config.memory_unit {
            MemoryUnit::MB => bytes as f64 / BYTES_PER_MB,
            MemoryUnit::GB => bytes as f64 / BYTES_PER_GB,
        }
    }

    /// Get memory unit string
    fn get_memory_unit_string(&self) -> &str {
        match self.config.memory_unit {
            MemoryUnit::MB => "MB",
            MemoryUnit::GB => "GB",
        }
    }

    /// Generate automatic caption
    fn generate_auto_caption(&self) -> String {
        let field_type = match self.config.field {
            MemoryField::Used => "RAM Used",
            MemoryField::Free => "RAM Free",
            MemoryField::Available => "RAM Avail",
            MemoryField::Percent => "RAM %",
            MemoryField::SwapUsed => "Swap Used",
            MemoryField::SwapPercent => "Swap %",
        };

        field_type.to_string()
    }
}

impl Default for MemorySource {
    fn default() -> Self {
        Self::new()
    }
}

impl DataSource for MemorySource {
    fn metadata(&self) -> &SourceMetadata {
        &self.metadata
    }

    fn fields(&self) -> Vec<FieldMetadata> {
        vec![
            FieldMetadata::new(
                "caption",
                "Caption",
                "Label identifying the memory metric",
                FieldType::Text,
                FieldPurpose::Caption,
            ),
            FieldMetadata::new(
                "value",
                "Value (Configured)",
                "The configured value based on memory field settings",
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
        ]
    }

    fn update(&mut self) -> Result<()> {
        // Refresh memory information
        self.system.refresh_memory();

        // Get values in bytes
        self.total_memory = self.system.total_memory();
        self.used_memory = self.system.used_memory();
        self.available_memory = self.system.available_memory();
        self.total_swap = self.system.total_swap();
        self.used_swap = self.system.used_swap();

        Ok(())
    }

    fn get_values(&self) -> HashMap<String, Value> {
        let mut values = HashMap::new();

        let caption = self.config.custom_caption
            .as_ref()
            .cloned()
            .unwrap_or_else(|| self.generate_auto_caption());

        // Calculate derived values
        let free_memory = self.total_memory.saturating_sub(self.used_memory);
        let memory_percent = if self.total_memory > 0 {
            (self.used_memory as f64 / self.total_memory as f64 * 100.0) as u32
        } else {
            0
        };

        let free_swap = self.total_swap.saturating_sub(self.used_swap);
        let swap_percent = if self.total_swap > 0 {
            (self.used_swap as f64 / self.total_swap as f64 * 100.0) as u32
        } else {
            0
        };

        // Apply field configuration to determine what goes in the main value/unit fields
        match self.config.field {
            MemoryField::Used => {
                let converted = self.convert_memory(self.used_memory);
                values.insert("caption".to_string(), Value::from(caption));
                values.insert("value".to_string(), Value::from(converted));
                values.insert("used".to_string(), Value::from(converted));
                values.insert("unit".to_string(), Value::from(self.get_memory_unit_string()));
            }
            MemoryField::Free => {
                let converted = self.convert_memory(free_memory);
                values.insert("caption".to_string(), Value::from(caption));
                values.insert("value".to_string(), Value::from(converted));
                values.insert("free".to_string(), Value::from(converted));
                values.insert("unit".to_string(), Value::from(self.get_memory_unit_string()));
            }
            MemoryField::Available => {
                let converted = self.convert_memory(self.available_memory);
                values.insert("caption".to_string(), Value::from(caption));
                values.insert("value".to_string(), Value::from(converted));
                values.insert("available".to_string(), Value::from(converted));
                values.insert("unit".to_string(), Value::from(self.get_memory_unit_string()));
            }
            MemoryField::Percent => {
                values.insert("caption".to_string(), Value::from(caption));
                values.insert("value".to_string(), Value::from(memory_percent));
                values.insert("percent".to_string(), Value::from(memory_percent));
                values.insert("unit".to_string(), Value::from("%"));
            }
            MemoryField::SwapUsed => {
                let converted = self.convert_memory(self.used_swap);
                values.insert("caption".to_string(), Value::from(caption));
                values.insert("value".to_string(), Value::from(converted));
                values.insert("swap_used".to_string(), Value::from(converted));
                values.insert("unit".to_string(), Value::from(self.get_memory_unit_string()));
            }
            MemoryField::SwapPercent => {
                values.insert("caption".to_string(), Value::from(caption));
                values.insert("value".to_string(), Value::from(swap_percent));
                values.insert("swap_percent".to_string(), Value::from(swap_percent));
                values.insert("unit".to_string(), Value::from("%"));
            }
        }

        // Also provide all raw data (in configured units)
        values.insert("raw_total".to_string(), Value::from(self.convert_memory(self.total_memory)));
        values.insert("raw_used".to_string(), Value::from(self.convert_memory(self.used_memory)));
        values.insert("raw_free".to_string(), Value::from(self.convert_memory(free_memory)));
        values.insert("raw_available".to_string(), Value::from(self.convert_memory(self.available_memory)));
        values.insert("raw_percent".to_string(), Value::from(memory_percent));
        values.insert("raw_swap_total".to_string(), Value::from(self.convert_memory(self.total_swap)));
        values.insert("raw_swap_used".to_string(), Value::from(self.convert_memory(self.used_swap)));
        values.insert("raw_swap_free".to_string(), Value::from(self.convert_memory(free_swap)));
        values.insert("raw_swap_percent".to_string(), Value::from(swap_percent));

        // Add limits based on field type
        let (min_limit, max_limit) = match self.config.field {
            MemoryField::Percent | MemoryField::SwapPercent => (0.0, 100.0),
            MemoryField::Used | MemoryField::Available => {
                (0.0, self.convert_memory(self.total_memory))
            }
            MemoryField::Free => {
                (0.0, self.convert_memory(self.total_memory))
            }
            MemoryField::SwapUsed => {
                (0.0, self.convert_memory(self.total_swap))
            }
        };

        values.insert("min_limit".to_string(), Value::from(min_limit));
        values.insert("max_limit".to_string(), Value::from(max_limit));

        values
    }

    fn configure(&mut self, config: &HashMap<String, Value>) -> Result<()> {
        // Look for memory_config in the configuration
        if let Some(memory_config_value) = config.get("memory_config") {
            // Try to deserialize it into MemorySourceConfig
            if let Ok(memory_config) = serde_json::from_value::<MemorySourceConfig>(memory_config_value.clone()) {
                self.set_config(memory_config);
            }
        }
        Ok(())
    }
}
