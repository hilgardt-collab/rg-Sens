//! Application and panel configuration

use anyhow::Result;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::audio::AlarmSoundConfig;
use crate::core::{
    AlarmConfig, DisplayerConfig, PanelAppearance, PanelBorderConfig, PanelData, PanelGeometry,
    SourceConfig, TimerConfig,
};
use crate::ui::background::BackgroundConfig;
use crate::ui::theme::ComboThemeConfig;

/// Current config format version
pub const CONFIG_VERSION: u32 = 2;

/// Application-wide configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Version of the config format (1 = legacy, 2 = PanelData-based)
    #[serde(default = "default_version")]
    pub version: u32,
    /// Window dimensions
    pub window: WindowConfig,
    /// Grid settings
    pub grid: GridConfig,
    /// Panels configuration (v2 format with typed configs)
    /// Note: This is NOT serialized directly - see load_from_string and as_v2
    #[serde(skip)]
    pub panels_v2: Vec<PanelConfigV2>,
    /// Legacy panels configuration (v1 format, for migration)
    /// Note: This is only used during v1 deserialization
    #[serde(skip)]
    pub panels_v1: Vec<PanelConfig>,
    /// Global timers (shared across all clock displays)
    #[serde(default)]
    pub timers: Vec<TimerConfig>,
    /// Global alarms (shared across all clock displays)
    #[serde(default)]
    pub alarms: Vec<AlarmConfig>,
    /// Global timer sound configuration (used for all timers)
    #[serde(default)]
    pub global_timer_sound: AlarmSoundConfig,
    /// Global theme for non-combo panels (colors, fonts, gradient)
    #[serde(default)]
    pub global_theme: ComboThemeConfig,
}

fn default_version() -> u32 {
    1 // Default to v1 for backward compatibility when loading old configs
}

impl AppConfig {
    /// Load configuration from disk with auto-migration
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        if !config_path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(&config_path)?;
        Self::load_from_string(&content)
    }

    /// Load configuration from a JSON string with auto-migration
    pub fn load_from_string(content: &str) -> Result<Self> {
        // Parse JSON once, then use from_value to deserialize into the correct struct
        let raw: serde_json::Value = serde_json::from_str(content)?;
        let version = raw.get("version").and_then(|v| v.as_u64()).unwrap_or(1) as u32;

        if version >= 2 {
            // V2 format - use AppConfigLoad which has the correct "panels" field
            // Use from_value to avoid parsing the JSON string twice
            let loaded: AppConfigLoad = serde_json::from_value(raw)?;
            Ok(AppConfig {
                version: loaded.version,
                window: loaded.window,
                grid: loaded.grid,
                panels_v2: loaded.panels,
                panels_v1: Vec::new(),
                timers: loaded.timers,
                alarms: loaded.alarms,
                global_timer_sound: loaded.global_timer_sound,
                global_theme: loaded.global_theme,
            })
        } else {
            // V1 format - load and migrate
            info!("Migrating config from v1 to v2 format");
            // Use from_value to avoid parsing the JSON string twice
            let v1_config: AppConfigV1 = serde_json::from_value(raw)?;
            Ok(v1_config.migrate_to_v2())
        }
    }

    /// Save configuration to disk (always saves in v2 format)
    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;

        // Ensure parent directory exists
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Always save as v2
        let save_config = self.as_v2();
        let content = serde_json::to_string_pretty(&save_config)?;
        std::fs::write(config_path, content)?;
        Ok(())
    }

    /// Get the configuration file path
    fn config_path() -> Result<PathBuf> {
        let dirs = directories::ProjectDirs::from("com", "github.hilgardt_collab", "rg-sens")
            .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;

        Ok(dirs.config_dir().join("config.json"))
    }

    /// Load configuration from a specific file path
    pub fn load_from_path(path: &PathBuf) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Self::load_from_string(&content)
    }

    /// Save configuration to a specific file path
    pub fn save_to_path(&self, path: &PathBuf) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let save_config = self.as_v2();
        let content = serde_json::to_string_pretty(&save_config)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Get panels as PanelData (prefers v2, falls back to migrated v1)
    pub fn get_panels(&self) -> Vec<PanelData> {
        if !self.panels_v2.is_empty() {
            self.panels_v2.iter().map(|p| p.to_panel_data()).collect()
        } else {
            // Migrate v1 panels on-the-fly
            self.panels_v1.iter().map(|p| p.to_panel_data()).collect()
        }
    }

    /// Set panels from PanelData (stores in v2 format)
    pub fn set_panels(&mut self, panels: Vec<PanelData>) {
        self.panels_v2 = panels
            .into_iter()
            .map(PanelConfigV2::from_panel_data)
            .collect();
        self.panels_v1.clear(); // Clear legacy panels
        self.version = CONFIG_VERSION;
    }

    /// Convert to v2 format for saving
    fn as_v2(&self) -> AppConfigSave {
        let panels = if !self.panels_v2.is_empty() {
            self.panels_v2.clone()
        } else {
            // Convert v1 to v2
            self.panels_v1
                .iter()
                .map(|p| PanelConfigV2::from_panel_data(p.to_panel_data()))
                .collect()
        };

        AppConfigSave {
            version: CONFIG_VERSION,
            window: self.window.clone(),
            grid: self.grid.clone(),
            panels,
            timers: self.timers.clone(),
            alarms: self.alarms.clone(),
            global_timer_sound: self.global_timer_sound.clone(),
            global_theme: self.global_theme.clone(),
        }
    }

    /// Update timers from global manager
    pub fn set_timers(&mut self, timers: Vec<TimerConfig>) {
        self.timers = timers;
    }

    /// Update alarms from global manager
    pub fn set_alarms(&mut self, alarms: Vec<AlarmConfig>) {
        self.alarms = alarms;
    }

    /// Update global timer sound from global manager
    pub fn set_global_timer_sound(&mut self, sound: AlarmSoundConfig) {
        self.global_timer_sound = sound;
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION,
            window: WindowConfig::default(),
            grid: GridConfig::default(),
            panels_v2: Vec::new(),
            panels_v1: Vec::new(),
            timers: Vec::new(),
            alarms: Vec::new(),
            global_timer_sound: AlarmSoundConfig::default(),
            global_theme: ComboThemeConfig::default(),
        }
    }
}

