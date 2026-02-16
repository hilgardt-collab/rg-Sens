//! LCARS (Library Computer Access/Retrieval System) display configuration types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::background::BackgroundConfig;
use crate::color::Color;
use crate::combo::{ComboFrameConfig, LayoutFrameConfig, ThemedFrameConfig};
use crate::field::{FieldMetadata, FieldPurpose, FieldType};
use crate::text::TextOverlayConfig;
use crate::theme::{
    deserialize_color_or_source, ColorSource, ComboThemeConfig, FontOrString, FontSource,
};

// Re-export sub-display types for convenience
pub use crate::display_configs::arc::ArcDisplayConfig;
pub use crate::display_configs::bar::BarDisplayConfig;
pub use crate::display_configs::core_bars::CoreBarsConfig;
pub use crate::display_configs::graph::{DataPoint, GraphDisplayConfig};
pub use crate::display_configs::speedometer::SpeedometerConfig;

// ============================================================================
// Enums
// ============================================================================

/// Sidebar position (left or right)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum SidebarPosition {
    #[default]
    #[serde(rename = "left")]
    Left,
    #[serde(rename = "right")]
    Right,
}

/// Extension mode (which extensions to show)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum ExtensionMode {
    #[default]
    #[serde(rename = "top")]
    Top,
    #[serde(rename = "bottom")]
    Bottom,
    #[serde(rename = "both")]
    Both,
    #[serde(rename = "none")]
    None,
}

/// Corner style for extensions
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum CornerStyle {
    #[default]
    #[serde(rename = "square")]
    Square,
    #[serde(rename = "round")]
    Round,
}

/// Header shape style
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum HeaderShape {
    #[default]
    #[serde(rename = "pill")]
    Pill,
    #[serde(rename = "square")]
    Square,
}

/// Header alignment (relative to sidebar position)
/// - Near: Next to the sidebar
/// - Center: In the middle of the extension
/// - Far: At the far end of the extension (away from sidebar)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum HeaderAlign {
    #[default]
    #[serde(rename = "left", alias = "near")]
    Left, // "Near" - next to sidebar
    #[serde(rename = "center")]
    Center,
    #[serde(rename = "right", alias = "far")]
    Right, // "Far" - far from sidebar
}

/// Header width mode
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum HeaderWidthMode {
    #[default]
    #[serde(rename = "full")]
    Full,
    #[serde(rename = "fit")]
    Fit,
}

/// Header position within extension
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum HeaderPosition {
    #[serde(rename = "top")]
    Top,
    #[serde(rename = "bottom")]
    Bottom,
    #[serde(rename = "none")]
    #[default]
    None,
}

/// Divider cap style
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum DividerCapStyle {
    #[default]
    #[serde(rename = "square")]
    Square,
    #[serde(rename = "round")]
    Round,
}

/// Split screen orientation
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum SplitOrientation {
    #[default]
    #[serde(rename = "vertical")]
    Vertical,
    #[serde(rename = "horizontal")]
    Horizontal,
}

/// Content item display type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum ContentDisplayType {
    #[default]
    #[serde(rename = "bar")]
    Bar,
    #[serde(rename = "text")]
    Text,
    #[serde(rename = "graph")]
    Graph,
    #[serde(rename = "level_bar")]
    LevelBar,
    #[serde(rename = "core_bars")]
    CoreBars,
    #[serde(rename = "static")]
    Static,
    #[serde(rename = "arc")]
    Arc,
    #[serde(rename = "speedometer")]
    Speedometer,
}

