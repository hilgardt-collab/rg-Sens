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
use std::sync::Arc;
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

// Thread-local scaled surface cache (separate from pixbuf cache for better memory management)
thread_local! {
    static SCALED_SURFACE_CACHE: RefCell<ScaledSurfaceCache> = RefCell::new(ScaledSurfaceCache::new());
}

/// Get or create a pre-scaled surface for the given image at the specified dimensions and display mode
/// This avoids expensive set_source_pixbuf + scale operations on every frame
pub fn get_cached_scaled_surface(
    path: &str,
    target_width: i32,
    target_height: i32,
    display_mode: u8, // 0=Fit, 1=Stretch, 2=Zoom
    alpha: f64,
) -> Option<cairo::ImageSurface> {
    SCALED_SURFACE_CACHE.with(|cache| {
        cache
            .borrow_mut()
            .get_or_create(path, target_width, target_height, display_mode, alpha)
    })
}

/// Cache for pre-scaled image surfaces at specific dimensions
struct ScaledSurfaceCache {
    /// Cached surfaces keyed by (path, width, height, mode, alpha*100)
    cache: HashMap<(String, i32, i32, u8, i32), ScaledSurfaceEntry>,
    max_entries: usize,
}

struct ScaledSurfaceEntry {
    surface: cairo::ImageSurface,
    last_access: Instant,
}

impl ScaledSurfaceCache {
    fn new() -> Self {
        Self {
            cache: HashMap::new(),
            max_entries: 5, // Reduced from 30 - each surface can be 30+ MB!
        }
    }

