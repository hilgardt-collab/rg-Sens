//! Configuration Helper Functions
//!
//! Provides utilities for saving and managing configuration:
//! - Save dialog for prompting users before closing
//! - Configuration saving to disk
//! - Monitor connector utilities

use gtk4::prelude::*;
use gtk4::ApplicationWindow;
use log::{info, warn};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::config::AppConfig;
use crate::core::{Panel, PanelData};
use crate::ui::GridLayout;

/// Show save dialog when closing with unsaved changes
pub fn show_save_dialog(
    window: &ApplicationWindow,
    grid_layout: &Rc<RefCell<GridLayout>>,
    app_config: &Rc<RefCell<AppConfig>>,
) {
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

    dialog.choose(
        Some(window),
        gtk4::gio::Cancellable::NONE,
        move |response| {
            match response {
                Ok(2) => {
                    // Save button (index 2)
                    info!("User chose to save configuration");
                    // Use with_panels to avoid cloning the Vec
                    grid_layout_clone.borrow().with_panels(|panels| {
                        save_config_with_app_config(
                            &mut app_config_clone.borrow_mut(),
                            &window_clone,
                            panels,
                        );
                    });
                    // Shutdown audio thread gracefully before exit
                    crate::core::shutdown_audio_thread();
                    window_clone.destroy(); // Use destroy to bypass close handler
                }
                Ok(0) => {
                    // Don't Save button (index 0)
                    info!("User chose not to save configuration");
                    // Shutdown audio thread gracefully before exit
                    crate::core::shutdown_audio_thread();
                    window_clone.destroy(); // Use destroy to bypass close handler
                }
                Ok(1) | Err(_) => {
                    // Cancel button (index 1) or dialog dismissed
                    info!("User cancelled close operation");
                }
                _ => {}
            }
        },
    );
}

/// Get the connector name of the monitor the window is currently on
pub fn get_window_monitor_connector(window: &ApplicationWindow) -> Option<String> {
    use gtk4::prelude::NativeExt;

    // Get the surface (realized window handle)
    let surface = window.surface()?;

    // Get the display
    let display = surface.display();

    // Use monitor_at_surface to get the monitor the window is on
    // This works on both X11 and Wayland
    if let Some(monitor) = display.monitor_at_surface(&surface) {
        if let Some(connector) = monitor.connector() {
            log::debug!("Window is on monitor: {}", connector);
            return Some(connector.to_string());
        }
    }

    // Fallback: return first monitor if monitor_at_surface fails
    let monitors = display.monitors();
    if monitors.n_items() > 0 {
        if let Some(mon) = monitors.item(0) {
            if let Ok(monitor) = mon.downcast::<gtk4::gdk::Monitor>() {
                return monitor.connector().map(|s| s.to_string());
            }
        }
    }

    None
}

/// Find monitor index by connector name
pub fn find_monitor_by_connector(connector: &str) -> Option<u32> {
    let display = gtk4::gdk::Display::default()?;
    let monitors = display.monitors();

    for i in 0..monitors.n_items() {
        if let Some(mon) = monitors.item(i) {
            if let Ok(monitor) = mon.downcast::<gtk4::gdk::Monitor>() {
                if let Some(mon_connector) = monitor.connector() {
                    if mon_connector == connector {
                        return Some(i);
                    }
                }
            }
        }
    }

    None
}

/// Save current configuration to disk
///
/// Updates the config in place and saves it, avoiding an extra clone.
pub fn save_config_with_app_config(
    app_config: &mut AppConfig,
    window: &ApplicationWindow,
    panels: &[Arc<RwLock<Panel>>],
) {
    // Get window dimensions
    let (width, height) = (window.default_width(), window.default_height());

    // Get window state
    let is_maximized = window.is_maximized();
    let is_fullscreen = window.is_fullscreen();

    // Get the monitor the window is on (by connector name)
    let monitor_connector = get_window_monitor_connector(window);

    // Convert panels to PanelData (new unified format)
    // Use try_read with retries to avoid blocking the GTK main thread indefinitely
    let panel_data_list: Vec<PanelData> = panels
        .iter()
        .filter_map(|panel| {
            // Try to acquire read lock with retries (avoid blocking GTK main thread)
            for attempt in 0..50 {
                // 50 attempts * 10ms = 500ms max wait per panel
                if let Ok(panel_guard) = panel.try_read() {
                    return Some(panel_guard.to_data());
                }
                // Brief sleep between retries - process GTK events to keep UI responsive
                if attempt < 49 {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
            }
            log::warn!(
                "Could not acquire read lock for panel during save - skipping (update in progress)"
            );
            None
        })
        .collect();

    // Update config in place instead of cloning
    app_config.window.width = width;
    app_config.window.height = height;
    app_config.window.x = None; // GTK4 doesn't provide window position reliably
    app_config.window.y = None;
    app_config.window.maximized = is_maximized;
    app_config.window.fullscreen_enabled = is_fullscreen;
    app_config.window.monitor_connector = monitor_connector.clone();
    // Also update fullscreen_monitor index for backward compatibility
    app_config.window.fullscreen_monitor = monitor_connector
        .as_ref()
        .and_then(|c| find_monitor_by_connector(c))
        .map(|i| i as i32);
    app_config.set_panels(panel_data_list);

    // Save global timers, alarms, and timer sound
    if let Ok(manager) = crate::core::global_timer_manager().read() {
        let (timers, alarms, global_sound) = manager.get_full_config();
        app_config.set_timers(timers);
        app_config.set_alarms(alarms);
        app_config.set_global_timer_sound(global_sound);
    }

    // Save to disk
    match app_config.save() {
        Ok(()) => {
            info!("Configuration saved successfully");
        }
        Err(e) => {
            warn!("Failed to save configuration: {}", e);
        }
    }
}
