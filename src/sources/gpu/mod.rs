//! GPU data source with multi-vendor support (NVIDIA, AMD)

mod backend;
mod nvidia;
mod amd;
mod detector;

pub use backend::{GpuBackend, GpuInfo};
use detector::detect_gpus;

use crate::core::{DataSource, FieldMetadata, FieldPurpose, FieldType, SourceMetadata};
use crate::ui::{GpuField, GpuSourceConfig, MemoryUnit, TemperatureUnit};
use anyhow::{anyhow, Result};
use once_cell::sync::Lazy;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Global GPU detection (performed once at startup)
static GPU_MANAGER: Lazy<GpuManager> = Lazy::new(|| {
    let detected = detect_gpus();
    GpuManager {
        backends: detected.gpus.into_iter().map(|b| Arc::new(Mutex::new(b))).collect(),
        gpu_info: detected.info,
    }
});

/// GPU manager holding all detected GPU backends
struct GpuManager {
    backends: Vec<Arc<Mutex<Box<dyn GpuBackend>>>>,
    gpu_info: Vec<GpuInfo>,
}

/// GPU data source
pub struct GpuSource {
    metadata: SourceMetadata,
    config: GpuSourceConfig,
    backend: Option<Arc<Mutex<Box<dyn GpuBackend>>>>,

    // Cached values (read from backend after update)
    temperature: Option<f32>,
    utilization: Option<u32>,
    memory_used: Option<u64>,
    memory_total: Option<u64>,
    power_usage: Option<f32>,
    fan_speed: Option<u32>,
    clock_core: Option<u32>,
    clock_memory: Option<u32>,
}

impl GpuSource {
    pub fn new() -> Self {
        let metadata = SourceMetadata {
            id: "gpu".to_string(),
            name: "GPU".to_string(),
            description: "GPU monitoring (NVIDIA/AMD) - temperature, utilization, memory, power".to_string(),
            available_keys: vec![
                "caption".to_string(),
                "value".to_string(),
                "unit".to_string(),
                "temperature".to_string(),
                "utilization".to_string(),
                "memory_used".to_string(),
                "memory_percent".to_string(),
                "power".to_string(),
                "fan_speed".to_string(),
            ],
            default_interval: Duration::from_millis(1000),
        };

        // Get backend for default GPU (index 0)
        let backend = GPU_MANAGER.backends.first().cloned();

        Self {
            metadata,
            config: GpuSourceConfig::default(),
            backend,
            temperature: None,
            utilization: None,
            memory_used: None,
            memory_total: None,
            power_usage: None,
            fan_speed: None,
            clock_core: None,
            clock_memory: None,
        }
    }

    /// Get cached GPU count
    pub fn get_cached_gpu_count() -> u32 {
        GPU_MANAGER.gpu_info.len() as u32
    }

    /// Get cached GPU names
    pub fn get_cached_gpu_names() -> Vec<String> {
        GPU_MANAGER.gpu_info.iter().map(|info| info.name.clone()).collect()
    }

    /// Set configuration
    pub fn set_config(&mut self, config: GpuSourceConfig) {
        // Update backend if GPU index changed
        if config.gpu_index != self.config.gpu_index {
            // Validate gpu_index is within bounds before accessing
            let gpu_count = GPU_MANAGER.backends.len();
            if (config.gpu_index as usize) < gpu_count {
                self.backend = GPU_MANAGER.backends.get(config.gpu_index as usize).cloned();
            } else {
                log::warn!(
                    "GPU index {} out of bounds (only {} GPUs available), keeping current backend",
                    config.gpu_index,
                    gpu_count
                );
                // Don't update backend - keep the existing one or None
            }
        }
        self.config = config;
    }

    /// Get current configuration
    pub fn get_config(&self) -> &GpuSourceConfig {
        &self.config
    }

    /// Convert temperature to configured unit
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

