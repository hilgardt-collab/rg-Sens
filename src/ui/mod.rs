//! UI components

mod render_utils;
pub mod widget_builder;
mod main_window;
mod grid_layout;
mod grid_properties_dialog;
mod config_dialog;
mod color_picker;
pub mod custom_color_picker;
pub mod background;
mod gradient_editor;
mod background_config_widget;
mod image_picker;
mod text_line_config_widget;
mod text_overlay_config_widget;
mod position_grid_widget;
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
pub use shared_font_dialog::{init_shared_font_dialog, warm_font_cache};
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
mod static_config_widget;
mod global_theme_widget;
pub mod cyberpunk_display;
mod cyberpunk_config_widget;
pub mod material_display;
mod material_config_widget;
pub mod industrial_display;
mod industrial_config_widget;
pub mod retro_terminal_display;
mod retro_terminal_config_widget;
pub mod fighter_hud_display;
mod fighter_hud_config_widget;
pub mod synthwave_display;
mod synthwave_config_widget;
pub mod art_deco_display;
mod art_deco_config_widget;
pub mod art_nouveau_display;
mod art_nouveau_config_widget;
pub mod combo_config_base;
pub mod theme;
pub mod css_template_display;
mod css_template_config_widget;
mod theme_color_selector;
mod theme_font_selector;
pub mod window_settings_dialog;
pub mod new_panel_dialog;
pub mod config_helpers;
pub mod context_menu;
pub mod auto_scroll;