/// V1 config format (for loading legacy configs)
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct AppConfigV1 {
    #[serde(default = "default_version")]
    pub version: u32,
    pub window: WindowConfig,
    pub grid: GridConfig,
    pub panels: Vec<PanelConfig>,
}

impl AppConfigV1 {
    fn migrate_to_v2(self) -> AppConfig {
        AppConfig {
            version: CONFIG_VERSION,
            window: self.window,
            grid: self.grid,
            panels_v2: self
                .panels
                .iter()
                .map(|p| PanelConfigV2::from_panel_data(p.to_panel_data()))
                .collect(),
            panels_v1: Vec::new(), // Don't keep v1 after migration
            timers: Vec::new(),    // V1 configs don't have global timers
            alarms: Vec::new(),    // V1 configs don't have global alarms
            global_timer_sound: AlarmSoundConfig::default(),
            global_theme: ComboThemeConfig::default(),
        }
    }
}

/// Config format for saving (always v2)
#[derive(Debug, Clone, Serialize)]
struct AppConfigSave {
    pub version: u32,
    pub window: WindowConfig,
    pub grid: GridConfig,
    pub panels: Vec<PanelConfigV2>,
    pub timers: Vec<TimerConfig>,
    pub alarms: Vec<AlarmConfig>,
    pub global_timer_sound: AlarmSoundConfig,
    pub global_theme: ComboThemeConfig,
}

/// Config format for loading v2 configs
#[derive(Debug, Clone, Deserialize)]
struct AppConfigLoad {
    #[serde(default = "default_version")]
    pub version: u32,
    pub window: WindowConfig,
    pub grid: GridConfig,
    #[serde(default)]
    pub panels: Vec<PanelConfigV2>,
    #[serde(default)]
    pub timers: Vec<TimerConfig>,
    #[serde(default)]
    pub alarms: Vec<AlarmConfig>,
    #[serde(default)]
    pub global_timer_sound: AlarmSoundConfig,
    #[serde(default)]
    pub global_theme: ComboThemeConfig,
}

