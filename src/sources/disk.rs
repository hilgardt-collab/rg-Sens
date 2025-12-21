//! Disk usage data source implementation

use crate::core::{DataSource, FieldMetadata, FieldPurpose, FieldType, SourceMetadata};
use crate::core::constants::{BYTES_PER_MB, BYTES_PER_GB, BYTES_PER_TB};
use crate::ui::{DiskField, DiskSourceConfig, DiskUnit};
use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::Duration;
use sysinfo::Disks;

/// Cached disk list for UI dropdowns (avoids expensive filesystem scan on each call)
static CACHED_DISKS: OnceLock<Vec<(String, String)>> = OnceLock::new();

/// Disk usage data source
///
/// Provides disk usage information for mounted filesystems.
pub struct DiskSource {
    metadata: SourceMetadata,
    disks: Disks,
    config: DiskSourceConfig,

    // Cached values (in bytes)
    total_space: u64,
    available_space: u64,
}

impl DiskSource {
    pub fn new() -> Self {
        let metadata = SourceMetadata {
            id: "disk".to_string(),
            name: "Disk Usage".to_string(),
            description: "Disk space usage monitoring for mounted filesystems".to_string(),
            available_keys: vec![
                "caption".to_string(),
                "value".to_string(),
                "unit".to_string(),
                "used".to_string(),
                "free".to_string(),
                "available".to_string(),
                "total".to_string(),
                "percent".to_string(),
                "mount_point".to_string(),
                "file_system".to_string(),
            ],
            default_interval: Duration::from_millis(2000),
        };

        let disks = Disks::new_with_refreshed_list();

        Self {
            metadata,
            disks,
            config: DiskSourceConfig::default(),
            total_space: 0,
            available_space: 0,
        }
    }

    /// Set configuration
    pub fn set_config(&mut self, config: DiskSourceConfig) {
        self.config = config;
    }

    /// Get current configuration
    pub fn get_config(&self) -> &DiskSourceConfig {
        &self.config
    }

    /// Get list of available disks with their mount points and names.
    ///
    /// This is cached on first call to avoid expensive filesystem scans on every
    /// UI interaction. The cache is populated once and reused for the lifetime
    /// of the application. Restart the app to detect newly mounted disks.
    pub fn get_available_disks() -> Vec<(String, String)> {
        CACHED_DISKS.get_or_init(|| {
            let disks = Disks::new_with_refreshed_list();
            disks
                .iter()
                .map(|disk| {
                    let mount_point = disk.mount_point().to_string_lossy().to_string();
                    let name = disk.name().to_string_lossy().to_string();
                    (mount_point, name)
                })
                .collect()
        }).clone()
    }

    /// Convert disk space from bytes to configured unit
    fn convert_space(&self, bytes: u64) -> f64 {
        match self.config.disk_unit {
            DiskUnit::MB => bytes as f64 / BYTES_PER_MB,
            DiskUnit::GB => bytes as f64 / BYTES_PER_GB,
            DiskUnit::TB => bytes as f64 / BYTES_PER_TB,
        }
    }

    /// Get disk unit string
    fn get_disk_unit_string(&self) -> &str {
        match self.config.disk_unit {
            DiskUnit::MB => "MB",
            DiskUnit::GB => "GB",
            DiskUnit::TB => "TB",
        }
    }

    /// Generate automatic caption
    fn generate_auto_caption(&self) -> String {
        let field_type = match self.config.field {
            DiskField::Used => "Used",
            DiskField::Free => "Free",
            DiskField::Total => "Total",
            DiskField::Percent => "%",
        };

        // Shorten common mount points
        let disk_label = match self.config.disk_path.as_str() {
            "/" => "Root",
            "/home" => "Home",
            "/boot" => "Boot",
            path => {
                // Use last component of path
                path.rsplit('/').next().unwrap_or(path)
            }
        };

        format!("{} {}", disk_label, field_type)
    }
}

impl Default for DiskSource {
    fn default() -> Self {
        Self::new()
    }
}

impl DataSource for DiskSource {
    fn metadata(&self) -> &SourceMetadata {
        &self.metadata
    }

    fn fields(&self) -> Vec<FieldMetadata> {
        vec![
            FieldMetadata::new(
                "caption",
                "Caption",
                "Display caption (auto-generated or custom)",
                FieldType::Text,
                FieldPurpose::Caption,
            ),
            FieldMetadata::new(
                "value",
                "Value",
                "The primary field value",
                FieldType::Numerical,
                FieldPurpose::Value,
            ),
            FieldMetadata::new(
                "unit",
                "Unit",
                "The unit of measurement",
                FieldType::Text,
                FieldPurpose::Unit,
            ),
            FieldMetadata::new(
                "used",
                "Used Space",
                "Used disk space",
                FieldType::Numerical,
                FieldPurpose::Value,
            ),
            FieldMetadata::new(
                "free",
                "Free Space",
                "Free disk space",
                FieldType::Numerical,
                FieldPurpose::Value,
            ),
            FieldMetadata::new(
                "available",
                "Available Space",
                "Available disk space",
                FieldType::Numerical,
                FieldPurpose::Value,
            ),
            FieldMetadata::new(
                "total",
                "Total Space",
                "Total disk space",
                FieldType::Numerical,
                FieldPurpose::Value,
            ),
            FieldMetadata::new(
                "percent",
                "Usage Percent",
                "Disk usage percentage",
                FieldType::Numerical,
                FieldPurpose::Value,
            ),
            FieldMetadata::new(
                "mount_point",
                "Mount Point",
                "Disk mount point",
                FieldType::Text,
                FieldPurpose::Other,
            ),
            FieldMetadata::new(
                "file_system",
                "File System",
                "File system type",
                FieldType::Text,
                FieldPurpose::Other,
            ),
        ]
    }

