//! New Panel Dialog
//!
//! Provides functionality for creating new panels:
//! - Panel creation from configuration
//! - Panel creation from data
//! - New panel dialog UI

use gtk4::prelude::*;
use gtk4::ApplicationWindow;
use log::{info, warn};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

use serde_json;
use std::collections::HashMap;

use crate::config::AppConfig;
use crate::core::{Panel, PanelData, PanelGeometry};
use crate::ui::GridLayout;

/// Create a panel from configuration parameters
#[allow(clippy::too_many_arguments)]
pub fn create_panel_from_config(
    id: &str,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    source_id: &str,
    displayer_id: &str,
    background: crate::ui::background::BackgroundConfig,
    corner_radius: f64,
    border: crate::core::PanelBorderConfig,
    settings: HashMap<String, serde_json::Value>,
    registry: &crate::core::Registry,
) -> anyhow::Result<Arc<RwLock<Panel>>> {
    use crate::config::DefaultsConfig;
    use crate::core::{DisplayerConfig, PanelAppearance, PanelData, SourceConfig};

    // Load defaults to check for displayer-specific defaults
    let defaults = DefaultsConfig::load();

    // Try to use saved displayer default, fall back to built-in default
    let displayer_config = if let Some(saved_config) = defaults.get_displayer_default(displayer_id)
    {
        // Try to deserialize the saved config into the proper DisplayerConfig type
        DisplayerConfig::from_value_for_type(displayer_id, saved_config.clone())
            .unwrap_or_else(|| DisplayerConfig::default_for_type(displayer_id).unwrap_or_default())
    } else {
        DisplayerConfig::default_for_type(displayer_id).unwrap_or_default()
    };

    // Create PanelData with proper defaults for the source and displayer types
    let panel_data = PanelData {
        id: id.to_string(),
        geometry: PanelGeometry {
            x,
            y,
            width,
            height,
        },
        source_config: SourceConfig::default_for_type(source_id).unwrap_or_default(),
        displayer_config,
        appearance: PanelAppearance {
            background,
            corner_radius,
            border,
            scale: 1.0,
            translate_x: 0.0,
            translate_y: 0.0,
            z_index: 0,
            ignore_collision: false,
        },
    };

    // Create panel from PanelData (this properly initializes panel.data)
    let mut panel = Panel::from_data_with_registry(panel_data, registry)?;

    // Apply additional settings if provided (for backward compatibility)
    if !settings.is_empty() {
        panel.apply_config(settings)?;
    }

    Ok(Arc::new(RwLock::new(panel)))
}

/// Create a panel from PanelData (new unified format)
pub fn create_panel_from_data(
    data: PanelData,
    registry: &crate::core::Registry,
) -> anyhow::Result<Arc<RwLock<Panel>>> {
    // Use Panel::from_data_with_registry which handles everything
    let panel = Panel::from_data_with_registry(data, registry)?;
    Ok(Arc::new(RwLock::new(panel)))
}

