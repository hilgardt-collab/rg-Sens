//! LCARS (Library Computer Access/Retrieval System) display rendering
//!
//! This module provides rendering functions for the LCARS-style interface,
//! featuring a sidebar with segments, optional top/bottom extensions,
//! and a configurable content area for displaying various data types.

use gtk4::cairo;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::VecDeque;
use std::f64::consts::PI;

use crate::ui::background::{BackgroundConfig, Color, render_background_with_theme};
use crate::ui::bar_display::{BarDisplayConfig, render_bar};
use crate::ui::theme::{ColorSource, ComboThemeConfig, FontOrString, FontSource, deserialize_color_or_source};
use crate::ui::core_bars_display::{CoreBarsConfig, render_core_bars};
use crate::ui::graph_display::{GraphDisplayConfig, DataPoint, render_graph};
use crate::ui::arc_display::ArcDisplayConfig;
use crate::ui::speedometer_display::SpeedometerConfig;

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
    Left,  // "Near" - next to sidebar
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
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[derive(Default)]
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
    /// - Text-only sources (like clock) → Text
    /// - Percentage/numerical sources → Bar (default)
    pub fn suggest_for_fields(fields: &[crate::core::FieldMetadata]) -> Self {
        use crate::core::{FieldType, FieldPurpose};

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

/// Configuration for a sidebar segment
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(from = "RawSegmentConfig")]
pub struct SegmentConfig {
    #[serde(default = "default_segment_height_weight")]
    pub height_weight: f64,
    #[serde(default = "default_segment_color", deserialize_with = "deserialize_color_or_source")]
    pub color: ColorSource,
    #[serde(default)]
    pub label: String,
    #[serde(default = "default_segment_font")]
    pub font: FontSource,
    #[serde(default = "default_segment_label_color", deserialize_with = "deserialize_color_or_source")]
    pub label_color: ColorSource,
}

/// Helper struct for deserializing SegmentConfig with backward compatibility
#[derive(Deserialize)]
struct RawSegmentConfig {
    #[serde(default = "default_segment_height_weight")]
    height_weight: f64,
    #[serde(default = "default_segment_color", deserialize_with = "deserialize_color_or_source")]
    color: ColorSource,
    #[serde(default)]
    label: String,
    // Font can be: FontSource object (new) or string (legacy family name)
    #[serde(default)]
    font: Option<FontOrString>,
    // Legacy format: font_size as separate field
    #[serde(default)]
    font_size: Option<f64>,
    #[serde(default = "default_segment_label_color", deserialize_with = "deserialize_color_or_source")]
    label_color: ColorSource,
}

impl From<RawSegmentConfig> for SegmentConfig {
    fn from(raw: RawSegmentConfig) -> Self {
        let default_size = raw.font_size.unwrap_or(12.0);
        let font = raw.font
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
    FontSource::Custom { family: "Sans".to_string(), size: 12.0 }
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
    #[serde(default = "default_header_text_color", deserialize_with = "deserialize_color_or_source")]
    pub text_color: ColorSource,
    #[serde(default = "default_header_bg_color", deserialize_with = "deserialize_color_or_source")]
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
    #[serde(default = "default_header_text_color", deserialize_with = "deserialize_color_or_source")]
    text_color: ColorSource,
    #[serde(default = "default_header_bg_color", deserialize_with = "deserialize_color_or_source")]
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
        let font = raw.font
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
    FontSource::Custom { family: "Sans".to_string(), size: 14.0 }
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

/// Configuration for split screen divider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DividerConfig {
    #[serde(default = "default_divider_width")]
    pub width: f64,
    #[serde(default = "default_divider_color", deserialize_with = "deserialize_color_or_source")]
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

/// Configuration for static display (background only, no dynamic data)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct StaticDisplayConfig {
    #[serde(default)]
    pub background: BackgroundConfig,
}

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

    // Divider settings - dividers between groups
    #[serde(default)]
    pub divider_config: DividerConfig,

    // Spacing
    #[serde(default = "default_item_spacing")]
    pub item_spacing: f64,

    /// Sync segment heights with group heights (only works when layout is Horizontal and segment_count == group_count)
    #[serde(default)]
    pub sync_segments_to_groups: bool,

    /// Theme configuration
    #[serde(default = "default_lcars_theme")]
    pub theme: crate::ui::theme::ComboThemeConfig,
}

fn default_lcars_theme() -> crate::ui::theme::ComboThemeConfig {
    crate::ui::theme::ComboThemeConfig::default_for_lcars()
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
            divider_config: DividerConfig::default(),
            item_spacing: default_item_spacing(),
            sync_segments_to_groups: false,
            theme: default_lcars_theme(),
        }
    }
}

