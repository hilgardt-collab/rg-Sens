use gtk4::gdk::Display;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, CssProvider};
use log::{info, warn};
use rg_sens::config::{AppConfig, PanelConfig};
use rg_sens::core::{Panel, PanelGeometry, UpdateManager};
use rg_sens::ui::{GridConfig as UiGridConfig, GridLayout};
use rg_sens::{displayers, sources};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

const APP_ID: &str = "com.github.hilgardt_collab.rg_sens";

fn main() {
    // Initialize logger
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    info!("Starting rg-Sens v{}", env!("CARGO_PKG_VERSION"));

    // Register all built-in sources and displayers
    sources::register_all();
    displayers::register_all();

    // Create GTK application
    let app = Application::builder().application_id(APP_ID).build();

    app.connect_activate(build_ui);

    // Run the application
    let args: Vec<String> = std::env::args().collect();
    app.run_with_args(&args);
}

fn build_ui(app: &Application) {
    info!("Building UI");

    // Load CSS for selection styling
    load_css();

    // Load configuration from disk (or use defaults)
    let app_config = match AppConfig::load() {
        Ok(config) => {
            info!("Loaded configuration from disk");
            config
        }
        Err(e) => {
            warn!("Failed to load config, using defaults: {}", e);
            AppConfig::default()
        }
    };

    // Wrap app_config in Rc<RefCell<>> for shared mutable access
    let app_config = Rc::new(RefCell::new(app_config));

    // Create grid configuration from loaded config
    let grid_config = {
        let cfg = app_config.borrow();
        UiGridConfig {
            rows: cfg.grid.rows,
            columns: cfg.grid.columns,
            cell_width: cfg.grid.cell_width,
            cell_height: cfg.grid.cell_height,
            spacing: cfg.grid.spacing,
        }
    };

    // Create the main window with saved dimensions
    let window = {
        let cfg = app_config.borrow();
        gtk4::ApplicationWindow::builder()
            .application(app)
            .title("rg-Sens - System Monitor")
            .default_width(cfg.window.width)
            .default_height(cfg.window.height)
            .build()
    };

    // Restore window position if saved
    {
        let cfg = app_config.borrow();
        if let (Some(x), Some(y)) = (cfg.window.x, cfg.window.y) {
            // Note: GTK4 doesn't directly support setting window position
            // This would need to be handled via window manager hints
            info!("Window position saved as ({}, {}), but GTK4 doesn't support direct positioning", x, y);
        }
    }

    // Create grid layout
    let mut grid_layout = GridLayout::new(grid_config);

    // Create registry
    let registry = rg_sens::core::global_registry();

    // Create update manager
    let update_manager = Arc::new(UpdateManager::new());

    let mut panels = Vec::new();

    // Create panels from configuration
    let panel_configs = app_config.borrow().panels.clone();
    if panel_configs.is_empty() {
        info!("No panels in config, creating default panels");

        // Create default panels
        let default_panels = vec![
            ("panel-1", 0, 0, 1, 1, "cpu", "text"),
            ("panel-2", 1, 0, 1, 1, "cpu", "text"),
            ("panel-3", 0, 1, 2, 1, "cpu", "text"),
        ];

        // Get window defaults for panels
        let default_corner_radius = app_config.borrow().window.panel_corner_radius;
        let default_border = app_config.borrow().window.panel_border.clone();

        for (id, x, y, width, height, source_id, displayer_id) in default_panels {
            match create_panel_from_config(
                id,
                x,
                y,
                width,
                height,
                source_id,
                displayer_id,
                Default::default(),
                default_corner_radius,
                default_border.clone(),
                HashMap::new(),
                &registry,
            ) {
                Ok(panel) => {
                    grid_layout.add_panel(panel.clone());
                    panels.push(panel);
                }
                Err(e) => {
                    warn!("Failed to create default panel {}: {}", id, e);
                }
            }
        }
    } else {
        info!("Loading {} panels from config", panel_configs.len());

        for panel_config in &panel_configs {
            match create_panel_from_config(
                &panel_config.id,
                panel_config.x,
                panel_config.y,
                panel_config.width,
                panel_config.height,
                &panel_config.source,
                &panel_config.displayer,
                panel_config.background.clone(),
                panel_config.corner_radius,
                panel_config.border.clone(),
                panel_config.settings.clone(),
                &registry,
            ) {
                Ok(panel) => {
                    grid_layout.add_panel(panel.clone());
                    panels.push(panel);
                }
                Err(e) => {
                    warn!("Failed to create panel {}: {}", panel_config.id, e);
                }
            }
        }
    }

    // Create window background
    let window_background = gtk4::DrawingArea::new();
    let window_bg_config = app_config.borrow().window.background.clone();
    window_background.set_draw_func(move |_, cr, width, height| {
        use rg_sens::ui::background::render_background;
        let _ = render_background(cr, &window_bg_config, width as f64, height as f64);
    });

    // Create overlay to show background behind grid
    let window_overlay = gtk4::Overlay::new();
    window_overlay.set_child(Some(&window_background));
    window_overlay.add_overlay(&grid_layout.widget());

    // Set overlay as window content
    window.set_child(Some(&window_overlay));

    // Track if configuration has changed (dirty flag)
    let config_dirty = Rc::new(RefCell::new(false));

    // Mark config as dirty when panels are moved
    let config_dirty_clone = config_dirty.clone();
    grid_layout.set_on_change(move || {
        *config_dirty_clone.borrow_mut() = true;
    });

    // Wrap grid_layout in Rc<RefCell<>> for sharing across closures
    let grid_layout = Rc::new(RefCell::new(grid_layout));

    // Mark config as dirty when window is resized
    let config_dirty_clone2 = config_dirty.clone();
    window.connect_default_width_notify(move |_| {
        *config_dirty_clone2.borrow_mut() = true;
    });

    let config_dirty_clone3 = config_dirty.clone();
    window.connect_default_height_notify(move |_| {
        *config_dirty_clone3.borrow_mut() = true;
    });

    // Setup save-on-close confirmation
    let grid_layout_for_close = grid_layout.clone();
    let config_dirty_clone4 = config_dirty.clone();
    let app_config_for_close = app_config.clone();

    window.connect_close_request(move |window| {
        let is_dirty = *config_dirty_clone4.borrow();

        if is_dirty {
            // Show save confirmation dialog
            show_save_dialog(window, &grid_layout_for_close, &app_config_for_close);
            glib::Propagation::Stop // Prevent immediate close
        } else {
            glib::Propagation::Proceed // Close without saving
        }
    });

    // Spawn tokio runtime for update loop
    let update_manager_clone = update_manager.clone();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            // Add all panels to update manager
            for panel in panels {
                update_manager_clone.add_panel(panel).await;
            }

            // Run update loop (base interval for checking, each panel updates at its own configured interval)
            info!("Starting update loop");
            update_manager_clone.run(Duration::from_millis(100)).await;
        });
    });

    // Add keyboard shortcut for settings (Ctrl+Comma)
    let key_controller = gtk4::EventControllerKey::new();
    let window_clone_for_settings = window.clone();
    let app_config_for_settings = app_config.clone();
    let window_bg_for_settings = window_background.clone();
    let grid_layout_for_settings = grid_layout.clone();
    let config_dirty_for_settings = config_dirty.clone();

    // Add right-click gesture for context menu
    let gesture_click = gtk4::GestureClick::new();
    gesture_click.set_button(gtk4::gdk::BUTTON_SECONDARY);

    // Clone variables for context menu
    let window_for_menu = window.clone();
    let app_config_for_menu = app_config.clone();
    let window_bg_for_menu = window_background.clone();
    let grid_layout_for_menu = grid_layout_for_settings.clone();
    let config_dirty_for_menu = config_dirty.clone();

    gesture_click.connect_pressed(move |gesture, _, x, y| {
        use gtk4::gio;
        use gtk4::PopoverMenu;

        let menu = gio::Menu::new();

        // Section 1: New panel
        let section1 = gio::Menu::new();
        section1.append(Some("New Panel"), Some("window.new-panel"));
        menu.append_section(None, &section1);

        // Section 2: Save layout
        let section2 = gio::Menu::new();
        section2.append(Some("Save Layout"), Some("window.save-layout"));
        menu.append_section(None, &section2);

        // Section 3: Save/Load from file
        let section3 = gio::Menu::new();
        section3.append(Some("Save Layout to File..."), Some("window.save-to-file"));
        section3.append(Some("Load Layout from File..."), Some("window.load-from-file"));
        menu.append_section(None, &section3);

        // Section 4: Options and Quit
        let section4 = gio::Menu::new();
        section4.append(Some("Options"), Some("window.options"));
        section4.append(Some("Quit"), Some("window.quit"));
        menu.append_section(None, &section4);

        let popover = PopoverMenu::from_model(Some(&menu));
        popover.set_parent(&window_for_menu);
        popover.set_has_arrow(false);

        // Position the popover while being aware of window and screen bounds
        // Calculate preferred position
        let mut menu_x = x as i32;
        let mut menu_y = y as i32;

        // Get window and screen dimensions for boundary checking
        if let Some(surface) = window_for_menu.surface() {
            let display = surface.display();
            // Get monitor geometry
            if let Some(monitor) = display.monitor_at_surface(&surface) {
                let monitor_geom = monitor.geometry();
                let window_width = window_for_menu.default_width();
                let window_height = window_for_menu.default_height();

                // Estimate menu size (PopoverMenu doesn't expose size before showing)
                let estimated_menu_width = 250;
                let estimated_menu_height = 300;

                // Check if menu would go off the right edge of window
                if menu_x + estimated_menu_width > window_width {
                    menu_x = (window_width - estimated_menu_width).max(0);
                }

                // Check if menu would go off the bottom edge of window
                if menu_y + estimated_menu_height > window_height {
                    menu_y = (window_height - estimated_menu_height).max(0);
                }

                // Ensure menu stays within screen bounds too
                if menu_x + estimated_menu_width > monitor_geom.width() {
                    menu_x = (monitor_geom.width() - estimated_menu_width).max(0);
                }
                if menu_y + estimated_menu_height > monitor_geom.height() {
                    menu_y = (monitor_geom.height() - estimated_menu_height).max(0);
                }
            }
        }

        popover.set_pointing_to(Some(&gtk4::gdk::Rectangle::new(
            menu_x,
            menu_y,
            1,
            1,
        )));

        // Setup action group
        let action_group = gio::SimpleActionGroup::new();

        // New panel action (with mouse coordinates)
        let window_for_new = window_for_menu.clone();
        let grid_layout_for_new = grid_layout_for_menu.clone();
        let config_dirty_for_new = config_dirty_for_menu.clone();
        let app_config_for_new = app_config_for_menu.clone();
        let mouse_x = x;
        let mouse_y = y;
        let new_panel_action = gio::SimpleAction::new("new-panel", None);
        new_panel_action.connect_activate(move |_, _| {
            info!("New panel requested at ({}, {})", mouse_x, mouse_y);
            show_new_panel_dialog(
                &window_for_new,
                &grid_layout_for_new,
                &config_dirty_for_new,
                &app_config_for_new,
                Some((mouse_x, mouse_y)),
            );
        });
        action_group.add_action(&new_panel_action);

        // Save layout action
        let grid_layout_for_save = grid_layout_for_menu.clone();
        let app_config_for_save = app_config_for_menu.clone();
        let window_for_save = window_for_menu.clone();
        let config_dirty_for_save = config_dirty_for_menu.clone();
        let save_layout_action = gio::SimpleAction::new("save-layout", None);
        save_layout_action.connect_activate(move |_, _| {
            info!("Save layout requested");
            // Get current panels from GridLayout (not the stale clone)
            let current_panels = grid_layout_for_save.borrow().get_panels();
            save_config_with_app_config(&app_config_for_save.borrow(), &window_for_save, &current_panels);
            *config_dirty_for_save.borrow_mut() = false;
        });
        action_group.add_action(&save_layout_action);

        // Save to file action
        let window_for_save_file = window_for_menu.clone();
        let grid_layout_for_save_file = grid_layout_for_menu.clone();
        let app_config_for_save_file = app_config_for_menu.clone();
        let config_dirty_for_save_file = config_dirty_for_menu.clone();
        let save_to_file_action = gio::SimpleAction::new("save-to-file", None);
        save_to_file_action.connect_activate(move |_, _| {
            info!("Save to file requested");
            let window = window_for_save_file.clone();
            let grid_layout = grid_layout_for_save_file.clone();
            let app_config = app_config_for_save_file.clone();
            let config_dirty = config_dirty_for_save_file.clone();

            gtk4::glib::MainContext::default().spawn_local(async move {
                use gtk4::FileDialog;

                let file_dialog = FileDialog::builder()
                    .title("Save Layout to File")
                    .modal(true)
                    .build();

                if let Ok(file) = file_dialog.save_future(Some(&window)).await {
                    if let Some(path) = file.path() {
                        info!("Saving layout to {:?}", path);

                        // Get current panels
                        let current_panels = grid_layout.borrow().get_panels();

                        // Create config
                        let (width, height) = (window.default_width(), window.default_height());
                        let panel_configs: Vec<PanelConfig> = current_panels
                            .iter()
                            .filter_map(|panel| {
                                if let Ok(panel_guard) = panel.try_read() {
                                    Some(PanelConfig {
                                        id: panel_guard.id.clone(),
                                        x: panel_guard.geometry.x,
                                        y: panel_guard.geometry.y,
                                        width: panel_guard.geometry.width,
                                        height: panel_guard.geometry.height,
                                        source: panel_guard.source.metadata().id.clone(),
                                        displayer: panel_guard.displayer.id().to_string(),
                                        background: panel_guard.background.clone(),
                                        corner_radius: panel_guard.corner_radius,
                                        border: panel_guard.border.clone(),
                                        settings: panel_guard.config.clone(),
                                    })
                                } else {
                                    None
                                }
                            })
                            .collect();

                        let mut config = app_config.borrow().clone();
                        config.window.width = width;
                        config.window.height = height;
                        config.panels = panel_configs;

                        match config.save_to_path(&path) {
                            Ok(()) => {
                                info!("Layout saved successfully to {:?}", path);
                                *config_dirty.borrow_mut() = false;
                            }
                            Err(e) => {
                                warn!("Failed to save layout: {}", e);
                            }
                        }
                    }
                }
            });
        });
        action_group.add_action(&save_to_file_action);

        // Load from file action
        let window_for_load_file = window_for_menu.clone();
        let grid_layout_for_load_file = grid_layout_for_menu.clone();
        let app_config_for_load_file = app_config_for_menu.clone();
        let config_dirty_for_load_file = config_dirty_for_menu.clone();
        let load_from_file_action = gio::SimpleAction::new("load-from-file", None);
        load_from_file_action.connect_activate(move |_, _| {
            info!("Load from file requested");
            let window = window_for_load_file.clone();
            let grid_layout = grid_layout_for_load_file.clone();
            let app_config = app_config_for_load_file.clone();
            let config_dirty = config_dirty_for_load_file.clone();

            gtk4::glib::MainContext::default().spawn_local(async move {
                use gtk4::FileDialog;

                let file_dialog = FileDialog::builder()
                    .title("Load Layout from File")
                    .modal(true)
                    .build();

                if let Ok(file) = file_dialog.open_future(Some(&window)).await {
                    if let Some(path) = file.path() {
                        info!("Loading layout from {:?}", path);

                        match AppConfig::load_from_path(&path) {
                            Ok(loaded_config) => {
                                info!("Layout loaded successfully from {:?}", path);

                                // Update app config
                                *app_config.borrow_mut() = loaded_config.clone();

                                // Clear current panels
                                grid_layout.borrow_mut().clear_all_panels();

                                // Create panels from loaded config
                                let registry = rg_sens::core::global_registry();
                                for panel_config in &loaded_config.panels {
                                    match create_panel_from_config(
                                        &panel_config.id,
                                        panel_config.x,
                                        panel_config.y,
                                        panel_config.width,
                                        panel_config.height,
                                        &panel_config.source,
                                        &panel_config.displayer,
                                        panel_config.background.clone(),
                                        panel_config.corner_radius,
                                        panel_config.border.clone(),
                                        panel_config.settings.clone(),
                                        &registry,
                                    ) {
                                        Ok(panel) => {
                                            grid_layout.borrow_mut().add_panel(panel);
                                        }
                                        Err(e) => {
                                            warn!("Failed to create panel {}: {}", panel_config.id, e);
                                        }
                                    }
                                }

                                // Update grid configuration
                                grid_layout.borrow_mut().update_grid_size(
                                    loaded_config.grid.cell_width,
                                    loaded_config.grid.cell_height,
                                    loaded_config.grid.spacing,
                                );

                                // Mark config as clean since we just loaded
                                *config_dirty.borrow_mut() = false;
                            }
                            Err(e) => {
                                warn!("Failed to load layout: {}", e);
                            }
                        }
                    }
                }
            });
        });
        action_group.add_action(&load_from_file_action);

        // Options action
        let window_for_options = window_for_menu.clone();
        let app_config_for_options = app_config_for_menu.clone();
        let window_bg_for_options = window_bg_for_menu.clone();
        let grid_layout_for_options = grid_layout_for_menu.clone();
        let config_dirty_for_options = config_dirty_for_menu.clone();
        let options_action = gio::SimpleAction::new("options", None);
        options_action.connect_activate(move |_, _| {
            show_window_settings_dialog(
                &window_for_options,
                &app_config_for_options,
                &window_bg_for_options,
                &grid_layout_for_options,
                &config_dirty_for_options,
            );
        });
        action_group.add_action(&options_action);

        // Quit action
        let window_for_quit = window_for_menu.clone();
        let quit_action = gio::SimpleAction::new("quit", None);
        quit_action.connect_activate(move |_, _| {
            window_for_quit.close();
        });
        action_group.add_action(&quit_action);

        window_for_menu.insert_action_group("window", Some(&action_group));

        popover.popup();
        gesture.set_state(gtk4::EventSequenceState::Claimed);
    });

    window.add_controller(gesture_click);

    // Clone for closure
    let grid_layout_for_key = grid_layout_for_settings.clone();

    key_controller.connect_key_pressed(move |_, key, _code, modifiers| {
        if modifiers.contains(gtk4::gdk::ModifierType::CONTROL_MASK)
            && key == gtk4::gdk::Key::comma {
            show_window_settings_dialog(
                &window_clone_for_settings,
                &app_config_for_settings,
                &window_bg_for_settings,
                &grid_layout_for_key,
                &config_dirty_for_settings,
            );
            glib::Propagation::Stop
        } else {
            glib::Propagation::Proceed
        }
    });

    window.add_controller(key_controller);

    window.present();
}

