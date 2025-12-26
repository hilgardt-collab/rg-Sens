//! Built-in displayers
//!
//! This module contains implementations of various visualization widgets.
//! Each displayer renders data in a specific visual format.

use serde_json::Value;
use std::collections::HashMap;

/// Extract only the values needed for text overlay rendering.
///
/// Instead of cloning the entire HashMap (which can have 50+ entries for CPU sources),
/// this extracts only the field_ids used in the TextDisplayerConfig.
/// Returns a smaller HashMap with just the needed values.
pub(crate) fn extract_text_values(
    data: &HashMap<String, Value>,
    text_config: &TextDisplayerConfig,
) -> HashMap<String, Value> {
    let mut result = HashMap::with_capacity(text_config.lines.len());
    for line in &text_config.lines {
        if let Some(value) = data.get(&line.field_id) {
            result.insert(line.field_id.clone(), value.clone());
        }
    }
    result
}

/// Extract a numeric value from data and normalize it to 0.0-1.0 range.
///
/// This helper looks for common keys like "value", "percent", "usage", "level"
/// and normalizes the value using min/max limits if available, or heuristics
/// based on the value range.
///
/// Used by bar, arc, and other gauge-style displayers.
pub(crate) fn extract_normalized_value(data: &HashMap<String, Value>) -> f64 {
    // Try to find a numeric value from common keys
    let raw_value = data
        .get("value")
        .or_else(|| data.get("percent"))
        .or_else(|| data.get("usage"))
        .or_else(|| data.get("level"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    // Get min/max limits from data source if available
    let min_limit = data.get("min_limit").and_then(|v| v.as_f64());
    let max_limit = data.get("max_limit").and_then(|v| v.as_f64());

    // Normalize to 0.0-1.0 range
    let normalized = if let (Some(min), Some(max)) = (min_limit, max_limit) {
        // Use min/max range if available
        if max > min {
            (raw_value - min) / (max - min)
        } else {
            0.0
        }
    } else if raw_value <= 1.0 {
        // Value already in 0-1 range
        raw_value
    } else if raw_value <= 100.0 {
        // Assume percentage (0-100)
        raw_value / 100.0
    } else {
        // For values > 100 without explicit range, can't normalize
        0.0
    };

    normalized.clamp(0.0, 1.0)
}

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
mod indicator;
mod cyberpunk;
mod material;
mod industrial;
mod retro_terminal;
mod fighter_hud;
mod synthwave;
pub mod combo_utils;
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
pub use indicator::{IndicatorDisplayer, IndicatorConfig, IndicatorShape, interpolate_gradient, render_indicator};
pub use cyberpunk::{CyberpunkDisplayer, CyberpunkDisplayConfig};
pub use material::{MaterialDisplayer, MaterialDisplayConfig};
pub use industrial::{IndustrialDisplayer, IndustrialDisplayConfig};
pub use retro_terminal::{RetroTerminalDisplayer, RetroTerminalDisplayConfig};
pub use fighter_hud::{FighterHudDisplayer, FighterHudDisplayConfig};
pub use synthwave::{SynthwaveDisplayer, SynthwaveDisplayConfig};

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

    // Register Indicator displayer
    global_registry().register_displayer_with_info(
        "indicator",
        "Indicator",
        || Box::new(IndicatorDisplayer::new()),
    );

    // Register Cyberpunk HUD displayer
    global_registry().register_displayer_with_info(
        "cyberpunk",
        "Cyberpunk HUD",
        || Box::new(CyberpunkDisplayer::new()),
    );

    // Register Material Cards displayer
    global_registry().register_displayer_with_info(
        "material",
        "Material Cards",
        || Box::new(MaterialDisplayer::new()),
    );

    // Register Industrial/Gauge Panel displayer
    global_registry().register_displayer_with_info(
        "industrial",
        "Industrial Gauge",
        || Box::new(IndustrialDisplayer::new()),
    );

    // Register Retro Terminal CRT displayer
    global_registry().register_displayer_with_info(
        "retro_terminal",
        "Retro Terminal",
        || Box::new(RetroTerminalDisplayer::new()),
    );

    // Register Fighter Jet HUD displayer
    global_registry().register_displayer_with_info(
        "fighter_hud",
        "Fighter HUD",
        || Box::new(FighterHudDisplayer::new()),
    );

    // Register Synthwave/Outrun displayer
    global_registry().register_displayer_with_info(
        "synthwave",
        "Synthwave",
        || Box::new(SynthwaveDisplayer::new()),
    );

    // TODO: Register more displayers
    // register_displayer!("level_bar", LevelBarDisplayer);
}
