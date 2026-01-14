//! Shared font dialog using modern GTK4 FontDialog API
//!
//! Uses the non-deprecated FontDialog which handles font selection asynchronously.

use gtk4::prelude::*;
use gtk4::{FontDialog, Window};
use std::sync::atomic::{AtomicBool, Ordering};

/// Flag to track if font cache has been warmed (can be set from any thread)
static FONT_CACHE_WARMED: AtomicBool = AtomicBool::new(false);

/// Warm up the font cache by enumerating fonts via PangoCairo.
/// This can be called from a background thread - it only does I/O.
pub fn warm_font_cache() {
    if FONT_CACHE_WARMED.load(Ordering::SeqCst) {
        return;
    }

    log::debug!("Warming font cache in background...");

    // Get the default font map and list all families
    // This triggers the expensive font file scanning
    let font_map = pangocairo::FontMap::default();
    let families = font_map.list_families();

    log::debug!("Font cache warmed: {} families found", families.len());
    FONT_CACHE_WARMED.store(true, Ordering::SeqCst);
}

/// Initialize the shared font dialog on the main thread.
/// With the modern FontDialog API, this is a no-op since FontDialog
/// doesn't need pre-initialization.
pub fn init_shared_font_dialog() {
    // Modern FontDialog doesn't need pre-initialization
    // The font cache warming is the main optimization
}

/// Show the font dialog and call the callback when a font is selected.
///
/// # Arguments
/// * `parent` - The parent window for the dialog
/// * `initial_font` - Optional initial font description to show
/// * `on_selected` - Callback called with the selected font (only on OK)
pub fn show_font_dialog<F>(
    parent: Option<&Window>,
    initial_font: Option<&pango::FontDescription>,
    on_selected: F,
) where
    F: FnOnce(pango::FontDescription) + 'static,
{
    let dialog = FontDialog::new();

    // Set title
    dialog.set_title("Select Font");

    // Clone initial font for the async block
    let initial = initial_font.cloned();

    // Clone parent window reference for async block
    let parent_window = parent.cloned();

    // Spawn async font selection
    gtk4::glib::MainContext::default().spawn_local(async move {
        let result = dialog
            .choose_font_future(parent_window.as_ref(), initial.as_ref())
            .await;

        if let Ok(font_desc) = result {
            on_selected(font_desc);
        }
    });
}
