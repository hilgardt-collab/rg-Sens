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

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use super::background::{Color, ColorStop, LinearGradientConfig};

// ============================================================================
// Serde Migration Helpers
// ============================================================================

/// Deserialize a ColorSource from either:
/// - New format: { "type": "Theme", "index": 1 } or { "type": "Custom", "color": {...} }
/// - Legacy format: { "r": 1.0, "g": 0.0, "b": 0.0, "a": 1.0 } (raw Color)
pub fn deserialize_color_or_source<'de, D>(deserializer: D) -> Result<ColorSource, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;

    // Check if it's the new ColorSource format (has "type" field)
    if value.get("type").is_some() {
        // New format - deserialize as ColorSource
        ColorSource::deserialize(value).map_err(serde::de::Error::custom)
    } else if value.get("r").is_some() {
        // Legacy format - raw Color, wrap in Custom
        let color: Color = Color::deserialize(value).map_err(serde::de::Error::custom)?;
        Ok(ColorSource::Custom { color })
    } else {
        Err(serde::de::Error::custom("Expected ColorSource or Color"))
    }
}

/// Deserialize a ColorStopSource from either:
/// - New format: { "position": 0.5, "color": { "type": "Theme", "index": 1 } }
/// - Legacy format: { "position": 0.5, "color": { "r": 1.0, ... } } (raw ColorStop)
pub fn deserialize_color_stop_or_source<'de, D>(deserializer: D) -> Result<ColorStopSource, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;

    let position = value.get("position")
        .and_then(|v| v.as_f64())
        .ok_or_else(|| serde::de::Error::custom("Missing position field"))?;

    let color_value = value.get("color")
        .ok_or_else(|| serde::de::Error::custom("Missing color field"))?;

    // Check if color is new ColorSource format or legacy Color format
    let color = if color_value.get("type").is_some() {
        // New format
        ColorSource::deserialize(color_value.clone()).map_err(serde::de::Error::custom)?
    } else if color_value.get("r").is_some() {
        // Legacy format - raw Color
        let raw_color: Color = Color::deserialize(color_value.clone()).map_err(serde::de::Error::custom)?;
        ColorSource::Custom { color: raw_color }
    } else {
        return Err(serde::de::Error::custom("Invalid color format in ColorStopSource"));
    };

    Ok(ColorStopSource { position, color })
}

/// Deserialize a Vec<ColorStopSource> from either format
pub fn deserialize_color_stops_vec<'de, D>(deserializer: D) -> Result<Vec<ColorStopSource>, D::Error>
where
    D: Deserializer<'de>,
{
    let values: Vec<Value> = Vec::deserialize(deserializer)?;

    values.into_iter()
        .map(|value| {
            let position = value.get("position")
                .and_then(|v| v.as_f64())
                .ok_or_else(|| serde::de::Error::custom("Missing position field"))?;

            let color_value = value.get("color")
                .ok_or_else(|| serde::de::Error::custom("Missing color field"))?;

            let color = if color_value.get("type").is_some() {
                ColorSource::deserialize(color_value.clone()).map_err(serde::de::Error::custom)?
            } else if color_value.get("r").is_some() {
                let raw_color: Color = Color::deserialize(color_value.clone()).map_err(serde::de::Error::custom)?;
                ColorSource::Custom { color: raw_color }
            } else {
                return Err(serde::de::Error::custom("Invalid color format"));
            };

            Ok(ColorStopSource { position, color })
        })
        .collect()
}

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

/// Deserialize a FontSource from either:
/// - New format: { "type": "Theme", "index": 1 } or { "type": "Custom", "family": "...", "size": ... }
/// - Legacy format: just a string like "Sans" (will use default size 12.0)
pub fn deserialize_font_or_source<'de, D>(deserializer: D) -> Result<FontSource, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;

    // Check if it's the new FontSource format (has "type" field)
    if value.get("type").is_some() {
        // New format - deserialize as FontSource
        FontSource::deserialize(value).map_err(serde::de::Error::custom)
    } else if let Some(family) = value.as_str() {
        // Legacy format - just a font family string, use default size
        Ok(FontSource::Custom { family: family.to_string(), size: 12.0 })
    } else {
        Err(serde::de::Error::custom("Expected FontSource or font family string"))
    }
}

