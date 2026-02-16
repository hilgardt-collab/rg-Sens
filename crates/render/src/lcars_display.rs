//! LCARS (Library Computer Access/Retrieval System) display rendering
//!
//! This module provides rendering functions for the LCARS-style interface,
//! featuring a sidebar with segments, optional top/bottom extensions,
//! and a configurable content area for displaying various data types.

use gtk4::cairo;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::f64::consts::PI;

use std::cell::RefCell;

use crate::combo_traits::FrameRenderer;
use crate::background::{render_background_with_theme, Color};
use crate::bar_display::render_bar;
use crate::core_bars_display::render_core_bars;
use crate::graph_display::render_graph_with_theme;
use crate::pango_text::{pango_show_text, pango_text_extents};
use rg_sens_types::theme::ComboThemeConfig;

// Re-export all LCARS types from the types crate
pub use rg_sens_types::display_configs::lcars::*;

// Thread-local buffer for render values HashMap to avoid per-frame allocations.
// The HashMap is reused across calls, only clearing and repopulating values.
thread_local! {
    static RENDER_VALUES_BUFFER: RefCell<HashMap<String, serde_json::Value>> = RefCell::new(HashMap::with_capacity(16));
}

/// Populate the thread-local render values buffer from ContentItemData.
/// Returns a reference to the populated HashMap via the callback.
/// This avoids allocating a new HashMap on every frame.
#[inline]
fn with_render_values<F, R>(
    data: &ContentItemData,
    slot_values: Option<&HashMap<String, serde_json::Value>>,
    f: F,
) -> R
where
    F: FnOnce(&HashMap<String, serde_json::Value>) -> R,
{
    RENDER_VALUES_BUFFER.with(|buffer| {
        let mut values = buffer.borrow_mut();
        values.clear();

        // Add basic ContentItemData fields
        // Using static key strings where possible to avoid allocation
        values.insert(
            "caption".to_string(),
            serde_json::Value::String(data.caption.clone()),
        );
        values.insert(
            "value".to_string(),
            serde_json::Value::String(data.value.clone()),
        );
        values.insert(
            "unit".to_string(),
            serde_json::Value::String(data.unit.clone()),
        );
        values.insert(
            "numerical_value".to_string(),
            serde_json::Value::from(data.numerical_value),
        );
        values.insert(
            "min_value".to_string(),
            serde_json::Value::from(data.min_value),
        );
        values.insert(
            "max_value".to_string(),
            serde_json::Value::from(data.max_value),
        );

        // Add any additional slot values (like hour, minute, second for clock source)
        if let Some(sv) = slot_values {
            for (k, v) in sv {
                // Don't override the basic fields we already set
                if !values.contains_key(k) {
                    values.insert(k.clone(), v.clone());
                }
            }
        }

        // Call the closure with the populated buffer
        f(&values)
    })
}

/// Frame renderer for LCARS theme
pub struct LcarsRenderer;

impl FrameRenderer for LcarsRenderer {
    type Config = LcarsFrameConfig;

