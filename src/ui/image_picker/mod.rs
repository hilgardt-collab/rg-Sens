//! Image picker with sortable list view and icon view
//!
//! This module provides a file dialog for image selection with:
//! - Sortable column view (Name, Size, Type, Modified)
//! - Icon/thumbnail grid view
//! - Toggle between list and icon views
//! - Sort dropdown for sorting options
//! - Directories always appear first

mod file_entry;

use file_entry::FileEntry;
use gtk4::gio;
use gtk4::glib;
use gtk4::glib::WeakRef;
use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, CheckButton, ColumnView, ColumnViewColumn, CustomSorter, DropDown,
    Entry, Frame, GridView, HeaderBar, Label, ListBox, ListBoxRow, Orientation, Paned, Picture,
    ScrolledWindow, SignalListItemFactory, SingleSelection, SortListModel, SorterChange,
    Stack, StringList, ToggleButton, Window,
};
use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

thread_local! {
    static IMAGE_PICKER_DIALOG: RefCell<Option<WeakRef<Window>>> = const { RefCell::new(None) };
}

/// Close the image picker dialog if it's open
pub fn close_image_picker_dialog() {
    IMAGE_PICKER_DIALOG.with(|dialog_ref| {
        let mut dialog_opt = dialog_ref.borrow_mut();
        if let Some(weak) = dialog_opt.take() {
            if let Some(dialog) = weak.upgrade() {
                dialog.close();
            }
        }
    });
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
    pub fn pick<F>(&self, parent: Option<&Window>, callback: F)
    where
        F: Fn(Option<PathBuf>) + 'static,
    {
        let callback = Rc::new(callback);

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
            *dialog_opt = Some(window.downgrade());
        });

        // Create header bar with buttons
        let header = HeaderBar::new();
        let cancel_button = Button::with_label("Cancel");
        let open_button = Button::with_label("Open");
        open_button.add_css_class("suggested-action");
        open_button.set_sensitive(false);

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
        let current_path = Rc::new(RefCell::new(PathBuf::from(&current_dir)));

        let path_label = Label::new(Some(&current_path.borrow().display().to_string()));
        path_label.set_halign(gtk4::Align::Start);
        path_label.set_ellipsize(pango::EllipsizeMode::Start);
        path_label.set_hexpand(true);
        path_label.set_margin_start(6);
        nav_box.append(&path_label);

        // Sort dropdown
        let sort_options = StringList::new(&["Name", "Size", "Type", "Modified"]);
        let sort_combo = DropDown::new(Some(sort_options), Option::<gtk4::Expression>::None);
        sort_combo.set_selected(0);
        sort_combo.set_tooltip_text(Some("Sort by"));
        nav_box.append(&sort_combo);

        // Sort direction button
        let sort_dir_button = Button::from_icon_name("view-sort-ascending-symbolic");
        sort_dir_button.set_tooltip_text(Some("Sort direction"));
        nav_box.append(&sort_dir_button);

        // View toggle buttons (list/grid)
        let view_toggle_box = GtkBox::new(Orientation::Horizontal, 0);
        view_toggle_box.add_css_class("linked");

        let list_view_button = ToggleButton::new();
        list_view_button.set_icon_name("view-list-symbolic");
        list_view_button.set_tooltip_text(Some("List view"));
        list_view_button.set_active(true);

        let grid_view_button = ToggleButton::new();
        grid_view_button.set_icon_name("view-grid-symbolic");
        grid_view_button.set_tooltip_text(Some("Icon view"));
        grid_view_button.set_group(Some(&list_view_button));

        view_toggle_box.append(&list_view_button);
        view_toggle_box.append(&grid_view_button);
        nav_box.append(&view_toggle_box);

        right_box.append(&nav_box);

        // Create the data model
        let file_store = gio::ListStore::new::<FileEntry>();

        // Create sort state
        let current_sort_column = Rc::new(RefCell::new("name".to_string()));
        let sort_ascending = Rc::new(RefCell::new(true));

        // Create custom sorter
        let sorter = create_file_sorter(&current_sort_column, &sort_ascending);
        let sort_model = SortListModel::new(Some(file_store.clone()), Some(sorter.clone()));

        // Create selection model
        let selection_model = SingleSelection::new(Some(sort_model.clone()));

        // Create ColumnView
        let column_view = ColumnView::new(Some(selection_model.clone()));
        column_view.set_show_column_separators(true);
        column_view.set_show_row_separators(true);

        // Create columns
        let name_column = create_name_column();
        let size_column = create_text_column("Size", "size-display", 80);
        let type_column = create_text_column("Type", "file-type", 80);
        let modified_column = create_text_column("Modified", "modified-display", 130);

        column_view.append_column(&name_column);
        column_view.append_column(&size_column);
        column_view.append_column(&type_column);
        column_view.append_column(&modified_column);

        // Create GridView for icon view
        let grid_view = create_grid_view(selection_model.clone());

        // Create view stack
        let view_stack = Stack::new();
        view_stack.set_vexpand(true);

        let list_scroll = ScrolledWindow::new();
        list_scroll.set_child(Some(&column_view));
        view_stack.add_named(&list_scroll, Some("list"));

        let grid_scroll = ScrolledWindow::new();
        grid_scroll.set_child(Some(&grid_view));
        view_stack.add_named(&grid_scroll, Some("grid"));

        right_box.append(&view_stack);

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
        let selected_file = Rc::new(RefCell::new(None::<PathBuf>));

        // Populate sidebar places
        let places_data = get_sidebar_places();
        for (name, path) in &places_data {
            let row = create_sidebar_row(name, path);
            places_list.append(&row);
        }

        // Function to load directory contents
        let load_directory = {
            let file_store = file_store.clone();
            let current_path = current_path.clone();
            let path_label = path_label.clone();
            let selected_file = selected_file.clone();
            let filter_combo = filter_combo.clone();
            let hidden_files_check = hidden_files_check.clone();

            move || {
                file_store.remove_all();

                let path = current_path.borrow().clone();
                path_label.set_text(&path.display().to_string());

                if let Ok(entries) = fs::read_dir(&path) {
                    let show_all = filter_combo.selected() == 1;
                    let show_hidden = hidden_files_check.is_active();

                    for entry in entries.filter_map(|e| e.ok()) {
                        let entry_path = entry.path();

                        let is_hidden = entry_path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .map(|n| n.starts_with('.'))
                            .unwrap_or(false);

                        if is_hidden && !show_hidden {
                            continue;
                        }

                        if entry_path.is_dir() || show_all || is_image_file(&entry_path) {
                            let file_entry = FileEntry::new(&entry_path);
                            file_store.append(&file_entry);
                        }
                    }
                }

                *selected_file.borrow_mut() = None;
            }
        };

        // Initial load
        load_directory();

        // Function to update preview
        let update_preview = {
            let preview_picture = preview_picture.clone();
            let preview_name_label = preview_name_label.clone();
            let preview_size_label = preview_size_label.clone();

            move |entry: Option<&FileEntry>| {
                if let Some(entry) = entry {
                    if !entry.is_dir() && is_image_file(&entry.path_buf()) {
                        let file = gio::File::for_path(entry.path_buf());
                        preview_picture.set_file(Some(&file));
                        preview_name_label.set_text(&entry.name());
                        preview_size_label.set_text(&entry.size_display());
                    } else {
                        preview_picture.set_file(gio::File::NONE);
                        preview_name_label.set_text("");
                        preview_size_label.set_text("");
                    }
                } else {
                    preview_picture.set_file(gio::File::NONE);
                    preview_name_label.set_text("");
                    preview_size_label.set_text("");
                }
            }
        };

        // Sort dropdown handler
        let current_sort_column_clone = current_sort_column.clone();
        let sorter_clone = sorter.clone();
        sort_combo.connect_selected_notify(move |combo| {
            let col_name = match combo.selected() {
                0 => "name",
                1 => "size",
                2 => "file-type",
                3 => "modified",
                _ => "name",
            };
            *current_sort_column_clone.borrow_mut() = col_name.to_string();
            sorter_clone.changed(SorterChange::Different);
        });

        // Sort direction button handler
        let sort_ascending_clone = sort_ascending.clone();
        let sorter_clone = sorter.clone();
        let sort_dir_button_clone = sort_dir_button.clone();
        sort_dir_button.connect_clicked(move |_| {
            let mut asc = sort_ascending_clone.borrow_mut();
            *asc = !*asc;
            if *asc {
                sort_dir_button_clone.set_icon_name("view-sort-ascending-symbolic");
            } else {
                sort_dir_button_clone.set_icon_name("view-sort-descending-symbolic");
            }
            drop(asc);
            sorter_clone.changed(SorterChange::Different);
        });

        // Handle selection changes
        let selected_file_clone = selected_file.clone();
        let name_entry_clone = name_entry.clone();
        let open_button_clone = open_button.clone();
        let update_preview_clone = update_preview.clone();

        selection_model.connect_selection_changed(move |model, _, _| {
            if let Some(item) = model.selected_item().and_downcast::<FileEntry>() {
                if !item.is_dir() {
                    *selected_file_clone.borrow_mut() = Some(item.path_buf());
                    name_entry_clone.set_text(&item.name());
                    open_button_clone.set_sensitive(true);
                    update_preview_clone(Some(&item));
                } else {
                    update_preview_clone(None);
                }
            }
        });

        // Handle double-click activation on ColumnView
        let current_path_clone = current_path.clone();
        let load_directory_clone = load_directory.clone();
        let update_preview_clone = update_preview.clone();
        let selected_file_clone = selected_file.clone();
        let name_entry_clone = name_entry.clone();
        let open_button_clone = open_button.clone();
        let window_clone = window.clone();
        let callback_clone = callback.clone();
        let selection_model_clone = selection_model.clone();

        column_view.connect_activate(move |_, position| {
            if let Some(item) = selection_model_clone
                .item(position)
                .and_downcast::<FileEntry>()
            {
                if item.is_dir() {
                    *current_path_clone.borrow_mut() = item.path_buf();
                    load_directory_clone();
                    update_preview_clone(None);
                } else {
                    // Double-click on file - open it
                    *selected_file_clone.borrow_mut() = Some(item.path_buf());
                    name_entry_clone.set_text(&item.name());
                    open_button_clone.set_sensitive(true);
                    callback_clone(Some(item.path_buf()));
                    window_clone.close();
                }
            }
        });

        // Handle double-click activation on GridView
        let current_path_clone = current_path.clone();
        let load_directory_clone = load_directory.clone();
        let update_preview_clone = update_preview.clone();
        let selected_file_clone = selected_file.clone();
        let name_entry_clone = name_entry.clone();
        let open_button_clone = open_button.clone();
        let window_clone = window.clone();
        let callback_clone = callback.clone();
        let selection_model_clone = selection_model.clone();

        grid_view.connect_activate(move |_, position| {
            if let Some(item) = selection_model_clone
                .item(position)
                .and_downcast::<FileEntry>()
            {
                if item.is_dir() {
                    *current_path_clone.borrow_mut() = item.path_buf();
                    load_directory_clone();
                    update_preview_clone(None);
                } else {
                    *selected_file_clone.borrow_mut() = Some(item.path_buf());
                    name_entry_clone.set_text(&item.name());
                    open_button_clone.set_sensitive(true);
                    callback_clone(Some(item.path_buf()));
                    window_clone.close();
                }
            }
        });

        // View toggle handlers
        let view_stack_clone = view_stack.clone();
        list_view_button.connect_toggled(move |btn| {
            if btn.is_active() {
                view_stack_clone.set_visible_child_name("list");
            }
        });

        let view_stack_clone = view_stack.clone();
        grid_view_button.connect_toggled(move |btn| {
            if btn.is_active() {
                view_stack_clone.set_visible_child_name("grid");
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
            glib::Propagation::Proceed
        });

        window.present();
    }
}

