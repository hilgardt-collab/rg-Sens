//! GPU data source with multi-vendor support (NVIDIA, AMD)

mod amd;
mod backend;
mod detector;
mod nvidia;

pub use backend::{GpuBackend, GpuBackendEnum, GpuInfo};
use detector::detect_gpus;

use rg_sens_core::constants::{BYTES_PER_GB, BYTES_PER_MB};
use rg_sens_core::{
    DataSource, FieldMetadata, FieldPurpose, FieldType, SourceConfig, SourceMetadata,
};
use rg_sens_types::source_configs::{GpuField, GpuSourceConfig, MemoryUnit, TemperatureUnit};
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
        backends: detected
            .gpus
            .into_iter()
            .map(|b| Arc::new(Mutex::new(b)))
            .collect(),
        gpu_info: detected.info,
    }
});

/// Cached GPU names (computed once from GPU_MANAGER)
static GPU_NAMES: Lazy<Vec<String>> = Lazy::new(|| {
    GPU_MANAGER
        .gpu_info
        .iter()
        .map(|info| info.name.clone())
        .collect()
});

/// GPU manager holding all detected GPU backends
struct GpuManager {
    /// Backends using enum instead of Box<dyn> for single indirection
    backends: Vec<Arc<Mutex<GpuBackendEnum>>>,
    gpu_info: Vec<GpuInfo>,
}

/// GPU data source
pub struct GpuSource {
    metadata: SourceMetadata,
    config: GpuSourceConfig,
    /// Backend using enum for single indirection (no Box overhead)
    backend: Option<Arc<Mutex<GpuBackendEnum>>>,

    // Cached static info (doesn't change, avoids mutex lock in hot paths)
    cached_vendor: String,

    // Cached values (read from backend after update)
    temperature: Option<f32>,
    utilization: Option<u32>,
    memory_used: Option<u64>,
    memory_total: Option<u64>,
    power_usage: Option<f32>,
    fan_speed: Option<u32>,
    clock_core: Option<u32>,
    clock_memory: Option<u32>,

    /// Cached output values - updated in update(), returned by reference in values_ref()
    values: HashMap<String, Value>,
}