impl ContentDisplayType {
    /// Suggest a display type based on the available fields
    ///
    /// This provides smart defaults based on what kind of data the source provides:
    /// - Text-only sources (like clock) -> Text
    /// - Percentage/numerical sources -> Bar (default)
    pub fn suggest_for_fields(fields: &[FieldMetadata]) -> Self {
        if fields.is_empty() {
            return ContentDisplayType::Text;
        }

        // Count field types
        let has_percentage = fields.iter().any(|f| f.field_type == FieldType::Percentage);
        let has_numerical = fields.iter().any(|f| f.field_type == FieldType::Numerical);
        let all_text = fields.iter().all(|f| f.field_type == FieldType::Text);
        let has_value_purpose = fields.iter().any(|f| f.purpose == FieldPurpose::Value);

        // If all fields are text (like clock, date), suggest Text displayer
        if all_text {
            return ContentDisplayType::Text;
        }

        // If there are percentage fields with Value purpose, Bar is good
        if has_percentage && has_value_purpose {
            return ContentDisplayType::Bar;
        }

        // If there are numerical values but no percentages, Text might be better
        // (e.g., temperature values, frequencies)
        if has_numerical && !has_percentage {
            // Check if there's also text - could be a mixed source
            let has_text = fields.iter().any(|f| f.field_type == FieldType::Text);
            if has_text {
                return ContentDisplayType::Text;
            }
        }

        // Default to Bar for numerical/percentage data
        ContentDisplayType::Bar
    }
}

// ============================================================================
// SegmentConfig
// ============================================================================

/// Configuration for a sidebar segment
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(from = "RawSegmentConfig")]
pub struct SegmentConfig {
    #[serde(default = "default_segment_height_weight")]
    pub height_weight: f64,
    #[serde(
        default = "default_segment_color",
        deserialize_with = "deserialize_color_or_source"
    )]
    pub color: ColorSource,
    #[serde(default)]
    pub label: String,
    #[serde(default = "default_segment_font")]
    pub font: FontSource,
    #[serde(
        default = "default_segment_label_color",
        deserialize_with = "deserialize_color_or_source"
    )]
    pub label_color: ColorSource,
}

/// Helper struct for deserializing SegmentConfig with backward compatibility
#[derive(Deserialize)]
struct RawSegmentConfig {
    #[serde(default = "default_segment_height_weight")]
    height_weight: f64,
    #[serde(
        default = "default_segment_color",
        deserialize_with = "deserialize_color_or_source"
    )]
    color: ColorSource,
    #[serde(default)]
    label: String,
    // Font can be: FontSource object (new) or string (legacy family name)
    #[serde(default)]
    font: Option<FontOrString>,
    // Legacy format: font_size as separate field
    #[serde(default)]
    font_size: Option<f64>,
    #[serde(
        default = "default_segment_label_color",
        deserialize_with = "deserialize_color_or_source"
    )]
    label_color: ColorSource,
}

impl From<RawSegmentConfig> for SegmentConfig {
    fn from(raw: RawSegmentConfig) -> Self {
        let default_size = raw.font_size.unwrap_or(12.0);
        let font = raw
            .font
            .and_then(|f| f.into_font_source(default_size))
            .unwrap_or_else(default_segment_font);

        Self {
            height_weight: raw.height_weight,
            color: raw.color,
            label: raw.label,
            font,
            label_color: raw.label_color,
        }
    }
}

fn default_segment_height_weight() -> f64 {
    1.0
}

fn default_segment_color() -> ColorSource {
    ColorSource::custom(Color::new(0.78, 0.39, 0.39, 1.0)) // Reddish LCARS color
}

fn default_segment_font() -> FontSource {
    FontSource::Custom {
        family: "Sans".to_string(),
        size: 12.0,
    }
}

fn default_segment_label_color() -> ColorSource {
    ColorSource::custom(Color::new(0.0, 0.0, 0.0, 1.0)) // Black
}

impl Default for SegmentConfig {
    fn default() -> Self {
        Self {
            height_weight: default_segment_height_weight(),
            color: default_segment_color(),
            label: String::new(),
            font: default_segment_font(),
            label_color: default_segment_label_color(),
        }
    }
}

// ============================================================================
// HeaderConfig
// ============================================================================

