//! NVIDIA GPU backend using NVML

use super::backend::{GpuBackend, GpuInfo, GpuMetrics};
#[cfg(feature = "nvidia")]
use super::backend::GpuVendor;
use anyhow::{anyhow, Result};

#[cfg(feature = "nvidia")]
use nvml_wrapper::{enum_wrappers::device::TemperatureSensor, Nvml};
#[cfg(feature = "nvidia")]
use once_cell::sync::Lazy;
#[cfg(feature = "nvidia")]
use std::sync::Arc;

/// Process-wide NVML handle, initialized once and shared between GPU
/// detection and every NVIDIA backend (NVML init is not free — previously it
/// ran N+1 times at startup).
#[cfg(feature = "nvidia")]
static SHARED_NVML: Lazy<Option<Arc<Nvml>>> = Lazy::new(|| match Nvml::init() {
    Ok(nvml) => Some(Arc::new(nvml)),
    Err(e) => {
        log::info!("NVML: Not available ({})", e);
        None
    }
});

/// Get the shared NVML instance (`None` if NVML is unavailable).
#[cfg(feature = "nvidia")]
pub fn shared_nvml() -> Option<Arc<Nvml>> {
    SHARED_NVML.clone()
}

/// NVIDIA GPU backend
pub struct NvidiaBackend {
    info: GpuInfo,
    metrics: GpuMetrics,
    #[cfg(feature = "nvidia")]
    nvml: Arc<Nvml>,
    #[cfg(feature = "nvidia")]
    device_index: u32,
}

impl NvidiaBackend {
    /// Create a new NVIDIA backend for the specified GPU index
    #[cfg(feature = "nvidia")]
    pub fn new(index: u32) -> Result<Self> {
        let nvml = shared_nvml().ok_or_else(|| anyhow!("NVML not available"))?;
        let device = nvml.device_by_index(index)?;
        let name = device
            .name()
            .unwrap_or_else(|_| format!("NVIDIA GPU {}", index));

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

}

impl GpuBackend for NvidiaBackend {
    fn info(&self) -> &GpuInfo {
        &self.info
    }

    fn update(&mut self) -> Result<()> {
        #[cfg(feature = "nvidia")]
        {
            let device = self
                .nvml
                .device_by_index(self.device_index)
                .map_err(|e| anyhow!("Failed to get NVIDIA GPU device: {}", e))?;

            // Temperature
            self.metrics.temperature = device
                .temperature(TemperatureSensor::Gpu)
                .ok()
                .map(|t| t as f32);

            // Utilization
            self.metrics.utilization = device.utilization_rates().ok().map(|u| u.gpu);

            // Memory (reset to None on failure so stale values don't freeze)
            match device.memory_info() {
                Ok(mem_info) => {
                    self.metrics.memory_used = Some(mem_info.used);
                    self.metrics.memory_total = Some(mem_info.total);
                }
                Err(_) => {
                    self.metrics.memory_used = None;
                    self.metrics.memory_total = None;
                }
            }

            // Power
            self.metrics.power_usage = device.power_usage().ok().map(|p| p as f32 / 1000.0); // mW to W

            // Fan speed
            self.metrics.fan_speed = device.fan_speed(0).ok();

            // Clock speeds
            self.metrics.clock_core = device
                .clock_info(nvml_wrapper::enum_wrappers::device::Clock::Graphics)
                .ok();
            self.metrics.clock_memory = device
                .clock_info(nvml_wrapper::enum_wrappers::device::Clock::Memory)
                .ok();

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