impl GpuSource {
    pub fn new() -> Self {
        let metadata = SourceMetadata {
            id: "gpu".to_string(),
            name: "GPU".to_string(),
            description: "GPU monitoring (NVIDIA/AMD) - temperature, utilization, memory, power"
                .to_string(),
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

        // Cache the vendor string to avoid mutex lock in generate_auto_caption
        let cached_vendor = GPU_MANAGER
            .gpu_info
            .first()
            .map(|info| info.vendor.as_str().to_string())
            .unwrap_or_else(|| "GPU".to_string());

        Self {
            metadata,
            config: GpuSourceConfig::default(),
            backend,
            cached_vendor,
            temperature: None,
            utilization: None,
            memory_used: None,
            memory_total: None,
            power_usage: None,
            fan_speed: None,
            clock_core: None,
            clock_memory: None,
            values: HashMap::with_capacity(24),
        }
    }

    /// Get cached GPU count
    pub fn get_cached_gpu_count() -> u32 {
        GPU_MANAGER.gpu_info.len() as u32
    }

    /// Get cached GPU names (returns clone of pre-computed names)
    pub fn get_cached_gpu_names() -> Vec<String> {
        GPU_NAMES.clone()
    }

    /// Set configuration
    pub fn set_config(&mut self, config: GpuSourceConfig) {
        let mut config = config;
        let gpu_count = GPU_MANAGER.backends.len();

        // Clamp gpu_index to valid range to ensure config and backend stay in sync
        if gpu_count > 0 && (config.gpu_index as usize) >= gpu_count {
            log::warn!(
                "GPU index {} out of bounds (only {} GPUs available), clamping to {}",
                config.gpu_index,
                gpu_count,
                gpu_count - 1
            );
            config.gpu_index = (gpu_count - 1) as u32;
        }

        // Update backend and cached vendor if GPU index changed
        if config.gpu_index != self.config.gpu_index {
            self.backend = GPU_MANAGER.backends.get(config.gpu_index as usize).cloned();
            // Update cached vendor for the new GPU
            self.cached_vendor = GPU_MANAGER
                .gpu_info
                .get(config.gpu_index as usize)
                .map(|info| info.vendor.as_str().to_string())
                .unwrap_or_else(|| "GPU".to_string());
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

    /// Generate automatic caption using cached vendor (no mutex lock needed)
    fn generate_auto_caption(&self) -> String {
        // Use cached vendor string to avoid locking backend mutex
        let gpu_prefix = if self.config.gpu_index > 0 {
            format!("{} {} ", self.cached_vendor, self.config.gpu_index)
        } else {
            format!("{} ", self.cached_vendor)
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

    /// Helper to insert N/A values for unavailable metrics (reduces code duplication)
    #[inline]
    fn insert_na_values(values: &mut HashMap<String, Value>, caption: String) {
        values.insert("caption".into(), Value::from(caption));
        values.insert("value".into(), Value::from("N/A"));
        values.insert("unit".into(), Value::from(""));
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
        let backend = self.backend.as_ref().ok_or_else(|| {
            anyhow!(
                "No GPU backend available for index {}",
                self.config.gpu_index
            )
        })?;

        let mut backend_guard = backend
            .lock()
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

        // Build values HashMap (reuse allocation, just clear and refill)
        self.values.clear();

        let caption = self
            .config
            .custom_caption
            .as_ref()
            .cloned()
            .unwrap_or_else(|| self.generate_auto_caption());

        // Use static string keys to avoid repeated allocations
        const KEY_CAPTION: &str = "caption";
        const KEY_VALUE: &str = "value";
        const KEY_UNIT: &str = "unit";

        match self.config.field {
            GpuField::Temperature => {
                if let Some(temp) = self.temperature {
                    let converted = self.convert_temperature(temp);
                    self.values.insert(KEY_CAPTION.into(), Value::from(caption));
                    self.values.insert(KEY_VALUE.into(), Value::from(converted));
                    self.values
                        .insert("temperature".into(), Value::from(converted));
                    self.values.insert(
                        KEY_UNIT.into(),
                        Value::from(self.get_temperature_unit_string()),
                    );
                } else {
                    Self::insert_na_values(&mut self.values, caption);
                }
            }
            GpuField::Utilization => {
                if let Some(util) = self.utilization {
                    self.values.insert(KEY_CAPTION.into(), Value::from(caption));
                    self.values.insert(KEY_VALUE.into(), Value::from(util));
                    self.values.insert("utilization".into(), Value::from(util));
                    self.values.insert(KEY_UNIT.into(), Value::from("%"));
                } else {
                    Self::insert_na_values(&mut self.values, caption);
                }
            }
            GpuField::MemoryUsed => {
                if let Some(mem) = self.memory_used {
                    let converted = self.convert_memory(mem);
                    self.values.insert(KEY_CAPTION.into(), Value::from(caption));
                    self.values.insert(KEY_VALUE.into(), Value::from(converted));
                    self.values
                        .insert("memory_used".into(), Value::from(converted));
                    self.values
                        .insert(KEY_UNIT.into(), Value::from(self.get_memory_unit_string()));
                } else {
                    Self::insert_na_values(&mut self.values, caption);
                }
            }
            GpuField::MemoryPercent => {
                if let (Some(used), Some(total)) = (self.memory_used, self.memory_total) {
                    if total > 0 {
                        let percent = (used as f64 / total as f64 * 100.0) as u32;
                        self.values.insert(KEY_CAPTION.into(), Value::from(caption));
                        self.values.insert(KEY_VALUE.into(), Value::from(percent));
                        self.values
                            .insert("memory_percent".into(), Value::from(percent));
                        self.values.insert(KEY_UNIT.into(), Value::from("%"));
                    } else {
                        Self::insert_na_values(&mut self.values, caption);
                    }
                } else {
                    Self::insert_na_values(&mut self.values, caption);
                }
            }
            GpuField::PowerUsage => {
                if let Some(power) = self.power_usage {
                    self.values.insert(KEY_CAPTION.into(), Value::from(caption));
                    self.values.insert(KEY_VALUE.into(), Value::from(power));
                    self.values.insert("power".into(), Value::from(power));
                    self.values.insert(KEY_UNIT.into(), Value::from("W"));
                } else {
                    Self::insert_na_values(&mut self.values, caption);
                }
            }
            GpuField::FanSpeed => {
                if let Some(fan) = self.fan_speed {
                    self.values.insert(KEY_CAPTION.into(), Value::from(caption));
                    self.values.insert(KEY_VALUE.into(), Value::from(fan));
                    self.values.insert("fan_speed".into(), Value::from(fan));
                    self.values.insert(KEY_UNIT.into(), Value::from("%"));
                } else {
                    Self::insert_na_values(&mut self.values, caption);
                }
            }
            GpuField::MemoryTotal => {
                if let Some(mem) = self.memory_total {
                    let converted = self.convert_memory(mem);
                    self.values.insert(KEY_CAPTION.into(), Value::from(caption));
                    self.values.insert(KEY_VALUE.into(), Value::from(converted));
                    self.values
                        .insert("memory_total".into(), Value::from(converted));
                    self.values
                        .insert(KEY_UNIT.into(), Value::from(self.get_memory_unit_string()));
                } else {
                    Self::insert_na_values(&mut self.values, caption);
                }
            }
            GpuField::ClockCore => {
                if let Some(clock) = self.clock_core {
                    use rg_sens_types::source_configs::gpu::FrequencyUnit as GpuFrequencyUnit;
                    let (value, unit) = match self.config.frequency_unit {
                        GpuFrequencyUnit::MHz => (clock as f64, "MHz"),
                        GpuFrequencyUnit::GHz => (clock as f64 / 1000.0, "GHz"),
                    };
                    self.values.insert(KEY_CAPTION.into(), Value::from(caption));
                    self.values.insert(KEY_VALUE.into(), Value::from(value));
                    self.values.insert("clock_core".into(), Value::from(value));
                    self.values.insert(KEY_UNIT.into(), Value::from(unit));
                } else {
                    Self::insert_na_values(&mut self.values, caption);
                }
            }
            GpuField::ClockMemory => {
                if let Some(clock) = self.clock_memory {
                    use rg_sens_types::source_configs::gpu::FrequencyUnit as GpuFrequencyUnit;
                    let (value, unit) = match self.config.frequency_unit {
                        GpuFrequencyUnit::MHz => (clock as f64, "MHz"),
                        GpuFrequencyUnit::GHz => (clock as f64 / 1000.0, "GHz"),
                    };
                    self.values.insert(KEY_CAPTION.into(), Value::from(caption));
                    self.values.insert(KEY_VALUE.into(), Value::from(value));
                    self.values
                        .insert("clock_memory".into(), Value::from(value));
                    self.values.insert(KEY_UNIT.into(), Value::from(unit));
                } else {
                    Self::insert_na_values(&mut self.values, caption);
                }
            }
        }

        // Also provide all raw data
        if let Some(temp) = self.temperature {
            self.values
                .insert("raw_temperature_celsius".into(), Value::from(temp));
        }
        if let Some(util) = self.utilization {
            self.values
                .insert("raw_utilization".into(), Value::from(util));
        }
        if let Some(mem_used) = self.memory_used {
            self.values
                .insert("raw_memory_used_bytes".into(), Value::from(mem_used));
        }
        if let Some(mem_total) = self.memory_total {
            self.values
                .insert("raw_memory_total_bytes".into(), Value::from(mem_total));
        }
        if let Some(power) = self.power_usage {
            self.values
                .insert("raw_power_watts".into(), Value::from(power));
        }
        if let Some(fan) = self.fan_speed {
            self.values.insert("raw_fan_speed".into(), Value::from(fan));
        }

        // Add limits
        let (min_limit, max_limit) = match self.config.field {
            GpuField::Temperature => {
                if self.config.auto_detect_limits {
                    // Auto-detect reasonable temperature range
                    (0.0, 100.0)
                } else {
                    (
                        self.config.min_limit.unwrap_or(0.0),
                        self.config.max_limit.unwrap_or(100.0),
                    )
                }
            }
            GpuField::Utilization | GpuField::MemoryPercent | GpuField::FanSpeed => (0.0, 100.0),
            GpuField::MemoryUsed => {
                if self.config.auto_detect_limits {
                    if let Some(total) = self.memory_total {
                        (0.0, self.convert_memory(total))
                    } else {
                        (0.0, 100.0)
                    }
                } else {
                    let default_max = if let Some(total) = self.memory_total {
                        self.convert_memory(total)
                    } else {
                        100.0
                    };
                    (
                        self.config.min_limit.unwrap_or(0.0),
                        self.config.max_limit.unwrap_or(default_max),
                    )
                }
            }
            GpuField::PowerUsage => {
                if self.config.auto_detect_limits {
                    (0.0, 300.0) // Reasonable default for most GPUs
                } else {
                    (
                        self.config.min_limit.unwrap_or(0.0),
                        self.config.max_limit.unwrap_or(300.0),
                    )
                }
            }
            GpuField::MemoryTotal => {
                if self.config.auto_detect_limits {
                    if let Some(total) = self.memory_total {
                        (0.0, self.convert_memory(total))
                    } else {
                        (0.0, 100.0)
                    }
                } else {
                    let default_max = if let Some(total) = self.memory_total {
                        self.convert_memory(total)
                    } else {
                        100.0
                    };
                    (
                        self.config.min_limit.unwrap_or(0.0),
                        self.config.max_limit.unwrap_or(default_max),
                    )
                }
            }
            GpuField::ClockCore => {
                use rg_sens_types::source_configs::gpu::FrequencyUnit as GpuFrequencyUnit;
                let default_max = match self.config.frequency_unit {
                    GpuFrequencyUnit::MHz => 3000.0,
                    GpuFrequencyUnit::GHz => 3.0, // 3 GHz = 3000 MHz
                };
                if self.config.auto_detect_limits {
                    (0.0, default_max)
                } else {
                    (
                        self.config.min_limit.unwrap_or(0.0),
                        self.config.max_limit.unwrap_or(default_max),
                    )
                }
            }
            GpuField::ClockMemory => {
                use rg_sens_types::source_configs::gpu::FrequencyUnit as GpuFrequencyUnit;
                let default_max = match self.config.frequency_unit {
                    GpuFrequencyUnit::MHz => 2500.0,
                    GpuFrequencyUnit::GHz => 2.5, // 2.5 GHz = 2500 MHz
                };
                if self.config.auto_detect_limits {
                    (0.0, default_max)
                } else {
                    (
                        self.config.min_limit.unwrap_or(0.0),
                        self.config.max_limit.unwrap_or(default_max),
                    )
                }
            }
        };

        self.values
            .insert("min_limit".into(), Value::from(min_limit));
        self.values
            .insert("max_limit".into(), Value::from(max_limit));

        Ok(())
    }

    fn get_values(&self) -> HashMap<String, Value> {
        self.values.clone()
    }

    fn values_ref(&self) -> Option<&HashMap<String, Value>> {
        Some(&self.values)
    }

    fn configure(&mut self, config: &HashMap<String, Value>) -> Result<()> {
        // Look for gpu_config in the configuration
        if let Some(gpu_config_value) = config.get("gpu_config") {
            // Try to deserialize it into GpuSourceConfig
            if let Ok(gpu_config) =
                serde_json::from_value::<GpuSourceConfig>(gpu_config_value.clone())
            {
                self.set_config(gpu_config);
            }
        }
        Ok(())
    }

    fn get_typed_config(&self) -> Option<SourceConfig> {
        Some(SourceConfig::Gpu(self.config.clone()))
    }
}
