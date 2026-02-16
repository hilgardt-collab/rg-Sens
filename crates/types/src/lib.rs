//! rg-sens-types: Shared data types for rg-Sens system monitor.
//!
//! This crate contains pure data types (configs, enums, field metadata, etc.)
//! that are shared across all rg-Sens crates. These types have no GTK or
//! Cairo dependencies, making them suitable as a foundation layer.

pub mod background;
pub mod color;
pub mod combo;
pub mod display_configs;
pub mod field;
pub mod panel;
pub mod source_configs;
pub mod text;
pub mod theme;
pub mod timer;

// Re-export commonly used types at the crate root for convenience
pub use background::{
    BackgroundConfig, BackgroundType, ImageDisplayMode, IndicatorBackgroundConfig,
    IndicatorBackgroundShape, PolygonConfig,
};
pub use color::{Color, ColorStop, LinearGradientConfig, RadialGradientConfig};
pub use field::{FieldMetadata, FieldPurpose, FieldType};
pub use panel::{
    DisplayerConfig, PanelAppearance, PanelBorderConfig, PanelData, PanelGeometry, SourceConfig,
};
pub use text::{
    CombineAlignment, CombineDirection, HorizontalPosition, TextBackgroundConfig,
    TextBackgroundType, TextDisplayerConfig, TextFillType, TextLineConfig, TextOverlayConfig,
    TextPosition, VerticalPosition,
};
pub use theme::{
    deserialize_color_or_source, deserialize_color_stop_or_source, deserialize_color_stops_vec,
    deserialize_font_or_source, ColorSource, ColorStopSource, ComboThemeConfig, FontOrString,
    FontSource, GradientSource, LinearGradientSourceConfig,
};