/// Window configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowConfig {
    pub width: i32,
    pub height: i32,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub background: BackgroundConfig,
    /// Default corner radius for new panels
    #[serde(default = "default_panel_corner_radius")]
    pub panel_corner_radius: f64,
    /// Default border config for new panels
    #[serde(default)]
    pub panel_border: PanelBorderConfig,
    /// Start in fullscreen mode
    #[serde(default)]
    pub fullscreen_enabled: bool,
    /// Monitor index for fullscreen (None = current monitor)
    #[serde(default)]
    pub fullscreen_monitor: Option<i32>,
    /// Borderless window mode (no title bar or window decorations)
    #[serde(default)]
    pub borderless: bool,
    /// Window was maximized when last closed
    #[serde(default)]
    pub maximized: bool,
    /// Monitor connector name where window was last shown (e.g., "HDMI-1", "DP-1")
    #[serde(default)]
    pub monitor_connector: Option<String>,
    /// Enable auto-scroll when content extends beyond visible area
    #[serde(default)]
    pub auto_scroll_enabled: bool,
    /// Delay between auto-scroll steps in milliseconds
    #[serde(default = "default_auto_scroll_delay")]
    pub auto_scroll_delay_ms: u64,
    /// Viewport width for auto-scroll page boundaries (0 = use window width)
    #[serde(default)]
    pub viewport_width: i32,
    /// Viewport height for auto-scroll page boundaries (0 = use window height)
    #[serde(default)]
    pub viewport_height: i32,
    /// Scroll whole pages only (align to viewport boundaries, ignore panel positions)
    #[serde(default)]
    pub auto_scroll_whole_pages: bool,
}

fn default_auto_scroll_delay() -> u64 {
    5000 // 5 seconds default
}

fn default_panel_corner_radius() -> f64 {
    8.0
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            width: 800,
            height: 600,
            x: None,
            y: None,
            background: BackgroundConfig::default(),
            panel_corner_radius: 8.0,
            panel_border: PanelBorderConfig::default(),
            fullscreen_enabled: false,
            fullscreen_monitor: None,
            borderless: false,
            maximized: false,
            monitor_connector: None,
            auto_scroll_enabled: false,
            auto_scroll_delay_ms: 5000,
            viewport_width: 0,
            viewport_height: 0,
            auto_scroll_whole_pages: false,
        }
    }
}

/// Grid configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridConfig {
    pub columns: u32,
    pub rows: u32,
    pub cell_width: i32,
    pub cell_height: i32,
    pub spacing: i32,
}

impl Default for GridConfig {
    fn default() -> Self {
        Self {
            columns: 4,
            rows: 3,
            cell_width: 16,
            cell_height: 16,
            spacing: 2,
        }
    }
}

/// Panel configuration (V2 format - uses typed configs)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelConfigV2 {
    /// Unique ID for this panel
    pub id: String,
    /// Position and size
    pub geometry: PanelGeometry,
    /// Type-safe source configuration
    pub source_config: SourceConfig,
    /// Type-safe displayer configuration
    pub displayer_config: DisplayerConfig,
    /// Visual appearance
    pub appearance: PanelAppearance,
}

impl PanelConfigV2 {
    /// Convert to PanelData
    pub fn to_panel_data(&self) -> PanelData {
        PanelData {
            id: self.id.clone(),
            geometry: self.geometry,
            source_config: self.source_config.clone(),
            displayer_config: self.displayer_config.clone(),
            appearance: self.appearance.clone(),
        }
    }

    /// Create from PanelData
    pub fn from_panel_data(data: PanelData) -> Self {
        Self {
            id: data.id,
            geometry: data.geometry,
            source_config: data.source_config,
            displayer_config: data.displayer_config,
            appearance: data.appearance,
        }
    }
}

/// Legacy panel configuration (V1 format - for migration)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelConfig {
    /// Unique ID for this panel
    pub id: String,
    /// Position and size
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    /// Data source ID
    pub source: String,
    /// Displayer ID
    pub displayer: String,
    /// Panel background
    #[serde(default)]
    pub background: BackgroundConfig,
    /// Corner radius for panel edges
    #[serde(default = "default_panel_corner_radius")]
    pub corner_radius: f64,
    /// Border configuration
    #[serde(default)]
    pub border: PanelBorderConfig,
    /// Custom settings
    #[serde(default)]
    pub settings: HashMap<String, serde_json::Value>,
}