/// Show save confirmation dialog on close
fn show_save_dialog(window: &ApplicationWindow, grid_layout: &Rc<RefCell<GridLayout>>, app_config: &Rc<RefCell<AppConfig>>) {
    use gtk4::AlertDialog;

    let dialog = AlertDialog::builder()
        .message("Save configuration before closing?")
        .detail("Your panel layout and window size have been modified.")
        .modal(true)
        .buttons(vec!["Don't Save", "Cancel", "Save"])
        .default_button(2) // "Save" button
        .cancel_button(1) // "Cancel" button
        .build();

    let window_clone = window.clone();
    let grid_layout_clone = grid_layout.clone();
    let app_config_clone = app_config.clone();

    dialog.choose(Some(window), gtk4::gio::Cancellable::NONE, move |response| {
        match response {
            Ok(2) => {
                // Save button (index 2)
                info!("User chose to save configuration");
                // Get current panels from GridLayout (not a stale clone)
                let current_panels = grid_layout_clone.borrow().get_panels();
                save_config_with_app_config(&app_config_clone.borrow(), &window_clone, &current_panels);
                window_clone.destroy(); // Use destroy to bypass close handler
            }
            Ok(0) => {
                // Don't Save button (index 0)
                info!("User chose not to save configuration");
                window_clone.destroy(); // Use destroy to bypass close handler
            }
            Ok(1) | Err(_) => {
                // Cancel button (index 1) or dialog dismissed
                info!("User cancelled close operation");
            }
            _ => {}
        }
    });
}