/// Reference to a theme font or custom font
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum FontSource {
    /// Use a theme font (index 1-2) with independent size
    Theme {
        index: u8,
        #[serde(default = "default_font_size")]
        size: f64,
    },
    /// Use a custom font
    Custom { family: String, size: f64 },
}

fn default_font_size() -> f64 {
    14.0
}

impl Default for FontSource {
    fn default() -> Self {
        FontSource::Theme { index: 1, size: 14.0 }
    }
}

impl FontSource {
    /// Create a theme font reference with size
    pub fn theme(index: u8, size: f64) -> Self {
        FontSource::Theme { index: index.clamp(1, 2), size }
    }

    /// Create a custom font
    pub fn custom(family: String, size: f64) -> Self {
        FontSource::Custom { family, size }
    }

    /// Resolve to actual font (family, size) using theme
    /// For theme fonts, uses family from theme but size from self
    pub fn resolve(&self, theme: &ComboThemeConfig) -> (String, f64) {
        match self {
            FontSource::Theme { index, size } => {
                let (family, _) = theme.get_font(*index);
                (family, *size)
            }
            FontSource::Custom { family, size } => (family.clone(), *size),
        }
    }

    /// Get the font size (independent of theme or custom)
    pub fn size(&self) -> f64 {
        match self {
            FontSource::Theme { size, .. } => *size,
            FontSource::Custom { size, .. } => *size,
        }
    }

    /// Create a new FontSource with updated size
    pub fn with_size(&self, new_size: f64) -> Self {
        match self {
            FontSource::Theme { index, .. } => FontSource::Theme { index: *index, size: new_size },
            FontSource::Custom { family, .. } => FontSource::Custom { family: family.clone(), size: new_size },
        }
    }

    /// Check if this is a theme reference
    pub fn is_theme(&self) -> bool {
        matches!(self, FontSource::Theme { .. })
    }

    /// Get theme index if this is a theme reference
    pub fn theme_index(&self) -> Option<u8> {
        match self {
            FontSource::Theme { index, .. } => Some(*index),
            FontSource::Custom { .. } => None,
        }
    }
}

/// Helper enum for deserializing font that can be either:
/// - A string (legacy format: just the font family)
/// - A FontSource object (new format)
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum FontOrString {
    /// New format: FontSource object
    Source(FontSource),
    /// Legacy format: just the font family as a string
    LegacyFamily(String),
}

impl FontOrString {
    /// Convert to Option<FontSource>, using default size for legacy strings
    /// Returns None if the legacy string was empty
    pub fn into_font_source(self, default_size: f64) -> Option<FontSource> {
        match self {
            FontOrString::Source(source) => Some(source),
            FontOrString::LegacyFamily(family) if family.is_empty() => None,
            FontOrString::LegacyFamily(family) => Some(FontSource::Custom { family, size: default_size }),
        }
    }
}

/// Reference to theme gradient or custom gradient
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(tag = "type")]
pub enum GradientSource {
    /// Use the theme gradient
    #[default]
    Theme,
    /// Use a custom gradient
    Custom { gradient: LinearGradientConfig },
}

impl GradientSource {
    /// Create a custom gradient
    pub fn custom(gradient: LinearGradientConfig) -> Self {
        GradientSource::Custom { gradient }
    }

    /// Resolve to actual gradient using theme
    pub fn resolve(&self, theme: &ComboThemeConfig) -> LinearGradientConfig {
        match self {
            GradientSource::Theme => theme.gradient.resolve(theme),
            GradientSource::Custom { gradient } => gradient.clone(),
        }
    }

