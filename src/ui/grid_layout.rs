//! Grid layout manager for panels with advanced features

use crate::core::Panel;
use gtk4::gdk::ModifierType;
use gtk4::{prelude::*, DrawingArea, Fixed, Frame, GestureClick, GestureDrag, Overlay, PopoverMenu, Widget};
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
    background_area: DrawingArea,
}

/// Grid layout manager
///
/// Manages multiple panels with drag-and-drop, collision detection, and multi-select.
pub struct GridLayout {
    config: Rc<RefCell<GridConfig>>,
    overlay: Overlay,
    container: Fixed,
    drop_zone_layer: DrawingArea,
    panels: Rc<RefCell<Vec<Arc<RwLock<Panel>>>>>,
    panel_states: Rc<RefCell<HashMap<String, PanelState>>>,
    selected_panels: Rc<RefCell<HashSet<String>>>,
    occupied_cells: Rc<RefCell<HashSet<(u32, u32)>>>,
    drag_preview_cells: Rc<RefCell<Vec<(u32, u32, u32, u32)>>>, // (x, y, width, height) for each panel
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

        // Wrap config in Rc<RefCell<>> for shared mutable access
        let config = Rc::new(RefCell::new(config));

        // Set the container size
        let config_borrow = config.borrow();
        let width = config_borrow.columns as i32 * (config_borrow.cell_width + config_borrow.spacing) - config_borrow.spacing;
        let height = config_borrow.rows as i32 * (config_borrow.cell_height + config_borrow.spacing) - config_borrow.spacing;
        drop(config_borrow); // Drop borrow before moving config

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
            panels: Rc::new(RefCell::new(Vec::new())),
            panel_states: Rc::new(RefCell::new(HashMap::new())),
            selected_panels: Rc::new(RefCell::new(HashSet::new())),
            occupied_cells: Rc::new(RefCell::new(HashSet::new())),
            drag_preview_cells: Rc::new(RefCell::new(Vec::new())),
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

    /// Get the list of panels
    pub fn get_panels(&self) -> Vec<Arc<RwLock<Panel>>> {
        self.panels.borrow().clone()
    }

