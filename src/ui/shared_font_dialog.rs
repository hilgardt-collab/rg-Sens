//! Font dialog factory
//!
//! Creates new FontDialog instances for each usage to avoid issues
//! with GTK4's FontDialog hanging when reusing the same instance.

use gtk4::FontDialog;
use std::rc::Rc;

/// Create a new font dialog instance for each usage.
///
/// GTK4's FontDialog can hang if the same instance is reused while
/// already displaying a dialog. Creating a fresh instance avoids this.
pub fn shared_font_dialog() -> Rc<FontDialog> {
    Rc::new(FontDialog::new())
}
