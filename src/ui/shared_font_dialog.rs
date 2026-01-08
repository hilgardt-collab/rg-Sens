//! Shared font dialog with pre-warmed font cache
//!
//! Uses a single FontDialog instance and pre-loads the system font list
//! at startup so subsequent dialog opens are instant.

use gtk4::prelude::{FontFamilyExt, FontMapExt};
use gtk4::FontDialog;
use pango::FontFamily;
use std::cell::RefCell;
use std::rc::Rc;

thread_local! {
    /// Global FontDialog instance - created once, reused for all font picking.
    static FONT_DIALOG: RefCell<Option<Rc<FontDialog>>> = const { RefCell::new(None) };

    /// Cached font family names for quick access
    static FONT_FAMILIES: RefCell<Option<Vec<String>>> = const { RefCell::new(None) };
}

/// Initialize the shared font dialog and pre-load font cache.
/// Call this at app startup to ensure fonts are loaded before first use.
/// This must be called from the main GTK thread.
pub fn init_shared_font_dialog() {
    // Pre-load font families by enumerating them through PangoCairo
    // This forces the font system to scan and cache all available fonts
    FONT_FAMILIES.with(|cell| {
        let mut families = cell.borrow_mut();
        if families.is_none() {
            log::debug!("Pre-loading system fonts...");

            // Get the default font map and list all families
            // This triggers the actual font enumeration/caching
            let font_map = pangocairo::FontMap::default();
            let family_list: Vec<String> = font_map
                .list_families()
                .iter()
                .map(|f: &FontFamily| f.name().to_string())
                .collect();

            log::debug!("Loaded {} font families", family_list.len());
            *families = Some(family_list);
        }
    });

    // Also create the FontDialog instance
    FONT_DIALOG.with(|cell| {
        let mut dialog = cell.borrow_mut();
        if dialog.is_none() {
            *dialog = Some(Rc::new(FontDialog::new()));
            log::debug!("Font dialog instance created");
        }
    });
}

/// Get the shared font dialog instance.
///
/// Returns the pre-warmed FontDialog. If not yet initialized,
/// creates it on first access (but startup init is preferred).
pub fn shared_font_dialog() -> Rc<FontDialog> {
    FONT_DIALOG.with(|cell| {
        let mut dialog = cell.borrow_mut();
        if dialog.is_none() {
            // Fallback: create on first use if not pre-warmed
            log::debug!("Creating font dialog (not pre-warmed)");
            *dialog = Some(Rc::new(FontDialog::new()));
        }
        dialog.as_ref().unwrap().clone()
    })
}

/// Get the cached list of font family names.
/// Returns None if fonts haven't been pre-loaded yet.
pub fn cached_font_families() -> Option<Vec<String>> {
    FONT_FAMILIES.with(|cell| cell.borrow().clone())
}
