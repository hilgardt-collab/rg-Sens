//! Image picker abstraction
//!
//! This module provides a file dialog for image selection that mimics
//! the deprecated FileChooserDialog appearance and behavior.

use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, CheckButton, DropDown, Entry, Frame, HeaderBar, Label, ListBox,
    ListBoxRow, Orientation, Paned, Picture, ScrolledWindow, StringList, Window,
};
use gtk4::glib::WeakRef;
use std::path::{Path, PathBuf};
use std::fs;
use std::cell::RefCell;

thread_local! {
    static IMAGE_PICKER_DIALOG: RefCell<Option<WeakRef<Window>>> = const { RefCell::new(None) };
}

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
    pub fn pick<F>(&self, parent: Option<&Window>, callback: F)
    where
        F: Fn(Option<PathBuf>) + 'static,
    {
        let callback = std::rc::Rc::new(callback);

        let window = Window::builder()
            .title(&self.title)
            .modal(false)
            .default_width(1100)
            .default_height(650)
            .build();

        if let Some(parent) = parent {
            window.set_transient_for(Some(parent));
        }

        // Close any existing dialog (singleton pattern)
        IMAGE_PICKER_DIALOG.with(|dialog_ref| {
            let mut dialog_opt = dialog_ref.borrow_mut();
            if let Some(weak) = dialog_opt.as_ref() {
                if let Some(existing) = weak.upgrade() {
                    existing.close();
                }
            }
            // Store the new dialog
            *dialog_opt = Some(window.downgrade());
        });

        // Create header bar with buttons
        let header = HeaderBar::new();
        let cancel_button = Button::with_label("Cancel");
        let open_button = Button::with_label("Open");
        open_button.add_css_class("suggested-action");
        open_button.set_sensitive(false); // Disabled until file selected

        header.pack_start(&cancel_button);
        header.pack_end(&open_button);
        window.set_titlebar(Some(&header));

        // Main container
        let main_box = GtkBox::new(Orientation::Vertical, 0);

        // Outer paned: sidebar | (file list | preview)
        let outer_paned = Paned::new(Orientation::Horizontal);
        outer_paned.set_vexpand(true);
        outer_paned.set_position(150);

        // LEFT SIDEBAR - Places
        let sidebar = GtkBox::new(Orientation::Vertical, 0);
        sidebar.set_width_request(150);

        let sidebar_label = Label::new(Some("Places"));
        sidebar_label.set_halign(gtk4::Align::Start);
        sidebar_label.set_margin_start(6);
        sidebar_label.set_margin_top(6);
        sidebar_label.set_margin_bottom(6);
        sidebar_label.add_css_class("heading");
        sidebar.append(&sidebar_label);

        let places_list = ListBox::new();
        places_list.add_css_class("navigation-sidebar");

        let sidebar_scroll = ScrolledWindow::new();
        sidebar_scroll.set_child(Some(&places_list));
        sidebar_scroll.set_vexpand(true);
        sidebar.append(&sidebar_scroll);

        outer_paned.set_start_child(Some(&sidebar));

        // Inner paned: file list | preview
        let inner_paned = Paned::new(Orientation::Horizontal);
        inner_paned.set_position(550);
        inner_paned.set_hexpand(true);

        // RIGHT SIDE - File browser
        let right_box = GtkBox::new(Orientation::Vertical, 6);
        right_box.set_margin_start(6);
        right_box.set_margin_end(6);
        right_box.set_margin_top(6);
        right_box.set_margin_bottom(6);

        // Path navigation bar
        let nav_box = GtkBox::new(Orientation::Horizontal, 6);
        let up_button = Button::from_icon_name("go-up-symbolic");
        up_button.set_tooltip_text(Some("Go to parent folder"));
        let home_button = Button::from_icon_name("go-home-symbolic");
        home_button.set_tooltip_text(Some("Go to home folder"));

        nav_box.append(&up_button);
        nav_box.append(&home_button);

        // Current path display
        let current_dir = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
        let current_path = std::rc::Rc::new(std::cell::RefCell::new(PathBuf::from(&current_dir)));

        let path_label = Label::new(Some(&current_path.borrow().display().to_string()));
        path_label.set_halign(gtk4::Align::Start);
        path_label.set_ellipsize(pango::EllipsizeMode::Start);
        path_label.set_hexpand(true);
        path_label.set_margin_start(6);
        nav_box.append(&path_label);

        right_box.append(&nav_box);

        // File list view
        let file_listbox = ListBox::new();
        file_listbox.set_selection_mode(gtk4::SelectionMode::Single);

        let file_scroll = ScrolledWindow::new();
        file_scroll.set_child(Some(&file_listbox));
        file_scroll.set_vexpand(true);
        right_box.append(&file_scroll);

        // Bottom bar with file name and filter
        let bottom_box = GtkBox::new(Orientation::Vertical, 6);
        bottom_box.set_margin_top(6);

        // File name entry
        let name_box = GtkBox::new(Orientation::Horizontal, 6);
        let name_label = Label::new(Some("File name:"));
        name_label.set_width_request(100);
        name_label.set_halign(gtk4::Align::Start);
        let name_entry = Entry::new();
        name_entry.set_hexpand(true);
        name_box.append(&name_label);
        name_box.append(&name_entry);
        bottom_box.append(&name_box);

        // File type filter
        let filter_box = GtkBox::new(Orientation::Horizontal, 6);
        let filter_label = Label::new(Some("File type:"));
        filter_label.set_width_request(100);
        filter_label.set_halign(gtk4::Align::Start);
        let filter_options = StringList::new(&["Image Files", "All Files"]);
        let filter_combo = DropDown::new(Some(filter_options), Option::<gtk4::Expression>::None);
        filter_combo.set_selected(0);
        filter_combo.set_hexpand(true);
        filter_box.append(&filter_label);
        filter_box.append(&filter_combo);
        bottom_box.append(&filter_box);

        // Show hidden files checkbox
        let hidden_files_check = CheckButton::with_label("Show hidden files");
        hidden_files_check.set_margin_top(6);
        bottom_box.append(&hidden_files_check);

        right_box.append(&bottom_box);
        inner_paned.set_start_child(Some(&right_box));

        // PREVIEW PANEL
        let preview_box = GtkBox::new(Orientation::Vertical, 6);
        preview_box.set_width_request(250);
        preview_box.set_margin_start(6);
        preview_box.set_margin_end(6);
        preview_box.set_margin_top(6);
        preview_box.set_margin_bottom(6);

        let preview_label = Label::new(Some("Preview"));
        preview_label.set_halign(gtk4::Align::Start);
        preview_label.add_css_class("heading");
        preview_box.append(&preview_label);

        // Preview frame
        let preview_frame = Frame::new(None);
        preview_frame.set_vexpand(true);

        let preview_content = GtkBox::new(Orientation::Vertical, 0);
        preview_content.set_valign(gtk4::Align::Center);
        preview_content.set_halign(gtk4::Align::Center);

        let preview_picture = Picture::new();
        preview_picture.set_can_shrink(true);
        preview_picture.set_content_fit(gtk4::ContentFit::Contain);
        preview_picture.set_vexpand(true);
        preview_picture.set_hexpand(true);

        preview_content.append(&preview_picture);
        preview_frame.set_child(Some(&preview_content));
        preview_box.append(&preview_frame);

        // Preview info labels
        let info_box = GtkBox::new(Orientation::Vertical, 3);
        info_box.set_margin_top(6);

        let preview_name_label = Label::new(None);
        preview_name_label.set_halign(gtk4::Align::Start);
        preview_name_label.set_ellipsize(pango::EllipsizeMode::Middle);
        preview_name_label.add_css_class("caption");
        info_box.append(&preview_name_label);

        let preview_size_label = Label::new(None);
        preview_size_label.set_halign(gtk4::Align::Start);
        preview_size_label.add_css_class("caption");
        preview_size_label.add_css_class("dim-label");
        info_box.append(&preview_size_label);

        preview_box.append(&info_box);

        inner_paned.set_end_child(Some(&preview_box));
        outer_paned.set_end_child(Some(&inner_paned));

        main_box.append(&outer_paned);
        window.set_child(Some(&main_box));

        // Selected file storage
        let selected_file = std::rc::Rc::new(std::cell::RefCell::new(None::<PathBuf>));

        // Populate sidebar places
        let places_data = get_sidebar_places();
        for (name, path) in &places_data {
            let row = create_sidebar_row(name, path);
            places_list.append(&row);
        }

        // Function to load directory contents
        let load_directory = {
            let file_listbox = file_listbox.clone();
            let current_path = current_path.clone();
            let path_label = path_label.clone();
            let selected_file = selected_file.clone();
            let filter_combo = filter_combo.clone();
            let hidden_files_check = hidden_files_check.clone();

            move || {
                // Clear existing items
                while let Some(child) = file_listbox.first_child() {
                    file_listbox.remove(&child);
                }

                let path = current_path.borrow().clone();
                path_label.set_text(&path.display().to_string());

                // Read directory
                if let Ok(entries) = fs::read_dir(&path) {
                    let mut all_entries: Vec<PathBuf> = entries
                        .filter_map(|e| e.ok())
                        .map(|e| e.path())
                        .collect();

                    all_entries.sort();

                    let show_all = filter_combo.selected() == 1;
                    let show_hidden = hidden_files_check.is_active();

                    for entry_path in all_entries {
                        // Check if it's a hidden file/directory
                        let is_hidden = entry_path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .map(|n| n.starts_with('.'))
                            .unwrap_or(false);

                        // Skip hidden files unless checkbox is checked
                        if is_hidden && !show_hidden {
                            continue;
                        }

                        if entry_path.is_dir() {
                            // Show directories
                            let row = create_file_row(&entry_path, true);
                            file_listbox.append(&row);
                        } else if show_all || is_image_file(&entry_path) {
                            // Show files based on filter
                            let row = create_file_row(&entry_path, false);
                            file_listbox.append(&row);
                        }
                    }
                }

                // Clear selection
                *selected_file.borrow_mut() = None;
                file_listbox.unselect_all();
            }
        };

        // Initial load
        load_directory();

        // Function to update preview
        let update_preview = {
            let preview_picture = preview_picture.clone();
            let preview_name_label = preview_name_label.clone();
            let preview_size_label = preview_size_label.clone();

            move |path: Option<&PathBuf>| {
                if let Some(path) = path {
                    if path.is_file() && is_image_file(path) {
                        // Load image into preview
                        let file = gtk4::gio::File::for_path(path);
                        preview_picture.set_file(Some(&file));

                        // Show file name
                        if let Some(name) = path.file_name() {
                            preview_name_label.set_text(&name.to_string_lossy());
                        }

                        // Show file size
                        if let Ok(metadata) = std::fs::metadata(path) {
                            let size = metadata.len();
                            let size_str = format_file_size(size);
                            preview_size_label.set_text(&size_str);
                        }
                    } else {
                        // Clear preview for non-image files
                        preview_picture.set_file(gtk4::gio::File::NONE);
                        preview_name_label.set_text("");
                        preview_size_label.set_text("");
                    }
                } else {
                    // Clear preview
                    preview_picture.set_file(gtk4::gio::File::NONE);
                    preview_name_label.set_text("");
                    preview_size_label.set_text("");
                }
            }
        };

        // Handle file list double-click
        let selected_file_clone = selected_file.clone();
        let name_entry_clone = name_entry.clone();
        let open_button_clone = open_button.clone();
        let current_path_clone = current_path.clone();
        let load_directory_clone = load_directory.clone();
        let update_preview_clone = update_preview.clone();

        file_listbox.connect_row_activated(move |_, row| {
            unsafe {
                if let Some(path) = row.data::<PathBuf>("file-path") {
                    let path = path.as_ref();
                    if path.is_dir() {
                        // Navigate into directory
                        *current_path_clone.borrow_mut() = path.clone();
                        load_directory_clone();
                        update_preview_clone(None);
                    } else {
                        // Select file
                        *selected_file_clone.borrow_mut() = Some(path.clone());
                        if let Some(name) = path.file_name() {
                            name_entry_clone.set_text(&name.to_string_lossy());
                        }
                        open_button_clone.set_sensitive(true);
                        update_preview_clone(Some(path));
                    }
                }
            }
        });

        // Handle file list single click selection
        let selected_file_clone = selected_file.clone();
        let name_entry_clone = name_entry.clone();
        let open_button_clone = open_button.clone();
        let update_preview_clone = update_preview.clone();

        file_listbox.connect_row_selected(move |_, row| {
            if let Some(row) = row {
                unsafe {
                    if let Some(path) = row.data::<PathBuf>("file-path") {
                        let path = path.as_ref();
                        if !path.is_dir() {
                            *selected_file_clone.borrow_mut() = Some(path.clone());
                            if let Some(name) = path.file_name() {
                                name_entry_clone.set_text(&name.to_string_lossy());
                            }
                            open_button_clone.set_sensitive(true);
                            update_preview_clone(Some(path));
                        } else {
                            update_preview_clone(None);
                        }
                    }
                }
            } else {
                update_preview_clone(None);
            }
        });

        // Handle sidebar places selection
        let current_path_clone = current_path.clone();
        let load_directory_clone = load_directory.clone();
        places_list.connect_row_activated(move |_, row| {
            unsafe {
                if let Some(path) = row.data::<PathBuf>("place-path") {
                    *current_path_clone.borrow_mut() = path.as_ref().clone();
                    load_directory_clone();
                }
            }
        });

        // Up button handler
        let current_path_clone = current_path.clone();
        let load_directory_clone = load_directory.clone();
        up_button.connect_clicked(move |_| {
            let mut path = current_path_clone.borrow_mut();
            if let Some(parent) = path.parent() {
                *path = parent.to_path_buf();
                drop(path);
                load_directory_clone();
            }
        });

        // Home button handler
        let current_path_clone = current_path.clone();
        let load_directory_clone = load_directory.clone();
        home_button.connect_clicked(move |_| {
            if let Ok(home) = std::env::var("HOME") {
                *current_path_clone.borrow_mut() = PathBuf::from(home);
                load_directory_clone();
            }
        });

        // Filter change handler
        let load_directory_clone = load_directory.clone();
        filter_combo.connect_selected_notify(move |_| {
            load_directory_clone();
        });

        // Hidden files checkbox handler
        let load_directory_clone = load_directory.clone();
        hidden_files_check.connect_toggled(move |_| {
            load_directory_clone();
        });

        // Name entry change handler
        let selected_file_clone = selected_file.clone();
        let open_button_clone = open_button.clone();
        let current_path_clone = current_path.clone();
        name_entry.connect_changed(move |entry| {
            let text = entry.text();
            if !text.is_empty() {
                let path = current_path_clone.borrow().join(text.as_str());
                if path.exists() && path.is_file() {
                    *selected_file_clone.borrow_mut() = Some(path);
                    open_button_clone.set_sensitive(true);
                } else {
                    open_button_clone.set_sensitive(false);
                }
            } else {
                open_button_clone.set_sensitive(false);
            }
        });

        // Cancel button
        let window_clone = window.clone();
        let callback_clone = callback.clone();
        cancel_button.connect_clicked(move |_| {
            callback_clone(None);
            window_clone.close();
        });

        // Open button
        let window_clone = window.clone();
        let selected_file_clone = selected_file.clone();
        let callback_clone = callback.clone();
        open_button.connect_clicked(move |_| {
            let result = selected_file_clone.borrow().clone();
            callback_clone(result);
            window_clone.close();
        });

        // Clear singleton reference when window closes
        window.connect_close_request(move |_| {
            IMAGE_PICKER_DIALOG.with(|dialog_ref| {
                *dialog_ref.borrow_mut() = None;
            });
            gtk4::glib::Propagation::Proceed
        });

        window.present();
    }
}