/// Configuration for a header bar
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(from = "RawHeaderConfig")]
pub struct HeaderConfig {
    #[serde(default)]
    pub position: HeaderPosition,
    #[serde(default)]
    pub text: String,
    #[serde(default = "default_header_font")]
    pub font: FontSource,
    #[serde(default = "default_header_font_bold")]
    pub font_bold: bool,
    #[serde(
        default = "default_header_text_color",
        deserialize_with = "deserialize_color_or_source"
    )]
    pub text_color: ColorSource,
    #[serde(
        default = "default_header_bg_color",
        deserialize_with = "deserialize_color_or_source"
    )]
    pub bg_color: ColorSource,
    #[serde(default)]
    pub shape: HeaderShape,
    #[serde(default)]
    pub align: HeaderAlign,
    #[serde(default)]
    pub width_mode: HeaderWidthMode,
    #[serde(default = "default_header_padding")]
    pub padding: f64,
    /// Header height as percentage of sidebar extension height (0.0-1.0)
    #[serde(default = "default_header_height_percent")]
    pub height_percent: f64,
    /// Header width as percentage of sidebar extension width (0.0-1.0)
    #[serde(default = "default_header_width_percent")]
    pub width_percent: f64,
}

/// Helper struct for deserializing HeaderConfig with backward compatibility
#[derive(Deserialize)]
struct RawHeaderConfig {
    #[serde(default)]
    position: HeaderPosition,
    #[serde(default)]
    text: String,
    // Font can be: FontSource object (new) or string (legacy family name)
    #[serde(default)]
    font: Option<FontOrString>,
    // Legacy format: font_size as separate field
    #[serde(default)]
    font_size: Option<f64>,
    #[serde(default = "default_header_font_bold")]
    font_bold: bool,
    #[serde(
        default = "default_header_text_color",
        deserialize_with = "deserialize_color_or_source"
    )]
    text_color: ColorSource,
    #[serde(
        default = "default_header_bg_color",
        deserialize_with = "deserialize_color_or_source"
    )]
    bg_color: ColorSource,
    #[serde(default)]
    shape: HeaderShape,
    #[serde(default)]
    align: HeaderAlign,
    #[serde(default)]
    width_mode: HeaderWidthMode,
    #[serde(default = "default_header_padding")]
    padding: f64,
    #[serde(default = "default_header_height_percent")]
    height_percent: f64,
    #[serde(default = "default_header_width_percent")]
    width_percent: f64,
}

impl From<RawHeaderConfig> for HeaderConfig {
    fn from(raw: RawHeaderConfig) -> Self {
        let default_size = raw.font_size.unwrap_or(14.0);
        let font = raw
            .font
            .and_then(|f| f.into_font_source(default_size))
            .unwrap_or_else(default_header_font);

        Self {
            position: raw.position,
            text: raw.text,
            font,
            font_bold: raw.font_bold,
            text_color: raw.text_color,
            bg_color: raw.bg_color,
            shape: raw.shape,
            align: raw.align,
            width_mode: raw.width_mode,
            padding: raw.padding,
            height_percent: raw.height_percent,
            width_percent: raw.width_percent,
        }
    }
}

fn default_header_font() -> FontSource {
    FontSource::Custom {
        family: "Sans".to_string(),
        size: 14.0,
    }
}

fn default_header_font_bold() -> bool {
    true
}

fn default_header_text_color() -> ColorSource {
    ColorSource::custom(Color::new(0.0, 0.0, 0.0, 1.0)) // Black
}

fn default_header_bg_color() -> ColorSource {
    ColorSource::custom(Color::new(0.6, 0.4, 0.8, 1.0)) // Purple LCARS color
}

fn default_header_padding() -> f64 {
    10.0
}

fn default_header_height_percent() -> f64 {
    1.0 // 100% of sidebar extension height by default
}

fn default_header_width_percent() -> f64 {
    1.0 // 100% of sidebar extension width by default
}

