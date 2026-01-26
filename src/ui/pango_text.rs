//! Pango-based text rendering to replace Cairo's toy font API
//!
//! Cairo's toy font API (select_font_face, show_text, text_extents) creates
//! internal font caches that grow unboundedly, causing memory leaks.
//! Pango properly manages font resources and integrates with fontconfig.
//!
//! This module provides drop-in replacements for Cairo text functions:
//! - `pango_text_extents()` replaces `cr.text_extents()`
//! - `pango_show_text()` replaces `cr.show_text()`

use cairo::Context;
use pango::{FontDescription, Layout, Style as PangoStyle, Weight as PangoWeight};
use pangocairo::functions::{create_layout, show_layout};
use std::cell::RefCell;
use std::collections::HashMap;
use std::time::Instant;

/// Text extents returned by pango_text_extents
#[derive(Debug, Clone, Copy, Default)]
pub struct TextExtents {
    pub width: f64,
    pub height: f64,
    pub x_bearing: f64,
    pub y_bearing: f64,
}

impl TextExtents {
    pub fn width(&self) -> f64 {
        self.width
    }

    pub fn height(&self) -> f64 {
        self.height
    }

    pub fn x_bearing(&self) -> f64 {
        self.x_bearing
    }

    pub fn y_bearing(&self) -> f64 {
        self.y_bearing
    }
}

/// Cache for FontDescription objects to avoid repeated allocations
struct FontDescriptionCache {
    cache: HashMap<FontKey, FontDescription>,
    max_entries: usize,
}

#[derive(Hash, Eq, PartialEq, Clone, Debug)]
struct FontKey {
    family: String,
    weight: i32,
    style: u8,
    size_pango: i32, // Size in Pango units (points * PANGO_SCALE)
}

impl FontDescriptionCache {
    fn new() -> Self {
        Self {
            cache: HashMap::new(),
            max_entries: 64,
        }
    }

    fn get_or_create(
        &mut self,
        family: &str,
        weight: PangoWeight,
        style: PangoStyle,
        size: f64,
    ) -> FontDescription {
        let size_pango = (size * pango::SCALE as f64) as i32;
        let weight_i32 = match weight {
            PangoWeight::Thin => 100,
            PangoWeight::Ultralight => 200,
            PangoWeight::Light => 300,
            PangoWeight::Semilight => 350,
            PangoWeight::Book => 380,
            PangoWeight::Normal => 400,
            PangoWeight::Medium => 500,
            PangoWeight::Semibold => 600,
            PangoWeight::Bold => 700,
            PangoWeight::Ultrabold => 800,
            PangoWeight::Heavy => 900,
            PangoWeight::Ultraheavy => 1000,
            _ => 400, // Default to normal
        };
        let key = FontKey {
            family: family.to_string(),
            weight: weight_i32,
            style: match style {
                PangoStyle::Normal => 0,
                PangoStyle::Italic => 1,
                PangoStyle::Oblique => 2,
                _ => 0,
            },
            size_pango,
        };

        if let Some(desc) = self.cache.get(&key) {
            return desc.clone();
        }

        // Evict if full (simple eviction - just clear half)
        if self.cache.len() >= self.max_entries {
            let keys_to_remove: Vec<_> = self
                .cache
                .keys()
                .take(self.max_entries / 2)
                .cloned()
                .collect();
            for k in keys_to_remove {
                self.cache.remove(&k);
            }
        }

        let mut desc = FontDescription::new();
        desc.set_family(family);
        desc.set_weight(weight);
        desc.set_style(style);
        desc.set_size(size_pango);

        self.cache.insert(key, desc.clone());
        desc
    }
}

/// Cache for reusing Pango Layout objects
/// Layouts are relatively expensive to create, so we reuse a small pool
#[allow(dead_code)]
struct LayoutCache {
    /// Cached layout (we only keep one since Context changes each frame)
    layout: Option<Layout>,
    /// Last clear time
    last_clear: Instant,
    /// Clear interval (5 minutes)
    clear_interval_secs: u64,
}

#[allow(dead_code)]
impl LayoutCache {
    fn new() -> Self {
        Self {
            layout: None,
            last_clear: Instant::now(),
            clear_interval_secs: 300, // 5 minutes
        }
    }

    /// Get or create a layout for the given context
    /// Note: We can't actually cache layouts across contexts, but this
    /// provides a hook for periodic cleanup
    fn get_layout(&mut self, cr: &Context) -> Layout {
        // Periodic cleanup to prevent memory growth
        if self.last_clear.elapsed().as_secs() > self.clear_interval_secs {
            self.clear();
            self.last_clear = Instant::now();
        }
        create_layout(cr)
    }

    fn clear(&mut self) {
        self.layout = None;
        log::debug!("LayoutCache cleared");
    }
}

// Thread-local caches for Pango objects
thread_local! {
    static FONT_DESC_CACHE: RefCell<FontDescriptionCache> = RefCell::new(FontDescriptionCache::new());
    static LAYOUT_CACHE: RefCell<LayoutCache> = RefCell::new(LayoutCache::new());
}

