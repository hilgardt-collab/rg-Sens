//! UI components

mod render_utils;
mod main_window;
mod grid_layout;
mod config_dialog;
mod color_picker;
pub mod custom_color_picker;
pub mod background;
mod gradient_editor;
mod background_config_widget;
mod image_picker;
mod text_line_config_widget;
mod cpu_source_config_widget;
mod gpu_source_config_widget;
mod memory_source_config_widget;
mod system_temp_config_widget;
mod fan_speed_config_widget;
mod disk_source_config_widget;
mod clock_source_config_widget;
mod clock_analog_config_widget;
mod clock_digital_config_widget;
mod combo_source_config_widget;
mod timezone_dialog;
mod alarm_timer_dialog;
pub mod clipboard;
pub mod bar_display;
mod bar_config_widget;
pub mod arc_display;
mod arc_config_widget;
pub mod speedometer_display;
mod speedometer_config_widget;
pub mod graph_display;
mod graph_config_widget;
pub mod text_renderer;
mod shared_font_dialog;
pub mod render_cache;
pub mod clock_display;
pub mod lcars_display;
mod lcars_config_widget;
mod color_button_widget;
pub mod core_bars_display;
mod core_bars_config_widget;
mod indicator_config_widget;
mod test_source_dialog;
mod test_source_config_widget;
mod static_text_config_widget;

pub use main_window::MainWindow;
pub use test_source_dialog::{show_test_source_dialog, show_test_source_dialog_with_callback, TestSourceSaveCallback};
pub use color_button_widget::ColorButtonWidget;
pub use grid_layout::{GridConfig, GridLayout, BorderlessDragCallback};
pub use config_dialog::ConfigDialog;
pub use color_picker::ColorPickerDialog;
pub use background::{BackgroundConfig, BackgroundType, Color, ColorStop, LinearGradientConfig, RadialGradientConfig, PolygonConfig, render_background, render_background_with_source, ImageDisplayMode, IndicatorBackgroundConfig, IndicatorBackgroundShape, render_indicator_background_with_value};
pub use gradient_editor::GradientEditor;
pub use background_config_widget::BackgroundConfigWidget;
pub use image_picker::ImagePicker;
pub use text_line_config_widget::TextLineConfigWidget;
pub use cpu_source_config_widget::{CpuSourceConfigWidget, CpuSourceConfig, CpuField, TemperatureUnit, FrequencyUnit, CoreSelection};
pub use gpu_source_config_widget::{GpuSourceConfigWidget, GpuSourceConfig, GpuField, MemoryUnit, FrequencyUnit as GpuFrequencyUnit};
pub use memory_source_config_widget::{MemorySourceConfigWidget, MemorySourceConfig, MemoryField};
pub use system_temp_config_widget::SystemTempConfigWidget;
pub use fan_speed_config_widget::FanSpeedConfigWidget;
pub use disk_source_config_widget::{DiskSourceConfigWidget, DiskSourceConfig, DiskField, DiskUnit};
pub use bar_display::{BarDisplayConfig, BarStyle, BarOrientation, BarFillDirection, BarFillType, BarBackgroundType, BorderConfig, render_bar};
pub use bar_config_widget::BarConfigWidget;
pub use arc_display::{ArcDisplayConfig, ArcCapStyle, ArcTaperStyle, ColorTransitionStyle, ColorApplicationMode, render_arc};
pub use arc_config_widget::ArcConfigWidget;
pub use speedometer_display::{SpeedometerConfig, NeedleStyle, NeedleTailStyle, TickStyle, BezelStyle, ValueZone, TextOverlayConfig, TickLabelConfig, render_speedometer};
pub use speedometer_config_widget::SpeedometerConfigWidget;
pub use graph_display::{GraphDisplayConfig, GraphType, LineStyle, FillMode, AxisConfig, Margin, DataPoint, render_graph};
pub use graph_config_widget::GraphConfigWidget;
pub use clipboard::{PanelStyle, CLIPBOARD};
pub use clock_display::{AnalogClockConfig, HandStyle, FaceStyle, TickStyle as ClockTickStyle, render_analog_clock};
pub use clock_source_config_widget::ClockSourceConfigWidget;
pub use clock_analog_config_widget::ClockAnalogConfigWidget;
pub use clock_digital_config_widget::ClockDigitalConfigWidget;
pub use combo_source_config_widget::ComboSourceConfigWidget;
pub use timezone_dialog::TimezoneDialog;
pub use alarm_timer_dialog::{AlarmTimerDialog, TimerAction};
pub use lcars_display::{
    LcarsFrameConfig, SidebarPosition, ExtensionMode, CornerStyle, HeaderShape, HeaderAlign,
    HeaderWidthMode, HeaderPosition, DividerCapStyle, SplitOrientation, ContentDisplayType,
    SegmentConfig, HeaderConfig, DividerConfig, ContentItemConfig,
    ContentItemData, render_lcars_frame, render_content_background, render_divider,
    render_content_bar, render_content_text, render_content_core_bars, get_content_bounds, calculate_item_layouts,
};
pub use lcars_config_widget::LcarsConfigWidget;
pub use core_bars_display::{CoreBarsConfig, LabelPosition, render_core_bars};
pub use core_bars_config_widget::CoreBarsConfigWidget;
pub use indicator_config_widget::IndicatorConfigWidget;
pub use test_source_config_widget::TestSourceConfigWidget;
pub use static_text_config_widget::StaticTextConfigWidget;
