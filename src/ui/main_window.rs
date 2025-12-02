//! Main application window

use gtk4::{Widget, Box as GtkBox};

pub struct MainWindow {
    container: GtkBox,
}

impl MainWindow {
    pub fn new() -> Self {
        let container = GtkBox::new(gtk4::Orientation::Vertical, 0);

        // TODO: Initialize grid layout and toolbar

        Self { container }
    }

    pub fn widget(&self) -> Widget {
        self.container.clone().upcast()
    }
}

impl Default for MainWindow {
    fn default() -> Self {
        Self::new()
    }
}