    fn theme_id(&self) -> &'static str {
        "lcars"
    }

    fn theme_name(&self) -> &'static str {
        "LCARS"
    }

    fn default_config(&self) -> Self::Config {
        LcarsFrameConfig::default()
    }

    fn render_frame(
        &self,
        cr: &cairo::Context,
        config: &Self::Config,
        width: f64,
        height: f64,
    ) -> anyhow::Result<(f64, f64, f64, f64)> {
        render_lcars_frame(cr, config, width, height).map_err(|e| anyhow::anyhow!("{}", e))?;
        Ok(get_content_bounds(config, width, height))
    }

    fn calculate_group_layouts(
        &self,
        config: &Self::Config,
        content_x: f64,
        content_y: f64,
        content_w: f64,
        content_h: f64,
    ) -> Vec<(f64, f64, f64, f64)> {
        // LCARS uses group_count and group_item_counts differently
        // Calculate group layouts based on the group count
        let group_count = config.group_count;
        if group_count == 0 {
            return Vec::new();
        }

        let empty_fixed_sizes: HashMap<usize, f64> = HashMap::new();
        calculate_item_layouts_with_orientation(
            content_x,
            content_y,
            content_w,
            content_h,
            group_count,
            config.item_spacing,
            &empty_fixed_sizes,
            config.layout_orientation,
        )
    }

    fn draw_group_dividers(
        &self,
        cr: &cairo::Context,
        config: &Self::Config,
        group_layouts: &[(f64, f64, f64, f64)],
    ) {
        // LCARS renders dividers as part of the frame rendering
        // but we can still draw them here for consistency
        if group_layouts.len() < 2 {
            return;
        }

        let divider_w = config.divider_config.width;

        for window in group_layouts.windows(2) {
            let (x1, y1, w1, h1) = window[0];
            let (x2, y2, _, _) = window[1];

            match config.layout_orientation {
                SplitOrientation::Vertical => {
                    // Vertical layout - groups side by side, dividers are vertical
                    let divider_x = x2 - divider_w / 2.0 - config.item_spacing / 2.0;
                    let _ = render_divider(
                        cr,
                        divider_x,
                        y1,
                        divider_w,
                        h1,
                        &config.divider_config,
                        SplitOrientation::Vertical,
                        &config.theme,
                    );
                }
                SplitOrientation::Horizontal => {
                    // Horizontal layout - groups stacked, dividers are horizontal
                    let divider_y = y2 - divider_w / 2.0 - config.item_spacing / 2.0;
                    let _ = render_divider(
                        cr,
                        x1,
                        divider_y,
                        w1,
                        divider_w,
                        &config.divider_config,
                        SplitOrientation::Horizontal,
                        &config.theme,
                    );
                }
            }
        }
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
            ((self.numerical_value - self.min_value) / (self.max_value - self.min_value))
                .clamp(0.0, 1.0)
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

    let has_top_ext = matches!(
        config.extension_mode,
        ExtensionMode::Top | ExtensionMode::Both
    );
    let has_bottom_ext = matches!(
        config.extension_mode,
        ExtensionMode::Bottom | ExtensionMode::Both
    );

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
            .map(|i| {
                config
                    .segments
                    .get(i)
                    .map(|s| s.height_weight)
                    .unwrap_or(1.0)
            })
            .sum()
    };
    let total_weight = if total_weight <= 0.0 {
        1.0
    } else {
        total_weight
    };

    // Available height for segments (between extensions)
    let segments_start_y = if has_top_ext { top_bar_h } else { 0.0 };
    let segments_end_y = if has_bottom_ext {
        height - bottom_ext_h
    } else {
        height
    };
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
            cr.arc_negative(
                sidebar_w + radius,
                segments_end_y - radius,
                radius,
                PI,
                0.5 * PI,
            );
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
        let text_extents = pango_text_extents(
            cr,
            &label_info.text,
            &label_info.font,
            cairo::FontSlant::Normal,
            cairo::FontWeight::Normal,
            label_info.font_size,
        );

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
        pango_show_text(
            cr,
            &label_info.text,
            &label_info.font,
            cairo::FontSlant::Normal,
            cairo::FontWeight::Normal,
            label_info.font_size,
        );
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
    let text_extents = pango_text_extents(
        cr,
        &text,
        &font_family,
        cairo::FontSlant::Normal,
        font_weight,
        font_size,
    );

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
            cr.arc(
                bar_x + bar_radius,
                bar_y + bar_radius,
                bar_radius,
                0.5 * PI,
                1.5 * PI,
            );
            cr.arc(
                bar_x + bar_w - bar_radius,
                bar_y + bar_radius,
                bar_radius,
                1.5 * PI,
                0.5 * PI,
            );
            cr.close_path();
        }
    }

    header_config
        .bg_color
        .resolve(&frame_config.theme)
        .apply_to_cairo(cr);
    cr.fill()?;

    // Draw header text
    header_config
        .text_color
        .resolve(&frame_config.theme)
        .apply_to_cairo(cr);
    let text_x = bar_x + (bar_w - text_extents.width()) / 2.0;
    let text_y = bar_y + (bar_content_h + font_size * 0.7) / 2.0;
    cr.move_to(text_x, text_y);
    pango_show_text(
        cr,
        &text,
        &font_family,
        cairo::FontSlant::Normal,
        font_weight,
        font_size,
    );

    cr.restore()?;
    Ok(())
}
pub fn get_content_bounds(
    config: &LcarsFrameConfig,
    width: f64,
    height: f64,
) -> (f64, f64, f64, f64) {
    let has_top_ext = matches!(
        config.extension_mode,
        ExtensionMode::Top | ExtensionMode::Both
    );
    let has_bottom_ext = matches!(
        config.extension_mode,
        ExtensionMode::Bottom | ExtensionMode::Both
    );

    let top_bar_h = if has_top_ext {
        config.top_bar_height
    } else {
        0.0
    };
    let bottom_ext_h = if has_bottom_ext {
        config.bottom_bar_height
    } else {
        0.0
    };

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

            let end_x = x + w
                - if divider_config.cap_end == DividerCapStyle::Round {
                    radius
                } else {
                    0.0
                };
            cr.line_to(end_x, y);

            match divider_config.cap_end {
                DividerCapStyle::Round => {
                    cr.arc(x + w - radius, y + radius, radius, 1.5 * PI, 0.5 * PI);
                }
                DividerCapStyle::Square => {
                    cr.line_to(x + w, y + h);
                }
            }

            let start_x = x + if divider_config.cap_start == DividerCapStyle::Round {
                radius
            } else {
                0.0
            };
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

            let end_y = y + h
                - if divider_config.cap_end == DividerCapStyle::Round {
                    radius
                } else {
                    0.0
                };
            cr.line_to(x + w, end_y);

            match divider_config.cap_end {
                DividerCapStyle::Round => {
                    cr.arc(x + radius, y + h - radius, radius, 0.0, PI);
                }
                DividerCapStyle::Square => {
                    cr.line_to(x, y + h);
                }
            }

            let start_y = y + if divider_config.cap_start == DividerCapStyle::Round {
                radius
            } else {
                0.0
            };
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
    // Guard against invalid dimensions that would cause Cairo matrix errors
    if !x.is_finite() || !y.is_finite() || !w.is_finite() || !h.is_finite() || w <= 0.0 || h <= 0.0
    {
        return Ok(());
    }

    cr.save()?;
    cr.translate(x, y);

    // Use thread-local buffer to avoid per-frame HashMap allocations
    let result = with_render_values(data, slot_values, |values| {
        render_bar(cr, bar_config, theme, animated_percent, values, w, h)
    });

    cr.restore()?;
    result
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
    // Guard against invalid dimensions that would cause Cairo matrix errors
    if !x.is_finite() || !y.is_finite() || !w.is_finite() || !h.is_finite() || w <= 0.0 || h <= 0.0
    {
        return Ok(());
    }

    cr.save()?;
    cr.translate(x, y);

    // Use thread-local buffer to avoid per-frame HashMap allocations
    // Render with 0 value to show text only (bar won't be visible)
    let result = with_render_values(data, slot_values, |values| {
        render_bar(cr, bar_config, theme, 0.0, values, w, h)
    });

    cr.restore()?;
    result
}

