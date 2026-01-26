//! Network interface data source implementation

use crate::core::{DataSource, FieldMetadata, FieldPurpose, FieldType, SourceConfig, SourceMetadata};
use crate::ui::{NetworkField, NetworkSourceConfig, NetworkSpeedUnit, NetworkTotalUnit};
use anyhow::Result;
use once_cell::sync::Lazy;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use sysinfo::Networks;

/// Bytes per unit constants
const BYTES_PER_KB: f64 = 1024.0;
const BYTES_PER_MB: f64 = 1024.0 * 1024.0;
const BYTES_PER_GB: f64 = 1024.0 * 1024.0 * 1024.0;

/// Cached network interface list for UI dropdowns
static CACHED_INTERFACES: OnceLock<Vec<String>> = OnceLock::new();

/// Shared Networks instance for all NetworkSource instances.
/// This reduces memory usage when multiple network sources exist.
static SHARED_NETWORKS: Lazy<Mutex<Networks>> = Lazy::new(|| {
    log::info!("Creating shared Networks sysinfo instance");
    Mutex::new(Networks::new_with_refreshed_list())
});

/// Network interface data source
///
/// Provides network interface statistics including download/upload speeds
/// and total bytes transferred.
pub struct NetworkSource {
    metadata: SourceMetadata,
    config: NetworkSourceConfig,

    // Cached values (in bytes)
    total_received: u64,
    total_transmitted: u64,
    // Previous values for calculating speed
    prev_received: u64,
    prev_transmitted: u64,
    prev_time: Option<Instant>,

    /// Cached output values - updated in update(), returned by reference in values_ref()
    values: HashMap<String, Value>,
}

impl NetworkSource {
    pub fn new() -> Self {
        let metadata = SourceMetadata {
            id: "network".to_string(),
            name: "Network Interface".to_string(),
            description: "Network interface traffic monitoring".to_string(),
            available_keys: vec![
                "caption".to_string(),
                "value".to_string(),
                "unit".to_string(),
                "download_speed".to_string(),
                "upload_speed".to_string(),
                "total_download".to_string(),
                "total_upload".to_string(),
                "interface_name".to_string(),
            ],
            default_interval: Duration::from_millis(1000),
        };

        Self {
            metadata,
            config: NetworkSourceConfig::default(),
            total_received: 0,
            total_transmitted: 0,
            prev_received: 0,
            prev_transmitted: 0,
            prev_time: None,
            values: HashMap::with_capacity(16),
        }
    }

    /// Set configuration
    pub fn set_config(&mut self, config: NetworkSourceConfig) {
        self.config = config;
        // Reset previous values when config changes (e.g., different interface)
        self.prev_received = 0;
        self.prev_transmitted = 0;
        self.prev_time = None;
    }

    /// Get current configuration
    pub fn get_config(&self) -> &NetworkSourceConfig {
        &self.config
    }

    /// Get list of available network interfaces.
    ///
    /// This is cached on first call to avoid repeated system calls.
    /// The cache is populated once and reused for the lifetime of the application.
    pub fn get_available_interfaces() -> Vec<String> {
        CACHED_INTERFACES
            .get_or_init(|| {
                let networks = Networks::new_with_refreshed_list();
                let mut interfaces: Vec<String> = networks.keys().cloned().collect();
                interfaces.sort();
                interfaces
            })
            .clone()
    }

    /// Convert bytes to configured speed unit
    fn convert_speed(&self, bytes_per_sec: f64) -> f64 {
        match self.config.speed_unit {
            NetworkSpeedUnit::BytesPerSec => bytes_per_sec,
            NetworkSpeedUnit::KBPerSec => bytes_per_sec / BYTES_PER_KB,
            NetworkSpeedUnit::MBPerSec => bytes_per_sec / BYTES_PER_MB,
            NetworkSpeedUnit::GBPerSec => bytes_per_sec / BYTES_PER_GB,
        }
    }

    /// Convert bytes to configured total unit
    fn convert_total(&self, bytes: u64) -> f64 {
        match self.config.total_unit {
            NetworkTotalUnit::Bytes => bytes as f64,
            NetworkTotalUnit::KB => bytes as f64 / BYTES_PER_KB,
            NetworkTotalUnit::MB => bytes as f64 / BYTES_PER_MB,
            NetworkTotalUnit::GB => bytes as f64 / BYTES_PER_GB,
        }
    }

