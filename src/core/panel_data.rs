//! Unified panel data structure - single source of truth for panel configuration.
//!
//! This module defines the `PanelData` struct which contains ALL configuration
//! for a panel in one place. This replaces the fragmented approach where config
//! was spread across Panel.config HashMap, source internals, and displayer internals.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::core::PanelBorderConfig;
use crate::core::PanelGeometry;
use crate::ui::BackgroundConfig;

// Re-export source configs
pub use crate::sources::ClockSourceConfig;
pub use crate::sources::ComboSourceConfig;
pub use crate::sources::FanSpeedConfig;
pub use crate::sources::StaticTextSourceConfig;
pub use crate::sources::SystemTempConfig;
pub use crate::sources::TestSourceConfig;
pub use crate::ui::CpuSourceConfig;
pub use crate::ui::DiskSourceConfig;
pub use crate::ui::GpuSourceConfig;
pub use crate::ui::MemorySourceConfig;
pub use crate::ui::NetworkSourceConfig;

// Re-export displayer configs
pub use crate::displayers::ArtDecoDisplayConfig;
pub use crate::displayers::ArtNouveauDisplayConfig;
pub use crate::displayers::CyberpunkDisplayConfig;
pub use crate::displayers::DigitalClockConfig;
pub use crate::displayers::FighterHudDisplayConfig;
pub use crate::displayers::IndicatorConfig;
pub use crate::displayers::IndustrialDisplayConfig;
pub use crate::displayers::LcarsDisplayConfig;
pub use crate::displayers::MaterialDisplayConfig;
pub use crate::displayers::RetroTerminalDisplayConfig;
pub use crate::displayers::SteampunkDisplayConfig;
pub use crate::displayers::SynthwaveDisplayConfig;
pub use crate::displayers::TextDisplayerConfig;
pub use crate::ui::css_template_display::CssTemplateDisplayConfig;
pub use crate::ui::AnalogClockConfig;
pub use crate::ui::ArcDisplayConfig;
pub use crate::ui::BarDisplayConfig;
pub use crate::ui::CoreBarsConfig;
pub use crate::ui::GraphDisplayConfig;
pub use crate::ui::SpeedometerConfig;

/// Type-safe enum for all source configurations.
/// Uses serde tag for JSON serialization: {"type": "cpu", ...}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "source_type")]
pub enum SourceConfig {
    #[serde(rename = "cpu")]
    Cpu(CpuSourceConfig),

    #[serde(rename = "gpu")]
    Gpu(GpuSourceConfig),

    #[serde(rename = "memory")]
    Memory(MemorySourceConfig),

    #[serde(rename = "disk")]
    Disk(DiskSourceConfig),

    #[serde(rename = "network")]
    Network(NetworkSourceConfig),

    #[serde(rename = "clock")]
    Clock(ClockSourceConfig),

    #[serde(rename = "combination")]
    Combo(ComboSourceConfig),

    #[serde(rename = "system_temp")]
    SystemTemp(SystemTempConfig),

    #[serde(rename = "fan_speed")]
    FanSpeed(FanSpeedConfig),

    #[serde(rename = "test")]
    Test(TestSourceConfig),

    #[serde(rename = "static_text")]
    StaticText(StaticTextSourceConfig),
}