    fn update(&mut self) -> Result<()> {
        self.disks.refresh();

        // Find the disk matching our configured path
        if let Some(disk) = self.disks.iter().find(|d| {
            d.mount_point().to_string_lossy() == self.config.disk_path
        }) {
            self.total_space = disk.total_space();
            self.available_space = disk.available_space();
        } else {
            // Disk not found, reset values
            self.total_space = 0;
            self.available_space = 0;
        }

        Ok(())
    }

    fn get_values(&self) -> HashMap<String, Value> {
        let mut values = HashMap::new();

        let caption = self.config.custom_caption.clone()
            .unwrap_or_else(|| self.generate_auto_caption());

        values.insert("mount_point".to_string(), Value::from(self.config.disk_path.as_str()));

        // Get file system type if available
        if let Some(disk) = self.disks.iter().find(|d| {
            d.mount_point().to_string_lossy() == self.config.disk_path
        }) {
            values.insert("file_system".to_string(),
                Value::from(disk.file_system().to_string_lossy().to_string()));
        }

        if self.total_space == 0 {
            // Disk not found or has no space
            values.insert("caption".to_string(), Value::from(caption));
            values.insert("value".to_string(), Value::from("N/A"));
            values.insert("unit".to_string(), Value::from(""));
            return values;
        }

        let used_space = self.total_space.saturating_sub(self.available_space);
        let percent = if self.total_space > 0 {
            (used_space as f64 / self.total_space as f64) * 100.0
        } else {
            0.0
        };

        // Provide all raw data
        values.insert("raw_used_bytes".to_string(), Value::from(used_space));
        values.insert("raw_free_bytes".to_string(), Value::from(self.available_space));
        values.insert("raw_total_bytes".to_string(), Value::from(self.total_space));
        values.insert("percent".to_string(), Value::from(percent));

        // Calculate limits based on field
        let (min_limit, max_limit) = match self.config.field {
            DiskField::Used => {
                if self.config.auto_detect_limits {
                    (0.0, self.convert_space(self.total_space))
                } else {
                    (self.config.min_limit.unwrap_or(0.0),
                     self.config.max_limit.unwrap_or(self.convert_space(self.total_space)))
                }
            }
            DiskField::Free => {
                if self.config.auto_detect_limits {
                    (0.0, self.convert_space(self.total_space))
                } else {
                    (self.config.min_limit.unwrap_or(0.0),
                     self.config.max_limit.unwrap_or(self.convert_space(self.total_space)))
                }
            }
            DiskField::Total => {
                let total = self.convert_space(self.total_space);
                if self.config.auto_detect_limits {
                    (0.0, total)
                } else {
                    (self.config.min_limit.unwrap_or(0.0),
                     self.config.max_limit.unwrap_or(total))
                }
            }
            DiskField::Percent => {
                if self.config.auto_detect_limits {
                    (0.0, 100.0)
                } else {
                    (self.config.min_limit.unwrap_or(0.0),
                     self.config.max_limit.unwrap_or(100.0))
                }
            }
        };

        values.insert("min_limit".to_string(), Value::from(min_limit));
        values.insert("max_limit".to_string(), Value::from(max_limit));

        // Set field-specific values
        match self.config.field {
            DiskField::Used => {
                let used = self.convert_space(used_space);
                values.insert("caption".to_string(), Value::from(caption));
                values.insert("value".to_string(), Value::from(used));
                values.insert("used".to_string(), Value::from(used));
                values.insert("unit".to_string(), Value::from(self.get_disk_unit_string()));
            }
            DiskField::Free => {
                let free = self.convert_space(self.available_space);
                values.insert("caption".to_string(), Value::from(caption));
                values.insert("value".to_string(), Value::from(free));
                values.insert("free".to_string(), Value::from(free));
                values.insert("available".to_string(), Value::from(free));
                values.insert("unit".to_string(), Value::from(self.get_disk_unit_string()));
            }
            DiskField::Total => {
                let total = self.convert_space(self.total_space);
                values.insert("caption".to_string(), Value::from(caption));
                values.insert("value".to_string(), Value::from(total));
                values.insert("total".to_string(), Value::from(total));
                values.insert("unit".to_string(), Value::from(self.get_disk_unit_string()));
            }
            DiskField::Percent => {
                values.insert("caption".to_string(), Value::from(caption));
                values.insert("value".to_string(), Value::from(percent));
                values.insert("unit".to_string(), Value::from("%"));
            }
        }

        // Also provide converted space values for all fields
        values.insert("used_converted".to_string(), Value::from(self.convert_space(used_space)));
        values.insert("free_converted".to_string(), Value::from(self.convert_space(self.available_space)));
        values.insert("total_converted".to_string(), Value::from(self.convert_space(self.total_space)));

        values
    }

    fn configure(&mut self, config: &HashMap<String, Value>) -> Result<()> {
        // Look for disk_config in the configuration
        if let Some(disk_config_value) = config.get("disk_config") {
            // Try to deserialize it into DiskSourceConfig
            if let Ok(disk_config) = serde_json::from_value::<DiskSourceConfig>(disk_config_value.clone()) {
                self.set_config(disk_config);
            }
        }
        Ok(())
    }
}
