//! Global clipboard for copy/paste operations across config dialogs

use once_cell::sync::Lazy;
use std::sync::Mutex;

use crate::displayers::{TextLineConfig, TextDisplayerConfig};
use crate::ui::{BackgroundConfig, ColorStop, CpuSourceConfig, GpuSourceConfig};
use crate::ui::bar_display::BarDisplayConfig;
use crate::ui::theme::FontSource;
use crate::ui::graph_display::GraphDisplayConfig;
use crate::ui::arc_display::ArcDisplayConfig;
use crate::ui::speedometer_display::SpeedometerConfig;
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
    /// Copied plain text
    pub text: Option<String>,
    /// Copied font (family, size, bold, italic) - legacy format
    pub font: Option<(String, f64, bool, bool)>,
    /// Copied font source (preserves theme reference)
    pub font_source: Option<(FontSource, bool, bool)>,
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
    /// Copied bar display configuration
    pub bar_display: Option<BarDisplayConfig>,
    /// Copied graph display configuration
    pub graph_display: Option<GraphDisplayConfig>,
    /// Copied text displayer configuration (all lines)
    pub text_display: Option<TextDisplayerConfig>,
    /// Copied arc display configuration
    pub arc_display: Option<ArcDisplayConfig>,
    /// Copied speedometer display configuration
    pub speedometer_display: Option<SpeedometerConfig>,
    /// Copied generic source configuration as JSON (for combo slots)
    pub source_config: Option<(String, Value)>, // (source_type, config_json)
}

impl Clipboard {
    /// Copy plain text to clipboard
    pub fn copy_text(&mut self, text: String) {
        self.text = Some(text);
    }

    /// Paste plain text from clipboard
    pub fn paste_text(&self) -> Option<String> {
        self.text.clone()
    }

    /// Copy font to clipboard
    pub fn copy_font(&mut self, family: String, size: f64, bold: bool, italic: bool) {
        self.font = Some((family, size, bold, italic));
    }

    /// Paste font from clipboard
    pub fn paste_font(&self) -> Option<(String, f64, bool, bool)> {
        self.font.clone()
    }

    /// Copy font source to clipboard (preserves theme reference)
    pub fn copy_font_source(&mut self, source: FontSource, bold: bool, italic: bool) {
        self.font_source = Some((source, bold, italic));
    }

    /// Paste font source from clipboard
    pub fn paste_font_source(&self) -> Option<(FontSource, bool, bool)> {
        self.font_source.clone()
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

    /// Copy bar display configuration to clipboard
    pub fn copy_bar_display(&mut self, config: BarDisplayConfig) {
        self.bar_display = Some(config);
    }

    /// Paste bar display configuration from clipboard
    pub fn paste_bar_display(&self) -> Option<BarDisplayConfig> {
        self.bar_display.clone()
    }

    /// Copy graph display configuration to clipboard
    pub fn copy_graph_display(&mut self, config: GraphDisplayConfig) {
        self.graph_display = Some(config);
    }

    /// Paste graph display configuration from clipboard
    pub fn paste_graph_display(&self) -> Option<GraphDisplayConfig> {
        self.graph_display.clone()
    }

    /// Copy text displayer configuration to clipboard
    pub fn copy_text_display(&mut self, config: TextDisplayerConfig) {
        self.text_display = Some(config);
    }

    /// Paste text displayer configuration from clipboard
    pub fn paste_text_display(&self) -> Option<TextDisplayerConfig> {
        self.text_display.clone()
    }

    /// Copy arc display configuration to clipboard
    pub fn copy_arc_display(&mut self, config: ArcDisplayConfig) {
        self.arc_display = Some(config);
    }

    /// Paste arc display configuration from clipboard
    pub fn paste_arc_display(&self) -> Option<ArcDisplayConfig> {
        self.arc_display.clone()
    }

    /// Copy speedometer display configuration to clipboard
    pub fn copy_speedometer_display(&mut self, config: SpeedometerConfig) {
        self.speedometer_display = Some(config);
    }

    /// Paste speedometer display configuration from clipboard
    pub fn paste_speedometer_display(&self) -> Option<SpeedometerConfig> {
        self.speedometer_display.clone()
    }

    /// Copy generic source configuration to clipboard (for combo slots)
    pub fn copy_source_config(&mut self, source_type: String, config: Value) {
        self.source_config = Some((source_type, config));
    }

    /// Paste generic source configuration from clipboard
    /// Returns (source_type, config_json) if available
    pub fn paste_source_config(&self) -> Option<(String, Value)> {
        self.source_config.clone()
    }

    /// Clear all clipboard data
    pub fn clear(&mut self) {
        self.text = None;
        self.font = None;
        self.font_source = None;
        self.color = None;
        self.text_line = None;
        self.background = None;
        self.cpu_source = None;
        self.gpu_source = None;
        self.panel_style = None;
        self.gradient_stops = None;
        self.bar_display = None;
        self.graph_display = None;
        self.text_display = None;
        self.arc_display = None;
        self.speedometer_display = None;
        self.source_config = None;
    }
}
