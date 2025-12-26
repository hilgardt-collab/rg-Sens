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

// ============================================================================
// Combo Panel Theme System
// ============================================================================

use serde::{Deserialize, Serialize};
use super::background::{Color, ColorStop, LinearGradientConfig};

/// Reference to a theme color or custom color
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum ColorSource {
    /// Use a theme color (index 1-4)
    Theme { index: u8 },
    /// Use a custom color
    Custom { color: Color },
}

impl Default for ColorSource {
    fn default() -> Self {
        ColorSource::Theme { index: 1 }
    }
}

impl ColorSource {
    /// Create a theme color reference
    pub fn theme(index: u8) -> Self {
        ColorSource::Theme { index: index.clamp(1, 4) }
    }

    /// Create a custom color
    pub fn custom(color: Color) -> Self {
        ColorSource::Custom { color }
    }

    /// Resolve to actual color using theme
    pub fn resolve(&self, theme: &ComboThemeConfig) -> Color {
        match self {
            ColorSource::Theme { index } => theme.get_color(*index),
            ColorSource::Custom { color } => *color,
        }
    }

    /// Check if this is a theme reference
    pub fn is_theme(&self) -> bool {
        matches!(self, ColorSource::Theme { .. })
    }

    /// Get theme index if this is a theme reference
    pub fn theme_index(&self) -> Option<u8> {
        match self {
            ColorSource::Theme { index } => Some(*index),
            ColorSource::Custom { .. } => None,
        }
    }
}

/// Reference to a theme font or custom font
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum FontSource {
    /// Use a theme font (index 1-2)
    Theme { index: u8 },
    /// Use a custom font
    Custom { family: String, size: f64 },
}

impl Default for FontSource {
    fn default() -> Self {
        FontSource::Theme { index: 1 }
    }
}

impl FontSource {
    /// Create a theme font reference
    pub fn theme(index: u8) -> Self {
        FontSource::Theme { index: index.clamp(1, 2) }
    }

    /// Create a custom font
    pub fn custom(family: String, size: f64) -> Self {
        FontSource::Custom { family, size }
    }

    /// Resolve to actual font (family, size) using theme
    pub fn resolve(&self, theme: &ComboThemeConfig) -> (String, f64) {
        match self {
            FontSource::Theme { index } => theme.get_font(*index),
            FontSource::Custom { family, size } => (family.clone(), *size),
        }
    }

    /// Check if this is a theme reference
    pub fn is_theme(&self) -> bool {
        matches!(self, FontSource::Theme { .. })
    }

    /// Get theme index if this is a theme reference
    pub fn theme_index(&self) -> Option<u8> {
        match self {
            FontSource::Theme { index } => Some(*index),
            FontSource::Custom { .. } => None,
        }
    }
}

/// Reference to theme gradient or custom gradient
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum GradientSource {
    /// Use the theme gradient
    Theme,
    /// Use a custom gradient
    Custom { gradient: LinearGradientConfig },
}

impl Default for GradientSource {
    fn default() -> Self {
        GradientSource::Theme
    }
}

impl GradientSource {
    /// Create a custom gradient
    pub fn custom(gradient: LinearGradientConfig) -> Self {
        GradientSource::Custom { gradient }
    }

    /// Resolve to actual gradient using theme
    pub fn resolve(&self, theme: &ComboThemeConfig) -> LinearGradientConfig {
        match self {
            GradientSource::Theme => theme.gradient.clone(),
            GradientSource::Custom { gradient } => gradient.clone(),
        }
    }

    /// Check if this is a theme reference
    pub fn is_theme(&self) -> bool {
        matches!(self, GradientSource::Theme)
    }
}

