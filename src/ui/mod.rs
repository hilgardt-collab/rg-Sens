//! UI components

// Re-export rendering modules from rg-sens-render crate
pub use rg_sens_render::arc_display;
pub use rg_sens_render::art_deco_display;
pub use rg_sens_render::art_nouveau_display;
pub use rg_sens_render::background;
pub use rg_sens_render::bar_display;
pub use rg_sens_render::clock_display;
pub use rg_sens_render::combo_traits;
pub use rg_sens_render::core_bars_display;
pub use rg_sens_render::cyberpunk_display;
pub use rg_sens_render::fighter_hud_display;
pub use rg_sens_render::graph_display;
pub use rg_sens_render::industrial_display;
pub use rg_sens_render::lcars_display;
pub use rg_sens_render::material_display;
pub use rg_sens_render::pango_text;
pub use rg_sens_render::render_cache;
pub use rg_sens_render::render_utils;
pub use rg_sens_render::retro_terminal_display;
pub use rg_sens_render::speedometer_display;
pub use rg_sens_render::steampunk_display;
pub use rg_sens_render::synthwave_display;
pub use rg_sens_render::text_renderer;

// Local UI modules (config widgets, dialogs, etc.)
mod alarm_timer_dialog;
mod background_config_widget;
pub mod claude_account;
pub mod clipboard;
mod color_picker;
pub mod custom_color_picker;
mod gradient_editor;
mod grid_layout;
mod position_grid_widget;
pub(crate) mod sensor_picker;
mod shared_font_dialog;
mod timezone_dialog;
pub mod widget_builder;
pub use shared_font_dialog::{init_shared_font_dialog, warm_font_cache};
pub mod auto_scroll;
mod color_button_widget;
pub mod config_helpers;
pub mod config_tree;
pub mod context_menu;
#[cfg(feature = "webkit")]
mod css_template_config_widget; // Config widget only when webkit is available
pub mod css_template_display; // Config types always available for serialization
pub mod editor_registry;
mod global_theme_widget;
pub mod new_panel_dialog;
mod test_source_config_widget;
mod test_source_dialog;
pub mod theme;
mod theme_color_selector;
mod theme_font_selector;
pub mod themes_groups_dialog;
pub mod window_settings_dialog;