impl SourceConfig {
    /// Get the source type ID string
    pub fn source_type(&self) -> &'static str {
        match self {
            SourceConfig::Cpu(_) => "cpu",
            SourceConfig::Gpu(_) => "gpu",
            SourceConfig::Memory(_) => "memory",
            SourceConfig::Disk(_) => "disk",
            SourceConfig::Network(_) => "network",
            SourceConfig::Clock(_) => "clock",
            SourceConfig::Combo(_) => "combination",
            SourceConfig::SystemTemp(_) => "system_temp",
            SourceConfig::FanSpeed(_) => "fan_speed",
            SourceConfig::Test(_) => "test",
            SourceConfig::StaticText(_) => "static_text",
        }
    }

    /// Get the update interval in milliseconds from this source config
    pub fn update_interval_ms(&self) -> u64 {
        match self {
            SourceConfig::Cpu(cfg) => cfg.update_interval_ms,
            SourceConfig::Gpu(cfg) => cfg.update_interval_ms,
            SourceConfig::Memory(cfg) => cfg.update_interval_ms,
            SourceConfig::Disk(cfg) => cfg.update_interval_ms,
            SourceConfig::Network(cfg) => cfg.update_interval_ms,
            SourceConfig::Clock(cfg) => cfg.update_interval_ms,
            SourceConfig::Combo(cfg) => cfg.update_interval_ms,
            SourceConfig::SystemTemp(cfg) => cfg.update_interval_ms,
            SourceConfig::FanSpeed(cfg) => cfg.update_interval_ms,
            SourceConfig::Test(cfg) => cfg.update_interval_ms,
            SourceConfig::StaticText(cfg) => cfg.update_interval_ms,
        }
    }

    /// Convert to HashMap<String, Value> for legacy interface compatibility
    pub fn to_hashmap(&self) -> HashMap<String, serde_json::Value> {
        let mut map = HashMap::new();
        match self {
            SourceConfig::Cpu(cfg) => {
                if let Ok(val) = serde_json::to_value(cfg) {
                    map.insert("cpu_config".to_string(), val);
                }
            }
            SourceConfig::Gpu(cfg) => {
                if let Ok(val) = serde_json::to_value(cfg) {
                    map.insert("gpu_config".to_string(), val);
                }
            }
            SourceConfig::Memory(cfg) => {
                if let Ok(val) = serde_json::to_value(cfg) {
                    map.insert("memory_config".to_string(), val);
                }
            }
            SourceConfig::Disk(cfg) => {
                if let Ok(val) = serde_json::to_value(cfg) {
                    map.insert("disk_config".to_string(), val);
                }
            }
            SourceConfig::Network(cfg) => {
                if let Ok(val) = serde_json::to_value(cfg) {
                    map.insert("network_config".to_string(), val);
                }
            }
            SourceConfig::Clock(cfg) => {
                if let Ok(val) = serde_json::to_value(cfg) {
                    map.insert("clock_config".to_string(), val);
                }
            }
            SourceConfig::Combo(cfg) => {
                if let Ok(val) = serde_json::to_value(cfg) {
                    map.insert("combo_config".to_string(), val);
                }
            }
            SourceConfig::SystemTemp(cfg) => {
                if let Ok(val) = serde_json::to_value(cfg) {
                    map.insert("system_temp_config".to_string(), val);
                }
            }
            SourceConfig::FanSpeed(cfg) => {
                if let Ok(val) = serde_json::to_value(cfg) {
                    map.insert("fan_speed_config".to_string(), val);
                }
            }
            SourceConfig::Test(cfg) => {
                if let Ok(val) = serde_json::to_value(cfg) {
                    map.insert("test_config".to_string(), val);
                }
            }
            SourceConfig::StaticText(cfg) => {
                if let Ok(val) = serde_json::to_value(cfg) {
                    map.insert("static_text_config".to_string(), val);
                }
            }
        }
        map
    }

    /// Create a default SourceConfig for a given source type ID
    pub fn default_for_type(source_type: &str) -> Option<Self> {
        match source_type {
            "cpu" => Some(SourceConfig::Cpu(CpuSourceConfig::default())),
            "gpu" => Some(SourceConfig::Gpu(GpuSourceConfig::default())),
            "memory" => Some(SourceConfig::Memory(MemorySourceConfig::default())),
            "disk" => Some(SourceConfig::Disk(DiskSourceConfig::default())),
            "network" => Some(SourceConfig::Network(NetworkSourceConfig::default())),
            "clock" => Some(SourceConfig::Clock(ClockSourceConfig::default())),
            "combination" => Some(SourceConfig::Combo(ComboSourceConfig::default())),
            "system_temp" => Some(SourceConfig::SystemTemp(SystemTempConfig::default())),
            "fan_speed" => Some(SourceConfig::FanSpeed(FanSpeedConfig::default())),
            "test" => Some(SourceConfig::Test(TestSourceConfig::default())),
            "static_text" => Some(SourceConfig::StaticText(StaticTextSourceConfig::default())),
            _ => None,
        }
    }

    /// Extract a typed SourceConfig from a legacy HashMap configuration
    ///
    /// This looks for the appropriate config key based on source type
    /// (e.g., "cpu_config" for cpu source) and deserializes it.
    pub fn extract_from_hashmap(
        config: &HashMap<String, serde_json::Value>,
        source_type: &str,
    ) -> Option<Self> {
        let config_key = match source_type {
            "cpu" => "cpu_config",
            "gpu" => "gpu_config",
            "memory" => "memory_config",
            "disk" => "disk_config",
            "clock" => "clock_config",
            "combination" => "combo_config",
            "system_temp" => "system_temp_config",
            "fan_speed" => "fan_speed_config",
            "test" => "test_config",
            "static_text" => "static_text_config",
            _ => return None,
        };

        config.get(config_key).and_then(|value| match source_type {
            "cpu" => serde_json::from_value::<CpuSourceConfig>(value.clone())
                .ok()
                .map(SourceConfig::Cpu),
            "gpu" => serde_json::from_value::<GpuSourceConfig>(value.clone())
                .ok()
                .map(SourceConfig::Gpu),
            "memory" => serde_json::from_value::<MemorySourceConfig>(value.clone())
                .ok()
                .map(SourceConfig::Memory),
            "disk" => serde_json::from_value::<DiskSourceConfig>(value.clone())
                .ok()
                .map(SourceConfig::Disk),
            "clock" => serde_json::from_value::<ClockSourceConfig>(value.clone())
                .ok()
                .map(SourceConfig::Clock),
            "combination" => serde_json::from_value::<ComboSourceConfig>(value.clone())
                .ok()
                .map(SourceConfig::Combo),
            "system_temp" => serde_json::from_value::<SystemTempConfig>(value.clone())
                .ok()
                .map(SourceConfig::SystemTemp),
            "fan_speed" => serde_json::from_value::<FanSpeedConfig>(value.clone())
                .ok()
                .map(SourceConfig::FanSpeed),
            "test" => serde_json::from_value::<TestSourceConfig>(value.clone())
                .ok()
                .map(SourceConfig::Test),
            "static_text" => serde_json::from_value::<StaticTextSourceConfig>(value.clone())
                .ok()
                .map(SourceConfig::StaticText),
            _ => None,
        })
    }
}

