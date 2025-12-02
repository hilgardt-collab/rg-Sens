//! Grid layout manager for panels

use crate::core::{Panel, PanelGeometry};
use gtk4::gdk::ModifierType;
use gtk4::glib;
use gtk4::{prelude::*, EventControllerMotion, Fixed, GestureDrag, Widget};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Grid configuration
#[derive(Debug, Clone, Copy)]
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
/// Manages multiple panels arranged in a grid with drag-and-drop support.
pub struct GridLayout {
    config: GridConfig,
    container: Fixed,
    panels: Vec<Arc<RwLock<Panel>>>,
    widget_panel_map: Rc<RefCell<HashMap<String, (Widget, Arc<RwLock<Panel>>)>>>,
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
            widget_panel_map: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    /// Add a panel to the grid with drag-and-drop support
    pub fn add_panel(&mut self, panel: Arc<RwLock<Panel>>) {
        let panel_id = {
            let panel_guard = panel.blocking_read();
            panel_guard.id.clone()
        };

        // Get panel geometry
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

        // Add drag gesture for repositioning
        self.setup_drag_and_drop(&widget, panel.clone());

        // Add widget to container at position
        self.container.put(&widget, x as f64, y as f64);

        // Store widget-panel mapping
        self.widget_panel_map
            .borrow_mut()
            .insert(panel_id.clone(), (widget.clone(), panel.clone()));

        // Store panel reference
        self.panels.push(panel);
    }

    /// Setup drag-and-drop for a panel widget
    fn setup_drag_and_drop(&self, widget: &Widget, panel: Arc<RwLock<Panel>>) {
        let drag_gesture = GestureDrag::new();
        drag_gesture.set_button(1); // Left mouse button

        let config = self.config;
        let container = self.container.clone();
        let panel_clone = panel.clone();

        // Store initial position when drag starts
        let start_pos = Rc::new(RefCell::new((0.0, 0.0)));
        let start_pos_clone = start_pos.clone();

        drag_gesture.connect_drag_begin(move |_, x, y| {
            *start_pos_clone.borrow_mut() = (x, y);
        });

        let widget_clone = widget.clone();
        drag_gesture.connect_drag_update(move |_, offset_x, offset_y| {
            // Get current position of widget
            if let Some(parent) = widget_clone.parent() {
                if let Ok(fixed) = parent.downcast::<Fixed>() {
                    // Get original position
                    let (orig_x, orig_y) = fixed.child_position(&widget_clone);

                    // Calculate new position
                    let new_x = orig_x + offset_x;
                    let new_y = orig_y + offset_y;

                    // Move widget to new position (visual feedback during drag)
                    fixed.move_(&widget_clone, new_x, new_y);
                }
            }
        });

        let widget_clone2 = widget.clone();
        drag_gesture.connect_drag_end(move |_, offset_x, offset_y| {
            // Snap to grid on drag end
            if let Some(parent) = widget_clone2.parent() {
                if let Ok(fixed) = parent.downcast::<Fixed>() {
                    let (current_x, current_y) = fixed.child_position(&widget_clone2);

                    // Calculate grid position from pixel coordinates
                    let grid_x =
                        ((current_x + config.cell_width as f64 / 2.0)
                            / (config.cell_width + config.spacing) as f64)
                            .floor() as u32;
                    let grid_y =
                        ((current_y + config.cell_height as f64 / 2.0)
                            / (config.cell_height + config.spacing) as f64)
                            .floor() as u32;

                    // Clamp to grid bounds
                    let grid_x = grid_x.min(config.columns.saturating_sub(1));
                    let grid_y = grid_y.min(config.rows.saturating_sub(1));

                    // Calculate snapped pixel position
                    let snapped_x =
                        grid_x as f64 * (config.cell_width + config.spacing) as f64;
                    let snapped_y =
                        grid_y as f64 * (config.cell_height + config.spacing) as f64;

                    // Move widget to snapped position
                    fixed.move_(&widget_clone2, snapped_x, snapped_y);

                    // Update panel geometry
                    if let Ok(mut panel_guard) = panel_clone.try_write() {
                        panel_guard.geometry.x = grid_x;
                        panel_guard.geometry.y = grid_y;
                    }
                }
            }
        });

        widget.add_controller(drag_gesture);
    }

    /// Remove a panel by ID
    pub fn remove_panel(&mut self, panel_id: &str) -> Option<Arc<RwLock<Panel>>> {
        if let Some(pos) = self
            .panels
            .iter()
            .position(|p| p.blocking_read().id == panel_id)
        {
            let panel = self.panels.remove(pos);

            // Remove from widget map
            if let Some((widget, _)) = self.widget_panel_map.borrow_mut().remove(panel_id) {
                // Remove widget from container
                self.container.remove(&widget);
            }

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

        // Reposition all existing panels
        for (widget, panel) in self.widget_panel_map.borrow().values() {
            let geometry = panel.blocking_read().geometry;
            let x = geometry.x as f64 * (config.cell_width + config.spacing) as f64;
            let y = geometry.y as f64 * (config.cell_height + config.spacing) as f64;
            self.container.move_(widget, x, y);
        }
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
