use gtk4::gdk::Display;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, CssProvider};
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

    // Create grid configuration from loaded config
    let grid_config = UiGridConfig {
        rows: app_config.grid.rows,
        columns: app_config.grid.columns,
        cell_width: 300,  // Fixed for now, could be configurable
        cell_height: 200, // Fixed for now, could be configurable
        spacing: app_config.grid.spacing as i32,
    };

    // Create the main window with saved dimensions
    let window = gtk4::ApplicationWindow::builder()
        .application(app)
        .title("rg-Sens - System Monitor")
        .default_width(app_config.window.width)
        .default_height(app_config.window.height)
        .build();

    // Restore window position if saved
    if let (Some(x), Some(y)) = (app_config.window.x, app_config.window.y) {
        // Note: GTK4 doesn't directly support setting window position
        // This would need to be handled via window manager hints
        info!("Window position saved as ({}, {}), but GTK4 doesn't support direct positioning", x, y);
    }

    // Create grid layout
    let mut grid_layout = GridLayout::new(grid_config);

    // Create registry
    let registry = rg_sens::core::global_registry();

    // Create update manager
    let update_manager = Arc::new(UpdateManager::new());

    let mut panels = Vec::new();

    // Create panels from configuration
    if app_config.panels.is_empty() {
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
        info!("Loading {} panels from config", app_config.panels.len());

        for panel_config in &app_config.panels {
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

    // Set grid as window content
    window.set_child(Some(&grid_layout.widget()));

    // Track if configuration has changed (dirty flag)
    let config_dirty = Rc::new(RefCell::new(false));

    // Mark config as dirty when panels are moved
    let config_dirty_clone = config_dirty.clone();
    grid_layout.set_on_change(move || {
        *config_dirty_clone.borrow_mut() = true;
        info!("Configuration marked as modified");
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
    let grid_config_for_close = grid_config;

    window.connect_close_request(move |window| {
        let is_dirty = *config_dirty_clone4.borrow();

        if is_dirty {
            // Show save confirmation dialog
            show_save_dialog(window, &panels_clone, grid_config_for_close);
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

    window.present();
    info!("Window presented with {} panels", grid_layout.panels().len());
}

/// Show save confirmation dialog on close
fn show_save_dialog(window: &ApplicationWindow, panels: &[Arc<RwLock<Panel>>], grid_config: UiGridConfig) {
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

    dialog.choose(Some(window), gtk4::gio::Cancellable::NONE, move |response| {
        match response {
            Ok(2) => {
                // Save button (index 2)
                info!("User chose to save configuration");
                save_config(&window_clone, &panels_clone, grid_config);
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
fn save_config(window: &ApplicationWindow, panels: &[Arc<RwLock<Panel>>], grid_config: UiGridConfig) {
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

    // Create config
    let config = AppConfig {
        version: 1,
        window: WindowConfig {
            width,
            height,
            x: None, // GTK4 doesn't provide window position
            y: None,
        },
        grid: ConfigGridConfig {
            columns: grid_config.columns,
            rows: grid_config.rows,
            spacing: grid_config.spacing as u32,
        },
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
        ",
    );

    gtk4::style_context_add_provider_for_display(
        &Display::default().expect("Could not connect to a display."),
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}