impl Default for HeaderConfig {
    fn default() -> Self {
        Self {
            position: HeaderPosition::default(),
            text: String::new(),
            font: default_header_font(),
            font_bold: default_header_font_bold(),
            text_color: default_header_text_color(),
            bg_color: default_header_bg_color(),
            shape: HeaderShape::default(),
            align: HeaderAlign::default(),
            width_mode: HeaderWidthMode::default(),
            padding: default_header_padding(),
            height_percent: default_header_height_percent(),
            width_percent: default_header_width_percent(),
        }
    }
}

// ============================================================================
// DividerConfig
// ============================================================================

/// Configuration for split screen divider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DividerConfig {
    #[serde(default = "default_divider_width")]
    pub width: f64,
    #[serde(
        default = "default_divider_color",
        deserialize_with = "deserialize_color_or_source"
    )]
    pub color: ColorSource,
    #[serde(default)]
    pub cap_start: DividerCapStyle,
    #[serde(default)]
    pub cap_end: DividerCapStyle,
    #[serde(default)]
    pub spacing_before: f64,
    #[serde(default)]
    pub spacing_after: f64,
}

fn default_divider_width() -> f64 {
    10.0
}

fn default_divider_color() -> ColorSource {
    ColorSource::custom(Color::new(1.0, 0.6, 0.4, 1.0)) // Orange LCARS color
}

impl Default for DividerConfig {
    fn default() -> Self {
        Self {
            width: default_divider_width(),
            color: default_divider_color(),
            cap_start: DividerCapStyle::default(),
            cap_end: DividerCapStyle::default(),
            spacing_before: 0.0,
            spacing_after: 0.0,
        }
    }
}

// ============================================================================
// StaticDisplayConfig
// ============================================================================

/// Configuration for static display (background with optional text overlay)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StaticDisplayConfig {
    #[serde(default)]
    pub background: BackgroundConfig,
    #[serde(default)]
    pub text_overlay: TextOverlayConfig,
}

impl Default for StaticDisplayConfig {
    fn default() -> Self {
        Self {
            background: BackgroundConfig::default(),
            text_overlay: TextOverlayConfig {
                enabled: false, // Disabled by default for static
                text_config: Default::default(),
            },
        }
    }
}

// ============================================================================
// ContentItemConfig
// ============================================================================

/// Configuration for a content item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentItemConfig {
    #[serde(default)]
    pub display_as: ContentDisplayType,
    #[serde(default = "default_true")]
    pub auto_height: bool,
    #[serde(default = "default_item_height")]
    pub item_height: f64,
    #[serde(default)]
    pub bar_config: BarDisplayConfig,
    #[serde(default)]
    pub graph_config: GraphDisplayConfig,
    #[serde(default)]
    pub core_bars_config: CoreBarsConfig,
    #[serde(default)]
    pub static_config: StaticDisplayConfig,
    #[serde(default)]
    pub arc_config: ArcDisplayConfig,
    #[serde(default)]
    pub speedometer_config: SpeedometerConfig,
}

fn default_true() -> bool {
    true
}

fn default_item_height() -> f64 {
    60.0
}

impl Default for ContentItemConfig {
    fn default() -> Self {
        Self {
            display_as: ContentDisplayType::default(),
            auto_height: true,
            item_height: default_item_height(),
            bar_config: BarDisplayConfig::default(),
            graph_config: GraphDisplayConfig::default(),
            core_bars_config: CoreBarsConfig::default(),
            static_config: StaticDisplayConfig::default(),
            arc_config: ArcDisplayConfig::default(),
            speedometer_config: SpeedometerConfig::default(),
        }
    }
}

// ============================================================================
// LcarsFrameConfig
// ============================================================================

