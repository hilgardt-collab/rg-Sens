//! Built-in data sources
//!
//! This module contains implementations of various system metric sources.
//! Each source collects specific system information (CPU, memory, GPU, etc.)

mod cpu;
mod gpu;
mod memory;
mod system_temp;
mod fan_speed;
mod disk;
// mod network;

pub use cpu::{CpuSensor, CpuSource};
pub use gpu::GpuSource;
pub use memory::MemorySource;
pub use system_temp::{SystemTempSource, SensorInfo, SensorCategory, SystemTempConfig, TemperatureUnit as SystemTempUnit};
pub use fan_speed::{FanSpeedSource, FanInfo, FanCategory, FanSpeedConfig};
pub use disk::DiskSource;
// pub use network::NetworkSource;

/// Register all built-in sources with the global registry
pub fn register_all() {
    use crate::core::global_registry;

    // Register CPU source
    global_registry().register_source("cpu", || Box::new(CpuSource::new()));

    // Register GPU source
    global_registry().register_source("gpu", || Box::new(GpuSource::new()));

    // Register Memory source
    global_registry().register_source("memory", || Box::new(MemorySource::new()));

    // Register System Temperature source
    global_registry().register_source("system_temp", || Box::new(SystemTempSource::new()));

    // Register Fan Speed source
    global_registry().register_source("fan_speed", || Box::new(FanSpeedSource::new()));

    // Register Disk source
    global_registry().register_source("disk", || Box::new(DiskSource::new()));

    // TODO: Register more sources
    // register_source!("network", NetworkSource);
}