/// Theme configuration for combo panels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComboThemeConfig {
    /// Theme color 1 (Primary)
    pub color1: Color,
    /// Theme color 2 (Secondary)
    pub color2: Color,
    /// Theme color 3 (Accent)
    pub color3: Color,
    /// Theme color 4 (Background/Highlight)
    pub color4: Color,
    /// Theme gradient
    pub gradient: LinearGradientConfig,
    /// Theme font 1 family
    pub font1_family: String,
    /// Theme font 1 size
    pub font1_size: f64,
    /// Theme font 2 family
    pub font2_family: String,
    /// Theme font 2 size
    pub font2_size: f64,
}

impl Default for ComboThemeConfig {
    fn default() -> Self {
        Self::default_for_synthwave()
    }
}

impl ComboThemeConfig {
    /// Get theme color by index (1-4)
    pub fn get_color(&self, index: u8) -> Color {
        match index {
            1 => self.color1,
            2 => self.color2,
            3 => self.color3,
            4 => self.color4,
            _ => self.color1, // Fallback to color1
        }
    }

    /// Get theme font by index (1-2) returns (family, size)
    pub fn get_font(&self, index: u8) -> (String, f64) {
        match index {
            1 => (self.font1_family.clone(), self.font1_size),
            2 => (self.font2_family.clone(), self.font2_size),
            _ => (self.font1_family.clone(), self.font1_size), // Fallback to font1
        }
    }

    /// Default theme for Synthwave panels
    pub fn default_for_synthwave() -> Self {
        Self {
            color1: Color::new(0.608, 0.349, 0.714, 1.0), // Purple #9b59b6
            color2: Color::new(0.914, 0.118, 0.549, 1.0), // Pink #e91e8c
            color3: Color::new(0.0, 1.0, 0.949, 1.0),     // Cyan #00fff2
            color4: Color::new(0.102, 0.039, 0.180, 1.0), // Dark purple #1a0a2e
            gradient: LinearGradientConfig {
                angle: 180.0,
                stops: vec![
                    ColorStop::new(0.0, Color::new(0.102, 0.039, 0.180, 1.0)),
                    ColorStop::new(1.0, Color::new(0.051, 0.020, 0.090, 1.0)),
                ],
            },
            font1_family: "sans-serif".to_string(),
            font1_size: 16.0,
            font2_family: "monospace".to_string(),
            font2_size: 12.0,
        }
    }

    /// Default theme for LCARS panels
    pub fn default_for_lcars() -> Self {
        Self {
            color1: Color::new(1.0, 0.6, 0.2, 1.0),       // Orange #ff9933
            color2: Color::new(0.8, 0.6, 1.0, 1.0),       // Purple #cc99ff
            color3: Color::new(0.6, 0.8, 0.8, 1.0),       // Teal #99cccc
            color4: Color::new(1.0, 0.8, 0.6, 1.0),       // Beige #ffcc99
            gradient: LinearGradientConfig {
                angle: 180.0,
                stops: vec![
                    ColorStop::new(0.0, Color::new(0.05, 0.05, 0.1, 1.0)),
                    ColorStop::new(1.0, Color::new(0.02, 0.02, 0.05, 1.0)),
                ],
            },
            font1_family: "Sans".to_string(),
            font1_size: 14.0,
            font2_family: "Sans Bold".to_string(),
            font2_size: 12.0,
        }
    }

    /// Default theme for Cyberpunk panels
    pub fn default_for_cyberpunk() -> Self {
        Self {
            color1: Color::new(0.0, 1.0, 1.0, 1.0),       // Cyan #00ffff
            color2: Color::new(0.0, 0.4, 1.0, 1.0),       // Blue #0066ff
            color3: Color::new(1.0, 1.0, 0.0, 1.0),       // Yellow #ffff00
            color4: Color::new(0.039, 0.039, 0.102, 1.0), // Dark blue #0a0a1a
            gradient: LinearGradientConfig {
                angle: 180.0,
                stops: vec![
                    ColorStop::new(0.0, Color::new(0.039, 0.039, 0.102, 1.0)),
                    ColorStop::new(1.0, Color::new(0.020, 0.020, 0.051, 1.0)),
                ],
            },
            font1_family: "Rajdhani".to_string(),
            font1_size: 18.0,
            font2_family: "monospace".to_string(),
            font2_size: 12.0,
        }
    }