/// Show dialog to create a new panel
pub fn show_new_panel_dialog(
    window: &ApplicationWindow,
    grid_layout: &Rc<RefCell<GridLayout>>,
    config_dirty: &Arc<AtomicBool>,
    app_config: &Rc<RefCell<AppConfig>>,
    mouse_coords: Option<(f64, f64)>,
) {
    use gtk4::{
        Adjustment, Box as GtkBox, Button, DropDown, Label, Orientation, SpinButton, StringList,
        Window,
    };

    let dialog = Window::builder()
        .title("New Panel")
        .transient_for(window)
        .modal(false)
        .default_width(400)
        .default_height(350)
        .resizable(false)
        .build();

    // Content area
    let vbox = GtkBox::new(Orientation::Vertical, 12);
    vbox.set_margin_start(12);
    vbox.set_margin_end(12);
    vbox.set_margin_top(12);
    vbox.set_margin_bottom(12);
    dialog.set_child(Some(&vbox));

    // Position section
    let pos_label = Label::new(Some("Position:"));
    pos_label.set_halign(gtk4::Align::Start);
    vbox.append(&pos_label);

    // Calculate grid coordinates from mouse position if provided
    let (default_x, default_y) = if let Some((mouse_x, mouse_y)) = mouse_coords {
        let cfg = app_config.borrow();
        let cell_width = cfg.grid.cell_width as f64;
        let cell_height = cfg.grid.cell_height as f64;
        let spacing = cfg.grid.spacing as f64;

        // Calculate grid cell coordinates
        let grid_x = (mouse_x / (cell_width + spacing)).floor().max(0.0);
        let grid_y = (mouse_y / (cell_height + spacing)).floor().max(0.0);

        (grid_x, grid_y)
    } else {
        (0.0, 0.0)
    };

    let pos_box = GtkBox::new(Orientation::Horizontal, 6);
    pos_box.append(&Label::new(Some("X:")));
    let x_adj = Adjustment::new(default_x, 0.0, 100.0, 1.0, 5.0, 0.0);
    let x_spin = SpinButton::new(Some(&x_adj), 1.0, 0);
    x_spin.set_hexpand(true);
    pos_box.append(&x_spin);

    pos_box.append(&Label::new(Some("Y:")));
    let y_adj = Adjustment::new(default_y, 0.0, 100.0, 1.0, 5.0, 0.0);
    let y_spin = SpinButton::new(Some(&y_adj), 1.0, 0);
    y_spin.set_hexpand(true);
    pos_box.append(&y_spin);
    vbox.append(&pos_box);

    // Size section - use defaults from DefaultsConfig
    let defaults = crate::config::DefaultsConfig::load();
    let default_width = defaults.general.default_panel_width as f64;
    let default_height = defaults.general.default_panel_height as f64;

    let size_label = Label::new(Some("Size:"));
    size_label.set_halign(gtk4::Align::Start);
    vbox.append(&size_label);

    let size_box = GtkBox::new(Orientation::Horizontal, 6);
    size_box.append(&Label::new(Some("Width:")));
    let width_adj = Adjustment::new(default_width, 1.0, 512.0, 1.0, 5.0, 0.0);
    let width_spin = SpinButton::new(Some(&width_adj), 1.0, 0);
    width_spin.set_hexpand(true);
    size_box.append(&width_spin);

    size_box.append(&Label::new(Some("Height:")));
    let height_adj = Adjustment::new(default_height, 1.0, 512.0, 1.0, 5.0, 0.0);
    let height_spin = SpinButton::new(Some(&height_adj), 1.0, 0);
    height_spin.set_hexpand(true);
    size_box.append(&height_spin);
    vbox.append(&size_box);

    // Data Source
    let source_label = Label::new(Some("Data Source:"));
    source_label.set_halign(gtk4::Align::Start);
    vbox.append(&source_label);

    let registry = crate::core::global_registry();
    let source_infos = registry.list_sources_with_info();
    let source_ids: Vec<String> = source_infos.iter().map(|s| s.id.clone()).collect();
    let source_display_names: Vec<String> = source_infos
        .iter()
        .map(|s| s.display_name.clone())
        .collect();
    let source_strings: Vec<&str> = source_display_names.iter().map(|s| s.as_str()).collect();
    let source_list = StringList::new(&source_strings);
    let source_combo = DropDown::new(Some(source_list), Option::<gtk4::Expression>::None);
    source_combo.set_selected(0);
    vbox.append(&source_combo);

    // Displayer - filtered based on selected source
    let displayer_label = Label::new(Some("Display Type:"));
    displayer_label.set_halign(gtk4::Align::Start);
    vbox.append(&displayer_label);

    // Get compatible displayers for the first source
    let first_source_id = source_ids.first().map(|s| s.as_str()).unwrap_or("cpu");
    let displayer_infos = registry.get_compatible_displayers(first_source_id);
    let displayer_ids: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(
        displayer_infos.iter().map(|d| d.id.clone()).collect(),
    ));
    let displayer_display_names: Vec<String> = displayer_infos
        .iter()
        .map(|d| d.display_name.clone())
        .collect();
    let displayer_strings: Vec<&str> = displayer_display_names.iter().map(|s| s.as_str()).collect();
    let displayer_list = StringList::new(&displayer_strings);
    let displayer_combo = DropDown::new(Some(displayer_list), Option::<gtk4::Expression>::None);
    displayer_combo.set_selected(0);
    vbox.append(&displayer_combo);

    // Update displayer dropdown when source changes
    let source_ids_for_change = source_ids.clone();
    let displayer_ids_for_change = displayer_ids.clone();
    let displayer_combo_clone = displayer_combo.clone();
    source_combo.connect_selected_notify(move |combo| {
        let selected_idx = combo.selected() as usize;
        if let Some(source_id) = source_ids_for_change.get(selected_idx) {
            let new_displayer_infos = registry.get_compatible_displayers(source_id);
            let new_displayer_ids: Vec<String> =
                new_displayer_infos.iter().map(|d| d.id.clone()).collect();
            let new_display_names: Vec<String> = new_displayer_infos
                .iter()
                .map(|d| d.display_name.clone())
                .collect();

            // Update stored displayer IDs
            *displayer_ids_for_change.borrow_mut() = new_displayer_ids;

            // Update dropdown model
            let display_strs: Vec<&str> = new_display_names.iter().map(|s| s.as_str()).collect();
            let new_list = StringList::new(&display_strs);
            displayer_combo_clone.set_model(Some(&new_list));
            displayer_combo_clone.set_selected(0);
        }
    });

    // Buttons
    let button_box = GtkBox::new(Orientation::Horizontal, 6);
    button_box.set_halign(gtk4::Align::End);
    button_box.set_margin_top(12);

    let cancel_button = Button::with_label("Cancel");
    let ok_button = Button::with_label("Create");
    ok_button.add_css_class("suggested-action");

    button_box.append(&cancel_button);
    button_box.append(&ok_button);
    vbox.append(&button_box);

    // Cancel handler
    let dialog_clone = dialog.clone();
    cancel_button.connect_clicked(move |_| {
        dialog_clone.destroy();
    });

    // OK handler
    let dialog_clone = dialog.clone();
    let grid_layout = grid_layout.clone();
    let config_dirty = config_dirty.clone();
    let displayer_ids_for_ok = displayer_ids.clone();
    ok_button.connect_clicked(move |_| {
        let x = x_spin.value() as u32;
        let y = y_spin.value() as u32;
        let width = width_spin.value() as u32;
        let height = height_spin.value() as u32;

        // Safely get selected indices with bounds checking
        let source_selected = source_combo.selected();
        let displayer_selected = displayer_combo.selected();

        // GTK returns GTK_INVALID_LIST_POSITION (u32::MAX) when nothing is selected
        if source_selected == gtk4::INVALID_LIST_POSITION {
            log::warn!("No source selected, cannot create panel");
            return;
        }
        let Some(source_id) = source_ids.get(source_selected as usize) else {
            log::warn!("Invalid source selection index: {}", source_selected);
            return;
        };

        let displayer_ids_borrowed = displayer_ids_for_ok.borrow();
        if displayer_selected == gtk4::INVALID_LIST_POSITION {
            log::warn!("No displayer selected, cannot create panel");
            return;
        }
        let Some(displayer_id) = displayer_ids_borrowed.get(displayer_selected as usize) else {
            log::warn!("Invalid displayer selection index: {}", displayer_selected);
            return;
        };

        // Generate unique ID
        let id = format!("panel_{}", uuid::Uuid::new_v4());

        info!(
            "Creating new panel: id={}, pos=({},{}), size={}x{}, source={}, displayer={}",
            id, x, y, width, height, source_id, displayer_id
        );

        // Load defaults for appearance
        let defaults = crate::config::DefaultsConfig::load();
        let background = defaults.general.default_background.clone();
        let corner_radius = defaults.general.default_corner_radius;
        let border = defaults.general.default_border.clone();
        let settings = HashMap::new();

        match create_panel_from_config(
            &id,
            x,
            y,
            width,
            height,
            source_id,
            displayer_id,
            background,
            corner_radius,
            border,
            settings,
            registry,
        ) {
            Ok(panel) => {
                // Add to grid (grid_layout maintains its own panels list)
                grid_layout.borrow_mut().add_panel(panel.clone());

                // Register with update manager so it gets periodic updates
                if let Some(update_manager) = crate::core::global_update_manager() {
                    update_manager.queue_add_panel(panel.clone());
                }

                // Mark config as dirty
                config_dirty.store(true, Ordering::Relaxed);

                info!("New panel created successfully");
                dialog_clone.destroy();
            }
            Err(e) => {
                warn!("Failed to create panel: {}", e);
                // Could show error dialog here
            }
        }
    });

    dialog.present();
}