/// Get sidebar places (common locations)
fn get_sidebar_places() -> Vec<(String, PathBuf)> {
    let mut places = Vec::new();

    if let Ok(home) = std::env::var("HOME") {
        let home_path = PathBuf::from(&home);
        places.push(("Home".to_string(), home_path.clone()));
        places.push(("Pictures".to_string(), home_path.join("Pictures")));
        places.push(("Downloads".to_string(), home_path.join("Downloads")));
        places.push(("Documents".to_string(), home_path.join("Documents")));
    }

    places.push(("Root".to_string(), PathBuf::from("/")));

    places
}

/// Create a sidebar row for a place
fn create_sidebar_row(name: &str, path: &Path) -> ListBoxRow {
    let row = ListBoxRow::new();
    let label = Label::new(Some(name));
    label.set_halign(gtk4::Align::Start);
    label.set_margin_start(6);
    label.set_margin_end(6);
    label.set_margin_top(3);
    label.set_margin_bottom(3);
    row.set_child(Some(&label));

    unsafe {
        row.set_data("place-path", path.to_path_buf());
    }

    row
}

/// Create a file list row
fn create_file_row(path: &Path, is_dir: bool) -> ListBoxRow {
    let row = ListBoxRow::new();
    let hbox = GtkBox::new(Orientation::Horizontal, 6);
    hbox.set_margin_start(6);
    hbox.set_margin_end(6);
    hbox.set_margin_top(3);
    hbox.set_margin_bottom(3);

    // Icon
    let icon = if is_dir { "folder-symbolic" } else { "text-x-generic-symbolic" };
    let icon_image = gtk4::Image::from_icon_name(icon);
    hbox.append(&icon_image);

    // File name
    if let Some(name) = path.file_name() {
        let label = Label::new(Some(&name.to_string_lossy()));
        label.set_halign(gtk4::Align::Start);
        label.set_hexpand(true);
        hbox.append(&label);
    }

    row.set_child(Some(&hbox));

    unsafe {
        row.set_data("file-path", path.to_path_buf());
    }

    row
}

/// Check if a path is an image file based on extension
fn is_image_file(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }

    let image_extensions = [
        "png", "jpg", "jpeg", "gif", "bmp", "webp", "svg", "tiff", "tif", "ico",
    ];

    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| image_extensions.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// Format file size in human-readable format
fn format_file_size(size: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size_f = size as f64;
    let mut unit_idx = 0;

    while size_f >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size_f /= 1024.0;
        unit_idx += 1;
    }

    if unit_idx == 0 {
        format!("{} {}", size, UNITS[unit_idx])
    } else {
        format!("{:.2} {}", size_f, UNITS[unit_idx])
    }
}
