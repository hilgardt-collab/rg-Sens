//! Widget for configuring text displayer lines

use crate::core::FieldMetadata;
use crate::displayers::{HorizontalPosition, TextDisplayerConfig, TextLineConfig, VerticalPosition};
use crate::ui::color_picker::ColorPickerDialog;
use crate::ui::background::Color;
use crate::ui::shared_font_dialog::shared_font_dialog;
use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, CheckButton, DropDown, Entry, Frame, Label, ListBox,
    Orientation, ScrolledWindow, SpinButton, StringList, Widget,
};
use std::cell::RefCell;
use std::rc::Rc;

/// Widget for configuring text displayer lines
pub struct TextLineConfigWidget {
    widget: GtkBox,
    lines: Rc<RefCell<Vec<TextLineConfig>>>,
    list_box: ListBox,
    available_fields: Vec<FieldMetadata>,
}

impl TextLineConfigWidget {
    /// Create a new text line configuration widget
    pub fn new(available_fields: Vec<FieldMetadata>) -> Self {
        let widget = GtkBox::new(Orientation::Vertical, 6);

        // Header with add button
        let header_box = GtkBox::new(Orientation::Horizontal, 6);
        let header_label = Label::new(Some("Text Lines"));
        header_label.set_hexpand(true);
        header_label.set_halign(gtk4::Align::Start);

        let add_button = Button::with_label("+ Add Line");
        header_box.append(&header_label);
        header_box.append(&add_button);
        widget.append(&header_box);

        // Scrolled window for line list
        let scrolled = ScrolledWindow::new();
        scrolled.set_vexpand(true);
        scrolled.set_min_content_height(200);

        let list_box = ListBox::new();
        list_box.set_selection_mode(gtk4::SelectionMode::None);
        scrolled.set_child(Some(&list_box));
        widget.append(&scrolled);

        let lines = Rc::new(RefCell::new(Vec::new()));

        let lines_clone = lines.clone();
        let list_box_clone = list_box.clone();
        let fields_clone = available_fields.clone();
        add_button.connect_clicked(move |_| {
            let mut lines = lines_clone.borrow_mut();
            let new_line = TextLineConfig::default();
            let index = lines.len(); // Get index before pushing
            lines.push(new_line.clone());
            drop(lines);
            Self::add_line_row(&list_box_clone, new_line, &fields_clone, lines_clone.clone(), index, None);
        });

        Self {
            widget,
            lines,
            list_box,
            available_fields,
        }
    }