impl Default for SourceConfig {
    fn default() -> Self {
        SourceConfig::Cpu(CpuSourceConfig::default())
    }
}

/// Type-safe enum for all displayer configurations.
/// Uses serde tag for JSON serialization: {"type": "bar", ...}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "displayer_type")]
pub enum DisplayerConfig {
    #[serde(rename = "text")]
    Text(TextDisplayerConfig),

    #[serde(rename = "bar")]
    Bar(BarDisplayConfig),

    #[serde(rename = "arc")]
    Arc(ArcDisplayConfig),

    #[serde(rename = "speedometer")]
    Speedometer(SpeedometerConfig),

    #[serde(rename = "graph")]
    Graph(GraphDisplayConfig),

    #[serde(rename = "clock_analog")]
    ClockAnalog(AnalogClockConfig),

    #[serde(rename = "clock_digital")]
    ClockDigital(DigitalClockConfig),

    #[serde(rename = "lcars")]
    Lcars(LcarsDisplayConfig),

    #[serde(rename = "cpu_cores")]
    CpuCores(CoreBarsConfig),

    #[serde(rename = "indicator")]
    Indicator(IndicatorConfig),

    #[serde(rename = "cyberpunk")]
    Cyberpunk(CyberpunkDisplayConfig),

    #[serde(rename = "material")]
    Material(MaterialDisplayConfig),

    #[serde(rename = "industrial")]
    Industrial(IndustrialDisplayConfig),

    #[serde(rename = "retro_terminal")]
    RetroTerminal(RetroTerminalDisplayConfig),

    #[serde(rename = "fighter_hud")]
    FighterHud(FighterHudDisplayConfig),

    #[serde(rename = "synthwave")]
    Synthwave(SynthwaveDisplayConfig),

    #[serde(rename = "art_deco")]
    ArtDeco(ArtDecoDisplayConfig),

    #[serde(rename = "art_nouveau")]
    ArtNouveau(ArtNouveauDisplayConfig),

    #[serde(rename = "steampunk")]
    Steampunk(SteampunkDisplayConfig),

    #[serde(rename = "css_template")]
    CssTemplate(CssTemplateDisplayConfig),
}

