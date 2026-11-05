//! Application and panel configuration

use anyhow::Result;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use crate::audio::AlarmSoundConfig;
use crate::core::{
    AlarmConfig, DisplayerConfig, PanelAppearance, PanelBorderConfig, PanelData, PanelGeometry,
    SourceConfig, TimerConfig,
};
use crate::ui::background::BackgroundConfig;
use crate::ui::theme::ComboThemeConfig;
use rg_sens_types::theme::{PanelGroup, ThemeLibrary, ThemeRef};

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
    /// User theme library, keyed by name. Built-in presets are not stored
    /// here; see `ComboThemeConfig::builtin_presets`.
    #[serde(default)]
    pub themes: BTreeMap<String, ComboThemeConfig>,
    /// Panel groups (sets of standalone panels sharing a theme).
    #[serde(default)]
    pub groups: Vec<PanelGroup>,
}

fn default_version() -> u32 {
    1 // Default to v1 for backward compatibility when loading old configs
}

impl AppConfig {
    /// Load configuration from disk with auto-migration
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        match std::fs::read_to_string(&config_path) {
            Ok(content) => Self::load_from_string(&content),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e.into()),
        }
    }

    /// Load configuration from a JSON string with auto-migration
    pub fn load_from_string(content: &str) -> Result<Self> {
        // Parse JSON once, then use from_value to deserialize into the correct struct
        let raw: serde_json::Value = serde_json::from_str(content)?;
        let version = raw.get("version").and_then(|v| v.as_u64()).unwrap_or(1) as u32;

        let mut config = if version >= 2 {
            // Panels written before the theme library existed have no "theme"
            // key. Detect that by key ABSENCE (not by value): a new combo panel
            // legitimately saved as `Inherit` must not be migrated on reload.
            let panels_missing_theme: Vec<bool> = raw
                .get("panels")
                .and_then(|p| p.as_array())
                .map(|arr| arr.iter().map(|p| p.get("theme").is_none()).collect())
                .unwrap_or_default();

            // V2 format - use AppConfigLoad which has the correct "panels" field
            // Use from_value to avoid parsing the JSON string twice
            let loaded: AppConfigLoad = serde_json::from_value(raw)?;
            let mut panels = loaded.panels;
            for (panel, missing) in panels.iter_mut().zip(panels_missing_theme) {
                if missing {
                    migrate_inline_theme(panel);
                }
            }
            AppConfig {
                version: loaded.version,
                window: loaded.window,
                grid: loaded.grid,
                panels_v2: panels,
                panels_v1: Vec::new(),
                timers: loaded.timers,
                alarms: loaded.alarms,
                global_timer_sound: loaded.global_timer_sound,
                global_theme: loaded.global_theme,
                themes: loaded.themes,
                groups: loaded.groups,
            }
        } else {
            // V1 format - load and migrate
            info!("Migrating config from v1 to v2 format");
            // Use from_value to avoid parsing the JSON string twice
            let v1_config: AppConfigV1 = serde_json::from_value(raw)?;
            v1_config.migrate_to_v2()
        };

        // Sanitize loaded config to fix any invalid values
        config.sanitize();

        Ok(config)
    }

    /// Save configuration to disk (always saves in v2 format)
    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;

        // Always save as v2
        let save_config = self.as_v2();
        let content = serde_json::to_string_pretty(&save_config)?;
        super::write_atomic(&config_path, &content)?;
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
    pub fn save_to_path(&self, path: &std::path::Path) -> Result<()> {
        let save_config = self.as_v2();
        let content = serde_json::to_string_pretty(&save_config)?;
        super::write_atomic(path, &content)?;
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
            themes: self.themes.clone(),
            groups: self.groups.clone(),
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

    /// Sanitize configuration to ensure all values are valid
    /// This corrects any invalid values loaded from disk (e.g., corrupted config)
    pub fn sanitize(&mut self) {
        let window_corrected = self.window.sanitize();
        let grid_corrected = self.grid.sanitize();

        if window_corrected || grid_corrected {
            log::info!("Configuration sanitized: corrected invalid values");
        }

        // A group_id naming no group would silently resolve to the global
        // theme with nothing in the UI explaining why; drop it and say so.
        let group_ids: Vec<String> = self.groups.iter().map(|g| g.id.clone()).collect();
        for panel in &mut self.panels_v2 {
            if let Some(gid) = &panel.group_id {
                if !group_ids.contains(gid) {
                    warn!(
                        "Panel {} references missing group {}; removing it from the group",
                        panel.id, gid
                    );
                    panel.group_id = None;
                }
            }
        }
        for group in &self.groups {
            if let Some(name) = group.theme.named_name() {
                if !self.themes.contains_key(name) && !ComboThemeConfig::is_builtin_name(name) {
                    warn!(
                        "Group '{}' references missing theme '{}'; it will fall back",
                        group.name, name
                    );
                }
            }
        }
    }

    // ------------------------------------------------------------------
    // Theme library
    // ------------------------------------------------------------------

    /// Borrowed view used for theme resolution.
    pub fn theme_library(&self) -> ThemeLibrary<'_> {
        ThemeLibrary {
            global: &self.global_theme,
            themes: &self.themes,
            groups: &self.groups,
        }
    }

    /// Resolve the concrete theme for a panel: panel → group → global.
    pub fn resolve_theme_for(&self, theme: &ThemeRef, group_id: Option<&str>) -> ComboThemeConfig {
        self.theme_library().resolve(theme, group_id)
    }

    /// Resolve the theme for a stored panel by id. Unknown ids resolve to
    /// the global theme.
    pub fn resolve_theme_for_panel_id(&self, panel_id: &str) -> ComboThemeConfig {
        match self.panels_v2.iter().find(|p| p.id == panel_id) {
            Some(p) => self.resolve_theme_for(&p.theme, p.group_id.as_deref()),
            None => self.global_theme.clone(),
        }
    }

    /// Find a group by id.
    pub fn group(&self, id: &str) -> Option<&PanelGroup> {
        self.groups.iter().find(|g| g.id == id)
    }

    /// Mutable access to a group by id.
    pub fn group_mut(&mut self, id: &str) -> Option<&mut PanelGroup> {
        self.groups.iter_mut().find(|g| g.id == id)
    }

    /// Ids of the panels in a group, in stored order.
    pub fn group_members(&self, group_id: &str) -> Vec<String> {
        self.panels_v2
            .iter()
            .filter(|p| p.group_id.as_deref() == Some(group_id))
            .map(|p| p.id.clone())
            .collect()
    }

    /// Everything that references a library theme by name: panel ids and
    /// group ids. Used for the "Used by" label and to refuse deletion.
    pub fn theme_users(&self, name: &str) -> (Vec<String>, Vec<String>) {
        let panels = self
            .panels_v2
            .iter()
            .filter(|p| p.theme.named_name() == Some(name))
            .map(|p| p.id.clone())
            .collect();
        let groups = self
            .groups
            .iter()
            .filter(|g| g.theme.named_name() == Some(name))
            .map(|g| g.id.clone())
            .collect();
        (panels, groups)
    }

    /// A theme name not yet taken by a user theme or a built-in preset,
    /// derived from `base` (`base`, `base 2`, `base 3`, …).
    pub fn unique_theme_name(&self, base: &str) -> String {
        let taken = |n: &str| self.themes.contains_key(n) || ComboThemeConfig::is_builtin_name(n);
        if !taken(base) {
            return base.to_string();
        }
        (2..)
            .map(|i| format!("{} {}", base, i))
            .find(|n| !taken(n))
            .expect("unbounded counter")
    }

    /// Add or replace a user theme. Built-in names cannot be shadowed.
    pub fn set_theme(&mut self, name: &str, theme: ComboThemeConfig) -> Result<()> {
        if ComboThemeConfig::is_builtin_name(name) {
            anyhow::bail!("'{}' is a built-in preset and cannot be replaced", name);
        }
        self.themes.insert(name.to_string(), theme);
        Ok(())
    }

    /// Rename a user theme and rewrite every reference to it.
    pub fn rename_theme(&mut self, old: &str, new: &str) -> Result<()> {
        if old == new {
            return Ok(());
        }
        if ComboThemeConfig::is_builtin_name(old) {
            anyhow::bail!("'{}' is a built-in preset and cannot be renamed", old);
        }
        if new.trim().is_empty() {
            anyhow::bail!("Theme name cannot be empty");
        }
        if self.themes.contains_key(new) || ComboThemeConfig::is_builtin_name(new) {
            anyhow::bail!("A theme named '{}' already exists", new);
        }
        let theme = self
            .themes
            .remove(old)
            .ok_or_else(|| anyhow::anyhow!("No theme named '{}'", old))?;
        self.themes.insert(new.to_string(), theme);
        for panel in &mut self.panels_v2 {
            if panel.theme.named_name() == Some(old) {
                panel.theme = ThemeRef::named(new);
            }
        }
        for group in &mut self.groups {
            if group.theme.named_name() == Some(old) {
                group.theme = ThemeRef::named(new);
            }
        }
        Ok(())
    }

    /// Delete a user theme. Refused while any panel or group references it.
    pub fn delete_theme(&mut self, name: &str) -> Result<()> {
        if ComboThemeConfig::is_builtin_name(name) {
            anyhow::bail!("'{}' is a built-in preset and cannot be deleted", name);
        }
        let (panels, groups) = self.theme_users(name);
        if !panels.is_empty() || !groups.is_empty() {
            anyhow::bail!(
                "Theme '{}' is in use by {} panel(s) and {} group(s)",
                name,
                panels.len(),
                groups.len()
            );
        }
        self.themes
            .remove(name)
            .map(|_| ())
            .ok_or_else(|| anyhow::anyhow!("No theme named '{}'", name))
    }

    /// Create a new group with the global theme; returns its id.
    pub fn add_group(&mut self, name: &str) -> String {
        let group = PanelGroup::new(name);
        let id = group.id.clone();
        self.groups.push(group);
        id
    }

    /// Delete a group. Its members become ungrouped.
    pub fn delete_group(&mut self, group_id: &str) {
        self.groups.retain(|g| g.id != group_id);
        for panel in &mut self.panels_v2 {
            if panel.group_id.as_deref() == Some(group_id) {
                panel.group_id = None;
            }
        }
    }

    /// Move a stored panel into a group (`None` = ungrouped).
    pub fn set_panel_group(&mut self, panel_id: &str, group_id: Option<&str>) -> Result<()> {
        if let Some(gid) = group_id {
            if self.group(gid).is_none() {
                anyhow::bail!("No group with id {}", gid);
            }
        }
        let panel = self
            .panels_v2
            .iter_mut()
            .find(|p| p.id == panel_id)
            .ok_or_else(|| anyhow::anyhow!("No panel with id {}", panel_id))?;
        panel.group_id = group_id.map(|s| s.to_string());
        Ok(())
    }

    /// Set a stored panel's theme reference.
    pub fn set_panel_theme(&mut self, panel_id: &str, theme: ThemeRef) -> Result<()> {
        let panel = self
            .panels_v2
            .iter_mut()
            .find(|p| p.id == panel_id)
            .ok_or_else(|| anyhow::anyhow!("No panel with id {}", panel_id))?;
        panel.theme = theme;
        Ok(())
    }
}