/// Create a custom sorter that sorts directories first, then by the specified column
fn create_file_sorter(
    current_column: &Rc<RefCell<String>>,
    ascending: &Rc<RefCell<bool>>,
) -> CustomSorter {
    let column = current_column.clone();
    let asc = ascending.clone();

    CustomSorter::new(move |obj1, obj2| {
        let entry1 = obj1.downcast_ref::<FileEntry>().unwrap();
        let entry2 = obj2.downcast_ref::<FileEntry>().unwrap();

        // Directories always come first
        match (entry1.is_dir(), entry2.is_dir()) {
            (true, false) => return gtk4::Ordering::Smaller,
            (false, true) => return gtk4::Ordering::Larger,
            _ => {}
        }

        let column_name = column.borrow();
        let cmp = match column_name.as_str() {
            "name" => entry1
                .name()
                .to_lowercase()
                .cmp(&entry2.name().to_lowercase()),
            "size" => entry1.size().cmp(&entry2.size()),
            "file-type" => entry1.file_type().cmp(&entry2.file_type()),
            "modified" => entry1.modified().cmp(&entry2.modified()),
            _ => std::cmp::Ordering::Equal,
        };

        let result = if *asc.borrow() { cmp } else { cmp.reverse() };

        match result {
            std::cmp::Ordering::Less => gtk4::Ordering::Smaller,
            std::cmp::Ordering::Equal => gtk4::Ordering::Equal,
            std::cmp::Ordering::Greater => gtk4::Ordering::Larger,
        }
    })
}

