//! NVIDIA GPU backend using NVML

use super::backend::{GpuBackend, GpuInfo, GpuMetrics, GpuVendor};
use anyhow::{anyhow, Result};

#[cfg(feature = "nvidia")]
use nvml_wrapper::{enum_wrappers::device::TemperatureSensor, Nvml};

/// NVIDIA GPU backend
pub struct NvidiaBackend {
    info: GpuInfo,
    metrics: GpuMetrics,
    #[cfg(feature = "nvidia")]
    nvml: Nvml,
    #[cfg(feature = "nvidia")]
    device_index: u32,
}

impl NvidiaBackend {
    /// Create a new NVIDIA backend for the specified GPU index
    #[cfg(feature = "nvidia")]
    pub fn new(index: u32) -> Result<Self> {
        let nvml = Nvml::init()?;
        let device = nvml.device_by_index(index)?;
        let name = device.name().unwrap_or_else(|_| format!("NVIDIA GPU {}", index));

        Ok(Self {
            info: GpuInfo {
                index,
                name,
                vendor: GpuVendor::Nvidia,
            },
            metrics: GpuMetrics::default(),
            nvml,
            device_index: index,
        })
    }

    /// Create a new NVIDIA backend (disabled when nvidia feature is off)
    #[cfg(not(feature = "nvidia"))]
    pub fn new(_index: u32) -> Result<Self> {
        Err(anyhow!("NVIDIA support not enabled at compile time"))
    }
}

impl GpuBackend for NvidiaBackend {
    fn info(&self) -> &GpuInfo {
        &self.info
    }

    fn update(&mut self) -> Result<()> {
        #[cfg(feature = "nvidia")]
        {
            let device = self.nvml.device_by_index(self.device_index)
                .map_err(|e| anyhow!("Failed to get NVIDIA GPU device: {}", e))?;

            // Temperature
            self.metrics.temperature = device
                .temperature(TemperatureSensor::Gpu)
                .ok()
                .map(|t| t as f32);

            // Utilization
            self.metrics.utilization = device.utilization_rates().ok().map(|u| u.gpu);

            // Memory
            if let Ok(mem_info) = device.memory_info() {
                self.metrics.memory_used = Some(mem_info.used);
                self.metrics.memory_total = Some(mem_info.total);
            }

            // Power
            self.metrics.power_usage = device.power_usage().ok().map(|p| p as f32 / 1000.0); // mW to W

            // Fan speed
            self.metrics.fan_speed = device.fan_speed(0).ok();

            Ok(())
        }

        #[cfg(not(feature = "nvidia"))]
        Err(anyhow!("NVIDIA support not enabled"))
    }

    fn metrics(&self) -> &GpuMetrics {
        &self.metrics
    }

    fn is_available(&self) -> bool {
        #[cfg(feature = "nvidia")]
        {
            self.nvml.device_by_index(self.device_index).is_ok()
        }

        #[cfg(not(feature = "nvidia"))]
        false
    }
}
