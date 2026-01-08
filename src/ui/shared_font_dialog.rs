//! Shared font dialog with pre-warmed font cache
//!
//! Uses a singleton FontChooserDialog that's created once and hidden/shown
//! rather than destroyed, avoiding the font enumeration on each open.

use gtk4::prelude::*;
use gtk4::{FontChooserDialog, ResponseType, Window};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};

thread_local! {
    /// Global FontChooserDialog instance - created once, hidden/shown for reuse
    static FONT_DIALOG: RefCell<Option<FontChooserDialog>> = const { RefCell::new(None) };
}

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
/// Call this after warm_font_cache() has completed.
/// MUST be called from the main GTK thread.
pub fn init_shared_font_dialog() {
    FONT_DIALOG.with(|cell| {
        let mut dialog = cell.borrow_mut();
        if dialog.is_none() {
            log::debug!("Creating shared FontChooserDialog...");

            let font_dialog = FontChooserDialog::new(Some("Select Font"), None::<&Window>);

            // Hide instead of destroy on close - this is the key optimization
            font_dialog.connect_close_request(|d| {
                d.set_visible(false);
                gtk4::glib::Propagation::Stop
            });

            *dialog = Some(font_dialog);
            log::debug!("Shared FontChooserDialog created");
        }
    });
}

/// Callback type for font selection
pub type FontSelectedCallback = Box<dyn FnOnce(pango::FontDescription) + 'static>;

/// Show the shared font dialog and call the callback when a font is selected.
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
    FONT_DIALOG.with(|cell| {
        let mut dialog_opt = cell.borrow_mut();

        // Create dialog if it doesn't exist yet
        if dialog_opt.is_none() {
            log::debug!("Creating FontChooserDialog on demand");
            let font_dialog = FontChooserDialog::new(Some("Select Font"), None::<&Window>);
            font_dialog.connect_close_request(|d| {
                d.set_visible(false);
                gtk4::glib::Propagation::Stop
            });
            *dialog_opt = Some(font_dialog);
        }

        let dialog = dialog_opt.as_ref().unwrap();

        // Set parent window
        if let Some(parent) = parent {
            dialog.set_transient_for(Some(parent));
            dialog.set_modal(true);
        }

        // Set initial font
        if let Some(font_desc) = initial_font {
            dialog.set_font_desc(font_desc);
        }

        // Store callback in Rc for the response handler
        let callback = Rc::new(RefCell::new(Some(on_selected)));
        let callback_clone = callback.clone();

        // Store handler_id so we can disconnect it after use
        let handler_id_cell: Rc<RefCell<Option<gtk4::glib::SignalHandlerId>>> = Rc::new(RefCell::new(None));
        let handler_id_for_response = handler_id_cell.clone();
        let handler_id_for_hide = handler_id_cell.clone();

        // Connect response handler (disconnect after use)
        let dialog_weak_for_response = dialog.downgrade();
        let handler_id = dialog.connect_response(move |dlg, response| {
            if response == ResponseType::Ok {
                if let Some(font_desc) = dlg.font_desc() {
                    if let Some(cb) = callback_clone.borrow_mut().take() {
                        cb(font_desc);
                    }
                }
            }
            dlg.set_visible(false);
            // Disconnect ourselves after handling
            if let Some(id) = handler_id_for_response.borrow_mut().take() {
                if let Some(d) = dialog_weak_for_response.upgrade() {
                    d.disconnect(id);
                }
            }
        });

        // Store the handler_id so it can be disconnected
        *handler_id_cell.borrow_mut() = Some(handler_id);

        // Also disconnect on hide (in case dialog is closed without response)
        let dialog_weak = dialog.downgrade();
        dialog.connect_hide(move |_| {
            if let Some(id) = handler_id_for_hide.borrow_mut().take() {
                if let Some(dlg) = dialog_weak.upgrade() {
                    dlg.disconnect(id);
                }
            }
        });

        dialog.present();
    });
}

/// Check if the font cache has been warmed
pub fn is_font_cache_warmed() -> bool {
    FONT_CACHE_WARMED.load(Ordering::SeqCst)
}
