//! Built-in displayers
//!
//! This module contains implementations of various visualization widgets.
//! Each displayer renders data in a specific visual format.

mod text;
mod text_config;
mod bar;
mod arc;
mod speedometer;
mod graph;
mod clock_analog;
mod clock_digital;
mod lcars_combo;
mod cpu_cores;
// mod level_bar;

pub use text::TextDisplayer;
pub use text_config::{HorizontalPosition, TextDisplayerConfig, TextLineConfig, VerticalPosition};
pub use bar::BarDisplayer;
pub use arc::ArcDisplayer;
pub use speedometer::SpeedometerDisplayer;
pub use graph::GraphDisplayer;
pub use clock_analog::ClockAnalogDisplayer;
pub use clock_digital::{ClockDigitalDisplayer, DigitalClockConfig, DigitalStyle};
pub use lcars_combo::{LcarsComboDisplayer, LcarsDisplayConfig};
pub use cpu_cores::CpuCoresDisplayer;

// Re-export FieldMetadata from core for convenience
pub use crate::core::FieldMetadata;
// pub use level_bar::LevelBarDisplayer;

/// Register all built-in displayers with the global registry
pub fn register_all() {
    use crate::core::global_registry;

    // Register text displayer
    global_registry().register_displayer_with_info(
        "text",
        "Text",
        || Box::new(TextDisplayer::new()),
    );

    // Register bar displayer
    global_registry().register_displayer_with_info(
        "bar",
        "Bar",
        || Box::new(BarDisplayer::new()),
    );

    // Register arc gauge displayer
    global_registry().register_displayer_with_info(
        "arc",
        "Arc Gauge",
        || Box::new(ArcDisplayer::new()),
    );

    // Register speedometer gauge displayer
    global_registry().register_displayer_with_info(
        "speedometer",
        "Speedometer",
        || Box::new(SpeedometerDisplayer::new()),
    );

    // Register graph displayer
    global_registry().register_displayer_with_info(
        "graph",
        "Graph",
        || Box::new(GraphDisplayer::new()),
    );

    // Register analog clock displayer
    global_registry().register_displayer_with_info(
        "clock_analog",
        "Analog Clock",
        || Box::new(ClockAnalogDisplayer::new()),
    );

    // Register digital clock displayer
    global_registry().register_displayer_with_info(
        "clock_digital",
        "Digital Clock",
        || Box::new(ClockDigitalDisplayer::new()),
    );

    // Register LCARS displayer (for Combination source)
    global_registry().register_displayer_with_info(
        "lcars",
        "LCARS",
        || Box::new(LcarsComboDisplayer::new()),
    );

    // Register CPU Cores displayer
    global_registry().register_displayer_with_info(
        "cpu_cores",
        "CPU Cores",
        || Box::new(CpuCoresDisplayer::new()),
    );

    // TODO: Register more displayers
    // register_displayer!("level_bar", LevelBarDisplayer);
}