impl DisplayerConfig {
    /// Get the displayer type ID string
    pub fn displayer_type(&self) -> &'static str {
        match self {
            DisplayerConfig::Text(_) => "text",
            DisplayerConfig::Bar(_) => "bar",
            DisplayerConfig::Arc(_) => "arc",
            DisplayerConfig::Speedometer(_) => "speedometer",
            DisplayerConfig::Graph(_) => "graph",
            DisplayerConfig::ClockAnalog(_) => "clock_analog",
            DisplayerConfig::ClockDigital(_) => "clock_digital",
            DisplayerConfig::Lcars(_) => "lcars",
            DisplayerConfig::CpuCores(_) => "cpu_cores",
            DisplayerConfig::Indicator(_) => "indicator",
            DisplayerConfig::Cyberpunk(_) => "cyberpunk",
            DisplayerConfig::Material(_) => "material",
            DisplayerConfig::Industrial(_) => "industrial",
            DisplayerConfig::RetroTerminal(_) => "retro_terminal",
            DisplayerConfig::FighterHud(_) => "fighter_hud",
            DisplayerConfig::Synthwave(_) => "synthwave",
            DisplayerConfig::ArtDeco(_) => "art_deco",
            DisplayerConfig::ArtNouveau(_) => "art_nouveau",
            DisplayerConfig::Steampunk(_) => "steampunk",
            DisplayerConfig::CssTemplate(_) => "css_template",
        }
    }

    /// Convert to HashMap<String, Value> for legacy interface compatibility
    pub fn to_hashmap(&self) -> HashMap<String, serde_json::Value> {
        let mut map = HashMap::new();
        match self {
            DisplayerConfig::Text(cfg) => {
                if let Ok(val) = serde_json::to_value(cfg) {
                    map.insert("text_config".to_string(), val);
                }
            }
            DisplayerConfig::Bar(cfg) => {
                if let Ok(val) = serde_json::to_value(cfg) {
                    map.insert("bar_config".to_string(), val);
                }
            }
            DisplayerConfig::Arc(cfg) => {
                if let Ok(val) = serde_json::to_value(cfg) {
                    map.insert("arc_config".to_string(), val);
                }
            }
            DisplayerConfig::Speedometer(cfg) => {
                if let Ok(val) = serde_json::to_value(cfg) {
                    map.insert("speedometer_config".to_string(), val);
                }
            }
            DisplayerConfig::Graph(cfg) => {
                if let Ok(val) = serde_json::to_value(cfg) {
                    map.insert("graph_config".to_string(), val);
                }
            }
            DisplayerConfig::ClockAnalog(cfg) => {
                if let Ok(val) = serde_json::to_value(cfg) {
                    map.insert("clock_analog_config".to_string(), val);
                }
            }
            DisplayerConfig::ClockDigital(cfg) => {
                if let Ok(val) = serde_json::to_value(cfg) {
                    map.insert("clock_digital_config".to_string(), val);
                }
            }
            DisplayerConfig::Lcars(cfg) => {
                if let Ok(val) = serde_json::to_value(cfg) {
                    map.insert("lcars_config".to_string(), val);
                }
            }
            DisplayerConfig::CpuCores(cfg) => {
                if let Ok(val) = serde_json::to_value(cfg) {
                    map.insert("core_bars_config".to_string(), val);
                }
            }
            DisplayerConfig::Indicator(cfg) => {
                if let Ok(val) = serde_json::to_value(cfg) {
                    map.insert("indicator_config".to_string(), val);
                }
            }
            DisplayerConfig::Cyberpunk(cfg) => {
                if let Ok(val) = serde_json::to_value(cfg) {
                    map.insert("cyberpunk_config".to_string(), val);
                }
            }
            DisplayerConfig::Material(cfg) => {
                if let Ok(val) = serde_json::to_value(cfg) {
                    map.insert("material_config".to_string(), val);
                }
            }
            DisplayerConfig::Industrial(cfg) => {
                if let Ok(val) = serde_json::to_value(cfg) {
                    map.insert("industrial_config".to_string(), val);
                }
            }
            DisplayerConfig::RetroTerminal(cfg) => {
                if let Ok(val) = serde_json::to_value(cfg) {
                    map.insert("retro_terminal_config".to_string(), val);
                }
            }
            DisplayerConfig::FighterHud(cfg) => {
                if let Ok(val) = serde_json::to_value(cfg) {
                    map.insert("fighter_hud_config".to_string(), val);
                }
            }
            DisplayerConfig::Synthwave(cfg) => {
                if let Ok(val) = serde_json::to_value(cfg) {
                    map.insert("synthwave_config".to_string(), val);
                }
            }
            DisplayerConfig::ArtDeco(cfg) => {
                if let Ok(val) = serde_json::to_value(cfg) {
                    map.insert("art_deco_config".to_string(), val);
                }
            }
            DisplayerConfig::ArtNouveau(cfg) => {
                if let Ok(val) = serde_json::to_value(cfg) {
                    map.insert("art_nouveau_config".to_string(), val);
                }
            }
            DisplayerConfig::Steampunk(cfg) => {
                if let Ok(val) = serde_json::to_value(cfg) {
                    map.insert("steampunk_config".to_string(), val);
                }
            }
            DisplayerConfig::CssTemplate(cfg) => {
                if let Ok(val) = serde_json::to_value(cfg) {
                    map.insert("css_template_config".to_string(), val);
                }
            }
        }
        map
    }

    /// Create a default DisplayerConfig for a given displayer type ID
    pub fn default_for_type(displayer_type: &str) -> Option<Self> {
        match displayer_type {
            "text" => Some(DisplayerConfig::Text(TextDisplayerConfig::default())),
            "bar" => Some(DisplayerConfig::Bar(BarDisplayConfig::default())),
            "arc" => Some(DisplayerConfig::Arc(ArcDisplayConfig::default())),
            "speedometer" => Some(DisplayerConfig::Speedometer(SpeedometerConfig::default())),
            "graph" => Some(DisplayerConfig::Graph(GraphDisplayConfig::default())),
            "clock_analog" => Some(DisplayerConfig::ClockAnalog(AnalogClockConfig::default())),
            "clock_digital" => Some(DisplayerConfig::ClockDigital(DigitalClockConfig::default())),
            "lcars" => Some(DisplayerConfig::Lcars(LcarsDisplayConfig::default())),
            "cpu_cores" => Some(DisplayerConfig::CpuCores(CoreBarsConfig::default())),
            "indicator" => Some(DisplayerConfig::Indicator(IndicatorConfig::default())),
            "cyberpunk" => Some(DisplayerConfig::Cyberpunk(CyberpunkDisplayConfig::default())),
            "material" => Some(DisplayerConfig::Material(MaterialDisplayConfig::default())),
            "industrial" => Some(DisplayerConfig::Industrial(
                IndustrialDisplayConfig::default(),
            )),
            "retro_terminal" => Some(DisplayerConfig::RetroTerminal(
                RetroTerminalDisplayConfig::default(),
            )),
            "fighter_hud" => Some(DisplayerConfig::FighterHud(
                FighterHudDisplayConfig::default(),
            )),
            "synthwave" => Some(DisplayerConfig::Synthwave(SynthwaveDisplayConfig::default())),
            "art_deco" => Some(DisplayerConfig::ArtDeco(ArtDecoDisplayConfig::default())),
            "art_nouveau" => Some(DisplayerConfig::ArtNouveau(
                ArtNouveauDisplayConfig::default(),
            )),
            "steampunk" => Some(DisplayerConfig::Steampunk(SteampunkDisplayConfig::default())),
            "css_template" => Some(DisplayerConfig::CssTemplate(
                CssTemplateDisplayConfig::default(),
            )),
            _ => None,
        }
    }

    /// Create a DisplayerConfig from a JSON value for a given displayer type ID
    /// This is used to restore saved default configurations
    /// Handles both wrapped format ({"lcars": {...}}) and direct format ({...})
    pub fn from_value_for_type(displayer_type: &str, value: serde_json::Value) -> Option<Self> {
        // First try to parse as the wrapped DisplayerConfig enum (handles old format)
        if let Ok(config) = serde_json::from_value::<DisplayerConfig>(value.clone()) {
            return Some(config);
        }

        // Fall back to parsing as the inner config type directly
        match displayer_type {
            "text" => serde_json::from_value(value)
                .ok()
                .map(DisplayerConfig::Text),
            "bar" => serde_json::from_value(value).ok().map(DisplayerConfig::Bar),
            "arc" => serde_json::from_value(value).ok().map(DisplayerConfig::Arc),
            "speedometer" => serde_json::from_value(value)
                .ok()
                .map(DisplayerConfig::Speedometer),
            "graph" => serde_json::from_value(value)
                .ok()
                .map(DisplayerConfig::Graph),
            "clock_analog" => serde_json::from_value(value)
                .ok()
                .map(DisplayerConfig::ClockAnalog),
            "clock_digital" => serde_json::from_value(value)
                .ok()
                .map(DisplayerConfig::ClockDigital),
            "lcars" | "lcars_combo" => serde_json::from_value(value)
                .ok()
                .map(DisplayerConfig::Lcars),
            "cpu_cores" => serde_json::from_value(value)
                .ok()
                .map(DisplayerConfig::CpuCores),
            "indicator" => serde_json::from_value(value)
                .ok()
                .map(DisplayerConfig::Indicator),
            "cyberpunk" => serde_json::from_value(value)
                .ok()
                .map(DisplayerConfig::Cyberpunk),
            "material" => serde_json::from_value(value)
                .ok()
                .map(DisplayerConfig::Material),
            "industrial" => serde_json::from_value(value)
                .ok()
                .map(DisplayerConfig::Industrial),
            "retro_terminal" => serde_json::from_value(value)
                .ok()
                .map(DisplayerConfig::RetroTerminal),
            "fighter_hud" => serde_json::from_value(value)
                .ok()
                .map(DisplayerConfig::FighterHud),
            "synthwave" => serde_json::from_value(value)
                .ok()
                .map(DisplayerConfig::Synthwave),
            "art_deco" => serde_json::from_value(value)
                .ok()
                .map(DisplayerConfig::ArtDeco),
            "art_nouveau" => serde_json::from_value(value)
                .ok()
                .map(DisplayerConfig::ArtNouveau),
            "steampunk" => serde_json::from_value(value)
                .ok()
                .map(DisplayerConfig::Steampunk),
            "css_template" => serde_json::from_value(value)
                .ok()
                .map(DisplayerConfig::CssTemplate),
            _ => None,
        }
    }

    /// Get the inner config as a JSON value (without the enum wrapper)
    /// This is used when saving defaults to ensure consistent format
    pub fn to_inner_value(&self) -> Option<serde_json::Value> {
        match self {
            DisplayerConfig::Text(c) => serde_json::to_value(c).ok(),
            DisplayerConfig::Bar(c) => serde_json::to_value(c).ok(),
            DisplayerConfig::Arc(c) => serde_json::to_value(c).ok(),
            DisplayerConfig::Speedometer(c) => serde_json::to_value(c).ok(),
            DisplayerConfig::Graph(c) => serde_json::to_value(c).ok(),
            DisplayerConfig::ClockAnalog(c) => serde_json::to_value(c).ok(),
            DisplayerConfig::ClockDigital(c) => serde_json::to_value(c).ok(),
            DisplayerConfig::Lcars(c) => serde_json::to_value(c).ok(),
            DisplayerConfig::CpuCores(c) => serde_json::to_value(c).ok(),
            DisplayerConfig::Indicator(c) => serde_json::to_value(c).ok(),
            DisplayerConfig::Cyberpunk(c) => serde_json::to_value(c).ok(),
            DisplayerConfig::Material(c) => serde_json::to_value(c).ok(),
            DisplayerConfig::Industrial(c) => serde_json::to_value(c).ok(),
            DisplayerConfig::RetroTerminal(c) => serde_json::to_value(c).ok(),
            DisplayerConfig::FighterHud(c) => serde_json::to_value(c).ok(),
            DisplayerConfig::Synthwave(c) => serde_json::to_value(c).ok(),
            DisplayerConfig::ArtDeco(c) => serde_json::to_value(c).ok(),
            DisplayerConfig::ArtNouveau(c) => serde_json::to_value(c).ok(),
            DisplayerConfig::Steampunk(c) => serde_json::to_value(c).ok(),
            DisplayerConfig::CssTemplate(c) => serde_json::to_value(c).ok(),
        }
    }
}

