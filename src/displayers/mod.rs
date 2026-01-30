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

/// Apply global theme from config if present.
///
/// This is a common pattern across displayers that support theming.
/// Returns true if a theme was applied.
///
/// # Arguments
/// * `config` - The config HashMap to check for "global_theme" key
/// * `apply_fn` - Closure that receives the parsed theme and applies it
pub(crate) fn apply_global_theme<T, F>(config: &HashMap<String, Value>, apply_fn: F) -> bool
where
    T: serde::de::DeserializeOwned,
    F: FnOnce(T),
{
    if let Some(theme_value) = config.get("global_theme") {
        if let Ok(theme) = serde_json::from_value::<T>(theme_value.clone()) {
            apply_fn(theme);
            return true;
        }
    }
    false
}

/// Rebuild cached field IDs from text overlay config lines.
///
/// Call this when config changes to update the cached field IDs vector.
pub(crate) fn rebuild_cached_field_ids(
    lines: &[crate::displayers::text_config::TextLineConfig],
) -> Vec<String> {
    lines.iter().map(|l| l.field_id.clone()).collect()
}

// ============================================================================
// Theme Displayer Helpers
// ============================================================================

/// Default animation enabled value for theme displayers
pub fn default_animation_enabled() -> bool {
    true
}

/// Default animation speed for theme displayers
pub fn default_animation_speed() -> f64 {
    8.0
}

/// Macro to generate a theme display config wrapper.
///
/// This generates the boilerplate DisplayConfig struct that wraps a FrameConfig,
/// along with Default, from_frame(), and to_frame() implementations.
///
/// # Usage
/// ```ignore
/// theme_display_config!(
///     ArtDecoDisplayConfig,      // Config struct name
///     ArtDecoFrameConfig         // Frame config type from ui module
/// );
/// ```
#[macro_export]
macro_rules! theme_display_config {
    ($config_name:ident, $frame_config:ty) => {
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        pub struct $config_name {
            #[serde(default)]
            pub frame: $frame_config,
            #[serde(default = "crate::displayers::default_animation_enabled")]
            pub animation_enabled: bool,
            #[serde(default = "crate::displayers::default_animation_speed")]
            pub animation_speed: f64,
        }

        impl Default for $config_name {
            fn default() -> Self {
                Self {
                    frame: <$frame_config>::default(),
                    animation_enabled: $crate::displayers::default_animation_enabled(),
                    animation_speed: $crate::displayers::default_animation_speed(),
                }
            }
        }

        impl $config_name {
            /// Create config from frame config, syncing animation fields
            pub fn from_frame(frame: $frame_config) -> Self {
                Self {
                    animation_enabled: frame.animation_enabled,
                    animation_speed: frame.animation_speed,
                    frame,
                }
            }

            /// Convert to frame config, syncing animation fields from wrapper
            pub fn to_frame(&self) -> $frame_config {
                let mut frame = self.frame.clone();
                frame.animation_enabled = self.animation_enabled;
                frame.animation_speed = self.animation_speed;
                frame
            }
        }
    };
}

/// Macro to generate theme displayer struct and basic Displayer trait delegation.
///
/// This generates the displayer struct wrapping GenericComboDisplayerShared,
/// along with new(), Default implementations.
///
/// The config_schema() and apply_config() methods must be implemented manually
/// since they vary between themes.
///
/// # Usage
/// ```ignore
/// theme_displayer_base!(
///     ArtDecoDisplayer,          // Displayer struct name
///     ArtDecoRenderer,           // Renderer type from ui module
///     ArtDecoRenderer            // Renderer constructor expression
/// );
/// ```
#[macro_export]
macro_rules! theme_displayer_base {
    ($displayer_name:ident, $renderer_type:ty, $renderer_expr:expr) => {
        pub struct $displayer_name {
            inner: $crate::displayers::combo_generic::GenericComboDisplayerShared<$renderer_type>,
        }

        impl $displayer_name {
            pub fn new() -> Self {
                Self {
                    inner: $crate::displayers::combo_generic::GenericComboDisplayerShared::new(
                        $renderer_expr,
                    ),
                }
            }
        }

        impl Default for $displayer_name {
            fn default() -> Self {
                Self::new()
            }
        }
    };
}

