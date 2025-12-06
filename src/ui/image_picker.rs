//! Image picker abstraction
//!
//! This module provides an abstraction for image file selection.
//! Currently uses FileChooserDialog but can be replaced with a custom
//! implementation in the future.

use gtk4::prelude::*;
use std::path::PathBuf;

/// Image picker for selecting image files
pub struct ImagePicker {
    title: String,
}

impl ImagePicker {
    /// Create a new image picker with a title
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
        }
    }

    /// Show the picker and call the callback with the selected file path
    ///
    /// # Arguments
    /// * `parent` - Optional parent window
    /// * `callback` - Callback to invoke with the selected path (or None if cancelled)
    pub fn pick<F>(&self, parent: Option<&gtk4::Window>, callback: F)
    where
        F: Fn(Option<PathBuf>) + 'static,
    {
        // Modern GTK4 FileDialog implementation
        use gtk4::FileDialog;

        let dialog = FileDialog::builder()
            .title(&self.title)
            .modal(true)
            .build();

        // Add image file filter
        let filter = gtk4::FileFilter::new();
        filter.set_name(Some("Image Files"));
        filter.add_mime_type("image/png");
        filter.add_mime_type("image/jpeg");
        filter.add_mime_type("image/jpg");
        filter.add_mime_type("image/gif");
        filter.add_mime_type("image/bmp");
        filter.add_mime_type("image/webp");
        filter.add_mime_type("image/svg+xml");
        filter.add_pattern("*.png");
        filter.add_pattern("*.jpg");
        filter.add_pattern("*.jpeg");
        filter.add_pattern("*.gif");
        filter.add_pattern("*.bmp");
        filter.add_pattern("*.webp");
        filter.add_pattern("*.svg");

        // Create filter list
        let filters = gtk4::gio::ListStore::new::<gtk4::FileFilter>();
        filters.append(&filter);

        // All files filter
        let all_filter = gtk4::FileFilter::new();
        all_filter.set_name(Some("All Files"));
        all_filter.add_pattern("*");
        filters.append(&all_filter);

        dialog.set_filters(Some(&filters));
        dialog.set_default_filter(Some(&filter));

        dialog.open(parent, gtk4::gio::Cancellable::NONE, move |result| {
            let path = result.ok().and_then(|file| file.path());
            callback(path);
        });
    }
}