    /// Check if this is a theme reference
    pub fn is_theme(&self) -> bool {
        matches!(self, GradientSource::Theme)
    }
}

/// A color stop that can reference theme colors
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ColorStopSource {
    pub position: f64,
    pub color: ColorSource,
}

/// Linear gradient configuration with theme-aware color stops
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LinearGradientSourceConfig {
    pub angle: f64,
    pub stops: Vec<ColorStopSource>,
}

impl LinearGradientSourceConfig {
    /// Create a new gradient from angle and theme-aware stops
    pub fn new(angle: f64, stops: Vec<ColorStopSource>) -> Self {
        Self { angle, stops }
    }

    /// Create from a concrete LinearGradientConfig (converts stops to Custom)
    pub fn from_concrete(config: &LinearGradientConfig) -> Self {
        Self {
            angle: config.angle,
            stops: config.stops.iter()
                .map(|s| ColorStopSource::custom(s.position, s.color))
                .collect(),
        }
    }

    /// Resolve to concrete LinearGradientConfig using theme
    pub fn resolve(&self, theme: &ComboThemeConfig) -> LinearGradientConfig {
        LinearGradientConfig {
            angle: self.angle,
            stops: self.stops.iter()
                .map(|s| s.resolve(theme))
                .collect(),
        }
    }
}

impl Default for LinearGradientSourceConfig {
    fn default() -> Self {
        Self {
            angle: 180.0,
            stops: vec![
                ColorStopSource::custom(0.0, Color::new(0.2, 0.2, 0.2, 1.0)),
                ColorStopSource::custom(1.0, Color::new(0.1, 0.1, 0.1, 1.0)),
            ],
        }
    }
}

impl ColorStopSource {
    /// Create a new color stop with theme color reference
    pub fn theme(position: f64, index: u8) -> Self {
        Self {
            position,
            color: ColorSource::theme(index),
        }
    }

    /// Create a new color stop with custom color
    pub fn custom(position: f64, color: Color) -> Self {
        Self {
            position,
            color: ColorSource::custom(color),
        }
    }