pub use alarm_timer_dialog::{AlarmTimerDialog, TimerAction};
pub use arc_display::{
    render_arc, ArcCapStyle, ArcDisplayConfig, ArcTaperStyle, ColorApplicationMode,
    ColorTransitionStyle,
};
pub use art_deco_display::{
    calculate_group_layouts as art_deco_calculate_group_layouts,
    draw_group_dividers as art_deco_draw_group_dividers, render_art_deco_frame, ArtDecoFrameConfig,
    ArtDecoRenderer, BackgroundPattern as ArtDecoBackgroundPattern,
    BorderStyle as ArtDecoBorderStyle, CornerStyle as ArtDecoCornerStyle,
    DividerStyle as ArtDecoDividerStyle, HeaderStyle as ArtDecoHeaderStyle,
};
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
pub use bar_display::{
    render_bar, BarBackgroundType, BarDisplayConfig, BarFillDirection, BarFillType, BarOrientation,
    BarStyle, BorderConfig,
};
pub use clipboard::{PanelStyle, CLIPBOARD};
pub use clock_display::{
    render_analog_clock, AnalogClockConfig, FaceStyle, HandStyle, TickStyle as ClockTickStyle,
};
pub use color_button_widget::ColorButtonWidget;
pub use color_picker::ColorPickerDialog;
pub use core_bars_display::{render_core_bars, CoreBarsConfig, LabelPosition};
#[cfg(feature = "webkit")]
pub use css_template_config_widget::CssTemplateConfigWidget;
pub use css_template_display::{CssTemplateDisplayConfig, PlaceholderMapping};
pub use cyberpunk_display::{
    calculate_group_layouts, draw_group_dividers, render_cyberpunk_frame,
    CornerStyle as CyberpunkCornerStyle, CyberpunkFrameConfig, CyberpunkRenderer,
    DividerStyle as CyberpunkDividerStyle, HeaderStyle as CyberpunkHeaderStyle,
};
pub use editor_registry::ComboReorder;
pub use fighter_hud_display::{
    calculate_group_layouts as fighter_hud_calculate_group_layouts,
    draw_group_dividers as fighter_hud_draw_group_dividers, render_fighter_hud_frame,
    FighterHudFrameConfig, FighterHudRenderer, HudColorPreset, HudDividerStyle, HudFrameStyle,
    HudHeaderStyle,
};
pub use global_theme_widget::GlobalThemeWidget;
pub use gradient_editor::GradientEditor;
pub use graph_display::{
    render_graph, AxisConfig, DataPoint, FillMode, GraphDisplayConfig, GraphType, LineStyle, Margin,
};
pub use grid_layout::{BorderlessDragCallback, GridConfig, GridLayout};
pub use industrial_display::{
    calculate_group_layouts as industrial_calculate_group_layouts,
    draw_group_dividers as industrial_draw_group_dividers,
    draw_group_panel as industrial_draw_group_panel, render_industrial_frame,
    DividerStyle as IndustrialDividerStyle, HeaderStyle as IndustrialHeaderStyle,
    IndustrialFrameConfig, IndustrialRenderer, RivetStyle, SurfaceTexture, WarningStripePosition,
};
pub use lcars_display::{
    calculate_item_layouts, calculate_item_layouts_with_orientation, get_content_bounds,
    render_content_background, render_content_bar, render_content_core_bars, render_content_text,
    render_divider, render_lcars_frame, ContentDisplayType, ContentItemConfig, ContentItemData,
    CornerStyle, DividerCapStyle, DividerConfig, ExtensionMode, HeaderAlign, HeaderConfig,
    HeaderPosition, HeaderShape, HeaderWidthMode, LcarsFrameConfig, LcarsRenderer, SegmentConfig,
    SidebarPosition, SplitOrientation,
};
pub use material_display::{
    calculate_group_layouts as material_calculate_group_layouts,
    draw_group_dividers as material_draw_group_dividers, render_material_frame, CardElevation,
    DividerStyle as MaterialDividerStyle, HeaderStyle as MaterialHeaderStyle, MaterialFrameConfig,
    MaterialRenderer, ThemeVariant,
};
pub use retro_terminal_display::{
    calculate_group_layouts as retro_calculate_group_layouts,
    draw_group_dividers as retro_draw_group_dividers, render_retro_terminal_frame,
    BezelStyle as RetroBezelStyle, PhosphorColor, RetroTerminalFrameConfig, RetroTerminalRenderer,
    TerminalDividerStyle, TerminalHeaderStyle,
};
pub use rg_sens_types::source_configs::claude::{ClaudeMetric, ClaudeSourceConfig};
pub use rg_sens_types::source_configs::cpu::{
    CoreSelection, CpuField, CpuSourceConfig, FrequencyUnit, TemperatureUnit,
};
pub use rg_sens_types::source_configs::disk::{DiskField, DiskSourceConfig, DiskUnit};
pub use rg_sens_types::source_configs::gpu::{
    FrequencyUnit as GpuFrequencyUnit, GpuField, GpuSourceConfig, MemoryUnit,
};
pub use rg_sens_types::source_configs::memory::{MemoryField, MemorySourceConfig};
pub use rg_sens_types::source_configs::network::{
    NetworkField, NetworkSourceConfig, NetworkSpeedUnit, NetworkTotalUnit,
};
pub use speedometer_display::{
    render_speedometer, BezelStyle, NeedleStyle, NeedleTailStyle, SpeedometerConfig,
    TickLabelConfig, TickStyle, ValueZone,
};
pub use steampunk_display::{
    calculate_group_layouts as steampunk_calculate_group_layouts,
    draw_group_dividers as steampunk_draw_group_dividers, render_steampunk_frame,
    BackgroundTexture, BorderStyle as SteampunkBorderStyle, CornerStyle as SteampunkCornerStyle,
    DividerStyle as SteampunkDividerStyle, HeaderStyle as SteampunkHeaderStyle,
    SteampunkFrameConfig, SteampunkRenderer,
};
pub use synthwave_display::{
    calculate_group_layouts as synthwave_calculate_group_layouts,
    draw_group_dividers as synthwave_draw_group_dividers, render_scanline_overlay,
    render_synthwave_frame, GridStyle, SynthwaveColorScheme, SynthwaveDividerStyle,
    SynthwaveFrameConfig, SynthwaveFrameStyle, SynthwaveHeaderStyle, SynthwaveRenderer,
};
pub use test_source_config_widget::TestSourceConfigWidget;
pub use test_source_dialog::{
    show_test_source_dialog, show_test_source_dialog_with_callback, TestSourceSaveCallback,
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
pub use timezone_dialog::close_timezone_dialog;

/// Close all open singleton dialogs
/// Call this when the main window is closing to clean up
pub fn close_all_dialogs() {
    config_tree::close_all_tree_properties();
    close_alarm_timer_dialog();
    close_timezone_dialog();
}