/// Phase-0 migration: a combo panel saved before the theme library existed
/// keeps rendering with the theme embedded in its frame by becoming
/// `Inline { frame.theme }`. Unconditional on key absence, so a combo whose
/// frame theme happens to equal the defaults is preserved too.
fn migrate_inline_theme(panel: &mut PanelConfigV2) {
    if let Some(frame_theme) = panel.displayer_config.theme() {
        panel.theme = ThemeRef::inline(frame_theme.clone());
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
            themes: BTreeMap::new(),
            groups: Vec::new(),
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
                .map(|p| {
                    let mut panel = PanelConfigV2::from_panel_data(p.to_panel_data());
                    // V1 never had a theme key: same migration as v2 key-absence
                    migrate_inline_theme(&mut panel);
                    panel
                })
                .collect(),
            panels_v1: Vec::new(), // Don't keep v1 after migration
            timers: Vec::new(),    // V1 configs don't have global timers
            alarms: Vec::new(),    // V1 configs don't have global alarms
            global_timer_sound: AlarmSoundConfig::default(),
            global_theme: ComboThemeConfig::default(),
            themes: BTreeMap::new(),
            groups: Vec::new(),
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
    pub themes: BTreeMap<String, ComboThemeConfig>,
    pub groups: Vec<PanelGroup>,
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
    #[serde(default)]
    pub themes: BTreeMap<String, ComboThemeConfig>,
    #[serde(default)]
    pub groups: Vec<PanelGroup>,
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
    /// Default graphics renderer (gl, ngl, vulkan, cairo)
    /// This is loaded at startup before GTK initialization
    #[serde(default)]
    pub renderer: Option<String>,
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
            renderer: None,
        }
    }
}

