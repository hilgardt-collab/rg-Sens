//! Display configuration types for all displayer types

pub mod arc;
pub mod art_deco;
pub mod art_nouveau;
pub mod bar;
pub mod clock;
pub mod core_bars;
pub mod css_template;
pub mod cyberpunk;
pub mod digital_clock;
pub mod fighter_hud;
pub mod graph;
pub mod indicator;
pub mod industrial;
pub mod lcars;
pub mod material;
pub mod retro_terminal;
pub mod speedometer;
pub mod steampunk;
pub mod synthwave;
pub mod themed_configs;

// Re-export commonly used types
pub use arc::{ArcCapStyle, ArcDisplayConfig, ArcTaperStyle, ColorApplicationMode, ColorTransitionStyle};
pub use bar::{
    BarBackgroundType, BarDisplayConfig, BarFillDirection, BarFillType, BarOrientation, BarStyle,
    BarTaperAlignment, BarTaperStyle, BorderConfig, ResolvedBarBackground, ResolvedBarFill,
};
pub use clock::{AnalogClockConfig, FaceStyle, HandStyle};
pub use core_bars::{CoreBarsConfig, LabelPosition};
pub use css_template::{CssTemplateDisplayConfig, PlaceholderDefault, PlaceholderMapping};
pub use cyberpunk::CyberpunkFrameConfig;
pub use digital_clock::{DigitalClockConfig, DigitalStyle};
pub use graph::{AxisConfig, DataPoint, FillMode, GraphDisplayConfig, GraphType, LineStyle, Margin};
pub use indicator::{IndicatorConfig, IndicatorShape};
pub use lcars::{
    ContentDisplayType, ContentItemConfig, CornerStyle, DividerCapStyle, DividerConfig,
    ExtensionMode, HeaderAlign, HeaderConfig, HeaderPosition, HeaderShape, HeaderWidthMode,
    LcarsFrameConfig, SegmentConfig, SidebarPosition, SplitOrientation, StaticDisplayConfig,
};
pub use material::MaterialFrameConfig;
pub use industrial::IndustrialFrameConfig;
pub use retro_terminal::RetroTerminalFrameConfig;
pub use fighter_hud::FighterHudFrameConfig;
pub use synthwave::SynthwaveFrameConfig;
pub use art_deco::ArtDecoFrameConfig;
pub use art_nouveau::ArtNouveauFrameConfig;
pub use steampunk::SteampunkFrameConfig;
pub use speedometer::{
    BezelStyle, NeedleStyle, NeedleTailStyle, SpeedometerConfig, TickLabelConfig, ValueZone,
};
pub use themed_configs::{
    ArtDecoDisplayConfig, ArtNouveauDisplayConfig, CyberpunkDisplayConfig,
    FighterHudDisplayConfig, IndustrialDisplayConfig, LcarsDisplayConfig,
    MaterialDisplayConfig, RetroTerminalDisplayConfig, SteampunkDisplayConfig,
    SynthwaveDisplayConfig,
};
