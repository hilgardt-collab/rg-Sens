//! Grid layout manager for panels with advanced features

use crate::config::AppConfig;
use crate::core::Panel;
use crate::ui::pango_text::pango_show_text;
use gtk4::gdk::ModifierType;
use gtk4::{
    prelude::*, DrawingArea, Fixed, Frame, GestureClick, GestureDrag, Overlay, PopoverMenu, Widget,
};
use log::info;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;
use tokio::sync::RwLock;

// Import show_panel_properties_dialog from extracted module
use super::grid_properties_dialog::show_panel_properties_dialog;

/// Source-specific config keys that should be filtered out when copying panel styles.
/// These keys contain data source configuration, not visual styling.
const SOURCE_CONFIG_KEYS: &[&str] = &[
    "cpu_config",
    "gpu_config",
    "memory_config",
    "disk_config",
    "clock_config",
    "combo_config",
    "system_temp_config",
    "fan_speed_config",
    "test_config",
    "static_text_config",
];

/// Filter out source-specific config keys from a panel config.
/// Used when copying/pasting panel styles to preserve only visual settings.
fn filter_source_config_keys(config: &mut HashMap<String, serde_json::Value>) {
    for key in SOURCE_CONFIG_KEYS {
        config.remove(*key);
    }
}

/// Remove all event controllers from a widget to prevent memory leaks.
/// This should be called before removing a widget from the container.
fn cleanup_widget_controllers(widget: &impl IsA<Widget>) {
    let widget_ref = widget.as_ref();

    // Remove the action group to drop closure references
    widget_ref.insert_action_group("panel", None::<&gtk4::gio::ActionGroup>);

    // Get list of all controllers and remove them
    let controllers = widget_ref.observe_controllers();
    let n = controllers.n_items();

    // Collect controllers first to avoid modifying while iterating
    let mut to_remove = Vec::new();
    for i in 0..n {
        if let Some(obj) = controllers.item(i) {
            if let Ok(controller) = obj.downcast::<gtk4::EventController>() {
                to_remove.push(controller);
            }
        }
    }

    // Now remove all collected controllers
    for controller in to_remove {
        widget_ref.remove_controller(&controller);
    }
}

/// Create the standard panel context menu model.
/// Returns a Menu with Properties, Copy/Paste Style, Save to File, and Delete sections.
fn create_panel_context_menu() -> gtk4::gio::Menu {
    use gtk4::gio;

    let menu = gio::Menu::new();

    // Section 1: Properties
    let section1 = gio::Menu::new();
    section1.append(Some("Properties..."), Some("panel.properties"));
    menu.append_section(None, &section1);

    // Section 2: Copy/Paste Style
    let section2 = gio::Menu::new();
    section2.append(Some("Copy Style"), Some("panel.copy_style"));
    section2.append(Some("Paste Style"), Some("panel.paste_style"));
    section2.append(
        Some("Set as Default Style"),
        Some("panel.set_default_style"),
    );
    menu.append_section(None, &section2);

    // Section 3: Save to File
    let section3 = gio::Menu::new();
    section3.append(Some("Save Panel to File..."), Some("panel.save_to_file"));
    menu.append_section(None, &section3);

    // Section 4: Delete
    let section4 = gio::Menu::new();
    section4.append(Some("Delete"), Some("panel.delete"));
    menu.append_section(None, &section4);

    menu
}

// PANEL_PROPERTIES_DIALOG thread_local moved to grid_properties_dialog.rs

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
            rows: 4,
            columns: 4,
            cell_width: 16,
            cell_height: 16,
            spacing: 3,
        }
    }
}

/// Panel state tracking
pub(crate) struct PanelState {
    pub(crate) widget: Widget,
    pub(crate) frame: Frame,
    pub(crate) panel: Arc<RwLock<Panel>>,
    pub(crate) selected: bool,
    pub(crate) background_area: DrawingArea,
}

/// Callback type for borderless window move/resize
/// Returns true if the event was handled (gesture should be claimed), false otherwise
pub type BorderlessDragCallback = Box<dyn Fn(&gtk4::GestureDrag, f64, f64) -> bool>;

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
    /// Callback for borderless window drag (move/resize)
    /// If set and returns true, the drag gesture is claimed for window operations
    on_borderless_drag: Rc<RefCell<Option<BorderlessDragCallback>>>,
    /// Viewport (window) dimensions for auto-scroll boundary visualization
    viewport_size: Rc<RefCell<(i32, i32)>>,
    /// Application config for accessing global theme
    app_config: Rc<RefCell<AppConfig>>,
}

impl GridLayout {
    /// Create a new grid layout with access to app configuration for global theme
    pub fn new(config: GridConfig, app_config: Rc<RefCell<AppConfig>>) -> Self {
        let overlay = Overlay::new();
        let container = Fixed::new();

        // Create drop zone visualization layer
        let drop_zone_layer = DrawingArea::new();
        drop_zone_layer.set_can_target(false); // Don't capture events

        // Wrap config in Rc<RefCell<>> for shared mutable access
        let config = Rc::new(RefCell::new(config));

        // Set the container size
        let config_borrow = config.borrow();
        let width = config_borrow.columns as i32
            * (config_borrow.cell_width + config_borrow.spacing)
            - config_borrow.spacing;
        let height = config_borrow.rows as i32
            * (config_borrow.cell_height + config_borrow.spacing)
            - config_borrow.spacing;
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
            on_borderless_drag: Rc::new(RefCell::new(None)),
            viewport_size: Rc::new(RefCell::new((800, 600))), // Default, updated by main window
            app_config,
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

    /// Set a callback for borderless window drag (move/resize)
    /// The callback receives the gesture and coordinates, and returns true if it handled the event
    pub fn set_on_borderless_drag<F>(&mut self, callback: F)
    where
        F: Fn(&gtk4::GestureDrag, f64, f64) -> bool + 'static,
    {
        *self.on_borderless_drag.borrow_mut() = Some(Box::new(callback));
    }

    /// Get the list of panels (clones the Vec of Arcs)
    pub fn get_panels(&self) -> Vec<Arc<RwLock<Panel>>> {
        self.panels.borrow().clone()
    }

    /// Access panels without cloning the Vec
    ///
    /// Use this when you need to iterate over panels but don't need to store the list.
    /// The closure receives a slice reference, avoiding the Vec clone.
    #[inline]
    pub fn with_panels<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&[Arc<RwLock<Panel>>]) -> R,
    {
        f(&self.panels.borrow())
    }

    /// Get the number of panels without cloning
    #[inline]
    pub fn panels_len(&self) -> usize {
        self.panels.borrow().len()
    }

    /// Find an available position for a panel with the given dimensions
    /// Returns (x, y) grid coordinates where the panel can be placed
    pub fn find_available_position(&self, width: u32, height: u32) -> (u32, u32) {
        let occupied = self.occupied_cells.borrow();
        let config = self.config.borrow();

        // Search within actual grid dimensions (with some overflow for expansion)
        // Add panel size to allow placing at edges, cap at reasonable max
        let max_x = (config.columns + width).min(100);
        let max_y = (config.rows + height).min(100);

        // Search for available position starting from (0, 0)
        // Scan row by row, column by column
        for y in 0..max_y {
            'next_x: for x in 0..max_x {
                // Check if all cells for this panel would be available
                for dx in 0..width {
                    for dy in 0..height {
                        if occupied.contains(&(x + dx, y + dy)) {
                            continue 'next_x;
                        }
                    }
                }
                // All cells are available
                return (x, y);
            }
        }