/// Clear all Pango-related caches
/// Call this periodically or when memory pressure is detected
pub fn clear_pango_caches() {
    FONT_DESC_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        cache.cache.clear();
    });
    LAYOUT_CACHE.with(|cache| {
        cache.borrow_mut().clear();
    });
    log::info!("Pango caches cleared");
}

/// Convert Cairo font weight to Pango weight
#[inline]
pub fn cairo_weight_to_pango(weight: cairo::FontWeight) -> PangoWeight {
    match weight {
        cairo::FontWeight::Normal => PangoWeight::Normal,
        cairo::FontWeight::Bold => PangoWeight::Bold,
        _ => PangoWeight::Normal,
    }
}

/// Convert Cairo font slant to Pango style
#[inline]
pub fn cairo_slant_to_pango(slant: cairo::FontSlant) -> PangoStyle {
    match slant {
        cairo::FontSlant::Normal => PangoStyle::Normal,
        cairo::FontSlant::Italic => PangoStyle::Italic,
        cairo::FontSlant::Oblique => PangoStyle::Oblique,
        _ => PangoStyle::Normal,
    }
}

/// Get text extents using Pango (replaces cr.text_extents)
///
/// This function uses Pango to measure text, avoiding Cairo's toy font API
/// which causes memory leaks.
///
/// # Arguments
/// * `cr` - Cairo context (used to create Pango layout)
/// * `text` - Text to measure
/// * `family` - Font family name (e.g., "Sans", "Monospace")
/// * `slant` - Cairo font slant
/// * `weight` - Cairo font weight
/// * `size` - Font size in points
///
/// # Returns
/// TextExtents with Cairo-compatible values:
/// - `y_bearing`: distance from baseline to ink top (usually negative for ascending text)
/// - `x_bearing`: distance from origin to ink left edge
pub fn pango_text_extents(
    cr: &Context,
    text: &str,
    family: &str,
    slant: cairo::FontSlant,
    weight: cairo::FontWeight,
    size: f64,
) -> TextExtents {
    let pango_weight = cairo_weight_to_pango(weight);
    let pango_style = cairo_slant_to_pango(slant);

    FONT_DESC_CACHE.with(|cache| {
        let font_desc = cache
            .borrow_mut()
            .get_or_create(family, pango_weight, pango_style, size);

        // Create layout
        let layout = create_layout(cr);
        layout.set_font_description(Some(&font_desc));
        layout.set_text(text);

        // Use high-precision extents (in Pango units) for accuracy
        let (ink_rect, _logical_rect) = layout.extents();
        let baseline = layout.baseline();

        // Convert from Pango units to pixels
        let scale = pango::SCALE as f64;

        // Calculate Cairo-compatible y_bearing:
        // Cairo's y_bearing = distance from baseline to ink top (negative for ascending text)
        // Pango's ink_rect.y() = distance from logical rect top to ink top
        // Pango's baseline = distance from logical rect top to baseline
        // So: y_bearing = ink_rect.y - baseline
        let y_bearing = (ink_rect.y() - baseline) as f64 / scale;

        TextExtents {
            width: ink_rect.width() as f64 / scale,
            height: ink_rect.height() as f64 / scale,
            x_bearing: ink_rect.x() as f64 / scale,
            y_bearing,
        }
    })
}

/// Show text using Pango (replaces cr.show_text)
///
/// This function renders text using Pango, avoiding Cairo's toy font API
/// which causes memory leaks.
///
/// Note: Unlike Cairo's show_text which advances the current point,
/// this function renders at the current point without advancing.
/// The text baseline is at the current y position.
///
/// # Arguments
/// * `cr` - Cairo context
/// * `text` - Text to render
/// * `family` - Font family name
/// * `slant` - Cairo font slant
/// * `weight` - Cairo font weight
/// * `size` - Font size in points
pub fn pango_show_text(
    cr: &Context,
    text: &str,
    family: &str,
    slant: cairo::FontSlant,
    weight: cairo::FontWeight,
    size: f64,
) {
    let pango_weight = cairo_weight_to_pango(weight);
    let pango_style = cairo_slant_to_pango(slant);

    FONT_DESC_CACHE.with(|cache| {
        let font_desc = cache
            .borrow_mut()
            .get_or_create(family, pango_weight, pango_style, size);

        let layout = create_layout(cr);
        layout.set_font_description(Some(&font_desc));
        layout.set_text(text);

        // Get the baseline offset - Pango draws from top-left, Cairo from baseline
        let baseline = layout.baseline() as f64 / pango::SCALE as f64;

        // Save current position
        let (x, y) = cr.current_point().unwrap_or((0.0, 0.0));

        // Move up by baseline to align with Cairo's baseline-relative positioning
        cr.rel_move_to(0.0, -baseline);

        // Show the layout
        show_layout(cr, &layout);

        // Restore position (Cairo's show_text advances the point, but we'll be consistent)
        cr.move_to(x, y);
    });
}

