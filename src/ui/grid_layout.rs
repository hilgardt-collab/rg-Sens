//! Grid layout manager for panels

use crate::core::{Panel, PanelGeometry};
use gtk4::{prelude::*, Fixed, Widget};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Grid configuration
#[derive(Debug, Clone)]
pub struct GridConfig {
    pub rows: u32,
    pub columns: u32,
    pub cell_width: i32,
    pub cell_height: i32,
    pub spacing: i32,
}

impl Default for GridConfig {
    fn default() -> Self {
        Self {
            rows: 3,
            columns: 4,
            cell_width: 200,
            cell_height: 150,
            spacing: 4,
        }
    }
}

/// Grid layout manager
///
/// Manages multiple panels arranged in a grid.
pub struct GridLayout {
    config: GridConfig,
    container: Fixed,
    panels: Vec<Arc<RwLock<Panel>>>,
}

impl GridLayout {
    /// Create a new grid layout
    pub fn new(config: GridConfig) -> Self {
        let container = Fixed::new();

        // Set the container size based on grid configuration
        let width = config.columns as i32 * (config.cell_width + config.spacing) - config.spacing;
        let height = config.rows as i32 * (config.cell_height + config.spacing) - config.spacing;
        container.set_size_request(width, height);

        Self {
            config,
            container,
            panels: Vec::new(),
        }
    }

    /// Add a panel to the grid
    ///
    /// The panel's geometry determines its position and size in grid cells.
    pub fn add_panel(&mut self, panel: Arc<RwLock<Panel>>) {
        // Get panel geometry and widget
        let geometry = {
            let panel_guard = panel.blocking_read();
            panel_guard.geometry
        };

        // Calculate pixel position from grid position
        let x = geometry.x as i32 * (self.config.cell_width + self.config.spacing);
        let y = geometry.y as i32 * (self.config.cell_height + self.config.spacing);

        // Calculate pixel size from grid size
        let width = geometry.width as i32 * self.config.cell_width
            + (geometry.width as i32 - 1) * self.config.spacing;
        let height = geometry.height as i32 * self.config.cell_height
            + (geometry.height as i32 - 1) * self.config.spacing;

        // Get the widget from the panel's displayer
        let widget = {
            let panel_guard = panel.blocking_read();
            panel_guard.displayer.create_widget()
        };

        // Set widget size
        widget.set_size_request(width, height);

        // Add widget to container at position
        self.container.put(&widget, x as f64, y as f64);

        // Store panel reference
        self.panels.push(panel);
    }

    /// Remove a panel by ID
    pub fn remove_panel(&mut self, panel_id: &str) -> Option<Arc<RwLock<Panel>>> {
        if let Some(pos) = self
            .panels
            .iter()
            .position(|p| p.blocking_read().id == panel_id)
        {
            let panel = self.panels.remove(pos);
            // Note: We should also remove the widget from the container,
            // but that requires tracking widget references
            Some(panel)
        } else {
            None
        }
    }

    /// Get all panels
    pub fn panels(&self) -> &[Arc<RwLock<Panel>>] {
        &self.panels
    }

    /// Get the GTK widget
    pub fn widget(&self) -> Widget {
        self.container.clone().upcast()
    }

    /// Update grid configuration
    pub fn set_config(&mut self, config: GridConfig) {
        self.config = config;

        // Update container size
        let width = config.columns as i32 * (config.cell_width + config.spacing) - config.spacing;
        let height = config.rows as i32 * (config.cell_height + config.spacing) - config.spacing;
        self.container.set_size_request(width, height);

        // Note: Should reposition all existing panels
        // For now, panels would need to be re-added after config change
    }

    /// Get current configuration
    pub fn config(&self) -> &GridConfig {
        &self.config
    }
}

impl Default for GridLayout {
    fn default() -> Self {
        Self::new(GridConfig::default())
    }
}