    /// Setup drop zone visualization
    fn setup_drop_zone_drawing(&self) {
        let config = self.config.clone();
        let occupied_cells = self.occupied_cells.clone();
        let drag_preview_cells = self.drag_preview_cells.clone();
        let is_dragging = self.is_dragging.clone();
        let selection_box = self.selection_box.clone();

        self.drop_zone_layer.set_draw_func(move |_, cr, width, height| {
            let config = config.borrow();
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
            let preview_panels = drag_preview_cells.borrow();

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

            // Highlight drop preview for all selected panels
            for (preview_x, preview_y, panel_width, panel_height) in preview_panels.iter() {
                let x = *preview_x as f64 * (config.cell_width + config.spacing) as f64;
                let y = *preview_y as f64 * (config.cell_height + config.spacing) as f64;
                let rect_width = *panel_width as f64 * config.cell_width as f64
                    + (*panel_width as f64 - 1.0) * config.spacing as f64;
                let rect_height = *panel_height as f64 * config.cell_height as f64
                    + (*panel_height as f64 - 1.0) * config.spacing as f64;

                // Check if any cell in this panel would collide
                let mut has_collision = false;
                for dx in 0..*panel_width {
                    for dy in 0..*panel_height {
                        if occupied.contains(&(preview_x + dx, preview_y + dy)) {
                            has_collision = true;
                            break;
                        }
                    }
                    if has_collision {
                        break;
                    }
                }

                // Green if valid, red if collision
                if has_collision {
                    cr.set_source_rgba(1.0, 0.0, 0.0, 0.4);
                } else {
                    cr.set_source_rgba(0.0, 1.0, 0.0, 0.4);
                }

                cr.rectangle(x, y, rect_width, rect_height);
                cr.fill().ok();

                // Border
                cr.set_source_rgba(1.0, 1.0, 1.0, 0.8);
                cr.set_line_width(2.0);
                cr.rectangle(x, y, rect_width, rect_height);
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

        drag_gesture.connect_drag_begin(move |_gesture, x, y| {
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
        let config = self.config.borrow();
        let x = geometry.x as i32 * (config.cell_width + config.spacing);
        let y = geometry.y as i32 * (config.cell_height + config.spacing);
        let width = geometry.width as i32 * config.cell_width
            + (geometry.width as i32 - 1) * config.spacing;
        let height = geometry.height as i32 * config.cell_height
            + (geometry.height as i32 - 1) * config.spacing;
        drop(config);

        // Create displayer widget
        let widget = {
            let panel_guard = panel.blocking_read();
            panel_guard.displayer.create_widget()
        };
        widget.set_size_request(width, height);

        // Create background drawing area
        let background_area = DrawingArea::new();
        background_area.set_size_request(width, height);

        // Setup background rendering
        let panel_clone_bg = panel.clone();
        background_area.set_draw_func(move |_, cr, w, h| {
            match panel_clone_bg.try_read() {
                Ok(panel_guard) => {
                    if let Err(e) = crate::ui::render_background(cr, &panel_guard.background, w as f64, h as f64) {
                        log::warn!("Failed to render background: {}", e);
                    }
                }
                Err(_) => {
                    log::warn!("Failed to acquire panel read lock for background rendering");
                }
            }
        });

        // Create overlay to stack background and widget
        let overlay = Overlay::new();
        overlay.set_child(Some(&background_area));

        // Make the widget transparent so the background shows through
        widget.add_css_class("transparent-background");
        overlay.add_overlay(&widget);

        // Create frame for selection visual feedback
        let frame = Frame::new(None);
        frame.set_child(Some(&overlay));
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
                background_area: background_area.clone(),
            },
        );

        self.panels.borrow_mut().push(panel);
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
                // If clicking on an already-selected panel that's part of a multi-selection,
                // keep the current selection (to allow dragging multiple panels)
                // Otherwise, clear other selections and select only this panel
                if !selected.contains(&panel_id_clone) || selected.len() == 1 {
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
                // else: panel is already selected in a multi-selection, keep it as-is
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

        // Section 1: Properties
        let section1 = gio::Menu::new();
        section1.append(Some("Properties..."), Some("panel.properties"));
        menu.append_section(None, &section1);

        // Section 2: Delete
        let section2 = gio::Menu::new();
        section2.append(Some("Delete"), Some("panel.delete"));
        menu.append_section(None, &section2);

        let popover = PopoverMenu::from_model(Some(&menu));
        popover.set_parent(widget);
        popover.set_has_arrow(false);

        // Setup action group for this panel
        let action_group = gio::SimpleActionGroup::new();

        // Properties action
        let panel_clone = panel.clone();
        let panel_id_clone = panel_id.clone();
        let config = self.config.clone();
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
                *config.borrow(),
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
        let panel_clone2 = panel.clone();
        let panel_states_clone = self.panel_states.clone();
        let occupied_cells_clone = self.occupied_cells.clone();
        let container_clone = self.container.clone();
        let on_change_clone = self.on_change.clone();
        let panels_clone = self.panels.clone();
        let delete_action = gio::SimpleAction::new("delete", None);
        delete_action.connect_activate(move |_, _| {
            info!("Delete requested for panel: {}", panel_id_clone2);

            // Get panel geometry before deletion
            let geometry = {
                let panel_guard = panel_clone2.blocking_read();
                panel_guard.geometry
            };

            // Show confirmation dialog
            use gtk4::AlertDialog;
            let dialog = AlertDialog::builder()
                .message("Delete Panel?")
                .detail("This action cannot be undone.")
                .modal(true)
                .buttons(vec!["Cancel", "Delete"])
                .default_button(0) // "Cancel" button
                .cancel_button(0) // "Cancel" button
                .build();

            let panel_id_for_delete = panel_id_clone2.clone();
            let panel_states_for_delete = panel_states_clone.clone();
            let occupied_cells_for_delete = occupied_cells_clone.clone();
            let container_for_delete = container_clone.clone();
            let on_change_for_delete = on_change_clone.clone();
            let panels_for_delete = panels_clone.clone();

            // We need a parent window for the dialog - get it from the container
            if let Some(root) = container_clone.root() {
                if let Some(window) = root.downcast_ref::<gtk4::Window>() {
                    dialog.choose(Some(window), gtk4::gio::Cancellable::NONE, move |response| {
                        if let Ok(1) = response {
                            // Delete button clicked (index 1)
                            info!("Deleting panel: {}", panel_id_for_delete);

                            // Remove from panel_states and get the frame widget
                            if let Some(state) = panel_states_for_delete.borrow_mut().remove(&panel_id_for_delete) {
                                // Remove widget from container
                                container_for_delete.remove(&state.frame);

                                // Free occupied cells
                                for dx in 0..geometry.width {
                                    for dy in 0..geometry.height {
                                        occupied_cells_for_delete
                                            .borrow_mut()
                                            .remove(&(geometry.x + dx, geometry.y + dy));
                                    }
                                }

                                // Remove from panels list
                                panels_for_delete.borrow_mut().retain(|p| {
                                    let p_guard = p.blocking_read();
                                    p_guard.id != panel_id_for_delete
                                });

                                // Trigger on_change callback to mark config as dirty
                                if let Some(ref callback) = *on_change_for_delete.borrow() {
                                    callback();
                                }

                                info!("Panel deleted successfully: {}", panel_id_for_delete);
                            } else {
                                log::warn!("Panel not found in states: {}", panel_id_for_delete);
                            }
                        }
                    });
                }
            }
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

        let config = self.config.clone();
        let selected_panels = self.selected_panels.clone();
        let panel_states = self.panel_states.clone();
        let occupied_cells = self.occupied_cells.clone();
        let drag_preview_cells = self.drag_preview_cells.clone();
        let is_dragging = self.is_dragging.clone();
        let drop_zone_layer = self.drop_zone_layer.clone();

        let panel_id = panel.blocking_read().id.clone();

        // Store initial positions and the ID of the panel being dragged
        let initial_positions: Rc<RefCell<HashMap<String, (f64, f64)>>> =
            Rc::new(RefCell::new(HashMap::new()));
        let initial_positions_clone = initial_positions.clone();
        let dragged_panel_id: Rc<RefCell<String>> = Rc::new(RefCell::new(String::new()));

        // Clone for drag_begin closure
        let selected_panels_begin = selected_panels.clone();
        let panel_states_begin = panel_states.clone();
        let is_dragging_begin = is_dragging.clone();
        let drop_zone_begin = drop_zone_layer.clone();
        let panel_id_for_drag_begin = panel_id.clone();
        let dragged_panel_id_begin = dragged_panel_id.clone();

        drag_gesture.connect_drag_begin(move |_, _, _| {
            // Enable grid visualization
            *is_dragging_begin.borrow_mut() = true;
            drop_zone_begin.queue_draw();

            // Store which panel is being dragged
            *dragged_panel_id_begin.borrow_mut() = panel_id_for_drag_begin.clone();

            // Ensure the dragged panel is in the selected set
            let mut selected = selected_panels_begin.borrow_mut();
            let mut states = panel_states_begin.borrow_mut();

            if !selected.contains(&panel_id_for_drag_begin) {
                // If dragging a non-selected panel, clear selection and select only this panel

                // Deselect all other panels
                for (id, state) in states.iter_mut() {
                    if selected.contains(id) {
                        state.selected = false;
                        state.frame.remove_css_class("selected");
                    }
                }
                selected.clear();

                // Select the dragged panel
                selected.insert(panel_id_for_drag_begin.clone());
                if let Some(state) = states.get_mut(&panel_id_for_drag_begin) {
                    state.selected = true;
                    state.frame.add_css_class("selected");
                }
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
        let _frame_clone = frame.clone();

        // Clone for drag_update closure
        let config_for_update = config.clone();
        let selected_panels_update = selected_panels.clone();
        let panel_states_update = panel_states.clone();
        let drag_preview_cells_update = drag_preview_cells.clone();
        let drop_zone_layer_update = drop_zone_layer.clone();
        let dragged_panel_id_update = dragged_panel_id.clone();

        drag_gesture.connect_drag_update(move |_, offset_x, offset_y| {
            let config = config_for_update.borrow();
            let positions = initial_positions_clone2.borrow();
            let selected = selected_panels_update.borrow();
            let states = panel_states_update.borrow();
            let dragged_id = dragged_panel_id_update.borrow();

            // Calculate preview positions for ALL selected panels
            let mut preview_rects = Vec::new();

            // Get the dragged panel's initial position to use as reference
            if let Some((dragged_orig_x, dragged_orig_y)) = positions.get(&*dragged_id) {
                // Calculate where the dragged panel would be
                let dragged_new_x = dragged_orig_x + offset_x;
                let dragged_new_y = dragged_orig_y + offset_y;

                // Calculate grid position of dragged panel
                let dragged_grid_x = ((dragged_new_x + config.cell_width as f64 / 2.0)
                    / (config.cell_width + config.spacing) as f64)
                    .floor() as i32;
                let dragged_grid_y = ((dragged_new_y + config.cell_height as f64 / 2.0)
                    / (config.cell_height + config.spacing) as f64)
                    .floor() as i32;

                // Get the dragged panel's original grid position
                let dragged_panel_orig_grid = if let Some(state) = states.get(&*dragged_id) {
                    let geom = state.panel.blocking_read().geometry;
                    (geom.x as i32, geom.y as i32)
                } else {
                    (0, 0)
                };

                // Calculate the grid offset from original position
                let grid_offset_x = dragged_grid_x - dragged_panel_orig_grid.0;
                let grid_offset_y = dragged_grid_y - dragged_panel_orig_grid.1;

                // Apply this offset to all selected panels
                for id in selected.iter() {
                    if let Some(state) = states.get(id) {
                        let geom = state.panel.blocking_read().geometry;
                        log::debug!("[DRAG] Panel {} drag preview using geometry {}x{} at ({},{})",
                                   id, geom.width, geom.height, geom.x, geom.y);

                        // Calculate new grid position
                        let new_grid_x = (geom.x as i32 + grid_offset_x).max(0) as u32;
                        let new_grid_y = (geom.y as i32 + grid_offset_y).max(0) as u32;

                        preview_rects.push((new_grid_x, new_grid_y, geom.width, geom.height));
                    }
                }
            }

            // Update preview and redraw
            let mut preview_cells = drag_preview_cells_update.borrow_mut();
            if *preview_cells != preview_rects {
                *preview_cells = preview_rects;
                drop(preview_cells);
                drop_zone_layer_update.queue_draw();
            }
        });

        let _panel_id_clone = panel_id.clone();

        // Clone for drag_end closure
        let config_for_end = config.clone();
        let selected_panels_end = selected_panels.clone();
        let panel_states_end = panel_states.clone();
        let occupied_cells_end = occupied_cells.clone();
        let drag_preview_cells_end = drag_preview_cells.clone();
        let is_dragging_end = is_dragging.clone();
        let drop_zone_layer_end = drop_zone_layer.clone();
        let on_change_end = self.on_change.clone();
        let container_for_copy = self.container.clone();
        let panels_for_copy = self.panels.clone();

        drag_gesture.connect_drag_end(move |gesture, offset_x, offset_y| {
            let config = config_for_end.borrow();
            let selected = selected_panels_end.borrow();
            let states = panel_states_end.borrow();
            let mut occupied = occupied_cells_end.borrow_mut();
            let positions = initial_positions.borrow();

            // Check if Ctrl key is held (copy mode)
            let modifiers = gesture.current_event_state();
            let is_copy_mode = modifiers.contains(gtk4::gdk::ModifierType::CONTROL_MASK);

            // Phase 1: Clear current occupied cells for ALL selected panels (only if moving, not copying)
            if !is_copy_mode {
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

            // Phase 3: Apply movement/copy based on collision check
            if group_has_collision {
                // Restore ALL panels to original positions (only needed in move mode)
                if !is_copy_mode {
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
                }
            } else if is_copy_mode {
                // COPY MODE: Create duplicates of panels at new positions
                drop(states); // Release borrow before creating new panels
                drop(selected);
                drop(occupied);

                use crate::core::Panel;

                for (old_id, grid_x, grid_y, _snapped_x, _snapped_y) in new_positions {
                    // Get the original panel to copy from
                    let panel_states_read = panel_states_end.borrow();
                    if let Some(state) = panel_states_read.get(&old_id) {
                        let original_panel = state.panel.clone();
                        drop(panel_states_read);

                        // Read original panel data
                        let (source_meta, displayer_id, config, background, geometry_size) = {
                            let panel_guard = original_panel.blocking_read();
                            (
                                panel_guard.source.metadata().clone(),
                                panel_guard.displayer.id().to_string(),
                                panel_guard.config.clone(),
                                panel_guard.background.clone(),
                                (panel_guard.geometry.width, panel_guard.geometry.height),
                            )
                        };

                        // Generate unique ID for the copy
                        let new_id = format!("panel_{}", uuid::Uuid::new_v4());

                        // Create new panel with copied configuration
                        let registry = crate::core::global_registry();
                        if let Some(source_factory) = registry.get_source(&source_meta.id) {
                            if let Some(displayer_factory) = registry.get_displayer(&displayer_id) {
                                let new_panel = Panel::new(
                                    new_id.clone(),
                                    grid_x,
                                    grid_y,
                                    geometry_size.0,
                                    geometry_size.1,
                                    source_factory(),
                                    displayer_factory(),
                                    background,
                                );

                                let new_panel = Arc::new(RwLock::new(new_panel));

                                // Apply the copied configuration
                                if let Ok(mut new_panel_guard) = new_panel.try_write() {
                                    let _ = new_panel_guard.apply_config(config);
                                }

                                // Add the copied panel to the grid
                                // Add to panels list
                                panels_for_copy.borrow_mut().push(new_panel.clone());

                                // Mark new cells as occupied
                                let mut occupied_write = occupied_cells_end.borrow_mut();
                                for dx in 0..geometry_size.0 {
                                    for dy in 0..geometry_size.1 {
                                        occupied_write.insert((grid_x + dx, grid_y + dy));
                                    }
                                }
                                drop(occupied_write);

                                // Create UI for the copied panel
                                let config_read = config_for_end.borrow();
                                let x = grid_x as i32 * (config_read.cell_width + config_read.spacing);
                                let y = grid_y as i32 * (config_read.cell_height + config_read.spacing);
                                let width = geometry_size.0 as i32 * config_read.cell_width
                                    + (geometry_size.0 as i32 - 1) * config_read.spacing;
                                let height = geometry_size.1 as i32 * config_read.cell_height
                                    + (geometry_size.1 as i32 - 1) * config_read.spacing;
                                drop(config_read);

                                // Create displayer widget
                                let widget = {
                                    let panel_guard = new_panel.blocking_read();
                                    panel_guard.displayer.create_widget()
                                };
                                widget.set_size_request(width, height);

                                // Create background drawing area
                                use gtk4::DrawingArea;
                                let background_area = DrawingArea::new();
                                background_area.set_size_request(width, height);

                                // Setup background rendering
                                let panel_clone_bg = new_panel.clone();
                                background_area.set_draw_func(move |_, cr, w, h| {
                                    match panel_clone_bg.try_read() {
                                        Ok(panel_guard) => {
                                            if let Err(e) = crate::ui::render_background(cr, &panel_guard.background, w as f64, h as f64) {
                                                log::warn!("Failed to render background: {}", e);
                                            }
                                        }
                                        Err(_) => {
                                            log::warn!("Failed to acquire panel read lock for background rendering");
                                        }
                                    }
                                });

                                // Create overlay to stack background and widget
                                use gtk4::Overlay;
                                let overlay = Overlay::new();
                                overlay.set_child(Some(&background_area));

                                // Make the widget transparent so the background shows through
                                widget.add_css_class("transparent-background");
                                overlay.add_overlay(&widget);

                                // Create frame for selection visual feedback
                                use gtk4::Frame;
                                let frame = Frame::new(None);
                                frame.set_child(Some(&overlay));
                                frame.set_size_request(width, height);

                                // Note: We cannot call setup_panel_interaction here as it requires &self
                                // The copied panel will not have interaction until the next app restart
                                // or until we implement a proper deferred setup mechanism
                                // For now, this is acceptable as a first implementation

                                // Add to container
                                container_for_copy.put(&frame, x as f64, y as f64);

                                // Store panel state
                                panel_states_end.borrow_mut().insert(
                                    new_id.clone(),
                                    PanelState {
                                        widget: widget.clone(),
                                        frame: frame.clone(),
                                        panel: new_panel.clone(),
                                        selected: false,
                                        background_area: background_area.clone(),
                                    },
                                );

                                log::info!("Created panel copy: {} at ({}, {})", new_id, grid_x, grid_y);
                            }
                        }
                    }
                }
            } else {
                // MOVE MODE: Move ALL panels to new positions
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
            drag_preview_cells_end.borrow_mut().clear();
            drop_zone_layer_end.queue_draw();
        });

        frame.add_controller(drag_gesture);
    }

    /// Remove a panel by ID
    pub fn remove_panel(&mut self, panel_id: &str) -> Option<Arc<RwLock<Panel>>> {
        // Remove from panels list
        if let Some(pos) = self
            .panels
            .borrow()
            .iter()
            .position(|p| p.blocking_read().id == panel_id)
        {
            let panel = self.panels.borrow_mut().remove(pos);

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

    pub fn widget(&self) -> Widget {
        self.overlay.clone().upcast()
    }

    /// Update grid cell size and spacing
    pub fn update_grid_size(&mut self, cell_width: i32, cell_height: i32, spacing: i32) {
        // Update config values
        {
            let mut config = self.config.borrow_mut();
            config.cell_width = cell_width;
            config.cell_height = cell_height;
            config.spacing = spacing;
        }

        // Update container size
        let config = self.config.borrow();
        let width = config.columns as i32 * (cell_width + spacing) - spacing;
        let height = config.rows as i32 * (cell_height + spacing) - spacing;
        drop(config);
        self.container.set_size_request(width, height);
        self.drop_zone_layer.set_size_request(width, height);

        // Update all panel sizes and positions
        for (_panel_id, state) in self.panel_states.borrow_mut().iter_mut() {
            if let Ok(panel_guard) = state.panel.try_read() {
                let geom = &panel_guard.geometry;

                // Calculate new pixel dimensions
                let pixel_width = geom.width as i32 * cell_width + (geom.width as i32 - 1) * spacing;
                let pixel_height = geom.height as i32 * cell_height + (geom.height as i32 - 1) * spacing;
                let x = (geom.x as i32 * (cell_width + spacing)) as f64;
                let y = (geom.y as i32 * (cell_height + spacing)) as f64;

                // Update frame size and position
                state.frame.set_size_request(pixel_width, pixel_height);
                self.container.move_(&state.frame, x, y);

                // Update widget size
                state.widget.set_size_request(pixel_width, pixel_height);

                // Update background area size
                state.background_area.set_size_request(pixel_width, pixel_height);
            }
        }

        self.drop_zone_layer.queue_draw();
    }

    pub fn set_config(&mut self, new_config: GridConfig) {
        *self.config.borrow_mut() = new_config;
        let width = new_config.columns as i32 * (new_config.cell_width + new_config.spacing) - new_config.spacing;
        let height = new_config.rows as i32 * (new_config.cell_height + new_config.spacing) - new_config.spacing;
        self.container.set_size_request(width, height);
        self.drop_zone_layer.set_size_request(width, height);
    }

    pub fn config(&self) -> GridConfig {
        *self.config.borrow()
    }
}

/// Show panel properties dialog
fn show_panel_properties_dialog(
    panel: &Arc<RwLock<Panel>>,
    config: GridConfig,
    panel_states: Rc<RefCell<HashMap<String, PanelState>>>,
    occupied_cells: Rc<RefCell<HashSet<(u32, u32)>>>,
    _container: Fixed,
    on_change: Rc<RefCell<Option<Box<dyn Fn()>>>>,
    drop_zone: DrawingArea,
    registry: &'static crate::core::Registry,
) {
    use gtk4::{Box as GtkBox, Button, DropDown, Label, Notebook, Orientation, SpinButton, StringList, Window};

    let panel_guard = match panel.try_read() {
        Ok(guard) => guard,
        Err(_) => {
            log::warn!("Failed to lock panel for properties dialog");
            return;
        }
    };

    let panel_id = panel_guard.id.clone();
    let old_geometry = Rc::new(RefCell::new(panel_guard.geometry));
    let old_source_id = panel_guard.source.metadata().id.clone();
    let old_displayer_id = panel_guard.displayer.id().to_string();

    // Create dialog window
    let dialog = Window::builder()
        .title(format!("Panel Properties - {}", panel_id))
        .modal(true)
        .default_width(500)
        .default_height(450)
        .build();

    // Main container
    let vbox = GtkBox::new(Orientation::Vertical, 12);
    vbox.set_margin_top(12);
    vbox.set_margin_bottom(12);
    vbox.set_margin_start(12);
    vbox.set_margin_end(12);

    // Create notebook for tabs
    let notebook = Notebook::new();
    notebook.set_vexpand(true);

    // === Tab 1: Panel Properties ===
    let panel_props_box = GtkBox::new(Orientation::Vertical, 12);
    panel_props_box.set_margin_top(12);
    panel_props_box.set_margin_bottom(12);
    panel_props_box.set_margin_start(12);
    panel_props_box.set_margin_end(12);

    // Panel Size section
    let size_label = Label::new(Some("Panel Size"));
    size_label.add_css_class("heading");
    panel_props_box.append(&size_label);

    let size_box = GtkBox::new(Orientation::Horizontal, 6);
    size_box.set_margin_start(12);

    // Width control
    let width_label = Label::new(Some("Width:"));
    let width_spin = SpinButton::with_range(1.0, 10.0, 1.0);
    width_spin.set_value(old_geometry.borrow().width as f64);

    // Height control
    let height_label = Label::new(Some("Height:"));
    let height_spin = SpinButton::with_range(1.0, 10.0, 1.0);
    height_spin.set_value(old_geometry.borrow().height as f64);

    size_box.append(&width_label);
    size_box.append(&width_spin);
    size_box.append(&height_label);
    size_box.append(&height_spin);

    panel_props_box.append(&size_box);

    // Background section
    let background_label = Label::new(Some("Background"));
    background_label.add_css_class("heading");
    background_label.set_margin_top(12);
    panel_props_box.append(&background_label);

    let background_widget = crate::ui::BackgroundConfigWidget::new();
    background_widget.set_config(panel_guard.background.clone());
    panel_props_box.append(background_widget.widget());

    // Wrap background_widget in Rc so we can share it with the closure
    let background_widget = Rc::new(background_widget);

    notebook.append_page(&panel_props_box, Some(&Label::new(Some("Panel Properties"))));

    // === Tab 2: Data Source ===
    let source_tab_box = GtkBox::new(Orientation::Vertical, 12);
    source_tab_box.set_margin_top(12);
    source_tab_box.set_margin_bottom(12);
    source_tab_box.set_margin_start(12);
    source_tab_box.set_margin_end(12);

    let source_label = Label::new(Some("Data Source"));
    source_label.add_css_class("heading");
    source_tab_box.append(&source_label);

    let source_box = GtkBox::new(Orientation::Horizontal, 6);
    source_box.set_margin_start(12);

    let source_combo_label = Label::new(Some("Source:"));

    // Populate source dropdown
    let sources = registry.list_sources();
    let mut selected_source_idx = 0;
    for (idx, source_id) in sources.iter().enumerate() {
        if source_id == &old_source_id {
            selected_source_idx = idx;
        }
    }

    let source_strings: Vec<&str> = sources.iter().map(|s| s.as_str()).collect();
    let source_list = StringList::new(&source_strings);
    let source_combo = DropDown::new(Some(source_list), Option::<gtk4::Expression>::None);
    source_combo.set_selected(selected_source_idx as u32);

    source_box.append(&source_combo_label);
    source_box.append(&source_combo);
    source_tab_box.append(&source_box);

    // CPU source configuration widget
    let cpu_config_widget = crate::ui::CpuSourceConfigWidget::new();
    cpu_config_widget.widget().set_visible(old_source_id == "cpu");

    // Populate sensor and core information from cached CPU hardware info
    cpu_config_widget.set_available_sensors(crate::sources::CpuSource::get_cached_sensors());
    cpu_config_widget.set_cpu_core_count(crate::sources::CpuSource::get_cached_core_count());

    // Load existing CPU config if source is CPU
    if old_source_id == "cpu" {
        if let Some(cpu_config_value) = panel_guard.config.get("cpu_config") {
            if let Ok(cpu_config) = serde_json::from_value::<crate::ui::CpuSourceConfig>(cpu_config_value.clone()) {
                cpu_config_widget.set_config(cpu_config);
            }
        }
    }

    source_tab_box.append(cpu_config_widget.widget());

    // Wrap cpu_config_widget in Rc for sharing
    let cpu_config_widget = Rc::new(cpu_config_widget);

    // GPU source configuration widget
    let gpu_config_widget = crate::ui::GpuSourceConfigWidget::new();
    gpu_config_widget.widget().set_visible(old_source_id == "gpu");

    // Populate GPU information from cached GPU hardware info
    let gpu_names: Vec<String> = crate::sources::GpuSource::get_cached_gpu_names()
        .iter()
        .map(|s| s.clone())
        .collect();
    gpu_config_widget.set_available_gpus(&gpu_names);

    // Load existing GPU config if source is GPU
    if old_source_id == "gpu" {
        if let Some(gpu_config_value) = panel_guard.config.get("gpu_config") {
            if let Ok(gpu_config) = serde_json::from_value::<crate::ui::GpuSourceConfig>(gpu_config_value.clone()) {
                gpu_config_widget.set_config(gpu_config);
            }
        }
    }

    source_tab_box.append(gpu_config_widget.widget());

    // Wrap gpu_config_widget in Rc for sharing
    let gpu_config_widget = Rc::new(gpu_config_widget);

    // Show/hide CPU and GPU config based on source selection
    {
        let cpu_widget_clone = cpu_config_widget.clone();
        let gpu_widget_clone = gpu_config_widget.clone();
        let sources_clone = sources.clone();
        source_combo.connect_selected_notify(move |combo| {
            let selected = combo.selected() as usize;
            if let Some(source_id) = sources_clone.get(selected) {
                cpu_widget_clone.widget().set_visible(source_id == "cpu");
                gpu_widget_clone.widget().set_visible(source_id == "gpu");
            }
        });
    }

    notebook.append_page(&source_tab_box, Some(&Label::new(Some("Data Source"))));

    // === Tab 3: Display Type ===
    let displayer_tab_box = GtkBox::new(Orientation::Vertical, 12);
    displayer_tab_box.set_margin_top(12);
    displayer_tab_box.set_margin_bottom(12);
    displayer_tab_box.set_margin_start(12);
    displayer_tab_box.set_margin_end(12);

    let displayer_label = Label::new(Some("Display Type"));
    displayer_label.add_css_class("heading");
    displayer_tab_box.append(&displayer_label);

    let displayer_box = GtkBox::new(Orientation::Horizontal, 6);
    displayer_box.set_margin_start(12);

    let displayer_combo_label = Label::new(Some("Displayer:"));

    // Populate displayer dropdown
    let displayers = registry.list_displayers();
    let mut selected_displayer_idx = 0;
    for (idx, displayer_id) in displayers.iter().enumerate() {
        if displayer_id == &old_displayer_id {
            selected_displayer_idx = idx;
        }
    }

    let displayer_strings: Vec<&str> = displayers.iter().map(|s| s.as_str()).collect();
    let displayer_list = StringList::new(&displayer_strings);
    let displayer_combo = DropDown::new(Some(displayer_list), Option::<gtk4::Expression>::None);
    displayer_combo.set_selected(selected_displayer_idx as u32);

    displayer_box.append(&displayer_combo_label);
    displayer_box.append(&displayer_combo);
    displayer_tab_box.append(&displayer_box);

    // Text displayer configuration (shown only when text displayer is selected)
    let text_config_label = Label::new(Some("Text Configuration"));
    text_config_label.add_css_class("heading");
    text_config_label.set_margin_top(12);

    // Get available fields from the current data source
    let available_fields = panel_guard.source.fields();

    let text_config_widget = crate::ui::TextLineConfigWidget::new(available_fields);
    text_config_widget.widget().set_visible(old_displayer_id == "text");
    text_config_label.set_visible(old_displayer_id == "text");

    // Load existing text config if displayer is text
    if old_displayer_id == "text" {
        // Try to load config from panel
        let text_config = if let Some(lines_value) = panel_guard.config.get("lines") {
            // Load from saved config
            serde_json::from_value::<crate::displayers::TextDisplayerConfig>(
                serde_json::json!({ "lines": lines_value })
            ).unwrap_or_default()
        } else {
            // Use default config if no saved config exists
            crate::displayers::TextDisplayerConfig::default()
        };
        text_config_widget.set_config(text_config);
    }

    displayer_tab_box.append(&text_config_label);
    displayer_tab_box.append(text_config_widget.widget());

    // Wrap text_config_widget in Rc for sharing
    let text_config_widget = Rc::new(text_config_widget);

    // Show/hide text config based on displayer selection
    {
        let text_widget_clone = text_config_widget.clone();
        let text_label_clone = text_config_label.clone();
        let displayers_clone = displayers.clone();
        displayer_combo.connect_selected_notify(move |combo| {
            let selected_idx = combo.selected() as usize;
            if let Some(displayer_id) = displayers_clone.get(selected_idx) {
                let is_text = displayer_id == "text";
                text_widget_clone.widget().set_visible(is_text);
                text_label_clone.set_visible(is_text);
            }
        });
    }

    // Update text config fields when data source changes
    {
        let _text_widget_clone = text_config_widget.clone();
        let sources_clone = sources.clone();
        source_combo.connect_selected_notify(move |combo| {
            let selected_idx = combo.selected() as usize;
            if let Some(source_id) = sources_clone.get(selected_idx) {
                // Create temporary source to get its fields
                if let Ok(temp_source) = registry.create_source(source_id) {
                    let new_fields = temp_source.fields();
                    // Note: TextLineConfigWidget doesn't have a method to update fields yet
                    // For now, this will need to be handled on next open or we need to add that method
                    // TODO: Add update_fields() method to TextLineConfigWidget
                    let _ = new_fields; // Suppress unused warning for now
                }
            }
        });
    }

    notebook.append_page(&displayer_tab_box, Some(&Label::new(Some("Display Type"))));

    // Add notebook to main vbox
    vbox.append(&notebook);

    // Buttons
    let button_box = GtkBox::new(Orientation::Horizontal, 6);
    button_box.set_halign(gtk4::Align::End);
    button_box.set_margin_top(12);

    let cancel_button = Button::with_label("Cancel");
    let apply_button = Button::with_label("Apply");
    let accept_button = Button::with_label("Accept");
    accept_button.add_css_class("suggested-action");

    let dialog_clone = dialog.clone();
    cancel_button.connect_clicked(move |_| {
        dialog_clone.close();
    });

    drop(panel_guard); // Release lock before closure

    // Create a shared closure for applying changes
    let panel_clone = panel.clone();
    let background_widget_clone = background_widget.clone();
    let text_config_widget_clone = text_config_widget.clone();
    let cpu_config_widget_clone = cpu_config_widget.clone();
    let gpu_config_widget_clone = gpu_config_widget.clone();
    let dialog_for_apply = dialog.clone();
    let width_spin_for_collision = width_spin.clone();
    let height_spin_for_collision = height_spin.clone();

    let apply_changes = Rc::new(move || {
        let new_width = width_spin.value() as u32;
        let new_height = height_spin.value() as u32;

        // Get selected source and displayer by index
        let new_source_id = sources.get(source_combo.selected() as usize)
            .cloned()
            .unwrap_or_else(|| old_source_id.clone());
        let new_displayer_id = displayers.get(displayer_combo.selected() as usize)
            .cloned()
            .unwrap_or_else(|| old_displayer_id.clone());

        // Get new background config
        let new_background = background_widget_clone.get_config();

        // Get current geometry (it may have changed from previous Apply)
        let current_geometry = *old_geometry.borrow();

        // Check if anything changed
        let size_changed = new_width != current_geometry.width || new_height != current_geometry.height;
        let source_changed = new_source_id != old_source_id;
        let displayer_changed = new_displayer_id != old_displayer_id;

        // Check if background changed (we'll always apply for now, can optimize later)
        let background_changed = true;

        if !size_changed && !source_changed && !displayer_changed && !background_changed {
            // No changes to apply
            return;
        }

        // Get panel state
        let mut states = panel_states.borrow_mut();
        let state = match states.get_mut(&panel_id) {
            Some(s) => s,
            None => {
                log::warn!("Panel state not found for {}", panel_id);
                return;
            }
        };

        // Clone background_area for later use (to avoid borrow conflicts)
        let background_area = state.background_area.clone();

        // Handle size change (collision check)
        if size_changed {
            let mut occupied = occupied_cells.borrow_mut();

            // Clear old occupied cells
            for dx in 0..current_geometry.width {
                for dy in 0..current_geometry.height {
                    occupied.remove(&(current_geometry.x + dx, current_geometry.y + dy));
                }
            }

            // Check if new size would cause collision
            let mut has_collision = false;
            for dx in 0..new_width {
                for dy in 0..new_height {
                    let cell = (current_geometry.x + dx, current_geometry.y + dy);
                    if occupied.contains(&cell) {
                        has_collision = true;
                        break;
                    }
                }
                if has_collision {
                    break;
                }
            }

            if has_collision {
                // Restore old occupied cells
                for dx in 0..current_geometry.width {
                    for dy in 0..current_geometry.height {
                        occupied.insert((current_geometry.x + dx, current_geometry.y + dy));
                    }
                }
                drop(occupied);
                drop(states);

                log::warn!("Cannot resize panel: collision detected");

                // Show error dialog and revert spinners
                let error_dialog = gtk4::AlertDialog::builder()
                    .message("Cannot Resize Panel")
                    .detail("The new size would overlap with another panel. Size has been reverted.")
                    .modal(true)
                    .buttons(vec!["OK"])
                    .build();

                // Revert spinners to current values
                width_spin_for_collision.set_value(current_geometry.width as f64);
                height_spin_for_collision.set_value(current_geometry.height as f64);

                error_dialog.show(Some(&dialog_for_apply));
                return;
            }

            // Mark new cells as occupied
            for dx in 0..new_width {
                for dy in 0..new_height {
                    occupied.insert((current_geometry.x + dx, current_geometry.y + dy));
                }
            }
        }

        // Update panel geometry, source, displayer, and background
        if let Ok(mut panel_guard) = panel_clone.try_write() {
            // Update size if changed
            if size_changed {
                log::info!("[RESIZE] Panel {} geometry changing from {}x{} to {}x{}",
                          panel_id, current_geometry.width, current_geometry.height,
                          new_width, new_height);
                panel_guard.geometry.width = new_width;
                panel_guard.geometry.height = new_height;
                log::info!("[RESIZE] Panel {} geometry updated to {}x{}",
                          panel_id, panel_guard.geometry.width, panel_guard.geometry.height);
            }

            // Update background if changed
            if background_changed {
                panel_guard.background = new_background;
            }

            // Update source if changed
            if source_changed {
                match registry.create_source(&new_source_id) {
                    Ok(new_source) => {
                        panel_guard.source = new_source;
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
                    }
                    Err(e) => {
                        log::warn!("Failed to create displayer {}: {}", new_displayer_id, e);
                    }
                }
            }

            // Apply text configuration if text displayer is active
            if new_displayer_id == "text" {
                let text_config = text_config_widget_clone.get_config();
                // Convert TextDisplayerConfig to HashMap for apply_config
                if let Ok(config_json) = serde_json::to_value(&text_config) {
                    if let Some(config_map) = config_json.as_object() {
                        let mut config_hash = std::collections::HashMap::new();
                        for (key, value) in config_map {
                            config_hash.insert(key.clone(), value.clone());
                        }
                        if let Err(e) = panel_guard.apply_config(config_hash) {
                            log::warn!("Failed to apply text config: {}", e);
                        }
                    }
                }
            }

            // Apply CPU source configuration if CPU source is active
            if new_source_id == "cpu" {
                let cpu_config = cpu_config_widget_clone.get_config();
                if let Ok(cpu_config_json) = serde_json::to_value(&cpu_config) {
                    panel_guard.config.insert("cpu_config".to_string(), cpu_config_json);

                    // Clone config before applying to avoid borrow checker issues
                    let config_clone = panel_guard.config.clone();

                    // Apply the configuration to the source
                    if let Err(e) = panel_guard.apply_config(config_clone) {
                        log::warn!("Failed to apply CPU config to source: {}", e);
                    }

                    // Update the source with new configuration
                    if let Err(e) = panel_guard.update() {
                        log::warn!("Failed to update panel after config change: {}", e);
                    }
                }
            }

            // Apply GPU source configuration if GPU source is active
            if new_source_id == "gpu" {
                let gpu_config = gpu_config_widget_clone.get_config();
                if let Ok(gpu_config_json) = serde_json::to_value(&gpu_config) {
                    panel_guard.config.insert("gpu_config".to_string(), gpu_config_json);

                    // Clone config before applying to avoid borrow checker issues
                    let config_clone = panel_guard.config.clone();

                    // Apply the configuration to the source
                    if let Err(e) = panel_guard.apply_config(config_clone) {
                        log::warn!("Failed to apply GPU config to source: {}", e);
                    }

                    // Update the source with new configuration
                    if let Err(e) = panel_guard.update() {
                        log::warn!("Failed to update panel after config change: {}", e);
                    }
                }
            }

            // Queue widget redraw to show updated data
            state.widget.queue_draw();

            // Drop the write lock before triggering redraws
            drop(panel_guard);
        }

        // Queue redraw of background AFTER releasing the panel write lock
        if background_changed {
            background_area.queue_draw();
        }

        // Update widget and frame sizes if size changed (and displayer wasn't replaced)
        if size_changed && !displayer_changed {
            let pixel_width = new_width as i32 * config.cell_width
                + (new_width as i32 - 1) * config.spacing;
            let pixel_height = new_height as i32 * config.cell_height
                + (new_height as i32 - 1) * config.spacing;

            state.widget.set_size_request(pixel_width, pixel_height);
            state.frame.set_size_request(pixel_width, pixel_height);
            state.background_area.set_size_request(pixel_width, pixel_height);
        }

        // Release panel_states borrow
        drop(states);

        // Trigger redraw of drop zone layer
        drop_zone.queue_draw();

        // Mark configuration as dirty
        if let Some(callback) = on_change.borrow().as_ref() {
            callback();
        }

        // Update old_geometry to reflect the new geometry for next Apply
        if size_changed {
            old_geometry.borrow_mut().width = new_width;
            old_geometry.borrow_mut().height = new_height;
        }
    });

    // Apply button - applies changes but keeps dialog open
    let apply_changes_clone = apply_changes.clone();
    apply_button.connect_clicked(move |_| {
        apply_changes_clone();
    });

    // Accept button - applies changes and closes dialog
    let apply_changes_clone2 = apply_changes.clone();
    let dialog_clone2 = dialog.clone();
    accept_button.connect_clicked(move |_| {
        apply_changes_clone2();
        dialog_clone2.close();
    });

    button_box.append(&cancel_button);
    button_box.append(&apply_button);
    button_box.append(&accept_button);

    vbox.append(&button_box);

    dialog.set_child(Some(&vbox));
    dialog.present();
}

impl Default for GridLayout {
    fn default() -> Self {
        Self::new(GridConfig::default())
    }
}