/// Calculate layout positions for content items
/// If orientation is Horizontal, items are laid out side by side (left to right).
/// If orientation is Vertical (default), items are stacked top to bottom.
pub fn calculate_item_layouts(
    content_x: f64,
    content_y: f64,
    content_w: f64,
    content_h: f64,
    item_count: u32,
    item_spacing: f64,
    fixed_sizes: &HashMap<usize, f64>, // Index -> fixed size (height for vertical, width for horizontal)
) -> Vec<(f64, f64, f64, f64)> {
    calculate_item_layouts_with_orientation(
        content_x,
        content_y,
        content_w,
        content_h,
        item_count,
        item_spacing,
        fixed_sizes,
        SplitOrientation::Vertical, // Default to vertical for backwards compatibility
    )
}

/// Calculate layout positions for content items with explicit orientation
/// If orientation is Horizontal, items are laid out side by side (left to right).
/// If orientation is Vertical, items are stacked top to bottom.
pub fn calculate_item_layouts_with_orientation(
    content_x: f64,
    content_y: f64,
    content_w: f64,
    content_h: f64,
    item_count: u32,
    item_spacing: f64,
    fixed_sizes: &HashMap<usize, f64>, // Index -> fixed size (height for vertical, width for horizontal)
    orientation: SplitOrientation,
) -> Vec<(f64, f64, f64, f64)> {
    if item_count == 0 {
        return Vec::new();
    }

    let mut layouts = Vec::new();
    let mut fixed_total = 0.0;
    let mut flex_count = 0u32;

    for i in 0..item_count as usize {
        if let Some(&fixed_s) = fixed_sizes.get(&i) {
            fixed_total += fixed_s;
        } else {
            flex_count += 1;
        }
    }

    let total_spacing = (item_count - 1) as f64 * item_spacing;

    match orientation {
        SplitOrientation::Vertical => {
            // Stack items top to bottom
            let flex_total = (content_h - fixed_total - total_spacing).max(0.0);
            let flex_size = if flex_count > 0 {
                flex_total / flex_count as f64
            } else {
                0.0
            };

            let mut current_y = content_y;

            for i in 0..item_count as usize {
                let item_h = fixed_sizes.get(&i).copied().unwrap_or(flex_size);
                let item_h = item_h.min(content_h - (current_y - content_y));

                if item_h > 0.0 {
                    layouts.push((content_x, current_y, content_w, item_h));
                    current_y += item_h + item_spacing;
                }
            }
        }
        SplitOrientation::Horizontal => {
            // Lay items side by side (left to right)
            let flex_total = (content_w - fixed_total - total_spacing).max(0.0);
            let flex_size = if flex_count > 0 {
                flex_total / flex_count as f64
            } else {
                0.0
            };

            let mut current_x = content_x;

            for i in 0..item_count as usize {
                let item_w = fixed_sizes.get(&i).copied().unwrap_or(flex_size);
                let item_w = item_w.min(content_w - (current_x - content_x));

                if item_w > 0.0 {
                    layouts.push((current_x, content_y, item_w, content_h));
                    current_x += item_w + item_spacing;
                }
            }
        }
    }

    layouts
}

