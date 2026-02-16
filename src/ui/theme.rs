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
        info!(
            "Applied system color scheme: {}",
            if prefer_dark { "dark" } else { "light" }
        );
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

    if schema_source
        .lookup("org.gnome.desktop.interface", true)
        .is_some()
    {
        let settings = gtk4::gio::Settings::new("org.gnome.desktop.interface");
        settings.connect_changed(Some("color-scheme"), |settings, _key| {
            let color_scheme: String = settings.string("color-scheme").to_string();
            let prefer_dark = color_scheme == "prefer-dark";

            if let Some(gtk_settings) = gtk4::Settings::default() {
                gtk_settings.set_gtk_application_prefer_dark_theme(prefer_dark);
                info!(
                    "System color scheme changed: {}",
                    if prefer_dark { "dark" } else { "light" }
                );
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
        info!(
            "Detected color scheme from freedesktop portal: {}",
            if prefer_dark { "dark" } else { "light" }
        );
        return prefer_dark;
    }

    // Method 2: Check GNOME settings directly
    if let Some(prefer_dark) = check_gnome_color_scheme() {
        info!(
            "Detected color scheme from GNOME settings: {}",
            if prefer_dark { "dark" } else { "light" }
        );
        return prefer_dark;
    }

    // Method 3: Check GTK_THEME environment variable for dark variant
    if let Ok(theme) = std::env::var("GTK_THEME") {
        let is_dark = theme.to_lowercase().contains("dark");
        info!(
            "Detected color scheme from GTK_THEME env ({}): {}",
            theme,
            if is_dark { "dark" } else { "light" }
        );
        if is_dark {
            return true;
        }
    }

    // Method 4: Check the current GTK theme name from settings
    if let Some(settings) = gtk4::Settings::default() {
        if let Some(theme_name) = settings.gtk_theme_name() {
            let theme_str = theme_name.to_lowercase();
            if theme_str.contains("dark") {
                info!("Detected dark mode from GTK theme name: {}", theme_name);
                return true;
            }
            // If theme name explicitly says light or doesn't contain dark, assume light
            info!("Detected light mode from GTK theme name: {}", theme_name);
            return false;
        }
    }

    // Default to light mode if no preference detected
    info!("No color scheme preference detected, defaulting to light mode");
    false
}

/// Check the freedesktop portal for color scheme preference
fn check_portal_color_scheme() -> Option<bool> {
    // Try to read from the portal settings
    // org.freedesktop.appearance color-scheme: 0=no-preference, 1=dark, 2=light
    let schema_source = gtk4::gio::SettingsSchemaSource::default()?;

    // Check if the schema exists before creating Settings
    if schema_source
        .lookup("org.freedesktop.appearance", true)
        .is_some()
    {
        let settings = gtk4::gio::Settings::new("org.freedesktop.appearance");
        let color_scheme: u32 = settings.uint("color-scheme");
        log::debug!(
            "freedesktop.appearance color-scheme value: {} (0=none, 1=dark, 2=light)",
            color_scheme
        );
        // 0 = no preference (don't use this method, try next)
        // 1 = prefer dark
        // 2 = prefer light
        if color_scheme == 0 {
            return None; // No preference set, try other methods
        }
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
    if schema_source
        .lookup("org.gnome.desktop.interface", true)
        .is_some()
    {
        let settings = gtk4::gio::Settings::new("org.gnome.desktop.interface");
        let color_scheme: String = settings.string("color-scheme").to_string();
        log::debug!("GNOME color-scheme value: '{}'", color_scheme);
        // "default" means no explicit preference, try other methods
        if color_scheme == "default" {
            return None;
        }
        return Some(color_scheme == "prefer-dark");
    }
    None
}

// ============================================================================
// Combo Panel Theme System
// ============================================================================
// Types are defined in rg-sens-types and re-exported here for backward compatibility.

// Re-export all theme types from rg-sens-types
pub use rg_sens_types::theme::{
    deserialize_color_or_source, deserialize_color_stop_or_source, deserialize_color_stops_vec,
    deserialize_font_or_source, ColorSource, ColorStopSource, ComboThemeConfig, FontOrString,
    FontSource, GradientSource, LinearGradientSourceConfig,
};
