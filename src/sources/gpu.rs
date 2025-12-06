//! GPU data source implementation using NVML

use crate::core::{DataSource, FieldMetadata, FieldPurpose, FieldType, SourceMetadata};
use crate::ui::{GpuField, GpuSourceConfig, TemperatureUnit};
use anyhow::{anyhow, Result};
use once_cell::sync::Lazy;
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;

#[cfg(feature = "nvidia")]
use nvml_wrapper::Nvml;

/// GPU information cached at startup
struct GpuHardwareInfo {
    gpu_count: u32,
    gpu_names: Vec<String>,
}

/// Global GPU hardware cache (initialized once)
static GPU_HARDWARE_INFO: Lazy<GpuHardwareInfo> = Lazy::new(|| {
    log::info!("=== Discovering GPU hardware (one-time initialization) ===");

    #[cfg(feature = "nvidia")]
    {
        match Nvml::init() {
            Ok(nvml) => {
                match nvml.device_count() {
                    Ok(count) => {
                        log::info!("Found {} NVIDIA GPU(s)", count);
                        let mut names = Vec::new();
                        for i in 0..count {
                            if let Ok(device) = nvml.device_by_index(i) {
                                if let Ok(name) = device.name() {
                                    log::info!("  GPU {}: {}", i, name);
                                    names.push(name);
                                } else {
                                    names.push(format!("GPU {}", i));
                                }
                            } else {
                                names.push(format!("GPU {}", i));
                            }
                        }
                        GpuHardwareInfo {
                            gpu_count: count,
                            gpu_names: names,
                        }
                    }
                    Err(e) => {
                        log::warn!("Failed to get GPU count: {}", e);
                        GpuHardwareInfo {
                            gpu_count: 0,
                            gpu_names: Vec::new(),
                        }
                    }
                }
            }
            Err(e) => {
                log::warn!("Failed to initialize NVML: {}", e);
                GpuHardwareInfo {
                    gpu_count: 0,
                    gpu_names: Vec::new(),
                }
            }
        }
    }

    #[cfg(not(feature = "nvidia"))]
    {
        log::warn!("NVIDIA support not enabled at compile time");
        GpuHardwareInfo {
            gpu_count: 0,
            gpu_names: Vec::new(),
        }
    }
});

/// GPU data source
pub struct GpuSource {
    metadata: SourceMetadata,
    config: GpuSourceConfig,
    #[cfg(feature = "nvidia")]
    nvml: Option<Nvml>,

    // Cached values
    temperature: Option<f32>,
    utilization: Option<u32>,
    memory_used: Option<u64>,
    memory_total: Option<u64>,
    power_usage: Option<f32>,
    fan_speed: Option<u32>,
}

impl GpuSource {
    pub fn new() -> Self {
        let metadata = SourceMetadata {
            id: "gpu".to_string(),
            name: "GPU".to_string(),
            description: "NVIDIA GPU monitoring (temperature, utilization, memory, power)".to_string(),
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

        #[cfg(feature = "nvidia")]
        let nvml = Nvml::init().ok();

        #[cfg(not(feature = "nvidia"))]
        let nvml = None;

        Self {
            metadata,
            config: GpuSourceConfig::default(),
            #[cfg(feature = "nvidia")]
            nvml,
            temperature: None,
            utilization: None,
            memory_used: None,
            memory_total: None,
            power_usage: None,
            fan_speed: None,
        }
    }

    /// Get cached GPU count
    pub fn get_cached_gpu_count() -> u32 {
        GPU_HARDWARE_INFO.gpu_count
    }

    /// Get cached GPU names
    pub fn get_cached_gpu_names() -> &'static [String] {
        &GPU_HARDWARE_INFO.gpu_names
    }

    /// Set configuration
    pub fn set_config(&mut self, config: GpuSourceConfig) {
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

    /// Generate automatic caption
    fn generate_auto_caption(&self) -> String {
        let gpu_prefix = if self.config.gpu_index > 0 {
            format!("GPU {} ", self.config.gpu_index)
        } else {
            "GPU ".to_string()
        };

        let field_type = match self.config.field {
            GpuField::Temperature => "Temp",
            GpuField::Utilization => "Load",
            GpuField::MemoryUsed => "VRAM",
            GpuField::MemoryPercent => "VRAM %",
            GpuField::PowerUsage => "Power",
            GpuField::FanSpeed => "Fan",
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
        #[cfg(feature = "nvidia")]
        {
            if let Some(ref nvml) = self.nvml {
                match nvml.device_by_index(self.config.gpu_index) {
                    Ok(device) => {
                        // Temperature
                        self.temperature = device
                            .temperature(nvml_wrapper::enum_wrappers::device::TemperatureSensor::Gpu)
                            .ok()
                            .map(|t| t as f32);

                        // Utilization
                        self.utilization = device.utilization_rates().ok().map(|u| u.gpu);

                        // Memory
                        if let Ok(mem_info) = device.memory_info() {
                            self.memory_used = Some(mem_info.used);
                            self.memory_total = Some(mem_info.total);
                        }

                        // Power
                        self.power_usage = device.power_usage().ok().map(|p| p as f32 / 1000.0); // mW to W

                        // Fan speed
                        self.fan_speed = device.fan_speed(0).ok();

                        Ok(())
                    }
                    Err(e) => Err(anyhow!("Failed to get GPU device: {}", e)),
                }
            } else {
                Err(anyhow!("NVML not initialized"))
            }
        }

        #[cfg(not(feature = "nvidia"))]
        Err(anyhow!("NVIDIA support not enabled"))
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
                    let mem_gb = mem as f64 / (1024.0 * 1024.0 * 1024.0);
                    values.insert("caption".to_string(), Value::from(caption));
                    values.insert("value".to_string(), Value::from(mem_gb));
                    values.insert("memory_used".to_string(), Value::from(mem_gb));
                    values.insert("unit".to_string(), Value::from("GB"));
                } else {
                    values.insert("caption".to_string(), Value::from(caption));
                    values.insert("value".to_string(), Value::from("N/A"));
                    values.insert("unit".to_string(), Value::from(""));
                }
            }
            GpuField::MemoryPercent => {
                if let (Some(used), Some(total)) = (self.memory_used, self.memory_total) {
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
        }

        values
    }

    fn is_available(&self) -> bool {
        #[cfg(feature = "nvidia")]
        {
            self.nvml.is_some() && GPU_HARDWARE_INFO.gpu_count > 0
        }

        #[cfg(not(feature = "nvidia"))]
        false
    }

    fn configure(&mut self, config: &HashMap<String, Value>) -> Result<()> {
        if let Some(gpu_config_value) = config.get("gpu_config") {
            if let Ok(gpu_config) = serde_json::from_value::<GpuSourceConfig>(gpu_config_value.clone()) {
                self.set_config(gpu_config);
            }
        }
        Ok(())
    }
}