/// Create the Name column with icon and label
fn create_name_column() -> ColumnViewColumn {
    let factory = SignalListItemFactory::new();

    factory.connect_setup(|_, list_item| {
        let list_item = list_item.downcast_ref::<gtk4::ListItem>().unwrap();
        let hbox = GtkBox::new(Orientation::Horizontal, 6);
        hbox.set_margin_start(4);
        hbox.set_margin_end(4);
        hbox.set_margin_top(2);
        hbox.set_margin_bottom(2);

        let icon = gtk4::Image::new();
        icon.set_icon_size(gtk4::IconSize::Normal);
        let label = Label::new(None);
        label.set_halign(gtk4::Align::Start);
        label.set_hexpand(true);
        label.set_ellipsize(pango::EllipsizeMode::End);

        hbox.append(&icon);
        hbox.append(&label);
        list_item.set_child(Some(&hbox));
    });

    factory.connect_bind(|_, list_item| {
        let list_item = list_item.downcast_ref::<gtk4::ListItem>().unwrap();
        let item = list_item.item().and_downcast::<FileEntry>().unwrap();
        let hbox = list_item.child().and_downcast::<GtkBox>().unwrap();

        if let Some(icon) = hbox.first_child().and_downcast::<gtk4::Image>() {
            icon.set_icon_name(Some(&item.icon_name()));
        }
        if let Some(label) = hbox
            .first_child()
            .and_then(|w| w.next_sibling())
            .and_downcast::<Label>()
        {
            label.set_text(&item.name());
        }
    });

    let column = ColumnViewColumn::new(Some("Name"), Some(factory));
    column.set_expand(true);
    column.set_resizable(true);
    column
}

