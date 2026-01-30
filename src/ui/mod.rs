//! UI components

mod alarm_timer_dialog;
mod arc_config_widget;
pub mod arc_display;
pub mod background;
mod background_config_widget;
mod bar_config_widget;
pub mod bar_display;
pub mod clipboard;
mod clock_analog_config_widget;
mod clock_digital_config_widget;
mod clock_source_config_widget;
mod color_picker;
mod combo_source_config_widget;
mod cpu_source_config_widget;
pub mod custom_color_picker;
mod disk_source_config_widget;
mod fan_speed_config_widget;
mod gpu_source_config_widget;
mod gradient_editor;
mod graph_config_widget;
pub mod graph_display;
mod grid_layout;
mod grid_properties_dialog;
mod memory_source_config_widget;
mod network_source_config_widget;
mod position_grid_widget;
mod render_utils;
mod shared_font_dialog;
mod speedometer_config_widget;
pub mod speedometer_display;
mod system_temp_config_widget;
mod text_line_config_widget;
mod text_overlay_config_widget;
pub mod text_renderer;
mod timezone_dialog;
pub mod widget_builder;
pub use shared_font_dialog::{init_shared_font_dialog, warm_font_cache};
mod art_deco_config_widget;
pub mod art_deco_display;
mod art_nouveau_config_widget;
pub mod art_nouveau_display;
pub mod auto_scroll;
pub mod clock_display;
mod color_button_widget;
pub mod combo_config_base;
pub mod config;
pub mod config_helpers;
pub mod context_menu;
mod core_bars_config_widget;
pub mod core_bars_display;
#[cfg(feature = "webkit")]
mod css_template_config_widget; // Config widget only when webkit is available
pub mod css_template_display; // Config types always available for serialization
mod cyberpunk_config_widget;
pub mod cyberpunk_display;
mod fighter_hud_config_widget;
pub mod fighter_hud_display;
mod global_theme_widget;
mod indicator_config_widget;
mod industrial_config_widget;
pub mod industrial_display;
mod lcars_config_widget;
pub mod lcars_display;
mod material_config_widget;
pub mod material_display;
pub mod new_panel_dialog;
pub mod pango_text;
pub mod render_cache;
mod retro_terminal_config_widget;
pub mod retro_terminal_display;
mod static_config_widget;
mod static_text_config_widget;
mod steampunk_config_widget;
pub mod steampunk_display;
mod synthwave_config_widget;
pub mod synthwave_display;
mod test_source_config_widget;
mod test_source_dialog;
pub mod theme;
mod theme_color_selector;
mod theme_font_selector;
pub mod window_settings_dialog;