    /// Convert memory from bytes to configured unit
    fn convert_memory(&self, bytes: u64) -> f64 {
        match self.config.memory_unit {
            MemoryUnit::MB => bytes as f64 / (1024.0 * 1024.0),
            MemoryUnit::GB => bytes as f64 / (1024.0 * 1024.0 * 1024.0),
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
        // Include vendor in caption if we have backend info
        let gpu_prefix = if let Some(ref backend) = self.backend {
            if let Ok(backend_guard) = backend.lock() {
                let info = backend_guard.info();
                if self.config.gpu_index > 0 {
                    format!("{} {} ", info.vendor.as_str(), self.config.gpu_index)
                } else {
                    format!("{} ", info.vendor.as_str())
                }
            } else {
                "GPU ".to_string()
            }
        } else {
            "GPU ".to_string()
        };

        let field_type = match self.config.field {
            GpuField::Temperature => "Temp",
            GpuField::Utilization => "Load",
            GpuField::MemoryUsed => "VRAM",
            GpuField::MemoryTotal => "VRAM Total",
            GpuField::MemoryPercent => "VRAM %",
            GpuField::PowerUsage => "Power",
            GpuField::FanSpeed => "Fan",
            GpuField::ClockCore => "Core Clock",
            GpuField::ClockMemory => "Mem Clock",
        };

        format!("{}{}", gpu_prefix, field_type)
    }
}

impl Default for GpuSource {
    fn default() -> Self {
        Self::new()
    }
}

impl DataSource for GpuSource {
    fn metadata(&self) -> &SourceMetadata {
        &self.metadata
    }

