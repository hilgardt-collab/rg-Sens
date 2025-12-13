//! Abstract GPU backend trait

use anyhow::Result;

/// GPU information structure
#[derive(Debug, Clone)]
pub struct GpuInfo {
    pub index: u32,
    pub name: String,
    pub vendor: GpuVendor,
}

/// GPU vendor enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum GpuVendor {
    Nvidia,
    Amd,
    Intel,
    Unknown,
}

impl GpuVendor {
    pub fn as_str(&self) -> &'static str {
        match self {
            GpuVendor::Nvidia => "NVIDIA",
            GpuVendor::Amd => "AMD",
            GpuVendor::Intel => "Intel",
            GpuVendor::Unknown => "Unknown",
        }
    }
}

/// GPU metrics structure
#[derive(Debug, Clone, Default)]
pub struct GpuMetrics {
    pub temperature: Option<f32>,         // Temperature in Celsius
    pub utilization: Option<u32>,         // GPU utilization in %
    pub memory_used: Option<u64>,         // Memory used in bytes
    pub memory_total: Option<u64>,        // Total memory in bytes
    pub power_usage: Option<f32>,         // Power usage in Watts
    pub fan_speed: Option<u32>,           // Fan speed in %
    pub clock_core: Option<u32>,          // Core clock in MHz
    pub clock_memory: Option<u32>,        // Memory clock in MHz
}

/// Abstract GPU backend trait
///
/// This trait abstracts GPU monitoring across different vendors (NVIDIA, AMD, Intel).
/// Each vendor implements this trait to provide consistent GPU metrics.
pub trait GpuBackend: Send + Sync {
    /// Get information about this GPU
    fn info(&self) -> &GpuInfo;

    /// Update GPU metrics (refresh data from hardware)
    fn update(&mut self) -> Result<()>;

    /// Get current metrics
    fn metrics(&self) -> &GpuMetrics;

    /// Get GPU vendor
    #[allow(dead_code)]
    fn vendor(&self) -> GpuVendor {
        self.info().vendor
    }

    /// Check if this backend is available/functional
    #[allow(dead_code)]
    fn is_available(&self) -> bool;
}
