//! Render cache for expensive rendering operations
//!
//! This module provides caching for:
//! - Loaded images and their Cairo surfaces
//! - Pre-computed gradients
//! - Color interpolation lookup tables

use gtk4::cairo;
use gtk4::gdk_pixbuf::Pixbuf;
use gtk4::prelude::GdkCairoContextExt;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::ui::background::{Color, ColorStop};

// Thread-local image cache (GTK objects aren't thread-safe)
thread_local! {
    static IMAGE_CACHE: RefCell<ImageCache> = RefCell::new(ImageCache::new());
}

/// Get or load a pixbuf from the thread-local cache
pub fn get_cached_pixbuf(path: &str) -> Option<Pixbuf> {
    IMAGE_CACHE.with(|cache| cache.borrow_mut().get_pixbuf(path))
}

/// Get or create a tile surface from the thread-local cache
pub fn get_cached_tile_surface(path: &str) -> Option<cairo::ImageSurface> {
    IMAGE_CACHE.with(|cache| cache.borrow_mut().get_tile_surface(path))
}

/// Invalidate a path in the thread-local cache
pub fn invalidate_cached_image(path: &str) {
    IMAGE_CACHE.with(|cache| cache.borrow_mut().invalidate(path));
}

/// Cache for loaded images and their Cairo surfaces
struct ImageCache {
    /// Cached pixbufs keyed by file path
    pixbufs: HashMap<String, CachedImage>,
    /// Maximum cache size in entries
    max_entries: usize,
    /// Time after which cache entries expire
    expiry_duration: Duration,
}

struct CachedImage {
    pixbuf: Pixbuf,
    /// Pre-rendered tile surface for tiling mode
    tile_surface: Option<cairo::ImageSurface>,
    last_access: Instant,
}

impl ImageCache {
    fn new() -> Self {
        Self {
            pixbufs: HashMap::new(),
            max_entries: 50,
            expiry_duration: Duration::from_secs(300), // 5 minutes
        }
    }

    /// Get or load a pixbuf from the cache
    fn get_pixbuf(&mut self, path: &str) -> Option<Pixbuf> {
        // Check if we have a valid cached entry
        if let Some(entry) = self.pixbufs.get_mut(path) {
            entry.last_access = Instant::now();
            return Some(entry.pixbuf.clone());
        }

        // Try to load the image
        if let Ok(pixbuf) = Pixbuf::from_file(path) {
            // Evict old entries if needed
            self.evict_if_needed();

            self.pixbufs.insert(
                path.to_string(),
                CachedImage {
                    pixbuf: pixbuf.clone(),
                    tile_surface: None,
                    last_access: Instant::now(),
                },
            );
            Some(pixbuf)
        } else {
            None
        }
    }

    /// Get or create a tile surface for a cached image
    fn get_tile_surface(&mut self, path: &str) -> Option<cairo::ImageSurface> {
        // First ensure the pixbuf is loaded
        if !self.pixbufs.contains_key(path) {
            self.get_pixbuf(path)?;
        }

        if let Some(entry) = self.pixbufs.get_mut(path) {
            entry.last_access = Instant::now();

            // Return cached tile surface if available
            if let Some(ref surface) = entry.tile_surface {
                return Some(surface.clone());
            }

            // Create tile surface from pixbuf
            let img_width = entry.pixbuf.width();
            let img_height = entry.pixbuf.height();

            if let Ok(surface) =
                cairo::ImageSurface::create(cairo::Format::ARgb32, img_width, img_height)
            {
                if let Ok(tmp_cr) = cairo::Context::new(&surface) {
                    tmp_cr.set_source_pixbuf(&entry.pixbuf, 0.0, 0.0);
                    let _ = tmp_cr.paint();

                    entry.tile_surface = Some(surface.clone());
                    return Some(surface);
                }
            }
        }
        None
    }

    /// Evict expired or excess entries
    fn evict_if_needed(&mut self) {
        let now = Instant::now();

        // Remove expired entries
        self.pixbufs
            .retain(|_, entry| now.duration_since(entry.last_access) < self.expiry_duration);

        // If still too many, remove oldest
        while self.pixbufs.len() >= self.max_entries {
            if let Some(oldest_key) = self
                .pixbufs
                .iter()
                .min_by_key(|(_, entry)| entry.last_access)
                .map(|(k, _)| k.clone())
            {
                self.pixbufs.remove(&oldest_key);
            } else {
                break;
            }
        }
    }