    /// Add a row for a text line
    fn add_line_row(
        list_box: &ListBox,
        line_config: TextLineConfig,
        fields: &[FieldMetadata],
        lines: Rc<RefCell<Vec<TextLineConfig>>>,
        list_index: usize,
        rebuild_callback: Option<Rc<dyn Fn()>>,
    ) {
        let row_box = GtkBox::new(Orientation::Vertical, 6);
        row_box.set_margin_top(6);
        row_box.set_margin_bottom(6);
        row_box.set_margin_start(6);
        row_box.set_margin_end(6);

        let frame = Frame::new(None);
        frame.set_child(Some(&row_box));

        // Field selector
        let field_box = GtkBox::new(Orientation::Horizontal, 6);
        field_box.append(&Label::new(Some("Field:")));

        let field_names: Vec<String> = fields.iter().map(|f| f.name.clone()).collect();
        let field_strings: Vec<&str> = field_names.iter().map(|s| s.as_str()).collect();
        let field_list = StringList::new(&field_strings);
        let field_combo = DropDown::new(Some(field_list), Option::<gtk4::Expression>::None);

        // Find selected index
        if let Some(idx) = fields.iter().position(|f| f.id == line_config.field_id) {
            field_combo.set_selected(idx as u32);
        }

        field_box.append(&field_combo);
        row_box.append(&field_box);

        // Position controls
        let pos_box = GtkBox::new(Orientation::Horizontal, 6);
        pos_box.append(&Label::new(Some("Position:")));

        // Vertical position
        let v_pos_list = StringList::new(&["Top", "Center", "Bottom"]);
        let v_pos_combo = DropDown::new(Some(v_pos_list), Option::<gtk4::Expression>::None);
        v_pos_combo.set_selected(match line_config.vertical_position {
            VerticalPosition::Top => 0,
            VerticalPosition::Center => 1,
            VerticalPosition::Bottom => 2,
        });
        pos_box.append(&v_pos_combo);

        // Horizontal position
        let h_pos_list = StringList::new(&["Left", "Center", "Right"]);
        let h_pos_combo = DropDown::new(Some(h_pos_list), Option::<gtk4::Expression>::None);
        h_pos_combo.set_selected(match line_config.horizontal_position {
            HorizontalPosition::Left => 0,
            HorizontalPosition::Center => 1,
            HorizontalPosition::Right => 2,
        });
        pos_box.append(&h_pos_combo);
        row_box.append(&pos_box);

        // Font controls
        let font_box = GtkBox::new(Orientation::Horizontal, 6);
        font_box.append(&Label::new(Some("Font:")));

        // Font selection button
        let font_button = Button::with_label(&format!("{} {:.0}", line_config.font_family, line_config.font_size));
        font_button.set_hexpand(true);

        font_box.append(&font_button);

        // Copy font button
        let copy_font_btn = Button::with_label("Copy");
        let lines_clone_copy_font = lines.clone();
        copy_font_btn.connect_clicked(move |_| {
            let lines_ref = lines_clone_copy_font.borrow();
            if let Some(line) = lines_ref.get(list_index) {
                if let Ok(mut clipboard) = crate::ui::clipboard::CLIPBOARD.lock() {
                    clipboard.copy_font(line.font_family.clone(), line.font_size);
                }
            }
        });
        font_box.append(&copy_font_btn);

        // Paste font button
        let paste_font_btn = Button::with_label("Paste");
        let lines_clone_paste_font = lines.clone();
        let font_button_clone = font_button.clone();
        paste_font_btn.connect_clicked(move |_| {
            if let Ok(clipboard) = crate::ui::clipboard::CLIPBOARD.lock() {
                if let Some((family, size)) = clipboard.paste_font() {
                    let mut lines_ref = lines_clone_paste_font.borrow_mut();
                    if let Some(line) = lines_ref.get_mut(list_index) {
                        line.font_family = family.clone();
                        line.font_size = size;
                    }
                    drop(lines_ref);

                    // Update button label
                    font_button_clone.set_label(&format!("{} {:.0}", family, size));
                }
            }
        });
        font_box.append(&paste_font_btn);

        row_box.append(&font_box);

        // Color and rotation
        let extras_box = GtkBox::new(Orientation::Horizontal, 6);
        extras_box.append(&Label::new(Some("Color:")));

        // Color button (opens ColorDialog)
        let color_button = Button::new();
        let rgba = gtk4::gdk::RGBA::new(
            line_config.color.0 as f32,
            line_config.color.1 as f32,
            line_config.color.2 as f32,
            line_config.color.3 as f32,
        );

        // Create a colored box to show current color with unique CSS class
        let color_box = GtkBox::new(Orientation::Horizontal, 0);
        color_box.set_size_request(40, 20);
        let color_class = format!("color-preview-{}", list_index);
        color_box.add_css_class(&color_class);

        // Set background color using CSS (GTK 4.10+ compatible)
        let css = format!(
            "background-color: rgba({}, {}, {}, {});",
            (rgba.red() * 255.0) as u8,
            (rgba.green() * 255.0) as u8,
            (rgba.blue() * 255.0) as u8,
            rgba.alpha()
        );
        let provider = gtk4::CssProvider::new();
        provider.load_from_data(&format!(".{} {{ {} }}", color_class, css));
        let display = color_box.display();
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        color_button.set_child(Some(&color_box));

        // Connect color button to ColorPickerDialog
        let lines_clone = lines.clone();
        let color_class_clone = color_class.clone();
        let color_box_clone_for_picker = color_box.clone(); // Clone before moving into closure
        color_button.connect_clicked(move |btn| {
            let current_color = {
                let lines_ref = lines_clone.borrow();
                if let Some(line) = lines_ref.get(list_index) {
                    Color::new(line.color.0, line.color.1, line.color.2, line.color.3)
                } else {
                    Color::default()
                }
            };

            let window = btn.root().and_then(|root| root.downcast::<gtk4::Window>().ok());
            let lines_clone2 = lines_clone.clone();
            let color_box_clone = color_box_clone_for_picker.clone();
            let color_class_clone2 = color_class_clone.clone();

            gtk4::glib::MainContext::default().spawn_local(async move {
                if let Some(new_color) = ColorPickerDialog::pick_color(window.as_ref(), current_color).await {
                    let mut lines_ref = lines_clone2.borrow_mut();
                    if let Some(line) = lines_ref.get_mut(list_index) {
                        line.color = (new_color.r, new_color.g, new_color.b, new_color.a);
                    }
                    drop(lines_ref);

                    // Update color preview with unique CSS class (GTK 4.10+ compatible)
                    let rgba = gtk4::gdk::RGBA::new(
                        new_color.r as f32,
                        new_color.g as f32,
                        new_color.b as f32,
                        new_color.a as f32,
                    );
                    let css = format!(
                        "background-color: rgba({}, {}, {}, {});",
                        (rgba.red() * 255.0) as u8,
                        (rgba.green() * 255.0) as u8,
                        (rgba.blue() * 255.0) as u8,
                        rgba.alpha()
                    );
                    let provider = gtk4::CssProvider::new();
                    provider.load_from_data(&format!(".{} {{ {} }}", color_class_clone2, css));
                    let display = color_box_clone.display();
                    gtk4::style_context_add_provider_for_display(
                        &display,
                        &provider,
                        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
                    );
                }
            });
        });

        extras_box.append(&color_button);

        // Copy color button
        let copy_color_btn = Button::with_label("Copy");
        let lines_clone_copy_color = lines.clone();
        copy_color_btn.connect_clicked(move |_| {
            let lines_ref = lines_clone_copy_color.borrow();
            if let Some(line) = lines_ref.get(list_index) {
                if let Ok(mut clipboard) = crate::ui::clipboard::CLIPBOARD.lock() {
                    clipboard.copy_color(line.color.0, line.color.1, line.color.2, line.color.3);
                }
            }
        });
        extras_box.append(&copy_color_btn);

        // Paste color button
        let paste_color_btn = Button::with_label("Paste");
        let lines_clone_paste_color = lines.clone();
        let color_box_clone_paste = color_box.clone();
        let color_class_clone_paste = color_class.clone();
        paste_color_btn.connect_clicked(move |_| {
            if let Ok(clipboard) = crate::ui::clipboard::CLIPBOARD.lock() {
                if let Some(color) = clipboard.paste_color() {
                    let mut lines_ref = lines_clone_paste_color.borrow_mut();
                    if let Some(line) = lines_ref.get_mut(list_index) {
                        line.color = color;
                    }
                    drop(lines_ref);

                    // Update color preview
                    let rgba = gtk4::gdk::RGBA::new(
                        color.0 as f32,
                        color.1 as f32,
                        color.2 as f32,
                        color.3 as f32,
                    );
                    let css = format!(
                        "background-color: rgba({}, {}, {}, {});",
                        (rgba.red() * 255.0) as u8,
                        (rgba.green() * 255.0) as u8,
                        (rgba.blue() * 255.0) as u8,
                        rgba.alpha()
                    );
                    let provider = gtk4::CssProvider::new();
                    provider.load_from_data(&format!(".{} {{ {} }}", color_class_clone_paste, css));
                    let display = color_box_clone_paste.display();
                    gtk4::style_context_add_provider_for_display(
                        &display,
                        &provider,
                        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
                    );
                }
            }
        });
        extras_box.append(&paste_color_btn);

        extras_box.append(&Label::new(Some("Angle:")));
        let angle_spin = SpinButton::with_range(0.0, 360.0, 5.0);
        angle_spin.set_value(line_config.rotation_angle);
        extras_box.append(&angle_spin);
        row_box.append(&extras_box);

        // Combine checkbox
        let combine_box = GtkBox::new(Orientation::Horizontal, 6);
        let combine_check = CheckButton::with_label("Combine with others");
        combine_check.set_active(line_config.is_combined);
        combine_box.append(&combine_check);

        combine_box.append(&Label::new(Some("Group:")));
        let group_entry = Entry::new();
        if let Some(group) = &line_config.group_id {
            group_entry.set_text(group);
        }
        group_entry.set_width_chars(10);
        combine_box.append(&group_entry);
        row_box.append(&combine_box);

        // Wire up change handlers to update the TextLineConfig in the lines Vec

        // Font selection button handler
        {
            let lines_clone = lines.clone();
            let font_button_clone = font_button.clone();
            font_button.connect_clicked(move |btn| {
                let window = btn.root().and_then(|root| root.downcast::<gtk4::Window>().ok());

                // Get current font description
                let current_font = {
                    let lines_ref = lines_clone.borrow();
                    if let Some(line) = lines_ref.get(list_index) {
                        let font_str = format!("{} {}", line.font_family, line.font_size as i32);
                        gtk4::pango::FontDescription::from_string(&font_str)
                    } else {
                        gtk4::pango::FontDescription::from_string("Sans 12")
                    }
                };

                let lines_clone2 = lines_clone.clone();
                let font_button_clone2 = font_button_clone.clone();

                // Use callback-based API for font selection with shared dialog
                shared_font_dialog().choose_font(
                    window.as_ref(),
                    Some(&current_font),
                    gtk4::gio::Cancellable::NONE,
                    move |result| {
                        if let Ok(font_desc) = result {
                            // Extract family and size from font description
                            let family = font_desc.family().map(|s| s.to_string()).unwrap_or_else(|| "Sans".to_string());
                            let size = font_desc.size() as f64 / gtk4::pango::SCALE as f64;

                            let mut lines_ref = lines_clone2.borrow_mut();
                            if let Some(line) = lines_ref.get_mut(list_index) {
                                line.font_family = family.clone();
                                line.font_size = size;
                            }
                            drop(lines_ref);

                            // Update button label
                            font_button_clone2.set_label(&format!("{} {:.0}", family, size));
                        }
                    },
                );
            });
        }

        // Field selector handler
        {
            let lines_clone = lines.clone();
            let fields_clone = fields.to_vec();
            field_combo.connect_selected_notify(move |combo| {
                let selected = combo.selected() as usize;
                if let Some(field) = fields_clone.get(selected) {
                    let mut lines_ref = lines_clone.borrow_mut();
                    if let Some(line) = lines_ref.get_mut(list_index) {
                        line.field_id = field.id.clone();
                    }
                }
            });
        }

        // Vertical position handler
        {
            let lines_clone = lines.clone();
            v_pos_combo.connect_selected_notify(move |combo| {
                let vpos = match combo.selected() {
                    0 => VerticalPosition::Top,
                    1 => VerticalPosition::Center,
                    2 => VerticalPosition::Bottom,
                    _ => VerticalPosition::Center,
                };
                let mut lines_ref = lines_clone.borrow_mut();
                if let Some(line) = lines_ref.get_mut(list_index) {
                    line.vertical_position = vpos;
                }
            });
        }

        // Horizontal position handler
        {
            let lines_clone = lines.clone();
            h_pos_combo.connect_selected_notify(move |combo| {
                let hpos = match combo.selected() {
                    0 => HorizontalPosition::Left,
                    1 => HorizontalPosition::Center,
                    2 => HorizontalPosition::Right,
                    _ => HorizontalPosition::Center,
                };
                let mut lines_ref = lines_clone.borrow_mut();
                if let Some(line) = lines_ref.get_mut(list_index) {
                    line.horizontal_position = hpos;
                }
            });
        }

        // Rotation angle handler
        {
            let lines_clone = lines.clone();
            angle_spin.connect_value_changed(move |spin| {
                let mut lines_ref = lines_clone.borrow_mut();
                if let Some(line) = lines_ref.get_mut(list_index) {
                    line.rotation_angle = spin.value();
                }
            });
        }

        // Combine checkbox handler
        {
            let lines_clone = lines.clone();
            combine_check.connect_toggled(move |check| {
                let mut lines_ref = lines_clone.borrow_mut();
                if let Some(line) = lines_ref.get_mut(list_index) {
                    line.is_combined = check.is_active();
                }
            });
        }

        // Group ID handler
        {
            let lines_clone = lines.clone();
            group_entry.connect_changed(move |entry| {
                let text = entry.text().to_string();
                let mut lines_ref = lines_clone.borrow_mut();
                if let Some(line) = lines_ref.get_mut(list_index) {
                    line.group_id = if text.is_empty() { None } else { Some(text) };
                }
            });
        }

        // Delete button
        let delete_button = Button::with_label("Remove Line");
        delete_button.add_css_class("destructive-action");
        row_box.append(&delete_button);

        // Delete button handler
        delete_button.connect_clicked(move |_| {
            let mut lines_ref = lines.borrow_mut();
            if list_index < lines_ref.len() {
                lines_ref.remove(list_index);
            }
            drop(lines_ref);

            // Call rebuild callback to refresh the entire list with correct indices
            if let Some(ref rebuild) = rebuild_callback {
                rebuild();
            }
        });

        list_box.append(&frame);
    }

