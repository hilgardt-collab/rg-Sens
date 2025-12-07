//! Panel - container for a data source and displayer pair

use super::{BoxedDataSource, BoxedDisplayer};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::ui::{BackgroundConfig, Color};

/// Position and size of a panel in the grid
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PanelGeometry {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// Panel border configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelBorderConfig {
    pub enabled: bool,
    pub width: f64,
    pub color: Color,
}

impl Default for PanelBorderConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            width: 1.0,
            color: Color::new(1.0, 1.0, 1.0, 1.0), // White
        }
    }
}

/// A panel combines a data source and a displayer
pub struct Panel {
    /// Unique ID for this panel instance
    pub id: String,
    /// Geometry in the grid
    pub geometry: PanelGeometry,
    /// The data source
    pub source: BoxedDataSource,
    /// The displayer
    pub displayer: BoxedDisplayer,
    /// Custom configuration
    pub config: HashMap<String, serde_json::Value>,
    /// Background configuration
    pub background: BackgroundConfig,
    /// Corner radius for panel edges
    pub corner_radius: f64,
    /// Border configuration
    pub border: PanelBorderConfig,
}

impl Panel {
    /// Create a new panel
    pub fn new(
        id: String,
        geometry: PanelGeometry,
        source: BoxedDataSource,
        displayer: BoxedDisplayer,
    ) -> Self {
        Self {
            id,
            geometry,
            source,
            displayer,
            config: HashMap::new(),
            background: BackgroundConfig::default(),
            corner_radius: 8.0, // Default corner radius
            border: PanelBorderConfig::default(),
        }
    }

    /// Update the data source and refresh the displayer
    pub fn update(&mut self) -> Result<()> {
        // Update source
        self.source.update()?;

        // Get data and update displayer
        let data = self.source.get_values();
        self.displayer.update_data(&data);

        Ok(())
    }

    /// Apply configuration to the source and displayer
    pub fn apply_config(&mut self, config: HashMap<String, serde_json::Value>) -> Result<()> {
        self.config = config.clone();

        // Configure the data source (e.g., CPU source configuration)
        self.source.configure(&config)?;

        // Configure the displayer
        self.displayer.apply_config(&config)
    }
}