mod arc;
mod art_deco;
mod art_nouveau;
mod bar;
mod clock_analog;
mod clock_digital;
pub mod combo_displayer_base;
pub mod combo_generic;
pub mod combo_utils;
mod cpu_cores;
// CSS Template backend modules
#[cfg(any(feature = "webkit", feature = "servo"))]
mod css_template;
#[cfg(any(feature = "webkit", feature = "servo"))]
mod css_template_backend;
mod cyberpunk;
mod fighter_hud;
mod graph;
mod indicator;
mod industrial;
mod lcars_combo;
mod material;
mod retro_terminal;
#[cfg(feature = "servo")]
pub mod servo_backend;
mod speedometer;
mod steampunk;
mod synthwave;
mod text;
mod text_config;
#[cfg(feature = "webkit")]
pub mod webkit_backend;
// mod level_bar;

pub use arc::ArcDisplayer;
pub use art_deco::{ArtDecoDisplayConfig, ArtDecoDisplayer};
pub use art_nouveau::{ArtNouveauDisplayConfig, ArtNouveauDisplayer};
pub use bar::BarDisplayer;
pub use clock_analog::ClockAnalogDisplayer;
pub use clock_digital::{ClockDigitalDisplayer, DigitalClockConfig, DigitalStyle};
pub use cpu_cores::CpuCoresDisplayer;
#[cfg(feature = "webkit")]
pub use css_template::shutdown_all as css_template_shutdown;
#[cfg(any(feature = "webkit", feature = "servo"))]
pub use css_template::CssTemplateDisplayer;
pub use cyberpunk::{CyberpunkDisplayConfig, CyberpunkDisplayer};
pub use fighter_hud::{FighterHudDisplayConfig, FighterHudDisplayer};
pub use graph::GraphDisplayer;
pub use indicator::{
    interpolate_gradient, render_indicator, IndicatorConfig, IndicatorDisplayer, IndicatorShape,
};
pub use industrial::{IndustrialDisplayConfig, IndustrialDisplayer};
pub use lcars_combo::{LcarsComboDisplayer, LcarsDisplayConfig};
pub use material::{MaterialDisplayConfig, MaterialDisplayer};
pub use retro_terminal::{RetroTerminalDisplayConfig, RetroTerminalDisplayer};
#[cfg(all(feature = "servo", not(feature = "webkit")))]
pub use servo_backend::shutdown_all as css_template_shutdown;
pub use speedometer::SpeedometerDisplayer;
pub use steampunk::{SteampunkDisplayConfig, SteampunkDisplayer};
pub use synthwave::{SynthwaveDisplayConfig, SynthwaveDisplayer};
pub use text::TextDisplayer;
pub use text_config::{
    CombineAlignment, CombineDirection, HorizontalPosition, TextBackgroundConfig,
    TextBackgroundType, TextDisplayerConfig, TextFillType, TextLineConfig, TextPosition,
    VerticalPosition,
};

// Re-export FieldMetadata from core for convenience
pub use crate::core::FieldMetadata;

// Re-export generic combo framework types
pub use combo_displayer_base::{ComboFrameConfig, FrameRenderer};
pub use combo_generic::{GenericComboDisplayer, GenericComboDisplayerShared};
// pub use level_bar::LevelBarDisplayer;

