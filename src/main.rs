use gtk4::glib;
use gtk4::prelude::*;
use gtk4::Application;
use log::info;
use rg_sens::core::{Panel, PanelGeometry, UpdateManager};
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

    // Create the main window
    let window = gtk4::ApplicationWindow::builder()
        .application(app)
        .title("rg-Sens - System Monitor")
        .default_width(400)
        .default_height(200)
        .build();

    // Create a CPU source and text displayer
    let registry = rg_sens::core::global_registry();
    let source = registry.create_source("cpu").expect("Failed to create CPU source");
    let displayer = registry.create_displayer("text").expect("Failed to create text displayer");

    // Get the widget from the displayer and add it to the window
    let widget = displayer.create_widget();
    window.set_child(Some(&widget));

    // Create a panel
    let panel = Panel::new(
        "panel-1".to_string(),
        PanelGeometry {
            x: 0,
            y: 0,
            width: 1,
            height: 1,
        },
        source,
        displayer,
    );

    let panel = Arc::new(RwLock::new(panel));

    // Create update manager and start the update loop
    let update_manager = Arc::new(UpdateManager::new());

    // Spawn tokio runtime in a separate thread
    let panel_clone = panel.clone();
    let update_manager_clone = update_manager.clone();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            // Add panel to update manager
            update_manager_clone.add_panel(panel_clone).await;

            // Run update loop
            info!("Starting update loop");
            update_manager_clone.run(Duration::from_millis(1000)).await;
        });
    });

    // Schedule periodic GTK redraws
    let panel_redraw = panel.clone();
    glib::timeout_add_local(Duration::from_millis(500), move || {
        // Queue redraw (the widget will automatically redraw)
        glib::ControlFlow::Continue
    });

    window.present();
    info!("Window presented");
}