/// Main LCARS frame configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LcarsFrameConfig {
    // Sidebar
    #[serde(default = "default_sidebar_width")]
    pub sidebar_width: f64,
    #[serde(default)]
    pub sidebar_position: SidebarPosition,

    // Extensions
    #[serde(default)]
    pub extension_mode: ExtensionMode,
    #[serde(default = "default_top_bar_height")]
    pub top_bar_height: f64,
    #[serde(default = "default_bottom_bar_height")]
    pub bottom_bar_height: f64,
    #[serde(default)]
    pub extension_corner_style: CornerStyle,

    // Frame geometry
    #[serde(default = "default_corner_radius")]
    pub corner_radius: f64,
    /// Overall content padding (added to all sides)
    #[serde(default = "default_content_padding")]
    pub content_padding: f64,
    /// Additional top padding (added to overall)
    #[serde(default)]
    pub content_padding_top: f64,
    /// Additional left padding (added to overall)
    #[serde(default)]
    pub content_padding_left: f64,
    /// Additional right padding (added to overall)
    #[serde(default)]
    pub content_padding_right: f64,
    /// Additional bottom padding (added to overall)
    #[serde(default)]
    pub content_padding_bottom: f64,

    // Colors
    #[serde(default = "default_frame_color")]
    pub frame_color: Color,
    #[serde(default = "default_content_bg_color")]
    pub content_bg_color: Color,

    // Segments
    #[serde(default = "default_segment_count")]
    pub segment_count: u32,
    #[serde(default)]
    pub segments: Vec<SegmentConfig>,

    // Headers
    #[serde(default)]
    pub top_header: HeaderConfig,
    #[serde(default)]
    pub bottom_header: HeaderConfig,

    // Content - new group-based structure
    /// Number of groups (1-8)
    #[serde(default = "default_group_count")]
    pub group_count: u32,
    /// Number of items in each group
    #[serde(default = "default_group_item_counts")]
    pub group_item_counts: Vec<u32>,
    /// Size weight for each group (relative sizing like segment height_weight)
    #[serde(default = "default_group_size_weights")]
    pub group_size_weights: Vec<f64>,
    /// Legacy: primary_count (for backwards compatibility)
    #[serde(default, skip_serializing)]
    pub primary_count: u32,
    /// Legacy: secondary_count (for backwards compatibility)
    #[serde(default, skip_serializing)]
    pub secondary_count: u32,
    #[serde(default)]
    pub content_items: HashMap<String, ContentItemConfig>,

    // Layout orientation (how groups are arranged)
    /// Layout orientation - Vertical = groups side by side, Horizontal = groups stacked
    #[serde(default)]
    pub layout_orientation: SplitOrientation,

    /// Item orientation within each group - defaults to same as layout_orientation
    /// Vertical = items stacked top to bottom, Horizontal = items side by side
    #[serde(default)]
    pub group_item_orientations: Vec<SplitOrientation>,

    // Divider settings - dividers between groups
    #[serde(default)]
    pub divider_config: DividerConfig,

    // Spacing
    #[serde(default = "default_item_spacing")]
    pub item_spacing: f64,

    /// Sync segment heights with group heights (only works when layout is Horizontal and segment_count == group_count)
    #[serde(default)]
    pub sync_segments_to_groups: bool,

    /// Animation enabled
    #[serde(default = "default_animation_enabled")]
    pub animation_enabled: bool,

    /// Animation speed
    #[serde(default = "default_animation_speed")]
    pub animation_speed: f64,

    /// Theme configuration
    #[serde(default = "default_lcars_theme")]
    pub theme: ComboThemeConfig,

    /// Shadow field for group_item_counts as usize (for ComboFrameConfig trait)
    #[serde(skip)]
    group_item_counts_usize: Vec<usize>,
}

fn default_animation_enabled() -> bool {
    true
}

fn default_animation_speed() -> f64 {
    8.0
}

fn default_lcars_theme() -> ComboThemeConfig {
    ComboThemeConfig::default_for_lcars()
}

fn default_sidebar_width() -> f64 {
    150.0
}

fn default_top_bar_height() -> f64 {
    40.0
}