        // Fallback: return (0, 0) if no space found
        (0, 0)
    }

    /// Check if placing a panel at the given position would collide with existing panels
    /// Returns true if there would be a collision
    pub fn check_collision(&self, x: u32, y: u32, width: u32, height: u32) -> bool {
        let occupied = self.occupied_cells.borrow();

        for dx in 0..width {
            for dy in 0..height {
                if occupied.contains(&(x + dx, y + dy)) {
                    return true;
                }
            }
        }

        false
    }

    /// Check if a pixel coordinate (relative to the grid container) is on any panel.
    /// Used to determine if a right-click should show the panel menu or the window menu.
    pub fn is_point_on_panel(&self, x: f64, y: f64) -> bool {
        let config = self.config.borrow();
        let states = self.panel_states.borrow();

        for state in states.values() {
            if let Ok(panel_guard) = state.panel.try_read() {
                let geom = &panel_guard.geometry;
                let panel_x = geom.x as f64 * (config.cell_width + config.spacing) as f64;
                let panel_y = geom.y as f64 * (config.cell_height + config.spacing) as f64;
                let panel_w = geom.width as f64 * config.cell_width as f64
                    + (geom.width as f64 - 1.0) * config.spacing as f64;
                let panel_h = geom.height as f64 * config.cell_height as f64
                    + (geom.height as f64 - 1.0) * config.spacing as f64;

                if x >= panel_x && x < panel_x + panel_w && y >= panel_y && y < panel_y + panel_h {
                    return true;
                }
            }
        }

        false
    }

    /// Setup drop zone visualization
    fn setup_drop_zone_drawing(&self) {
        let config = self.config.clone();
        let occupied_cells = self.occupied_cells.clone();
        let drag_preview_cells = self.drag_preview_cells.clone();
        let is_dragging = self.is_dragging.clone();
        let selection_box = self.selection_box.clone();
        let viewport_size = self.viewport_size.clone();

        self.drop_zone_layer
            .set_draw_func(move |_, cr, width, height| {
                let config = config.borrow();
                let sel_box = selection_box.borrow();
                let viewport = *viewport_size.borrow();

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
                let available_cols =
                    (width as f64 / (config.cell_width + config.spacing) as f64).floor() as u32;
                let available_rows =
                    (height as f64 / (config.cell_height + config.spacing) as f64).floor() as u32;

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

                // Draw viewport boundary rectangles (window-sized areas)
                // Only draw if viewport is valid (non-zero dimensions)
                if viewport.0 > 0 && viewport.1 > 0 {
                    let vp_width = viewport.0 as f64;
                    let vp_height = viewport.1 as f64;

                    // Calculate how many viewports fit in the grid area
                    let vp_cols = (width as f64 / vp_width).ceil() as i32;
                    let vp_rows = (height as f64 / vp_height).ceil() as i32;

                    // Draw viewport boundary rectangles with dashed lines
                    cr.save().ok();
                    cr.set_source_rgba(0.9, 0.6, 0.1, 0.7); // Orange color for visibility
                    cr.set_line_width(2.0);
                    cr.set_dash(&[8.0, 4.0], 0.0);

                    for vp_row in 0..vp_rows {
                        for vp_col in 0..vp_cols {
                            let rect_x = vp_col as f64 * vp_width;
                            let rect_y = vp_row as f64 * vp_height;

                            // Only draw if within grid bounds
                            if rect_x < width as f64 && rect_y < height as f64 {
                                cr.rectangle(rect_x, rect_y, vp_width, vp_height);
                            }
                        }
                    }
                    cr.stroke().ok();

                    // Add viewport labels at top-left of each viewport
                    // OPTIMIZATION: Reuse string buffer to avoid allocations in render loop
                    cr.set_source_rgba(0.9, 0.6, 0.1, 0.9);
                    let mut label_buf = String::with_capacity(16);
                    for vp_row in 0..vp_rows {
                        for vp_col in 0..vp_cols {
                            let rect_x = vp_col as f64 * vp_width + 5.0;
                            let rect_y = vp_row as f64 * vp_height + 15.0;

                            if rect_x < width as f64 && rect_y < height as f64 {
                                label_buf.clear();
                                use std::fmt::Write;
                                let _ = write!(label_buf, "Page {}", vp_row * vp_cols + vp_col + 1);
                                cr.move_to(rect_x, rect_y);
                                pango_show_text(
                                    cr,
                                    &label_buf,
                                    "Sans",
                                    gtk4::cairo::FontSlant::Normal,
                                    gtk4::cairo::FontWeight::Normal,
                                    12.0,
                                );
                            }
                        }
                    }
                    cr.restore().ok();
                }

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

                        if x >= panel_x
                            && x <= panel_x + panel_width
                            && y >= panel_y
                            && y <= panel_y + panel_height
                        {
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
        // Track if borderless drag callback claimed the gesture
        let borderless_drag_claimed: Rc<RefCell<bool>> = Rc::new(RefCell::new(false));

        let drag_on_empty_space_begin = drag_on_empty_space.clone();
        let drag_start_pos_begin = drag_start_pos.clone();
        let panel_states_begin = panel_states_drag.clone();
        let borderless_drag_callback = self.on_borderless_drag.clone();
        let borderless_drag_claimed_begin = borderless_drag_claimed.clone();

        drag_gesture.connect_drag_begin(move |gesture, x, y| {
            // Reset claimed state
            *borderless_drag_claimed_begin.borrow_mut() = false;

            // First, check if borderless drag callback wants to handle this
            if let Some(ref callback) = *borderless_drag_callback.borrow() {
                if callback(gesture, x, y) {
                    // Callback handled it (e.g., started window move/resize)
                    *borderless_drag_claimed_begin.borrow_mut() = true;
                    return;
                }
            }

            // Check if drag started on empty space
            let states = panel_states_begin.borrow();
            let mut on_panel = false;

            for state in states.values() {
                if let Some(parent) = state.frame.parent() {
                    if let Ok(fixed) = parent.downcast::<Fixed>() {
                        let (panel_x, panel_y) = fixed.child_position(&state.frame);
                        let panel_width = state.frame.width() as f64;
                        let panel_height = state.frame.height() as f64;

                        if x >= panel_x
                            && x <= panel_x + panel_width
                            && y >= panel_y
                            && y <= panel_y + panel_height
                        {
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
        let borderless_drag_claimed_update = borderless_drag_claimed.clone();

        drag_gesture.connect_drag_update(move |_, offset_x, offset_y| {
            // Skip if borderless callback claimed the gesture
            if *borderless_drag_claimed_update.borrow() {
                return;
            }
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
        let borderless_drag_claimed_end = borderless_drag_claimed.clone();

        drag_gesture.connect_drag_end(move |_, offset_x, offset_y| {
            // Skip if borderless callback claimed the gesture
            if *borderless_drag_claimed_end.borrow() {
                return;
            }
            if *drag_on_empty_space_end.borrow() {
                if let Some((start_x, start_y)) = *drag_start_pos_end.borrow() {
                    let end_x = start_x + offset_x;
                    let end_y = start_y + offset_y;

                    // Check if this was essentially a click (very small drag)
                    let drag_distance = (offset_x * offset_x + offset_y * offset_y).sqrt();
                    let is_click = drag_distance < 5.0;

                    if is_click {
                        // This was a click on empty space - deselect all panels
                        let states = panel_states_drag.borrow();
                        let mut selected = selected_panels_drag.borrow_mut();
                        for (id, state) in states.iter() {
                            if selected.contains(id) {
                                state.frame.remove_css_class("selected");
                            }
                        }
                        selected.clear();
                    } else {
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
                                    let intersects = !(rect_x2 < panel_x
                                        || rect_x1 > panel_x2
                                        || rect_y2 < panel_y
                                        || rect_y1 > panel_y2);

                                    if intersects && !selected.contains(id) {
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

        // Keyboard handling for Delete key
        let key_controller = gtk4::EventControllerKey::new();
        let selected_panels_key = self.selected_panels.clone();
        let panel_states_key = self.panel_states.clone();
        let occupied_cells_key = self.occupied_cells.clone();
        let container_key = self.container.clone();
        let panels_key = self.panels.clone();
        let on_change_key = self.on_change.clone();

        key_controller.connect_key_pressed(move |_, keyval, _keycode, _state| {
            use gtk4::gdk::Key;

            if keyval == Key::Delete || keyval == Key::BackSpace {
                let selected = selected_panels_key.borrow();
                if selected.is_empty() {
                    return gtk4::glib::Propagation::Proceed;
                }

                let selected_ids: Vec<String> = selected.iter().cloned().collect();
                let count = selected_ids.len();
                drop(selected);

                log::info!("Delete key pressed, deleting {} selected panels", count);

                // Show confirmation dialog
                let dialog = gtk4::AlertDialog::builder()
                    .modal(true)
                    .message(format!(
                        "Delete {} Panel{}?",
                        count,
                        if count > 1 { "s" } else { "" }
                    ))
                    .detail(format!(
                        "This will permanently delete the selected panel{}.",
                        if count > 1 { "s" } else { "" }
                    ))
                    .buttons(vec!["Cancel", "Delete"])
                    .default_button(0)
                    .cancel_button(0)
                    .build();

                let selected_panels_del = selected_panels_key.clone();
                let panel_states_del = panel_states_key.clone();
                let occupied_cells_del = occupied_cells_key.clone();
                let container_del = container_key.clone();
                let panels_del = panels_key.clone();
                let on_change_del = on_change_key.clone();

                dialog.choose(
                    None::<&gtk4::Window>,
                    None::<&gtk4::gio::Cancellable>,
                    move |result| {
                        if let Ok(response) = result {
                            if response == 1 {
                                // Delete button clicked
                                delete_selected_panels(
                                    &selected_ids,
                                    &selected_panels_del,
                                    &panel_states_del,
                                    &occupied_cells_del,
                                    &container_del,
                                    &panels_del,
                                    &on_change_del,
                                );
                            }
                        }
                    },
                );

                return gtk4::glib::Propagation::Stop;
            }

            gtk4::glib::Propagation::Proceed
        });

        // Make the overlay focusable to receive key events
        self.overlay.set_focusable(true);
        self.overlay.set_can_focus(true);
        self.overlay.add_controller(key_controller);
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
        let (widget, displayer_id) = {
            let panel_guard = panel.blocking_read();
            (
                panel_guard.displayer.create_widget(),
                panel_guard.displayer.id().to_string(),
            )
        };
        widget.set_size_request(width, height);

        // For clock displayers, add click handler for alarm/timer management
        if displayer_id == "clock_analog" || displayer_id == "clock_digital" {
            let gesture = gtk4::GestureClick::new();
            let panel_for_click = panel.clone();
            gesture.connect_released(move |gesture, _, click_x, click_y| {
                if let Some(widget) = gesture.widget() {
                    // First check if any alarm is triggered or timer is finished - if so, clicking anywhere dismisses it
                    // Use global timer manager for accurate state
                    let (alarm_triggered, timer_finished) = {
                        if let Ok(manager) = crate::core::global_timer_manager().read() {
                            (manager.any_alarm_triggered(), manager.any_timer_finished())
                        } else {
                            (false, false)
                        }
                    };

                    if alarm_triggered || timer_finished {
                        // Click anywhere dismisses all triggered alarms and finished timers
                        // Also stop any playing sounds
                        crate::core::stop_all_sounds();
                        if let Ok(mut manager) = crate::core::global_timer_manager().write() {
                            if alarm_triggered {
                                manager.dismiss_all_alarms();
                            }
                            if timer_finished {
                                manager.dismiss_finished_timers();
                            }
                        }
                        return; // Don't open dialog when dismissing
                    }

                    // Check if click is on the icon - only open dialog if clicked on icon
                    let on_icon = if let Ok(panel_guard) = panel_for_click.try_read() {
                        if let Some((ix, iy, iw, ih)) = panel_guard.displayer.get_icon_bounds() {
                            click_x >= ix
                                && click_x <= ix + iw
                                && click_y >= iy
                                && click_y <= iy + ih
                        } else {
                            // No icon bounds means no icon shown, allow click anywhere
                            true
                        }
                    } else {
                        true // Couldn't read panel, allow click
                    };

                    if on_icon {
                        let window = widget
                            .root()
                            .and_then(|r| r.downcast::<gtk4::Window>().ok());

                        crate::ui::AlarmTimerDialog::show(window.as_ref());
                    }
                }
            });
            widget.add_controller(gesture);
        }

        // Create background drawing area
        let background_area = DrawingArea::new();
        background_area.set_size_request(width, height);

        // Setup background rendering
        let panel_clone_bg = panel.clone();
        let app_config_bg = self.app_config.clone();
        let background_area_weak = background_area.downgrade();
        background_area.set_draw_func(move |_, cr, w, h| {
            match panel_clone_bg.try_read() {
                Ok(panel_guard) => {
                    let width = w as f64;
                    let height = h as f64;
                    let radius = panel_guard.corner_radius.min(width / 2.0).min(height / 2.0);

                    // Create rounded rectangle path
                    cr.new_path();
                    if radius > 0.0 {
                        cr.arc(
                            radius,
                            radius,
                            radius,
                            std::f64::consts::PI,
                            3.0 * std::f64::consts::PI / 2.0,
                        );
                        cr.arc(
                            width - radius,
                            radius,
                            radius,
                            3.0 * std::f64::consts::PI / 2.0,
                            0.0,
                        );
                        cr.arc(
                            width - radius,
                            height - radius,
                            radius,
                            0.0,
                            std::f64::consts::PI / 2.0,
                        );
                        cr.arc(
                            radius,
                            height - radius,
                            radius,
                            std::f64::consts::PI / 2.0,
                            std::f64::consts::PI,
                        );
                        cr.close_path();
                    } else {
                        cr.rectangle(0.0, 0.0, width, height);
                    }

                    // Render background with clipping
                    cr.save().ok();
                    cr.clip();
                    // Get source values for indicator backgrounds and theme for color resolution
                    let source_values = panel_guard.source.get_values();
                    let theme = app_config_bg.borrow().global_theme.clone();
                    if let Err(e) = crate::ui::render_background_with_source_and_theme(
                        cr,
                        &panel_guard.background,
                        width,
                        height,
                        &source_values,
                        Some(&theme),
                    ) {
                        log::warn!("Failed to render background: {}", e);
                    }
                    cr.restore().ok();

                    // Render border if enabled
                    if panel_guard.border.enabled {
                        if radius > 0.0 {
                            cr.arc(
                                radius,
                                radius,
                                radius,
                                std::f64::consts::PI,
                                3.0 * std::f64::consts::PI / 2.0,
                            );
                            cr.arc(
                                width - radius,
                                radius,
                                radius,
                                3.0 * std::f64::consts::PI / 2.0,
                                0.0,
                            );
                            cr.arc(
                                width - radius,
                                height - radius,
                                radius,
                                0.0,
                                std::f64::consts::PI / 2.0,
                            );
                            cr.arc(
                                radius,
                                height - radius,
                                radius,
                                std::f64::consts::PI / 2.0,
                                std::f64::consts::PI,
                            );
                            cr.close_path();
                        } else {
                            cr.rectangle(0.0, 0.0, width, height);
                        }
                        panel_guard.border.color.apply_to_cairo(cr);
                        cr.set_line_width(panel_guard.border.width);
                        cr.stroke().ok();
                    }
                }
                Err(_) => {
                    // Lock contention - draw a fallback solid background and schedule a retry
                    // This ensures the panel is never completely invisible
                    log::debug!(
                        "Lock contention during background render, drawing fallback and scheduling retry"
                    );
                    let width = w as f64;
                    let height = h as f64;
                    cr.save().ok();
                    cr.set_source_rgba(0.1, 0.1, 0.1, 0.8); // Dark fallback background
                    cr.rectangle(0.0, 0.0, width, height);
                    cr.fill().ok();
                    cr.restore().ok();

                    if let Some(bg_area) = background_area_weak.upgrade() {
                        gtk4::glib::idle_add_local_once(move || {
                            bg_area.queue_draw();
                        });
                    }
                }
            }
        });

        // Set up periodic redraw for indicator backgrounds
        // This ensures the background color updates when source values change
        // Only redraws when the indicator value actually changes to avoid wasting CPU
        // 500ms interval is sufficient since panel update cycle also triggers redraws
        // Only create timer if panel has indicator background to avoid unnecessary timers
        let is_indicator_bg = panel
            .blocking_read()
            .background
            .background
            .is_indicator();
        if is_indicator_bg {
            let panel_for_bg_timer = panel.clone();
            let background_area_weak_timer = background_area.downgrade();
            let last_indicator_value: Rc<RefCell<Option<f64>>> = Rc::new(RefCell::new(None));
            gtk4::glib::timeout_add_local(std::time::Duration::from_millis(500), move || {
                // Stop if background area is gone (panel deleted)
                let Some(bg_area) = background_area_weak_timer.upgrade() else {
                    log::debug!("Indicator background timer stopping: widget destroyed");
                    return gtk4::glib::ControlFlow::Break;
                };

                // Stop if widget is orphaned (removed from widget tree but not destroyed)
                // This prevents memory leaks when panels are replaced
                if bg_area.root().is_none() {
                    log::debug!("Indicator background timer stopping: widget orphaned");
                    return gtk4::glib::ControlFlow::Break;
                }

                // Check if panel background is still indicator type and value changed
                if let Ok(panel_guard) = panel_for_bg_timer.try_read() {
                    if let crate::ui::BackgroundType::Indicator(ref indicator) =
                        panel_guard.background.background
                    {
                        // Get current value from source (not config - config stores source settings, not values)
                        let source_values = panel_guard.source.get_values();
                        let current_value = if !indicator.value_field.is_empty() {
                            source_values
                                .get(&indicator.value_field)
                                .and_then(|v| v.as_f64())
                        } else {
                            Some(indicator.static_value)
                        };

                        // Only redraw if value changed
                        let mut last_val = last_indicator_value.borrow_mut();
                        if *last_val != current_value {
                            *last_val = current_value;
                            bg_area.queue_draw();
                        }
                    } else {
                        // Background type changed from indicator to something else, stop timer
                        log::debug!(
                            "Indicator background timer stopping: background type changed"
                        );
                        return gtk4::glib::ControlFlow::Break;
                    }
                }
                gtk4::glib::ControlFlow::Continue
            });
        }

        // Create overlay to stack background and widget
        let overlay = Overlay::new();
        overlay.set_child(Some(&background_area));

        // Make the widget transparent so the background shows through
        widget.add_css_class("transparent-background");
        overlay.add_overlay(&widget);

        // Create frame for selection visual feedback
        // Note: Corner radius is handled by Cairo rendering in background_area.set_draw_func
        let frame = Frame::new(None);
        frame.set_child(Some(&overlay));
        frame.set_size_request(width, height);

        // Apply corner radius clipping via CSS
        {
            let panel_guard = panel.blocking_read();
            let radius = panel_guard.corner_radius;
            let panel_id = panel_guard.id.clone();
            drop(panel_guard);

            if radius > 0.0 {
                // Use a unique CSS class per panel to avoid global style conflicts
                let css_class = format!("panel-radius-{}", panel_id.replace('-', "_"));
                frame.add_css_class(&css_class);

                let css_provider = gtk4::CssProvider::new();
                let css = format!(".{} {{ border-radius: {}px; }}", css_class, radius);
                css_provider.load_from_data(&css);
                // Add provider to display (CSS class ensures it only affects this panel)
                let display = frame.display();
                gtk4::style_context_add_provider_for_display(
                    &display,
                    &css_provider,
                    gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
                );
            }
        }

        // Setup drag-and-drop and selection
        self.setup_panel_interaction(&frame, &widget, panel.clone());

        // Add to container
        self.container.put(&frame, x as f64, y as f64);

        // Mark cells as occupied (only if panel participates in collision detection)
        let ignore_collision = panel.blocking_read().ignore_collision;
        if !ignore_collision {
            for dx in 0..geometry.width {
                for dy in 0..geometry.height {
                    self.occupied_cells
                        .borrow_mut()
                        .insert((geometry.x + dx, geometry.y + dy));
                }
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

        // Update container size to fit all content (enables scrolling for off-screen panels)
        self.update_content_bounds();
    }

    /// Reorder all panel widgets in the container based on their z_index.
    /// Panels with lower z_index are placed first (behind), higher z_index on top.
    pub fn reorder_panels_by_z_index(&self) {
        // Collect panel IDs with their z_index and current positions
        let mut panel_info: Vec<(String, i32, f64, f64)> = Vec::new();
        let states = self.panel_states.borrow();
        let config = self.config.borrow();

        for (panel_id, state) in states.iter() {
            let panel_guard = state.panel.blocking_read();
            let z_index = panel_guard.z_index;
            let x = panel_guard.geometry.x as f64 * (config.cell_width + config.spacing) as f64;
            let y = panel_guard.geometry.y as f64 * (config.cell_height + config.spacing) as f64;
            panel_info.push((panel_id.clone(), z_index, x, y));
        }

        // Sort by z_index (ascending - lower z_index first means behind)
        panel_info.sort_by_key(|(_, z, _, _)| *z);

        drop(config);
        drop(states);

        // Re-add frames in z-order (removing and re-adding changes stacking order)
        let states = self.panel_states.borrow();
        for (panel_id, _, x, y) in panel_info {
            if let Some(state) = states.get(&panel_id) {
                // Remove and re-add at same position to change stacking order
                self.container.remove(&state.frame);
                self.container.put(&state.frame, x, y);
            }
        }
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

        let menu = create_panel_context_menu();
        let popover = PopoverMenu::from_model(Some(&menu));
        popover.set_parent(widget);
        popover.set_has_arrow(false);

        // Ensure popover is unparented when widget is destroyed to avoid GTK warnings
        let popover_weak = popover.downgrade();
        widget.connect_destroy(move |_| {
            if let Some(p) = popover_weak.upgrade() {
                p.unparent();
            }
        });

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
        let app_config_props = self.app_config.clone();

        let properties_action = gio::SimpleAction::new("properties", None);
        let selected_panels_props = self.selected_panels.clone();
        let panels_props = self.panels.clone();
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
                selected_panels_props.clone(),
                panels_props.clone(),
                app_config_props.borrow().global_theme.clone(),
            );
        });
        action_group.add_action(&properties_action);

        // Copy Style action
        let panel_copy_style = panel.clone();
        let copy_style_action = gio::SimpleAction::new("copy_style", None);
        copy_style_action.connect_activate(move |_, _| {
            info!("Copying panel style");
            let panel_guard = panel_copy_style.blocking_read();
            use crate::ui::{PanelStyle, CLIPBOARD};

            // Filter out source-specific config keys
            let mut displayer_config = panel_guard.config.clone();
            filter_source_config_keys(&mut displayer_config);

            let style = PanelStyle {
                background: panel_guard.background.clone(),
                corner_radius: panel_guard.corner_radius,
                border: panel_guard.border.clone(),
                displayer_config,
            };

            if let Ok(mut clipboard) = CLIPBOARD.lock() {
                clipboard.copy_panel_style(style);
                info!("Panel style copied to clipboard");
            }
        });
        action_group.add_action(&copy_style_action);

        // Paste Style action
        let panel_paste_style = panel.clone();
        let panel_states_paste = self.panel_states.clone();
        let on_change_paste = self.on_change.clone();
        let drop_zone_paste = self.drop_zone_layer.clone();
        let paste_style_action = gio::SimpleAction::new("paste_style", None);
        paste_style_action.connect_activate(move |_, _| {
            use crate::ui::CLIPBOARD;

            if let Ok(clipboard) = CLIPBOARD.lock() {
                if let Some(style) = clipboard.paste_panel_style() {
                    info!("Pasting panel style");

                    // Apply style and get panel ID, then drop the lock before calling callbacks
                    let panel_id = {
                        let mut panel_guard = panel_paste_style.blocking_write();
                        // Apply the style
                        panel_guard.background = style.background;
                        panel_guard.corner_radius = style.corner_radius;
                        panel_guard.border = style.border;

                        // Merge displayer config (keep source-specific configs)
                        for (key, value) in style.displayer_config {
                            panel_guard.config.insert(key, value);
                        }

                        // Apply config to displayer (clone config to avoid borrow conflict)
                        let config_clone = panel_guard.config.clone();
                        let _ = panel_guard.displayer.apply_config(&config_clone);

                        panel_guard.id.clone()
                    }; // panel_guard dropped here

                    // Trigger redraw (after releasing panel lock)
                    if let Some(state) = panel_states_paste.borrow().get(&panel_id) {
                        state.background_area.queue_draw();
                        state.widget.queue_draw();
                    }

                    // Trigger on_change callback (after releasing panel lock to avoid deadlock)
                    if let Some(ref callback) = *on_change_paste.borrow() {
                        callback();
                    }

                    drop_zone_paste.queue_draw();
                    info!("Panel style pasted successfully");
                } else {
                    info!("No panel style in clipboard");
                }
            }
        });
        action_group.add_action(&paste_style_action);

        // Set as Default Style action
        let panel_set_default = panel.clone();
        let set_default_style_action = gio::SimpleAction::new("set_default_style", None);
        set_default_style_action.connect_activate(move |_, _| {
            use crate::config::DefaultsConfig;

            info!("Setting panel style as default");
            let panel_guard = panel_set_default.blocking_read();

            // Get the displayer ID and its typed config
            let displayer_id = panel_guard.displayer.id().to_string();

            // Get the displayer's typed config and convert to JSON Value
            // Use to_inner_value() to save just the config, not the enum wrapper
            let displayer_config =
                if let Some(typed_config) = panel_guard.displayer.get_typed_config() {
                    // Use to_inner_value() for consistent serialization format
                    typed_config.to_inner_value()
                } else {
                    // Fall back to HashMap config (filter out source-specific keys)
                    let mut config = panel_guard.config.clone();
                    filter_source_config_keys(&mut config);
                    serde_json::to_value(&config).ok()
                };

            drop(panel_guard);

            if let Some(config_value) = displayer_config {
                // Load current defaults, update, and save
                let mut defaults = DefaultsConfig::load();
                defaults.set_displayer_default(&displayer_id, config_value);

                match defaults.save() {
                    Ok(()) => {
                        info!("Default style saved for displayer: {}", displayer_id);
                    }
                    Err(e) => {
                        log::warn!("Failed to save default style: {}", e);
                    }
                }
            } else {
                log::warn!("Could not serialize displayer config for default style");
            }
        });
        action_group.add_action(&set_default_style_action);

        // Save to File action
        let panel_save_file = panel.clone();
        let container_for_save = self.container.clone();
        let save_to_file_action = gio::SimpleAction::new("save_to_file", None);
        save_to_file_action.connect_activate(move |_, _| {
            info!("Saving panel to file");

            // Get panel data (use blocking read to ensure we get the data)
            let panel_data = {
                let panel_guard = panel_save_file.blocking_read();
                panel_guard.to_data()
            };

            let data = panel_data;
            // Get the parent window
            if let Some(root) = container_for_save.root() {
                if let Some(window) = root.downcast_ref::<gtk4::Window>() {
                    let window_clone = window.clone();

                    gtk4::glib::MainContext::default().spawn_local(async move {
                        use gtk4::FileDialog;

                        // Get initial directory (config dir)
                        let initial_dir = directories::ProjectDirs::from(
                            "com",
                            "github.hilgardt_collab",
                            "rg-sens",
                        )
                        .map(|d| d.config_dir().to_path_buf())
                        .unwrap_or_else(|| std::path::PathBuf::from("/"));

                        // Create file filter for JSON files
                        let json_filter = gtk4::FileFilter::new();
                        json_filter.set_name(Some("JSON files"));
                        json_filter.add_pattern("*.json");

                        let all_filter = gtk4::FileFilter::new();
                        all_filter.set_name(Some("All files"));
                        all_filter.add_pattern("*");

                        let filters = gio::ListStore::new::<gtk4::FileFilter>();
                        filters.append(&json_filter);
                        filters.append(&all_filter);

                        // Suggest a filename based on panel id
                        let suggested_name = format!("panel_{}.json", data.id.replace("-", "_"));

                        let file_dialog = FileDialog::builder()
                            .title("Save Panel to File")
                            .modal(true)
                            .initial_folder(&gio::File::for_path(&initial_dir))
                            .initial_name(&suggested_name)
                            .filters(&filters)
                            .default_filter(&json_filter)
                            .build();

                        match file_dialog.save_future(Some(&window_clone)).await {
                            Ok(file) => {
                                if let Some(path) = file.path() {
                                    info!("Saving panel to {:?}", path);

                                    // Serialize panel data to JSON
                                    match serde_json::to_string_pretty(&data) {
                                        Ok(json) => match std::fs::write(&path, json) {
                                            Ok(()) => {
                                                info!("Panel saved successfully to {:?}", path);
                                            }
                                            Err(e) => {
                                                log::warn!("Failed to write panel file: {}", e);
                                            }
                                        },
                                        Err(e) => {
                                            log::warn!("Failed to serialize panel data: {}", e);
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                info!("Save panel dialog cancelled or failed: {}", e);
                            }
                        }
                    });
                }
            }
        });
        action_group.add_action(&save_to_file_action);

        // Delete action - deletes all selected panels
        let panel_id_clone2 = panel_id.clone();
        let selected_panels_delete = self.selected_panels.clone();
        let panel_states_clone = self.panel_states.clone();
        let occupied_cells_clone = self.occupied_cells.clone();
        let container_clone = self.container.clone();
        let on_change_clone = self.on_change.clone();
        let panels_clone = self.panels.clone();
        let delete_action = gio::SimpleAction::new("delete", None);
        delete_action.connect_activate(move |_, _| {
            // Get all selected panels, or just the clicked panel if none selected
            let selected = selected_panels_delete.borrow();
            let panel_ids: Vec<String> =
                if selected.is_empty() || !selected.contains(&panel_id_clone2) {
                    vec![panel_id_clone2.clone()]
                } else {
                    selected.iter().cloned().collect()
                };
            let count = panel_ids.len();
            drop(selected);

            info!("Delete requested for {} panel(s)", count);

            // Show confirmation dialog
            use gtk4::AlertDialog;
            let dialog = AlertDialog::builder()
                .message(format!(
                    "Delete {} Panel{}?",
                    count,
                    if count > 1 { "s" } else { "" }
                ))
                .detail(format!(
                    "This will permanently delete the selected panel{}.",
                    if count > 1 { "s" } else { "" }
                ))
                .modal(true)
                .buttons(vec!["Cancel", "Delete"])
                .default_button(0)
                .cancel_button(0)
                .build();

            let selected_panels_for_delete = selected_panels_delete.clone();
            let panel_states_for_delete = panel_states_clone.clone();
            let occupied_cells_for_delete = occupied_cells_clone.clone();
            let container_for_delete = container_clone.clone();
            let on_change_for_delete = on_change_clone.clone();
            let panels_for_delete = panels_clone.clone();

            // We need a parent window for the dialog - get it from the container
            if let Some(root) = container_clone.root() {
                if let Some(window) = root.downcast_ref::<gtk4::Window>() {
                    dialog.choose(
                        Some(window),
                        gtk4::gio::Cancellable::NONE,
                        move |response| {
                            if let Ok(1) = response {
                                // Delete button clicked
                                delete_selected_panels(
                                    &panel_ids,
                                    &selected_panels_for_delete,
                                    &panel_states_for_delete,
                                    &occupied_cells_for_delete,
                                    &container_for_delete,
                                    &panels_for_delete,
                                    &on_change_for_delete,
                                );
                            }
                        },
                    );
                }
            }
        });
        action_group.add_action(&delete_action);

        widget.insert_action_group("panel", Some(&action_group));

        // Right-click gesture
        let gesture_secondary = GestureClick::new();
        gesture_secondary.set_button(3); // Right mouse button

        gesture_secondary.connect_pressed(move |gesture, _, x, y| {
            popover.set_pointing_to(Some(&gtk4::gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
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
        // Cache panel geometries at drag begin to avoid blocking reads during drag
        let cached_geometries: Rc<RefCell<HashMap<String, crate::core::PanelGeometry>>> =
            Rc::new(RefCell::new(HashMap::new()));
        // Cache ignore_collision flags to avoid blocking reads during drag_end
        let cached_ignore_collision: Rc<RefCell<HashMap<String, bool>>> =
            Rc::new(RefCell::new(HashMap::new()));

        // Clone for drag_begin closure
        let selected_panels_begin = selected_panels.clone();
        let panel_states_begin = panel_states.clone();
        let is_dragging_begin = is_dragging.clone();
        let drop_zone_begin = drop_zone_layer.clone();
        let panel_id_for_drag_begin = panel_id.clone();
        let dragged_panel_id_begin = dragged_panel_id.clone();
        let cached_geometries_begin = cached_geometries.clone();
        let cached_ignore_collision_begin = cached_ignore_collision.clone();

        drag_gesture.connect_drag_begin(move |gesture, _, _| {
            // Enable grid visualization
            *is_dragging_begin.borrow_mut() = true;
            drop_zone_begin.queue_draw();

            // Store which panel is being dragged
            *dragged_panel_id_begin.borrow_mut() = panel_id_for_drag_begin.clone();

            // Check if Ctrl is held (for multi-select drag)
            let modifiers = gesture.current_event_state();
            let ctrl_pressed = modifiers.contains(gtk4::gdk::ModifierType::CONTROL_MASK);

            // Ensure the dragged panel is in the selected set
            let mut selected = selected_panels_begin.borrow_mut();
            let mut states = panel_states_begin.borrow_mut();

            if !selected.contains(&panel_id_for_drag_begin) {
                if ctrl_pressed && !selected.is_empty() {
                    // Ctrl is held and there are already selected panels - ADD this panel to selection
                    selected.insert(panel_id_for_drag_begin.clone());
                    if let Some(state) = states.get_mut(&panel_id_for_drag_begin) {
                        state.selected = true;
                        state.frame.add_css_class("selected");
                    }
                } else {
                    // No Ctrl or no existing selection - clear and select only this panel

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
            }

            // Store initial positions and cache geometries of all selected panels
            let mut positions = initial_positions_clone.borrow_mut();
            let mut geometries = cached_geometries_begin.borrow_mut();
            let mut ignore_collision_cache = cached_ignore_collision_begin.borrow_mut();
            positions.clear();
            geometries.clear();
            ignore_collision_cache.clear();

            for id in selected.iter() {
                if let Some(state) = states.get(id) {
                    if let Some(parent) = state.frame.parent() {
                        if let Ok(fixed) = parent.downcast::<Fixed>() {
                            let pos = fixed.child_position(&state.frame);
                            positions.insert(id.clone(), pos);
                        }
                    }
                    // Cache the geometry and ignore_collision at drag begin to avoid blocking reads during drag
                    // Use blocking_read here since drag_begin only happens once (not at 60fps)
                    let panel_guard = state.panel.blocking_read();
                    geometries.insert(id.clone(), panel_guard.geometry);
                    ignore_collision_cache.insert(id.clone(), panel_guard.ignore_collision);
                }
            }
        });

        let initial_positions_clone2 = initial_positions.clone();
        let _frame_clone = frame.clone();

        // Clone for drag_update closure
        let config_for_update = config.clone();
        let selected_panels_update = selected_panels.clone();
        let drag_preview_cells_update = drag_preview_cells.clone();
        let drop_zone_layer_update = drop_zone_layer.clone();
        let dragged_panel_id_update = dragged_panel_id.clone();
        let cached_geometries_update = cached_geometries.clone();

        drag_gesture.connect_drag_update(move |_, offset_x, offset_y| {
            let config = config_for_update.borrow();
            let positions = initial_positions_clone2.borrow();
            let selected = selected_panels_update.borrow();
            let dragged_id = dragged_panel_id_update.borrow();
            let geometries = cached_geometries_update.borrow();

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

                // Get the dragged panel's original grid position from cache
                let dragged_panel_orig_grid = if let Some(geom) = geometries.get(&*dragged_id) {
                    (geom.x as i32, geom.y as i32)
                } else {
                    (0, 0)
                };

                // Calculate the grid offset from original position
                let grid_offset_x = dragged_grid_x - dragged_panel_orig_grid.0;
                let grid_offset_y = dragged_grid_y - dragged_panel_orig_grid.1;

                // Apply this offset to all selected panels using cached geometries
                for id in selected.iter() {
                    if let Some(geom) = geometries.get(id) {
                        log::debug!(
                            "[DRAG] Panel {} drag preview using cached geometry {}x{} at ({},{})",
                            id,
                            geom.width,
                            geom.height,
                            geom.x,
                            geom.y
                        );

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
        let cached_geometries_end = cached_geometries.clone();
        let cached_ignore_collision_end = cached_ignore_collision.clone();
        let app_config_for_copy = self.app_config.clone();

        drag_gesture.connect_drag_end(move |gesture, offset_x, offset_y| {
            let config = config_for_end.borrow();
            let selected = selected_panels_end.borrow();
            let states = panel_states_end.borrow();
            let mut occupied = occupied_cells_end.borrow_mut();
            let positions = initial_positions.borrow();
            let cached_geoms = cached_geometries_end.borrow();
            let cached_ignore = cached_ignore_collision_end.borrow();

            // Check if Ctrl key is held (copy mode)
            let modifiers = gesture.current_event_state();
            let is_copy_mode = modifiers.contains(gtk4::gdk::ModifierType::CONTROL_MASK);

            // Phase 1: Clear current occupied cells for ALL selected panels (only if moving, not copying)
            // Panels with ignore_collision don't participate in collision detection
            // Use cached values from drag_begin to avoid blocking reads
            if !is_copy_mode {
                for id in selected.iter() {
                    if states.contains_key(id) {
                        // Use cached ignore_collision and geometry from drag_begin
                        let ignore_collision = cached_ignore.get(id).copied().unwrap_or(false);
                        if !ignore_collision {
                            if let Some(geom) = cached_geoms.get(id) {
                                for dx in 0..geom.width {
                                    for dy in 0..geom.height {
                                        occupied.remove(&(geom.x + dx, geom.y + dy));
                                    }
                                }
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
                if states.contains_key(id) {
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


                    // Check if this panel would collide (skip for panels with ignore_collision)
                    // Use cached values from drag_begin to avoid blocking reads
                    let ignore_collision = cached_ignore.get(id).copied().unwrap_or(false);
                    if let Some(geom) = cached_geoms.get(id) {
                        if !ignore_collision {
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
                        }

                        // Calculate snapped pixel position
                        let snapped_x = grid_x as f64 * (config.cell_width + config.spacing) as f64;
                        let snapped_y = grid_y as f64 * (config.cell_height + config.spacing) as f64;

                        new_positions.push((id.clone(), grid_x, grid_y, snapped_x, snapped_y));
                    }
                }
            }

            // Phase 3: Apply movement/copy based on collision check
            if group_has_collision {
                // Restore ALL panels to original positions (only needed in move mode)
                // Only restore cells for panels that participate in collision detection
                // Use cached values from drag_begin to avoid blocking reads
                if !is_copy_mode {
                    for id in selected.iter() {
                        if states.contains_key(id) {
                            let ignore_collision = cached_ignore.get(id).copied().unwrap_or(false);
                            if !ignore_collision {
                                if let Some(geom) = cached_geoms.get(id) {
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
                        let (source_meta, displayer_id, config, background, corner_radius, border, geometry_size, scale, translate_x, translate_y, z_index, ignore_collision, panel_data) = {
                            let panel_guard = original_panel.blocking_read();
                            (
                                panel_guard.source.metadata().clone(),
                                panel_guard.displayer.id().to_string(),
                                panel_guard.config.clone(),
                                panel_guard.background.clone(),
                                panel_guard.corner_radius,
                                panel_guard.border.clone(),
                                (panel_guard.geometry.width, panel_guard.geometry.height),
                                panel_guard.scale,
                                panel_guard.translate_x,
                                panel_guard.translate_y,
                                panel_guard.z_index,
                                panel_guard.ignore_collision,
                                panel_guard.data.clone(),
                            )
                        };

                        // Generate unique ID for the copy
                        let new_id = format!("panel_{}", uuid::Uuid::new_v4());

                        // Create new panel with copied configuration
                        use crate::core::PanelGeometry;
                        let registry = crate::core::global_registry();
                        if let Ok(new_source) = registry.create_source(&source_meta.id) {
                            if let Ok(new_displayer) = registry.create_displayer(&displayer_id) {
                                let geometry = PanelGeometry {
                                    x: grid_x,
                                    y: grid_y,
                                    width: geometry_size.0,
                                    height: geometry_size.1,
                                };

                                let mut new_panel = Panel::new(
                                    new_id.clone(),
                                    geometry,
                                    new_source,
                                    new_displayer,
                                );

                                // Set the background, corner radius, border, scale, offset, z_index, and ignore_collision
                                new_panel.background = background;
                                new_panel.corner_radius = corner_radius;
                                new_panel.border = border;
                                new_panel.scale = scale;
                                new_panel.translate_x = translate_x;
                                new_panel.translate_y = translate_y;
                                new_panel.z_index = z_index;
                                new_panel.ignore_collision = ignore_collision;
                                new_panel.data = panel_data;

                                let new_panel = Arc::new(RwLock::new(new_panel));

                                // Apply the copied configuration
                                {
                                    let mut new_panel_guard = new_panel.blocking_write();
                                    let _ = new_panel_guard.apply_config(config);
                                }

                                // Add the copied panel to the grid
                                // Add to panels list
                                panels_for_copy.borrow_mut().push(new_panel.clone());

                                // Register with update manager so the panel gets periodic updates
                                if let Some(update_manager) = crate::core::global_update_manager() {
                                    update_manager.queue_add_panel(new_panel.clone());
                                }

                                // Mark new cells as occupied (only if panel participates in collision detection)
                                if !ignore_collision {
                                    let mut occupied_write = occupied_cells_end.borrow_mut();
                                    for dx in 0..geometry_size.0 {
                                        for dy in 0..geometry_size.1 {
                                            occupied_write.insert((grid_x + dx, grid_y + dy));
                                        }
                                    }
                                    drop(occupied_write);
                                }

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
                                let app_config_bg = app_config_for_copy.clone();
                                let background_area_weak = background_area.downgrade();
                                background_area.set_draw_func(move |_, cr, w, h| {
                                    match panel_clone_bg.try_read() {
                                        Ok(panel_guard) => {
                                            let width = w as f64;
                                            let height = h as f64;
                                            let radius = panel_guard.corner_radius.min(width / 2.0).min(height / 2.0);

                                            // Create rounded rectangle path
                                            cr.new_path();
                                            if radius > 0.0 {
                                                cr.arc(radius, radius, radius, std::f64::consts::PI, 3.0 * std::f64::consts::PI / 2.0);
                                                cr.arc(width - radius, radius, radius, 3.0 * std::f64::consts::PI / 2.0, 0.0);
                                                cr.arc(width - radius, height - radius, radius, 0.0, std::f64::consts::PI / 2.0);
                                                cr.arc(radius, height - radius, radius, std::f64::consts::PI / 2.0, std::f64::consts::PI);
                                                cr.close_path();
                                            } else {
                                                cr.rectangle(0.0, 0.0, width, height);
                                            }

                                            // Render background with clipping
                                            cr.save().ok();
                                            cr.clip();
                                            let source_values = panel_guard.source.get_values();
                                            let theme = app_config_bg.borrow().global_theme.clone();
                                            if let Err(e) = crate::ui::render_background_with_source_and_theme(cr, &panel_guard.background, width, height, &source_values, Some(&theme)) {
                                                log::warn!("Failed to render background: {}", e);
                                            }
                                            cr.restore().ok();

                                            // Render border if enabled
                                            if panel_guard.border.enabled {
                                                if radius > 0.0 {
                                                    cr.arc(radius, radius, radius, std::f64::consts::PI, 3.0 * std::f64::consts::PI / 2.0);
                                                    cr.arc(width - radius, radius, radius, 3.0 * std::f64::consts::PI / 2.0, 0.0);
                                                    cr.arc(width - radius, height - radius, radius, 0.0, std::f64::consts::PI / 2.0);
                                                    cr.arc(radius, height - radius, radius, std::f64::consts::PI / 2.0, std::f64::consts::PI);
                                                    cr.close_path();
                                                } else {
                                                    cr.rectangle(0.0, 0.0, width, height);
                                                }
                                                panel_guard.border.color.apply_to_cairo(cr);
                                                cr.set_line_width(panel_guard.border.width);
                                                cr.stroke().ok();
                                            }
                                        }
                                        Err(_) => {
                                            // Lock contention - schedule a retry on next frame
                                            log::debug!("Skipped background render due to lock contention, scheduling retry");
                                            if let Some(bg_area) = background_area_weak.upgrade() {
                                                gtk4::glib::idle_add_local_once(move || {
                                                    bg_area.queue_draw();
                                                });
                                            }
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
                                // Note: Corner radius is handled by Cairo rendering in background_area.set_draw_func
                                use gtk4::Frame;
                                let frame = Frame::new(None);
                                frame.set_child(Some(&overlay));
                                frame.set_size_request(width, height);

                                // Store panel state first (needed for interaction setup)
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

                                // Set up interaction for the copied panel
                                // We inline the setup code since we can't call setup_panel_interaction(&self)

                                // Setup context menu
                                let panel_for_menu = new_panel.clone();
                                let widget_for_menu = widget.clone();
                                let panel_id_for_menu = new_id.clone();
                                let config_for_menu = config_for_end.clone();
                                let panel_states_for_menu = panel_states_end.clone();
                                let occupied_cells_for_menu = occupied_cells_end.clone();
                                let on_change_for_menu = on_change_end.clone();
                                let drop_zone_for_menu = drop_zone_layer_end.clone();

                                // Create action group for context menu
                                use gtk4::gio;
                                let action_group = gio::SimpleActionGroup::new();

                                // Properties action
                                let properties_action = gio::SimpleAction::new("properties", None);
                                let panel_clone_props = panel_for_menu.clone();
                                let config_clone_props = config_for_menu.clone();
                                let panel_states_props = panel_states_for_menu.clone();
                                let occupied_cells_props = occupied_cells_for_menu.clone();
                                let on_change_props = on_change_for_menu.clone();
                                let drop_zone_props = drop_zone_for_menu.clone();
                                let panel_id_props = panel_id_for_menu.clone();
                                let container_props = container_for_copy.clone();
                                let selected_panels_props = selected_panels_end.clone();
                                let panels_props = panels_for_copy.clone();
                                let app_config_props = app_config_for_copy.clone();

                                properties_action.connect_activate(move |_, _| {
                                    log::info!("Opening properties dialog for copied panel: {}", panel_id_props);
                                    let registry = crate::core::global_registry();
                                    show_panel_properties_dialog(
                                        &panel_clone_props,
                                        *config_clone_props.borrow(),
                                        panel_states_props.clone(),
                                        occupied_cells_props.clone(),
                                        container_props.clone(),
                                        on_change_props.clone(),
                                        drop_zone_props.clone(),
                                        registry,
                                        selected_panels_props.clone(),
                                        panels_props.clone(),
                                        app_config_props.borrow().global_theme.clone(),
                                    );
                                });
                                action_group.add_action(&properties_action);

                                // Delete action - deletes all selected panels
                                let delete_action = gio::SimpleAction::new("delete", None);
                                let panel_id_del = panel_id_for_menu.clone();
                                let selected_panels_del = selected_panels_end.clone();
                                let panel_states_del = panel_states_for_menu.clone();
                                let occupied_cells_del = occupied_cells_for_menu.clone();
                                let panels_del = panels_for_copy.clone();
                                let on_change_del = on_change_for_menu.clone();
                                let container_del = container_for_copy.clone();

                                delete_action.connect_activate(move |_, _| {
                                    use gtk4::AlertDialog;

                                    // Get all selected panels, or just the clicked panel if none selected
                                    let selected = selected_panels_del.borrow();
                                    let panel_ids: Vec<String> = if selected.is_empty() || !selected.contains(&panel_id_del) {
                                        vec![panel_id_del.clone()]
                                    } else {
                                        selected.iter().cloned().collect()
                                    };
                                    let count = panel_ids.len();
                                    drop(selected);

                                    log::info!("Delete requested for {} panel(s)", count);

                                    let dialog = AlertDialog::builder()
                                        .message(format!("Delete {} Panel{}?", count, if count > 1 { "s" } else { "" }))
                                        .detail(format!("This will permanently delete the selected panel{}.", if count > 1 { "s" } else { "" }))
                                        .modal(true)
                                        .buttons(vec!["Cancel", "Delete"])
                                        .default_button(0)
                                        .cancel_button(0)
                                        .build();

                                    let selected_panels_confirm = selected_panels_del.clone();
                                    let panel_states_confirm = panel_states_del.clone();
                                    let occupied_cells_confirm = occupied_cells_del.clone();
                                    let panels_confirm = panels_del.clone();
                                    let on_change_confirm = on_change_del.clone();
                                    let container_confirm = container_del.clone();

                                    // We need a window for the dialog - try to get it from the container
                                    if let Some(root) = container_del.root() {
                                        if let Some(window) = root.downcast_ref::<gtk4::Window>() {
                                            dialog.choose(Some(window), gtk4::gio::Cancellable::NONE, move |response| {
                                                if let Ok(1) = response {
                                                    delete_selected_panels(
                                                        &panel_ids,
                                                        &selected_panels_confirm,
                                                        &panel_states_confirm,
                                                        &occupied_cells_confirm,
                                                        &container_confirm,
                                                        &panels_confirm,
                                                        &on_change_confirm,
                                                    );
                                                }
                                            });
                                        }
                                    }
                                });
                                action_group.add_action(&delete_action);

                                widget_for_menu.insert_action_group("panel", Some(&action_group));

                                // Setup left-click selection gesture
                                use gtk4::GestureClick;
                                let gesture_click = GestureClick::new();
                                let panel_states_click = panel_states_end.clone();
                                let selected_panels_click = selected_panels_end.clone();
                                let panel_id_click = new_id.clone();
                                let frame_click = frame.clone();

                                gesture_click.connect_pressed(move |gesture, _, _, _| {
                                    use gtk4::gdk::ModifierType;
                                    let modifiers = gesture.current_event_state();
                                    let ctrl_pressed = modifiers.contains(ModifierType::CONTROL_MASK);

                                    let mut states = panel_states_click.borrow_mut();
                                    let mut selected = selected_panels_click.borrow_mut();

                                    if ctrl_pressed {
                                        // Toggle selection
                                        if selected.contains(&panel_id_click) {
                                            selected.remove(&panel_id_click);
                                            if let Some(state) = states.get_mut(&panel_id_click) {
                                                state.selected = false;
                                                frame_click.remove_css_class("selected");
                                            }
                                        } else {
                                            selected.insert(panel_id_click.clone());
                                            if let Some(state) = states.get_mut(&panel_id_click) {
                                                state.selected = true;
                                                frame_click.add_css_class("selected");
                                            }
                                        }
                                    } else {
                                        // If clicking on an already-selected panel that's part of a multi-selection,
                                        // keep the current selection (to allow dragging multiple panels)
                                        // Otherwise, clear other selections and select only this panel
                                        if !selected.contains(&panel_id_click) || selected.len() == 1 {
                                            // Clear other selections
                                            for (id, state) in states.iter_mut() {
                                                if state.selected && id != &panel_id_click {
                                                    state.selected = false;
                                                    state.frame.remove_css_class("selected");
                                                }
                                            }
                                            selected.clear();

                                            // Select this panel
                                            selected.insert(panel_id_click.clone());
                                            if let Some(state) = states.get_mut(&panel_id_click) {
                                                state.selected = true;
                                                frame_click.add_css_class("selected");
                                            }
                                        }
                                        // else: panel is already selected in a multi-selection, keep it as-is
                                    }
                                });

                                widget_for_menu.add_controller(gesture_click);

                                // Add Copy Style action
                                let copy_style_action = gio::SimpleAction::new("copy_style", None);
                                let panel_copy_style = new_panel.clone();
                                copy_style_action.connect_activate(move |_, _| {
                                    log::info!("Copying panel style");
                                    let panel_guard = panel_copy_style.blocking_read();
                                    use crate::ui::{PanelStyle, CLIPBOARD};

                                    let mut displayer_config = panel_guard.config.clone();
                                    displayer_config.remove("cpu_config");
                                    displayer_config.remove("gpu_config");
                                    displayer_config.remove("memory_config");
                                    displayer_config.remove("disk_config");
                                    displayer_config.remove("clock_config");
                                    displayer_config.remove("combo_config");
                                    displayer_config.remove("system_temp_config");
                                    displayer_config.remove("fan_speed_config");
                                    displayer_config.remove("test_config");

                                    let style = PanelStyle {
                                        background: panel_guard.background.clone(),
                                        corner_radius: panel_guard.corner_radius,
                                        border: panel_guard.border.clone(),
                                        displayer_config,
                                    };

                                    if let Ok(mut clipboard) = CLIPBOARD.lock() {
                                        clipboard.copy_panel_style(style);
                                        log::info!("Panel style copied to clipboard");
                                    }
                                });
                                action_group.add_action(&copy_style_action);

                                // Add Paste Style action
                                let paste_style_action = gio::SimpleAction::new("paste_style", None);
                                let panel_paste_style = new_panel.clone();
                                let panel_states_paste = panel_states_end.clone();
                                let on_change_paste = on_change_end.clone();
                                let drop_zone_paste = drop_zone_layer_end.clone();
                                paste_style_action.connect_activate(move |_, _| {
                                    use crate::ui::CLIPBOARD;

                                    if let Ok(clipboard) = CLIPBOARD.lock() {
                                        if let Some(style) = clipboard.paste_panel_style() {
                                            log::info!("Pasting panel style");

                                            // Apply style and get panel ID, then drop the lock before calling callbacks
                                            let panel_id = {
                                                let mut panel_guard = panel_paste_style.blocking_write();
                                                panel_guard.background = style.background;
                                                panel_guard.corner_radius = style.corner_radius;
                                                panel_guard.border = style.border;

                                                for (key, value) in style.displayer_config {
                                                    panel_guard.config.insert(key, value);
                                                }

                                                let config_clone = panel_guard.config.clone();
                                                let _ = panel_guard.displayer.apply_config(&config_clone);

                                                panel_guard.id.clone()
                                            }; // panel_guard dropped here

                                            // Trigger redraw (after releasing panel lock)
                                            if let Some(state) = panel_states_paste.borrow().get(&panel_id) {
                                                state.background_area.queue_draw();
                                                state.widget.queue_draw();
                                            }

                                            // Trigger on_change callback (after releasing panel lock to avoid deadlock)
                                            if let Some(ref callback) = *on_change_paste.borrow() {
                                                callback();
                                            }

                                            drop_zone_paste.queue_draw();
                                            log::info!("Panel style pasted successfully");
                                        } else {
                                            log::info!("No panel style in clipboard");
                                        }
                                    }
                                });
                                action_group.add_action(&paste_style_action);

                                // Add Save to File action
                                let save_to_file_action = gio::SimpleAction::new("save_to_file", None);
                                let panel_save_file = new_panel.clone();
                                let container_for_save = container_for_copy.clone();
                                save_to_file_action.connect_activate(move |_, _| {
                                    log::info!("Saving panel to file");

                                    let panel_data = {
                                        let panel_guard = panel_save_file.blocking_read();
                                        panel_guard.to_data()
                                    };

                                    let data = panel_data;
                                    if let Some(root) = container_for_save.root() {
                                        if let Some(window) = root.downcast_ref::<gtk4::Window>() {
                                            let window_clone = window.clone();

                                            gtk4::glib::MainContext::default().spawn_local(async move {
                                                use gtk4::FileDialog;

                                                let initial_dir = directories::ProjectDirs::from("com", "github.hilgardt_collab", "rg-sens")
                                                    .map(|d| d.config_dir().to_path_buf())
                                                    .unwrap_or_else(|| std::path::PathBuf::from("/"));

                                                let json_filter = gtk4::FileFilter::new();
                                                json_filter.set_name(Some("JSON files"));
                                                json_filter.add_pattern("*.json");

                                                let all_filter = gtk4::FileFilter::new();
                                                all_filter.set_name(Some("All files"));
                                                all_filter.add_pattern("*");

                                                let filters = gio::ListStore::new::<gtk4::FileFilter>();
                                                filters.append(&json_filter);
                                                filters.append(&all_filter);

                                                let suggested_name = format!("panel_{}.json", data.id.replace("-", "_"));

                                                let file_dialog = FileDialog::builder()
                                                    .title("Save Panel to File")
                                                    .modal(true)
                                                    .initial_folder(&gio::File::for_path(&initial_dir))
                                                    .initial_name(&suggested_name)
                                                    .filters(&filters)
                                                    .default_filter(&json_filter)
                                                    .build();

                                                match file_dialog.save_future(Some(&window_clone)).await {
                                                    Ok(file) => {
                                                        if let Some(path) = file.path() {
                                                            log::info!("Saving panel to {:?}", path);

                                                            match serde_json::to_string_pretty(&data) {
                                                                Ok(json) => {
                                                                    match std::fs::write(&path, json) {
                                                                        Ok(()) => {
                                                                            log::info!("Panel saved successfully to {:?}", path);
                                                                        }
                                                                        Err(e) => {
                                                                            log::warn!("Failed to write panel file: {}", e);
                                                                        }
                                                                    }
                                                                }
                                                                Err(e) => {
                                                                    log::warn!("Failed to serialize panel data: {}", e);
                                                                }
                                                            }
                                                        }
                                                    }
                                                    Err(e) => {
                                                        log::info!("Save panel dialog cancelled or failed: {}", e);
                                                    }
                                                }
                                            });
                                        }
                                    }
                                });
                                action_group.add_action(&save_to_file_action);

                                // Setup right-click context menu
                                let gesture_secondary = GestureClick::new();
                                gesture_secondary.set_button(3);

                                use gtk4::{PopoverMenu, gio::Menu};
                                let menu = Menu::new();

                                // Section 1: Properties
                                let section1 = gio::Menu::new();
                                section1.append(Some("Properties..."), Some("panel.properties"));
                                menu.append_section(None, &section1);

                                // Section 2: Copy/Paste Style
                                let section2 = gio::Menu::new();
                                section2.append(Some("Copy Style"), Some("panel.copy_style"));
                                section2.append(Some("Paste Style"), Some("panel.paste_style"));
                                menu.append_section(None, &section2);

                                // Section 3: Save to File
                                let section3 = gio::Menu::new();
                                section3.append(Some("Save Panel to File..."), Some("panel.save_to_file"));
                                menu.append_section(None, &section3);

                                // Section 4: Delete
                                let section4 = gio::Menu::new();
                                section4.append(Some("Delete"), Some("panel.delete"));
                                menu.append_section(None, &section4);

                                let popover = PopoverMenu::from_model(Some(&menu));
                                popover.set_parent(&widget_for_menu);
                                popover.set_has_arrow(false);

                                // Ensure popover is unparented when widget is destroyed
                                let popover_weak = popover.downgrade();
                                widget_for_menu.connect_destroy(move |_| {
                                    if let Some(p) = popover_weak.upgrade() {
                                        p.unparent();
                                    }
                                });

                                gesture_secondary.connect_pressed(move |_gesture, _, x, y| {
                                    popover.set_pointing_to(Some(&gtk4::gdk::Rectangle::new(
                                        x as i32,
                                        y as i32,
                                        1,
                                        1,
                                    )));
                                    popover.popup();
                                });

                                widget_for_menu.add_controller(gesture_secondary);

                                // Setup drag gesture for copied panel
                                use gtk4::GestureDrag;
                                let drag_gesture_copy = GestureDrag::new();
                                drag_gesture_copy.set_button(1);

                                // Clone variables for nested closure
                                let config_for_nested = config_for_end.clone();
                                let container_for_nested = container_for_copy.clone();

                                // Store initial positions and the ID of the panel being dragged
                                let initial_positions_copy: Rc<RefCell<HashMap<String, (f64, f64)>>> =
                                    Rc::new(RefCell::new(HashMap::new()));
                                let dragged_panel_id_copy: Rc<RefCell<String>> = Rc::new(RefCell::new(String::new()));

                                // Clone for drag_begin
                                let initial_positions_begin = initial_positions_copy.clone();
                                let dragged_panel_id_begin = dragged_panel_id_copy.clone();
                                let selected_panels_drag_begin = selected_panels_end.clone();
                                let panel_states_drag_begin = panel_states_end.clone();
                                let is_dragging_drag_begin = is_dragging_end.clone();
                                let drop_zone_drag_begin = drop_zone_layer_end.clone();
                                let panel_id_drag_begin = new_id.clone();

                                drag_gesture_copy.connect_drag_begin(move |gesture, _, _| {
                                    *is_dragging_drag_begin.borrow_mut() = true;
                                    drop_zone_drag_begin.queue_draw();

                                    *dragged_panel_id_begin.borrow_mut() = panel_id_drag_begin.clone();

                                    // Check if Ctrl is held (for multi-select drag)
                                    let modifiers = gesture.current_event_state();
                                    let ctrl_pressed = modifiers.contains(gtk4::gdk::ModifierType::CONTROL_MASK);

                                    let mut selected = selected_panels_drag_begin.borrow_mut();
                                    let mut states = panel_states_drag_begin.borrow_mut();

                                    if !selected.contains(&panel_id_drag_begin) {
                                        if ctrl_pressed && !selected.is_empty() {
                                            // Ctrl is held - ADD this panel to selection
                                            selected.insert(panel_id_drag_begin.clone());
                                            if let Some(state) = states.get_mut(&panel_id_drag_begin) {
                                                state.selected = true;
                                                state.frame.add_css_class("selected");
                                            }
                                        } else {
                                            // No Ctrl - clear and select only this panel
                                            for (id, state) in states.iter_mut() {
                                                if selected.contains(id) {
                                                    state.selected = false;
                                                    state.frame.remove_css_class("selected");
                                                }
                                            }
                                            selected.clear();

                                            selected.insert(panel_id_drag_begin.clone());
                                            if let Some(state) = states.get_mut(&panel_id_drag_begin) {
                                                state.selected = true;
                                                state.frame.add_css_class("selected");
                                            }
                                        }
                                    }

                                    let mut positions = initial_positions_begin.borrow_mut();
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

                                // Clone for drag_update
                                let initial_positions_update = initial_positions_copy.clone();
                                let dragged_panel_id_update = dragged_panel_id_copy.clone();
                                let config_drag_update = config_for_end.clone();
                                let selected_panels_drag_update = selected_panels_end.clone();
                                let panel_states_drag_update = panel_states_end.clone();
                                let drag_preview_cells_drag_update = drag_preview_cells_end.clone();
                                let drop_zone_drag_update = drop_zone_layer_end.clone();

                                drag_gesture_copy.connect_drag_update(move |_, offset_x, offset_y| {
                                    let config = config_drag_update.borrow();
                                    let positions = initial_positions_update.borrow();
                                    let selected = selected_panels_drag_update.borrow();
                                    let states = panel_states_drag_update.borrow();
                                    let dragged_id = dragged_panel_id_update.borrow();

                                    let mut preview_rects = Vec::new();

                                    if let Some((dragged_orig_x, dragged_orig_y)) = positions.get(&*dragged_id) {
                                        let dragged_new_x = dragged_orig_x + offset_x;
                                        let dragged_new_y = dragged_orig_y + offset_y;

                                        let dragged_grid_x = ((dragged_new_x + config.cell_width as f64 / 2.0)
                                            / (config.cell_width + config.spacing) as f64)
                                            .floor() as i32;
                                        let dragged_grid_y = ((dragged_new_y + config.cell_height as f64 / 2.0)
                                            / (config.cell_height + config.spacing) as f64)
                                            .floor() as i32;

                                        let dragged_panel_orig_grid = if let Some(state) = states.get(&*dragged_id) {
                                            let geom = state.panel.blocking_read().geometry;
                                            (geom.x as i32, geom.y as i32)
                                        } else {
                                            (0, 0)
                                        };

                                        let grid_offset_x = dragged_grid_x - dragged_panel_orig_grid.0;
                                        let grid_offset_y = dragged_grid_y - dragged_panel_orig_grid.1;

                                        for id in selected.iter() {
                                            if let Some(state) = states.get(id) {
                                                let geom = state.panel.blocking_read().geometry;
                                                let new_grid_x = (geom.x as i32 + grid_offset_x).max(0) as u32;
                                                let new_grid_y = (geom.y as i32 + grid_offset_y).max(0) as u32;
                                                preview_rects.push((new_grid_x, new_grid_y, geom.width, geom.height));
                                            }
                                        }
                                    }

                                    let mut preview_cells = drag_preview_cells_drag_update.borrow_mut();
                                    if *preview_cells != preview_rects {
                                        *preview_cells = preview_rects;
                                        drop(preview_cells);
                                        drop_zone_drag_update.queue_draw();
                                    }
                                });

                                // Clone for drag_end
                                let initial_positions_drag_end = initial_positions_copy.clone();
                                let config_drag_end = config_for_end.clone();
                                let selected_panels_drag_end = selected_panels_end.clone();
                                let panel_states_drag_end = panel_states_end.clone();
                                let occupied_cells_drag_end = occupied_cells_end.clone();
                                let drag_preview_cells_drag_end = drag_preview_cells_end.clone();
                                let is_dragging_drag_end = is_dragging_end.clone();
                                let drop_zone_drag_end = drop_zone_layer_end.clone();
                                let on_change_drag_end = on_change_end.clone();
                                let panels_drag_end = panels_for_copy.clone();
                                let app_config_drag_end = app_config_for_copy.clone();

                                drag_gesture_copy.connect_drag_end(move |gesture, offset_x, offset_y| {
                                    let config = config_drag_end.borrow();
                                    let selected = selected_panels_drag_end.borrow();
                                    let states = panel_states_drag_end.borrow();
                                    let mut occupied = occupied_cells_drag_end.borrow_mut();
                                    let positions = initial_positions_drag_end.borrow();

                                    // Check if Ctrl key is held (copy mode)
                                    let modifiers = gesture.current_event_state();
                                    let is_copy_mode = modifiers.contains(gtk4::gdk::ModifierType::CONTROL_MASK);

                                    // Phase 1: Clear occupied cells (only if moving, not copying)
                                    // Panels with ignore_collision don't participate in collision detection
                                    if !is_copy_mode {
                                        for id in selected.iter() {
                                            if let Some(state) = states.get(id) {
                                                let panel_guard = state.panel.blocking_read();
                                                if !panel_guard.ignore_collision {
                                                    let geom = panel_guard.geometry;
                                                    for dx in 0..geom.width {
                                                        for dy in 0..geom.height {
                                                            occupied.remove(&(geom.x + dx, geom.y + dy));
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    // Phase 2: Calculate new positions
                                    let mut new_positions: Vec<(String, u32, u32, f64, f64)> = Vec::new();
                                    let mut group_has_collision = false;

                                    for id in selected.iter() {
                                        if let Some(state) = states.get(id) {
                                            if let Some((orig_x, orig_y)) = positions.get(id) {
                                                let new_x = orig_x + offset_x;
                                                let new_y = orig_y + offset_y;

                                                let grid_x = ((new_x + config.cell_width as f64 / 2.0)
                                                    / (config.cell_width + config.spacing) as f64)
                                                    .floor()
                                                    .max(0.0) as u32;
                                                let grid_y = ((new_y + config.cell_height as f64 / 2.0)
                                                    / (config.cell_height + config.spacing) as f64)
                                                    .floor()
                                                    .max(0.0) as u32;

                                                let snapped_x = grid_x as f64 * (config.cell_width + config.spacing) as f64;
                                                let snapped_y = grid_y as f64 * (config.cell_height + config.spacing) as f64;

                                                // Check collision (skip for panels with ignore_collision)
                                                let panel_guard = state.panel.blocking_read();
                                                let geom = panel_guard.geometry;
                                                if !panel_guard.ignore_collision {
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
                                                }

                                                new_positions.push((id.clone(), grid_x, grid_y, snapped_x, snapped_y));
                                            }
                                        }
                                    }

                                    // Phase 3: Apply changes
                                    if group_has_collision && !is_copy_mode {
                                        // Restore original positions (only for panels that participate in collision)
                                        for id in selected.iter() {
                                            if let Some(state) = states.get(id) {
                                                let panel_guard = state.panel.blocking_read();
                                                if !panel_guard.ignore_collision {
                                                    let geom = panel_guard.geometry;
                                                    for dx in 0..geom.width {
                                                        for dy in 0..geom.height {
                                                            occupied.insert((geom.x + dx, geom.y + dy));
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    } else if !group_has_collision {
                                        drop(states);
                                        drop(selected);
                                        drop(occupied);

                                        if is_copy_mode {
                                            // Copy mode - create new panels at target positions
                                            log::info!("Creating nested copies of {} panels", new_positions.len());

                                            use crate::core::Panel;

                                            for (old_id, grid_x, grid_y, _snapped_x, _snapped_y) in new_positions {
                                                // Get the source panel to copy
                                                let panel_states_read = panel_states_drag_end.borrow();
                                                if let Some(state) = panel_states_read.get(&old_id) {
                                                    let original_panel = state.panel.clone();
                                                    drop(panel_states_read);

                                                    // Read original panel configuration
                                                    let (source_meta, displayer_id, config, background, corner_radius, border, geometry_size, scale, translate_x, translate_y, z_index, ignore_collision, panel_data) = {
                                                        let panel_guard = original_panel.blocking_read();
                                                        (
                                                            panel_guard.source.metadata().clone(),
                                                            panel_guard.displayer.id().to_string(),
                                                            panel_guard.config.clone(),
                                                            panel_guard.background.clone(),
                                                            panel_guard.corner_radius,
                                                            panel_guard.border.clone(),
                                                            (panel_guard.geometry.width, panel_guard.geometry.height),
                                                            panel_guard.scale,
                                                            panel_guard.translate_x,
                                                            panel_guard.translate_y,
                                                            panel_guard.z_index,
                                                            panel_guard.ignore_collision,
                                                            panel_guard.data.clone(),
                                                        )
                                                    };

                                                    // Generate unique ID for the new copy
                                                    let new_id = format!("panel_{}", uuid::Uuid::new_v4());

                                                    // Create new panel instance
                                                    use crate::core::PanelGeometry;
                                                    let registry = crate::core::global_registry();
                                                    if let Ok(new_source) = registry.create_source(&source_meta.id) {
                                                        if let Ok(new_displayer) = registry.create_displayer(&displayer_id) {
                                                            let geometry = PanelGeometry {
                                                                x: grid_x,
                                                                y: grid_y,
                                                                width: geometry_size.0,
                                                                height: geometry_size.1,
                                                            };

                                                            let mut new_panel = Panel::new(
                                                                new_id.clone(),
                                                                geometry,
                                                                new_source,
                                                                new_displayer,
                                                            );

                                                            // Copy all settings including scale, offset, z_index, and ignore_collision
                                                            new_panel.background = background;
                                                            new_panel.corner_radius = corner_radius;
                                                            new_panel.border = border;
                                                            new_panel.scale = scale;
                                                            new_panel.translate_x = translate_x;
                                                            new_panel.translate_y = translate_y;
                                                            new_panel.z_index = z_index;
                                                            new_panel.ignore_collision = ignore_collision;
                                                            new_panel.data = panel_data;

                                                            let new_panel = Arc::new(RwLock::new(new_panel));

                                                            // Apply configuration
                                                            {
                                                                let mut panel_guard = new_panel.blocking_write();
                                                                let _ = panel_guard.apply_config(config);
                                                            }

                                                            // Add to panels list
                                                            panels_drag_end.borrow_mut().push(new_panel.clone());

                                                            // Mark cells as occupied (only if panel participates in collision detection)
                                                            if !ignore_collision {
                                                                let mut occupied_write = occupied_cells_drag_end.borrow_mut();
                                                                for dx in 0..geometry_size.0 {
                                                                    for dy in 0..geometry_size.1 {
                                                                        occupied_write.insert((grid_x + dx, grid_y + dy));
                                                                    }
                                                                }
                                                                drop(occupied_write);
                                                            }

                                                            // Create UI for the nested copy (similar to add_panel)
                                                            let config_read = config_for_nested.borrow();
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
                                                            let app_config_bg = app_config_drag_end.clone();
                                                            let background_area_weak = background_area.downgrade();
                                                            background_area.set_draw_func(move |_, cr, w, h| {
                                                                match panel_clone_bg.try_read() {
                                                                    Ok(panel_guard) => {
                                                                        let width = w as f64;
                                                                        let height = h as f64;
                                                                        let radius = panel_guard.corner_radius.min(width / 2.0).min(height / 2.0);

                                                                        cr.new_path();
                                                                        if radius > 0.0 {
                                                                            cr.arc(radius, radius, radius, std::f64::consts::PI, 3.0 * std::f64::consts::PI / 2.0);
                                                                            cr.arc(width - radius, radius, radius, 3.0 * std::f64::consts::PI / 2.0, 0.0);
                                                                            cr.arc(width - radius, height - radius, radius, 0.0, std::f64::consts::PI / 2.0);
                                                                            cr.arc(radius, height - radius, radius, std::f64::consts::PI / 2.0, std::f64::consts::PI);
                                                                            cr.close_path();
                                                                        } else {
                                                                            cr.rectangle(0.0, 0.0, width, height);
                                                                        }

                                                                        cr.save().ok();
                                                                        cr.clip();
                                                                        let source_values = panel_guard.source.get_values();
                                                                        let theme = app_config_bg.borrow().global_theme.clone();
                                                                        if let Err(e) = crate::ui::render_background_with_source_and_theme(cr, &panel_guard.background, width, height, &source_values, Some(&theme)) {
                                                                            log::warn!("Failed to render background: {}", e);
                                                                        }
                                                                        cr.restore().ok();

                                                                        if panel_guard.border.enabled {
                                                                            if radius > 0.0 {
                                                                                cr.arc(radius, radius, radius, std::f64::consts::PI, 3.0 * std::f64::consts::PI / 2.0);
                                                                                cr.arc(width - radius, radius, radius, 3.0 * std::f64::consts::PI / 2.0, 0.0);
                                                                                cr.arc(width - radius, height - radius, radius, 0.0, std::f64::consts::PI / 2.0);
                                                                                cr.arc(radius, height - radius, radius, std::f64::consts::PI / 2.0, std::f64::consts::PI);
                                                                                cr.close_path();
                                                                            } else {
                                                                                cr.rectangle(0.0, 0.0, width, height);
                                                                            }
                                                                            panel_guard.border.color.apply_to_cairo(cr);
                                                                            cr.set_line_width(panel_guard.border.width);
                                                                            cr.stroke().ok();
                                                                        }
                                                                    }
                                                                    Err(_) => {
                                                                        // Lock contention - schedule a retry on next frame
                                                                        log::debug!("Skipped background render due to lock contention, scheduling retry");
                                                                        if let Some(bg_area) = background_area_weak.upgrade() {
                                                                            gtk4::glib::idle_add_local_once(move || {
                                                                                bg_area.queue_draw();
                                                                            });
                                                                        }
                                                                    }
                                                                }
                                                            });

                                                            // Create overlay
                                                            use gtk4::Overlay;
                                                            let overlay = Overlay::new();
                                                            overlay.set_child(Some(&background_area));
                                                            widget.add_css_class("transparent-background");
                                                            overlay.add_overlay(&widget);

                                                            // Create frame
                                                            use gtk4::Frame;
                                                            let frame = Frame::new(None);
                                                            frame.set_child(Some(&overlay));
                                                            frame.set_size_request(width, height);

                                                            // Store panel state
                                                            panel_states_drag_end.borrow_mut().insert(
                                                                new_id.clone(),
                                                                PanelState {
                                                                    widget: widget.clone(),
                                                                    frame: frame.clone(),
                                                                    panel: new_panel.clone(),
                                                                    selected: false,
                                                                    background_area: background_area.clone(),
                                                                },
                                                            );

                                                            // Setup interaction for nested copy
                                                            // Use idle_add to defer setup until after current borrow is released
                                                            let widget_for_interaction = widget.clone();
                                                            let frame_for_interaction = frame.clone();
                                                            let new_panel_for_interaction = new_panel.clone();
                                                            let new_id_for_interaction = new_id.clone();
                                                            let panel_states_for_interaction = panel_states_drag_end.clone();
                                                            let selected_panels_for_interaction = selected_panels_drag_end.clone();
                                                            let occupied_cells_for_interaction = occupied_cells_drag_end.clone();
                                                            let config_for_interaction = config_for_nested.clone();
                                                            let container_for_interaction = container_for_nested.clone();
                                                            let on_change_for_interaction = on_change_drag_end.clone();
                                                            let drop_zone_for_interaction = drop_zone_drag_end.clone();
                                                            let panels_for_interaction = panels_drag_end.clone();
                                                            let is_dragging_for_interaction = is_dragging_drag_end.clone();
                                                            let drag_preview_cells_for_interaction = drag_preview_cells_drag_end.clone();
                                                            let app_config_for_interaction = app_config_drag_end.clone();

                                                            gtk4::glib::idle_add_local_once(move || {
                                                                setup_copied_panel_interaction(
                                                                    &widget_for_interaction,
                                                                    &frame_for_interaction,
                                                                    new_panel_for_interaction,
                                                                    new_id_for_interaction,
                                                                    panel_states_for_interaction,
                                                                    selected_panels_for_interaction,
                                                                    occupied_cells_for_interaction,
                                                                    config_for_interaction,
                                                                    container_for_interaction,
                                                                    on_change_for_interaction,
                                                                    drop_zone_for_interaction,
                                                                    panels_for_interaction,
                                                                    is_dragging_for_interaction,
                                                                    drag_preview_cells_for_interaction,
                                                                    app_config_for_interaction,
                                                                );
                                                            });

                                                            // Add to container
                                                            container_for_nested.put(&frame, x as f64, y as f64);

                                                            // Update the panel with initial data
                                                            {
                                                                let mut panel_guard = new_panel.blocking_write();
                                                                let _ = panel_guard.update();
                                                            }

                                                            log::info!("Created and displayed nested copy panel: {} at ({}, {})", new_id, grid_x, grid_y);
                                                        }
                                                    }
                                                }
                                            }

                                            // Trigger change callback
                                            if let Some(ref callback) = *on_change_drag_end.borrow() {
                                                callback();
                                            }
                                        } else {
                                            // Move mode
                                            let _selected = selected_panels_drag_end.borrow();
                                            let states = panel_states_drag_end.borrow();
                                            let mut occupied = occupied_cells_drag_end.borrow_mut();

                                            for (id, grid_x, grid_y, snapped_x, snapped_y) in new_positions {
                                                if let Some(state) = states.get(&id) {
                                                    if let Some(parent) = state.frame.parent() {
                                                        if let Ok(fixed) = parent.downcast::<Fixed>() {
                                                            fixed.move_(&state.frame, snapped_x, snapped_y);
                                                        }
                                                    }

                                                    {
                                                        let mut panel_guard = state.panel.blocking_write();
                                                        panel_guard.geometry.x = grid_x;
                                                        panel_guard.geometry.y = grid_y;
                                                    }

                                                    // Mark new cells as occupied (only if panel participates in collision)
                                                    let panel_guard = state.panel.blocking_read();
                                                    if !panel_guard.ignore_collision {
                                                        let geom = panel_guard.geometry;
                                                        for dx in 0..geom.width {
                                                            for dy in 0..geom.height {
                                                                occupied.insert((grid_x + dx, grid_y + dy));
                                                            }
                                                        }
                                                    }
                                                }
                                            }

                                            if let Some(ref callback) = *on_change_drag_end.borrow() {
                                                callback();
                                            }
                                        }
                                    }

                                    // Clear preview and disable grid
                                    *drag_preview_cells_drag_end.borrow_mut() = Vec::new();
                                    *is_dragging_drag_end.borrow_mut() = false;
                                    drop_zone_drag_end.queue_draw();
                                });

                                frame.add_controller(drag_gesture_copy);

                                // Add to container
                                container_for_copy.put(&frame, x as f64, y as f64);

                                // Update the panel to populate the displayer with data
                                {
                                    let mut panel_guard = new_panel.blocking_write();
                                    let _ = panel_guard.update();
                                }

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
                        {
                            let mut panel_guard = state.panel.blocking_write();
                            panel_guard.geometry.x = grid_x;
                            panel_guard.geometry.y = grid_y;
                        }

                        // Mark new cells as occupied (only if panel participates in collision)
                        let panel_guard = state.panel.blocking_read();
                        if !panel_guard.ignore_collision {
                            let geom = panel_guard.geometry;
                            for dx in 0..geom.width {
                                for dy in 0..geom.height {
                                    occupied.insert((grid_x + dx, grid_y + dy));
                                }
                            }
                        }
                    }
                }
            }

            // Update container size to fit all content (for scrolling)
            {
                let config = config_for_end.borrow();
                let states = panel_states_end.borrow();
                let cell_width = config.cell_width;
                let cell_height = config.cell_height;
                let spacing = config.spacing;

                let mut max_width = config.columns as i32 * (cell_width + spacing) - spacing;
                let mut max_height = config.rows as i32 * (cell_height + spacing) - spacing;

                for (_panel_id, state) in states.iter() {
                    let panel_guard = state.panel.blocking_read();
                    let geom = &panel_guard.geometry;
                    let panel_right = (geom.x + geom.width) as i32 * (cell_width + spacing) - spacing;
                    let panel_bottom = (geom.y + geom.height) as i32 * (cell_height + spacing) - spacing;
                    max_width = max_width.max(panel_right);
                    max_height = max_height.max(panel_bottom);
                }

                container_for_copy.set_size_request(max_width, max_height);
                drop_zone_layer_end.set_size_request(max_width, max_height);
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
        // Find position first, then release borrow before mutating
        let pos = self
            .panels
            .borrow()
            .iter()
            .position(|p| p.blocking_read().id == panel_id);

        if let Some(pos) = pos {
            let panel = self.panels.borrow_mut().remove(pos);

            // Unregister from update manager to stop updates
            if let Some(update_manager) = crate::core::global_update_manager() {
                update_manager.queue_remove_panel(panel_id.to_string());
            }

            // Remove from states and container
            if let Some(state) = self.panel_states.borrow_mut().remove(panel_id) {
                // Clean up event controllers and action groups to prevent memory leaks
                cleanup_widget_controllers(&state.widget);
                cleanup_widget_controllers(&state.frame);
                cleanup_widget_controllers(&state.background_area);

                self.container.remove(&state.frame);

                // Clear occupied cells (only if panel participated in collision detection)
                let panel_guard = state.panel.blocking_read();
                if !panel_guard.ignore_collision {
                    let geom = panel_guard.geometry;
                    drop(panel_guard); // Release lock before borrowing occupied_cells
                    let mut occupied = self.occupied_cells.borrow_mut();
                    for dx in 0..geom.width {
                        for dy in 0..geom.height {
                            occupied.remove(&(geom.x + dx, geom.y + dy));
                        }
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

    /// Remove all panels from the grid
    pub fn clear_all_panels(&mut self) {
        // Get all panel IDs
        let panel_ids: Vec<String> = self
            .panels
            .borrow()
            .iter()
            .map(|p| p.blocking_read().id.clone())
            .collect();

        // Remove each panel
        for panel_id in panel_ids {
            self.remove_panel(&panel_id);
        }
    }

    pub fn widget(&self) -> Widget {
        self.overlay.clone().upcast()
    }

    /// Get the current content size (width, height) in pixels
    /// This accounts for panels that may extend beyond the default grid bounds
    pub fn get_content_size(&self) -> (i32, i32) {
        let config = self.config.borrow();
        let cell_width = config.cell_width;
        let cell_height = config.cell_height;
        let spacing = config.spacing;

        // Start with config-based size
        let mut max_width = config.columns as i32 * (cell_width + spacing) - spacing;
        let mut max_height = config.rows as i32 * (cell_height + spacing) - spacing;

        // Check if any panels extend beyond the default grid bounds
        for (_panel_id, state) in self.panel_states.borrow().iter() {
            let panel_guard = state.panel.blocking_read();
            let geom = &panel_guard.geometry;

            // Calculate pixel position + size for this panel
            let panel_right = (geom.x + geom.width) as i32 * (cell_width + spacing) - spacing;
            let panel_bottom = (geom.y + geom.height) as i32 * (cell_height + spacing) - spacing;

            max_width = max_width.max(panel_right);
            max_height = max_height.max(panel_bottom);
        }

        (max_width, max_height)
    }

    /// Update the container size to fit all content
    /// Call this after adding/moving panels to ensure scrolling works correctly
    pub fn update_content_bounds(&self) {
        let (width, height) = self.get_content_size();
        self.container.set_size_request(width, height);
        self.drop_zone_layer.set_size_request(width, height);
    }

    /// Set the viewport (window) dimensions for auto-scroll boundary visualization
    /// These rectangles are shown when dragging panels to help with placement
    pub fn set_viewport_size(&self, width: i32, height: i32) {
        *self.viewport_size.borrow_mut() = (width, height);
        // Trigger redraw of drop zone layer to update viewport boundaries
        self.drop_zone_layer.queue_draw();
    }

    /// Trigger redraw of all panels (useful when global theme changes)
    pub fn queue_redraw_all_panels(&self) {
        let states = self.panel_states.borrow();
        for state in states.values() {
            state.background_area.queue_draw();
            state.widget.queue_draw();
        }
    }

    /// Show or hide the grid overlay (cell grid and viewport boundaries)
    /// Used for space bar hold and window resize visualization
    pub fn set_grid_visible(&self, visible: bool) {
        *self.is_dragging.borrow_mut() = visible;
        self.drop_zone_layer.queue_draw();
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
            let panel_guard = state.panel.blocking_read();
            let geom = &panel_guard.geometry;

            // Calculate new pixel dimensions
            let pixel_width = geom.width as i32 * cell_width + (geom.width as i32 - 1) * spacing;
            let pixel_height =
                geom.height as i32 * cell_height + (geom.height as i32 - 1) * spacing;
            let x = (geom.x as i32 * (cell_width + spacing)) as f64;
            let y = (geom.y as i32 * (cell_height + spacing)) as f64;

            // Update frame size and position
            state.frame.set_size_request(pixel_width, pixel_height);
            self.container.move_(&state.frame, x, y);

            // Update widget size
            state.widget.set_size_request(pixel_width, pixel_height);

            // Update background area size
            state
                .background_area
                .set_size_request(pixel_width, pixel_height);
        }

        self.drop_zone_layer.queue_draw();
    }

    pub fn set_config(&mut self, new_config: GridConfig) {
        *self.config.borrow_mut() = new_config;
        let width = new_config.columns as i32 * (new_config.cell_width + new_config.spacing)
            - new_config.spacing;
        let height = new_config.rows as i32 * (new_config.cell_height + new_config.spacing)
            - new_config.spacing;
        self.container.set_size_request(width, height);
        self.drop_zone_layer.set_size_request(width, height);
    }

    pub fn config(&self) -> GridConfig {
        *self.config.borrow()
    }
}

/// Helper function to setup interaction for a copied panel
/// This is called via idle_add to avoid borrow conflicts during copy operations
#[allow(clippy::too_many_arguments)]
fn setup_copied_panel_interaction(
    widget: &Widget,
    frame: &Frame,
    panel: Arc<RwLock<Panel>>,
    panel_id: String,
    panel_states: Rc<RefCell<HashMap<String, PanelState>>>,
    selected_panels: Rc<RefCell<HashSet<String>>>,
    occupied_cells: Rc<RefCell<HashSet<(u32, u32)>>>,
    config: Rc<RefCell<GridConfig>>,
    container: Fixed,
    on_change: Rc<RefCell<Option<Box<dyn Fn()>>>>,
    drop_zone_layer: DrawingArea,
    panels: Rc<RefCell<Vec<Arc<RwLock<Panel>>>>>,
    is_dragging: Rc<RefCell<bool>>,
    drag_preview_cells: Rc<RefCell<Vec<(u32, u32, u32, u32)>>>,
    app_config: Rc<RefCell<crate::config::AppConfig>>,
) {
    use gtk4::gio;

    // Setup click selection gesture
    let gesture_click = GestureClick::new();
    let panel_states_click = panel_states.clone();
    let selected_panels_click = selected_panels.clone();
    let panel_id_click = panel_id.clone();
    let frame_click = frame.clone();

    gesture_click.connect_pressed(move |gesture, _, _, _| {
        use gtk4::gdk::ModifierType;
        let modifiers = gesture.current_event_state();
        let ctrl_pressed = modifiers.contains(ModifierType::CONTROL_MASK);

        let mut states = panel_states_click.borrow_mut();
        let mut selected = selected_panels_click.borrow_mut();

        if ctrl_pressed {
            if selected.contains(&panel_id_click) {
                selected.remove(&panel_id_click);
                if let Some(state) = states.get_mut(&panel_id_click) {
                    state.selected = false;
                    frame_click.remove_css_class("selected");
                }
            } else {
                selected.insert(panel_id_click.clone());
                if let Some(state) = states.get_mut(&panel_id_click) {
                    state.selected = true;
                    frame_click.add_css_class("selected");
                }
            }
        } else if !selected.contains(&panel_id_click) || selected.len() == 1 {
            for (id, state) in states.iter_mut() {
                if state.selected && id != &panel_id_click {
                    state.selected = false;
                    state.frame.remove_css_class("selected");
                }
            }
            selected.clear();
            selected.insert(panel_id_click.clone());
            if let Some(state) = states.get_mut(&panel_id_click) {
                state.selected = true;
                frame_click.add_css_class("selected");
            }
        }
    });
    widget.add_controller(gesture_click);

    // Setup action group
    let action_group = gio::SimpleActionGroup::new();

    // Properties action
    let properties_action = gio::SimpleAction::new("properties", None);
    let panel_props = panel.clone();
    let config_props = config.clone();
    let panel_states_props = panel_states.clone();
    let occupied_cells_props = occupied_cells.clone();
    let on_change_props = on_change.clone();
    let drop_zone_props = drop_zone_layer.clone();
    let container_props = container.clone();
    let selected_panels_props = selected_panels.clone();
    let panels_props = panels.clone();
    let panel_id_props = panel_id.clone();
    let app_config_props = app_config.clone();

    properties_action.connect_activate(move |_, _| {
        log::info!("Opening properties dialog for panel: {}", panel_id_props);
        let registry = crate::core::global_registry();
        show_panel_properties_dialog(
            &panel_props,
            *config_props.borrow(),
            panel_states_props.clone(),
            occupied_cells_props.clone(),
            container_props.clone(),
            on_change_props.clone(),
            drop_zone_props.clone(),
            registry,
            selected_panels_props.clone(),
            panels_props.clone(),
            app_config_props.borrow().global_theme.clone(),
        );
    });
    action_group.add_action(&properties_action);

    // Delete action
    let delete_action = gio::SimpleAction::new("delete", None);
    let panel_id_del = panel_id.clone();
    let selected_panels_del = selected_panels.clone();
    let panel_states_del = panel_states.clone();
    let occupied_cells_del = occupied_cells.clone();
    let panels_del = panels.clone();
    let on_change_del = on_change.clone();
    let container_del = container.clone();

    delete_action.connect_activate(move |_, _| {
        use gtk4::AlertDialog;

        let selected = selected_panels_del.borrow();
        let panel_ids: Vec<String> = if selected.is_empty() || !selected.contains(&panel_id_del) {
            vec![panel_id_del.clone()]
        } else {
            selected.iter().cloned().collect()
        };
        let count = panel_ids.len();
        drop(selected);

        let dialog = AlertDialog::builder()
            .message(format!(
                "Delete {} Panel{}?",
                count,
                if count > 1 { "s" } else { "" }
            ))
            .detail(format!(
                "This will permanently delete the selected panel{}.",
                if count > 1 { "s" } else { "" }
            ))
            .modal(true)
            .buttons(vec!["Cancel", "Delete"])
            .default_button(0)
            .cancel_button(0)
            .build();

        let selected_panels_confirm = selected_panels_del.clone();
        let panel_states_confirm = panel_states_del.clone();
        let occupied_cells_confirm = occupied_cells_del.clone();
        let panels_confirm = panels_del.clone();
        let on_change_confirm = on_change_del.clone();
        let container_confirm = container_del.clone();

        if let Some(root) = container_del.root() {
            if let Some(window) = root.downcast_ref::<gtk4::Window>() {
                dialog.choose(
                    Some(window),
                    gtk4::gio::Cancellable::NONE,
                    move |response| {
                        if let Ok(1) = response {
                            delete_selected_panels(
                                &panel_ids,
                                &selected_panels_confirm,
                                &panel_states_confirm,
                                &occupied_cells_confirm,
                                &container_confirm,
                                &panels_confirm,
                                &on_change_confirm,
                            );
                        }
                    },
                );
            }
        }
    });
    action_group.add_action(&delete_action);

    // Copy Style action
    let copy_style_action = gio::SimpleAction::new("copy_style", None);
    let panel_copy_style = panel.clone();
    copy_style_action.connect_activate(move |_, _| {
        log::info!("Copying panel style");
        let panel_guard = panel_copy_style.blocking_read();
        use crate::ui::{PanelStyle, CLIPBOARD};

        let mut displayer_config = panel_guard.config.clone();
        filter_source_config_keys(&mut displayer_config);

        let style = PanelStyle {
            background: panel_guard.background.clone(),
            corner_radius: panel_guard.corner_radius,
            border: panel_guard.border.clone(),
            displayer_config,
        };

        if let Ok(mut clipboard) = CLIPBOARD.lock() {
            clipboard.copy_panel_style(style);
        }
    });
    action_group.add_action(&copy_style_action);

    // Paste Style action
    let paste_style_action = gio::SimpleAction::new("paste_style", None);
    let panel_paste_style = panel.clone();
    let panel_states_paste = panel_states.clone();
    let on_change_paste = on_change.clone();
    let drop_zone_paste = drop_zone_layer.clone();
    paste_style_action.connect_activate(move |_, _| {
        use crate::ui::CLIPBOARD;

        if let Ok(clipboard) = CLIPBOARD.lock() {
            if let Some(style) = clipboard.paste_panel_style() {
                let panel_id = {
                    let mut panel_guard = panel_paste_style.blocking_write();
                    panel_guard.background = style.background;
                    panel_guard.corner_radius = style.corner_radius;
                    panel_guard.border = style.border;

                    for (key, value) in style.displayer_config {
                        panel_guard.config.insert(key, value);
                    }

                    let config_clone = panel_guard.config.clone();
                    let _ = panel_guard.displayer.apply_config(&config_clone);
                    panel_guard.id.clone()
                };

                if let Some(state) = panel_states_paste.borrow().get(&panel_id) {
                    state.background_area.queue_draw();
                    state.widget.queue_draw();
                }

                if let Some(ref callback) = *on_change_paste.borrow() {
                    callback();
                }
                drop_zone_paste.queue_draw();
            }
        }
    });
    action_group.add_action(&paste_style_action);

    // Set as Default Style action
    let set_default_style_action = gio::SimpleAction::new("set_default_style", None);
    let panel_set_default = panel.clone();
    set_default_style_action.connect_activate(move |_, _| {
        use crate::config::DefaultsConfig;

        log::info!("Setting panel style as default");
        let panel_guard = panel_set_default.blocking_read();

        let displayer_id = panel_guard.displayer.id().to_string();

        // Use to_inner_value() to save just the config, not the enum wrapper
        let displayer_config = if let Some(typed_config) = panel_guard.displayer.get_typed_config()
        {
            typed_config.to_inner_value()
        } else {
            let mut config = panel_guard.config.clone();
            filter_source_config_keys(&mut config);
            serde_json::to_value(&config).ok()
        };

        drop(panel_guard);

        if let Some(config_value) = displayer_config {
            let mut defaults = DefaultsConfig::load();
            defaults.set_displayer_default(&displayer_id, config_value);
            if let Err(e) = defaults.save() {
                log::warn!("Failed to save default style: {}", e);
            }
        }
    });
    action_group.add_action(&set_default_style_action);

    // Save to File action
    let save_to_file_action = gio::SimpleAction::new("save_to_file", None);
    let panel_save_file = panel.clone();
    let container_for_save = container.clone();
    save_to_file_action.connect_activate(move |_, _| {
        let panel_data = {
            let panel_guard = panel_save_file.blocking_read();
            panel_guard.to_data()
        };

        if let Some(root) = container_for_save.root() {
            if let Some(window) = root.downcast_ref::<gtk4::Window>() {
                let window_clone = window.clone();
                let data = panel_data;

                gtk4::glib::MainContext::default().spawn_local(async move {
                    use gtk4::FileDialog;

                    let initial_dir =
                        directories::ProjectDirs::from("com", "github.hilgardt_collab", "rg-sens")
                            .map(|d| d.config_dir().to_path_buf())
                            .unwrap_or_else(|| std::path::PathBuf::from("/"));

                    let json_filter = gtk4::FileFilter::new();
                    json_filter.set_name(Some("JSON files"));
                    json_filter.add_pattern("*.json");

                    let filters = gio::ListStore::new::<gtk4::FileFilter>();
                    filters.append(&json_filter);

                    let suggested_name = format!("panel_{}.json", data.id.replace('-', "_"));

                    let file_dialog = FileDialog::builder()
                        .title("Save Panel to File")
                        .modal(true)
                        .initial_folder(&gio::File::for_path(&initial_dir))
                        .initial_name(&suggested_name)
                        .filters(&filters)
                        .default_filter(&json_filter)
                        .build();

                    if let Ok(file) = file_dialog.save_future(Some(&window_clone)).await {
                        if let Some(path) = file.path() {
                            if let Ok(json) = serde_json::to_string_pretty(&data) {
                                let _ = std::fs::write(&path, json);
                            }
                        }
                    }
                });
            }
        }
    });
    action_group.add_action(&save_to_file_action);

    widget.insert_action_group("panel", Some(&action_group));

    // Setup right-click context menu
    let gesture_secondary = GestureClick::new();
    gesture_secondary.set_button(3);

    let menu = create_panel_context_menu();
    let popover = PopoverMenu::from_model(Some(&menu));
    popover.set_parent(widget);
    popover.set_has_arrow(false);

    // Ensure popover is unparented when widget is destroyed
    let popover_weak = popover.downgrade();
    widget.connect_destroy(move |_| {
        if let Some(p) = popover_weak.upgrade() {
            p.unparent();
        }
    });

    gesture_secondary.connect_pressed(move |_gesture, _, x, y| {
        popover.set_pointing_to(Some(&gtk4::gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
        popover.popup();
    });
    widget.add_controller(gesture_secondary);

    // Setup drag gesture
    let drag_gesture = GestureDrag::new();
    drag_gesture.set_button(1);

    let initial_positions: Rc<RefCell<HashMap<String, (f64, f64)>>> =
        Rc::new(RefCell::new(HashMap::new()));
    let dragged_panel_id: Rc<RefCell<String>> = Rc::new(RefCell::new(String::new()));

    // drag_begin
    let initial_positions_begin = initial_positions.clone();
    let dragged_panel_id_begin = dragged_panel_id.clone();
    let selected_panels_begin = selected_panels.clone();
    let panel_states_begin = panel_states.clone();
    let is_dragging_begin = is_dragging.clone();
    let drop_zone_begin = drop_zone_layer.clone();
    let panel_id_begin = panel_id.clone();
    let frame_begin = frame.clone();

    drag_gesture.connect_drag_begin(move |gesture, _, _| {
        *is_dragging_begin.borrow_mut() = true;
        drop_zone_begin.queue_draw();
        *dragged_panel_id_begin.borrow_mut() = panel_id_begin.clone();

        let modifiers = gesture.current_event_state();
        let ctrl_pressed = modifiers.contains(gtk4::gdk::ModifierType::CONTROL_MASK);

        let mut selected = selected_panels_begin.borrow_mut();
        let mut states = panel_states_begin.borrow_mut();

        if !selected.contains(&panel_id_begin) {
            if ctrl_pressed && !selected.is_empty() {
                selected.insert(panel_id_begin.clone());
                if let Some(state) = states.get_mut(&panel_id_begin) {
                    state.selected = true;
                    state.frame.add_css_class("selected");
                }
            } else {
                for (id, state) in states.iter_mut() {
                    if selected.contains(id) {
                        state.selected = false;
                        state.frame.remove_css_class("selected");
                    }
                }
                selected.clear();
                selected.insert(panel_id_begin.clone());
                if let Some(state) = states.get_mut(&panel_id_begin) {
                    state.selected = true;
                    frame_begin.add_css_class("selected");
                }
            }
        }

        let mut positions = initial_positions_begin.borrow_mut();
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

    // drag_update
    let initial_positions_update = initial_positions.clone();
    let dragged_panel_id_update = dragged_panel_id.clone();
    let config_update = config.clone();
    let selected_panels_update = selected_panels.clone();
    let panel_states_update = panel_states.clone();
    let drag_preview_cells_update = drag_preview_cells.clone();
    let drop_zone_update = drop_zone_layer.clone();

    drag_gesture.connect_drag_update(move |_, offset_x, offset_y| {
        let cfg = config_update.borrow();
        let positions = initial_positions_update.borrow();
        let selected = selected_panels_update.borrow();
        let states = panel_states_update.borrow();
        let dragged_id = dragged_panel_id_update.borrow();

        let mut preview_rects = Vec::new();

        if let Some((dragged_orig_x, dragged_orig_y)) = positions.get(&*dragged_id) {
            let dragged_new_x = dragged_orig_x + offset_x;
            let dragged_new_y = dragged_orig_y + offset_y;

            let dragged_grid_x = ((dragged_new_x + cfg.cell_width as f64 / 2.0)
                / (cfg.cell_width + cfg.spacing) as f64)
                .floor()
                .max(0.0) as u32;
            let dragged_grid_y = ((dragged_new_y + cfg.cell_height as f64 / 2.0)
                / (cfg.cell_height + cfg.spacing) as f64)
                .floor()
                .max(0.0) as u32;

            if let Some(dragged_state) = states.get(&*dragged_id) {
                let dragged_geom = dragged_state.panel.blocking_read().geometry;
                let delta_grid_x = dragged_grid_x as i32 - dragged_geom.x as i32;
                let delta_grid_y = dragged_grid_y as i32 - dragged_geom.y as i32;

                for id in selected.iter() {
                    if let Some(state) = states.get(id) {
                        let geom = state.panel.blocking_read().geometry;
                        let new_grid_x = (geom.x as i32 + delta_grid_x).max(0) as u32;
                        let new_grid_y = (geom.y as i32 + delta_grid_y).max(0) as u32;
                        preview_rects.push((new_grid_x, new_grid_y, geom.width, geom.height));
                    }
                }
            }
        }

        let mut preview_cells = drag_preview_cells_update.borrow_mut();
        if *preview_cells != preview_rects {
            *preview_cells = preview_rects;
            drop(preview_cells);
            drop_zone_update.queue_draw();
        }
    });

    // drag_end
    let initial_positions_end = initial_positions.clone();
    let dragged_panel_id_end = dragged_panel_id.clone();
    let config_end = config.clone();
    let selected_panels_end = selected_panels.clone();
    let panel_states_end = panel_states.clone();
    let occupied_cells_end = occupied_cells.clone();
    let drag_preview_cells_end = drag_preview_cells.clone();
    let is_dragging_end = is_dragging.clone();
    let drop_zone_end = drop_zone_layer.clone();
    let on_change_end = on_change.clone();
    let container_end = container.clone();
    let panels_end = panels.clone();
    let app_config_end = app_config.clone();

    drag_gesture.connect_drag_end(move |gesture, offset_x, offset_y| {
        let cfg = config_end.borrow();
        let selected = selected_panels_end.borrow();
        let states = panel_states_end.borrow();
        let mut occupied = occupied_cells_end.borrow_mut();
        let positions = initial_positions_end.borrow();
        let dragged_id = dragged_panel_id_end.borrow();

        let modifiers = gesture.current_event_state();
        let is_copy_mode = modifiers.contains(gtk4::gdk::ModifierType::CONTROL_MASK);

        // Clear occupied cells only if moving (skip panels with ignore_collision)
        if !is_copy_mode {
            for id in selected.iter() {
                if let Some(state) = states.get(id) {
                    let panel_guard = state.panel.blocking_read();
                    if !panel_guard.ignore_collision {
                        let geom = panel_guard.geometry;
                        for dx in 0..geom.width {
                            for dy in 0..geom.height {
                                occupied.remove(&(geom.x + dx, geom.y + dy));
                            }
                        }
                    }
                }
            }
        }

        // Calculate new positions
        let mut new_positions: Vec<(String, u32, u32, f64, f64)> = Vec::new();
        let mut group_has_collision = false;

        if let Some((dragged_orig_x, dragged_orig_y)) = positions.get(&*dragged_id) {
            let dragged_new_x = dragged_orig_x + offset_x;
            let dragged_new_y = dragged_orig_y + offset_y;

            let dragged_grid_x = ((dragged_new_x + cfg.cell_width as f64 / 2.0)
                / (cfg.cell_width + cfg.spacing) as f64)
                .floor()
                .max(0.0) as u32;
            let dragged_grid_y = ((dragged_new_y + cfg.cell_height as f64 / 2.0)
                / (cfg.cell_height + cfg.spacing) as f64)
                .floor()
                .max(0.0) as u32;

            if let Some(dragged_state) = states.get(&*dragged_id) {
                let dragged_geom = dragged_state.panel.blocking_read().geometry;
                let delta_grid_x = dragged_grid_x as i32 - dragged_geom.x as i32;
                let delta_grid_y = dragged_grid_y as i32 - dragged_geom.y as i32;

                for id in selected.iter() {
                    if let Some(state) = states.get(id) {
                        let panel_guard = state.panel.blocking_read();
                        let geom = panel_guard.geometry;
                        let grid_x = (geom.x as i32 + delta_grid_x).max(0) as u32;
                        let grid_y = (geom.y as i32 + delta_grid_y).max(0) as u32;

                        let snapped_x = grid_x as f64 * (cfg.cell_width + cfg.spacing) as f64;
                        let snapped_y = grid_y as f64 * (cfg.cell_height + cfg.spacing) as f64;

                        // Check collision (skip for panels with ignore_collision)
                        if !panel_guard.ignore_collision {
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
                        }

                        new_positions.push((id.clone(), grid_x, grid_y, snapped_x, snapped_y));
                    }
                }
            }
        }

        // Apply changes
        if group_has_collision && !is_copy_mode {
            // Restore original positions (only for panels that participate in collision)
            for id in selected.iter() {
                if let Some(state) = states.get(id) {
                    let panel_guard = state.panel.blocking_read();
                    if !panel_guard.ignore_collision {
                        let geom = panel_guard.geometry;
                        for dx in 0..geom.width {
                            for dy in 0..geom.height {
                                occupied.insert((geom.x + dx, geom.y + dy));
                            }
                        }
                    }
                }
            }
        } else if !group_has_collision {
            drop(states);
            drop(selected);
            drop(occupied);
            drop(positions);
            drop(dragged_id);
            drop(cfg);

            if is_copy_mode {
                // Copy mode - create new panels
                use crate::core::Panel;

                for (old_id, grid_x, grid_y, _snapped_x, _snapped_y) in new_positions {
                    let panel_states_read = panel_states_end.borrow();
                    if let Some(state) = panel_states_read.get(&old_id) {
                        let original_panel = state.panel.clone();
                        drop(panel_states_read);

                        let (
                            source_meta,
                            displayer_id,
                            panel_config,
                            background,
                            corner_radius,
                            border,
                            geometry_size,
                            scale,
                            translate_x,
                            translate_y,
                            z_index,
                            ignore_collision,
                            panel_data,
                        ) = {
                            let panel_guard = original_panel.blocking_read();
                            (
                                panel_guard.source.metadata().clone(),
                                panel_guard.displayer.id().to_string(),
                                panel_guard.config.clone(),
                                panel_guard.background.clone(),
                                panel_guard.corner_radius,
                                panel_guard.border.clone(),
                                (panel_guard.geometry.width, panel_guard.geometry.height),
                                panel_guard.scale,
                                panel_guard.translate_x,
                                panel_guard.translate_y,
                                panel_guard.z_index,
                                panel_guard.ignore_collision,
                                panel_guard.data.clone(),
                            )
                        };

                        let new_id = format!("panel_{}", uuid::Uuid::new_v4());
                        use crate::core::PanelGeometry;
                        let registry = crate::core::global_registry();

                        if let Ok(new_source) = registry.create_source(&source_meta.id) {
                            if let Ok(new_displayer) = registry.create_displayer(&displayer_id) {
                                let geometry = PanelGeometry {
                                    x: grid_x,
                                    y: grid_y,
                                    width: geometry_size.0,
                                    height: geometry_size.1,
                                };

                                let mut new_panel =
                                    Panel::new(new_id.clone(), geometry, new_source, new_displayer);
                                new_panel.background = background;
                                new_panel.corner_radius = corner_radius;
                                new_panel.border = border;
                                new_panel.scale = scale;
                                new_panel.translate_x = translate_x;
                                new_panel.translate_y = translate_y;
                                new_panel.z_index = z_index;
                                new_panel.ignore_collision = ignore_collision;
                                new_panel.data = panel_data;

                                let new_panel = Arc::new(RwLock::new(new_panel));
                                {
                                    let mut panel_guard = new_panel.blocking_write();
                                    let _ = panel_guard.apply_config(panel_config);
                                }

                                panels_end.borrow_mut().push(new_panel.clone());

                                if let Some(update_manager) = crate::core::global_update_manager() {
                                    update_manager.queue_add_panel(new_panel.clone());
                                }

                                // Mark cells as occupied (only if panel participates in collision)
                                if !ignore_collision {
                                    let mut occupied_write = occupied_cells_end.borrow_mut();
                                    for dx in 0..geometry_size.0 {
                                        for dy in 0..geometry_size.1 {
                                            occupied_write.insert((grid_x + dx, grid_y + dy));
                                        }
                                    }
                                    drop(occupied_write);
                                }

                                // Create UI
                                let cfg = config_end.borrow();
                                let x = grid_x as i32 * (cfg.cell_width + cfg.spacing);
                                let y = grid_y as i32 * (cfg.cell_height + cfg.spacing);
                                let width = geometry_size.0 as i32 * cfg.cell_width
                                    + (geometry_size.0 as i32 - 1) * cfg.spacing;
                                let height = geometry_size.1 as i32 * cfg.cell_height
                                    + (geometry_size.1 as i32 - 1) * cfg.spacing;
                                drop(cfg);

                                let new_widget = {
                                    let panel_guard = new_panel.blocking_read();
                                    panel_guard.displayer.create_widget()
                                };
                                new_widget.set_size_request(width, height);

                                let new_background_area = DrawingArea::new();
                                new_background_area.set_size_request(width, height);

                                let panel_clone_bg = new_panel.clone();
                                let app_config_bg = app_config_end.clone();
                                let background_area_weak = new_background_area.downgrade();
                                new_background_area.set_draw_func(move |_, cr, w, h| {
                                    if let Ok(panel_guard) = panel_clone_bg.try_read() {
                                        let width = w as f64;
                                        let height = h as f64;
                                        let radius = panel_guard
                                            .corner_radius
                                            .min(width / 2.0)
                                            .min(height / 2.0);

                                        cr.new_path();
                                        if radius > 0.0 {
                                            cr.arc(
                                                radius,
                                                radius,
                                                radius,
                                                std::f64::consts::PI,
                                                3.0 * std::f64::consts::FRAC_PI_2,
                                            );
                                            cr.arc(
                                                width - radius,
                                                radius,
                                                radius,
                                                3.0 * std::f64::consts::FRAC_PI_2,
                                                0.0,
                                            );
                                            cr.arc(
                                                width - radius,
                                                height - radius,
                                                radius,
                                                0.0,
                                                std::f64::consts::FRAC_PI_2,
                                            );
                                            cr.arc(
                                                radius,
                                                height - radius,
                                                radius,
                                                std::f64::consts::FRAC_PI_2,
                                                std::f64::consts::PI,
                                            );
                                            cr.close_path();
                                        } else {
                                            cr.rectangle(0.0, 0.0, width, height);
                                        }

                                        cr.save().ok();
                                        cr.clip();
                                        let source_values = panel_guard.source.get_values();
                                        let theme = app_config_bg.borrow().global_theme.clone();
                                        let _ = crate::ui::render_background_with_source_and_theme(
                                            cr,
                                            &panel_guard.background,
                                            width,
                                            height,
                                            &source_values,
                                            Some(&theme),
                                        );
                                        cr.restore().ok();

                                        if panel_guard.border.enabled {
                                            if radius > 0.0 {
                                                cr.arc(
                                                    radius,
                                                    radius,
                                                    radius,
                                                    std::f64::consts::PI,
                                                    3.0 * std::f64::consts::FRAC_PI_2,
                                                );
                                                cr.arc(
                                                    width - radius,
                                                    radius,
                                                    radius,
                                                    3.0 * std::f64::consts::FRAC_PI_2,
                                                    0.0,
                                                );
                                                cr.arc(
                                                    width - radius,
                                                    height - radius,
                                                    radius,
                                                    0.0,
                                                    std::f64::consts::FRAC_PI_2,
                                                );
                                                cr.arc(
                                                    radius,
                                                    height - radius,
                                                    radius,
                                                    std::f64::consts::FRAC_PI_2,
                                                    std::f64::consts::PI,
                                                );
                                                cr.close_path();
                                            } else {
                                                cr.rectangle(0.0, 0.0, width, height);
                                            }
                                            panel_guard.border.color.apply_to_cairo(cr);
                                            cr.set_line_width(panel_guard.border.width);
                                            cr.stroke().ok();
                                        }
                                    } else if let Some(bg_area) = background_area_weak.upgrade() {
                                        gtk4::glib::idle_add_local_once(move || {
                                            bg_area.queue_draw();
                                        });
                                    }
                                });

                                let new_overlay = Overlay::new();
                                new_overlay.set_child(Some(&new_background_area));
                                new_widget.add_css_class("transparent-background");
                                new_overlay.add_overlay(&new_widget);

                                let new_frame = Frame::new(None);
                                new_frame.set_child(Some(&new_overlay));
                                new_frame.set_size_request(width, height);

                                panel_states_end.borrow_mut().insert(
                                    new_id.clone(),
                                    PanelState {
                                        widget: new_widget.clone(),
                                        frame: new_frame.clone(),
                                        panel: new_panel.clone(),
                                        selected: false,
                                        background_area: new_background_area.clone(),
                                    },
                                );

                                // Setup interaction recursively via idle
                                let widget_i = new_widget.clone();
                                let frame_i = new_frame.clone();
                                let panel_i = new_panel.clone();
                                let id_i = new_id.clone();
                                let states_i = panel_states_end.clone();
                                let selected_i = selected_panels_end.clone();
                                let occupied_i = occupied_cells_end.clone();
                                let config_i = config_end.clone();
                                let container_i = container_end.clone();
                                let on_change_i = on_change_end.clone();
                                let drop_zone_i = drop_zone_end.clone();
                                let panels_i = panels_end.clone();
                                let is_dragging_i = is_dragging_end.clone();
                                let drag_preview_i = drag_preview_cells_end.clone();
                                let app_config_i = app_config_end.clone();

                                gtk4::glib::idle_add_local_once(move || {
                                    setup_copied_panel_interaction(
                                        &widget_i,
                                        &frame_i,
                                        panel_i,
                                        id_i,
                                        states_i,
                                        selected_i,
                                        occupied_i,
                                        config_i,
                                        container_i,
                                        on_change_i,
                                        drop_zone_i,
                                        panels_i,
                                        is_dragging_i,
                                        drag_preview_i,
                                        app_config_i,
                                    );
                                });

                                container_end.put(&new_frame, x as f64, y as f64);

                                {
                                    let mut panel_guard = new_panel.blocking_write();
                                    let _ = panel_guard.update();
                                }
                            }
                        }
                    }
                }
            } else {
                // Move mode
                let states = panel_states_end.borrow();
                let mut occupied = occupied_cells_end.borrow_mut();

                for (id, grid_x, grid_y, snapped_x, snapped_y) in new_positions {
                    if let Some(state) = states.get(&id) {
                        if let Some(parent) = state.frame.parent() {
                            if let Ok(fixed) = parent.downcast::<Fixed>() {
                                fixed.move_(&state.frame, snapped_x, snapped_y);
                            }
                        }

                        {
                            let mut panel_guard = state.panel.blocking_write();
                            panel_guard.geometry.x = grid_x;
                            panel_guard.geometry.y = grid_y;
                        }

                        // Mark new cells as occupied (only if panel participates in collision)
                        let panel_guard = state.panel.blocking_read();
                        if !panel_guard.ignore_collision {
                            let geom = panel_guard.geometry;
                            for dx in 0..geom.width {
                                for dy in 0..geom.height {
                                    occupied.insert((grid_x + dx, grid_y + dy));
                                }
                            }
                        }
                    }
                }
            }

            if let Some(ref callback) = *on_change_end.borrow() {
                callback();
            }
        }

        *drag_preview_cells_end.borrow_mut() = Vec::new();
        *is_dragging_end.borrow_mut() = false;
        drop_zone_end.queue_draw();
    });

    frame.add_controller(drag_gesture);
}

/// Helper function to delete multiple selected panels
pub(crate) fn delete_selected_panels(
    panel_ids: &[String],
    selected_panels: &Rc<RefCell<HashSet<String>>>,
    panel_states: &Rc<RefCell<HashMap<String, PanelState>>>,
    occupied_cells: &Rc<RefCell<HashSet<(u32, u32)>>>,
    container: &Fixed,
    panels: &Rc<RefCell<Vec<Arc<RwLock<Panel>>>>>,
    on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
) {
    log::info!("Deleting {} panels", panel_ids.len());

    for panel_id in panel_ids {
        // Remove from update manager
        if let Some(update_manager) = crate::core::global_update_manager() {
            update_manager.queue_remove_panel(panel_id.clone());
        }

        // Remove from panel states and UI
        if let Some(state) = panel_states.borrow_mut().remove(panel_id) {
            // Clean up event controllers and action groups to prevent memory leaks
            cleanup_widget_controllers(&state.widget);
            cleanup_widget_controllers(&state.frame);
            cleanup_widget_controllers(&state.background_area);

            container.remove(&state.frame);

            // Clear occupied cells (only if panel participated in collision detection)
            if let Ok(panel_guard) = state.panel.try_read() {
                if !panel_guard.ignore_collision {
                    let geom = panel_guard.geometry;
                    let mut occupied = occupied_cells.borrow_mut();
                    for dx in 0..geom.width {
                        for dy in 0..geom.height {
                            occupied.remove(&(geom.x + dx, geom.y + dy));
                        }
                    }
                }
            }

            // Remove from panels list
            panels.borrow_mut().retain(|p| {
                let p_guard = p.blocking_read();
                p_guard.id != *panel_id
            });

            log::info!("Panel deleted: {}", panel_id);
        } else {
            log::warn!("Panel not found in states: {}", panel_id);
        }

        // Remove from selected set
        selected_panels.borrow_mut().remove(panel_id);
    }

    // Trigger on_change callback
    if let Some(ref callback) = *on_change.borrow() {
        callback();
    }
}

impl Default for GridLayout {
    fn default() -> Self {
        Self::new(
            GridConfig::default(),
            Rc::new(RefCell::new(crate::config::AppConfig::default())),
        )
    }
}