pub use alarm_timer_dialog::{AlarmTimerDialog, TimerAction};
pub use arc_config_widget::{ArcConfigWidget, LazyArcConfigWidget};
pub use arc_display::{
    render_arc, ArcCapStyle, ArcDisplayConfig, ArcTaperStyle, ColorApplicationMode,
    ColorTransitionStyle,
};
pub use art_deco_config_widget::ArtDecoConfigWidget;
pub use art_deco_display::{
    calculate_group_layouts as art_deco_calculate_group_layouts,
    draw_group_dividers as art_deco_draw_group_dividers, render_art_deco_frame, ArtDecoFrameConfig,
    ArtDecoRenderer, BackgroundPattern as ArtDecoBackgroundPattern,
    BorderStyle as ArtDecoBorderStyle, CornerStyle as ArtDecoCornerStyle,
    DividerStyle as ArtDecoDividerStyle, HeaderStyle as ArtDecoHeaderStyle,
};
pub use art_nouveau_config_widget::ArtNouveauConfigWidget;
pub use art_nouveau_display::{
    calculate_group_layouts as art_nouveau_calculate_group_layouts,
    draw_group_dividers as art_nouveau_draw_group_dividers, render_art_nouveau_frame,
    ArtNouveauFrameConfig, ArtNouveauRenderer, BackgroundPattern as ArtNouveauBackgroundPattern,
    BorderStyle as ArtNouveauBorderStyle, CornerStyle as ArtNouveauCornerStyle,
    DividerStyle as ArtNouveauDividerStyle, HeaderStyle as ArtNouveauHeaderStyle,
};
pub use background::{
    render_background, render_background_with_source, render_background_with_source_and_theme,
    render_background_with_theme, render_indicator_background_with_value, BackgroundConfig,
    BackgroundType, Color, ColorStop, ImageDisplayMode, IndicatorBackgroundConfig,
    IndicatorBackgroundShape, LinearGradientConfig, PolygonConfig, RadialGradientConfig,
};
pub use background_config_widget::BackgroundConfigWidget;
pub use bar_config_widget::{BarConfigWidget, LazyBarConfigWidget};
pub use bar_display::{
    render_bar, BarBackgroundType, BarDisplayConfig, BarFillDirection, BarFillType, BarOrientation,
    BarStyle, BorderConfig,
};
pub use clipboard::{PanelStyle, CLIPBOARD};
pub use clock_analog_config_widget::ClockAnalogConfigWidget;
pub use clock_digital_config_widget::ClockDigitalConfigWidget;
pub use clock_display::{
    render_analog_clock, AnalogClockConfig, FaceStyle, HandStyle, TickStyle as ClockTickStyle,
};
pub use clock_source_config_widget::ClockSourceConfigWidget;
pub use color_button_widget::ColorButtonWidget;
pub use color_picker::ColorPickerDialog;
pub use combo_source_config_widget::ComboSourceConfigWidget;
pub use core_bars_config_widget::{CoreBarsConfigWidget, LazyCoreBarsConfigWidget};
pub use core_bars_display::{render_core_bars, CoreBarsConfig, LabelPosition};
pub use cpu_source_config_widget::{
    CoreSelection, CpuField, CpuSourceConfig, CpuSourceConfigWidget, FrequencyUnit, TemperatureUnit,
};
#[cfg(feature = "webkit")]
pub use css_template_config_widget::CssTemplateConfigWidget;
pub use css_template_display::{CssTemplateDisplayConfig, PlaceholderMapping};
pub use cyberpunk_config_widget::CyberpunkConfigWidget;
pub use cyberpunk_display::{
    calculate_group_layouts, draw_group_dividers, render_cyberpunk_frame,
    CornerStyle as CyberpunkCornerStyle, CyberpunkFrameConfig, CyberpunkRenderer,
    DividerStyle as CyberpunkDividerStyle, HeaderStyle as CyberpunkHeaderStyle,
};
pub use disk_source_config_widget::{
    DiskField, DiskSourceConfig, DiskSourceConfigWidget, DiskUnit,
};
pub use fan_speed_config_widget::FanSpeedConfigWidget;
pub use fighter_hud_config_widget::FighterHudConfigWidget;
pub use fighter_hud_display::{
    calculate_group_layouts as fighter_hud_calculate_group_layouts,
    draw_group_dividers as fighter_hud_draw_group_dividers, render_fighter_hud_frame,
    FighterHudFrameConfig, FighterHudRenderer, HudColorPreset, HudDividerStyle, HudFrameStyle,
    HudHeaderStyle,
};
pub use global_theme_widget::GlobalThemeWidget;
pub use gpu_source_config_widget::{
    FrequencyUnit as GpuFrequencyUnit, GpuField, GpuSourceConfig, GpuSourceConfigWidget, MemoryUnit,
};
pub use gradient_editor::GradientEditor;
pub use graph_config_widget::{GraphConfigWidget, LazyGraphConfigWidget};
pub use graph_display::{
    render_graph, AxisConfig, DataPoint, FillMode, GraphDisplayConfig, GraphType, LineStyle, Margin,
};
pub use grid_layout::{BorderlessDragCallback, GridConfig, GridLayout};
pub use indicator_config_widget::IndicatorConfigWidget;
pub use industrial_config_widget::IndustrialConfigWidget;
pub use industrial_display::{
    calculate_group_layouts as industrial_calculate_group_layouts,
    draw_group_dividers as industrial_draw_group_dividers,
    draw_group_panel as industrial_draw_group_panel, render_industrial_frame,
    DividerStyle as IndustrialDividerStyle, HeaderStyle as IndustrialHeaderStyle,
    IndustrialFrameConfig, IndustrialRenderer, RivetStyle, SurfaceTexture, WarningStripePosition,
};
pub use lcars_config_widget::LcarsConfigWidget;
pub use lcars_display::{
    calculate_item_layouts, calculate_item_layouts_with_orientation, get_content_bounds,
    render_content_background, render_content_bar, render_content_core_bars, render_content_text,
    render_divider, render_lcars_frame, ContentDisplayType, ContentItemConfig, ContentItemData,
    CornerStyle, DividerCapStyle, DividerConfig, ExtensionMode, HeaderAlign, HeaderConfig,
    HeaderPosition, HeaderShape, HeaderWidthMode, LcarsFrameConfig, LcarsRenderer, SegmentConfig,
    SidebarPosition, SplitOrientation,
};
pub use material_config_widget::MaterialConfigWidget;
pub use material_display::{
    calculate_group_layouts as material_calculate_group_layouts,
    draw_group_dividers as material_draw_group_dividers, render_material_frame, CardElevation,
    DividerStyle as MaterialDividerStyle, HeaderStyle as MaterialHeaderStyle, MaterialFrameConfig,
    MaterialRenderer, ThemeVariant,
};
pub use memory_source_config_widget::{MemoryField, MemorySourceConfig, MemorySourceConfigWidget};
pub use network_source_config_widget::{
    NetworkField, NetworkSourceConfig, NetworkSourceConfigWidget, NetworkSpeedUnit,
    NetworkTotalUnit,
};
pub use retro_terminal_config_widget::RetroTerminalConfigWidget;
pub use retro_terminal_display::{
    calculate_group_layouts as retro_calculate_group_layouts,
    draw_group_dividers as retro_draw_group_dividers, render_retro_terminal_frame,
    BezelStyle as RetroBezelStyle, PhosphorColor, RetroTerminalFrameConfig, RetroTerminalRenderer,
    TerminalDividerStyle, TerminalHeaderStyle,
};
pub use speedometer_config_widget::{LazySpeedometerConfigWidget, SpeedometerConfigWidget};
pub use speedometer_display::{
    render_speedometer, BezelStyle, NeedleStyle, NeedleTailStyle, SpeedometerConfig,
    TickLabelConfig, TickStyle, ValueZone,
};
pub use static_config_widget::{LazyStaticConfigWidget, StaticConfigWidget};
pub use static_text_config_widget::StaticTextConfigWidget;
pub use steampunk_config_widget::SteampunkConfigWidget;
pub use steampunk_display::{
    calculate_group_layouts as steampunk_calculate_group_layouts,
    draw_group_dividers as steampunk_draw_group_dividers, render_steampunk_frame,
    BackgroundTexture, BorderStyle as SteampunkBorderStyle, CornerStyle as SteampunkCornerStyle,
    DividerStyle as SteampunkDividerStyle, HeaderStyle as SteampunkHeaderStyle,
    SteampunkFrameConfig, SteampunkRenderer,
};
pub use synthwave_config_widget::SynthwaveConfigWidget;
pub use synthwave_display::{
    calculate_group_layouts as synthwave_calculate_group_layouts,
    draw_group_dividers as synthwave_draw_group_dividers, render_scanline_overlay,
    render_synthwave_frame, GridStyle, SynthwaveColorScheme, SynthwaveDividerStyle,
    SynthwaveFrameConfig, SynthwaveFrameStyle, SynthwaveHeaderStyle, SynthwaveRenderer,
};
pub use system_temp_config_widget::SystemTempConfigWidget;
pub use test_source_config_widget::TestSourceConfigWidget;
pub use test_source_dialog::{
    show_test_source_dialog, show_test_source_dialog_with_callback, TestSourceSaveCallback,
};
pub use text_line_config_widget::{LazyTextLineConfigWidget, TextLineConfigWidget};
pub use text_overlay_config_widget::{
    LazyTextOverlayConfigWidget, TextOverlayConfig, TextOverlayConfigWidget,
};
pub use theme_color_selector::ThemeColorSelector;
pub use theme_font_selector::ThemeFontSelector;
pub use timezone_dialog::TimezoneDialog;