/// Register all built-in displayers with the global registry
///
/// # Arguments
/// * `webkit_enabled` - If true, register the CSS Template displayer with WebKit backend (requires --webkit-enable flag)
/// * `servo_enabled` - If true, register the CSS Template displayer with Servo backend (requires --servo-enable flag)
///
/// Note: webkit_enabled and servo_enabled are mutually exclusive (enforced by CLI argument parsing)
pub fn register_all(webkit_enabled: bool, servo_enabled: bool) {
    use crate::core::global_registry;

    // Register text displayer
    global_registry()
        .register_displayer_with_info("text", "Text", || Box::new(TextDisplayer::new()));

    // Register bar displayer
    global_registry().register_displayer_with_info("bar", "Bar", || Box::new(BarDisplayer::new()));

    // Register arc gauge displayer
    global_registry()
        .register_displayer_with_info("arc", "Arc Gauge", || Box::new(ArcDisplayer::new()));

    // Register speedometer gauge displayer
    global_registry().register_displayer_with_info("speedometer", "Speedometer", || {
        Box::new(SpeedometerDisplayer::new())
    });

    // Register graph displayer
    global_registry()
        .register_displayer_with_info("graph", "Graph", || Box::new(GraphDisplayer::new()));

    // Register analog clock displayer
    global_registry().register_displayer_with_info("clock_analog", "Analog Clock", || {
        Box::new(ClockAnalogDisplayer::new())
    });

    // Register digital clock displayer
    global_registry().register_displayer_with_info("clock_digital", "Digital Clock", || {
        Box::new(ClockDigitalDisplayer::new())
    });

    // Register LCARS displayer (for Combination source)
    global_registry()
        .register_displayer_with_info("lcars", "LCARS", || Box::new(LcarsComboDisplayer::new()));

    // Register CPU Cores displayer
    global_registry().register_displayer_with_info("cpu_cores", "CPU Cores", || {
        Box::new(CpuCoresDisplayer::new())
    });

    // Register Indicator displayer
    global_registry().register_displayer_with_info("indicator", "Indicator", || {
        Box::new(IndicatorDisplayer::new())
    });

    // Register Cyberpunk HUD displayer
    global_registry().register_displayer_with_info("cyberpunk", "Cyberpunk HUD", || {
        Box::new(CyberpunkDisplayer::new())
    });

    // Register Material Cards displayer
    global_registry().register_displayer_with_info("material", "Material Cards", || {
        Box::new(MaterialDisplayer::new())
    });

    // Register Industrial/Gauge Panel displayer
    global_registry().register_displayer_with_info("industrial", "Industrial Gauge", || {
        Box::new(IndustrialDisplayer::new())
    });

    // Register Retro Terminal CRT displayer
    global_registry().register_displayer_with_info("retro_terminal", "Retro Terminal", || {
        Box::new(RetroTerminalDisplayer::new())
    });

    // Register Fighter Jet HUD displayer
    global_registry().register_displayer_with_info("fighter_hud", "Fighter HUD", || {
        Box::new(FighterHudDisplayer::new())
    });

    // Register Synthwave/Outrun displayer
    global_registry().register_displayer_with_info("synthwave", "Synthwave", || {
        Box::new(SynthwaveDisplayer::new())
    });

    // Register Art Deco displayer
    global_registry()
        .register_displayer_with_info("art_deco", "Art Deco", || Box::new(ArtDecoDisplayer::new()));

    // Register Art Nouveau displayer
    global_registry().register_displayer_with_info("art_nouveau", "Art Nouveau", || {
        Box::new(ArtNouveauDisplayer::new())
    });

    // Register Steampunk displayer
    global_registry().register_displayer_with_info("steampunk", "Steampunk", || {
        Box::new(SteampunkDisplayer::new())
    });

    // Register CSS Template displayer based on enabled backend
    // WebKit backend (--webkit-enable)
    #[cfg(feature = "webkit")]
    if webkit_enabled {
        global_registry().register_displayer_with_info("css_template", "CSS Template", || {
            Box::new(CssTemplateDisplayer::with_webkit_backend())
        });
        log::info!("CSS Template displayer enabled with WebKit backend (--webkit-enable)");
    }

    // Servo backend (--servo-enable)
    #[cfg(feature = "servo")]
    if servo_enabled {
        global_registry().register_displayer_with_info("css_template", "CSS Template", || {
            Box::new(CssTemplateDisplayer::with_servo_backend())
        });
        log::info!("CSS Template displayer enabled with Servo backend (--servo-enable)");
    }

    // Suppress unused variable warnings when features are not enabled
    #[cfg(not(feature = "webkit"))]
    let _ = webkit_enabled;
    #[cfg(not(feature = "servo"))]
    let _ = servo_enabled;
}