    /// Invalidate a specific path (call when file might have changed)
    fn invalidate(&mut self, path: &str) {
        self.pixbufs.remove(path);
    }
}

/// Pre-computed color gradient lookup table
#[derive(Clone)]
pub struct ColorGradientLUT {
    /// Pre-computed colors at fixed intervals
    colors: Vec<Color>,
    /// Number of entries in the LUT
    resolution: usize,
}

impl ColorGradientLUT {
    /// Create a new LUT from color stops
    pub fn from_stops(stops: &[ColorStop], resolution: usize) -> Self {
        let resolution = resolution.max(2);
        let mut colors = Vec::with_capacity(resolution);

        for i in 0..resolution {
            let t = i as f64 / (resolution - 1) as f64;
            colors.push(interpolate_color_at(stops, t));
        }

        Self { colors, resolution }
    }

    /// Get color at position t (0.0 to 1.0)
    pub fn get_color(&self, t: f64) -> Color {
        let t = t.clamp(0.0, 1.0);
        let index = (t * (self.resolution - 1) as f64) as usize;
        let index = index.min(self.resolution - 1);

        // For smoother results, interpolate between adjacent LUT entries
        let t_scaled = t * (self.resolution - 1) as f64;
        let frac = t_scaled - t_scaled.floor();

        if frac < 0.001 || index >= self.resolution - 1 {
            self.colors[index]
        } else {
            let c1 = &self.colors[index];
            let c2 = &self.colors[index + 1];
            Color {
                r: c1.r + (c2.r - c1.r) * frac,
                g: c1.g + (c2.g - c1.g) * frac,
                b: c1.b + (c2.b - c1.b) * frac,
                a: c1.a + (c2.a - c1.a) * frac,
            }
        }
    }
}

/// Interpolate color at position t from sorted color stops
fn interpolate_color_at(stops: &[ColorStop], t: f64) -> Color {
    if stops.is_empty() {
        return Color::default();
    }
    if stops.len() == 1 {
        return stops[0].color;
    }

    let t = t.clamp(0.0, 1.0);

    // Find surrounding stops
    let mut prev_stop = &stops[0];
    let mut next_stop = &stops[stops.len() - 1];

    for i in 0..stops.len() - 1 {
        if stops[i].position <= t && stops[i + 1].position >= t {
            prev_stop = &stops[i];
            next_stop = &stops[i + 1];
            break;
        }
    }

    // Handle edge cases
    if t <= prev_stop.position {
        return prev_stop.color;
    }
    if t >= next_stop.position {
        return next_stop.color;
    }

    // Interpolate
    let range = next_stop.position - prev_stop.position;
    if range < 0.001 {
        return prev_stop.color;
    }

    let local_t = (t - prev_stop.position) / range;
    let c1 = &prev_stop.color;
    let c2 = &next_stop.color;

    Color {
        r: c1.r + (c2.r - c1.r) * local_t,
        g: c1.g + (c2.g - c1.g) * local_t,
        b: c1.b + (c2.b - c1.b) * local_t,
        a: c1.a + (c2.a - c1.a) * local_t,
    }
}

/// Cache for text extent measurements
pub struct TextExtentsCache {
    /// Cached extents keyed by (font_family, font_size_x10, text)
    cache: HashMap<(String, i32, String), cairo::TextExtents>,
    max_entries: usize,
}

