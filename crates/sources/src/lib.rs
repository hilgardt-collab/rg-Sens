//! rg-sens-sources: Data source implementations for rg-Sens system monitoring.

mod combo;
mod cpu;
mod disk;
mod fan_speed;
mod gpu;
mod memory;
mod network;
mod shared_sensors;
mod static_text;
mod system_temp;
mod test;

pub use rg_sens_types::timer::{AlarmConfig, AlarmSoundConfig, TimerConfig, TimerDisplayConfig, TimerMode, TimerState};
pub use combo::{ComboSource, ComboSourceConfig, GroupConfig, SlotConfig};
pub use cpu::{CpuSensor, CpuSource};
pub use disk::DiskSource;
pub use fan_speed::{FanCategory, FanInfo, FanSpeedConfig, FanSpeedSource};
pub use gpu::GpuSource;
pub use memory::MemorySource;
pub use network::NetworkSource;
pub use static_text::{StaticTextLine, StaticTextSource, StaticTextSourceConfig};
pub use system_temp::{
    resolve_sensor_index, set_sensor_by_index, SensorCategory, SensorInfo, SystemTempConfig,
    SystemTempSource, TemperatureUnit as SystemTempUnit,
};
pub use test::{TestMode, TestSource, TestSourceConfig, TEST_SOURCE_STATE};

/// Initialize shared sensor caches (call once at startup)
pub fn initialize_sensors() {
    shared_sensors::initialize();
}

/// Register all built-in sources with the global registry
pub fn register_all() {
    use rg_sens_core::global_registry;

    // General metric displayers available to most sources
    let general_displayers = &["text", "bar", "arc", "speedometer", "graph", "indicator"];

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

    global_registry().register_source_with_info("gpu", "Gpu", general_displayers, || {
        Box::new(GpuSource::new())
    });

    global_registry().register_source_with_info("memory", "Memory", general_displayers, || {
        Box::new(MemorySource::new())
    });

    global_registry().register_source_with_info(
        "system_temp",
        "System Temperature",
        general_displayers,
        || Box::new(SystemTempSource::new()),
    );

    global_registry().register_source_with_info(
        "fan_speed",
        "Fan Speed",
        general_displayers,
        || Box::new(FanSpeedSource::new()),
    );

    global_registry().register_source_with_info("disk", "Disk", general_displayers, || {
        Box::new(DiskSource::new())
    });

    global_registry().register_source_with_info("network", "Network", general_displayers, || {
        Box::new(NetworkSource::new())
    });

    // Note: Clock source is registered by the main crate (depends on timer_manager)

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

    global_registry().register_source_with_info("test", "Test", general_displayers, || {
        Box::new(TestSource::new())
    });

    global_registry().register_source_with_info(
        "static_text",
        "Static Text",
        &["text"],
        || Box::new(StaticTextSource::new()),
    );
}