/// Save current configuration to disk
fn save_config_with_app_config(app_config: &AppConfig, window: &ApplicationWindow, panels: &[Arc<RwLock<Panel>>]) {
    // Get window dimensions
    let (width, height) = (window.default_width(), window.default_height());

    // Convert panels to PanelConfig
    let panel_configs: Vec<PanelConfig> = panels
        .iter()
        .filter_map(|panel| {
            if let Ok(panel_guard) = panel.try_read() {
                Some(PanelConfig {
                    id: panel_guard.id.clone(),
                    x: panel_guard.geometry.x,
                    y: panel_guard.geometry.y,
                    width: panel_guard.geometry.width,
                    height: panel_guard.geometry.height,
                    source: panel_guard.source.metadata().id.clone(),
                    displayer: panel_guard.displayer.id().to_string(),
                    background: panel_guard.background.clone(),
                    corner_radius: panel_guard.corner_radius,
                    border: panel_guard.border.clone(),
                    settings: panel_guard.config.clone(),
                })
            } else {
                None
            }
        })
        .collect();

    // Create config with all settings
    let mut config = app_config.clone();
    config.window.width = width;
    config.window.height = height;
    config.window.x = None; // GTK4 doesn't provide window position
    config.window.y = None;
    config.panels = panel_configs;

    // Save to disk
    match config.save() {
        Ok(()) => {
            info!("Configuration saved successfully");
        }
        Err(e) => {
            warn!("Failed to save configuration: {}", e);
        }
    }
}