pub use main_window::MainWindow;
pub use test_source_dialog::{show_test_source_dialog, show_test_source_dialog_with_callback, TestSourceSaveCallback};
pub use color_button_widget::ColorButtonWidget;
pub use grid_layout::{GridConfig, GridLayout, BorderlessDragCallback};
pub use config_dialog::ConfigDialog;
pub use color_picker::ColorPickerDialog;
pub use background::{BackgroundConfig, BackgroundType, Color, ColorStop, LinearGradientConfig, RadialGradientConfig, PolygonConfig, render_background, render_background_with_source, render_background_with_theme, render_background_with_source_and_theme, ImageDisplayMode, IndicatorBackgroundConfig, IndicatorBackgroundShape, render_indicator_background_with_value};
pub use gradient_editor::GradientEditor;
pub use background_config_widget::BackgroundConfigWidget;
pub use image_picker::ImagePicker;
pub use text_line_config_widget::{TextLineConfigWidget, LazyTextLineConfigWidget};
pub use text_overlay_config_widget::{LazyTextOverlayConfigWidget, TextOverlayConfig, TextOverlayConfigWidget};
pub use cpu_source_config_widget::{CpuSourceConfigWidget, CpuSourceConfig, CpuField, TemperatureUnit, FrequencyUnit, CoreSelection};
pub use gpu_source_config_widget::{GpuSourceConfigWidget, GpuSourceConfig, GpuField, MemoryUnit, FrequencyUnit as GpuFrequencyUnit};
pub use memory_source_config_widget::{MemorySourceConfigWidget, MemorySourceConfig, MemoryField};
pub use system_temp_config_widget::SystemTempConfigWidget;
pub use fan_speed_config_widget::FanSpeedConfigWidget;
pub use disk_source_config_widget::{DiskSourceConfigWidget, DiskSourceConfig, DiskField, DiskUnit};
pub use bar_display::{BarDisplayConfig, BarStyle, BarOrientation, BarFillDirection, BarFillType, BarBackgroundType, BorderConfig, render_bar};
pub use bar_config_widget::{BarConfigWidget, LazyBarConfigWidget};
pub use arc_display::{ArcDisplayConfig, ArcCapStyle, ArcTaperStyle, ColorTransitionStyle, ColorApplicationMode, render_arc};
pub use arc_config_widget::ArcConfigWidget;
pub use speedometer_display::{SpeedometerConfig, NeedleStyle, NeedleTailStyle, TickStyle, BezelStyle, ValueZone, TickLabelConfig, render_speedometer};
pub use speedometer_config_widget::SpeedometerConfigWidget;
pub use graph_display::{GraphDisplayConfig, GraphType, LineStyle, FillMode, AxisConfig, Margin, DataPoint, render_graph};
pub use graph_config_widget::{GraphConfigWidget, LazyGraphConfigWidget};
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
    render_content_bar, render_content_text, render_content_core_bars, get_content_bounds,
    calculate_item_layouts, calculate_item_layouts_with_orientation,
};
pub use lcars_config_widget::LcarsConfigWidget;
pub use core_bars_display::{CoreBarsConfig, LabelPosition, render_core_bars};
pub use core_bars_config_widget::CoreBarsConfigWidget;
pub use indicator_config_widget::IndicatorConfigWidget;
pub use test_source_config_widget::TestSourceConfigWidget;
pub use static_text_config_widget::StaticTextConfigWidget;
pub use static_config_widget::StaticConfigWidget;
pub use global_theme_widget::GlobalThemeWidget;
pub use cyberpunk_display::{
    CyberpunkFrameConfig, CornerStyle as CyberpunkCornerStyle, HeaderStyle as CyberpunkHeaderStyle,
    DividerStyle as CyberpunkDividerStyle, render_cyberpunk_frame, calculate_group_layouts, draw_group_dividers,
};
pub use cyberpunk_config_widget::CyberpunkConfigWidget;
pub use material_display::{
    MaterialFrameConfig, CardElevation, HeaderStyle as MaterialHeaderStyle,
    DividerStyle as MaterialDividerStyle, ThemeVariant, render_material_frame,
    calculate_group_layouts as material_calculate_group_layouts, draw_group_dividers as material_draw_group_dividers,
};
pub use material_config_widget::MaterialConfigWidget;
pub use industrial_display::{
    IndustrialFrameConfig, SurfaceTexture, RivetStyle, WarningStripePosition,
    HeaderStyle as IndustrialHeaderStyle, DividerStyle as IndustrialDividerStyle,
    render_industrial_frame, calculate_group_layouts as industrial_calculate_group_layouts,
    draw_group_dividers as industrial_draw_group_dividers, draw_group_panel as industrial_draw_group_panel,
};
pub use industrial_config_widget::IndustrialConfigWidget;
pub use retro_terminal_display::{
    RetroTerminalFrameConfig, PhosphorColor, BezelStyle as RetroBezelStyle,
    TerminalHeaderStyle, TerminalDividerStyle, render_retro_terminal_frame,
    calculate_group_layouts as retro_calculate_group_layouts,
    draw_group_dividers as retro_draw_group_dividers,
};
pub use retro_terminal_config_widget::RetroTerminalConfigWidget;
pub use fighter_hud_display::{
    FighterHudFrameConfig, HudColorPreset, HudFrameStyle,
    HudHeaderStyle, HudDividerStyle, render_fighter_hud_frame,
    calculate_group_layouts as fighter_hud_calculate_group_layouts,
    draw_group_dividers as fighter_hud_draw_group_dividers,
};
pub use fighter_hud_config_widget::FighterHudConfigWidget;
pub use synthwave_display::{
    SynthwaveFrameConfig, SynthwaveColorScheme, SynthwaveFrameStyle,
    GridStyle, SynthwaveHeaderStyle, SynthwaveDividerStyle, render_synthwave_frame,
    render_scanline_overlay,
    calculate_group_layouts as synthwave_calculate_group_layouts,
    draw_group_dividers as synthwave_draw_group_dividers,
};
pub use synthwave_config_widget::SynthwaveConfigWidget;
pub use art_deco_config_widget::ArtDecoConfigWidget;
pub use art_nouveau_display::{
    ArtNouveauFrameConfig, BorderStyle as ArtNouveauBorderStyle,
    CornerStyle as ArtNouveauCornerStyle, BackgroundPattern as ArtNouveauBackgroundPattern,
    HeaderStyle as ArtNouveauHeaderStyle, DividerStyle as ArtNouveauDividerStyle,
    render_art_nouveau_frame, calculate_group_layouts as art_nouveau_calculate_group_layouts,
    draw_group_dividers as art_nouveau_draw_group_dividers,
};
pub use art_nouveau_config_widget::ArtNouveauConfigWidget;
pub use theme_color_selector::ThemeColorSelector;
pub use theme_font_selector::ThemeFontSelector;
pub use css_template_display::{CssTemplateDisplayConfig, PlaceholderMapping};
pub use css_template_config_widget::CssTemplateConfigWidget;

// Dialog close functions
pub use grid_properties_dialog::close_panel_properties_dialog;
pub use alarm_timer_dialog::close_alarm_timer_dialog;
pub use image_picker::close_image_picker_dialog;
pub use timezone_dialog::close_timezone_dialog;

/// Close all open singleton dialogs
/// Call this when the main window is closing to clean up
pub fn close_all_dialogs() {
    close_panel_properties_dialog();
    close_alarm_timer_dialog();
    close_image_picker_dialog();
    close_timezone_dialog();
}