fn default_bottom_bar_height() -> f64 {
    40.0
}

fn default_corner_radius() -> f64 {
    60.0
}

fn default_content_padding() -> f64 {
    5.0
}

fn default_frame_color() -> Color {
    Color::new(1.0, 0.6, 0.4, 1.0) // Orange LCARS color
}

fn default_content_bg_color() -> Color {
    Color::new(0.0, 0.0, 0.0, 0.3)
}

fn default_segment_count() -> u32 {
    3
}

fn default_group_count() -> u32 {
    1
}

fn default_group_item_counts() -> Vec<u32> {
    vec![2] // Default: 1 group with 2 items
}

fn default_group_size_weights() -> Vec<f64> {
    vec![1.0] // Default: equal weight
}

fn default_item_spacing() -> f64 {
    5.0
}

impl Default for LcarsFrameConfig {
    fn default() -> Self {
        Self {
            sidebar_width: default_sidebar_width(),
            sidebar_position: SidebarPosition::default(),
            extension_mode: ExtensionMode::default(),
            top_bar_height: default_top_bar_height(),
            bottom_bar_height: default_bottom_bar_height(),
            extension_corner_style: CornerStyle::default(),
            corner_radius: default_corner_radius(),
            content_padding: default_content_padding(),
            content_padding_top: 0.0,
            content_padding_left: 0.0,
            content_padding_right: 0.0,
            content_padding_bottom: 0.0,
            frame_color: default_frame_color(),
            content_bg_color: default_content_bg_color(),
            segment_count: default_segment_count(),
            segments: vec![
                SegmentConfig {
                    color: ColorSource::custom(Color::new(1.0, 0.6, 0.4, 1.0)),
                    label: "SYS".to_string(),
                    ..Default::default()
                },
                SegmentConfig {
                    color: ColorSource::custom(Color::new(0.8, 0.6, 0.9, 1.0)),
                    label: "MON".to_string(),
                    ..Default::default()
                },
                SegmentConfig {
                    color: ColorSource::custom(Color::new(0.6, 0.8, 1.0, 1.0)),
                    label: "DATA".to_string(),
                    ..Default::default()
                },
            ],
            top_header: HeaderConfig {
                position: HeaderPosition::Top,
                ..Default::default()
            },
            bottom_header: HeaderConfig {
                position: HeaderPosition::Bottom,
                ..Default::default()
            },
            group_count: default_group_count(),
            group_item_counts: default_group_item_counts(),
            group_size_weights: default_group_size_weights(),
            primary_count: 0,
            secondary_count: 0,
            content_items: HashMap::new(),
            layout_orientation: SplitOrientation::default(),
            group_item_orientations: Vec::new(), // Empty = use layout_orientation for all
            divider_config: DividerConfig::default(),
            item_spacing: default_item_spacing(),
            sync_segments_to_groups: false,
            animation_enabled: default_animation_enabled(),
            animation_speed: default_animation_speed(),
            theme: default_lcars_theme(),
            group_item_counts_usize: vec![2],
        }
    }
}

// ============================================================================
// Trait implementations for LcarsFrameConfig
// ============================================================================

impl LayoutFrameConfig for LcarsFrameConfig {
    fn group_count(&self) -> usize {
        self.group_count as usize
    }

    fn group_size_weights(&self) -> &Vec<f64> {
        &self.group_size_weights
    }

    fn group_size_weights_mut(&mut self) -> &mut Vec<f64> {
        &mut self.group_size_weights
    }

    fn group_item_orientations(&self) -> &Vec<SplitOrientation> {
        &self.group_item_orientations
    }

    fn group_item_orientations_mut(&mut self) -> &mut Vec<SplitOrientation> {
        &mut self.group_item_orientations
    }

    fn split_orientation(&self) -> SplitOrientation {
        self.layout_orientation
    }
}