/// Show text at a specific position using Pango
///
/// This is a convenience function that combines move_to and show_text.
/// The y coordinate is treated as the text baseline (like Cairo).
///
/// # Arguments
/// * `cr` - Cairo context
/// * `x` - X position
/// * `y` - Y position (baseline)
/// * `text` - Text to render
/// * `family` - Font family name
/// * `slant` - Cairo font slant
/// * `weight` - Cairo font weight
/// * `size` - Font size in points
pub fn pango_show_text_at(
    cr: &Context,
    x: f64,
    y: f64,
    text: &str,
    family: &str,
    slant: cairo::FontSlant,
    weight: cairo::FontWeight,
    size: f64,
) {
    cr.move_to(x, y);
    pango_show_text(cr, text, family, slant, weight, size);
}

/// Simplified text rendering - shows text at current position
///
/// Uses the font already set up via apply_pango_font or similar.
/// This is for cases where font is set once and multiple texts are drawn.
pub fn show_layout_text(cr: &Context, text: &str, font_desc: &FontDescription) {
    let layout = create_layout(cr);
    layout.set_font_description(Some(font_desc));
    layout.set_text(text);

    // Get baseline for proper positioning
    let baseline = layout.baseline() as f64 / pango::SCALE as f64;

    let (x, y) = cr.current_point().unwrap_or((0.0, 0.0));
    cr.rel_move_to(0.0, -baseline);
    show_layout(cr, &layout);
    cr.move_to(x, y);
}

/// Get a cached FontDescription for use with show_layout_text
pub fn get_font_description(
    family: &str,
    slant: cairo::FontSlant,
    weight: cairo::FontWeight,
    size: f64,
) -> FontDescription {
    let pango_weight = cairo_weight_to_pango(weight);
    let pango_style = cairo_slant_to_pango(slant);

    FONT_DESC_CACHE.with(|cache| {
        cache
            .borrow_mut()
            .get_or_create(family, pango_weight, pango_style, size)
    })
}

/// Get text extents using an existing FontDescription
pub fn get_text_extents_with_font(
    cr: &Context,
    text: &str,
    font_desc: &FontDescription,
) -> TextExtents {
    let layout = create_layout(cr);
    layout.set_font_description(Some(font_desc));
    layout.set_text(text);

    // Use high-precision extents (in Pango units) for accuracy
    let (ink_rect, _logical_rect) = layout.extents();
    let baseline = layout.baseline();

    // Convert from Pango units to pixels
    let scale = pango::SCALE as f64;

    // Calculate Cairo-compatible y_bearing (distance from baseline to ink top)
    let y_bearing = (ink_rect.y() - baseline) as f64 / scale;

    TextExtents {
        width: ink_rect.width() as f64 / scale,
        height: ink_rect.height() as f64 / scale,
        x_bearing: ink_rect.x() as f64 / scale,
        y_bearing,
    }
}

/// Font metrics for proper text positioning
#[derive(Debug, Clone, Copy, Default)]
pub struct FontMetrics {
    /// Distance from top of logical rect to baseline (use this instead of font_size for positioning)
    pub ascent: f64,
    /// Distance from baseline to bottom of logical rect
    pub descent: f64,
    /// Total logical height (ascent + descent)
    pub height: f64,
}

/// Get font metrics for proper baseline positioning
///
/// Use `ascent` instead of `font_size` when calculating baseline position.
/// For example: `baseline_y = top_y + metrics.ascent`
pub fn get_font_metrics(
    cr: &Context,
    family: &str,
    slant: cairo::FontSlant,
    weight: cairo::FontWeight,
    size: f64,
) -> FontMetrics {
    let pango_weight = cairo_weight_to_pango(weight);
    let pango_style = cairo_slant_to_pango(slant);

    FONT_DESC_CACHE.with(|cache| {
        let font_desc = cache
            .borrow_mut()
            .get_or_create(family, pango_weight, pango_style, size);

        let layout = create_layout(cr);
        layout.set_font_description(Some(&font_desc));
        layout.set_text("Xg"); // Representative text with ascender and descender

        let scale = pango::SCALE as f64;
        let baseline = layout.baseline() as f64 / scale;
        let (_, logical_rect) = layout.extents();
        let height = logical_rect.height() as f64 / scale;

        FontMetrics {
            ascent: baseline,
            descent: height - baseline,
            height,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_key_hash() {
        // Use the same weight conversion as get_or_create() since PangoWeight doesn't impl Into<i32>
        let weight_i32 = 400; // PangoWeight::Normal = 400
        let key1 = FontKey {
            family: "Sans".to_string(),
            weight: weight_i32,
            style: 0,
            size_pango: 12 * pango::SCALE,
        };
        let key2 = FontKey {
            family: "Sans".to_string(),
            weight: weight_i32,
            style: 0,
            size_pango: 12 * pango::SCALE,
        };
        assert_eq!(key1, key2);
    }
}
