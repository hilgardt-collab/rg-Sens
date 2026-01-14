//! Built-in data sources
//!
//! This module contains implementations of various system metric sources.
//! Each source collects specific system information (CPU, memory, GPU, etc.)

mod clock;
mod combo;
mod cpu;
mod disk;
mod fan_speed;
mod gpu;
mod memory;
mod shared_sensors;
mod static_text;
mod system_temp;
mod test;
mod network;

pub use crate::audio::AlarmSoundConfig;
pub use crate::core::{TimerDisplayConfig, TimerMode, TimerState};
pub use clock::{AlarmConfig, ClockSource, ClockSourceConfig, DateFormat, TimeFormat, TimerConfig};
pub use combo::{ComboSource, ComboSourceConfig, GroupConfig, SlotConfig};
pub use cpu::{CpuSensor, CpuSource};
pub use disk::DiskSource;
pub use fan_speed::{FanCategory, FanInfo, FanSpeedConfig, FanSpeedSource};
pub use gpu::GpuSource;
pub use memory::MemorySource;
pub use static_text::{StaticTextLine, StaticTextSource, StaticTextSourceConfig};
pub use system_temp::{
    SensorCategory, SensorInfo, SystemTempConfig, SystemTempSource,
    TemperatureUnit as SystemTempUnit,
};
pub use test::{TestMode, TestSource, TestSourceConfig, TEST_SOURCE_STATE};
pub use network::NetworkSource;

/// Initialize shared sensor caches (call once at startup)
pub fn initialize_sensors() {
    shared_sensors::initialize();
}

/// Register all built-in sources with the global registry
pub fn register_all() {
    use crate::core::global_registry;

    // General metric displayers available to most sources
    let general_displayers = &["text", "bar", "arc", "speedometer", "graph", "indicator"];

    // Register CPU source - includes cpu_cores for per-core visualization
    global_registry().register_source_with_info(
        "cpu",
        "Cpu",
        &[
            "text",
            "bar",
            "arc",
            "speedometer",
            "graph",
            "indicator",
            "cpu_cores",
        ],
        || Box::new(CpuSource::new()),
    );

    // Register GPU source
    global_registry().register_source_with_info("gpu", "Gpu", general_displayers, || {
        Box::new(GpuSource::new())
    });

    // Register Memory source
    global_registry().register_source_with_info("memory", "Memory", general_displayers, || {
        Box::new(MemorySource::new())
    });

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
    global_registry().register_source_with_info("disk", "Disk", general_displayers, || {
        Box::new(DiskSource::new())
    });

    // Register Network source
    global_registry().register_source_with_info("network", "Network", general_displayers, || {
        Box::new(NetworkSource::new())
    });

    // Register Clock source - only compatible with clock displayers
    global_registry().register_source_with_info(
        "clock",
        "Clock",
        &["clock_analog", "clock_digital"],
        || Box::new(ClockSource::new()),
    );

    // Register Combination source - compatible with LCARS, Cyberpunk, Material, Industrial, Retro Terminal, Fighter HUD, Synthwave, Art Deco, Art Nouveau, Steampunk, and CSS Template displayers
    global_registry().register_source_with_info(
        "combination",
        "Combination",
        &[
            "lcars",
            "cyberpunk",
            "material",
            "industrial",
            "retro_terminal",
            "fighter_hud",
            "synthwave",
            "art_deco",
            "art_nouveau",
            "steampunk",
            "css_template",
        ],
        || Box::new(ComboSource::new()),
    );

    // Register Test source - for debugging and demonstration
    global_registry().register_source_with_info("test", "Test", general_displayers, || {
        Box::new(TestSource::new())
    });

    // Register Static Text source - for custom text overlays
    global_registry().register_source_with_info(
        "static_text",
        "Static Text",
        &["text"], // Only compatible with text displayer
        || Box::new(StaticTextSource::new()),
    );
}