    /// Resolve to actual ColorStop using theme
    pub fn resolve(&self, theme: &ComboThemeConfig) -> ColorStop {
        ColorStop {
            position: self.position,
            color: self.color.resolve(theme),
        }
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
    /// Theme gradient (with theme-aware color stops)
    pub gradient: LinearGradientSourceConfig,
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
            gradient: LinearGradientSourceConfig {
                angle: 180.0,
                stops: vec![
                    ColorStopSource::custom(0.0, Color::new(0.102, 0.039, 0.180, 1.0)),
                    ColorStopSource::custom(1.0, Color::new(0.051, 0.020, 0.090, 1.0)),
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
            gradient: LinearGradientSourceConfig {
                angle: 180.0,
                stops: vec![
                    ColorStopSource::custom(0.0, Color::new(0.05, 0.05, 0.1, 1.0)),
                    ColorStopSource::custom(1.0, Color::new(0.02, 0.02, 0.05, 1.0)),
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
            gradient: LinearGradientSourceConfig {
                angle: 180.0,
                stops: vec![
                    ColorStopSource::custom(0.0, Color::new(0.039, 0.039, 0.102, 1.0)),
                    ColorStopSource::custom(1.0, Color::new(0.020, 0.020, 0.051, 1.0)),
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
        Self::default_for_material_light()
    }

    /// Default theme for Material Design panels (Light variant)
    pub fn default_for_material_light() -> Self {
        Self {
            color1: Color::new(0.129, 0.588, 0.953, 1.0), // Blue 500 #2196f3 - Primary
            color2: Color::new(0.0, 0.588, 0.533, 1.0),   // Teal #009688 - Secondary
            color3: Color::new(1.0, 0.596, 0.0, 1.0),     // Orange #ff9800 - Accent
            color4: Color::new(0.459, 0.459, 0.459, 1.0), // Gray #757575 - Highlight
            gradient: LinearGradientSourceConfig {
                angle: 180.0,
                stops: vec![
                    ColorStopSource::custom(0.0, Color::new(0.98, 0.98, 0.98, 1.0)),
                    ColorStopSource::custom(1.0, Color::new(0.96, 0.96, 0.96, 1.0)),
                ],
            },
            font1_family: "Roboto".to_string(),
            font1_size: 14.0,
            font2_family: "Roboto".to_string(),
            font2_size: 12.0,
        }
    }

    /// Default theme for Material Design panels (Dark variant)
    pub fn default_for_material_dark() -> Self {
        Self {
            color1: Color::new(0.565, 0.792, 0.976, 1.0), // Blue 200 #90caf9 - Primary
            color2: Color::new(0.502, 0.796, 0.769, 1.0), // Teal 200 #80cbc4 - Secondary
            color3: Color::new(1.0, 0.702, 0.349, 1.0),   // Orange 300 #ffb74d - Accent
            color4: Color::new(0.741, 0.741, 0.741, 1.0), // Gray 400 #bdbdbd - Highlight
            gradient: LinearGradientSourceConfig {
                angle: 180.0,
                stops: vec![
                    ColorStopSource::custom(0.0, Color::new(0.188, 0.188, 0.188, 1.0)), // #303030
                    ColorStopSource::custom(1.0, Color::new(0.141, 0.141, 0.141, 1.0)), // #242424
                ],
            },
            font1_family: "Roboto".to_string(),
            font1_size: 14.0,
            font2_family: "Roboto".to_string(),
            font2_size: 12.0,
        }
    }

    /// Default theme for Material Design panels (Teal variant)
    pub fn default_for_material_teal() -> Self {
        Self {
            color1: Color::new(0.0, 0.588, 0.533, 1.0),   // Teal 500 #009688 - Primary
            color2: Color::new(0.0, 0.447, 0.420, 1.0),   // Teal 700 #00796b - Secondary
            color3: Color::new(1.0, 0.341, 0.133, 1.0),   // Deep Orange #ff5722 - Accent
            color4: Color::new(0.459, 0.459, 0.459, 1.0), // Gray #757575 - Highlight
            gradient: LinearGradientSourceConfig {
                angle: 180.0,
                stops: vec![
                    ColorStopSource::custom(0.0, Color::new(0.878, 0.949, 0.945, 1.0)), // Teal 50 #e0f2f1
                    ColorStopSource::custom(1.0, Color::new(0.757, 0.890, 0.882, 1.0)), // Teal 100 #b2dfdb
                ],
            },
            font1_family: "Roboto".to_string(),
            font1_size: 14.0,
            font2_family: "Roboto".to_string(),
            font2_size: 12.0,
        }
    }

    /// Default theme for Material Design panels (Purple variant)
    pub fn default_for_material_purple() -> Self {
        Self {
            color1: Color::new(0.404, 0.227, 0.718, 1.0), // Deep Purple 500 #673ab7 - Primary
            color2: Color::new(0.482, 0.416, 0.882, 1.0), // Deep Purple 300 #7b6ae1 - Secondary
            color3: Color::new(0.0, 0.898, 0.694, 1.0),   // Teal A400 #00e5b1 - Accent
            color4: Color::new(0.620, 0.620, 0.620, 1.0), // Gray 500 #9e9e9e - Highlight
            gradient: LinearGradientSourceConfig {
                angle: 180.0,
                stops: vec![
                    ColorStopSource::custom(0.0, Color::new(0.188, 0.141, 0.251, 1.0)), // Deep Purple 900 #311b40
                    ColorStopSource::custom(1.0, Color::new(0.122, 0.098, 0.176, 1.0)), // Darker purple #1f192d
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
            gradient: LinearGradientSourceConfig {
                angle: 180.0,
                stops: vec![
                    ColorStopSource::custom(0.0, Color::new(0.35, 0.38, 0.42, 1.0)),
                    ColorStopSource::custom(1.0, Color::new(0.25, 0.28, 0.32, 1.0)),
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
            gradient: LinearGradientSourceConfig {
                angle: 180.0,
                stops: vec![
                    ColorStopSource::custom(0.0, Color::new(0.02, 0.02, 0.02, 1.0)),
                    ColorStopSource::custom(1.0, Color::new(0.01, 0.01, 0.01, 1.0)),
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
            gradient: LinearGradientSourceConfig {
                angle: 180.0,
                stops: vec![
                    ColorStopSource::custom(0.0, Color::new(0.0, 0.0, 0.0, 0.8)),
                    ColorStopSource::custom(1.0, Color::new(0.0, 0.0, 0.0, 0.9)),
                ],
            },
            font1_family: "monospace".to_string(),
            font1_size: 12.0,
            font2_family: "monospace".to_string(),
            font2_size: 10.0,
        }
    }

    /// Default theme for Art Deco panels
    pub fn default_for_art_deco() -> Self {
        Self {
            color1: Color::new(0.831, 0.686, 0.216, 1.0), // Gold #D4AF37
            color2: Color::new(0.722, 0.451, 0.200, 1.0), // Copper #B87333
            color3: Color::new(0.804, 0.608, 0.114, 1.0), // Brass #CD9B1D
            color4: Color::new(0.102, 0.102, 0.102, 1.0), // Dark charcoal #1A1A1A
            gradient: LinearGradientSourceConfig {
                angle: 180.0,
                stops: vec![
                    ColorStopSource::theme(0.0, 1), // Gold at top
                    ColorStopSource::theme(1.0, 2), // Copper at bottom
                ],
            },
            font1_family: "Sans Bold".to_string(),
            font1_size: 14.0,
            font2_family: "Sans".to_string(),
            font2_size: 11.0,
        }
    }

    /// Default theme for Art Nouveau style - organic, flowing, nature-inspired
    pub fn default_for_art_nouveau() -> Self {
        Self {
            color1: Color::new(0.420, 0.557, 0.137, 1.0), // Olive green #6B8E23
            color2: Color::new(0.855, 0.647, 0.125, 1.0), // Goldenrod #DAA520
            color3: Color::new(0.961, 0.961, 0.863, 1.0), // Beige/Cream #F5F5DC
            color4: Color::new(0.180, 0.310, 0.180, 1.0), // Dark olive #2E4F2E
            gradient: LinearGradientSourceConfig {
                angle: 180.0,
                stops: vec![
                    ColorStopSource::theme(0.0, 1), // Olive at top
                    ColorStopSource::theme(1.0, 4), // Dark olive at bottom
                ],
            },
            font1_family: "Serif".to_string(),
            font1_size: 14.0,
            font2_family: "Serif".to_string(),
            font2_size: 11.0,
        }
    }

    /// Default theme for Steampunk panels - Victorian industrial brass/copper
    pub fn default_for_steampunk() -> Self {
        Self {
            color1: Color::new(0.804, 0.608, 0.114, 1.0), // Brass #CD9B1D
            color2: Color::new(0.722, 0.451, 0.200, 1.0), // Copper #B87333
            color3: Color::new(0.545, 0.412, 0.078, 1.0), // Bronze #8B6914
            color4: Color::new(0.180, 0.137, 0.098, 1.0), // Dark brown #2E2319
            gradient: LinearGradientSourceConfig {
                angle: 180.0,
                stops: vec![
                    ColorStopSource::custom(0.0, Color::new(0.25, 0.20, 0.15, 1.0)), // Dark sepia top
                    ColorStopSource::custom(1.0, Color::new(0.15, 0.12, 0.08, 1.0)), // Darker brown bottom
                ],
            },
            font1_family: "Serif".to_string(),
            font1_size: 15.0,
            font2_family: "Sans".to_string(),
            font2_size: 11.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_color_source_new_format_theme() {
        let json = r#"{"type": "Theme", "index": 2}"#;
        let result: ColorSource = serde_json::from_str(json).unwrap();
        assert_eq!(result, ColorSource::Theme { index: 2 });
    }

    #[test]
    fn test_deserialize_color_source_new_format_custom() {
        let json = r#"{"type": "Custom", "color": {"r": 1.0, "g": 0.5, "b": 0.0, "a": 1.0}}"#;
        let result: ColorSource = serde_json::from_str(json).unwrap();
        match result {
            ColorSource::Custom { color } => {
                assert!((color.r - 1.0).abs() < 0.001);
                assert!((color.g - 0.5).abs() < 0.001);
                assert!((color.b - 0.0).abs() < 0.001);
            }
            _ => panic!("Expected Custom variant"),
        }
    }

    #[test]
    fn test_deserialize_legacy_color_as_source() {
        // Legacy format should be auto-converted to ColorSource::Custom
        #[derive(Deserialize)]
        struct TestStruct {
            #[serde(deserialize_with = "deserialize_color_or_source")]
            color: ColorSource,
        }

        let json = r#"{"color": {"r": 0.0, "g": 1.0, "b": 0.0, "a": 0.8}}"#;
        let result: TestStruct = serde_json::from_str(json).unwrap();
        match result.color {
            ColorSource::Custom { color } => {
                assert!((color.r - 0.0).abs() < 0.001);
                assert!((color.g - 1.0).abs() < 0.001);
                assert!((color.b - 0.0).abs() < 0.001);
                assert!((color.a - 0.8).abs() < 0.001);
            }
            _ => panic!("Expected Custom variant from legacy format"),
        }
    }

    #[test]
    fn test_deserialize_color_stops_vec_legacy() {
        #[derive(Deserialize)]
        struct TestStruct {
            #[serde(deserialize_with = "deserialize_color_stops_vec")]
            stops: Vec<ColorStopSource>,
        }

        // Legacy gradient stops with raw colors
        let json = r#"{
            "stops": [
                {"position": 0.0, "color": {"r": 1.0, "g": 0.0, "b": 0.0, "a": 1.0}},
                {"position": 1.0, "color": {"r": 0.0, "g": 0.0, "b": 1.0, "a": 1.0}}
            ]
        }"#;

        let result: TestStruct = serde_json::from_str(json).unwrap();
        assert_eq!(result.stops.len(), 2);
        assert!((result.stops[0].position - 0.0).abs() < 0.001);
        assert!((result.stops[1].position - 1.0).abs() < 0.001);

        // Both should be Custom variants from legacy format
        match &result.stops[0].color {
            ColorSource::Custom { color } => {
                assert!((color.r - 1.0).abs() < 0.001);
                assert!((color.g - 0.0).abs() < 0.001);
            }
            _ => panic!("Expected Custom variant"),
        }
    }

    #[test]
    fn test_deserialize_color_stops_vec_new_format() {
        #[derive(Deserialize)]
        struct TestStruct {
            #[serde(deserialize_with = "deserialize_color_stops_vec")]
            stops: Vec<ColorStopSource>,
        }

        // New format with theme references
        let json = r#"{
            "stops": [
                {"position": 0.0, "color": {"type": "Theme", "index": 1}},
                {"position": 1.0, "color": {"type": "Theme", "index": 2}}
            ]
        }"#;

        let result: TestStruct = serde_json::from_str(json).unwrap();
        assert_eq!(result.stops.len(), 2);
        assert_eq!(result.stops[0].color, ColorSource::Theme { index: 1 });
        assert_eq!(result.stops[1].color, ColorSource::Theme { index: 2 });
    }
}
