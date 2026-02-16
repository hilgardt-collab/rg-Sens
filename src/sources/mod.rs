//! Built-in data sources
//!
//! Re-exports from the rg-sens-sources crate, plus clock source
//! which depends on the main crate's timer_manager.

// Re-export everything from the sources crate
pub use rg_sens_sources::*;

// Clock source stays in main crate (depends on timer_manager)
mod clock;
pub use clock::{ClockSource, ClockSourceConfig, DateFormat, TimeFormat};

/// Register all built-in sources with the global registry
pub fn register_all() {
    // Register sources from the sources crate
    rg_sens_sources::register_all();

    // Register clock source (main crate only, depends on timer_manager)
    use crate::core::global_registry;
    global_registry().register_source_with_info(
        "clock",
        "Clock",
        &["clock_analog", "clock_digital"],
        || Box::new(ClockSource::new()),
    );
}
