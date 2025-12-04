use gtk4::gdk::Display;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, CssProvider, EventControllerKey, Overlay};
use log::{info, warn};
use rg_sens::config::{AppConfig, GridConfig as ConfigGridConfig, PanelConfig, WindowConfig};
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

        for (id, x, y, width, height, source_id, displayer_id) in default_panels {
            match create_panel_from_config(id, x, y, width, height, source_id, displayer_id, &registry) {
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
    let panels_clone = panels.clone();
    let config_dirty_clone4 = config_dirty.clone();
    let app_config_for_close = app_config.clone();

    window.connect_close_request(move |window| {
        let is_dirty = *config_dirty_clone4.borrow();

        if is_dirty {
            // Show save confirmation dialog
            show_save_dialog(window, &panels_clone, &app_config_for_close);
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

            // Run update loop
            info!("Starting update loop");
            update_manager_clone.run(Duration::from_millis(1000)).await;
        });
    });

    // Add keyboard shortcut for settings (Ctrl+Comma)
    let key_controller = gtk4::EventControllerKey::new();
    let window_clone_for_settings = window.clone();
    let app_config_for_settings = app_config.clone();
    let window_bg_for_settings = window_background.clone();
    let grid_layout_for_settings = Rc::new(RefCell::new(grid_layout));
    let config_dirty_for_settings = config_dirty.clone();

    // Add right-click gesture for window settings
    let gesture_click = gtk4::GestureClick::new();
    gesture_click.set_button(gtk4::gdk::BUTTON_SECONDARY);

    let window_clone_for_menu = window_clone_for_settings.clone();
    let app_config_for_menu = app_config_for_settings.clone();
    let window_bg_for_menu = window_bg_for_settings.clone();
    let grid_layout_for_menu = grid_layout_for_settings.clone();
    let config_dirty_for_menu = config_dirty_for_settings.clone();

    gesture_click.connect_pressed(move |_, _, _, _| {
        show_window_settings_dialog(
            &window_clone_for_menu,
            &app_config_for_menu,
            &window_bg_for_menu,
            &grid_layout_for_menu,
            &config_dirty_for_menu,
        );
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
fn show_save_dialog(window: &ApplicationWindow, panels: &[Arc<RwLock<Panel>>], app_config: &Rc<RefCell<AppConfig>>) {
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
    let panels_clone = panels.to_vec();
    let app_config_clone = app_config.clone();

    dialog.choose(Some(window), gtk4::gio::Cancellable::NONE, move |response| {
        match response {
            Ok(2) => {
                // Save button (index 2)
                info!("User chose to save configuration");
                save_config_with_app_config(&app_config_clone.borrow(), &window_clone, &panels_clone);
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
                    settings: HashMap::new(), // TODO: Save displayer/source settings
                })
            } else {
                None
            }
        })
        .collect();

    // Create config with all settings
    let config = AppConfig {
        version: 1,
        window: WindowConfig {
            width,
            height,
            x: None, // GTK4 doesn't provide window position
            y: None,
            background: app_config.window.background.clone(),
        },
        grid: app_config.grid.clone(),
        panels: panel_configs,
    };

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
    use gtk4::{Box as GtkBox, Button, Dialog, Label, Orientation, SpinButton};
    use rg_sens::ui::BackgroundConfigWidget;

    let dialog = Dialog::builder()
        .title("Window Settings")
        .transient_for(parent_window)
        .modal(true)
        .default_width(500)
        .default_height(600)
        .build();

    let vbox = GtkBox::new(Orientation::Vertical, 12);
    vbox.set_margin_start(12);
    vbox.set_margin_end(12);
    vbox.set_margin_top(12);
    vbox.set_margin_bottom(12);

    // Window Background Section
    let bg_label = Label::new(Some("Window Background"));
    bg_label.add_css_class("heading");
    bg_label.set_margin_top(12);
    vbox.append(&bg_label);

    let background_widget = BackgroundConfigWidget::new();
    background_widget.set_config(app_config.borrow().window.background.clone());
    vbox.append(background_widget.widget());

    let background_widget = Rc::new(background_widget);

    // Grid Settings Section
    let grid_label = Label::new(Some("Grid Settings"));
    grid_label.add_css_class("heading");
    grid_label.set_margin_top(12);
    vbox.append(&grid_label);

    // Cell Width
    let cell_width_box = GtkBox::new(Orientation::Horizontal, 6);
    cell_width_box.append(&Label::new(Some("Cell Width:")));
    let cell_width_spin = SpinButton::with_range(50.0, 1000.0, 10.0);
    cell_width_spin.set_value(app_config.borrow().grid.cell_width as f64);
    cell_width_spin.set_hexpand(true);
    cell_width_box.append(&cell_width_spin);
    vbox.append(&cell_width_box);

    // Cell Height
    let cell_height_box = GtkBox::new(Orientation::Horizontal, 6);
    cell_height_box.append(&Label::new(Some("Cell Height:")));
    let cell_height_spin = SpinButton::with_range(50.0, 1000.0, 10.0);
    cell_height_spin.set_value(app_config.borrow().grid.cell_height as f64);
    cell_height_spin.set_hexpand(true);
    cell_height_box.append(&cell_height_spin);
    vbox.append(&cell_height_box);

    // Spacing
    let spacing_box = GtkBox::new(Orientation::Horizontal, 6);
    spacing_box.append(&Label::new(Some("Spacing:")));
    let spacing_spin = SpinButton::with_range(0.0, 50.0, 1.0);
    spacing_spin.set_value(app_config.borrow().grid.spacing as f64);
    spacing_spin.set_hexpand(true);
    spacing_box.append(&spacing_spin);
    vbox.append(&spacing_box);

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
    registry: &rg_sens::core::Registry,
) -> anyhow::Result<Arc<RwLock<Panel>>> {
    // Create source and displayer
    let source = registry.create_source(source_id)?;
    let displayer = registry.create_displayer(displayer_id)?;

    // Create panel
    let panel = Panel::new(
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

    Ok(Arc::new(RwLock::new(panel)))
}

/// Load CSS styling for the application
fn load_css() {
    let provider = CssProvider::new();
    provider.load_from_data(
        "
        .selected {
            border: 3px solid #00ff00;
            border-radius: 4px;
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