impl LcarsFrameConfig {
    /// Migrate legacy primary/secondary config to groups format
    pub fn migrate_legacy(&mut self) {
        // Only migrate if group_item_counts is empty and we have legacy counts
        if self.group_item_counts.is_empty() && (self.primary_count > 0 || self.secondary_count > 0) {
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

/// Data for rendering content items
#[derive(Debug, Clone, Default)]
pub struct ContentItemData {
    pub caption: String,
    pub value: String,
    pub unit: String,
    pub numerical_value: f64,
    pub min_value: f64,
    pub max_value: f64,
}

impl ContentItemData {
    pub fn percent(&self) -> f64 {
        if self.max_value <= self.min_value {
            0.0
        } else {
            ((self.numerical_value - self.min_value) / (self.max_value - self.min_value)).clamp(0.0, 1.0)
        }
    }
}

/// Render the LCARS frame and sidebar
///
/// New approach: Draw segments WITH their extensions as complete units.
/// - First segment includes top extension and outer top corner curves
/// - Middle segments are simple rectangles
/// - Last segment includes bottom extension and outer bottom corner curves
///
/// This eliminates the need for overlay fixes.
pub fn render_lcars_frame(
    cr: &cairo::Context,
    config: &LcarsFrameConfig,
    width: f64,
    height: f64,
) -> Result<(), cairo::Error> {
    let sidebar_w = config.sidebar_width;
    let radius = config.corner_radius;
    let top_bar_h = config.top_bar_height;
    let bottom_ext_h = config.bottom_bar_height;

    let has_top_ext = matches!(config.extension_mode, ExtensionMode::Top | ExtensionMode::Both);
    let has_bottom_ext = matches!(config.extension_mode, ExtensionMode::Bottom | ExtensionMode::Both);

    let r_top = top_bar_h / 2.0;
    let r_bot = bottom_ext_h / 2.0;
    let ext_style = config.extension_corner_style;

    let is_right = config.sidebar_position == SidebarPosition::Right;

    // Calculate segment heights
    let segment_count = config.segment_count as usize;
    if segment_count == 0 {
        return Ok(());
    }

    // Check if we should sync segment heights with group weights
    let use_group_weights = config.sync_segments_to_groups
        && config.layout_orientation == SplitOrientation::Horizontal
        && segment_count == config.group_count as usize;

    let total_weight: f64 = if use_group_weights {
        (0..segment_count)
            .map(|i| config.group_size_weights.get(i).copied().unwrap_or(1.0))
            .sum()
    } else {
        (0..segment_count)
            .map(|i| config.segments.get(i).map(|s| s.height_weight).unwrap_or(1.0))
            .sum()
    };
    let total_weight = if total_weight <= 0.0 { 1.0 } else { total_weight };

    // Available height for segments (between extensions)
    let segments_start_y = if has_top_ext { top_bar_h } else { 0.0 };
    let segments_end_y = if has_bottom_ext { height - bottom_ext_h } else { height };
    let available_h = segments_end_y - segments_start_y;

    // Collect label info during segment drawing, then draw labels without transform
    struct LabelInfo {
        text: String,
        font: String,
        font_size: f64,
        color: Color,
        y: f64,
        seg_h: f64,
        is_last: bool,
    }
    let mut labels: Vec<LabelInfo> = Vec::new();

    cr.save()?;

    // Apply transform for right sidebar
    if is_right {
        cr.translate(width, 0.0);
        cr.scale(-1.0, 1.0);
    }

    let mut current_y = segments_start_y;

    for i in 0..segment_count {
        let segment = config.segments.get(i).cloned().unwrap_or_default();
        let weight = if use_group_weights {
            config.group_size_weights.get(i).copied().unwrap_or(1.0)
        } else {
            segment.height_weight
        };
        let seg_h = (weight / total_weight) * available_h;
        let is_first = i == 0;
        let is_last = i == segment_count - 1;

        segment.color.resolve(&config.theme).apply_to_cairo(cr);
        cr.new_path();

        if is_first && has_top_ext {
            // First segment WITH top extension - includes outer top-left corner curve
            // Start at outer top-left corner
            cr.move_to(0.0, radius);
            cr.arc(radius, radius, radius, PI, 1.5 * PI);
            // Top extension
            cr.line_to(width - r_top, 0.0);
            if ext_style == CornerStyle::Round && r_top > 0.0 {
                cr.arc(width - r_top, r_top, r_top, 1.5 * PI, 0.5 * PI);
            } else {
                cr.line_to(width, 0.0);
                cr.line_to(width, top_bar_h);
            }
            // Inner corner back to sidebar
            cr.line_to(sidebar_w + radius, top_bar_h);
            cr.arc_negative(sidebar_w + radius, top_bar_h + radius, radius, 1.5 * PI, PI);
            // Down to segment bottom
            cr.line_to(sidebar_w, current_y + seg_h);
            // Back to left edge
            cr.line_to(0.0, current_y + seg_h);
            cr.close_path();
        } else if is_last && has_bottom_ext {
            // Last segment WITH bottom extension - includes outer bottom-left corner curve
            // Start at top-left of segment
            cr.move_to(0.0, current_y);
            // Right to sidebar edge
            cr.line_to(sidebar_w, current_y);
            // Down to inner corner
            cr.line_to(sidebar_w, segments_end_y - radius);
            // Inner corner curve
            cr.arc_negative(sidebar_w + radius, segments_end_y - radius, radius, PI, 0.5 * PI);
            // Bottom extension
            cr.line_to(width - r_bot, segments_end_y);
            if ext_style == CornerStyle::Round && r_bot > 0.0 {
                cr.arc(width - r_bot, height - r_bot, r_bot, 1.5 * PI, 0.5 * PI);
            } else {
                cr.line_to(width, segments_end_y);
                cr.line_to(width, height);
            }
            // Outer bottom-left corner curve
            cr.line_to(radius, height);
            cr.arc(radius, height - radius, radius, 0.5 * PI, PI);
            // Back up to start
            cr.close_path();
        } else if is_first && !has_top_ext {
            // First segment without top extension - corner style follows extension_corner_style
            if ext_style == CornerStyle::Round {
                cr.move_to(0.0, radius);
                cr.arc(radius, radius, radius, PI, 1.5 * PI);
                cr.line_to(sidebar_w, 0.0);
            } else {
                cr.move_to(0.0, 0.0);
                cr.line_to(sidebar_w, 0.0);
            }
            cr.line_to(sidebar_w, current_y + seg_h);
            cr.line_to(0.0, current_y + seg_h);
            cr.close_path();
        } else if is_last && !has_bottom_ext {
            // Last segment without bottom extension - corner style follows extension_corner_style
            cr.move_to(0.0, current_y);
            cr.line_to(sidebar_w, current_y);
            cr.line_to(sidebar_w, height);
            if ext_style == CornerStyle::Round {
                cr.line_to(radius, height);
                cr.arc(radius, height - radius, radius, 0.5 * PI, PI);
            } else {
                cr.line_to(0.0, height);
            }
            cr.close_path();
        } else {
            // Middle segment - simple rectangle
            cr.rectangle(0.0, current_y, sidebar_w, seg_h);
        }

        cr.fill()?;

        // Draw separator line (except for last segment)
        if i < segment_count - 1 {
            cr.set_source_rgba(0.0, 0.0, 0.0, 1.0);
            cr.set_line_width(2.0);
            cr.move_to(0.0, current_y + seg_h);
            cr.line_to(sidebar_w, current_y + seg_h);
            cr.stroke()?;
        }

        // Collect label info to draw after restoring transform
        if !segment.label.is_empty() {
            let (font_family, font_size) = segment.font.resolve(&config.theme);
            labels.push(LabelInfo {
                text: segment.label.to_uppercase(),
                font: font_family,
                font_size,
                color: segment.label_color.resolve(&config.theme),
                y: current_y,
                seg_h,
                is_last,
            });
        }

        current_y += seg_h;
    }

    cr.restore()?;

    // Draw segment labels without transform (so text isn't mirrored)
    for label_info in &labels {
        cr.select_font_face(&label_info.font, cairo::FontSlant::Normal, cairo::FontWeight::Normal);
        cr.set_font_size(label_info.font_size);

        if let Ok(text_extents) = cr.text_extents(&label_info.text) {
            label_info.color.apply_to_cairo(cr);

            // Calculate x position based on sidebar position
            let label_x = if is_right {
                // Sidebar on right: text aligned to left of sidebar
                width - sidebar_w + 5.0
            } else {
                // Sidebar on left: text aligned to right of sidebar
                sidebar_w - text_extents.width() - 5.0
            };

            // Last segment: label at top; others: label at bottom
            let label_y = if label_info.is_last {
                label_info.y + label_info.font_size + 5.0
            } else {
                label_info.y + label_info.seg_h - 5.0
            };

            cr.move_to(label_x, label_y);
            let _ = cr.show_text(&label_info.text);
        }
    }

    // Draw headers (need to handle mirroring separately for text)
    if config.top_header.position == HeaderPosition::Top && has_top_ext {
        render_header_bar(cr, config, &config.top_header, width, height, true)?;
    }
    if config.bottom_header.position == HeaderPosition::Bottom && has_bottom_ext {
        render_header_bar(cr, config, &config.bottom_header, width, height, false)?;
    }

    Ok(())
}

/// Render an extension bar with the given color
#[allow(dead_code)]
fn render_extension(
    cr: &cairo::Context,
    config: &LcarsFrameConfig,
    width: f64,
    height: f64,
    is_top: bool,
    color: &Color,
) -> Result<(), cairo::Error> {
    let sidebar_w = config.sidebar_width;
    let radius = config.corner_radius;
    let bar_h = if is_top { config.top_bar_height } else { config.bottom_bar_height };
    let r_end = bar_h / 2.0;
    let ext_style = config.extension_corner_style;
    let is_right = config.sidebar_position == SidebarPosition::Right;

    cr.save()?;

    // If sidebar is on the right, flip the coordinate system
    if is_right {
        cr.translate(width, 0.0);
        cr.scale(-1.0, 1.0);
    }

    cr.new_path();

    if is_top {
        // Top extension: from sidebar edge to right side at top
        // Include the corner curve area to fill with segment color
        cr.move_to(sidebar_w, 0.0);
        cr.line_to(width - r_end, 0.0);
        if ext_style == CornerStyle::Round && r_end > 0.0 {
            cr.arc(width - r_end, r_end, r_end, 1.5 * PI, 0.5 * PI);
        } else {
            cr.line_to(width, 0.0);
            cr.line_to(width, bar_h);
        }
        cr.line_to(sidebar_w + radius, bar_h);
        cr.arc_negative(sidebar_w + radius, bar_h + radius, radius, 1.5 * PI, PI);
        cr.line_to(sidebar_w, 0.0);
    } else {
        // Bottom extension: from sidebar edge to right side at bottom
        // Include the corner curve area to fill with segment color
        let y_start = height - bar_h;
        // Start from sidebar at the curve start point
        cr.move_to(sidebar_w, y_start - radius);
        // Draw the curved corner
        cr.arc_negative(sidebar_w + radius, y_start - radius, radius, PI, 0.5 * PI);
        cr.line_to(width - r_end, y_start);
        if ext_style == CornerStyle::Round && r_end > 0.0 {
            cr.arc(width - r_end, height - r_end, r_end, 1.5 * PI, 0.5 * PI);
        } else {
            cr.line_to(width, y_start);
            cr.line_to(width, height);
        }
        cr.line_to(sidebar_w, height);
        cr.close_path();
    }

    color.apply_to_cairo(cr);
    cr.fill()?;

    cr.restore()?;

    // For bottom extension, also draw the sidebar's bottom corner with the same color
    // This is the curved corner at the bottom-left of the sidebar that connects to the bottom edge
    // We use the same transform approach as the frame to ensure exact alignment
    if !is_top {
        cr.save()?;

        // Apply the same transform as the frame uses
        if is_right {
            cr.translate(width, 0.0);
            cr.scale(-1.0, 1.0);
        }

        // Draw the bottom-left corner (will be bottom-right after transform if is_right)
        // This matches exactly what the frame draws at lines 683-684
        cr.new_path();
        cr.move_to(0.0, height - radius);
        cr.line_to(0.0, height);
        cr.line_to(radius, height);
        cr.arc(radius, height - radius, radius, 0.5 * PI, PI);
        cr.close_path();

        color.apply_to_cairo(cr);
        cr.fill()?;

        cr.restore()?;
    }

    Ok(())
}

/// Render a header bar within an extension
fn render_header_bar(
    cr: &cairo::Context,
    frame_config: &LcarsFrameConfig,
    header_config: &HeaderConfig,
    width: f64,
    height: f64,
    is_top: bool,
) -> Result<(), cairo::Error> {
    let bar_h = if is_top {
        frame_config.top_bar_height
    } else {
        frame_config.bottom_bar_height
    };
    let sidebar_w = frame_config.sidebar_width;
    let radius = frame_config.corner_radius;
    let padding = header_config.padding;

    // Apply height percentage to scale the header height
    let max_bar_content_h = bar_h - (2.0 * padding);
    let bar_content_h = max_bar_content_h * header_config.height_percent.clamp(0.0, 1.0);
    if bar_content_h <= 0.0 {
        return Ok(());
    }

    let text = header_config.text.to_uppercase();

    cr.save()?;

    // Calculate text dimensions
    let (font_family, font_size) = header_config.font.resolve(&frame_config.theme);
    let font_weight = if header_config.font_bold {
        cairo::FontWeight::Bold
    } else {
        cairo::FontWeight::Normal
    };
    cr.select_font_face(&font_family, cairo::FontSlant::Normal, font_weight);
    cr.set_font_size(font_size);
    let text_extents = cr.text_extents(&text)?;

    // Calculate available space for width calculations
    let max_available_space = match frame_config.sidebar_position {
        SidebarPosition::Left => width - (sidebar_w + radius + padding) - padding,
        SidebarPosition::Right => width - (sidebar_w + radius + padding) - padding,
    };

    // Calculate bar width, applying width percentage for Full mode
    let bar_w = match header_config.width_mode {
        HeaderWidthMode::Fit => {
            let extra = if header_config.shape == HeaderShape::Pill {
                bar_content_h
            } else {
                0.0
            };
            text_extents.width() + (padding * 2.0) + extra
        }
        HeaderWidthMode::Full => {
            // Apply width percentage to scale the header width
            max_available_space * header_config.width_percent.clamp(0.0, 1.0)
        }
    };

    if bar_w <= 0.0 {
        cr.restore()?;
        return Ok(());
    }

    // Calculate bar position (centered vertically within the available height)
    let height_offset = (max_bar_content_h - bar_content_h) / 2.0;
    let bar_y = if is_top {
        padding + height_offset
    } else {
        height - bar_h + padding + height_offset
    };

    let content_start_x = match frame_config.sidebar_position {
        SidebarPosition::Left => sidebar_w + radius + padding,
        SidebarPosition::Right => padding,
    };
    let available_space = max_available_space;

    // Calculate bar_x position
    // Left = Near (next to sidebar), Right = Far (away from sidebar)
    // So when sidebar is on Right, Near is on the right side of the content area
    let bar_x = match (header_config.align, frame_config.sidebar_position) {
        // Near sidebar: put header next to the sidebar
        (HeaderAlign::Left, SidebarPosition::Left) => content_start_x,
        (HeaderAlign::Left, SidebarPosition::Right) => content_start_x + available_space - bar_w,
        // Center: always in the middle
        (HeaderAlign::Center, _) => content_start_x + (available_space - bar_w) / 2.0,
        // Far from sidebar: put header at the far end
        (HeaderAlign::Right, SidebarPosition::Left) => content_start_x + available_space - bar_w,
        (HeaderAlign::Right, SidebarPosition::Right) => content_start_x,
    };

    // Draw header background
    let bar_radius = bar_content_h / 2.0;
    cr.new_path();

    match header_config.shape {
        HeaderShape::Square => {
            cr.rectangle(bar_x, bar_y, bar_w, bar_content_h);
        }
        HeaderShape::Pill => {
            cr.arc(bar_x + bar_radius, bar_y + bar_radius, bar_radius, 0.5 * PI, 1.5 * PI);
            cr.arc(bar_x + bar_w - bar_radius, bar_y + bar_radius, bar_radius, 1.5 * PI, 0.5 * PI);
            cr.close_path();
        }
    }

    header_config.bg_color.resolve(&frame_config.theme).apply_to_cairo(cr);
    cr.fill()?;

    // Draw header text
    header_config.text_color.resolve(&frame_config.theme).apply_to_cairo(cr);
    let text_x = bar_x + (bar_w - text_extents.width()) / 2.0;
    let text_y = bar_y + (bar_content_h + font_size * 0.7) / 2.0;
    cr.move_to(text_x, text_y);
    cr.show_text(&text)?;

    cr.restore()?;
    Ok(())
}

/// Render sidebar segments
#[allow(dead_code)]
fn render_sidebar_segments(
    cr: &cairo::Context,
    config: &LcarsFrameConfig,
    width: f64,
    height: f64,
) -> Result<(), cairo::Error> {
    if config.segment_count == 0 {
        return Ok(());
    }

    let sidebar_w = config.sidebar_width;
    let has_top_ext = matches!(config.extension_mode, ExtensionMode::Top | ExtensionMode::Both);
    let has_bottom_ext = matches!(config.extension_mode, ExtensionMode::Bottom | ExtensionMode::Both);

    let top_bar_h = if has_top_ext { config.top_bar_height } else { 0.0 };
    let bottom_ext_h = if has_bottom_ext { config.bottom_bar_height } else { 0.0 };
    // When there's a bottom extension, subtract the corner radius from sidebar segments
    // to leave room for the curved corner that connects to the bottom extension
    let bottom_corner_offset = if has_bottom_ext { config.corner_radius } else { 0.0 };
    let available_h = height - top_bar_h - bottom_ext_h - bottom_corner_offset;

    // Check if we should sync segment heights with group weights
    let segment_count = config.segment_count as usize;
    let use_group_weights = config.sync_segments_to_groups
        && config.layout_orientation == SplitOrientation::Horizontal
        && segment_count == config.group_count as usize;

    // Calculate total weight
    let total_weight: f64 = if use_group_weights {
        (0..segment_count)
            .map(|i| config.group_size_weights.get(i).copied().unwrap_or(1.0))
            .sum()
    } else {
        (0..segment_count)
            .map(|i| {
                config.segments.get(i)
                    .map(|s| s.height_weight)
                    .unwrap_or(1.0)
            })
            .sum()
    };

    let total_weight = if total_weight <= 0.0 { 1.0 } else { total_weight };

    let is_right = config.sidebar_position == SidebarPosition::Right;
    let sidebar_x = if is_right { width - sidebar_w } else { 0.0 };

    let mut current_y = top_bar_h;

    for i in 0..segment_count {
        let segment = config.segments.get(i).cloned().unwrap_or_default();
        let weight = if use_group_weights {
            config.group_size_weights.get(i).copied().unwrap_or(1.0)
        } else {
            segment.height_weight
        };
        let seg_h = (weight / total_weight) * available_h;

        cr.save()?;

        // Draw segment background
        segment.color.resolve(&config.theme).apply_to_cairo(cr);
        cr.rectangle(sidebar_x, current_y, sidebar_w, seg_h);
        cr.fill()?;

        // Draw separator line (except for last segment)
        if i < config.segment_count as usize - 1 {
            cr.set_source_rgba(0.0, 0.0, 0.0, 1.0);
            cr.set_line_width(2.0);
            cr.move_to(sidebar_x, current_y + seg_h);
            cr.line_to(sidebar_x + sidebar_w, current_y + seg_h);
            cr.stroke()?;
        }

        // Draw segment label
        if !segment.label.is_empty() {
            let label_text = segment.label.to_uppercase();
            let (font_family, font_size) = segment.font.resolve(&config.theme);
            cr.select_font_face(&font_family, cairo::FontSlant::Normal, cairo::FontWeight::Normal);
            cr.set_font_size(font_size);

            let text_extents = cr.text_extents(&label_text)?;
            segment.label_color.resolve(&config.theme).apply_to_cairo(cr);

            // Position label at bottom-right of segment
            let label_x = sidebar_x + sidebar_w - text_extents.width() - 5.0;
            let label_y = current_y + seg_h - 5.0;
            cr.move_to(label_x, label_y);
            cr.show_text(&label_text)?;
        }

        cr.restore()?;
        current_y += seg_h;
    }

    // Fill the gap between segments and bottom extension with last segment's color
    // This covers the entire rectangular part of the sidebar from where segments end
    // all the way down to the bottom extension bar. This area is initially filled with
    // frame_color and needs to be overdrawn with the last segment's color.
    if has_bottom_ext && config.segment_count > 0 {
        let last_idx = (config.segment_count as usize).saturating_sub(1);
        let last_segment_color = config.segments.get(last_idx)
            .map(|s| s.color.resolve(&config.theme))
            .unwrap_or_else(|| default_segment_color().resolve(&config.theme));

        cr.save()?;
        last_segment_color.apply_to_cairo(cr);
        // Fill from where segments end (current_y) down to the bottom extension
        let gap_h = height - bottom_ext_h - current_y;
        if gap_h > 0.0 {
            cr.rectangle(sidebar_x, current_y, sidebar_w, gap_h);
            cr.fill()?;
        }
        cr.restore()?;

        // Also draw the curved bottom corner with the last segment color
        // This is the outer corner at the bottom-left (or bottom-right when sidebar is on right)
        // Use the same transform approach as the frame to ensure exact alignment
        let radius = config.corner_radius;
        cr.save()?;

        // Apply the same transform as the frame uses
        if is_right {
            cr.translate(width, 0.0);
            cr.scale(-1.0, 1.0);
        }

        last_segment_color.apply_to_cairo(cr);

        // Draw the bottom-left corner (will be bottom-right after transform if is_right)
        // This matches exactly what the frame draws
        cr.new_path();
        cr.move_to(0.0, height - radius);
        cr.line_to(0.0, height);
        cr.line_to(radius, height);
        cr.arc(radius, height - radius, radius, 0.5 * PI, PI);
        cr.close_path();
        cr.fill()?;
        cr.restore()?;
    }

    Ok(())
}

/// Get the content area bounds
pub fn get_content_bounds(
    config: &LcarsFrameConfig,
    width: f64,
    height: f64,
) -> (f64, f64, f64, f64) {
    let has_top_ext = matches!(config.extension_mode, ExtensionMode::Top | ExtensionMode::Both);
    let has_bottom_ext = matches!(config.extension_mode, ExtensionMode::Bottom | ExtensionMode::Both);

    let top_bar_h = if has_top_ext { config.top_bar_height } else { 0.0 };
    let bottom_ext_h = if has_bottom_ext { config.bottom_bar_height } else { 0.0 };

    // Calculate effective padding for each side (overall + individual)
    let padding_top = config.content_padding + config.content_padding_top;
    let padding_bottom = config.content_padding + config.content_padding_bottom;
    let padding_left = config.content_padding + config.content_padding_left;
    let padding_right = config.content_padding + config.content_padding_right;

    let content_x = match config.sidebar_position {
        SidebarPosition::Left => config.sidebar_width + padding_left,
        SidebarPosition::Right => padding_left,
    };
    let content_y = top_bar_h + padding_top;
    let content_w = match config.sidebar_position {
        SidebarPosition::Left => width - config.sidebar_width - padding_left - padding_right,
        SidebarPosition::Right => width - config.sidebar_width - padding_left - padding_right,
    };
    let content_h = height - top_bar_h - bottom_ext_h - padding_top - padding_bottom;

    (content_x, content_y, content_w, content_h)
}

/// Render the content area background
pub fn render_content_background(
    cr: &cairo::Context,
    config: &LcarsFrameConfig,
    width: f64,
    height: f64,
) -> Result<(), cairo::Error> {
    let (content_x, content_y, content_w, content_h) = get_content_bounds(config, width, height);

    if content_w <= 0.0 || content_h <= 0.0 {
        return Ok(());
    }

    cr.save()?;
    cr.rectangle(content_x, content_y, content_w, content_h);
    cr.clip();

    config.content_bg_color.apply_to_cairo(cr);
    cr.paint()?;

    cr.restore()?;
    Ok(())
}

/// Render a split screen divider
pub fn render_divider(
    cr: &cairo::Context,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    divider_config: &DividerConfig,
    orientation: SplitOrientation,
    theme: &ComboThemeConfig,
) -> Result<(), cairo::Error> {
    let radius = w.min(h) / 2.0;

    cr.save()?;
    cr.new_path();

    match orientation {
        SplitOrientation::Horizontal => {
            // Horizontal divider (width is full, height is divider width)
            match divider_config.cap_start {
                DividerCapStyle::Round => {
                    cr.arc(x + radius, y + radius, radius, 0.5 * PI, 1.5 * PI);
                }
                DividerCapStyle::Square => {
                    cr.move_to(x, y + h);
                    cr.line_to(x, y);
                }
            }

            let end_x = x + w - if divider_config.cap_end == DividerCapStyle::Round { radius } else { 0.0 };
            cr.line_to(end_x, y);

            match divider_config.cap_end {
                DividerCapStyle::Round => {
                    cr.arc(x + w - radius, y + radius, radius, 1.5 * PI, 0.5 * PI);
                }
                DividerCapStyle::Square => {
                    cr.line_to(x + w, y + h);
                }
            }

            let start_x = x + if divider_config.cap_start == DividerCapStyle::Round { radius } else { 0.0 };
            cr.line_to(start_x, y + h);
        }
        SplitOrientation::Vertical => {
            // Vertical divider (height is full, width is divider width)
            match divider_config.cap_start {
                DividerCapStyle::Round => {
                    cr.arc(x + radius, y + radius, radius, PI, 0.0);
                }
                DividerCapStyle::Square => {
                    cr.move_to(x, y);
                    cr.line_to(x + w, y);
                }
            }

            let end_y = y + h - if divider_config.cap_end == DividerCapStyle::Round { radius } else { 0.0 };
            cr.line_to(x + w, end_y);

            match divider_config.cap_end {
                DividerCapStyle::Round => {
                    cr.arc(x + radius, y + h - radius, radius, 0.0, PI);
                }
                DividerCapStyle::Square => {
                    cr.line_to(x, y + h);
                }
            }

            let start_y = y + if divider_config.cap_start == DividerCapStyle::Round { radius } else { 0.0 };
            cr.line_to(x, start_y);
        }
    }

    cr.close_path();
    divider_config.color.resolve(theme).apply_to_cairo(cr);
    cr.fill()?;

    cr.restore()?;
    Ok(())
}

/// Render a content bar with label and value using the reusable bar_display module
pub fn render_content_bar(
    cr: &cairo::Context,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    bar_config: &BarDisplayConfig,
    theme: &ComboThemeConfig,
    data: &ContentItemData,
    animated_percent: f64,
    slot_values: Option<&HashMap<String, serde_json::Value>>,
) -> Result<(), cairo::Error> {
    cr.save()?;
    cr.translate(x, y);

    // Build values HashMap for render_bar's text overlay
    let mut values = HashMap::new();

    // Add basic ContentItemData fields
    values.insert("caption".to_string(), serde_json::json!(data.caption));
    values.insert("value".to_string(), serde_json::json!(data.value));
    values.insert("unit".to_string(), serde_json::json!(data.unit));
    values.insert("numerical_value".to_string(), serde_json::json!(data.numerical_value));
    values.insert("min_value".to_string(), serde_json::json!(data.min_value));
    values.insert("max_value".to_string(), serde_json::json!(data.max_value));

    // Add any additional slot values (like hour, minute, second for clock source)
    if let Some(sv) = slot_values {
        for (k, v) in sv {
            // Don't override the basic fields we already set
            if !values.contains_key(k) {
                values.insert(k.clone(), v.clone());
            }
        }
    }

    // Call the reusable render_bar function
    render_bar(cr, bar_config, theme, animated_percent, &values, w, h)?;

    cr.restore()?;
    Ok(())
}

/// Render text-only content (no bar) - uses bar renderer with transparent background
pub fn render_content_text(
    cr: &cairo::Context,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    bar_config: &BarDisplayConfig,
    theme: &ComboThemeConfig,
    data: &ContentItemData,
    slot_values: Option<&HashMap<String, serde_json::Value>>,
) -> Result<(), cairo::Error> {
    cr.save()?;
    cr.translate(x, y);

    // Build values HashMap for text rendering
    let mut values = HashMap::new();

    // Add basic ContentItemData fields
    values.insert("caption".to_string(), serde_json::json!(data.caption));
    values.insert("value".to_string(), serde_json::json!(data.value));
    values.insert("unit".to_string(), serde_json::json!(data.unit));
    values.insert("numerical_value".to_string(), serde_json::json!(data.numerical_value));
    values.insert("min_value".to_string(), serde_json::json!(data.min_value));
    values.insert("max_value".to_string(), serde_json::json!(data.max_value));

    // Add any additional slot values (like hour, minute, second for clock source)
    if let Some(sv) = slot_values {
        for (k, v) in sv {
            // Don't override the basic fields we already set
            if !values.contains_key(k) {
                values.insert(k.clone(), v.clone());
            }
        }
    }

    // Render with 0 value to show text only (bar won't be visible)
    render_bar(cr, bar_config, theme, 0.0, &values, w, h)?;

    cr.restore()?;
    Ok(())
}

/// Calculate layout positions for content items
pub fn calculate_item_layouts(
    content_x: f64,
    content_y: f64,
    content_w: f64,
    content_h: f64,
    item_count: u32,
    item_spacing: f64,
    fixed_heights: &HashMap<usize, f64>, // Index -> fixed height (for graph/level_bar)
) -> Vec<(f64, f64, f64, f64)> {
    if item_count == 0 {
        return Vec::new();
    }

    let mut layouts = Vec::new();
    let mut fixed_total = 0.0;
    let mut flex_count = 0u32;

    for i in 0..item_count as usize {
        if let Some(&fixed_h) = fixed_heights.get(&i) {
            fixed_total += fixed_h;
        } else {
            flex_count += 1;
        }
    }

    let total_spacing = (item_count - 1) as f64 * item_spacing;
    let flex_total = (content_h - fixed_total - total_spacing).max(0.0);
    let flex_height = if flex_count > 0 {
        flex_total / flex_count as f64
    } else {
        0.0
    };

    let mut current_y = content_y;

    for i in 0..item_count as usize {
        let item_h = fixed_heights.get(&i).copied().unwrap_or(flex_height);
        let item_h = item_h.min(content_h - (current_y - content_y));

        if item_h > 0.0 {
            layouts.push((content_x, current_y, content_w, item_h));
            current_y += item_h + item_spacing;
        }
    }

    layouts
}

/// Render a graph content item using the graph_display module
pub fn render_content_graph(
    cr: &cairo::Context,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    config: &GraphDisplayConfig,
    data: &VecDeque<DataPoint>,
    source_values: &HashMap<String, serde_json::Value>,
) -> anyhow::Result<()> {
    log::debug!(
        "LCARS render_content_graph: text_overlay has {} lines, source_values keys: {:?}",
        config.text_overlay.len(),
        source_values.keys().collect::<Vec<_>>()
    );
    if !config.text_overlay.is_empty() {
        for (i, line) in config.text_overlay.iter().enumerate() {
            log::debug!("  text_overlay[{}]: field_id='{}', found={}",
                i, line.field_id, source_values.contains_key(&line.field_id));
        }
    }

    // Save state and translate to the item's position
    cr.save()?;
    cr.translate(x, y);

    // Call the graph_display render function (no scroll animation in LCARS mode)
    render_graph(cr, config, data, source_values, w, h, 0.0)?;

    cr.restore()?;
    Ok(())
}

/// Render core bars content item using the core_bars_display module
pub fn render_content_core_bars(
    cr: &cairo::Context,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    config: &CoreBarsConfig,
    theme: &ComboThemeConfig,
    core_values: &[f64],
    slot_values: Option<&HashMap<String, serde_json::Value>>,
) -> Result<(), cairo::Error> {
    cr.save()?;
    cr.translate(x, y);

    // Call the core_bars_display render function with source values for text overlay
    if let Some(values) = slot_values {
        crate::ui::core_bars_display::render_core_bars_with_values(cr, config, theme, core_values, w, h, values)?;
    } else {
        render_core_bars(cr, config, theme, core_values, w, h)?;
    }

    cr.restore()?;
    Ok(())
}

/// Render static content item (background with optional text overlay)
pub fn render_content_static(
    cr: &cairo::Context,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    config: &StaticDisplayConfig,
    bar_config: &BarDisplayConfig,
    theme: &ComboThemeConfig,
    slot_values: Option<&HashMap<String, serde_json::Value>>,
) -> Result<(), cairo::Error> {
    cr.save()?;
    cr.translate(x, y);

    // Render the background (with theme for polygon color support)
    render_background_with_theme(cr, &config.background, w, h, Some(theme))?;

    // Render text overlay if enabled
    if bar_config.text_overlay.enabled {
        let values = slot_values.cloned().unwrap_or_default();
        crate::ui::text_renderer::render_text_lines_with_theme(
            cr,
            w,
            h,
            &bar_config.text_overlay.text_config,
            &values,
            Some(theme),
        );
    }

    cr.restore()?;
    Ok(())
}
