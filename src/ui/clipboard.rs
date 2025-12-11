//! Global clipboard for copy/paste operations across config dialogs

use once_cell::sync::Lazy;
use std::sync::Mutex;

use crate::displayers::TextLineConfig;
use crate::ui::{BackgroundConfig, ColorStop, CpuSourceConfig, GpuSourceConfig};
use crate::core::PanelBorderConfig;
use serde_json::Value;
use std::collections::HashMap;

/// Global clipboard that persists across config dialogs
pub static CLIPBOARD: Lazy<Mutex<Clipboard>> = Lazy::new(|| {
    Mutex::new(Clipboard::default())
});

/// Panel style (all visual settings except source config)
#[derive(Debug, Clone)]
pub struct PanelStyle {
    pub background: BackgroundConfig,
    pub corner_radius: f64,
    pub border: PanelBorderConfig,
    pub displayer_config: HashMap<String, Value>,
}

/// Clipboard data structure
#[derive(Debug, Default, Clone)]
pub struct Clipboard {
    /// Copied font (family, size, bold, italic)
    pub font: Option<(String, f64, bool, bool)>,
    /// Copied color (r, g, b, a)
    pub color: Option<(f64, f64, f64, f64)>,
    /// Copied text line configuration
    pub text_line: Option<TextLineConfig>,
    /// Copied background configuration
    pub background: Option<BackgroundConfig>,
    /// Copied CPU source configuration
    pub cpu_source: Option<CpuSourceConfig>,
    /// Copied GPU source configuration
    pub gpu_source: Option<GpuSourceConfig>,
    /// Copied panel style (background, border, corner radius, displayer config)
    pub panel_style: Option<PanelStyle>,
    /// Copied gradient color stops (universal for all gradient types - backgrounds and arc displayers)
    pub gradient_stops: Option<Vec<ColorStop>>,
}

impl Clipboard {
    /// Copy font to clipboard
    pub fn copy_font(&mut self, family: String, size: f64, bold: bool, italic: bool) {
        self.font = Some((family, size, bold, italic));
    }

    /// Paste font from clipboard
    pub fn paste_font(&self) -> Option<(String, f64, bool, bool)> {
        self.font.clone()
    }

    /// Copy color to clipboard
    pub fn copy_color(&mut self, r: f64, g: f64, b: f64, a: f64) {
        self.color = Some((r, g, b, a));
    }

    /// Paste color from clipboard
    pub fn paste_color(&self) -> Option<(f64, f64, f64, f64)> {
        self.color
    }

    /// Copy text line configuration to clipboard
    pub fn copy_text_line(&mut self, config: TextLineConfig) {
        self.text_line = Some(config);
    }

    /// Paste text line configuration from clipboard
    pub fn paste_text_line(&self) -> Option<TextLineConfig> {
        self.text_line.clone()
    }

    /// Copy background configuration to clipboard
    pub fn copy_background(&mut self, config: BackgroundConfig) {
        self.background = Some(config);
    }

    /// Paste background configuration from clipboard
    pub fn paste_background(&self) -> Option<BackgroundConfig> {
        self.background.clone()
    }

    /// Copy CPU source configuration to clipboard
    pub fn copy_cpu_source(&mut self, config: CpuSourceConfig) {
        self.cpu_source = Some(config);
    }

    /// Paste CPU source configuration from clipboard
    pub fn paste_cpu_source(&self) -> Option<CpuSourceConfig> {
        self.cpu_source.clone()
    }

    /// Copy GPU source configuration to clipboard
    pub fn copy_gpu_source(&mut self, config: GpuSourceConfig) {
        self.gpu_source = Some(config);
    }

    /// Paste GPU source configuration from clipboard
    pub fn paste_gpu_source(&self) -> Option<GpuSourceConfig> {
        self.gpu_source.clone()
    }

    /// Copy panel style to clipboard (background, border, corner radius, displayer config)
    pub fn copy_panel_style(&mut self, style: PanelStyle) {
        self.panel_style = Some(style);
    }

    /// Paste panel style from clipboard
    pub fn paste_panel_style(&self) -> Option<PanelStyle> {
        self.panel_style.clone()
    }

    /// Copy gradient color stops to clipboard (universal for all gradient types)
    pub fn copy_gradient_stops(&mut self, stops: Vec<ColorStop>) {
        self.gradient_stops = Some(stops);
    }

    /// Paste gradient color stops from clipboard
    pub fn paste_gradient_stops(&self) -> Option<Vec<ColorStop>> {
        self.gradient_stops.clone()
    }

    /// Clear all clipboard data
    pub fn clear(&mut self) {
        self.font = None;
        self.color = None;
        self.text_line = None;
        self.background = None;
        self.cpu_source = None;
        self.gpu_source = None;
        self.panel_style = None;
        self.gradient_stops = None;
    }
}