    /// Get speed unit string
    fn get_speed_unit_string(&self) -> &str {
        match self.config.speed_unit {
            NetworkSpeedUnit::BytesPerSec => "B/s",
            NetworkSpeedUnit::KBPerSec => "KB/s",
            NetworkSpeedUnit::MBPerSec => "MB/s",
            NetworkSpeedUnit::GBPerSec => "GB/s",
        }
    }

    /// Get total unit string
    fn get_total_unit_string(&self) -> &str {
        match self.config.total_unit {
            NetworkTotalUnit::Bytes => "B",
            NetworkTotalUnit::KB => "KB",
            NetworkTotalUnit::MB => "MB",
            NetworkTotalUnit::GB => "GB",
        }
    }

    /// Generate automatic caption
    fn generate_auto_caption(&self) -> String {
        let field_type = match self.config.field {
            NetworkField::DownloadSpeed => "DL",
            NetworkField::UploadSpeed => "UL",
            NetworkField::TotalDownload => "Total DL",
            NetworkField::TotalUpload => "Total UL",
        };

        // Shorten common interface names
        let interface_label = match self.config.interface.as_str() {
            name if name.starts_with("eth") => name.to_string(),
            name if name.starts_with("enp") => name.to_string(),
            name if name.starts_with("wlan") => name.to_string(),
            name if name.starts_with("wlp") => name.to_string(),
            "lo" => "Loopback".to_string(),
            name => name.to_string(),
        };

        format!("{} {}", interface_label, field_type)
    }
}

impl Default for NetworkSource {
    fn default() -> Self {
        Self::new()
    }
}