    fn fields(&self) -> Vec<FieldMetadata> {
        vec![
            FieldMetadata::new(
                "caption",
                "Caption",
                "Label identifying the GPU metric",
                FieldType::Text,
                FieldPurpose::Caption,
            ),
            FieldMetadata::new(
                "value",
                "Value (Configured)",
                "The configured value (temperature/utilization/memory/power/fan based on GPU settings)",
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
        let backend = self.backend.as_ref()
            .ok_or_else(|| anyhow!("No GPU backend available for index {}", self.config.gpu_index))?;

        let mut backend_guard = backend.lock()
            .map_err(|e| anyhow!("Failed to lock GPU backend: {}", e))?;

        // Update backend (refresh hardware data)
        backend_guard.update()?;

        // Copy metrics to our cache
        let metrics = backend_guard.metrics();
        self.temperature = metrics.temperature;
        self.utilization = metrics.utilization;
        self.memory_used = metrics.memory_used;
        self.memory_total = metrics.memory_total;
        self.power_usage = metrics.power_usage;
        self.fan_speed = metrics.fan_speed;
        self.clock_core = metrics.clock_core;
        self.clock_memory = metrics.clock_memory;

        Ok(())
    }

    fn get_values(&self) -> HashMap<String, Value> {
        let mut values = HashMap::new();

        let caption = self.config.custom_caption
            .as_ref()
            .cloned()
            .unwrap_or_else(|| self.generate_auto_caption());

        match self.config.field {
            GpuField::Temperature => {
                if let Some(temp) = self.temperature {
                    let converted = self.convert_temperature(temp);
                    values.insert("caption".to_string(), Value::from(caption));
                    values.insert("value".to_string(), Value::from(converted));
                    values.insert("temperature".to_string(), Value::from(converted));
                    values.insert("unit".to_string(), Value::from(self.get_temperature_unit_string()));
                } else {
                    values.insert("caption".to_string(), Value::from(caption));
                    values.insert("value".to_string(), Value::from("N/A"));
                    values.insert("unit".to_string(), Value::from(""));
                }
            }
            GpuField::Utilization => {
                if let Some(util) = self.utilization {
                    values.insert("caption".to_string(), Value::from(caption));
                    values.insert("value".to_string(), Value::from(util));
                    values.insert("utilization".to_string(), Value::from(util));
                    values.insert("unit".to_string(), Value::from("%"));
                } else {
                    values.insert("caption".to_string(), Value::from(caption));
                    values.insert("value".to_string(), Value::from("N/A"));
                    values.insert("unit".to_string(), Value::from(""));
                }
            }
            GpuField::MemoryUsed => {
                if let Some(mem) = self.memory_used {
                    let converted = self.convert_memory(mem);
                    values.insert("caption".to_string(), Value::from(caption));
                    values.insert("value".to_string(), Value::from(converted));
                    values.insert("memory_used".to_string(), Value::from(converted));
                    values.insert("unit".to_string(), Value::from(self.get_memory_unit_string()));
                } else {
                    values.insert("caption".to_string(), Value::from(caption));
                    values.insert("value".to_string(), Value::from("N/A"));
                    values.insert("unit".to_string(), Value::from(""));
                }
            }
            GpuField::MemoryPercent => {
                if let (Some(used), Some(total)) = (self.memory_used, self.memory_total) {
                    if total > 0 {
                        let percent = (used as f64 / total as f64 * 100.0) as u32;
                        values.insert("caption".to_string(), Value::from(caption));
                        values.insert("value".to_string(), Value::from(percent));
                        values.insert("memory_percent".to_string(), Value::from(percent));
                        values.insert("unit".to_string(), Value::from("%"));
                    } else {
                        values.insert("caption".to_string(), Value::from(caption));
                        values.insert("value".to_string(), Value::from("N/A"));
                        values.insert("unit".to_string(), Value::from(""));
                    }
                } else {
                    values.insert("caption".to_string(), Value::from(caption));
                    values.insert("value".to_string(), Value::from("N/A"));
                    values.insert("unit".to_string(), Value::from(""));
                }
            }
            GpuField::PowerUsage => {
                if let Some(power) = self.power_usage {
                    values.insert("caption".to_string(), Value::from(caption));
                    values.insert("value".to_string(), Value::from(power));
                    values.insert("power".to_string(), Value::from(power));
                    values.insert("unit".to_string(), Value::from("W"));
                } else {
                    values.insert("caption".to_string(), Value::from(caption));
                    values.insert("value".to_string(), Value::from("N/A"));
                    values.insert("unit".to_string(), Value::from(""));
                }
            }
            GpuField::FanSpeed => {
                if let Some(fan) = self.fan_speed {
                    values.insert("caption".to_string(), Value::from(caption));
                    values.insert("value".to_string(), Value::from(fan));
                    values.insert("fan_speed".to_string(), Value::from(fan));
                    values.insert("unit".to_string(), Value::from("%"));
                } else {
                    values.insert("caption".to_string(), Value::from(caption));
                    values.insert("value".to_string(), Value::from("N/A"));
                    values.insert("unit".to_string(), Value::from(""));
                }
            }
            GpuField::MemoryTotal => {
                if let Some(mem) = self.memory_total {
                    let converted = self.convert_memory(mem);
                    values.insert("caption".to_string(), Value::from(caption));
                    values.insert("value".to_string(), Value::from(converted));
                    values.insert("memory_total".to_string(), Value::from(converted));
                    values.insert("unit".to_string(), Value::from(self.get_memory_unit_string()));
                } else {
                    values.insert("caption".to_string(), Value::from(caption));
                    values.insert("value".to_string(), Value::from("N/A"));
                    values.insert("unit".to_string(), Value::from(""));
                }
            }
            GpuField::ClockCore => {
                if let Some(clock) = self.clock_core {
                    use crate::ui::GpuFrequencyUnit;
                    let (value, unit) = match self.config.frequency_unit {
                        GpuFrequencyUnit::MHz => (clock as f64, "MHz"),
                        GpuFrequencyUnit::GHz => (clock as f64 / 1000.0, "GHz"),
                    };
                    values.insert("caption".to_string(), Value::from(caption));
                    values.insert("value".to_string(), Value::from(value));
                    values.insert("clock_core".to_string(), Value::from(value));
                    values.insert("unit".to_string(), Value::from(unit));
                } else {
                    values.insert("caption".to_string(), Value::from(caption));
                    values.insert("value".to_string(), Value::from("N/A"));
                    values.insert("unit".to_string(), Value::from(""));
                }
            }
            GpuField::ClockMemory => {
                if let Some(clock) = self.clock_memory {
                    use crate::ui::GpuFrequencyUnit;
                    let (value, unit) = match self.config.frequency_unit {
                        GpuFrequencyUnit::MHz => (clock as f64, "MHz"),
                        GpuFrequencyUnit::GHz => (clock as f64 / 1000.0, "GHz"),
                    };
                    values.insert("caption".to_string(), Value::from(caption));
                    values.insert("value".to_string(), Value::from(value));
                    values.insert("clock_memory".to_string(), Value::from(value));
                    values.insert("unit".to_string(), Value::from(unit));
                } else {
                    values.insert("caption".to_string(), Value::from(caption));
                    values.insert("value".to_string(), Value::from("N/A"));
                    values.insert("unit".to_string(), Value::from(""));
                }
            }
        }

        // Also provide all raw data
        if let Some(temp) = self.temperature {
            values.insert("raw_temperature_celsius".to_string(), Value::from(temp));
        }
        if let Some(util) = self.utilization {
            values.insert("raw_utilization".to_string(), Value::from(util));
        }
        if let Some(mem_used) = self.memory_used {
            values.insert("raw_memory_used_bytes".to_string(), Value::from(mem_used));
        }
        if let Some(mem_total) = self.memory_total {
            values.insert("raw_memory_total_bytes".to_string(), Value::from(mem_total));
        }
        if let Some(power) = self.power_usage {
            values.insert("raw_power_watts".to_string(), Value::from(power));
        }
        if let Some(fan) = self.fan_speed {
            values.insert("raw_fan_speed".to_string(), Value::from(fan));
        }

        // Add limits
        let (min_limit, max_limit) = match self.config.field {
            GpuField::Temperature => {
                if self.config.auto_detect_limits {
                    // Auto-detect reasonable temperature range
                    (0.0, 100.0)
                } else {
                    (self.config.min_limit.unwrap_or(0.0), self.config.max_limit.unwrap_or(100.0))
                }
            }
            GpuField::Utilization | GpuField::MemoryPercent | GpuField::FanSpeed => (0.0, 100.0),
            GpuField::MemoryUsed => {
                if let Some(total) = self.memory_total {
                    (0.0, self.convert_memory(total))
                } else {
                    (0.0, 100.0)
                }
            }
            GpuField::PowerUsage => {
                if self.config.auto_detect_limits {
                    (0.0, 300.0) // Reasonable default for most GPUs
                } else {
                    (self.config.min_limit.unwrap_or(0.0), self.config.max_limit.unwrap_or(300.0))
                }
            }
            GpuField::MemoryTotal => {
                if let Some(total) = self.memory_total {
                    (0.0, self.convert_memory(total))
                } else {
                    (0.0, 100.0)
                }
            }
            GpuField::ClockCore => {
                use crate::ui::GpuFrequencyUnit;
                let default_max = match self.config.frequency_unit {
                    GpuFrequencyUnit::MHz => 3000.0,
                    GpuFrequencyUnit::GHz => 3.0, // 3 GHz = 3000 MHz
                };
                if self.config.auto_detect_limits {
                    (0.0, default_max)
                } else {
                    (self.config.min_limit.unwrap_or(0.0), self.config.max_limit.unwrap_or(default_max))
                }
            }
            GpuField::ClockMemory => {
                use crate::ui::GpuFrequencyUnit;
                let default_max = match self.config.frequency_unit {
                    GpuFrequencyUnit::MHz => 2500.0,
                    GpuFrequencyUnit::GHz => 2.5, // 2.5 GHz = 2500 MHz
                };
                if self.config.auto_detect_limits {
                    (0.0, default_max)
                } else {
                    (self.config.min_limit.unwrap_or(0.0), self.config.max_limit.unwrap_or(default_max))
                }
            }
        };

        values.insert("min_limit".to_string(), Value::from(min_limit));
        values.insert("max_limit".to_string(), Value::from(max_limit));

        values
    }

    fn configure(&mut self, config: &HashMap<String, Value>) -> Result<()> {
        // Look for gpu_config in the configuration
        if let Some(gpu_config_value) = config.get("gpu_config") {
            // Try to deserialize it into GpuSourceConfig
            if let Ok(gpu_config) = serde_json::from_value::<GpuSourceConfig>(gpu_config_value.clone()) {
                self.set_config(gpu_config);
            }
        }
        Ok(())
    }
}
