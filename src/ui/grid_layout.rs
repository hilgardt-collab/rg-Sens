//! Grid layout manager for panels with advanced features

use crate::core::{Panel, PanelGeometry};
use gtk4::gdk::{Key, ModifierType};
use gtk4::glib;
use gtk4::{prelude::*, DrawingArea, EventControllerKey, Fixed, Frame, GestureDrag, Overlay, Widget};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
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

/// Panel state tracking
struct PanelState {
    widget: Widget,
    frame: Frame,
    panel: Arc<RwLock<Panel>>,
    selected: bool,
}

/// Grid layout manager
///
/// Manages multiple panels with drag-and-drop, collision detection, and multi-select.
pub struct GridLayout {
    config: GridConfig,
    overlay: Overlay,
    container: Fixed,
    drop_zone_layer: DrawingArea,
    panels: Vec<Arc<RwLock<Panel>>>,
    panel_states: Rc<RefCell<HashMap<String, PanelState>>>,
    selected_panels: Rc<RefCell<HashSet<String>>>,
    occupied_cells: Rc<RefCell<HashSet<(u32, u32)>>>,
    drag_preview_cell: Rc<RefCell<Option<(u32, u32)>>>,
}

impl GridLayout {
    /// Create a new grid layout
    pub fn new(config: GridConfig) -> Self {
        let overlay = Overlay::new();
        let container = Fixed::new();

        // Create drop zone visualization layer
        let drop_zone_layer = DrawingArea::new();
        drop_zone_layer.set_can_target(false); // Don't capture events

        // Set the container size
        let width = config.columns as i32 * (config.cell_width + config.spacing) - config.spacing;
        let height = config.rows as i32 * (config.cell_height + config.spacing) - config.spacing;
        container.set_size_request(width, height);
        drop_zone_layer.set_size_request(width, height);

        // Setup overlay layers
        overlay.set_child(Some(&container));
        overlay.add_overlay(&drop_zone_layer);

        let grid_layout = Self {
            config,
            overlay,
            container,
            drop_zone_layer,
            panels: Vec::new(),
            panel_states: Rc::new(RefCell::new(HashMap::new())),
            selected_panels: Rc::new(RefCell::new(HashSet::new())),
            occupied_cells: Rc::new(RefCell::new(HashSet::new())),
            drag_preview_cell: Rc::new(RefCell::new(None)),
        };

        grid_layout.setup_drop_zone_drawing();
        grid_layout
    }

    /// Setup drop zone visualization
    fn setup_drop_zone_drawing(&self) {
        let config = self.config;
        let occupied_cells = self.occupied_cells.clone();
        let drag_preview_cell = self.drag_preview_cell.clone();

        self.drop_zone_layer.set_draw_func(move |_, cr, width, height| {
            let occupied = occupied_cells.borrow();
            let preview = drag_preview_cell.borrow();

            // Calculate available columns and rows based on actual widget size
            let available_cols = (width as f64 / (config.cell_width + config.spacing) as f64).floor() as u32;
            let available_rows = (height as f64 / (config.cell_height + config.spacing) as f64).floor() as u32;

            // Draw grid lines
            cr.set_source_rgba(0.3, 0.3, 0.3, 0.3);
            cr.set_line_width(1.0);

            for col in 0..=available_cols {
                let x = col as f64 * (config.cell_width + config.spacing) as f64;
                cr.move_to(x, 0.0);
                cr.line_to(x, height as f64);
            }

            for row in 0..=available_rows {
                let y = row as f64 * (config.cell_height + config.spacing) as f64;
                cr.move_to(0.0, y);
                cr.line_to(width as f64, y);
            }
            cr.stroke().ok();

            // Highlight occupied cells in red
            for (cell_x, cell_y) in occupied.iter() {
                let x = *cell_x as f64 * (config.cell_width + config.spacing) as f64;
                let y = *cell_y as f64 * (config.cell_height + config.spacing) as f64;

                cr.set_source_rgba(1.0, 0.0, 0.0, 0.2);
                cr.rectangle(x, y, config.cell_width as f64, config.cell_height as f64);
                cr.fill().ok();
            }

            // Highlight drop preview in green/red
            if let Some((preview_x, preview_y)) = *preview {
                let x = preview_x as f64 * (config.cell_width + config.spacing) as f64;
                let y = preview_y as f64 * (config.cell_height + config.spacing) as f64;

                // Green if valid, red if collision
                let is_collision = occupied.contains(&(preview_x, preview_y));
                if is_collision {
                    cr.set_source_rgba(1.0, 0.0, 0.0, 0.4);
                } else {
                    cr.set_source_rgba(0.0, 1.0, 0.0, 0.4);
                }

                cr.rectangle(x, y, config.cell_width as f64, config.cell_height as f64);
                cr.fill().ok();

                // Border
                cr.set_source_rgba(1.0, 1.0, 1.0, 0.8);
                cr.set_line_width(2.0);
                cr.rectangle(x, y, config.cell_width as f64, config.cell_height as f64);
                cr.stroke().ok();
            }
        });
    }

