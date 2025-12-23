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
mod clock;
mod shared_sensors;
mod combo;
mod test;
mod static_text;
// mod network;

pub use cpu::{CpuSensor, CpuSource};
pub use gpu::GpuSource;
pub use memory::MemorySource;
pub use system_temp::{SystemTempSource, SensorInfo, SensorCategory, SystemTempConfig, TemperatureUnit as SystemTempUnit};
pub use fan_speed::{FanSpeedSource, FanInfo, FanCategory, FanSpeedConfig};
pub use disk::DiskSource;
pub use clock::{ClockSource, ClockSourceConfig, TimeFormat, DateFormat, AlarmConfig, TimerConfig};
pub use crate::audio::AlarmSoundConfig;
pub use crate::core::{TimerMode, TimerState, TimerDisplayConfig};
pub use combo::{ComboSource, ComboSourceConfig, SlotConfig, GroupConfig};
pub use test::{TestSource, TestSourceConfig, TestMode, TEST_SOURCE_STATE};
pub use static_text::{StaticTextSource, StaticTextSourceConfig, StaticTextLine};
// pub use network::NetworkSource;

/// Initialize shared sensor caches (call once at startup)
pub fn initialize_sensors() {
    shared_sensors::initialize();
}

/// Register all built-in sources with the global registry
pub fn register_all() {
    use crate::core::global_registry;

    // General metric displayers available to most sources
    let general_displayers = &["text", "bar", "arc", "speedometer", "graph", "indicator"];

    // Register CPU source
    global_registry().register_source_with_info(
        "cpu",
        "Cpu",
        general_displayers,
        || Box::new(CpuSource::new()),
    );

    // Register GPU source
    global_registry().register_source_with_info(
        "gpu",
        "Gpu",
        general_displayers,
        || Box::new(GpuSource::new()),
    );

    // Register Memory source
    global_registry().register_source_with_info(
        "memory",
        "Memory",
        general_displayers,
        || Box::new(MemorySource::new()),
    );

    // Register System Temperature source
    global_registry().register_source_with_info(
        "system_temp",
        "System Temperature",
        general_displayers,
        || Box::new(SystemTempSource::new()),
    );

    // Register Fan Speed source
    global_registry().register_source_with_info(
        "fan_speed",
        "Fan Speed",
        general_displayers,
        || Box::new(FanSpeedSource::new()),
    );

    // Register Disk source
    global_registry().register_source_with_info(
        "disk",
        "Disk",
        general_displayers,
        || Box::new(DiskSource::new()),
    );

    // Register Clock source - only compatible with clock displayers
    global_registry().register_source_with_info(
        "clock",
        "Clock",
        &["clock_analog", "clock_digital"],
        || Box::new(ClockSource::new()),
    );

    // Register Combination source - compatible with LCARS, Cyberpunk, Material, Industrial and Retro Terminal displayers
    global_registry().register_source_with_info(
        "combination",
        "Combination",
        &["lcars", "cyberpunk", "material", "industrial", "retro_terminal"],
        || Box::new(ComboSource::new()),
    );

    // Register Test source - for debugging and demonstration
    global_registry().register_source_with_info(
        "test",
        "Test",
        general_displayers,
        || Box::new(TestSource::new()),
    );

    // Register Static Text source - for custom text overlays
    global_registry().register_source_with_info(
        "static_text",
        "Static Text",
        &["text"],  // Only compatible with text displayer
        || Box::new(StaticTextSource::new()),
    );

    // TODO: Register more sources
    // register_source!("network", NetworkSource);
}
