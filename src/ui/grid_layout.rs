//! Grid layout manager for panels with advanced features

use crate::core::Panel;
use gtk4::gdk::{Key, ModifierType};
use gtk4::glib;
use gtk4::{prelude::*, DrawingArea, EventControllerKey, Fixed, Frame, GestureClick, GestureDrag, Overlay, PopoverMenu, Widget};
use log::info;
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
    is_dragging: Rc<RefCell<bool>>,
    selection_box: Rc<RefCell<Option<(f64, f64, f64, f64)>>>,
    on_change: Rc<RefCell<Option<Box<dyn Fn()>>>>,
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
            is_dragging: Rc::new(RefCell::new(false)),
            selection_box: Rc::new(RefCell::new(None)),
            on_change: Rc::new(RefCell::new(None)),
        };

        grid_layout.setup_drop_zone_drawing();
        grid_layout.setup_container_interaction();
        grid_layout
    }

    /// Set a callback to be called when panel positions change
    pub fn set_on_change<F>(&mut self, callback: F)
    where
        F: Fn() + 'static,
    {
        *self.on_change.borrow_mut() = Some(Box::new(callback));
    }

    /// Setup drop zone visualization
    fn setup_drop_zone_drawing(&self) {
        let config = self.config;
        let occupied_cells = self.occupied_cells.clone();
        let drag_preview_cell = self.drag_preview_cell.clone();
        let is_dragging = self.is_dragging.clone();
        let selection_box = self.selection_box.clone();

        self.drop_zone_layer.set_draw_func(move |_, cr, width, height| {
            let sel_box = selection_box.borrow();

            // Draw selection box if present
            if let Some((x1, y1, x2, y2)) = *sel_box {
                let rect_x = x1.min(x2);
                let rect_y = y1.min(y2);
                let rect_width = (x2 - x1).abs();
                let rect_height = (y2 - y1).abs();

                // Fill
                cr.set_source_rgba(0.2, 0.5, 0.8, 0.2);
                cr.rectangle(rect_x, rect_y, rect_width, rect_height);
                cr.fill().ok();

                // Border
                cr.set_source_rgba(0.2, 0.5, 0.8, 0.8);
                cr.set_line_width(2.0);
                cr.rectangle(rect_x, rect_y, rect_width, rect_height);
                cr.stroke().ok();
            }
            drop(sel_box);

            // Only draw grid when actively dragging panels
            if !*is_dragging.borrow() {
                return;
            }

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

    /// Setup container interaction for box selection and deselection
    fn setup_container_interaction(&self) {
        // Click on empty space to deselect all
        let click_gesture = GestureClick::new();
        click_gesture.set_button(1);

        let panel_states = self.panel_states.clone();
        let selected_panels = self.selected_panels.clone();

        click_gesture.connect_pressed(move |_, _, x, y| {
            // Check if click is on empty space (not on any panel)
            let states = panel_states.borrow();
            let mut clicked_on_panel = false;

            for state in states.values() {
                if let Some(parent) = state.frame.parent() {
                    if let Ok(fixed) = parent.downcast::<Fixed>() {
                        let (panel_x, panel_y) = fixed.child_position(&state.frame);
                        let panel_width = state.frame.width() as f64;
                        let panel_height = state.frame.height() as f64;

                        if x >= panel_x && x <= panel_x + panel_width
                            && y >= panel_y && y <= panel_y + panel_height {
                            clicked_on_panel = true;
                            break;
                        }
                    }
                }
            }

            // If click is on empty space, deselect all
            if !clicked_on_panel {
                let mut selected = selected_panels.borrow_mut();
                for (id, state) in states.iter() {
                    if selected.contains(id) {
                        state.frame.remove_css_class("selected");
                    }
                }
                selected.clear();
                info!("Deselected all panels");
            }
        });

        self.container.add_controller(click_gesture);

        // Drag from empty space for box selection
        let drag_gesture = GestureDrag::new();
        drag_gesture.set_button(1);

        let selection_box = self.selection_box.clone();
        let drop_zone_layer = self.drop_zone_layer.clone();
        let panel_states_drag = self.panel_states.clone();
        let selected_panels_drag = self.selected_panels.clone();

        // Store whether drag started on empty space
        let drag_on_empty_space: Rc<RefCell<bool>> = Rc::new(RefCell::new(false));
        let drag_start_pos: Rc<RefCell<Option<(f64, f64)>>> = Rc::new(RefCell::new(None));

        let drag_on_empty_space_begin = drag_on_empty_space.clone();
        let drag_start_pos_begin = drag_start_pos.clone();
        let panel_states_begin = panel_states_drag.clone();

        drag_gesture.connect_drag_begin(move |gesture, x, y| {
            // Check if drag started on empty space
            let states = panel_states_begin.borrow();
            let mut on_panel = false;

            for state in states.values() {
                if let Some(parent) = state.frame.parent() {
                    if let Ok(fixed) = parent.downcast::<Fixed>() {
                        let (panel_x, panel_y) = fixed.child_position(&state.frame);
                        let panel_width = state.frame.width() as f64;
                        let panel_height = state.frame.height() as f64;

                        if x >= panel_x && x <= panel_x + panel_width
                            && y >= panel_y && y <= panel_y + panel_height {
                            on_panel = true;
                            break;
                        }
                    }
                }
            }

            *drag_on_empty_space_begin.borrow_mut() = !on_panel;
            if !on_panel {
                *drag_start_pos_begin.borrow_mut() = Some((x, y));
            }
        });

        let drag_on_empty_space_update = drag_on_empty_space.clone();
        let drag_start_pos_update = drag_start_pos.clone();
        let selection_box_update = selection_box.clone();
        let drop_zone_update = drop_zone_layer.clone();

        drag_gesture.connect_drag_update(move |_, offset_x, offset_y| {
            if *drag_on_empty_space_update.borrow() {
                if let Some((start_x, start_y)) = *drag_start_pos_update.borrow() {
                    let end_x = start_x + offset_x;
                    let end_y = start_y + offset_y;

                    *selection_box_update.borrow_mut() = Some((start_x, start_y, end_x, end_y));
                    drop_zone_update.queue_draw();
                }
            }
        });

        let drag_on_empty_space_end = drag_on_empty_space.clone();
        let drag_start_pos_end = drag_start_pos.clone();

        drag_gesture.connect_drag_end(move |_, offset_x, offset_y| {
            if *drag_on_empty_space_end.borrow() {
                if let Some((start_x, start_y)) = *drag_start_pos_end.borrow() {
                    let end_x = start_x + offset_x;
                    let end_y = start_y + offset_y;

                    // Calculate selection rectangle
                    let rect_x1 = start_x.min(end_x);
                    let rect_y1 = start_y.min(end_y);
                    let rect_x2 = start_x.max(end_x);
                    let rect_y2 = start_y.max(end_y);

                    // Find panels that intersect with selection box
                    let mut states = panel_states_drag.borrow_mut();
                    let mut selected = selected_panels_drag.borrow_mut();

                    for (id, state) in states.iter_mut() {
                        if let Some(parent) = state.frame.parent() {
                            if let Ok(fixed) = parent.downcast::<Fixed>() {
                                let (panel_x, panel_y) = fixed.child_position(&state.frame);
                                let panel_width = state.frame.width() as f64;
                                let panel_height = state.frame.height() as f64;
                                let panel_x2 = panel_x + panel_width;
                                let panel_y2 = panel_y + panel_height;

                                // Check if rectangles intersect
                                let intersects = !(rect_x2 < panel_x || rect_x1 > panel_x2
                                    || rect_y2 < panel_y || rect_y1 > panel_y2);

                                if intersects {
                                    if !selected.contains(id) {
                                        selected.insert(id.clone());
                                        state.selected = true;
                                        state.frame.add_css_class("selected");
                                        info!("Selected panel: {}", id);
                                    }
                                }
                            }
                        }
                    }
                }

                // Clear selection box
                *selection_box.borrow_mut() = None;
                *drag_start_pos_end.borrow_mut() = None;
                drop_zone_layer.queue_draw();
            }

            *drag_on_empty_space_end.borrow_mut() = false;
        });

        self.container.add_controller(drag_gesture);
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

        // Right-click context menu
        let panel_clone = panel.clone();
        let panel_id_clone2 = panel_id.clone();

        self.setup_context_menu(widget, panel_clone, panel_id_clone2);

        // Drag gesture
        self.setup_drag_gesture(frame, panel);
    }

    /// Setup context menu for panel
    fn setup_context_menu(&self, widget: &Widget, panel: Arc<RwLock<Panel>>, panel_id: String) {
        use gtk4::gio;

        let menu = gio::Menu::new();

        // Properties option (combines configure + resize)
        menu.append(Some("Properties..."), Some("panel.properties"));

        menu.append(Some("---"), None);

        // Delete option
        menu.append(Some("Delete"), Some("panel.delete"));

        let popover = PopoverMenu::from_model(Some(&menu));
        popover.set_parent(widget);
        popover.set_has_arrow(false);

        // Setup action group for this panel
        let action_group = gio::SimpleActionGroup::new();

        // Properties action
        let panel_clone = panel.clone();
        let panel_id_clone = panel_id.clone();
        let config = self.config;
        let panel_states = self.panel_states.clone();
        let occupied_cells = self.occupied_cells.clone();
        let container = self.container.clone();
        let on_change = self.on_change.clone();
        let drop_zone = self.drop_zone_layer.clone();

        let properties_action = gio::SimpleAction::new("properties", None);
        properties_action.connect_activate(move |_, _| {
            info!("Opening properties dialog for panel: {}", panel_id_clone);
            let registry = crate::core::global_registry();
            show_panel_properties_dialog(
                &panel_clone,
                config,
                panel_states.clone(),
                occupied_cells.clone(),
                container.clone(),
                on_change.clone(),
                drop_zone.clone(),
                registry,
            );
        });
        action_group.add_action(&properties_action);

        // Delete action
        let panel_id_clone2 = panel_id.clone();
        let delete_action = gio::SimpleAction::new("delete", None);
        delete_action.connect_activate(move |_, _| {
            info!("Delete requested for panel: {}", panel_id_clone2);
            // TODO: Implement delete with confirmation
        });
        action_group.add_action(&delete_action);

        widget.insert_action_group("panel", Some(&action_group));

        // Right-click gesture
        let gesture_secondary = GestureClick::new();
        gesture_secondary.set_button(3); // Right mouse button

        gesture_secondary.connect_pressed(move |gesture, _, x, y| {
            popover.set_pointing_to(Some(&gtk4::gdk::Rectangle::new(
                x as i32,
                y as i32,
                1,
                1,
            )));
            popover.popup();
            gesture.set_state(gtk4::EventSequenceState::Claimed);
        });

        widget.add_controller(gesture_secondary);
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
        let is_dragging = self.is_dragging.clone();
        let drop_zone_layer = self.drop_zone_layer.clone();

        let panel_id = panel.blocking_read().id.clone();

        // Store initial positions
        let initial_positions: Rc<RefCell<HashMap<String, (f64, f64)>>> =
            Rc::new(RefCell::new(HashMap::new()));
        let initial_positions_clone = initial_positions.clone();

        // Clone for drag_begin closure
        let selected_panels_begin = selected_panels.clone();
        let panel_states_begin = panel_states.clone();
        let is_dragging_begin = is_dragging.clone();
        let drop_zone_begin = drop_zone_layer.clone();

        drag_gesture.connect_drag_begin(move |_, _, _| {
            // Enable grid visualization
            *is_dragging_begin.borrow_mut() = true;
            drop_zone_begin.queue_draw();

            // Ensure the dragged panel is in the selected set
            let mut selected = selected_panels_begin.borrow_mut();
            let mut states = panel_states_begin.borrow_mut();

            if !selected.contains(&panel_id) {
                // If dragging a non-selected panel, clear selection and select only this panel
                info!("Dragging non-selected panel {} - clearing other selections", panel_id);

                // Deselect all other panels
                for (id, state) in states.iter_mut() {
                    if selected.contains(id) {
                        state.selected = false;
                        state.frame.remove_css_class("selected");
                    }
                }
                selected.clear();

                // Select the dragged panel
                selected.insert(panel_id.clone());
                if let Some(state) = states.get_mut(&panel_id) {
                    state.selected = true;
                    state.frame.add_css_class("selected");
                }
            } else {
                info!("Dragging selected panel {} with {} total selected panels", panel_id, selected.len());
            }

            // Store initial positions of all selected panels
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
            let positions = initial_positions_clone2.borrow();

            // Don't move panels during drag - this causes a feedback loop!
            // Instead, calculate the preview position from the initial position + offset

            // Get the initial position of the dragged panel
            if let Some((orig_x, orig_y)) = positions.values().next() {
                // Calculate where the panel would be
                let new_x = orig_x + offset_x;
                let new_y = orig_y + offset_y;

                // Calculate grid position from the prospective location
                let grid_x = ((new_x + config.cell_width as f64 / 2.0)
                    / (config.cell_width + config.spacing) as f64)
                    .floor() as u32;
                let grid_y = ((new_y + config.cell_height as f64 / 2.0)
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
        });

        let panel_id_clone = panel_id.clone();

        // Clone for drag_end closure
        let selected_panels_end = selected_panels.clone();
        let panel_states_end = panel_states.clone();
        let occupied_cells_end = occupied_cells.clone();
        let drag_preview_cell_end = drag_preview_cell.clone();
        let is_dragging_end = is_dragging.clone();
        let drop_zone_layer_end = drop_zone_layer.clone();
        let on_change_end = self.on_change.clone();

        drag_gesture.connect_drag_end(move |_, offset_x, offset_y| {
            let selected = selected_panels_end.borrow();
            let states = panel_states_end.borrow();
            let mut occupied = occupied_cells_end.borrow_mut();
            let positions = initial_positions.borrow();

            // Phase 1: Clear current occupied cells for ALL selected panels
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

            // Phase 2: Calculate new positions for ALL selected panels and check for ANY collisions
            let mut new_positions: Vec<(String, u32, u32, f64, f64)> = Vec::new();
            let mut group_has_collision = false;

            // Get container dimensions for bounds checking
            let (available_cols, available_rows) = if let Some(state) = states.values().next() {
                if let Some(parent) = state.frame.parent() {
                    if let Ok(fixed) = parent.downcast::<Fixed>() {
                        let container_width = fixed.width() as f64;
                        let container_height = fixed.height() as f64;
                        (
                            (container_width / (config.cell_width + config.spacing) as f64).floor() as u32,
                            (container_height / (config.cell_height + config.spacing) as f64).floor() as u32,
                        )
                    } else {
                        (config.columns, config.rows)
                    }
                } else {
                    (config.columns, config.rows)
                }
            } else {
                (config.columns, config.rows)
            };

            // Calculate new positions and check collisions
            for id in selected.iter() {
                if let Some(state) = states.get(id) {
                    let (orig_x, orig_y) = positions.get(id).unwrap_or(&(0.0, 0.0));
                    let final_x = orig_x + offset_x;
                    let final_y = orig_y + offset_y;

                    // Calculate grid position from final position
                    let grid_x = ((final_x + config.cell_width as f64 / 2.0)
                        / (config.cell_width + config.spacing) as f64)
                        .floor() as u32;
                    let grid_y = ((final_y + config.cell_height as f64 / 2.0)
                        / (config.cell_height + config.spacing) as f64)
                        .floor() as u32;

                    let grid_x = grid_x.min(available_cols.saturating_sub(1));
                    let grid_y = grid_y.min(available_rows.saturating_sub(1));

                    // Check if this panel would collide
                    let geom = state.panel.blocking_read().geometry;
                    for dx in 0..geom.width {
                        for dy in 0..geom.height {
                            let cell = (grid_x + dx, grid_y + dy);
                            if occupied.contains(&cell) {
                                group_has_collision = true;
                                info!("Multi-panel drag collision detected at cell {:?} for panel {}", cell, id);
                                break;
                            }
                        }
                        if group_has_collision {
                            break;
                        }
                    }

                    // Calculate snapped pixel position
                    let snapped_x = grid_x as f64 * (config.cell_width + config.spacing) as f64;
                    let snapped_y = grid_y as f64 * (config.cell_height + config.spacing) as f64;

                    new_positions.push((id.clone(), grid_x, grid_y, snapped_x, snapped_y));
                }
            }

            // Phase 3: Apply movement based on collision check
            if group_has_collision {
                // Restore ALL panels to original positions
                info!("Collision detected - restoring all selected panels to original positions");
                for id in selected.iter() {
                    if let Some(state) = states.get(id) {
                        let geom = state.panel.blocking_read().geometry;

                        // Restore occupied cells
                        for dx in 0..geom.width {
                            for dy in 0..geom.height {
                                occupied.insert((geom.x + dx, geom.y + dy));
                            }
                        }
                    }
                }
            } else {
                // Move ALL panels to new positions
                info!("Moving {} selected panels together", new_positions.len());
                for (id, grid_x, grid_y, snapped_x, snapped_y) in new_positions {
                    if let Some(state) = states.get(&id) {
                        // Move widget
                        if let Some(parent) = state.frame.parent() {
                            if let Ok(fixed) = parent.downcast::<Fixed>() {
                                fixed.move_(&state.frame, snapped_x, snapped_y);
                            }
                        }

                        // Update geometry
                        if let Ok(mut panel_guard) = state.panel.try_write() {
                            panel_guard.geometry.x = grid_x;
                            panel_guard.geometry.y = grid_y;
                        }

                        // Mark new cells as occupied
                        let geom = state.panel.blocking_read().geometry;
                        for dx in 0..geom.width {
                            for dy in 0..geom.height {
                                occupied.insert((grid_x + dx, grid_y + dy));
                            }
                        }
                    }
                }
            }

            // Notify that panel positions have changed
            if let Some(callback) = on_change_end.borrow().as_ref() {
                callback();
            }

            // Disable grid visualization
            *is_dragging_end.borrow_mut() = false;

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

/// Show panel properties dialog
fn show_panel_properties_dialog(
    panel: &Arc<RwLock<Panel>>,
    config: GridConfig,
    panel_states: Rc<RefCell<HashMap<String, PanelState>>>,
    occupied_cells: Rc<RefCell<HashSet<(u32, u32)>>>,
    container: Fixed,
    on_change: Rc<RefCell<Option<Box<dyn Fn()>>>>,
    drop_zone: DrawingArea,
    registry: &'static mut crate::core::Registry,
) {
    use gtk4::{Box as GtkBox, Button, ComboBoxText, Label, Orientation, SpinButton, Window};

    let panel_guard = match panel.try_read() {
        Ok(guard) => guard,
        Err(_) => {
            log::warn!("Failed to lock panel for properties dialog");
            return;
        }
    };

    let panel_id = panel_guard.id.clone();
    let old_geometry = panel_guard.geometry;
    let old_source_id = panel_guard.source.metadata().id.clone();
    let old_displayer_id = panel_guard.displayer.id().to_string();

    // Create dialog window
    let dialog = Window::builder()
        .title(format!("Panel Properties - {}", panel_id))
        .modal(true)
        .default_width(400)
        .default_height(300)
        .build();

    // Main container
    let vbox = GtkBox::new(Orientation::Vertical, 12);
    vbox.set_margin_top(12);
    vbox.set_margin_bottom(12);
    vbox.set_margin_start(12);
    vbox.set_margin_end(12);

    // Panel Size section
    let size_label = Label::new(Some("Panel Size"));
    size_label.add_css_class("heading");
    vbox.append(&size_label);

    let size_box = GtkBox::new(Orientation::Horizontal, 6);
    size_box.set_margin_start(12);

    // Width control
    let width_label = Label::new(Some("Width:"));
    let width_spin = SpinButton::with_range(1.0, 10.0, 1.0);
    width_spin.set_value(old_geometry.width as f64);

    // Height control
    let height_label = Label::new(Some("Height:"));
    let height_spin = SpinButton::with_range(1.0, 10.0, 1.0);
    height_spin.set_value(old_geometry.height as f64);

    size_box.append(&width_label);
    size_box.append(&width_spin);
    size_box.append(&height_label);
    size_box.append(&height_spin);

    vbox.append(&size_box);

    // Data Source section
    let source_label = Label::new(Some("Data Source"));
    source_label.add_css_class("heading");
    source_label.set_margin_top(12);
    vbox.append(&source_label);

    let source_box = GtkBox::new(Orientation::Horizontal, 6);
    source_box.set_margin_start(12);

    let source_combo_label = Label::new(Some("Source:"));
    let source_combo = ComboBoxText::new();

    // Populate source combo
    let sources = registry.list_sources();
    let mut selected_source_idx = 0;
    for (idx, source_id) in sources.iter().enumerate() {
        source_combo.append_text(source_id);
        if source_id == &old_source_id {
            selected_source_idx = idx;
        }
    }
    source_combo.set_active(Some(selected_source_idx as u32));

    source_box.append(&source_combo_label);
    source_box.append(&source_combo);
    vbox.append(&source_box);

    // Displayer section
    let displayer_label = Label::new(Some("Display Type"));
    displayer_label.add_css_class("heading");
    displayer_label.set_margin_top(12);
    vbox.append(&displayer_label);

    let displayer_box = GtkBox::new(Orientation::Horizontal, 6);
    displayer_box.set_margin_start(12);

    let displayer_combo_label = Label::new(Some("Displayer:"));
    let displayer_combo = ComboBoxText::new();

    // Populate displayer combo
    let displayers = registry.list_displayers();
    let mut selected_displayer_idx = 0;
    for (idx, displayer_id) in displayers.iter().enumerate() {
        displayer_combo.append_text(displayer_id);
        if displayer_id == &old_displayer_id {
            selected_displayer_idx = idx;
        }
    }
    displayer_combo.set_active(Some(selected_displayer_idx as u32));

    displayer_box.append(&displayer_combo_label);
    displayer_box.append(&displayer_combo);
    vbox.append(&displayer_box);

    // Buttons
    let button_box = GtkBox::new(Orientation::Horizontal, 6);
    button_box.set_halign(gtk4::Align::End);
    button_box.set_margin_top(12);

    let cancel_button = Button::with_label("Cancel");
    let apply_button = Button::with_label("Apply");
    apply_button.add_css_class("suggested-action");

    let dialog_clone = dialog.clone();
    cancel_button.connect_clicked(move |_| {
        dialog_clone.close();
    });

    drop(panel_guard); // Release lock before closure

    let panel_clone = panel.clone();
    let dialog_clone2 = dialog.clone();
    apply_button.connect_clicked(move |_| {
        let new_width = width_spin.value() as u32;
        let new_height = height_spin.value() as u32;

        // Get selected source and displayer
        let new_source_id = source_combo.active_text()
            .map(|s| s.to_string())
            .unwrap_or(old_source_id.clone());
        let new_displayer_id = displayer_combo.active_text()
            .map(|s| s.to_string())
            .unwrap_or(old_displayer_id.clone());

        // Check if anything changed
        let size_changed = new_width != old_geometry.width || new_height != old_geometry.height;
        let source_changed = new_source_id != old_source_id;
        let displayer_changed = new_displayer_id != old_displayer_id;

        if !size_changed && !source_changed && !displayer_changed {
            info!("No changes made to panel, skipping update");
            dialog_clone2.close();
            return;
        }

        info!("Updating panel {}: size_changed={}, source_changed={}, displayer_changed={}",
              panel_id, size_changed, source_changed, displayer_changed);

        // Get panel state
        let mut states = panel_states.borrow_mut();
        let state = match states.get_mut(&panel_id) {
            Some(s) => s,
            None => {
                log::warn!("Panel state not found for {}", panel_id);
                dialog_clone2.close();
                return;
            }
        };

        // Handle size change (collision check)
        if size_changed {
            let mut occupied = occupied_cells.borrow_mut();

            // Clear old occupied cells
            for dx in 0..old_geometry.width {
                for dy in 0..old_geometry.height {
                    occupied.remove(&(old_geometry.x + dx, old_geometry.y + dy));
                }
            }

            // Check if new size would cause collision
            let mut has_collision = false;
            for dx in 0..new_width {
                for dy in 0..new_height {
                    let cell = (old_geometry.x + dx, old_geometry.y + dy);
                    if occupied.contains(&cell) {
                        has_collision = true;
                        info!("Collision detected at cell {:?}", cell);
                        break;
                    }
                }
                if has_collision {
                    break;
                }
            }

            if has_collision {
                // Restore old occupied cells
                for dx in 0..old_geometry.width {
                    for dy in 0..old_geometry.height {
                        occupied.insert((old_geometry.x + dx, old_geometry.y + dy));
                    }
                }
                drop(occupied);
                drop(states);

                log::warn!("Cannot resize panel: collision detected");

                // Show error dialog
                let error_dialog = gtk4::AlertDialog::builder()
                    .message("Cannot Resize Panel")
                    .detail("The new size would overlap with another panel.")
                    .modal(true)
                    .buttons(vec!["OK"])
                    .build();

                let dialog_ref = dialog_clone2.clone();
                error_dialog.choose(Some(&dialog_clone2), gtk4::gio::Cancellable::NONE, move |_| {
                    dialog_ref.present();
                });
                return;
            }

            // Mark new cells as occupied
            for dx in 0..new_width {
                for dy in 0..new_height {
                    occupied.insert((old_geometry.x + dx, old_geometry.y + dy));
                }
            }
        }

        // Update panel geometry, source, and displayer
        if let Ok(mut panel_guard) = panel_clone.try_write() {
            // Update size if changed
            if size_changed {
                panel_guard.geometry.width = new_width;
                panel_guard.geometry.height = new_height;
                info!("Updated panel geometry: {:?}", panel_guard.geometry);
            }

            // Update source if changed
            if source_changed {
                match registry.create_source(&new_source_id) {
                    Ok(new_source) => {
                        panel_guard.source = new_source;
                        info!("Updated panel source to: {}", new_source_id);
                    }
                    Err(e) => {
                        log::warn!("Failed to create source {}: {}", new_source_id, e);
                    }
                }
            }

            // Update displayer if changed
            if displayer_changed {
                match registry.create_displayer(&new_displayer_id) {
                    Ok(new_displayer) => {
                        // Create new widget from new displayer
                        let new_widget = new_displayer.create_widget();

                        // Calculate pixel dimensions
                        let pixel_width = panel_guard.geometry.width as i32 * config.cell_width
                            + (panel_guard.geometry.width as i32 - 1) * config.spacing;
                        let pixel_height = panel_guard.geometry.height as i32 * config.cell_height
                            + (panel_guard.geometry.height as i32 - 1) * config.spacing;
                        new_widget.set_size_request(pixel_width, pixel_height);

                        // Replace widget in frame
                        state.frame.set_child(Some(&new_widget));

                        // Update panel displayer
                        panel_guard.displayer = new_displayer;

                        // Update panel state widget reference
                        state.widget = new_widget;

                        info!("Updated panel displayer to: {}", new_displayer_id);
                    }
                    Err(e) => {
                        log::warn!("Failed to create displayer {}: {}", new_displayer_id, e);
                    }
                }
            }
        }

        // Update widget and frame sizes if size changed (and displayer wasn't replaced)
        if size_changed && !displayer_changed {
            let pixel_width = new_width as i32 * config.cell_width
                + (new_width as i32 - 1) * config.spacing;
            let pixel_height = new_height as i32 * config.cell_height
                + (new_height as i32 - 1) * config.spacing;

            state.widget.set_size_request(pixel_width, pixel_height);
            state.frame.set_size_request(pixel_width, pixel_height);

            info!("Resized panel widget to {}x{} pixels", pixel_width, pixel_height);
        }

        // Release panel_states borrow
        drop(states);

        // Trigger redraw of drop zone layer
        drop_zone.queue_draw();

        // Mark configuration as dirty
        if let Some(callback) = on_change.borrow().as_ref() {
            callback();
        }

        dialog_clone2.close();
    });

    button_box.append(&cancel_button);
    button_box.append(&apply_button);

    vbox.append(&button_box);

    dialog.set_child(Some(&vbox));
    dialog.present();
}

impl Default for GridLayout {
    fn default() -> Self {
        Self::new(GridConfig::default())
    }
}