    /// Add a panel to the grid
    pub fn add_panel(&mut self, panel: Arc<RwLock<Panel>>) {
        let panel_id = {
            let panel_guard = panel.blocking_read();
            panel_guard.id.clone()
        };

        let geometry = {
            let panel_guard = panel.blocking_read();
            panel_guard.geometry
        };

        // Calculate pixel position
        let x = geometry.x as i32 * (self.config.cell_width + self.config.spacing);
        let y = geometry.y as i32 * (self.config.cell_height + self.config.spacing);
        let width = geometry.width as i32 * self.config.cell_width
            + (geometry.width as i32 - 1) * self.config.spacing;
        let height = geometry.height as i32 * self.config.cell_height
            + (geometry.height as i32 - 1) * self.config.spacing;

        // Create widget
        let widget = {
            let panel_guard = panel.blocking_read();
            panel_guard.displayer.create_widget()
        };
        widget.set_size_request(width, height);

        // Create frame for selection visual feedback
        let frame = Frame::new(None);
        frame.set_child(Some(&widget));
        frame.set_size_request(width, height);

        // Setup drag-and-drop and selection
        self.setup_panel_interaction(&frame, &widget, panel.clone());

        // Add to container
        self.container.put(&frame, x as f64, y as f64);

        // Mark cells as occupied
        for dx in 0..geometry.width {
            for dy in 0..geometry.height {
                self.occupied_cells
                    .borrow_mut()
                    .insert((geometry.x + dx, geometry.y + dy));
            }
        }

        // Store panel state
        self.panel_states.borrow_mut().insert(
            panel_id.clone(),
            PanelState {
                widget: widget.clone(),
                frame: frame.clone(),
                panel: panel.clone(),
                selected: false,
            },
        );

        self.panels.push(panel);
    }

    /// Setup panel interaction (selection and drag)
    fn setup_panel_interaction(&self, frame: &Frame, widget: &Widget, panel: Arc<RwLock<Panel>>) {
        let panel_id = panel.blocking_read().id.clone();

        // Click for selection (Ctrl+Click for multi-select)
        let gesture_click = gtk4::GestureClick::new();
        let panel_states = self.panel_states.clone();
        let selected_panels = self.selected_panels.clone();
        let panel_id_clone = panel_id.clone();
        let frame_clone = frame.clone();

        gesture_click.connect_pressed(move |gesture, _, _, _| {
            let modifiers = gesture.current_event_state();
            let ctrl_pressed = modifiers.contains(ModifierType::CONTROL_MASK);

            let mut states = panel_states.borrow_mut();
            let mut selected = selected_panels.borrow_mut();

            if ctrl_pressed {
                // Toggle selection
                if selected.contains(&panel_id_clone) {
                    selected.remove(&panel_id_clone);
                    if let Some(state) = states.get_mut(&panel_id_clone) {
                        state.selected = false;
                        frame_clone.remove_css_class("selected");
                    }
                } else {
                    selected.insert(panel_id_clone.clone());
                    if let Some(state) = states.get_mut(&panel_id_clone) {
                        state.selected = true;
                        frame_clone.add_css_class("selected");
                    }
                }
            } else {
                // Clear other selections
                for (id, state) in states.iter_mut() {
                    if state.selected && id != &panel_id_clone {
                        state.selected = false;
                        state.frame.remove_css_class("selected");
                    }
                }
                selected.clear();

                // Select this panel
                selected.insert(panel_id_clone.clone());
                if let Some(state) = states.get_mut(&panel_id_clone) {
                    state.selected = true;
                    frame_clone.add_css_class("selected");
                }
            }
        });

        widget.add_controller(gesture_click);

        // Drag gesture
        self.setup_drag_gesture(frame, panel);
    }