impl DataSource for NetworkSource {
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
                "download_speed",
                "Download Speed",
                "Current download speed",
                FieldType::Numerical,
                FieldPurpose::Value,
            ),
            FieldMetadata::new(
                "upload_speed",
                "Upload Speed",
                "Current upload speed",
                FieldType::Numerical,
                FieldPurpose::Value,
            ),
            FieldMetadata::new(
                "total_download",
                "Total Downloaded",
                "Total bytes received since boot",
                FieldType::Numerical,
                FieldPurpose::Value,
            ),
            FieldMetadata::new(
                "total_upload",
                "Total Uploaded",
                "Total bytes transmitted since boot",
                FieldType::Numerical,
                FieldPurpose::Value,
            ),
            FieldMetadata::new(
                "interface_name",
                "Interface",
                "Network interface name",
                FieldType::Text,
                FieldPurpose::Other,
            ),
        ]
    }

    fn update(&mut self) -> Result<()> {
        // Use shared Networks instance to reduce memory usage
        let mut networks = SHARED_NETWORKS
            .lock()
            .map_err(|e| anyhow::anyhow!("Networks mutex poisoned: {}", e))?;
        networks.refresh();

        // Find the network interface matching our configured interface
        if let Some((_, data)) = networks
            .iter()
            .find(|(name, _)| name.as_str() == self.config.interface)
        {
            self.total_received = data.total_received();
            self.total_transmitted = data.total_transmitted();
        } else {
            // Interface not found, reset values
            self.total_received = 0;
            self.total_transmitted = 0;
        }

        // Drop the lock before doing any other processing
        drop(networks);

        // Calculate speeds
        let now = Instant::now();
        let (download_speed, upload_speed) = if let Some(prev_time) = self.prev_time {
            let elapsed = now.duration_since(prev_time).as_secs_f64();
            if elapsed > 0.0 {
                let download_bytes = self.total_received.saturating_sub(self.prev_received);
                let upload_bytes = self.total_transmitted.saturating_sub(self.prev_transmitted);
                (
                    download_bytes as f64 / elapsed,
                    upload_bytes as f64 / elapsed,
                )
            } else {
                (0.0, 0.0)
            }
        } else {
            (0.0, 0.0)
        };

        // Update previous values for next calculation
        self.prev_received = self.total_received;
        self.prev_transmitted = self.total_transmitted;
        self.prev_time = Some(now);

        // Build values HashMap (reuse allocation, just clear and refill)
        self.values.clear();

        let caption = self
            .config
            .custom_caption
            .clone()
            .unwrap_or_else(|| self.generate_auto_caption());

        self.values.insert(
            "interface_name".to_string(),
            Value::from(self.config.interface.as_str()),
        );

        // Provide all raw data (in bytes and bytes/s)
        self.values
            .insert("raw_download_speed".to_string(), Value::from(download_speed));
        self.values
            .insert("raw_upload_speed".to_string(), Value::from(upload_speed));
        self.values.insert(
            "raw_total_download".to_string(),
            Value::from(self.total_received),
        );
        self.values.insert(
            "raw_total_upload".to_string(),
            Value::from(self.total_transmitted),
        );

        // Provide converted values
        let download_speed_converted = self.convert_speed(download_speed);
        let upload_speed_converted = self.convert_speed(upload_speed);
        let total_download_converted = self.convert_total(self.total_received);
        let total_upload_converted = self.convert_total(self.total_transmitted);

        self.values.insert(
            "download_speed".to_string(),
            Value::from(download_speed_converted),
        );
        self.values.insert(
            "upload_speed".to_string(),
            Value::from(upload_speed_converted),
        );
        self.values.insert(
            "total_download".to_string(),
            Value::from(total_download_converted),
        );
        self.values.insert(
            "total_upload".to_string(),
            Value::from(total_upload_converted),
        );

        // Set field-specific values
        match self.config.field {
            NetworkField::DownloadSpeed => {
                self.values
                    .insert("caption".to_string(), Value::from(caption));
                self.values
                    .insert("value".to_string(), Value::from(download_speed_converted));
                self.values
                    .insert("unit".to_string(), Value::from(self.get_speed_unit_string()));
            }
            NetworkField::UploadSpeed => {
                self.values
                    .insert("caption".to_string(), Value::from(caption));
                self.values
                    .insert("value".to_string(), Value::from(upload_speed_converted));
                self.values
                    .insert("unit".to_string(), Value::from(self.get_speed_unit_string()));
            }
            NetworkField::TotalDownload => {
                self.values
                    .insert("caption".to_string(), Value::from(caption));
                self.values
                    .insert("value".to_string(), Value::from(total_download_converted));
                self.values
                    .insert("unit".to_string(), Value::from(self.get_total_unit_string()));
            }
            NetworkField::TotalUpload => {
                self.values
                    .insert("caption".to_string(), Value::from(caption));
                self.values
                    .insert("value".to_string(), Value::from(total_upload_converted));
                self.values
                    .insert("unit".to_string(), Value::from(self.get_total_unit_string()));
            }
        }

        // Calculate limits based on field
        let (min_limit, max_limit) = match self.config.field {
            NetworkField::DownloadSpeed | NetworkField::UploadSpeed => {
                if self.config.auto_detect_limits {
                    // For speed, we don't have a natural max, so use a reasonable default
                    // or track the max seen value over time
                    (0.0, self.config.max_limit.unwrap_or(100.0))
                } else {
                    (
                        self.config.min_limit.unwrap_or(0.0),
                        self.config.max_limit.unwrap_or(100.0),
                    )
                }
            }
            NetworkField::TotalDownload | NetworkField::TotalUpload => {
                if self.config.auto_detect_limits {
                    (0.0, self.config.max_limit.unwrap_or(1000.0))
                } else {
                    (
                        self.config.min_limit.unwrap_or(0.0),
                        self.config.max_limit.unwrap_or(1000.0),
                    )
                }
            }
        };

        self.values
            .insert("min_limit".to_string(), Value::from(min_limit));
        self.values
            .insert("max_limit".to_string(), Value::from(max_limit));

        Ok(())
    }

    fn get_values(&self) -> HashMap<String, Value> {
        self.values.clone()
    }

    fn values_ref(&self) -> Option<&HashMap<String, Value>> {
        Some(&self.values)
    }

    fn configure(&mut self, config: &HashMap<String, Value>) -> Result<()> {
        // Look for network_config in the configuration
        if let Some(network_config_value) = config.get("network_config") {
            // Try to deserialize it into NetworkSourceConfig
            if let Ok(network_config) =
                serde_json::from_value::<NetworkSourceConfig>(network_config_value.clone())
            {
                self.set_config(network_config);
            }
        }
        Ok(())
    }

    fn get_typed_config(&self) -> Option<SourceConfig> {
        Some(SourceConfig::Network(self.config.clone()))
    }
}