impl Default for DisplayerConfig {
    fn default() -> Self {
        DisplayerConfig::Text(TextDisplayerConfig::default())
    }
}

/// Panel appearance settings (background, border, corner radius, transform)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelAppearance {
    pub background: BackgroundConfig,
    pub corner_radius: f64,
    pub border: PanelBorderConfig,
    /// Scale factor for content (1.0 = normal, 0.5 = half size, 2.0 = double)
    #[serde(default = "default_scale")]
    pub scale: f64,
    /// Translation X offset in pixels
    #[serde(default)]
    pub translate_x: f64,
    /// Translation Y offset in pixels
    #[serde(default)]
    pub translate_y: f64,
    /// Z-index for layering (higher = in front, lower = behind, default 0)
    #[serde(default)]
    pub z_index: i32,
    /// If true, this panel ignores collision detection and can overlap other panels
    #[serde(default)]
    pub ignore_collision: bool,
}

fn default_scale() -> f64 {
    1.0
}

impl Default for PanelAppearance {
    fn default() -> Self {
        Self {
            background: BackgroundConfig::default(),
            corner_radius: 8.0,
            border: PanelBorderConfig::default(),
            scale: 1.0,
            translate_x: 0.0,
            translate_y: 0.0,
            z_index: 0,
            ignore_collision: false,
        }
    }
}

