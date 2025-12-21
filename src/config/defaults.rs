//! Default configuration for new panels
//!
//! Stores default styles for displayers and general panel settings
//! that are applied when creating new panels.

use anyhow::Result;
use log::{info, warn};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::RwLock;

use crate::core::PanelBorderConfig;
use crate::ui::background::BackgroundConfig;

/// Global cached defaults configuration
static DEFAULTS_CACHE: Lazy<RwLock<DefaultsConfig>> = Lazy::new(|| {
    RwLock::new(DefaultsConfig::load_from_disk())
});

/// Current defaults format version
pub const DEFAULTS_VERSION: u32 = 1;

/// Default configuration for new panels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultsConfig {
    /// Version of the defaults format
    #[serde(default = "default_version")]
    pub version: u32,
    /// Displayer-specific defaults keyed by displayer ID (e.g., "lcars_combo", "bar", "text")
    #[serde(default)]
    pub displayer_defaults: HashMap<String, serde_json::Value>,
    /// General defaults for panel appearance and grid
    #[serde(default)]
    pub general: GeneralDefaults,
}

fn default_version() -> u32 {
    DEFAULTS_VERSION
}

/// General default settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralDefaults {
    /// Default panel width in grid cells
    #[serde(default = "default_panel_width")]
    pub default_panel_width: u32,
    /// Default panel height in grid cells
    #[serde(default = "default_panel_height")]
    pub default_panel_height: u32,
    /// Default corner radius for panels
    #[serde(default = "default_corner_radius")]
    pub default_corner_radius: f64,
    /// Default border configuration
    #[serde(default)]
    pub default_border: PanelBorderConfig,
    /// Default background configuration
    #[serde(default)]
    pub default_background: BackgroundConfig,
    /// Grid cell width in pixels
    #[serde(default = "default_grid_cell_width")]
    pub grid_cell_width: u32,
    /// Grid cell height in pixels
    #[serde(default = "default_grid_cell_height")]
    pub grid_cell_height: u32,
    /// Grid spacing in pixels
    #[serde(default = "default_grid_spacing")]
    pub grid_spacing: u32,
}

fn default_panel_width() -> u32 {
    2
}

fn default_panel_height() -> u32 {
    2
}

fn default_corner_radius() -> f64 {
    10.0
}

fn default_grid_cell_width() -> u32 {
    100
}

fn default_grid_cell_height() -> u32 {
    100
}

fn default_grid_spacing() -> u32 {
    5
}

impl Default for GeneralDefaults {
    fn default() -> Self {
        Self {
            default_panel_width: default_panel_width(),
            default_panel_height: default_panel_height(),
            default_corner_radius: default_corner_radius(),
            default_border: PanelBorderConfig::default(),
            default_background: BackgroundConfig::default(),
            grid_cell_width: default_grid_cell_width(),
            grid_cell_height: default_grid_cell_height(),
            grid_spacing: default_grid_spacing(),
        }
    }
}

impl Default for DefaultsConfig {
    fn default() -> Self {
        Self {
            version: DEFAULTS_VERSION,
            displayer_defaults: HashMap::new(),
            general: GeneralDefaults::default(),
        }
    }
}

impl DefaultsConfig {
    /// Load defaults from the global cache (fast, no disk I/O)
    pub fn load() -> Self {
        DEFAULTS_CACHE
            .read()
            .map(|guard| guard.clone())
            .unwrap_or_else(|_| {
                warn!("Failed to read defaults cache, using built-in defaults");
                Self::default()
            })
    }

    /// Load defaults directly from disk (used for initial cache population)
    fn load_from_disk() -> Self {
        match Self::try_load_from_disk() {
            Ok(config) => config,
            Err(e) => {
                warn!("Failed to load defaults.json, using built-in defaults: {}", e);
                Self::default()
            }
        }
    }

    /// Try to load defaults from disk
    fn try_load_from_disk() -> Result<Self> {
        let config_path = Self::config_path()?;

        if !config_path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(&config_path)?;
        let config: DefaultsConfig = serde_json::from_str(&content)?;
        info!("Loaded defaults from {:?}", config_path);
        Ok(config)
    }

    /// Save defaults to disk and update the global cache
    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;

        // Ensure parent directory exists
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(&config_path, content)?;
        info!("Saved defaults to {:?}", config_path);

        // Update the global cache
        if let Ok(mut cache) = DEFAULTS_CACHE.write() {
            *cache = self.clone();
        }

        Ok(())
    }

    /// Reload defaults from disk into the cache (use after external changes)
    pub fn reload_cache() {
        if let Ok(mut cache) = DEFAULTS_CACHE.write() {
            *cache = Self::load_from_disk();
        }
    }

    /// Get the defaults file path
    fn config_path() -> Result<PathBuf> {
        let dirs = directories::ProjectDirs::from("com", "github.hilgardt_collab", "rg-sens")
            .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;

        Ok(dirs.config_dir().join("defaults.json"))
    }

    /// Set a displayer default
    pub fn set_displayer_default(&mut self, displayer_id: &str, config: serde_json::Value) {
        self.displayer_defaults.insert(displayer_id.to_string(), config);
    }

    /// Get a displayer default if it exists
    pub fn get_displayer_default(&self, displayer_id: &str) -> Option<&serde_json::Value> {
        self.displayer_defaults.get(displayer_id)
    }

    /// Remove a displayer default
    pub fn remove_displayer_default(&mut self, displayer_id: &str) -> Option<serde_json::Value> {
        self.displayer_defaults.remove(displayer_id)
    }

    /// Clear all displayer defaults
    pub fn clear_displayer_defaults(&mut self) {
        self.displayer_defaults.clear();
    }

    /// Get list of displayer IDs that have defaults set
    pub fn get_displayer_default_ids(&self) -> Vec<String> {
        self.displayer_defaults.keys().cloned().collect()
    }
}
