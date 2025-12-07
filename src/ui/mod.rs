//! UI components

mod main_window;
mod grid_layout;
mod config_dialog;
mod color_picker;
pub mod background;
mod gradient_editor;
mod background_config_widget;
mod image_picker;
mod text_line_config_widget;
mod cpu_source_config_widget;
mod gpu_source_config_widget;
pub mod clipboard;
pub mod bar_display;
mod bar_config_widget;
pub mod text_renderer;
mod shared_font_dialog;

pub use main_window::MainWindow;
pub use grid_layout::{GridConfig, GridLayout};
pub use config_dialog::ConfigDialog;
pub use color_picker::ColorPickerDialog;
pub use background::{BackgroundConfig, BackgroundType, Color, ColorStop, LinearGradientConfig, RadialGradientConfig, PolygonConfig, render_background, ImageDisplayMode};
pub use gradient_editor::GradientEditor;
pub use background_config_widget::BackgroundConfigWidget;
pub use image_picker::ImagePicker;
pub use text_line_config_widget::TextLineConfigWidget;
pub use cpu_source_config_widget::{CpuSourceConfigWidget, CpuSourceConfig, CpuField, TemperatureUnit, FrequencyUnit, CoreSelection};
pub use gpu_source_config_widget::{GpuSourceConfigWidget, GpuSourceConfig, GpuField, MemoryUnit};
pub use bar_display::{BarDisplayConfig, BarStyle, BarOrientation, BarFillDirection, BarFillType, BarBackgroundType, TextOverlayConfig, BorderConfig, render_bar};
pub use bar_config_widget::BarConfigWidget;