/// The unified panel data structure - single source of truth for all panel configuration.
///
/// This struct contains everything needed to:
/// 1. Create and configure a panel at runtime
/// 2. Serialize/deserialize to JSON for persistence
/// 3. Build the config dialog UI
/// 4. Update sources and displayers when config changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelData {
    /// Unique panel identifier
    pub id: String,

    /// Grid position and size
    pub geometry: PanelGeometry,

    /// Type-safe source configuration (includes source type in the enum variant)
    pub source_config: SourceConfig,

    /// Type-safe displayer configuration (includes displayer type in the enum variant)
    pub displayer_config: DisplayerConfig,

    /// Visual appearance (background, border, corners)
    pub appearance: PanelAppearance,
}

impl PanelData {
    /// Create a new PanelData with default configurations
    pub fn new(id: String, geometry: PanelGeometry) -> Self {
        Self {
            id,
            geometry,
            source_config: SourceConfig::default(),
            displayer_config: DisplayerConfig::default(),
            appearance: PanelAppearance::default(),
        }
    }

    /// Create PanelData with specific source and displayer types (using defaults for those types)
    pub fn with_types(
        id: String,
        geometry: PanelGeometry,
        source_type: &str,
        displayer_type: &str,
    ) -> Self {
        Self {
            id,
            geometry,
            source_config: SourceConfig::default_for_type(source_type).unwrap_or_default(),
            displayer_config: DisplayerConfig::default_for_type(displayer_type).unwrap_or_default(),
            appearance: PanelAppearance::default(),
        }
    }