impl TextExtentsCache {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            max_entries: 500,
        }
    }

    /// Get cached text extents or compute and cache them
    pub fn get_or_compute(
        &mut self,
        cr: &cairo::Context,
        font_family: &str,
        font_size: f64,
        bold: bool,
        italic: bool,
        text: &str,
    ) -> Option<cairo::TextExtents> {
        // Create cache key (multiply size by 10 to capture one decimal place)
        let size_key = (font_size * 10.0) as i32;
        let style_suffix = match (bold, italic) {
            (true, true) => "_BI",
            (true, false) => "_B",
            (false, true) => "_I",
            (false, false) => "",
        };
        let key = (
            format!("{}{}", font_family, style_suffix),
            size_key,
            text.to_string(),
        );

        // Return cached value if available
        if let Some(extents) = self.cache.get(&key) {
            return Some(*extents);
        }

        // Compute extents
        cr.save().ok()?;

        let font_slant = if italic {
            cairo::FontSlant::Italic
        } else {
            cairo::FontSlant::Normal
        };
        let font_weight = if bold {
            cairo::FontWeight::Bold
        } else {
            cairo::FontWeight::Normal
        };

        cr.select_font_face(font_family, font_slant, font_weight);
        cr.set_font_size(font_size);

        let extents = cr.text_extents(text).ok()?;

        cr.restore().ok()?;

        // Cache the result (evict if needed)
        if self.cache.len() >= self.max_entries {
            // Simple eviction: clear half the cache
            let keys_to_remove: Vec<_> = self
                .cache
                .keys()
                .take(self.max_entries / 2)
                .cloned()
                .collect();
            for key in keys_to_remove {
                self.cache.remove(&key);
            }
        }

        self.cache.insert(key, extents);
        Some(extents)
    }

    pub fn clear(&mut self) {
        self.cache.clear();
    }
}

impl Default for TextExtentsCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Global text extents cache
pub static TEXT_EXTENTS_CACHE: once_cell::sync::Lazy<Mutex<TextExtentsCache>> =
    once_cell::sync::Lazy::new(|| Mutex::new(TextExtentsCache::new()));

// Thread-local gradient LUT cache
thread_local! {
    static GRADIENT_LUT_CACHE: RefCell<GradientLUTCache> = RefCell::new(GradientLUTCache::new());
}

/// Cache for pre-computed color gradient LUTs
struct GradientLUTCache {
    /// Cached LUTs keyed by a hash of color stops
    cache: HashMap<u64, ColorGradientLUT>,
    max_entries: usize,
}

impl GradientLUTCache {
    fn new() -> Self {
        Self {
            cache: HashMap::new(),
            max_entries: 20,
        }
    }

    fn get_or_create(&mut self, stops: &[ColorStop], resolution: usize) -> ColorGradientLUT {
        let key = hash_color_stops(stops);

        if let Some(lut) = self.cache.get(&key) {
            return lut.clone();
        }

        // Evict oldest if full
        if self.cache.len() >= self.max_entries {
            if let Some(&oldest_key) = self.cache.keys().next() {
                self.cache.remove(&oldest_key);
            }
        }

        let lut = ColorGradientLUT::from_stops(stops, resolution);
        self.cache.insert(key, lut.clone());
        lut
    }
}

/// Hash color stops for cache key
fn hash_color_stops(stops: &[ColorStop]) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    for stop in stops {
        // Hash position (convert to integer bits)
        stop.position.to_bits().hash(&mut hasher);
        // Hash color components
        stop.color.r.to_bits().hash(&mut hasher);
        stop.color.g.to_bits().hash(&mut hasher);
        stop.color.b.to_bits().hash(&mut hasher);
        stop.color.a.to_bits().hash(&mut hasher);
    }
    hasher.finish()
}

/// Get a cached color gradient LUT or create one
/// Resolution of 256 is enough for smooth gradients while being efficient
pub fn get_gradient_lut(stops: &[ColorStop]) -> ColorGradientLUT {
    const DEFAULT_RESOLUTION: usize = 256;
    GRADIENT_LUT_CACHE.with(|cache| cache.borrow_mut().get_or_create(stops, DEFAULT_RESOLUTION))
}

/// Get color at position using cached LUT (for Smooth transition)
/// For Abrupt transitions, use get_abrupt_color instead
pub fn get_cached_color_at(stops: &[ColorStop], t: f64) -> Color {
    if stops.is_empty() {
        return Color::default();
    }
    if stops.len() == 1 {
        return stops[0].color;
    }

    let lut = get_gradient_lut(stops);
    lut.get_color(t)
}

/// Get color using abrupt transition (no interpolation)
pub fn get_abrupt_color(stops: &[ColorStop], t: f64) -> Color {
    if stops.is_empty() {
        return Color::default();
    }

    let t = t.clamp(0.0, 1.0);

    // Find the stop at or before t
    let mut result = &stops[0];
    for stop in stops {
        if stop.position <= t {
            result = stop;
        } else {
            break;
        }
    }
    result.color
}
