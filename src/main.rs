use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow};
use log::info;
use rg_sens::ui::MainWindow;

const APP_ID: &str = "com.github.hilgardt_collab.rg_sens";

fn main() {
    // Initialize logger
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    info!("Starting rg-Sens v{}", env!("CARGO_PKG_VERSION"));

    // Create GTK application
    let app = Application::builder()
        .application_id(APP_ID)
        .build();

    app.connect_activate(build_ui);

    // Run the application
    let args: Vec<String> = std::env::args().collect();
    app.run_with_args(&args);
}

fn build_ui(app: &Application) {
    info!("Building UI");

    // Create main window (will be implemented in ui module)
    let window = ApplicationWindow::builder()
        .application(app)
        .title("rg-Sens - System Monitor")
        .default_width(800)
        .default_height(600)
        .build();

    // TODO: Initialize MainWindow with grid layout
    // let main_window = MainWindow::new();
    // window.set_child(Some(&main_window.widget()));

    window.present();
}
