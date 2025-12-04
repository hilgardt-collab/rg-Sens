//! UI components

mod main_window;
mod grid_layout;
mod config_dialog;
mod color_picker;
pub mod background;
mod gradient_editor;
mod background_config_widget;

pub use main_window::MainWindow;
pub use grid_layout::{GridConfig, GridLayout};
pub use config_dialog::ConfigDialog;
pub use color_picker::ColorPickerDialog;
pub use background::{BackgroundConfig, BackgroundType, Color, ColorStop, LinearGradientConfig, RadialGradientConfig, PolygonConfig, render_background};
pub use gradient_editor::GradientEditor;
pub use background_config_widget::BackgroundConfigWidget;