    fn get_or_create(
        &mut self,
        path: &str,
        target_width: i32,
        target_height: i32,
        display_mode: u8,
        alpha: f64,
    ) -> Option<cairo::ImageSurface> {
        // Round dimensions to nearest 16 pixels to reduce cache key explosion
        // This prevents creating new 30MB+ surfaces for tiny size changes
        let rounded_width = ((target_width + 8) / 16) * 16;
        let rounded_height = ((target_height + 8) / 16) * 16;

        // Quantize alpha to 10% precision (was 1%) to further reduce cache entries
        let alpha_key = (alpha * 10.0) as i32;
        let key = (
            path.to_string(),
            rounded_width,
            rounded_height,
            display_mode,
            alpha_key,
        );

        // Check cache
        if let Some(entry) = self.cache.get_mut(&key) {
            entry.last_access = Instant::now();
            return Some(entry.surface.clone());
        }

        // Load the source pixbuf
        let pixbuf = get_cached_pixbuf(path)?;
        let img_width = pixbuf.width() as f64;
        let img_height = pixbuf.height() as f64;

        // Create target surface at rounded dimensions (matches cache key)
        // Using rounded dimensions ensures cache consistency
        let surface =
            cairo::ImageSurface::create(cairo::Format::ARgb32, rounded_width, rounded_height)
                .ok()?;

        let cr = cairo::Context::new(&surface).ok()?;
        let width = rounded_width as f64;
        let height = rounded_height as f64;

        // Render scaled image based on display mode
        match display_mode {
            0 => {
                // Fit: Scale to fit (maintain aspect ratio, may have empty space)
                let scale = (width / img_width).min(height / img_height);
                cr.scale(scale, scale);
                cr.translate(
                    (width / scale - img_width) / 2.0,
                    (height / scale - img_height) / 2.0,
                );
                cr.set_source_pixbuf(&pixbuf, 0.0, 0.0);
                cr.paint_with_alpha(alpha).ok()?;
            }
            1 => {
                // Stretch: Stretch to fill (may distort)
                cr.scale(width / img_width, height / img_height);
                cr.set_source_pixbuf(&pixbuf, 0.0, 0.0);
                cr.paint_with_alpha(alpha).ok()?;
            }
            2 => {
                // Zoom: Scale to fill (maintain aspect ratio, may crop)
                let scale = (width / img_width).max(height / img_height);
                cr.scale(scale, scale);
                cr.translate(
                    (width / scale - img_width) / 2.0,
                    (height / scale - img_height) / 2.0,
                );
                cr.set_source_pixbuf(&pixbuf, 0.0, 0.0);
                cr.paint_with_alpha(alpha).ok()?;
            }
            _ => return None,
        }

        // Evict LRU if full
        if self.cache.len() >= self.max_entries {
            if let Some(oldest_key) = self
                .cache
                .iter()
                .min_by_key(|(_, e)| e.last_access)
                .map(|(k, _)| k.clone())
            {
                self.cache.remove(&oldest_key);
            }
        }

        self.cache.insert(
            key,
            ScaledSurfaceEntry {
                surface: surface.clone(),
                last_access: Instant::now(),
            },
        );

        Some(surface)
    }
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
    /// Stops are sorted by position during LUT construction (handles unsorted input)
    pub fn from_stops(stops: &[ColorStop], resolution: usize) -> Self {
        let resolution = resolution.max(2);
        let mut colors = Vec::with_capacity(resolution);

        // Sort stops by position (only done once during LUT creation)
        let mut sorted_stops: Vec<ColorStop> = stops.to_vec();
        sorted_stops.sort_by(|a, b| {
            a.position
                .partial_cmp(&b.position)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        for i in 0..resolution {
            let t = i as f64 / (resolution - 1) as f64;
            colors.push(interpolate_color_at(&sorted_stops, t));
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

// NOTE: TextExtentsCache was removed because it used Cairo's toy font API
// which causes memory leaks. Use pango_text_extents() from pango_text.rs instead.

// Thread-local scaled font cache to prevent Cairo font memory leaks
thread_local! {
    static SCALED_FONT_CACHE: RefCell<ScaledFontCache> = RefCell::new(ScaledFontCache::new());
}

/// Cache for Cairo ScaledFont objects to prevent unbounded font memory growth
/// Cairo's select_font_face creates internal font caches that never get released.
/// By caching ScaledFont objects ourselves, we control the lifecycle and limit memory usage.
struct ScaledFontCache {
    /// Cached fonts keyed by (family, slant as u8, weight as u8, size * 10 as i32)
    cache: HashMap<(String, u8, u8, i32), (cairo::ScaledFont, Instant)>,
    max_entries: usize,
}

impl ScaledFontCache {
    fn new() -> Self {
        Self {
            cache: HashMap::new(),
            max_entries: 32, // Limit to 32 unique font configurations
        }
    }

    fn get_or_create(
        &mut self,
        family: &str,
        slant: cairo::FontSlant,
        weight: cairo::FontWeight,
        size: f64,
    ) -> Option<cairo::ScaledFont> {
        let slant_u8 = match slant {
            cairo::FontSlant::Normal => 0,
            cairo::FontSlant::Italic => 1,
            cairo::FontSlant::Oblique => 2,
            _ => 0,
        };
        let weight_u8 = match weight {
            cairo::FontWeight::Normal => 0,
            cairo::FontWeight::Bold => 1,
            _ => 0,
        };
        let size_key = (size * 10.0) as i32;
        let key = (family.to_string(), slant_u8, weight_u8, size_key);

        // Check cache
        if let Some((font, access_time)) = self.cache.get_mut(&key) {
            *access_time = Instant::now();
            return Some(font.clone());
        }

        // Create new ScaledFont
        let font_face = cairo::FontFace::toy_create(family, slant, weight).ok()?;
        let matrix = cairo::Matrix::new(size, 0.0, 0.0, size, 0.0, 0.0);
        let ctm = cairo::Matrix::identity();
        let options = cairo::FontOptions::new().ok()?;
        let scaled_font = cairo::ScaledFont::new(&font_face, &matrix, &ctm, &options).ok()?;

        // Evict LRU if full
        if self.cache.len() >= self.max_entries {
            if let Some(oldest_key) = self
                .cache
                .iter()
                .min_by_key(|(_, (_, time))| *time)
                .map(|(k, _)| k.clone())
            {
                self.cache.remove(&oldest_key);
            }
        }

        self.cache
            .insert(key, (scaled_font.clone(), Instant::now()));
        Some(scaled_font)
    }
}

/// Apply a cached scaled font to a Cairo context
/// This prevents Cairo's internal font cache from growing unboundedly
pub fn apply_cached_font(
    cr: &cairo::Context,
    family: &str,
    slant: cairo::FontSlant,
    weight: cairo::FontWeight,
    size: f64,
) {
    SCALED_FONT_CACHE.with(|cache| {
        if let Some(scaled_font) = cache
            .borrow_mut()
            .get_or_create(family, slant, weight, size)
        {
            cr.set_scaled_font(&scaled_font);
        } else {
            // Fallback to toy API if ScaledFont creation fails
            cr.select_font_face(family, slant, weight);
            cr.set_font_size(size);
        }
    });
}

// Thread-local gradient LUT cache
thread_local! {
    static GRADIENT_LUT_CACHE: RefCell<GradientLUTCache> = RefCell::new(GradientLUTCache::new());
}

/// Cache for pre-computed color gradient LUTs
struct GradientLUTCache {
    /// Cached LUTs keyed by a hash of color stops (Arc to avoid cloning large Vec)
    cache: HashMap<u64, Arc<ColorGradientLUT>>,
    max_entries: usize,
}

impl GradientLUTCache {
    fn new() -> Self {
        Self {
            cache: HashMap::new(),
            max_entries: 20,
        }
    }

    fn get_or_create(&mut self, stops: &[ColorStop], resolution: usize) -> Arc<ColorGradientLUT> {
        let key = hash_color_stops(stops);

        if let Some(lut) = self.cache.get(&key) {
            return Arc::clone(lut); // Cheap Arc clone instead of Vec clone
        }

        // Evict oldest if full
        if self.cache.len() >= self.max_entries {
            if let Some(&oldest_key) = self.cache.keys().next() {
                self.cache.remove(&oldest_key);
            }
        }

        let lut = Arc::new(ColorGradientLUT::from_stops(stops, resolution));
        self.cache.insert(key, Arc::clone(&lut));
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
/// Returns Arc to avoid expensive Vec cloning on every call
fn get_gradient_lut(stops: &[ColorStop]) -> Arc<ColorGradientLUT> {
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

/// Clear all render caches to free memory
/// Call this periodically or when memory pressure is detected
pub fn clear_all_render_caches() {
    // Clear image cache
    IMAGE_CACHE.with(|cache| {
        cache.borrow_mut().pixbufs.clear();
    });

    // Clear scaled surface cache
    SCALED_SURFACE_CACHE.with(|cache| {
        cache.borrow_mut().cache.clear();
    });

    // Clear scaled font cache
    SCALED_FONT_CACHE.with(|cache| {
        cache.borrow_mut().cache.clear();
    });

    // Clear gradient LUT cache
    GRADIENT_LUT_CACHE.with(|cache| {
        cache.borrow_mut().cache.clear();
    });

    log::info!("All render caches cleared");
}

/// Get cache statistics for monitoring
pub fn get_cache_stats() -> String {
    let image_count = IMAGE_CACHE.with(|c| c.borrow().pixbufs.len());
    let surface_count = SCALED_SURFACE_CACHE.with(|c| c.borrow().cache.len());
    let font_count = SCALED_FONT_CACHE.with(|c| c.borrow().cache.len());
    let gradient_count = GRADIENT_LUT_CACHE.with(|c| c.borrow().cache.len());

    format!(
        "images={}, surfaces={}, fonts={}, gradients={}",
        image_count, surface_count, font_count, gradient_count
    )
}