/// Create a text column for Size, Type, or Modified
fn create_text_column(title: &str, property: &str, width: i32) -> ColumnViewColumn {
    let factory = SignalListItemFactory::new();
    let prop = property.to_string();

    factory.connect_setup(|_, list_item| {
        let list_item = list_item.downcast_ref::<gtk4::ListItem>().unwrap();
        let label = Label::new(None);
        label.set_halign(gtk4::Align::Start);
        label.set_margin_start(4);
        label.set_margin_end(4);
        label.set_margin_top(2);
        label.set_margin_bottom(2);
        list_item.set_child(Some(&label));
    });

    factory.connect_bind(move |_, list_item| {
        let list_item = list_item.downcast_ref::<gtk4::ListItem>().unwrap();
        let item = list_item.item().and_downcast::<FileEntry>().unwrap();
        let label = list_item.child().and_downcast::<Label>().unwrap();

        let text = match prop.as_str() {
            "size-display" => item.size_display(),
            "file-type" => item.file_type(),
            "modified-display" => item.modified_display(),
            _ => String::new(),
        };
        label.set_text(&text);
    });

    let column = ColumnViewColumn::new(Some(title), Some(factory));
    column.set_fixed_width(width);
    column.set_resizable(true);
    column
}

/// Create the GridView for icon/thumbnail view
fn create_grid_view(selection_model: SingleSelection) -> GridView {
    let factory = SignalListItemFactory::new();

    factory.connect_setup(|_, list_item| {
        let list_item = list_item.downcast_ref::<gtk4::ListItem>().unwrap();
        let vbox = GtkBox::new(Orientation::Vertical, 4);
        vbox.set_halign(gtk4::Align::Center);
        vbox.set_valign(gtk4::Align::Center);
        vbox.set_width_request(100);
        vbox.set_margin_start(8);
        vbox.set_margin_end(8);
        vbox.set_margin_top(8);
        vbox.set_margin_bottom(8);

        let icon = gtk4::Image::new();
        icon.set_pixel_size(64);
        icon.set_valign(gtk4::Align::Center);

        let label = Label::new(None);
        label.set_max_width_chars(12);
        label.set_ellipsize(pango::EllipsizeMode::Middle);
        label.set_justify(gtk4::Justification::Center);
        label.set_wrap(true);
        label.set_wrap_mode(pango::WrapMode::WordChar);
        label.set_lines(2);

        vbox.append(&icon);
        vbox.append(&label);
        list_item.set_child(Some(&vbox));
    });

    factory.connect_bind(|_, list_item| {
        let list_item = list_item.downcast_ref::<gtk4::ListItem>().unwrap();
        let item = list_item.item().and_downcast::<FileEntry>().unwrap();
        let vbox = list_item.child().and_downcast::<GtkBox>().unwrap();

        if let Some(icon) = vbox.first_child().and_downcast::<gtk4::Image>() {
            // For images, try to load thumbnail
            let path = item.path_buf();
            if !item.is_dir() && is_image_file(&path) {
                // Try to load as thumbnail
                if let Ok(pixbuf) =
                    gtk4::gdk_pixbuf::Pixbuf::from_file_at_scale(&path, 64, 64, true)
                {
                    let texture = gtk4::gdk::Texture::for_pixbuf(&pixbuf);
                    icon.set_paintable(Some(&texture));
                } else {
                    icon.set_icon_name(Some(&item.icon_name()));
                }
            } else {
                icon.set_icon_name(Some(&item.icon_name()));
            }
        }

        if let Some(label) = vbox
            .first_child()
            .and_then(|w| w.next_sibling())
            .and_downcast::<Label>()
        {
            label.set_text(&item.name());
        }
    });

    let grid_view = GridView::new(Some(selection_model), Some(factory));
    grid_view.set_min_columns(3);
    grid_view.set_max_columns(10);
    grid_view
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

/// Check if a path is an image file based on extension
fn is_image_file(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }

    const IMAGE_EXTENSIONS: &[&str] = &[
        "png", "jpg", "jpeg", "gif", "bmp", "webp", "svg", "tiff", "tif", "ico",
    ];

    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| IMAGE_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}