impl PanelConfig {
    /// Convert legacy V1 config to PanelData
    ///
    /// This attempts to migrate the old HashMap-based settings to typed configs.
    /// If migration fails for source/displayer config, defaults are used.
    pub fn to_panel_data(&self) -> PanelData {
        let geometry = PanelGeometry {
            x: self.x,
            y: self.y,
            width: self.width,
            height: self.height,
        };

        // Try to reconstruct source config from settings
        let source_config = self.migrate_source_config();

        // Try to reconstruct displayer config from settings
        let displayer_config = self.migrate_displayer_config();

        let appearance = PanelAppearance {
            background: self.background.clone(),
            corner_radius: self.corner_radius,
            border: self.border.clone(),
            scale: 1.0,
            translate_x: 0.0,
            translate_y: 0.0,
            z_index: 0,
            ignore_collision: false,
        };

        PanelData {
            id: self.id.clone(),
            geometry,
            source_config,
            displayer_config,
            appearance,
        }
    }

    /// Migrate source settings to typed SourceConfig
    fn migrate_source_config(&self) -> SourceConfig {
        // Helper to try migration with logging on failure
        fn try_migrate<T: serde::de::DeserializeOwned>(
            val: &serde_json::Value,
            source_type: &str,
            panel_id: &str,
        ) -> Option<T> {
            match serde_json::from_value(val.clone()) {
                Ok(cfg) => Some(cfg),
                Err(e) => {
                    warn!(
                        "Failed to migrate {} config for panel '{}': {}. Using defaults.",
                        source_type, panel_id, e
                    );
                    None
                }
            }
        }

        // Try to extract typed config from settings based on source type
        match self.source.as_str() {
            "cpu" => {
                if let Some(val) = self.settings.get("cpu_config") {
                    if let Some(cfg) = try_migrate(val, "cpu", &self.id) {
                        return SourceConfig::Cpu(cfg);
                    }
                }
                SourceConfig::default_for_type("cpu").unwrap_or_default()
            }
            "gpu" => {
                if let Some(val) = self.settings.get("gpu_config") {
                    if let Some(cfg) = try_migrate(val, "gpu", &self.id) {
                        return SourceConfig::Gpu(cfg);
                    }
                }
                SourceConfig::default_for_type("gpu").unwrap_or_default()
            }
            "memory" => {
                if let Some(val) = self.settings.get("memory_config") {
                    if let Some(cfg) = try_migrate(val, "memory", &self.id) {
                        return SourceConfig::Memory(cfg);
                    }
                }
                SourceConfig::default_for_type("memory").unwrap_or_default()
            }
            "disk" => {
                if let Some(val) = self.settings.get("disk_config") {
                    if let Some(cfg) = try_migrate(val, "disk", &self.id) {
                        return SourceConfig::Disk(cfg);
                    }
                }
                SourceConfig::default_for_type("disk").unwrap_or_default()
            }
            "clock" => {
                if let Some(val) = self.settings.get("clock_config") {
                    if let Some(cfg) = try_migrate(val, "clock", &self.id) {
                        return SourceConfig::Clock(cfg);
                    }
                }
                SourceConfig::default_for_type("clock").unwrap_or_default()
            }
            "combination" => {
                if let Some(val) = self.settings.get("combo_config") {
                    if let Some(cfg) = try_migrate(val, "combination", &self.id) {
                        return SourceConfig::Combo(cfg);
                    }
                }
                SourceConfig::default_for_type("combination").unwrap_or_default()
            }
            "system_temp" => {
                if let Some(val) = self.settings.get("system_temp_config") {
                    if let Some(cfg) = try_migrate(val, "system_temp", &self.id) {
                        return SourceConfig::SystemTemp(cfg);
                    }
                }
                SourceConfig::default_for_type("system_temp").unwrap_or_default()
            }
            "fan_speed" => {
                if let Some(val) = self.settings.get("fan_speed_config") {
                    if let Some(cfg) = try_migrate(val, "fan_speed", &self.id) {
                        return SourceConfig::FanSpeed(cfg);
                    }
                }
                SourceConfig::default_for_type("fan_speed").unwrap_or_default()
            }
            "test" => {
                if let Some(val) = self.settings.get("test_config") {
                    if let Some(cfg) = try_migrate(val, "test", &self.id) {
                        return SourceConfig::Test(cfg);
                    }
                }
                SourceConfig::default_for_type("test").unwrap_or_default()
            }
            "static_text" => {
                if let Some(val) = self.settings.get("static_text_config") {
                    if let Some(cfg) = try_migrate(val, "static_text", &self.id) {
                        return SourceConfig::StaticText(cfg);
                    }
                }
                SourceConfig::default_for_type("static_text").unwrap_or_default()
            }
            _ => {
                warn!(
                    "Unknown source type '{}', using default CPU config",
                    self.source
                );
                SourceConfig::default()
            }
        }
    }

