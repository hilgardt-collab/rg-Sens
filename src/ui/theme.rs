//! Theme and CSS utilities for the application
//!
//! Handles:
//! - Loading application CSS styles
//! - Detecting system dark/light mode preference
//! - Listening for system color scheme changes

use gtk4::gdk::Display;
use gtk4::prelude::SettingsExt;
use gtk4::CssProvider;
use log::info;
use std::cell::RefCell;

/// Load application CSS styles
pub fn load_css() {
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

/// Apply system color scheme preference (dark/light mode)
pub fn apply_system_color_scheme() {
    // Try to get the system color scheme from the freedesktop portal
    // color-scheme values: 0 = no preference, 1 = prefer dark, 2 = prefer light
    let prefer_dark = detect_system_dark_mode();

    if let Some(settings) = gtk4::Settings::default() {
        settings.set_gtk_application_prefer_dark_theme(prefer_dark);
        info!("Applied system color scheme: {}", if prefer_dark { "dark" } else { "light" });
    }

    // Listen for changes to the system color scheme
    setup_color_scheme_listener();
}

// Thread-local holder for the settings object to keep it alive for the listener
thread_local! {
    static COLOR_SCHEME_SETTINGS: RefCell<Option<gtk4::gio::Settings>> = const { RefCell::new(None) };
}

/// Set up a listener for system color scheme changes
pub fn setup_color_scheme_listener() {
    // Listen to GNOME settings changes if available
    let schema_source = match gtk4::gio::SettingsSchemaSource::default() {
        Some(source) => source,
        None => return,
    };

    if schema_source.lookup("org.gnome.desktop.interface", true).is_some() {
        let settings = gtk4::gio::Settings::new("org.gnome.desktop.interface");
        settings.connect_changed(Some("color-scheme"), |settings, _key| {
            let color_scheme: String = settings.string("color-scheme").to_string();
            let prefer_dark = color_scheme == "prefer-dark";

            if let Some(gtk_settings) = gtk4::Settings::default() {
                gtk_settings.set_gtk_application_prefer_dark_theme(prefer_dark);
                info!("System color scheme changed: {}", if prefer_dark { "dark" } else { "light" });
            }
        });
        // Store settings in thread-local to keep it alive for the lifetime of the app
        COLOR_SCHEME_SETTINGS.with(|cell| {
            *cell.borrow_mut() = Some(settings);
        });
    }
}

/// Detect if the system prefers dark mode
pub fn detect_system_dark_mode() -> bool {
    // Method 1: Check freedesktop portal settings via GSettings
    // This works on GNOME, KDE Plasma 5.24+, and other modern desktops
    if let Some(prefer_dark) = check_portal_color_scheme() {
        return prefer_dark;
    }

    // Method 2: Check GNOME settings directly
    if let Some(prefer_dark) = check_gnome_color_scheme() {
        return prefer_dark;
    }

    // Method 3: Check GTK_THEME environment variable for dark variant
    if let Ok(theme) = std::env::var("GTK_THEME") {
        if theme.to_lowercase().contains("dark") {
            return true;
        }
    }

    // Default to light mode if no preference detected
    false
}

/// Check the freedesktop portal for color scheme preference
fn check_portal_color_scheme() -> Option<bool> {
    // Try to read from the portal settings
    // org.freedesktop.appearance color-scheme: 0=no-preference, 1=dark, 2=light
    let schema_source = gtk4::gio::SettingsSchemaSource::default()?;

    // Check if the schema exists before creating Settings
    if schema_source.lookup("org.freedesktop.appearance", true).is_some() {
        let settings = gtk4::gio::Settings::new("org.freedesktop.appearance");
        let color_scheme: u32 = settings.uint("color-scheme");
        return Some(color_scheme == 1); // 1 = prefer dark
    }
    None
}

/// Check GNOME desktop settings for color scheme
fn check_gnome_color_scheme() -> Option<bool> {
    // Check org.gnome.desktop.interface color-scheme
    // Values: "default", "prefer-dark", "prefer-light"
    let schema_source = gtk4::gio::SettingsSchemaSource::default()?;

    // Check if the schema exists
    if schema_source.lookup("org.gnome.desktop.interface", true).is_some() {
        let settings = gtk4::gio::Settings::new("org.gnome.desktop.interface");
        let color_scheme: String = settings.string("color-scheme").to_string();
        return Some(color_scheme == "prefer-dark");
    }
    None
}