impl WindowConfig {
    /// Sanitize window configuration to ensure valid values
    /// Returns true if any values were corrected
    pub fn sanitize(&mut self) -> bool {
        let mut corrected = false;

        // Window dimensions must be positive (minimum 100x100 for usability)
        if self.width < 100 {
            log::warn!("Invalid window width {} corrected to 800", self.width);
            self.width = 800;
            corrected = true;
        }
        if self.height < 100 {
            log::warn!("Invalid window height {} corrected to 600", self.height);
            self.height = 600;
            corrected = true;
        }

        // Corner radius must be non-negative
        if self.panel_corner_radius < 0.0 {
            log::warn!(
                "Invalid corner radius {} corrected to 8.0",
                self.panel_corner_radius
            );
            self.panel_corner_radius = 8.0;
            corrected = true;
        }

        // Viewport dimensions must be non-negative (0 means use window size)
        if self.viewport_width < 0 {
            log::warn!(
                "Invalid viewport width {} corrected to 0",
                self.viewport_width
            );
            self.viewport_width = 0;
            corrected = true;
        }
        if self.viewport_height < 0 {
            log::warn!(
                "Invalid viewport height {} corrected to 0",
                self.viewport_height
            );
            self.viewport_height = 0;
            corrected = true;
        }

        corrected
    }