/// Show window settings dialog
fn show_window_settings_dialog(
    parent_window: &ApplicationWindow,
    app_config: &Rc<RefCell<AppConfig>>,
    window_background: &gtk4::DrawingArea,
    grid_layout: &Rc<RefCell<GridLayout>>,
    config_dirty: &Rc<RefCell<bool>>,
) {
    use gtk4::{Box as GtkBox, Button, CheckButton, Label, Notebook, Orientation, SpinButton, Window};
    use rg_sens::ui::BackgroundConfigWidget;

    let dialog = Window::builder()
        .title("Window Settings")
        .transient_for(parent_window)
        .modal(false)
        .default_width(550)
        .default_height(650)
        .build();

    let vbox = GtkBox::new(Orientation::Vertical, 12);
    vbox.set_margin_start(12);
    vbox.set_margin_end(12);
    vbox.set_margin_top(12);
    vbox.set_margin_bottom(12);

    // Create notebook for tabs
    let notebook = Notebook::new();
    notebook.set_vexpand(true);

    // === Tab 1: Grid Settings ===
    let grid_tab_box = GtkBox::new(Orientation::Vertical, 12);
    grid_tab_box.set_margin_start(12);
    grid_tab_box.set_margin_end(12);
    grid_tab_box.set_margin_top(12);
    grid_tab_box.set_margin_bottom(12);

    // Cell Width
    let cell_width_box = GtkBox::new(Orientation::Horizontal, 6);
    cell_width_box.append(&Label::new(Some("Cell Width:")));
    let cell_width_spin = SpinButton::with_range(10.0, 1000.0, 10.0);
    cell_width_spin.set_value(app_config.borrow().grid.cell_width as f64);
    cell_width_spin.set_hexpand(true);
    cell_width_box.append(&cell_width_spin);
    grid_tab_box.append(&cell_width_box);

    // Cell Height
    let cell_height_box = GtkBox::new(Orientation::Horizontal, 6);
    cell_height_box.append(&Label::new(Some("Cell Height:")));
    let cell_height_spin = SpinButton::with_range(10.0, 1000.0, 10.0);
    cell_height_spin.set_value(app_config.borrow().grid.cell_height as f64);
    cell_height_spin.set_hexpand(true);
    cell_height_box.append(&cell_height_spin);
    grid_tab_box.append(&cell_height_box);

    // Spacing
    let spacing_box = GtkBox::new(Orientation::Horizontal, 6);
    spacing_box.append(&Label::new(Some("Spacing:")));
    let spacing_spin = SpinButton::with_range(0.0, 50.0, 1.0);
    spacing_spin.set_value(app_config.borrow().grid.spacing as f64);
    spacing_spin.set_hexpand(true);
    spacing_box.append(&spacing_spin);
    grid_tab_box.append(&spacing_box);

    notebook.append_page(&grid_tab_box, Some(&Label::new(Some("Grid"))));

    // === Tab 2: Background ===
    let bg_tab_box = GtkBox::new(Orientation::Vertical, 12);
    bg_tab_box.set_margin_start(12);
    bg_tab_box.set_margin_end(12);
    bg_tab_box.set_margin_top(12);
    bg_tab_box.set_margin_bottom(12);

    let background_widget = BackgroundConfigWidget::new();
    background_widget.set_config(app_config.borrow().window.background.clone());
    bg_tab_box.append(background_widget.widget());

    let background_widget = Rc::new(background_widget);

    notebook.append_page(&bg_tab_box, Some(&Label::new(Some("Background"))));

    // === Tab 3: Panel Defaults ===
    let panel_tab_box = GtkBox::new(Orientation::Vertical, 12);
    panel_tab_box.set_margin_start(12);
    panel_tab_box.set_margin_end(12);
    panel_tab_box.set_margin_top(12);
    panel_tab_box.set_margin_bottom(12);

    // Corner radius
    let corner_radius_label = Label::new(Some("Panel Corner Radius"));
    corner_radius_label.add_css_class("heading");
    panel_tab_box.append(&corner_radius_label);

    let corner_radius_box = GtkBox::new(Orientation::Horizontal, 6);
    corner_radius_box.set_margin_start(12);
    corner_radius_box.append(&Label::new(Some("Radius:")));
    let corner_radius_spin = SpinButton::with_range(0.0, 50.0, 1.0);
    corner_radius_spin.set_value(app_config.borrow().window.panel_corner_radius);
    corner_radius_spin.set_hexpand(true);
    corner_radius_box.append(&corner_radius_spin);
    panel_tab_box.append(&corner_radius_box);

    // Border section
    let border_label = Label::new(Some("Panel Border"));
    border_label.add_css_class("heading");
    border_label.set_margin_top(12);
    panel_tab_box.append(&border_label);

    let border_enabled_check = CheckButton::with_label("Show Border on New Panels");
    border_enabled_check.set_active(app_config.borrow().window.panel_border.enabled);
    border_enabled_check.set_margin_start(12);
    panel_tab_box.append(&border_enabled_check);

    let border_width_box = GtkBox::new(Orientation::Horizontal, 6);
    border_width_box.set_margin_start(12);
    border_width_box.append(&Label::new(Some("Width:")));
    let border_width_spin = SpinButton::with_range(0.5, 10.0, 0.5);
    border_width_spin.set_value(app_config.borrow().window.panel_border.width);
    border_width_spin.set_hexpand(true);
    border_width_box.append(&border_width_spin);
    panel_tab_box.append(&border_width_box);

    let border_color_btn = Button::with_label("Border Color");
    border_color_btn.set_margin_start(12);
    panel_tab_box.append(&border_color_btn);

    // Store border color in a shared Rc<RefCell>
    let border_color = Rc::new(RefCell::new(app_config.borrow().window.panel_border.color));

    // Border color button handler
    {
        let border_color_clone = border_color.clone();
        let dialog_clone = dialog.clone();
        border_color_btn.connect_clicked(move |_| {
            let current_color = *border_color_clone.borrow();
            let window_opt = dialog_clone.clone().upcast::<Window>();
            let border_color_clone2 = border_color_clone.clone();

            gtk4::glib::MainContext::default().spawn_local(async move {
                if let Some(new_color) = rg_sens::ui::ColorPickerDialog::pick_color(Some(&window_opt), current_color).await {
                    *border_color_clone2.borrow_mut() = new_color;
                }
            });
        });
    }

    notebook.append_page(&panel_tab_box, Some(&Label::new(Some("Panel Defaults"))));

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

    // Apply logic
    let app_config_clone = app_config.clone();
    let background_widget_clone = background_widget.clone();
    let window_background_clone = window_background.clone();
    let grid_layout_clone = grid_layout.clone();
    let config_dirty_clone = config_dirty.clone();
    let corner_radius_spin_clone = corner_radius_spin.clone();
    let border_enabled_check_clone = border_enabled_check.clone();
    let border_width_spin_clone = border_width_spin.clone();
    let border_color_clone = border_color.clone();

    let apply_changes = Rc::new(move || {
        let new_background = background_widget_clone.get_config();
        let new_cell_width = cell_width_spin.value() as i32;
        let new_cell_height = cell_height_spin.value() as i32;
        let new_spacing = spacing_spin.value() as i32;

        // Update app config
        let mut cfg = app_config_clone.borrow_mut();
        cfg.window.background = new_background.clone();
        cfg.grid.cell_width = new_cell_width;
        cfg.grid.cell_height = new_cell_height;
        cfg.grid.spacing = new_spacing;
        cfg.window.panel_corner_radius = corner_radius_spin_clone.value();
        cfg.window.panel_border.enabled = border_enabled_check_clone.is_active();
        cfg.window.panel_border.width = border_width_spin_clone.value();
        cfg.window.panel_border.color = *border_color_clone.borrow();
        drop(cfg);

        // Update window background rendering
        let bg_config = new_background;
        window_background_clone.set_draw_func(move |_, cr, width, height| {
            use rg_sens::ui::background::render_background;
            let _ = render_background(cr, &bg_config, width as f64, height as f64);
        });
        window_background_clone.queue_draw();

        // Update grid layout
        grid_layout_clone.borrow_mut().update_grid_size(new_cell_width, new_cell_height, new_spacing);

        // Mark config as dirty
        *config_dirty_clone.borrow_mut() = true;

        info!("Window settings applied");
    });

    // Apply button
    let apply_changes_clone = apply_changes.clone();
    apply_button.connect_clicked(move |_| {
        apply_changes_clone();
    });

    // Accept button
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

/// Create a panel from configuration parameters
fn create_panel_from_config(
    id: &str,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    source_id: &str,
    displayer_id: &str,
    background: rg_sens::ui::background::BackgroundConfig,
    corner_radius: f64,
    border: rg_sens::core::PanelBorderConfig,
    settings: HashMap<String, serde_json::Value>,
    registry: &rg_sens::core::Registry,
) -> anyhow::Result<Arc<RwLock<Panel>>> {
    // Create source and displayer
    let source = registry.create_source(source_id)?;
    let displayer = registry.create_displayer(displayer_id)?;

    // Create panel
    let mut panel = Panel::new(
        id.to_string(),
        PanelGeometry {
            x,
            y,
            width,
            height,
        },
        source,
        displayer,
    );

    // Set background, corner radius, and border
    panel.background = background;
    panel.corner_radius = corner_radius;
    panel.border = border;

    // Apply settings if provided
    if !settings.is_empty() {
        panel.apply_config(settings)?;
    }

    Ok(Arc::new(RwLock::new(panel)))
}

/// Show dialog to create a new panel
fn show_new_panel_dialog(
    window: &ApplicationWindow,
    grid_layout: &Rc<RefCell<GridLayout>>,
    config_dirty: &Rc<RefCell<bool>>,
    app_config: &Rc<RefCell<AppConfig>>,
    mouse_coords: Option<(f64, f64)>,
) {
    use gtk4::{Adjustment, Box as GtkBox, Button, DropDown, Label, Orientation, SpinButton, StringList, Window};

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

    // Size section
    let size_label = Label::new(Some("Size:"));
    size_label.set_halign(gtk4::Align::Start);
    vbox.append(&size_label);

    let size_box = GtkBox::new(Orientation::Horizontal, 6);
    size_box.append(&Label::new(Some("Width:")));
    let width_adj = Adjustment::new(4.0, 1.0, 50.0, 1.0, 5.0, 0.0);
    let width_spin = SpinButton::new(Some(&width_adj), 1.0, 0);
    width_spin.set_hexpand(true);
    size_box.append(&width_spin);

    size_box.append(&Label::new(Some("Height:")));
    let height_adj = Adjustment::new(2.0, 1.0, 50.0, 1.0, 5.0, 0.0);
    let height_spin = SpinButton::new(Some(&height_adj), 1.0, 0);
    height_spin.set_hexpand(true);
    size_box.append(&height_spin);
    vbox.append(&size_box);

    // Data Source
    let source_label = Label::new(Some("Data Source:"));
    source_label.set_halign(gtk4::Align::Start);
    vbox.append(&source_label);

    let registry = rg_sens::core::global_registry();
    let source_ids = registry.list_sources();
    let source_strings: Vec<&str> = source_ids.iter().map(|s| s.as_str()).collect();
    let source_list = StringList::new(&source_strings);
    let source_combo = DropDown::new(Some(source_list), Option::<gtk4::Expression>::None);
    source_combo.set_selected(0);
    vbox.append(&source_combo);

    // Displayer
    let displayer_label = Label::new(Some("Displayer:"));
    displayer_label.set_halign(gtk4::Align::Start);
    vbox.append(&displayer_label);

    let displayer_ids = registry.list_displayers();
    let displayer_strings: Vec<&str> = displayer_ids.iter().map(|d| d.as_str()).collect();
    let displayer_list = StringList::new(&displayer_strings);
    let displayer_combo = DropDown::new(Some(displayer_list), Option::<gtk4::Expression>::None);
    displayer_combo.set_selected(0);
    vbox.append(&displayer_combo);

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
    let app_config = app_config.clone();
    ok_button.connect_clicked(move |_| {
        let x = x_spin.value() as u32;
        let y = y_spin.value() as u32;
        let width = width_spin.value() as u32;
        let height = height_spin.value() as u32;

        let source_id = &source_ids[source_combo.selected() as usize];
        let displayer_id = &displayer_ids[displayer_combo.selected() as usize];

        // Generate unique ID
        let id = format!("panel_{}", uuid::Uuid::new_v4());

        info!("Creating new panel: id={}, pos=({},{}), size={}x{}, source={}, displayer={}",
              id, x, y, width, height, source_id, displayer_id);

        // Create panel with default background and window defaults for corner_radius and border
        let background = rg_sens::ui::background::BackgroundConfig::default();
        let corner_radius = app_config.borrow().window.panel_corner_radius;
        let border = app_config.borrow().window.panel_border.clone();
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
            &registry,
        ) {
            Ok(panel) => {
                // Add to grid (grid_layout maintains its own panels list)
                grid_layout.borrow_mut().add_panel(panel.clone());

                // Mark config as dirty
                *config_dirty.borrow_mut() = true;

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

/// Load CSS styling for the application
fn load_css() {
    let provider = CssProvider::new();
    provider.load_from_data(
        "
        frame {
            border: none;
            padding: 0;
            margin: 0;
            border-radius: 0;
        }

        overlay {
            border-radius: 0;
        }

        drawingarea {
            border-radius: 0;
        }

        .selected {
            border: 3px solid #00ff00;
        }

        .transparent-background {
            background: transparent;
            background-color: transparent;
        }
        ",
    );

    gtk4::style_context_add_provider_for_display(
        &Display::default().expect("Could not connect to a display."),
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}
