//! Built-in data sources
//!
//! This module contains implementations of various system metric sources.
//! Each source collects specific system information (CPU, memory, GPU, etc.)

mod cpu;
// mod memory;
// mod gpu_nvidia;
// mod temps;
// mod disk;
// mod network;

pub use cpu::CpuSource;
// pub use memory::MemorySource;
// pub use gpu_nvidia::NvidiaGpuSource;
// pub use temps::TemperatureSource;
// pub use disk::DiskSource;
// pub use network::NetworkSource;

/// Register all built-in sources with the global registry
pub fn register_all() {
    use crate::core::global_registry;

    // Register CPU source
    global_registry().register_source("cpu", || Box::new(CpuSource::new()));

    // TODO: Register more sources
    // register_source!("memory", MemorySource);
    // register_source!("gpu.nvidia", NvidiaGpuSource);
    // register_source!("temps", TemperatureSource);
    // register_source!("disk", DiskSource);
    // register_source!("network", NetworkSource);
}