    /// Resolve the effective viewport width: configured value if > 0, else `default`.
    pub fn effective_viewport_width(&self, default: i32) -> i32 {
        if self.viewport_width > 0 {
            self.viewport_width
        } else {
            default
        }
    }

    /// Resolve the effective viewport height: configured value if > 0, else `default`.
    pub fn effective_viewport_height(&self, default: i32) -> i32 {
        if self.viewport_height > 0 {
            self.viewport_height
        } else {
            default
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

impl GridConfig {
    /// Sanitize grid configuration to ensure valid values
    /// Returns true if any values were corrected
    pub fn sanitize(&mut self) -> bool {
        let mut corrected = false;

        // Grid must have at least 1 column and row
        if self.columns == 0 {
            log::warn!("Invalid grid columns 0 corrected to 4");
            self.columns = 4;
            corrected = true;
        }
        if self.rows == 0 {
            log::warn!("Invalid grid rows 0 corrected to 3");
            self.rows = 3;
            corrected = true;
        }

        // Cell dimensions must be positive (minimum 1 pixel)
        if self.cell_width < 1 {
            log::warn!("Invalid cell width {} corrected to 16", self.cell_width);
            self.cell_width = 16;
            corrected = true;
        }
        if self.cell_height < 1 {
            log::warn!("Invalid cell height {} corrected to 16", self.cell_height);
            self.cell_height = 16;
            corrected = true;
        }

        // Spacing must be non-negative
        if self.spacing < 0 {
            log::warn!("Invalid spacing {} corrected to 2", self.spacing);
            self.spacing = 2;
            corrected = true;
        }

        corrected
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
    /// Theme reference (absent in pre-library configs; see `migrate_inline_theme`)
    #[serde(default)]
    pub theme: ThemeRef,
    /// Panel group membership
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_id: Option<String>,
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
            theme: self.theme.clone(),
            group_id: self.group_id.clone(),
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
            theme: data.theme,
            group_id: data.group_id,
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
            theme: ThemeRef::Inherit,
            group_id: None,
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
            "network" => {
                if let Some(val) = self.settings.get("network_config") {
                    if let Some(cfg) = try_migrate(val, "network", &self.id) {
                        return SourceConfig::Network(cfg);
                    }
                }
                SourceConfig::default_for_type("network").unwrap_or_default()
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
            "cyberpunk" => {
                if let Some(val) = self.settings.get("cyberpunk_config") {
                    if let Some(cfg) = try_migrate(val, "cyberpunk", &self.id) {
                        return DisplayerConfig::Cyberpunk(cfg);
                    }
                }
                DisplayerConfig::default_for_type("cyberpunk").unwrap_or_default()
            }
            "material" => {
                if let Some(val) = self.settings.get("material_config") {
                    if let Some(cfg) = try_migrate(val, "material", &self.id) {
                        return DisplayerConfig::Material(cfg);
                    }
                }
                DisplayerConfig::default_for_type("material").unwrap_or_default()
            }
            "industrial" => {
                if let Some(val) = self.settings.get("industrial_config") {
                    if let Some(cfg) = try_migrate(val, "industrial", &self.id) {
                        return DisplayerConfig::Industrial(cfg);
                    }
                }
                DisplayerConfig::default_for_type("industrial").unwrap_or_default()
            }
            "retro_terminal" => {
                if let Some(val) = self.settings.get("retro_terminal_config") {
                    if let Some(cfg) = try_migrate(val, "retro_terminal", &self.id) {
                        return DisplayerConfig::RetroTerminal(cfg);
                    }
                }
                DisplayerConfig::default_for_type("retro_terminal").unwrap_or_default()
            }
            "fighter_hud" => {
                if let Some(val) = self.settings.get("fighter_hud_config") {
                    if let Some(cfg) = try_migrate(val, "fighter_hud", &self.id) {
                        return DisplayerConfig::FighterHud(cfg);
                    }
                }
                DisplayerConfig::default_for_type("fighter_hud").unwrap_or_default()
            }
            "synthwave" => {
                if let Some(val) = self.settings.get("synthwave_config") {
                    if let Some(cfg) = try_migrate(val, "synthwave", &self.id) {
                        return DisplayerConfig::Synthwave(cfg);
                    }
                }
                DisplayerConfig::default_for_type("synthwave").unwrap_or_default()
            }
            "art_deco" => {
                if let Some(val) = self.settings.get("art_deco_config") {
                    if let Some(cfg) = try_migrate(val, "art_deco", &self.id) {
                        return DisplayerConfig::ArtDeco(cfg);
                    }
                }
                DisplayerConfig::default_for_type("art_deco").unwrap_or_default()
            }
            "art_nouveau" => {
                if let Some(val) = self.settings.get("art_nouveau_config") {
                    if let Some(cfg) = try_migrate(val, "art_nouveau", &self.id) {
                        return DisplayerConfig::ArtNouveau(cfg);
                    }
                }
                DisplayerConfig::default_for_type("art_nouveau").unwrap_or_default()
            }
            "steampunk" => {
                if let Some(val) = self.settings.get("steampunk_config") {
                    if let Some(cfg) = try_migrate(val, "steampunk", &self.id) {
                        return DisplayerConfig::Steampunk(cfg);
                    }
                }
                DisplayerConfig::default_for_type("steampunk").unwrap_or_default()
            }
            "css_template" => {
                if let Some(val) = self.settings.get("css_template_config") {
                    if let Some(cfg) = try_migrate(val, "css_template", &self.id) {
                        return DisplayerConfig::CssTemplate(cfg);
                    }
                }
                DisplayerConfig::default_for_type("css_template").unwrap_or_default()
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

#[cfg(test)]
mod theme_library_tests {
    use super::*;
    use rg_sens_types::display_configs::themed_configs::LcarsDisplayConfig;
    use rg_sens_types::display_configs::BarDisplayConfig;
    use rg_sens_types::source_configs::cpu::CpuSourceConfig;
    use rg_sens_types::Color;

    fn theme(r: f64) -> ComboThemeConfig {
        ComboThemeConfig {
            color1: Color::new(r, 0.0, 0.0, 1.0),
            ..ComboThemeConfig::default()
        }
    }

    fn combo_panel(id: &str, frame_theme: ComboThemeConfig) -> PanelConfigV2 {
        let mut cfg = LcarsDisplayConfig::default();
        cfg.frame.theme = frame_theme;
        PanelConfigV2 {
            id: id.to_string(),
            geometry: PanelGeometry::default(),
            source_config: SourceConfig::Cpu(CpuSourceConfig::default()),
            displayer_config: DisplayerConfig::Lcars(cfg),
            appearance: PanelAppearance::default(),
            theme: ThemeRef::Inherit,
            group_id: None,
        }
    }

    fn bar_panel(id: &str) -> PanelConfigV2 {
        PanelConfigV2 {
            id: id.to_string(),
            geometry: PanelGeometry::default(),
            source_config: SourceConfig::Cpu(CpuSourceConfig::default()),
            displayer_config: DisplayerConfig::Bar(BarDisplayConfig::default()),
            appearance: PanelAppearance::default(),
            theme: ThemeRef::Inherit,
            group_id: None,
        }
    }

    fn config_with(panels: Vec<PanelConfigV2>) -> AppConfig {
        AppConfig {
            panels_v2: panels,
            ..AppConfig::default()
        }
    }

    /// Serialise through the save path and strip the `theme` key from the
    /// named panels, producing a file as an older binary would have written it.
    fn save_without_theme_keys(config: &AppConfig, strip_ids: &[&str]) -> String {
        let mut raw = serde_json::to_value(config.as_v2()).unwrap();
        for panel in raw["panels"].as_array_mut().unwrap() {
            let id = panel["id"].as_str().unwrap().to_string();
            if strip_ids.contains(&id.as_str()) {
                panel.as_object_mut().unwrap().remove("theme");
            }
        }
        serde_json::to_string(&raw).unwrap()
    }

    #[test]
    fn missing_theme_key_migrates_combo_to_inline_but_not_explicit_inherit() {
        // "old": written by a binary without the theme key; its frame theme
        // equals the defaults, which must still be preserved as Inline.
        // "new": a combo saved by this binary with an explicit Inherit.
        let old = combo_panel("old", ComboThemeConfig::default());
        let new = combo_panel("new", theme(0.5));
        let bar = bar_panel("bar");
        let config = config_with(vec![old, new, bar]);
        let json = save_without_theme_keys(&config, &["old", "bar"]);

        let loaded = AppConfig::load_from_string(&json).unwrap();
        let by_id = |id: &str| loaded.panels_v2.iter().find(|p| p.id == id).unwrap();

        assert_eq!(
            by_id("old").theme,
            ThemeRef::inline(ComboThemeConfig::default()),
            "combo without a theme key becomes Inline with its frame theme"
        );
        assert_eq!(
            by_id("new").theme,
            ThemeRef::Inherit,
            "an explicit Inherit written by this binary is left alone"
        );
        assert_eq!(
            by_id("bar").theme,
            ThemeRef::Inherit,
            "standalone panels have no frame theme and stay Inherit"
        );
    }

    #[test]
    fn combo_round_trip_preserves_frame_theme_exactly() {
        let config = config_with(vec![combo_panel("c", theme(0.42))]);
        let json = save_without_theme_keys(&config, &["c"]);
        let loaded = AppConfig::load_from_string(&json).unwrap();
        let saved_again = serde_json::to_string_pretty(&loaded.as_v2()).unwrap();
        let reloaded = AppConfig::load_from_string(&saved_again).unwrap();
        let frame_theme = reloaded.panels_v2[0]
            .displayer_config
            .theme()
            .unwrap()
            .clone();
        assert_eq!(frame_theme, theme(0.42));
        assert_eq!(reloaded.panels_v2[0].theme, ThemeRef::inline(theme(0.42)));
        assert_eq!(
            reloaded.resolve_theme_for_panel_id("c"),
            theme(0.42),
            "Inline resolves to the embedded theme regardless of global"
        );
    }

    #[test]
    fn themes_and_groups_round_trip() {
        let mut config = config_with(vec![bar_panel("a"), bar_panel("b")]);
        config.themes.insert("Blue".to_string(), theme(0.2));
        let gid = config.add_group("Left");
        config.group_mut(&gid).unwrap().theme = ThemeRef::named("Blue");
        config.set_panel_group("a", Some(&gid)).unwrap();

        let json = serde_json::to_string(&config.as_v2()).unwrap();
        let loaded = AppConfig::load_from_string(&json).unwrap();
        assert_eq!(loaded.themes.get("Blue"), Some(&theme(0.2)));
        assert_eq!(loaded.groups.len(), 1);
        assert_eq!(loaded.group_members(&gid), vec!["a".to_string()]);
        assert_eq!(loaded.resolve_theme_for_panel_id("a"), theme(0.2));
        assert_eq!(loaded.resolve_theme_for_panel_id("b"), loaded.global_theme);
    }

    #[test]
    fn sanitize_drops_dangling_group_id() {
        let mut config = config_with(vec![bar_panel("a")]);
        config.panels_v2[0].group_id = Some("ghost".to_string());
        config.sanitize();
        assert_eq!(config.panels_v2[0].group_id, None);
    }

    #[test]
    fn rename_rewrites_all_references() {
        let mut config = config_with(vec![bar_panel("a"), bar_panel("b")]);
        config.themes.insert("Old".to_string(), theme(0.3));
        let gid = config.add_group("G");
        config.group_mut(&gid).unwrap().theme = ThemeRef::named("Old");
        config.set_panel_theme("a", ThemeRef::named("Old")).unwrap();

        config.rename_theme("Old", "New").unwrap();
        assert!(config.themes.contains_key("New"));
        assert!(!config.themes.contains_key("Old"));
        assert_eq!(config.panels_v2[0].theme, ThemeRef::named("New"));
        assert_eq!(config.panels_v2[1].theme, ThemeRef::Inherit);
        assert_eq!(config.group(&gid).unwrap().theme, ThemeRef::named("New"));

        // Collisions and built-ins are refused
        config.themes.insert("Other".to_string(), theme(0.1));
        assert!(config.rename_theme("New", "Other").is_err());
        assert!(config.rename_theme("New", "LCARS").is_err());
        assert!(config.rename_theme("New", "  ").is_err());
    }

    #[test]
    fn delete_refused_while_in_use() {
        let mut config = config_with(vec![bar_panel("a")]);
        config.themes.insert("T".to_string(), theme(0.3));
        config.set_panel_theme("a", ThemeRef::named("T")).unwrap();
        assert!(config.delete_theme("T").is_err());
        config.set_panel_theme("a", ThemeRef::Inherit).unwrap();
        config.delete_theme("T").unwrap();
        assert!(
            config.delete_theme("LCARS").is_err(),
            "built-ins are not deletable"
        );
    }

    #[test]
    fn delete_group_ungroups_members() {
        let mut config = config_with(vec![bar_panel("a")]);
        let gid = config.add_group("G");
        config.set_panel_group("a", Some(&gid)).unwrap();
        config.delete_group(&gid);
        assert!(config.groups.is_empty());
        assert_eq!(config.panels_v2[0].group_id, None);
        assert!(config.set_panel_group("a", Some("missing")).is_err());
    }

    #[test]
    fn unique_theme_name_avoids_user_and_builtin_names() {
        let mut config = AppConfig::default();
        assert_eq!(config.unique_theme_name("Mine"), "Mine");
        config.themes.insert("Mine".to_string(), theme(0.1));
        assert_eq!(config.unique_theme_name("Mine"), "Mine 2");
        assert_eq!(config.unique_theme_name("LCARS"), "LCARS 2");
        assert!(config.set_theme("LCARS", theme(0.1)).is_err());
    }
}
