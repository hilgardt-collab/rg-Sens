//! Global clipboard for copy/paste operations across config dialogs

use once_cell::sync::Lazy;
use std::sync::Mutex;

use crate::displayers::TextLineConfig;
use crate::ui::{BackgroundConfig, CpuSourceConfig, GpuSourceConfig};

/// Global clipboard that persists across config dialogs
pub static CLIPBOARD: Lazy<Mutex<Clipboard>> = Lazy::new(|| {
    Mutex::new(Clipboard::default())
});

/// Clipboard data structure
#[derive(Debug, Default, Clone)]
pub struct Clipboard {
    /// Copied font (family, size)
    pub font: Option<(String, f64)>,
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
}

impl Clipboard {
    /// Copy font to clipboard
    pub fn copy_font(&mut self, family: String, size: f64) {
        self.font = Some((family, size));
    }

    /// Paste font from clipboard
    pub fn paste_font(&self) -> Option<(String, f64)> {
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

    /// Clear all clipboard data
    pub fn clear(&mut self) {
        self.font = None;
        self.color = None;
        self.text_line = None;
        self.background = None;
        self.cpu_source = None;
        self.gpu_source = None;
    }
}