    /// Migrate displayer settings to typed DisplayerConfig
    fn migrate_displayer_config(&self) -> DisplayerConfig {
        // Helper to try migration with logging on failure
        fn try_migrate<T: serde::de::DeserializeOwned>(
            val: &serde_json::Value,
            displayer_type: &str,
            panel_id: &str,
        ) -> Option<T> {
            match serde_json::from_value(val.clone()) {
                Ok(cfg) => Some(cfg),
                Err(e) => {
                    warn!(
                        "Failed to migrate {} displayer config for panel '{}': {}. Using defaults.",
                        displayer_type, panel_id, e
                    );
                    None
                }
            }
        }

        // Try to extract typed config from settings based on displayer type
        match self.displayer.as_str() {
            "text" => {
                if let Some(val) = self.settings.get("text_config") {
                    if let Some(cfg) = try_migrate(val, "text", &self.id) {
                        return DisplayerConfig::Text(cfg);
                    }
                }
                DisplayerConfig::default_for_type("text").unwrap_or_default()
            }
            "bar" => {
                if let Some(val) = self.settings.get("bar_config") {
                    if let Some(cfg) = try_migrate(val, "bar", &self.id) {
                        return DisplayerConfig::Bar(cfg);
                    }
                }
                DisplayerConfig::default_for_type("bar").unwrap_or_default()
            }
            "arc" => {
                if let Some(val) = self.settings.get("arc_config") {
                    if let Some(cfg) = try_migrate(val, "arc", &self.id) {
                        return DisplayerConfig::Arc(cfg);
                    }
                }
                DisplayerConfig::default_for_type("arc").unwrap_or_default()
            }
            "speedometer" => {
                if let Some(val) = self.settings.get("speedometer_config") {
                    if let Some(cfg) = try_migrate(val, "speedometer", &self.id) {
                        return DisplayerConfig::Speedometer(cfg);
                    }
                }
                DisplayerConfig::default_for_type("speedometer").unwrap_or_default()
            }
            "graph" => {
                if let Some(val) = self.settings.get("graph_config") {
                    if let Some(cfg) = try_migrate(val, "graph", &self.id) {
                        return DisplayerConfig::Graph(cfg);
                    }
                }
                DisplayerConfig::default_for_type("graph").unwrap_or_default()
            }
            "clock_analog" => {
                if let Some(val) = self.settings.get("clock_analog_config") {
                    if let Some(cfg) = try_migrate(val, "clock_analog", &self.id) {
                        return DisplayerConfig::ClockAnalog(cfg);
                    }
                }
                DisplayerConfig::default_for_type("clock_analog").unwrap_or_default()
            }
            "clock_digital" => {
                if let Some(val) = self.settings.get("clock_digital_config") {
                    if let Some(cfg) = try_migrate(val, "clock_digital", &self.id) {
                        return DisplayerConfig::ClockDigital(cfg);
                    }
                }
                DisplayerConfig::default_for_type("clock_digital").unwrap_or_default()
            }
            "lcars" => {
                if let Some(val) = self.settings.get("lcars_config") {
                    if let Some(cfg) = try_migrate(val, "lcars", &self.id) {
                        return DisplayerConfig::Lcars(cfg);
                    }
                }
                DisplayerConfig::default_for_type("lcars").unwrap_or_default()
            }
            "cpu_cores" => {
                if let Some(val) = self.settings.get("core_bars_config") {
                    if let Some(cfg) = try_migrate(val, "cpu_cores", &self.id) {
                        return DisplayerConfig::CpuCores(cfg);
                    }
                }
                DisplayerConfig::default_for_type("cpu_cores").unwrap_or_default()
            }
            "indicator" => {
                if let Some(val) = self.settings.get("indicator_config") {
                    if let Some(cfg) = try_migrate(val, "indicator", &self.id) {
                        return DisplayerConfig::Indicator(cfg);
                    }
                }
                DisplayerConfig::default_for_type("indicator").unwrap_or_default()
            }
            _ => {
                warn!(
                    "Unknown displayer type '{}', using default Text config",
                    self.displayer
                );
                DisplayerConfig::default()
            }
        }
    }
}