    /// Setup drag gesture for a panel
    fn setup_drag_gesture(&self, frame: &Frame, panel: Arc<RwLock<Panel>>) {
        let drag_gesture = GestureDrag::new();
        drag_gesture.set_button(1);

        let config = self.config;
        let selected_panels = self.selected_panels.clone();
        let panel_states = self.panel_states.clone();
        let occupied_cells = self.occupied_cells.clone();
        let drag_preview_cell = self.drag_preview_cell.clone();
        let drop_zone_layer = self.drop_zone_layer.clone();

        let panel_id = panel.blocking_read().id.clone();

        // Store initial positions
        let initial_positions: Rc<RefCell<HashMap<String, (f64, f64)>>> =
            Rc::new(RefCell::new(HashMap::new()));
        let initial_positions_clone = initial_positions.clone();

        // Clone for drag_begin closure
        let selected_panels_begin = selected_panels.clone();
        let panel_states_begin = panel_states.clone();

        drag_gesture.connect_drag_begin(move |_, _, _| {
            // Store initial positions of all selected panels
            let selected = selected_panels_begin.borrow();
            let states = panel_states_begin.borrow();

            let mut positions = initial_positions_clone.borrow_mut();
            positions.clear();

            for id in selected.iter() {
                if let Some(state) = states.get(id) {
                    if let Some(parent) = state.frame.parent() {
                        if let Ok(fixed) = parent.downcast::<Fixed>() {
                            let pos = fixed.child_position(&state.frame);
                            positions.insert(id.clone(), pos);
                        }
                    }
                }
            }
        });

        let initial_positions_clone2 = initial_positions.clone();
        let frame_clone = frame.clone();

        // Clone for drag_update closure
        let selected_panels_update = selected_panels.clone();
        let panel_states_update = panel_states.clone();
        let drag_preview_cell_update = drag_preview_cell.clone();
        let drop_zone_layer_update = drop_zone_layer.clone();

        drag_gesture.connect_drag_update(move |_, offset_x, offset_y| {
            let selected = selected_panels_update.borrow();
            let states = panel_states_update.borrow();
            let positions = initial_positions_clone2.borrow();

            // Move all selected panels
            for id in selected.iter() {
                if let Some(state) = states.get(id) {
                    if let Some(parent) = state.frame.parent() {
                        if let Ok(fixed) = parent.downcast::<Fixed>() {
                            if let Some((orig_x, orig_y)) = positions.get(id) {
                                fixed.move_(&state.frame, orig_x + offset_x, orig_y + offset_y);
                            }
                        }
                    }
                }
            }

            // Update drop preview only if grid position changed
            if let Some(parent) = frame_clone.parent() {
                if let Ok(fixed) = parent.downcast::<Fixed>() {
                    let (current_x, current_y) = fixed.child_position(&frame_clone);
                    let grid_x = ((current_x + config.cell_width as f64 / 2.0)
                        / (config.cell_width + config.spacing) as f64)
                        .floor() as u32;
                    let grid_y = ((current_y + config.cell_height as f64 / 2.0)
                        / (config.cell_height + config.spacing) as f64)
                        .floor() as u32;

                    let new_preview = Some((grid_x, grid_y));
                    let mut preview_cell = drag_preview_cell_update.borrow_mut();

                    // Only update and redraw if the grid cell changed
                    if *preview_cell != new_preview {
                        *preview_cell = new_preview;
                        drop_zone_layer_update.queue_draw();
                    }
                }
            }
        });

        let panel_id_clone = panel_id.clone();

        // Clone for drag_end closure
        let selected_panels_end = selected_panels.clone();
        let panel_states_end = panel_states.clone();
        let occupied_cells_end = occupied_cells.clone();
        let drag_preview_cell_end = drag_preview_cell.clone();
        let drop_zone_layer_end = drop_zone_layer.clone();

        drag_gesture.connect_drag_end(move |_, _, _| {
            let selected = selected_panels_end.borrow();
            let states = panel_states_end.borrow();
            let mut occupied = occupied_cells_end.borrow_mut();

            // Clear current occupied cells for selected panels
            for id in selected.iter() {
                if let Some(state) = states.get(id) {
                    let geom = state.panel.blocking_read().geometry;
                    for dx in 0..geom.width {
                        for dy in 0..geom.height {
                            occupied.remove(&(geom.x + dx, geom.y + dy));
                        }
                    }
                }
            }

            // Snap all selected panels
            for id in selected.iter() {
                if let Some(state) = states.get(id) {
                    if let Some(parent) = state.frame.parent() {
                        if let Ok(fixed) = parent.downcast::<Fixed>() {
                            let (current_x, current_y) = fixed.child_position(&state.frame);

                            // Calculate available grid size based on container size
                            let container_width = fixed.width() as f64;
                            let container_height = fixed.height() as f64;
                            let available_cols = (container_width / (config.cell_width + config.spacing) as f64).floor() as u32;
                            let available_rows = (container_height / (config.cell_height + config.spacing) as f64).floor() as u32;

                            // Calculate grid position
                            let grid_x = ((current_x + config.cell_width as f64 / 2.0)
                                / (config.cell_width + config.spacing) as f64)
                                .floor() as u32;
                            let grid_y = ((current_y + config.cell_height as f64 / 2.0)
                                / (config.cell_height + config.spacing) as f64)
                                .floor() as u32;

                            let grid_x = grid_x.min(available_cols.saturating_sub(1));
                            let grid_y = grid_y.min(available_rows.saturating_sub(1));

                            // Check collision
                            let geom = state.panel.blocking_read().geometry;
                            let mut has_collision = false;
                            for dx in 0..geom.width {
                                for dy in 0..geom.height {
                                    if occupied.contains(&(grid_x + dx, grid_y + dy)) {
                                        has_collision = true;
                                    }
                                }
                            }

                            // Only move if no collision
                            if !has_collision {
                                // Snap to grid
                                let snapped_x =
                                    grid_x as f64 * (config.cell_width + config.spacing) as f64;
                                let snapped_y =
                                    grid_y as f64 * (config.cell_height + config.spacing) as f64;
                                fixed.move_(&state.frame, snapped_x, snapped_y);

                                // Update geometry
                                if let Ok(mut panel_guard) = state.panel.try_write() {
                                    panel_guard.geometry.x = grid_x;
                                    panel_guard.geometry.y = grid_y;
                                }

                                // Mark new cells as occupied
                                for dx in 0..geom.width {
                                    for dy in 0..geom.height {
                                        occupied.insert((grid_x + dx, grid_y + dy));
                                    }
                                }
                            } else {
                                // Collision! Snap back to original position
                                let orig_x =
                                    geom.x as f64 * (config.cell_width + config.spacing) as f64;
                                let orig_y =
                                    geom.y as f64 * (config.cell_height + config.spacing) as f64;
                                fixed.move_(&state.frame, orig_x, orig_y);

                                // Restore occupied cells
                                for dx in 0..geom.width {
                                    for dy in 0..geom.height {
                                        occupied.insert((geom.x + dx, geom.y + dy));
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Clear drop preview
            *drag_preview_cell_end.borrow_mut() = None;
            drop_zone_layer_end.queue_draw();
        });

        frame.add_controller(drag_gesture);
    }

    /// Remove a panel by ID
    pub fn remove_panel(&mut self, panel_id: &str) -> Option<Arc<RwLock<Panel>>> {
        // Remove from panels list
        if let Some(pos) = self
            .panels
            .iter()
            .position(|p| p.blocking_read().id == panel_id)
        {
            let panel = self.panels.remove(pos);

            // Remove from states and container
            if let Some(state) = self.panel_states.borrow_mut().remove(panel_id) {
                self.container.remove(&state.frame);

                // Clear occupied cells
                let geom = state.panel.blocking_read().geometry;
                let mut occupied = self.occupied_cells.borrow_mut();
                for dx in 0..geom.width {
                    for dy in 0..geom.height {
                        occupied.remove(&(geom.x + dx, geom.y + dy));
                    }
                }
            }

            // Remove from selection
            self.selected_panels.borrow_mut().remove(panel_id);

            Some(panel)
        } else {
            None
        }
    }

    pub fn panels(&self) -> &[Arc<RwLock<Panel>>] {
        &self.panels
    }

    pub fn widget(&self) -> Widget {
        self.overlay.clone().upcast()
    }

    pub fn set_config(&mut self, config: GridConfig) {
        self.config = config;
        let width = config.columns as i32 * (config.cell_width + config.spacing) - config.spacing;
        let height = config.rows as i32 * (config.cell_height + config.spacing) - config.spacing;
        self.container.set_size_request(width, height);
        self.drop_zone_layer.set_size_request(width, height);
    }

    pub fn config(&self) -> &GridConfig {
        &self.config
    }
}

impl Default for GridLayout {
    fn default() -> Self {
        Self::new(GridConfig::default())
    }
}
