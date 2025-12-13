//! Shared font dialog singleton

use gtk4::FontDialog;
use std::cell::RefCell;
use std::rc::Rc;

thread_local! {
    static FONT_DIALOG: RefCell<Option<Rc<FontDialog>>> = const { RefCell::new(None) };
}

/// Get the shared font dialog instance (thread-local for GTK thread safety)
pub fn shared_font_dialog() -> Rc<FontDialog> {
    FONT_DIALOG.with(|dialog| {
        let mut dialog_ref = dialog.borrow_mut();
        if dialog_ref.is_none() {
            *dialog_ref = Some(Rc::new(FontDialog::new()));
        }
        dialog_ref.as_ref().unwrap().clone()
    })
}
