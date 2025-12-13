//! Widget for configuring text displayer lines

use crate::core::FieldMetadata;
use crate::displayers::{HorizontalPosition, TextDisplayerConfig, TextLineConfig, VerticalPosition};
use crate::ui::color_button_widget::ColorButtonWidget;
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
    on_change: Rc<RefCell<Option<Box<dyn Fn()>>>>,
}

impl TextLineConfigWidget {
    /// Create a new text line configuration widget
    pub fn new(available_fields: Vec<FieldMetadata>) -> Self {
        let widget = GtkBox::new(Orientation::Vertical, 6);

        // Header with add button and copy/paste
        let header_box = GtkBox::new(Orientation::Horizontal, 6);
        let header_label = Label::new(Some("Text Lines"));
        header_label.set_hexpand(true);
        header_label.set_halign(gtk4::Align::Start);

        let copy_btn = Button::with_label("Copy");
        let paste_btn = Button::with_label("Paste");
        let add_button = Button::with_label("+ Add Line");
        header_box.append(&header_label);
        header_box.append(&copy_btn);
        header_box.append(&paste_btn);
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
        let on_change: Rc<RefCell<Option<Box<dyn Fn()>>>> = Rc::new(RefCell::new(None));

        // Set up add button - uses a self-contained rebuild callback like delete
        let lines_for_add = lines.clone();
        let list_box_for_add = list_box.clone();
        let fields_for_add = available_fields.clone();
        let on_change_for_rebuild = on_change.clone();

        // Create self-referential rebuild callback for add
        let rebuild_fn: Rc<RefCell<Option<Rc<dyn Fn()>>>> = Rc::new(RefCell::new(None));

        let rebuild_closure: Rc<dyn Fn()> = Rc::new({
            let lines_inner = lines_for_add.clone();
            let list_box_inner = list_box_for_add.clone();
            let fields_inner = fields_for_add.clone();
            let rebuild_fn_inner = rebuild_fn.clone();
            let on_change_inner = on_change_for_rebuild.clone();

            move || {
                // Clear list box
                while let Some(child) = list_box_inner.first_child() {
                    list_box_inner.remove(&child);
                }

                // Rebuild all rows with the same rebuild callback
                let lines_data = lines_inner.borrow().clone();
                let callback_to_pass = rebuild_fn_inner.borrow().clone();
                for (index, line) in lines_data.into_iter().enumerate() {
                    Self::add_line_row(
                        &list_box_inner,
                        line,
                        &fields_inner,
                        lines_inner.clone(),
                        index,
                        callback_to_pass.clone(),
                        on_change_inner.clone(),
                    );
                }
            }
        });

        // Store the callback so it can reference itself
        *rebuild_fn.borrow_mut() = Some(rebuild_closure.clone());

        let on_change_for_add = on_change.clone();
        add_button.connect_clicked(move |_| {
            // Add new line to data
            {
                let mut lines = lines_for_add.borrow_mut();
                let new_line = TextLineConfig::default();
                lines.push(new_line);
            }
            // Trigger full rebuild so the new line has the rebuild callback
            rebuild_closure();
            // Trigger on_change callback
            if let Some(ref callback) = *on_change_for_add.borrow() {
                callback();
            }
        });

        // Copy button handler
        let lines_for_copy = lines.clone();
        copy_btn.connect_clicked(move |_| {
            let config = crate::displayers::TextDisplayerConfig {
                lines: lines_for_copy.borrow().clone(),
            };
            if let Ok(mut clipboard) = crate::ui::clipboard::CLIPBOARD.lock() {
                clipboard.copy_text_display(config);
            }
        });

        // Paste button handler - needs to rebuild list
        let lines_for_paste = lines.clone();
        let list_box_for_paste = list_box.clone();
        let fields_for_paste = available_fields.clone();
        let on_change_for_paste = on_change.clone();
        let rebuild_fn_for_paste = rebuild_fn.clone();
        paste_btn.connect_clicked(move |_| {
            let pasted = if let Ok(clipboard) = crate::ui::clipboard::CLIPBOARD.lock() {
                clipboard.paste_text_display()
            } else {
                None
            };

            if let Some(config) = pasted {
                // Update lines
                *lines_for_paste.borrow_mut() = config.lines;

                // Rebuild list
                while let Some(child) = list_box_for_paste.first_child() {
                    list_box_for_paste.remove(&child);
                }
                let lines_data = lines_for_paste.borrow().clone();
                let callback_to_pass = rebuild_fn_for_paste.borrow().clone();
                for (index, line) in lines_data.into_iter().enumerate() {
                    Self::add_line_row(
                        &list_box_for_paste,
                        line,
                        &fields_for_paste,
                        lines_for_paste.clone(),
                        index,
                        callback_to_pass.clone(),
                        on_change_for_paste.clone(),
                    );
                }

                // Trigger on_change
                if let Some(ref callback) = *on_change_for_paste.borrow() {
                    callback();
                }
            }
        });

        Self {
            widget,
            lines,
            list_box,
            available_fields,
            on_change,
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
        on_change: Rc<RefCell<Option<Box<dyn Fn()>>>>,
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

        // Font size spinner
        let size_label = Label::new(Some("Size:"));
        font_box.append(&size_label);

        let size_spin = SpinButton::with_range(6.0, 200.0, 1.0);
        size_spin.set_value(line_config.font_size);
        size_spin.set_width_chars(4);
        font_box.append(&size_spin);

        // Update font size when spinner changes
        let lines_clone_size = lines.clone();
        let font_button_clone_size = font_button.clone();
        let on_change_size = on_change.clone();
        size_spin.connect_value_changed(move |spin| {
            let new_size = spin.value();
            let mut lines_ref = lines_clone_size.borrow_mut();
            if let Some(line) = lines_ref.get_mut(list_index) {
                line.font_size = new_size;
                // Update button label
                font_button_clone_size.set_label(&format!("{} {:.0}", line.font_family, new_size));
            }
            drop(lines_ref);
            if let Some(ref callback) = *on_change_size.borrow() {
                callback();
            }
        });

        // Bold/Italic checkboxes
        let bold_check = CheckButton::with_label("B");
        bold_check.set_active(line_config.bold);
        bold_check.set_tooltip_text(Some("Bold"));
        font_box.append(&bold_check);

        let lines_clone_bold = lines.clone();
        let on_change_bold = on_change.clone();
        bold_check.connect_toggled(move |check| {
            let mut lines_ref = lines_clone_bold.borrow_mut();
            if let Some(line) = lines_ref.get_mut(list_index) {
                line.bold = check.is_active();
            }
            drop(lines_ref);
            if let Some(ref callback) = *on_change_bold.borrow() {
                callback();
            }
        });

        let italic_check = CheckButton::with_label("I");
        italic_check.set_active(line_config.italic);
        italic_check.set_tooltip_text(Some("Italic"));
        font_box.append(&italic_check);

        let lines_clone_italic = lines.clone();
        let on_change_italic = on_change.clone();
        italic_check.connect_toggled(move |check| {
            let mut lines_ref = lines_clone_italic.borrow_mut();
            if let Some(line) = lines_ref.get_mut(list_index) {
                line.italic = check.is_active();
            }
            drop(lines_ref);
            if let Some(ref callback) = *on_change_italic.borrow() {
                callback();
            }
        });

        // Copy font button
        let copy_font_btn = Button::with_label("Copy");
        let lines_clone_copy_font = lines.clone();
        copy_font_btn.connect_clicked(move |_| {
            let lines_ref = lines_clone_copy_font.borrow();
            if let Some(line) = lines_ref.get(list_index) {
                if let Ok(mut clipboard) = crate::ui::clipboard::CLIPBOARD.lock() {
                    clipboard.copy_font(line.font_family.clone(), line.font_size, line.bold, line.italic);
                }
            }
        });
        font_box.append(&copy_font_btn);

        // Paste font button
        let paste_font_btn = Button::with_label("Paste");
        let lines_clone_paste_font = lines.clone();
        let font_button_clone = font_button.clone();
        let size_spin_clone = size_spin.clone();
        let bold_check_clone = bold_check.clone();
        let italic_check_clone = italic_check.clone();
        let on_change_paste = on_change.clone();
        paste_font_btn.connect_clicked(move |_| {
            if let Ok(clipboard) = crate::ui::clipboard::CLIPBOARD.lock() {
                if let Some((family, size, bold, italic)) = clipboard.paste_font() {
                    let mut lines_ref = lines_clone_paste_font.borrow_mut();
                    if let Some(line) = lines_ref.get_mut(list_index) {
                        line.font_family = family.clone();
                        line.font_size = size;
                        line.bold = bold;
                        line.italic = italic;
                    }
                    drop(lines_ref);

                    // Update button label, size spinner, and bold/italic checks
                    font_button_clone.set_label(&format!("{} {:.0}", family, size));
                    size_spin_clone.set_value(size);
                    bold_check_clone.set_active(bold);
                    italic_check_clone.set_active(italic);

                    if let Some(ref callback) = *on_change_paste.borrow() {
                        callback();
                    }
                }
            }
        });
        font_box.append(&paste_font_btn);

        row_box.append(&font_box);

        // Color and rotation
        let extras_box = GtkBox::new(Orientation::Horizontal, 6);
        extras_box.append(&Label::new(Some("Color:")));

        // Color widget using ColorButtonWidget
        let initial_color = Color::new(
            line_config.color.0,
            line_config.color.1,
            line_config.color.2,
            line_config.color.3,
        );
        let color_widget = ColorButtonWidget::new(initial_color);
        extras_box.append(color_widget.widget());

        // Connect color widget change handler
        let lines_clone = lines.clone();
        let on_change_color = on_change.clone();
        color_widget.set_on_change(move |new_color| {
            let mut lines_ref = lines_clone.borrow_mut();
            if let Some(line) = lines_ref.get_mut(list_index) {
                line.color = (new_color.r, new_color.g, new_color.b, new_color.a);
            }
            drop(lines_ref);
            if let Some(ref callback) = *on_change_color.borrow() {
                callback();
            }
        });

        extras_box.append(&Label::new(Some("Angle:")));
        let angle_spin = SpinButton::with_range(0.0, 360.0, 5.0);
        angle_spin.set_value(line_config.rotation_angle);
        extras_box.append(&angle_spin);
        row_box.append(&extras_box);

        // Position offset controls
        let offset_box = GtkBox::new(Orientation::Horizontal, 6);
        offset_box.append(&Label::new(Some("Offset X:")));
        let offset_x_spin = SpinButton::with_range(-500.0, 500.0, 1.0);
        offset_x_spin.set_value(line_config.offset_x);
        offset_x_spin.set_width_chars(5);
        offset_box.append(&offset_x_spin);

        offset_box.append(&Label::new(Some("Y:")));
        let offset_y_spin = SpinButton::with_range(-500.0, 500.0, 1.0);
        offset_y_spin.set_value(line_config.offset_y);
        offset_y_spin.set_width_chars(5);
        offset_box.append(&offset_y_spin);
        row_box.append(&offset_box);

        // Combine checkbox and group selector
        let combine_box = GtkBox::new(Orientation::Horizontal, 6);
        let combine_check = CheckButton::with_label("Combine with others");
        combine_check.set_active(line_config.is_combined);
        combine_box.append(&combine_check);

        combine_box.append(&Label::new(Some("Group:")));

        // Group dropdown with 8 presets + Custom
        let group_options = vec![
            "Group 1", "Group 2", "Group 3", "Group 4",
            "Group 5", "Group 6", "Group 7", "Group 8",
            "Custom"
        ];
        let group_list = StringList::new(&group_options);
        let group_combo = DropDown::new(Some(group_list), Option::<gtk4::Expression>::None);

        // Determine initial selection
        let (initial_group_index, is_custom) = if let Some(ref group) = line_config.group_id {
            match group.as_str() {
                "Group 1" => (0, false),
                "Group 2" => (1, false),
                "Group 3" => (2, false),
                "Group 4" => (3, false),
                "Group 5" => (4, false),
                "Group 6" => (5, false),
                "Group 7" => (6, false),
                "Group 8" => (7, false),
                _ => (8, true), // Custom
            }
        } else {
            (0, false)
        };

        group_combo.set_selected(initial_group_index as u32);
        combine_box.append(&group_combo);

        // Custom group text entry (shown when "Custom" is selected)
        let group_entry = Entry::new();
        if let Some(group) = &line_config.group_id {
            if is_custom {
                group_entry.set_text(group);
            }
        }
        group_entry.set_width_chars(10);
        group_entry.set_visible(is_custom);
        combine_box.append(&group_entry);

        row_box.append(&combine_box);

        // Wire up change handlers to update the TextLineConfig in the lines Vec

        // Font selection button handler
        {
            let lines_clone = lines.clone();
            let font_button_clone = font_button.clone();
            let size_spin_clone = size_spin.clone();
            let on_change_font = on_change.clone();
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
                let size_spin_clone2 = size_spin_clone.clone();
                let on_change_font2 = on_change_font.clone();

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

                            // Update button label and size spinner
                            font_button_clone2.set_label(&format!("{} {:.0}", family, size));
                            size_spin_clone2.set_value(size);

                            if let Some(ref callback) = *on_change_font2.borrow() {
                                callback();
                            }
                        }
                    },
                );
            });
        }

        // Field selector handler
        {
            let lines_clone = lines.clone();
            let fields_clone = fields.to_vec();
            let on_change_field = on_change.clone();
            field_combo.connect_selected_notify(move |combo| {
                let selected = combo.selected() as usize;
                if let Some(field) = fields_clone.get(selected) {
                    let mut lines_ref = lines_clone.borrow_mut();
                    if let Some(line) = lines_ref.get_mut(list_index) {
                        line.field_id = field.id.clone();
                    }
                    drop(lines_ref);
                    if let Some(ref callback) = *on_change_field.borrow() {
                        callback();
                    }
                }
            });
        }

        // Vertical position handler
        {
            let lines_clone = lines.clone();
            let on_change_vpos = on_change.clone();
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
                drop(lines_ref);
                if let Some(ref callback) = *on_change_vpos.borrow() {
                    callback();
                }
            });
        }

        // Horizontal position handler
        {
            let lines_clone = lines.clone();
            let on_change_hpos = on_change.clone();
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
                drop(lines_ref);
                if let Some(ref callback) = *on_change_hpos.borrow() {
                    callback();
                }
            });
        }

        // Rotation angle handler
        {
            let lines_clone = lines.clone();
            let on_change_angle = on_change.clone();
            angle_spin.connect_value_changed(move |spin| {
                let mut lines_ref = lines_clone.borrow_mut();
                if let Some(line) = lines_ref.get_mut(list_index) {
                    line.rotation_angle = spin.value();
                }
                drop(lines_ref);
                if let Some(ref callback) = *on_change_angle.borrow() {
                    callback();
                }
            });
        }

        // Offset X handler
        {
            let lines_clone = lines.clone();
            let on_change_x = on_change.clone();
            offset_x_spin.connect_value_changed(move |spin| {
                let mut lines_ref = lines_clone.borrow_mut();
                if let Some(line) = lines_ref.get_mut(list_index) {
                    line.offset_x = spin.value();
                }
                drop(lines_ref);
                if let Some(ref callback) = *on_change_x.borrow() {
                    callback();
                }
            });
        }

        // Offset Y handler
        {
            let lines_clone = lines.clone();
            let on_change_y = on_change.clone();
            offset_y_spin.connect_value_changed(move |spin| {
                let mut lines_ref = lines_clone.borrow_mut();
                if let Some(line) = lines_ref.get_mut(list_index) {
                    line.offset_y = spin.value();
                }
                drop(lines_ref);
                if let Some(ref callback) = *on_change_y.borrow() {
                    callback();
                }
            });
        }

        // Helper function to check if this is the first line in a group
        let is_first_in_group = |lines: &[TextLineConfig], index: usize| -> bool {
            if index >= lines.len() {
                return true;
            }
            let current_line = &lines[index];
            if !current_line.is_combined {
                return true; // Not in a group, can set angle
            }
            if current_line.group_id.is_none() {
                return true; // No group set, can set angle
            }

            // Check if there's any earlier line with the same group_id
            let group_id = current_line.group_id.as_ref().unwrap();
            for (i, line) in lines.iter().enumerate() {
                if i >= index {
                    break;
                }
                if line.is_combined && line.group_id.as_ref() == Some(group_id) {
                    return false; // Found an earlier line in the same group
                }
            }
            true // This is the first line in the group
        };

        // Update angle spinner sensitivity based on group position
        let update_angle_sensitivity = {
            let lines_clone = lines.clone();
            let angle_spin_clone = angle_spin.clone();
            move || {
                let lines_ref = lines_clone.borrow();
                let is_first = is_first_in_group(&lines_ref, list_index);
                angle_spin_clone.set_sensitive(is_first);
            }
        };

        // Initial update of angle sensitivity
        update_angle_sensitivity();

        // Combine checkbox handler
        {
            let lines_clone = lines.clone();
            let rebuild = rebuild_callback.clone();
            let on_change_combine = on_change.clone();
            combine_check.connect_toggled(move |check| {
                let mut lines_ref = lines_clone.borrow_mut();
                if let Some(line) = lines_ref.get_mut(list_index) {
                    line.is_combined = check.is_active();
                }
                drop(lines_ref);
                // Rebuild all widgets so all lines can re-evaluate angle sensitivity
                if let Some(ref rebuild_fn) = rebuild {
                    rebuild_fn();
                }
                if let Some(ref callback) = *on_change_combine.borrow() {
                    callback();
                }
            });
        }

        // Group dropdown handler
        {
            let lines_clone = lines.clone();
            let group_entry_clone = group_entry.clone();
            let rebuild = rebuild_callback.clone();
            let on_change_group = on_change.clone();
            group_combo.connect_selected_notify(move |combo| {
                let selected = combo.selected();
                let is_custom = selected == 8; // Last option is "Custom"

                // Show/hide custom entry
                group_entry_clone.set_visible(is_custom);

                // Update group_id
                let mut lines_ref = lines_clone.borrow_mut();
                if let Some(line) = lines_ref.get_mut(list_index) {
                    if is_custom {
                        // Use custom text from entry
                        let custom_text = group_entry_clone.text().to_string();
                        line.group_id = if custom_text.is_empty() {
                            Some("Custom".to_string())
                        } else {
                            Some(custom_text)
                        };
                    } else {
                        // Use preset group name
                        line.group_id = Some(format!("Group {}", selected + 1));
                    }
                }
                drop(lines_ref);
                // Rebuild all widgets so all lines can re-evaluate angle sensitivity
                if let Some(ref rebuild_fn) = rebuild {
                    rebuild_fn();
                }
                if let Some(ref callback) = *on_change_group.borrow() {
                    callback();
                }
            });
        }

        // Custom group entry handler
        {
            let lines_clone = lines.clone();
            let update_angle = update_angle_sensitivity.clone();
            let on_change_entry = on_change.clone();
            group_entry.connect_changed(move |entry| {
                let text = entry.text().to_string();
                let mut lines_ref = lines_clone.borrow_mut();
                if let Some(line) = lines_ref.get_mut(list_index) {
                    // Only update if custom entry is visible
                    if gtk4::prelude::WidgetExt::is_visible(entry) {
                        line.group_id = if text.is_empty() {
                            Some("Custom".to_string())
                        } else {
                            Some(text)
                        };
                    }
                }
                drop(lines_ref);
                update_angle();
                if let Some(ref callback) = *on_change_entry.borrow() {
                    callback();
                }
            });
        }

        // Delete button
        let delete_button = Button::with_label("Remove Line");
        delete_button.add_css_class("destructive-action");
        row_box.append(&delete_button);

        // Delete button handler
        let on_change_delete = on_change.clone();
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
            if let Some(ref callback) = *on_change_delete.borrow() {
                callback();
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
        let on_change_clone = self.on_change.clone();

        // Create rebuild function as Rc<RefCell> to allow self-reference
        let rebuild_fn: Rc<RefCell<Option<Rc<dyn Fn()>>>> = Rc::new(RefCell::new(None));
        let rebuild_fn_clone = rebuild_fn.clone();
        let on_change_for_rebuild = on_change_clone.clone();

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
                    on_change_for_rebuild.clone(),
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
                on_change_clone.clone(),
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

    /// Set the on_change callback
    pub fn set_on_change<F: Fn() + 'static>(&self, callback: F) {
        *self.on_change.borrow_mut() = Some(Box::new(callback));
    }
}
