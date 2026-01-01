//! Abstract GPU backend trait

use anyhow::Result;
use super::nvidia::NvidiaBackend;
use super::amd::AmdBackend;

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

/// Concrete GPU backend enum - eliminates Box<dyn GpuBackend> indirection
///
/// This enum wraps the concrete backend types, allowing us to use
/// Arc<Mutex<GpuBackendEnum>> instead of Arc<Mutex<Box<dyn GpuBackend>>>.
/// This eliminates one level of pointer indirection in hot paths.
pub enum GpuBackendEnum {
    Nvidia(NvidiaBackend),
    Amd(AmdBackend),
}

impl GpuBackend for GpuBackendEnum {
    fn info(&self) -> &GpuInfo {
        match self {
            GpuBackendEnum::Nvidia(b) => b.info(),
            GpuBackendEnum::Amd(b) => b.info(),
        }
    }

    fn update(&mut self) -> Result<()> {
        match self {
            GpuBackendEnum::Nvidia(b) => b.update(),
            GpuBackendEnum::Amd(b) => b.update(),
        }
    }

    fn metrics(&self) -> &GpuMetrics {
        match self {
            GpuBackendEnum::Nvidia(b) => b.metrics(),
            GpuBackendEnum::Amd(b) => b.metrics(),
        }
    }

    fn is_available(&self) -> bool {
        match self {
            GpuBackendEnum::Nvidia(b) => b.is_available(),
            GpuBackendEnum::Amd(b) => b.is_available(),
        }
    }
}