impl ThemedFrameConfig for LcarsFrameConfig {
    fn theme(&self) -> &ComboThemeConfig {
        &self.theme
    }

    fn theme_mut(&mut self) -> &mut ComboThemeConfig {
        &mut self.theme
    }

    fn content_items(&self) -> &HashMap<String, ContentItemConfig> {
        &self.content_items
    }

    fn content_items_mut(&mut self) -> &mut HashMap<String, ContentItemConfig> {
        &mut self.content_items
    }
}

impl ComboFrameConfig for LcarsFrameConfig {
    fn animation_enabled(&self) -> bool {
        self.animation_enabled
    }

    fn set_animation_enabled(&mut self, enabled: bool) {
        self.animation_enabled = enabled;
    }

    fn animation_speed(&self) -> f64 {
        self.animation_speed
    }

    fn set_animation_speed(&mut self, speed: f64) {
        self.animation_speed = speed;
    }

    fn group_item_counts(&self) -> &[usize] {
        &self.group_item_counts_usize
    }

    fn group_item_counts_mut(&mut self) -> &mut Vec<usize> {
        // Sync from u32 to usize before returning
        self.group_item_counts_usize =
            self.group_item_counts.iter().map(|&c| c as usize).collect();
        &mut self.group_item_counts_usize
    }
}

impl LcarsFrameConfig {
    /// Sync the usize shadow field from the u32 field
    pub fn sync_group_item_counts(&mut self) {
        self.group_item_counts_usize =
            self.group_item_counts.iter().map(|&c| c as usize).collect();
    }

    /// Migrate legacy primary/secondary config to groups format
    pub fn migrate_legacy(&mut self) {
        // Only migrate if group_item_counts is empty and we have legacy counts
        if self.group_item_counts.is_empty() && (self.primary_count > 0 || self.secondary_count > 0)
        {
            log::info!("LcarsFrameConfig: Migrating legacy primary/secondary to groups format");

            // Convert primary to group 1
            if self.primary_count > 0 {
                self.group_item_counts.push(self.primary_count);
            }
            // Convert secondary to group 2
            if self.secondary_count > 0 {
                self.group_item_counts.push(self.secondary_count);
            }

            self.group_count = self.group_item_counts.len() as u32;

            // Migrate content item keys: primary1 -> group1_1, secondary1 -> group2_1
            let mut new_items = HashMap::new();
            for (old_name, config) in &self.content_items {
                let new_name = if old_name.starts_with("primary") {
                    let num: String = old_name.chars().filter(|c| c.is_ascii_digit()).collect();
                    format!("group1_{}", num)
                } else if old_name.starts_with("secondary") {
                    let num: String = old_name.chars().filter(|c| c.is_ascii_digit()).collect();
                    format!("group2_{}", num)
                } else {
                    old_name.clone()
                };
                new_items.insert(new_name, config.clone());
            }
            self.content_items = new_items;

            // Clear legacy fields
            self.primary_count = 0;
            self.secondary_count = 0;
        }

        // Ensure at least one group exists
        if self.group_item_counts.is_empty() {
            self.group_item_counts.push(2);
            self.group_count = 1;
        }

        // Ensure group_count matches the vector length
        self.group_count = self.group_item_counts.len() as u32;
    }

    /// Get total number of items across all groups
    pub fn total_items(&self) -> usize {
        self.group_item_counts.iter().map(|&c| c as usize).sum()
    }

    /// Get total number of items across all groups (u32 version for backwards compatibility)
    pub fn total_item_count(&self) -> u32 {
        self.group_item_counts.iter().sum()
    }

    /// Get slot names for all groups
    pub fn get_slot_names(&self) -> Vec<String> {
        let mut names = Vec::new();
        for (group_idx, &item_count) in self.group_item_counts.iter().enumerate() {
            let group_num = group_idx + 1;
            for item_idx in 1..=item_count {
                names.push(format!("group{}_{}", group_num, item_idx));
            }
        }
        names
    }
}