    /// Get the source type ID string
    pub fn source_type(&self) -> &'static str {
        self.source_config.source_type()
    }

    /// Get the displayer type ID string
    pub fn displayer_type(&self) -> &'static str {
        self.displayer_config.displayer_type()
    }

    /// Convert source config to legacy HashMap format
    pub fn source_config_map(&self) -> HashMap<String, serde_json::Value> {
        self.source_config.to_hashmap()
    }

    /// Convert displayer config to legacy HashMap format
    pub fn displayer_config_map(&self) -> HashMap<String, serde_json::Value> {
        self.displayer_config.to_hashmap()
    }

    /// Get combined config map (source + displayer) for legacy interfaces
    pub fn combined_config_map(&self) -> HashMap<String, serde_json::Value> {
        let mut map = self.source_config.to_hashmap();
        map.extend(self.displayer_config.to_hashmap());
        map
    }
}

impl Default for PanelData {
    fn default() -> Self {
        Self::new("default".to_string(), PanelGeometry::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_config_serialization() {
        let config = SourceConfig::Cpu(CpuSourceConfig::default());
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"source_type\":\"cpu\""));

        let deserialized: SourceConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.source_type(), "cpu");
    }

    #[test]
    fn test_displayer_config_serialization() {
        let config = DisplayerConfig::Bar(BarDisplayConfig::default());
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"displayer_type\":\"bar\""));

        let deserialized: DisplayerConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.displayer_type(), "bar");
    }

    #[test]
    fn test_panel_data_serialization() {
        let data = PanelData::with_types(
            "test-panel".to_string(),
            PanelGeometry {
                x: 0,
                y: 0,
                width: 2,
                height: 1,
            },
            "cpu",
            "bar",
        );

        let json = serde_json::to_string_pretty(&data).unwrap();
        let deserialized: PanelData = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, "test-panel");
        assert_eq!(deserialized.source_type(), "cpu");
        assert_eq!(deserialized.displayer_type(), "bar");
    }

    #[test]
    fn test_to_hashmap_compatibility() {
        let config = SourceConfig::Cpu(CpuSourceConfig::default());
        let map = config.to_hashmap();

        assert!(map.contains_key("cpu_config"));
    }
}
