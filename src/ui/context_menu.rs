//! Context Menu
//!
//! Provides the right-click context menu for the main window with options for:
//! - Creating new panels
//! - Loading/saving layouts
//! - Accessing options
//! - Quitting the application

use gtk4::prelude::*;
use gtk4::{ApplicationWindow, DrawingArea, Popover};
use log::{info, warn};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::config::AppConfig;
use crate::core::PanelData;
use crate::ui::{GridLayout, new_panel_dialog, window_settings_dialog, config_helpers};

/// Create and show the context menu popover at the given coordinates
#[allow(clippy::too_many_arguments)]
pub fn show_context_menu<F>(
    window: &ApplicationWindow,
    app_config: &Rc<RefCell<AppConfig>>,
    window_background: &DrawingArea,
    grid_layout: &Rc<RefCell<GridLayout>>,
    config_dirty: &Arc<AtomicBool>,
    start_auto_scroll: &Rc<F>,
    x: f64,
    y: f64,
    scroll_x: f64,
    scroll_y: f64,
) where
    F: Fn() + 'static,
{
    use gtk4::{Box as GtkBox, Button, Separator, Orientation};

    // Create a custom popover with buttons
    let popover = Popover::new();
    popover.set_parent(window);
    popover.set_has_arrow(false);
    popover.set_autohide(true);

    // Create menu content box
    let menu_box = GtkBox::new(Orientation::Vertical, 0);
    menu_box.set_margin_top(6);
    menu_box.set_margin_bottom(6);
    menu_box.set_margin_start(6);
    menu_box.set_margin_end(6);

    // Helper to create menu buttons with consistent styling
    let create_menu_button = |label: &str| -> Button {
        let btn = Button::with_label(label);
        btn.add_css_class("flat");
        btn.set_halign(gtk4::Align::Fill);
        btn
    };

    // New Panel button
    let new_panel_btn = create_menu_button("New Panel");
    menu_box.append(&new_panel_btn);

    // Load Panel from File button
    let load_panel_btn = create_menu_button("Load Panel from File...");
    menu_box.append(&load_panel_btn);

    menu_box.append(&Separator::new(Orientation::Horizontal));

    // Save Layout button
    let save_layout_btn = create_menu_button("Save Layout");
    menu_box.append(&save_layout_btn);

    menu_box.append(&Separator::new(Orientation::Horizontal));

    // Save to File button
    let save_file_btn = create_menu_button("Save Layout to File...");
    menu_box.append(&save_file_btn);

    // Load from File button
    let load_file_btn = create_menu_button("Load Layout from File...");
    menu_box.append(&load_file_btn);

    menu_box.append(&Separator::new(Orientation::Horizontal));

    // Test Source button
    let test_source_btn = create_menu_button("Test Source...");
    menu_box.append(&test_source_btn);

    // Options button
    let options_btn = create_menu_button("Options");
    menu_box.append(&options_btn);

    // Quit button
    let quit_btn = create_menu_button("Quit");
    menu_box.append(&quit_btn);

    popover.set_child(Some(&menu_box));

    // Position at click location
    popover.set_pointing_to(Some(&gtk4::gdk::Rectangle::new(
        x as i32,
        y as i32,
        1,
        1,
    )));

    // Store references for button handlers
    let popover_ref = popover.clone();

    // New Panel button handler
    let window_for_new = window.clone();
    let grid_layout_for_new = grid_layout.clone();
    let config_dirty_for_new = config_dirty.clone();
    let app_config_for_new = app_config.clone();
    // Add scroll offset to mouse coordinates to get grid-relative position
    let grid_x = x + scroll_x;
    let grid_y = y + scroll_y;
    let popover_for_new = popover_ref.clone();
    new_panel_btn.connect_clicked(move |_| {
        popover_for_new.popdown();
        info!("New panel requested at grid position ({}, {})", grid_x, grid_y);
        new_panel_dialog::show_new_panel_dialog(
            &window_for_new,
            &grid_layout_for_new,
            &config_dirty_for_new,
            &app_config_for_new,
            Some((grid_x, grid_y)),
        );
    });

    // Load Panel from File button handler
    let window_for_load_panel = window.clone();
    let grid_layout_for_load_panel = grid_layout.clone();
    let config_dirty_for_load_panel = config_dirty.clone();
    let app_config_for_load_panel = app_config.clone();
    let popover_for_load_panel = popover_ref.clone();
    // Add scroll offset to mouse coordinates to get grid-relative position
    let load_grid_x = x + scroll_x;
    let load_grid_y = y + scroll_y;
    load_panel_btn.connect_clicked(move |_| {
        popover_for_load_panel.popdown();
        info!("Load panel from file requested");
        let window = window_for_load_panel.clone();
        let grid_layout = grid_layout_for_load_panel.clone();
        let config_dirty = config_dirty_for_load_panel.clone();
        let app_config = app_config_for_load_panel.clone();
        let mouse_x = load_grid_x;
        let mouse_y = load_grid_y;

        gtk4::glib::MainContext::default().spawn_local(async move {
            use gtk4::FileDialog;
            use gtk4::gio;

            // Get initial directory (config dir)
            let initial_dir = directories::ProjectDirs::from("com", "github.hilgardt_collab", "rg-sens")
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

            let file_dialog = FileDialog::builder()
                .title("Load Panel from File")
                .modal(true)
                .initial_folder(&gio::File::for_path(&initial_dir))
                .filters(&filters)
                .default_filter(&json_filter)
                .build();

            match file_dialog.open_future(Some(&window)).await {
                Ok(file) => {
                    if let Some(path) = file.path() {
                        info!("Loading panel from {:?}", path);

                        // Read and parse the JSON file
                        match std::fs::read_to_string(&path) {
                            Ok(json) => {
                                match serde_json::from_str::<PanelData>(&json) {
                                    Ok(mut panel_data) => {
                                        // Generate a new unique ID for the loaded panel
                                        panel_data.id = uuid::Uuid::new_v4().to_string();

                                        // Calculate grid position from mouse coordinates
                                        let cfg = app_config.borrow();
                                        let cell_width = cfg.grid.cell_width as f64;
                                        let cell_height = cfg.grid.cell_height as f64;
                                        let spacing = cfg.grid.spacing as f64;
                                        drop(cfg);

                                        let grid_x = (mouse_x / (cell_width + spacing)).floor() as u32;
                                        let grid_y = (mouse_y / (cell_height + spacing)).floor() as u32;

                                        // Check for collision at this position
                                        let has_collision = grid_layout.borrow().check_collision(
                                            grid_x,
                                            grid_y,
                                            panel_data.geometry.width,
                                            panel_data.geometry.height,
                                        );

                                        if has_collision {
                                            // Show error dialog
                                            let dialog = gtk4::AlertDialog::builder()
                                                .message("Cannot Load Panel")
                                                .detail("The panel cannot be placed at this position because it would overlap with existing panels.")
                                                .modal(true)
                                                .build();
                                            dialog.show(Some(&window));
                                            return;
                                        }

                                        panel_data.geometry.x = grid_x;
                                        panel_data.geometry.y = grid_y;

                                        info!("Placing panel at grid position ({}, {})", grid_x, grid_y);

                                        // Create the panel from data
                                        let registry = crate::core::global_registry();
                                        match new_panel_dialog::create_panel_from_data(panel_data, registry) {
                                            Ok(panel) => {
                                                // Add to grid layout
                                                grid_layout.borrow_mut().add_panel(panel.clone());

                                                // Register with update manager
                                                if let Some(update_manager) = crate::core::global_update_manager() {
                                                    update_manager.queue_add_panel(panel);
                                                }

                                                // Mark config as dirty
                                                config_dirty.store(true, Ordering::Relaxed);
                                                info!("Panel loaded successfully from {:?}", path);
                                            }
                                            Err(e) => {
                                                warn!("Failed to create panel from data: {}", e);
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        warn!("Failed to parse panel JSON: {}", e);
                                    }
                                }
                            }
                            Err(e) => {
                                warn!("Failed to read panel file: {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    info!("Load panel dialog cancelled or failed: {}", e);
                }
            }
        });
    });

    // Save Layout button handler
    let grid_layout_for_save = grid_layout.clone();
    let app_config_for_save = app_config.clone();
    let window_for_save = window.clone();
    let config_dirty_for_save = config_dirty.clone();
    let popover_for_save = popover_ref.clone();
    save_layout_btn.connect_clicked(move |_| {
        popover_for_save.popdown();
        info!("Save layout requested");
        // Use with_panels to avoid cloning the Vec
        grid_layout_for_save.borrow().with_panels(|panels| {
            config_helpers::save_config_with_app_config(&mut app_config_for_save.borrow_mut(), &window_for_save, panels);
        });
        config_dirty_for_save.store(false, Ordering::Relaxed);
    });

    // Save to File button handler
    let window_for_save_file = window.clone();
    let grid_layout_for_save_file = grid_layout.clone();
    let app_config_for_save_file = app_config.clone();
    let config_dirty_for_save_file = config_dirty.clone();
    let popover_for_save_file = popover_ref.clone();
    save_file_btn.connect_clicked(move |_| {
        popover_for_save_file.popdown();
        info!("Save to file requested");
        let window = window_for_save_file.clone();
        let grid_layout = grid_layout_for_save_file.clone();
        let app_config = app_config_for_save_file.clone();
        let config_dirty = config_dirty_for_save_file.clone();

        gtk4::glib::MainContext::default().spawn_local(async move {
            use gtk4::FileDialog;
            use gtk4::gio;

            info!("Creating save file dialog");

            // Get initial directory (config dir)
            let initial_dir = directories::ProjectDirs::from("com", "github.hilgardt_collab", "rg-sens")
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

            let file_dialog = FileDialog::builder()
                .title("Save Layout to File")
                .modal(true)
                .initial_folder(&gio::File::for_path(&initial_dir))
                .initial_name("layout.json")
                .filters(&filters)
                .default_filter(&json_filter)
                .build();

            info!("Showing save file dialog");
            match file_dialog.save_future(Some(&window)).await {
                Ok(file) => {
                    if let Some(path) = file.path() {
                        info!("Saving layout to {:?}", path);

                        let (width, height) = (window.default_width(), window.default_height());
                        // Use with_panels to avoid cloning the Vec
                        let panel_data_list: Vec<PanelData> = grid_layout.borrow().with_panels(|panels| {
                            panels
                                .iter()
                                .map(|panel| {
                                    let panel_guard = panel.blocking_read();
                                    panel_guard.to_data()
                                })
                                .collect()
                        });

                        // Update config in place instead of cloning
                        {
                            let mut config = app_config.borrow_mut();
                            config.window.width = width;
                            config.window.height = height;
                            config.set_panels(panel_data_list);
                        }

                        match app_config.borrow().save_to_path(&path) {
                            Ok(()) => {
                                info!("Layout saved successfully to {:?}", path);
                                config_dirty.store(false, Ordering::Relaxed);
                            }
                            Err(e) => {
                                warn!("Failed to save layout: {}", e);
                            }
                        }
                    } else {
                        warn!("File dialog returned no path");
                    }
                }
                Err(e) => {
                    // User cancelled or error occurred
                    info!("Save file dialog cancelled or failed: {}", e);
                }
            }
        });
    });

    // Load from File button handler
    let window_for_load_file = window.clone();
    let grid_layout_for_load_file = grid_layout.clone();
    let app_config_for_load_file = app_config.clone();
    let config_dirty_for_load_file = config_dirty.clone();
    let popover_for_load_file = popover_ref.clone();
    load_file_btn.connect_clicked(move |_| {
        popover_for_load_file.popdown();
        info!("Load from file requested");
        let window = window_for_load_file.clone();
        let grid_layout = grid_layout_for_load_file.clone();
        let app_config = app_config_for_load_file.clone();
        let config_dirty = config_dirty_for_load_file.clone();

        gtk4::glib::MainContext::default().spawn_local(async move {
            use gtk4::FileDialog;
            use gtk4::gio;

            info!("Creating load file dialog");

            // Get initial directory (config dir)
            let initial_dir = directories::ProjectDirs::from("com", "github.hilgardt_collab", "rg-sens")
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

            let file_dialog = FileDialog::builder()
                .title("Load Layout from File")
                .modal(true)
                .initial_folder(&gio::File::for_path(&initial_dir))
                .filters(&filters)
                .default_filter(&json_filter)
                .build();

            info!("Showing load file dialog");
            match file_dialog.open_future(Some(&window)).await {
                Ok(file) => {
                    if let Some(path) = file.path() {
                        info!("Loading layout from {:?}", path);

                        match AppConfig::load_from_path(&path) {
                            Ok(loaded_config) => {
                                info!("Layout loaded successfully from {:?}", path);
                                *app_config.borrow_mut() = loaded_config.clone();
                                grid_layout.borrow_mut().clear_all_panels();

                                let registry = crate::core::global_registry();
                                for panel_data in loaded_config.get_panels() {
                                    let panel_id = panel_data.id.clone();
                                    match new_panel_dialog::create_panel_from_data(panel_data, registry) {
                                        Ok(panel) => {
                                            grid_layout.borrow_mut().add_panel(panel.clone());

                                            // Register with update manager so panels get periodic updates
                                            if let Some(update_manager) = crate::core::global_update_manager() {
                                                update_manager.queue_add_panel(panel.clone());
                                            }
                                        }
                                        Err(e) => {
                                            warn!("Failed to create panel {}: {}", panel_id, e);
                                        }
                                    }
                                }

                                grid_layout.borrow_mut().update_grid_size(
                                    loaded_config.grid.cell_width,
                                    loaded_config.grid.cell_height,
                                    loaded_config.grid.spacing,
                                );
                                config_dirty.store(false, Ordering::Relaxed);
                            }
                            Err(e) => {
                                warn!("Failed to load layout: {}", e);
                            }
                        }
                    } else {
                        warn!("File dialog returned no path");
                    }
                }
                Err(e) => {
                    // User cancelled or error occurred
                    info!("Load file dialog cancelled or failed: {}", e);
                }
            }
        });
    });

    // Test Source button handler
    let window_for_test = window.clone();
    let popover_for_test = popover_ref.clone();
    let grid_layout_for_test = grid_layout.clone();
    let config_dirty_for_test = config_dirty.clone();
    test_source_btn.connect_clicked(move |_| {
        popover_for_test.popdown();
        // Create callback to save test source config to all test panels
        let grid_layout = grid_layout_for_test.clone();
        let config_dirty = config_dirty_for_test.clone();
        let save_callback: crate::ui::TestSourceSaveCallback = Box::new(move |test_config| {
            // Save to all panels that use test source (use with_panels to avoid Vec clone)
            grid_layout.borrow().with_panels(|panels| {
                for panel in panels {
                    if let Ok(mut panel_guard) = panel.try_write() {
                        if panel_guard.source.metadata().id == "test" {
                            if let Ok(config_json) = serde_json::to_value(test_config) {
                                panel_guard.config.insert("test_config".to_string(), config_json);
                                log::debug!("Saved test source config to panel {}", panel_guard.id);
                            }
                        }
                    }
                }
            });
            // Mark config as dirty so it gets saved
            config_dirty.store(true, Ordering::Relaxed);
        });
        crate::ui::show_test_source_dialog_with_callback(&window_for_test, Some(save_callback));
    });

    // Options button handler
    let window_for_options = window.clone();
    let app_config_for_options = app_config.clone();
    let window_bg_for_options = window_background.clone();
    let grid_layout_for_options = grid_layout.clone();
    let config_dirty_for_options = config_dirty.clone();
    let start_auto_scroll_for_options = start_auto_scroll.clone();
    let popover_for_options = popover_ref.clone();
    options_btn.connect_clicked(move |_| {
        popover_for_options.popdown();
        window_settings_dialog::show_window_settings_dialog(
            &window_for_options,
            &app_config_for_options,
            &window_bg_for_options,
            &grid_layout_for_options,
            &config_dirty_for_options,
            &start_auto_scroll_for_options,
        );
    });

    // Quit button handler
    let window_for_quit = window.clone();
    let popover_for_quit = popover_ref.clone();
    quit_btn.connect_clicked(move |_| {
        popover_for_quit.popdown();
        window_for_quit.close();
    });

    popover.popup();
}
