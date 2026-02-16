//! Foundational color types used throughout rg-Sens.
//!
//! Color, ColorStop, and gradient config types are the building blocks
//! for all visual configuration in the system.

use serde::{Deserialize, Serialize};

/// RGBA color with alpha channel
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

impl Color {
    pub fn new(r: f64, g: f64, b: f64, a: f64) -> Self {
        Self { r, g, b, a }
    }

    pub fn from_rgba8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            r: r as f64 / 255.0,
            g: g as f64 / 255.0,
            b: b as f64 / 255.0,
            a: a as f64 / 255.0,
        }
    }

    pub fn to_rgba8(&self) -> (u8, u8, u8, u8) {
        (
            (self.r * 255.0) as u8,
            (self.g * 255.0) as u8,
            (self.b * 255.0) as u8,
            (self.a * 255.0) as u8,
        )
    }

    /// Convert to GTK RGBA
    #[cfg(feature = "gtk")]
    pub fn to_gdk_rgba(&self) -> gdk4::RGBA {
        gdk4::RGBA::new(self.r as f32, self.g as f32, self.b as f32, self.a as f32)
    }

    /// Create from GTK RGBA
    #[cfg(feature = "gtk")]
    pub fn from_gdk_rgba(rgba: &gdk4::RGBA) -> Self {
        Self {
            r: rgba.red() as f64,
            g: rgba.green() as f64,
            b: rgba.blue() as f64,
            a: rgba.alpha() as f64,
        }
    }

    /// Apply to Cairo context
    #[cfg(feature = "gtk")]
    pub fn apply_to_cairo(&self, cr: &cairo::Context) {
        cr.set_source_rgba(self.r, self.g, self.b, self.a);
    }
}

impl Default for Color {
    fn default() -> Self {
        Self::new(0.0, 0.0, 0.0, 1.0)
    }
}

/// Color stop for gradients
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ColorStop {
    pub position: f64, // 0.0 to 1.0
    pub color: Color,
}

impl ColorStop {
    pub fn new(position: f64, color: Color) -> Self {
        Self { position, color }
    }
}

/// Linear gradient configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LinearGradientConfig {
    pub angle: f64, // Angle in degrees (0 = left to right, 90 = top to bottom)
    pub stops: Vec<ColorStop>,
}

impl Default for LinearGradientConfig {
    fn default() -> Self {
        Self {
            angle: 90.0,
            stops: vec![
                ColorStop::new(0.0, Color::new(0.2, 0.2, 0.2, 1.0)),
                ColorStop::new(1.0, Color::new(0.1, 0.1, 0.1, 1.0)),
            ],
        }
    }
}

/// Radial gradient configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RadialGradientConfig {
    pub center_x: f64, // 0.0 to 1.0 (relative to width)
    pub center_y: f64, // 0.0 to 1.0 (relative to height)
    pub radius: f64,   // 0.0 to 1.0 (relative to diagonal)
    pub stops: Vec<ColorStop>,
}

impl Default for RadialGradientConfig {
    fn default() -> Self {
        Self {
            center_x: 0.5,
            center_y: 0.5,
            radius: 0.7,
            stops: vec![
                ColorStop::new(0.0, Color::new(0.3, 0.3, 0.3, 1.0)),
                ColorStop::new(1.0, Color::new(0.1, 0.1, 0.1, 1.0)),
            ],
        }
    }
}
