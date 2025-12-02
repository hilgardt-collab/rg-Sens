use gtk4::glib;
use gtk4::prelude::*;
use gtk4::Application;
use log::info;
use rg_sens::core::{Panel, PanelGeometry, UpdateManager};
use rg_sens::ui::{GridConfig, GridLayout};
use rg_sens::{displayers, sources};
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

    // Create grid configuration
    let grid_config = GridConfig {
        rows: 2,
        columns: 2,
        cell_width: 300,
        cell_height: 200,
        spacing: 8,
    };

    // Create the main window
    let window_width = grid_config.columns as i32 * (grid_config.cell_width + grid_config.spacing);
    let window_height = grid_config.rows as i32 * (grid_config.cell_height + grid_config.spacing);

    let window = gtk4::ApplicationWindow::builder()
        .application(app)
        .title("rg-Sens - System Monitor")
        .default_width(window_width)
        .default_height(window_height)
        .build();

    // Create grid layout
    let mut grid_layout = GridLayout::new(grid_config);

    // Create registry
    let registry = rg_sens::core::global_registry();

    // Create update manager
    let update_manager = Arc::new(UpdateManager::new());

    // Create multiple panels in different grid positions
    let panel_configs = vec![
        ("panel-1", 0, 0, 1, 1, "CPU Monitor 1"),
        ("panel-2", 1, 0, 1, 1, "CPU Monitor 2"),
        ("panel-3", 0, 1, 2, 1, "CPU Monitor 3 (wide)"),
    ];

    let mut panels = Vec::new();

    for (id, x, y, width, height, _name) in panel_configs {
        // Create source and displayer
        let source = registry
            .create_source("cpu")
            .expect("Failed to create CPU source");
        let displayer = registry
            .create_displayer("text")
            .expect("Failed to create text displayer");

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

        let panel = Arc::new(RwLock::new(panel));

        // Add panel to grid
        grid_layout.add_panel(panel.clone());

        panels.push(panel);
    }

    // Set grid as window content
    window.set_child(Some(&grid_layout.widget()));

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
