//! Shared font dialog with pre-warmed font cache
//!
//! Uses a single FontDialog instance that's created once at startup,
//! pre-loading the system font list so subsequent opens are instant.

use gtk4::FontDialog;
use std::cell::RefCell;
use std::rc::Rc;

thread_local! {
    /// Global FontDialog instance - created once, reused for all font picking.
    /// This pre-warms the font cache so opening the dialog is instant.
    static FONT_DIALOG: RefCell<Option<Rc<FontDialog>>> = const { RefCell::new(None) };
}

/// Initialize the shared font dialog.
/// Call this at app startup to pre-warm the font cache.
/// This must be called from the main GTK thread.
pub fn init_shared_font_dialog() {
    FONT_DIALOG.with(|cell| {
        let mut dialog = cell.borrow_mut();
        if dialog.is_none() {
            log::debug!("Pre-warming font dialog cache...");
            *dialog = Some(Rc::new(FontDialog::new()));
            log::debug!("Font dialog cache warmed");
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