    /// Rebuild the entire list UI from the current lines data
    fn rebuild_list(&self) {
        // Clear list box
        while let Some(child) = self.list_box.first_child() {
            self.list_box.remove(&child);
        }

        // Create rebuild callback for delete buttons
        let list_box_clone = self.list_box.clone();
        let lines_clone = self.lines.clone();
        let fields_clone = self.available_fields.clone();

        // Create rebuild function as Rc<RefCell> to allow self-reference
        let rebuild_fn: Rc<RefCell<Option<Rc<dyn Fn()>>>> = Rc::new(RefCell::new(None));
        let rebuild_fn_clone = rebuild_fn.clone();

        let rebuild_closure: Rc<dyn Fn()> = Rc::new(move || {
            // Clear list box
            while let Some(child) = list_box_clone.first_child() {
                list_box_clone.remove(&child);
            }

            // Rebuild all rows with the same rebuild callback
            let lines_data = lines_clone.borrow().clone();
            let callback_to_pass = rebuild_fn_clone.borrow().clone();
            for (index, line) in lines_data.into_iter().enumerate() {
                Self::add_line_row(
                    &list_box_clone,
                    line,
                    &fields_clone,
                    lines_clone.clone(),
                    index,
                    callback_to_pass.clone(),
                );
            }
        });

        // Store the callback in the RefCell
        *rebuild_fn.borrow_mut() = Some(rebuild_closure.clone());

        // Rebuild all rows with correct indices
        let lines = self.lines.borrow().clone();
        for (index, line) in lines.into_iter().enumerate() {
            Self::add_line_row(
                &self.list_box,
                line,
                &self.available_fields,
                self.lines.clone(),
                index,
                Some(rebuild_closure.clone()),
            );
        }
    }

    /// Set the configuration
    pub fn set_config(&self, config: TextDisplayerConfig) {
        // Update lines vector
        *self.lines.borrow_mut() = config.lines;

        // Rebuild UI
        self.rebuild_list();
    }

    /// Get the current configuration
    pub fn get_config(&self) -> TextDisplayerConfig {
        TextDisplayerConfig {
            lines: self.lines.borrow().clone(),
        }
    }

    /// Get the widget
    pub fn widget(&self) -> &Widget {
        self.widget.upcast_ref()
    }
}