// Stub type when webkit is disabled - provides same interface but no functionality
#[cfg(not(feature = "webkit"))]
pub struct CssTemplateConfigWidget {
    widget: gtk4::Box,
}

#[cfg(not(feature = "webkit"))]
impl CssTemplateConfigWidget {
    pub fn new() -> Self {
        use gtk4::prelude::*;
        let widget = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        let label = gtk4::Label::new(Some("CSS Template requires webkit feature"));
        widget.append(&label);
        Self { widget }
    }
    pub fn widget(&self) -> &gtk4::Box {
        &self.widget
    }
    pub fn set_config(&self, _config: &CssTemplateDisplayConfig) {}
    pub fn get_config(&self) -> CssTemplateDisplayConfig {
        CssTemplateDisplayConfig::default()
    }
    pub fn set_source_summaries(&self, _summaries: Vec<(String, String, usize, u32)>) {}
    pub fn set_on_change<F: Fn() + 'static>(&self, _f: F) {}
}

// Dialog close functions
pub use alarm_timer_dialog::close_alarm_timer_dialog;
pub use grid_properties_dialog::close_panel_properties_dialog;
pub use timezone_dialog::close_timezone_dialog;

/// Close all open singleton dialogs
/// Call this when the main window is closing to clean up
pub fn close_all_dialogs() {
    close_panel_properties_dialog();
    close_alarm_timer_dialog();
    close_timezone_dialog();
}