    /// Default theme for Material Design panels
    pub fn default_for_material() -> Self {
        Self {
            color1: Color::new(0.129, 0.588, 0.953, 1.0), // Blue 500 #2196f3
            color2: Color::new(0.0, 0.588, 0.533, 1.0),   // Teal #009688
            color3: Color::new(1.0, 0.596, 0.0, 1.0),     // Orange #ff9800
            color4: Color::new(0.459, 0.459, 0.459, 1.0), // Gray #757575
            gradient: LinearGradientConfig {
                angle: 180.0,
                stops: vec![
                    ColorStop::new(0.0, Color::new(0.98, 0.98, 0.98, 1.0)),
                    ColorStop::new(1.0, Color::new(0.96, 0.96, 0.96, 1.0)),
                ],
            },
            font1_family: "Roboto".to_string(),
            font1_size: 14.0,
            font2_family: "Roboto".to_string(),
            font2_size: 12.0,
        }
    }

    /// Default theme for Industrial panels
    pub fn default_for_industrial() -> Self {
        Self {
            color1: Color::new(0.439, 0.502, 0.565, 1.0), // Steel #708090
            color2: Color::new(0.290, 0.333, 0.408, 1.0), // Dark steel #4a5568
            color3: Color::new(1.0, 0.757, 0.027, 1.0),   // Warning yellow #ffc107
            color4: Color::new(0.353, 0.353, 0.353, 1.0), // Rivet #5a5a5a
            gradient: LinearGradientConfig {
                angle: 180.0,
                stops: vec![
                    ColorStop::new(0.0, Color::new(0.35, 0.38, 0.42, 1.0)),
                    ColorStop::new(1.0, Color::new(0.25, 0.28, 0.32, 1.0)),
                ],
            },
            font1_family: "Sans Bold".to_string(),
            font1_size: 16.0,
            font2_family: "Sans".to_string(),
            font2_size: 12.0,
        }
    }

    /// Default theme for Retro Terminal panels
    pub fn default_for_retro_terminal() -> Self {
        Self {
            color1: Color::new(0.2, 1.0, 0.2, 1.0),       // Green #33ff33
            color2: Color::new(0.102, 0.502, 0.102, 1.0), // Dim green #1a801a
            color3: Color::new(0.898, 0.898, 0.863, 1.0), // White #e5e5dc
            color4: Color::new(0.020, 0.020, 0.020, 1.0), // Black #050505
            gradient: LinearGradientConfig {
                angle: 180.0,
                stops: vec![
                    ColorStop::new(0.0, Color::new(0.02, 0.02, 0.02, 1.0)),
                    ColorStop::new(1.0, Color::new(0.01, 0.01, 0.01, 1.0)),
                ],
            },
            font1_family: "monospace".to_string(),
            font1_size: 14.0,
            font2_family: "monospace".to_string(),
            font2_size: 12.0,
        }
    }

    /// Default theme for Fighter HUD panels
    pub fn default_for_fighter_hud() -> Self {
        Self {
            color1: Color::new(0.0, 0.902, 0.302, 1.0),   // Military green #00e64d
            color2: Color::new(0.0, 0.451, 0.149, 1.0),   // Dim green #007326
            color3: Color::new(0.0, 1.0, 0.333, 1.0),     // Bright green #00ff55
            color4: Color::new(0.039, 0.039, 0.039, 1.0), // Black #0a0a0a
            gradient: LinearGradientConfig {
                angle: 180.0,
                stops: vec![
                    ColorStop::new(0.0, Color::new(0.0, 0.0, 0.0, 0.8)),
                    ColorStop::new(1.0, Color::new(0.0, 0.0, 0.0, 0.9)),
                ],
            },
            font1_family: "monospace".to_string(),
            font1_size: 12.0,
            font2_family: "monospace".to_string(),
            font2_size: 10.0,
        }
    }
}
