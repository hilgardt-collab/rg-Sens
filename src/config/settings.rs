//! Application and panel configuration

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::ui::background::BackgroundConfig;

/// Application-wide configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Version of the config format
    pub version: u32,
    /// Window dimensions
    pub window: WindowConfig,
    /// Grid settings
    pub grid: GridConfig,
    /// Panels configuration
    pub panels: Vec<PanelConfig>,
}

impl AppConfig {
    /// Load configuration from disk
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        if !config_path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(config_path)?;
        let config = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// Save configuration to disk
    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;

        // Ensure parent directory exists
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(config_path, content)?;
        Ok(())
    }

    /// Get the configuration file path
    fn config_path() -> Result<PathBuf> {
        let dirs = directories::ProjectDirs::from("com", "github.hilgardt_collab", "rg-sens")
            .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;

        Ok(dirs.config_dir().join("config.json"))
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: 1,
            window: WindowConfig::default(),
            grid: GridConfig::default(),
            panels: Vec::new(),
        }
    }
}

/// Window configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowConfig {
    pub width: i32,
    pub height: i32,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub background: BackgroundConfig,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            width: 800,
            height: 600,
            x: None,
            y: None,
            background: BackgroundConfig::default(),
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
            cell_width: 300,
            cell_height: 200,
            spacing: 4,
        }
    }
}

/// Panel configuration
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
    /// Custom settings
    pub settings: HashMap<String, serde_json::Value>,
}