/// Render a graph content item using the graph_display module
///
/// The `theme` parameter allows passing the panel's current theme for color resolution,
/// ensuring graph colors update when panel theme colors change.
pub fn render_content_graph(
    cr: &cairo::Context,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    config: &GraphDisplayConfig,
    theme: &ComboThemeConfig,
    data: &VecDeque<DataPoint>,
    source_values: &HashMap<String, serde_json::Value>,
) -> anyhow::Result<()> {
    // Guard against invalid dimensions that would cause Cairo matrix errors
    if !x.is_finite() || !y.is_finite() || !w.is_finite() || !h.is_finite() || w <= 0.0 || h <= 0.0
    {
        return Ok(());
    }

    log::debug!(
        "LCARS render_content_graph: text_overlay enabled={}, lines={}, source_values keys: {:?}",
        config.text_overlay.enabled,
        config.text_overlay.text_config.lines.len(),
        source_values.keys().collect::<Vec<_>>()
    );
    if config.text_overlay.enabled && !config.text_overlay.text_config.lines.is_empty() {
        for (i, line) in config.text_overlay.text_config.lines.iter().enumerate() {
            log::debug!(
                "  text_overlay[{}]: field_id='{}', found={}",
                i,
                line.field_id,
                source_values.contains_key(&line.field_id)
            );
        }
    }

    // Save state and translate to the item's position
    cr.save()?;
    cr.translate(x, y);

    // Call the graph_display render function with the panel's theme for color resolution
    render_graph_with_theme(cr, config, data, source_values, w, h, 0.0, Some(theme))?;

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
    // Guard against invalid dimensions that would cause Cairo matrix errors
    if !x.is_finite() || !y.is_finite() || !w.is_finite() || !h.is_finite() || w <= 0.0 || h <= 0.0
    {
        return Ok(());
    }

    cr.save()?;
    cr.translate(x, y);

    // Call the core_bars_display render function with source values for text overlay
    if let Some(values) = slot_values {
        crate::core_bars_display::render_core_bars_with_values(
            cr,
            config,
            theme,
            core_values,
            w,
            h,
            values,
        )?;
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
    theme: &ComboThemeConfig,
    slot_values: Option<&HashMap<String, serde_json::Value>>,
) -> Result<(), cairo::Error> {
    // Guard against invalid dimensions that would cause Cairo matrix errors
    if !x.is_finite() || !y.is_finite() || !w.is_finite() || !h.is_finite() || w <= 0.0 || h <= 0.0
    {
        return Ok(());
    }

    cr.save()?;
    cr.translate(x, y);

    // Render the background (with theme for polygon color support)
    render_background_with_theme(cr, &config.background, w, h, Some(theme))?;

    // Render text overlay if enabled
    if config.text_overlay.enabled {
        let values = slot_values.cloned().unwrap_or_default();
        crate::text_renderer::render_text_lines_with_theme(
            cr,
            w,
            h,
            &config.text_overlay.text_config,
            &values,
            Some(theme),
        );
    }

    cr.restore()?;
    Ok(())
}
