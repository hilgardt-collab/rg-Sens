//! Built-in displayers
//!
//! This module contains implementations of various visualization widgets.
//! Each displayer renders data in a specific visual format.

mod text;
mod text_config;
mod bar;
// mod level_bar;
// mod arc_gauge;
// mod line_graph;
// mod analog_clock;

pub use text::TextDisplayer;
pub use text_config::{HorizontalPosition, TextDisplayerConfig, TextLineConfig, VerticalPosition};
pub use bar::BarDisplayer;
// pub use level_bar::LevelBarDisplayer;
// pub use arc_gauge::ArcGaugeDisplayer;
// pub use line_graph::LineGraphDisplayer;
// pub use analog_clock::AnalogClockDisplayer;

/// Register all built-in displayers with the global registry
pub fn register_all() {
    use crate::core::global_registry;

    // Register text displayer
    global_registry().register_displayer("text", || Box::new(TextDisplayer::new()));

    // Register bar displayer
    global_registry().register_displayer("bar", || Box::new(BarDisplayer::new()));

    // TODO: Register more displayers
    // register_displayer!("level_bar", LevelBarDisplayer);
    // register_displayer!("arc_gauge", ArcGaugeDisplayer);
    // register_displayer!("line_graph", LineGraphDisplayer);
    // register_displayer!("analog_clock", AnalogClockDisplayer);
}
