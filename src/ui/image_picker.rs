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
    #[allow(deprecated)]
    pub fn pick<F>(&self, parent: Option<&gtk4::Window>, callback: F)
    where
        F: Fn(Option<PathBuf>) + 'static,
    {
        // Current implementation: FileChooserDialog
        // TODO: Replace with custom image picker implementation that includes:
        //   - Image thumbnails/previews
        //   - Grid view of images
        //   - Filter by image type
        //   - Recent files
        //   - Favorites
        //   - Resizable window

        use gtk4::{FileChooserAction, FileChooserDialog, ResponseType};

        let dialog = FileChooserDialog::new(
            Some(&self.title),
            parent,
            FileChooserAction::Open,
            &[("Cancel", ResponseType::Cancel), ("Open", ResponseType::Accept)],
        );

        // Add image file filters
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
        dialog.add_filter(&filter);

        // All files filter
        let all_filter = gtk4::FileFilter::new();
        all_filter.set_name(Some("All Files"));
        all_filter.add_pattern("*");
        dialog.add_filter(&all_filter);

        dialog.connect_response(move |dialog, response| {
            let result = if response == ResponseType::Accept {
                dialog.file().and_then(|f| f.path())
            } else {
                None
            };
            callback(result);
            dialog.close();
        });

        dialog.show();
    }
}
