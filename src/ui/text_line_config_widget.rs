//! Widget for configuring text displayer lines

use crate::core::FieldMetadata;
use crate::displayers::{
    CombineDirection, TextBackgroundConfig, TextBackgroundType, TextDisplayerConfig,
    TextFillType, TextLineConfig,
};
use crate::ui::background::{Color, ColorStop, LinearGradientConfig};
use crate::ui::color_button_widget::ColorButtonWidget;
use crate::ui::gradient_editor::GradientEditor;
use crate::ui::position_grid_widget::PositionGridWidget;
use crate::ui::shared_font_dialog::shared_font_dialog;
use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, CheckButton, DropDown, Entry, Frame, Label, ListBox, Orientation,
    ScrolledWindow, SpinButton, StringList, Widget,
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
        log::debug!("TextLineConfigWidget::new() called with {} available fields", available_fields.len());
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
            log::info!("=== TextLineConfigWidget: Add Line clicked ===");
            // Add new line to data
            {
                let mut lines = lines_for_add.borrow_mut();
                let new_line = TextLineConfig::default();
                log::info!("    Adding new line with default field_id='{}'", new_line.field_id);
                lines.push(new_line);
                log::info!("    Total lines now: {}", lines.len());
            }
            // Trigger full rebuild so the new line has the rebuild callback
            rebuild_closure();
            // Trigger on_change callback
            if let Some(ref callback) = *on_change_for_add.borrow() {
                log::info!("    Calling on_change callback");
                callback();
            } else {
                log::warn!("    on_change callback is None!");
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

        // Find selected index, or default to first field if field_id is empty
        if let Some(idx) = fields.iter().position(|f| f.id == line_config.field_id) {
            field_combo.set_selected(idx as u32);
        } else if !fields.is_empty() {
            // If field_id doesn't match any field (e.g., new line with empty field_id),
            // set it to the first available field
            field_combo.set_selected(0);
            let mut lines_ref = lines.borrow_mut();
            if let Some(line) = lines_ref.get_mut(list_index) {
                line.field_id = fields[0].id.clone();
            }
        }

        field_box.append(&field_combo);

        // Add spacer to push reorder buttons to the right
        let spacer = GtkBox::new(Orientation::Horizontal, 0);
        spacer.set_hexpand(true);
        field_box.append(&spacer);

        // Move up button
        let move_up_btn = Button::with_label("↑");
        move_up_btn.set_tooltip_text(Some("Move line up"));
        move_up_btn.set_sensitive(list_index > 0);
        field_box.append(&move_up_btn);

        // Move down button
        let move_down_btn = Button::with_label("↓");
        move_down_btn.set_tooltip_text(Some("Move line down"));
        // We'll update sensitivity after we know total line count
        field_box.append(&move_down_btn);

        // Update move down button sensitivity based on total lines
        {
            let lines_count = lines.borrow().len();
            move_down_btn.set_sensitive(list_index < lines_count.saturating_sub(1));
        }

        // Move up handler
        {
            let lines_clone = lines.clone();
            let rebuild = rebuild_callback.clone();
            let on_change_up = on_change.clone();
            move_up_btn.connect_clicked(move |_| {
                if list_index > 0 {
                    let mut lines_ref = lines_clone.borrow_mut();
                    if list_index < lines_ref.len() {
                        lines_ref.swap(list_index, list_index - 1);
                    }
                    drop(lines_ref);
                    if let Some(ref rebuild_fn) = rebuild {
                        rebuild_fn();
                    }
                    if let Some(ref callback) = *on_change_up.borrow() {
                        callback();
                    }
                }
            });
        }

        // Move down handler
        {
            let lines_clone = lines.clone();
            let rebuild = rebuild_callback.clone();
            let on_change_down = on_change.clone();
            move_down_btn.connect_clicked(move |_| {
                let mut lines_ref = lines_clone.borrow_mut();
                if list_index + 1 < lines_ref.len() {
                    lines_ref.swap(list_index, list_index + 1);
                }
                drop(lines_ref);
                if let Some(ref rebuild_fn) = rebuild {
                    rebuild_fn();
                }
                if let Some(ref callback) = *on_change_down.borrow() {
                    callback();
                }
            });
        }

        row_box.append(&field_box);

        // Position controls - 3x3 grid
        let pos_box = GtkBox::new(Orientation::Horizontal, 6);
        pos_box.append(&Label::new(Some("Position:")));

        let position_grid = Rc::new(PositionGridWidget::new(line_config.position));
        pos_box.append(position_grid.widget());

        // Connect position grid change handler
        let lines_clone_pos = lines.clone();
        let on_change_pos = on_change.clone();
        position_grid.set_on_change(move |new_pos| {
            let mut lines_ref = lines_clone_pos.borrow_mut();
            if let Some(line) = lines_ref.get_mut(list_index) {
                line.position = new_pos;
            }
            drop(lines_ref);
            if let Some(ref callback) = *on_change_pos.borrow() {
                callback();
            }
        });

        row_box.append(&pos_box);

        // Combine direction and alignment (only visible when is_combined=true and this is first in group)
        let direction_align_box = GtkBox::new(Orientation::Vertical, 6);

        let direction_row = GtkBox::new(Orientation::Horizontal, 6);
        direction_row.append(&Label::new(Some("Direction:")));

        let direction_list = StringList::new(&["Horizontal", "Vertical"]);
        let direction_combo = DropDown::new(Some(direction_list), Option::<gtk4::Expression>::None);
        direction_combo.set_selected(match line_config.combine_direction {
            CombineDirection::Horizontal => 0,
            CombineDirection::Vertical => 1,
        });
        direction_row.append(&direction_combo);
        direction_align_box.append(&direction_row);

        // Alignment using 3x3 position grid
        let alignment_row = GtkBox::new(Orientation::Horizontal, 6);
        alignment_row.append(&Label::new(Some("Align:")));
        let alignment_grid = Rc::new(PositionGridWidget::new(line_config.combine_alignment));
        alignment_row.append(alignment_grid.widget());
        direction_align_box.append(&alignment_row);

        // Helper to check if direction/alignment should be visible:
        // 1. This line must be combined with a group_id
        // 2. Must be the FIRST line in the group
        // 3. There must be 2+ lines in the group
        let should_show_dir_align = {
            let all_lines = lines.borrow();
            if !line_config.is_combined || line_config.group_id.is_none() {
                false
            } else {
                let group_id = line_config.group_id.as_ref().unwrap();

                // Find first line in group and count lines in group
                let mut first_index_in_group: Option<usize> = None;
                let mut group_count = 0;

                for (i, line) in all_lines.iter().enumerate() {
                    if line.is_combined && line.group_id.as_ref() == Some(group_id) {
                        // Track first line in group
                        if first_index_in_group.is_none() {
                            first_index_in_group = Some(i);
                        }
                        group_count += 1;
                    }
                }

                // Show only if this is the first line in group AND there are 2+ lines in the group
                first_index_in_group == Some(list_index) && group_count >= 2
            }
        };

        direction_align_box.set_visible(should_show_dir_align);
        row_box.append(&direction_align_box);

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

        // Fill type selector (Solid/Gradient)
        let fill_frame = Frame::new(Some("Text Fill"));
        let fill_box = GtkBox::new(Orientation::Vertical, 6);
        fill_box.set_margin_start(6);
        fill_box.set_margin_end(6);
        fill_box.set_margin_top(6);
        fill_box.set_margin_bottom(6);

        let fill_type_row = GtkBox::new(Orientation::Horizontal, 6);
        fill_type_row.append(&Label::new(Some("Type:")));

        let fill_type_list = StringList::new(&["Solid Color", "Gradient"]);
        let fill_type_combo = DropDown::new(Some(fill_type_list), Option::<gtk4::Expression>::None);
        let initial_fill_type_index = match &line_config.fill {
            TextFillType::Solid { .. } => 0,
            TextFillType::LinearGradient { .. } => 1,
        };
        fill_type_combo.set_selected(initial_fill_type_index);
        fill_type_row.append(&fill_type_combo);
        fill_box.append(&fill_type_row);

        // Solid color container
        let solid_fill_box = GtkBox::new(Orientation::Horizontal, 6);
        solid_fill_box.append(&Label::new(Some("Color:")));
        let line_color = line_config.color();
        let initial_color = Color::new(line_color.0, line_color.1, line_color.2, line_color.3);
        let fill_color_widget = Rc::new(ColorButtonWidget::new(initial_color));
        solid_fill_box.append(fill_color_widget.widget());
        solid_fill_box.set_visible(initial_fill_type_index == 0);
        fill_box.append(&solid_fill_box);

        // Gradient fill container
        let gradient_fill_box = GtkBox::new(Orientation::Vertical, 6);
        let fill_gradient_editor = Rc::new(GradientEditor::new());
        // Initialize gradient editor with current value or defaults
        match &line_config.fill {
            TextFillType::LinearGradient { stops, angle } => {
                fill_gradient_editor.set_gradient(&LinearGradientConfig {
                    stops: stops.clone(),
                    angle: *angle,
                });
            }
            TextFillType::Solid { color } => {
                // Default 2-stop gradient using the solid color
                fill_gradient_editor.set_gradient(&LinearGradientConfig {
                    stops: vec![
                        ColorStop::new(0.0, *color),
                        ColorStop::new(1.0, Color::new(color.r * 0.5, color.g * 0.5, color.b * 0.5, color.a)),
                    ],
                    angle: 0.0,
                });
            }
        }
        gradient_fill_box.append(fill_gradient_editor.widget());
        gradient_fill_box.set_visible(initial_fill_type_index == 1);
        fill_box.append(&gradient_fill_box);

        fill_frame.set_child(Some(&fill_box));
        row_box.append(&fill_frame);

        // Connect fill type combo change handler
        {
            let solid_fill_box_clone = solid_fill_box.clone();
            let gradient_fill_box_clone = gradient_fill_box.clone();
            let lines_clone = lines.clone();
            let fill_color_widget_clone = fill_color_widget.clone();
            let fill_gradient_editor_clone = fill_gradient_editor.clone();
            let on_change_fill_type = on_change.clone();
            fill_type_combo.connect_selected_notify(move |combo| {
                let selected = combo.selected();
                solid_fill_box_clone.set_visible(selected == 0);
                gradient_fill_box_clone.set_visible(selected == 1);

                // Update fill type in config
                let mut lines_ref = lines_clone.borrow_mut();
                if let Some(line) = lines_ref.get_mut(list_index) {
                    match selected {
                        0 => {
                            // Switch to Solid - use current color widget color
                            let color = fill_color_widget_clone.color();
                            line.fill = TextFillType::Solid { color };
                        }
                        1 => {
                            // Switch to Gradient - use current gradient editor values
                            let grad = fill_gradient_editor_clone.get_gradient();
                            line.fill = TextFillType::LinearGradient {
                                stops: grad.stops,
                                angle: grad.angle,
                            };
                        }
                        _ => {}
                    }
                }
                drop(lines_ref);
                if let Some(ref callback) = *on_change_fill_type.borrow() {
                    callback();
                }
            });
        }

        // Connect solid fill color change handler
        {
            let lines_clone = lines.clone();
            let on_change_color = on_change.clone();
            fill_color_widget.set_on_change(move |new_color| {
                let mut lines_ref = lines_clone.borrow_mut();
                if let Some(line) = lines_ref.get_mut(list_index) {
                    line.fill = TextFillType::Solid { color: new_color };
                }
                drop(lines_ref);
                if let Some(ref callback) = *on_change_color.borrow() {
                    callback();
                }
            });
        }

        // Connect gradient editor change handler
        {
            let lines_clone = lines.clone();
            let fill_gradient_editor_clone = fill_gradient_editor.clone();
            let on_change_grad = on_change.clone();
            fill_gradient_editor.set_on_change(move || {
                let grad = fill_gradient_editor_clone.get_gradient();
                let mut lines_ref = lines_clone.borrow_mut();
                if let Some(line) = lines_ref.get_mut(list_index) {
                    line.fill = TextFillType::LinearGradient {
                        stops: grad.stops,
                        angle: grad.angle,
                    };
                }
                drop(lines_ref);
                if let Some(ref callback) = *on_change_grad.borrow() {
                    callback();
                }
            });
        }

        // Text background section
        let bg_frame = Frame::new(Some("Text Background"));
        let bg_box = GtkBox::new(Orientation::Vertical, 6);
        bg_box.set_margin_start(6);
        bg_box.set_margin_end(6);
        bg_box.set_margin_top(6);
        bg_box.set_margin_bottom(6);

        let bg_type_row = GtkBox::new(Orientation::Horizontal, 6);
        bg_type_row.append(&Label::new(Some("Type:")));

        let bg_type_list = StringList::new(&["None", "Solid Color", "Gradient"]);
        let bg_type_combo = DropDown::new(Some(bg_type_list), Option::<gtk4::Expression>::None);
        let initial_bg_type_index = match &line_config.text_background.background {
            TextBackgroundType::None => 0,
            TextBackgroundType::Solid { .. } => 1,
            TextBackgroundType::LinearGradient { .. } => 2,
        };
        bg_type_combo.set_selected(initial_bg_type_index);
        bg_type_row.append(&bg_type_combo);
        bg_box.append(&bg_type_row);

        // Background padding and corner radius (visible except for None)
        let bg_params_box = GtkBox::new(Orientation::Horizontal, 6);
        bg_params_box.append(&Label::new(Some("Padding:")));
        let bg_padding_spin = SpinButton::with_range(0.0, 50.0, 1.0);
        bg_padding_spin.set_value(line_config.text_background.padding);
        bg_padding_spin.set_width_chars(4);
        bg_params_box.append(&bg_padding_spin);
        bg_params_box.append(&Label::new(Some("Radius:")));
        let bg_radius_spin = SpinButton::with_range(0.0, 50.0, 1.0);
        bg_radius_spin.set_value(line_config.text_background.corner_radius);
        bg_radius_spin.set_width_chars(4);
        bg_params_box.append(&bg_radius_spin);
        bg_params_box.set_visible(initial_bg_type_index != 0);
        bg_box.append(&bg_params_box);

        // Background solid color container
        let bg_solid_box = GtkBox::new(Orientation::Horizontal, 6);
        bg_solid_box.append(&Label::new(Some("Color:")));
        let bg_solid_color = match &line_config.text_background.background {
            TextBackgroundType::Solid { color } => *color,
            _ => Color::new(0.0, 0.0, 0.0, 0.5),
        };
        let bg_color_widget = Rc::new(ColorButtonWidget::new(bg_solid_color));
        bg_solid_box.append(bg_color_widget.widget());
        bg_solid_box.set_visible(initial_bg_type_index == 1);
        bg_box.append(&bg_solid_box);

        // Background gradient container
        let bg_gradient_box = GtkBox::new(Orientation::Vertical, 6);
        let bg_gradient_editor = Rc::new(GradientEditor::new());
        match &line_config.text_background.background {
            TextBackgroundType::LinearGradient { stops, angle } => {
                bg_gradient_editor.set_gradient(&LinearGradientConfig {
                    stops: stops.clone(),
                    angle: *angle,
                });
            }
            _ => {
                bg_gradient_editor.set_gradient(&LinearGradientConfig {
                    stops: vec![
                        ColorStop::new(0.0, Color::new(0.0, 0.0, 0.0, 0.7)),
                        ColorStop::new(1.0, Color::new(0.2, 0.2, 0.2, 0.7)),
                    ],
                    angle: 0.0,
                });
            }
        }
        bg_gradient_box.append(bg_gradient_editor.widget());
        bg_gradient_box.set_visible(initial_bg_type_index == 2);
        bg_box.append(&bg_gradient_box);

        bg_frame.set_child(Some(&bg_box));
        row_box.append(&bg_frame);

        // Connect background type combo change handler
        {
            let bg_params_box_clone = bg_params_box.clone();
            let bg_solid_box_clone = bg_solid_box.clone();
            let bg_gradient_box_clone = bg_gradient_box.clone();
            let lines_clone = lines.clone();
            let bg_color_widget_clone = bg_color_widget.clone();
            let bg_gradient_editor_clone = bg_gradient_editor.clone();
            let bg_padding_spin_clone = bg_padding_spin.clone();
            let bg_radius_spin_clone = bg_radius_spin.clone();
            let on_change_bg_type = on_change.clone();
            bg_type_combo.connect_selected_notify(move |combo| {
                let selected = combo.selected();
                bg_params_box_clone.set_visible(selected != 0);
                bg_solid_box_clone.set_visible(selected == 1);
                bg_gradient_box_clone.set_visible(selected == 2);

                let mut lines_ref = lines_clone.borrow_mut();
                if let Some(line) = lines_ref.get_mut(list_index) {
                    let bg_type = match selected {
                        0 => TextBackgroundType::None,
                        1 => TextBackgroundType::Solid {
                            color: bg_color_widget_clone.color(),
                        },
                        2 => {
                            let grad = bg_gradient_editor_clone.get_gradient();
                            TextBackgroundType::LinearGradient {
                                stops: grad.stops,
                                angle: grad.angle,
                            }
                        }
                        _ => TextBackgroundType::None,
                    };
                    line.text_background = TextBackgroundConfig {
                        background: bg_type,
                        padding: bg_padding_spin_clone.value(),
                        corner_radius: bg_radius_spin_clone.value(),
                    };
                }
                drop(lines_ref);
                if let Some(ref callback) = *on_change_bg_type.borrow() {
                    callback();
                }
            });
        }

        // Connect background color change handler
        {
            let lines_clone = lines.clone();
            let bg_padding_spin_clone = bg_padding_spin.clone();
            let bg_radius_spin_clone = bg_radius_spin.clone();
            let on_change_bg_color = on_change.clone();
            bg_color_widget.set_on_change(move |new_color| {
                let mut lines_ref = lines_clone.borrow_mut();
                if let Some(line) = lines_ref.get_mut(list_index) {
                    line.text_background = TextBackgroundConfig {
                        background: TextBackgroundType::Solid { color: new_color },
                        padding: bg_padding_spin_clone.value(),
                        corner_radius: bg_radius_spin_clone.value(),
                    };
                }
                drop(lines_ref);
                if let Some(ref callback) = *on_change_bg_color.borrow() {
                    callback();
                }
            });
        }

        // Connect background gradient editor change handler
        {
            let lines_clone = lines.clone();
            let bg_gradient_editor_clone = bg_gradient_editor.clone();
            let bg_padding_spin_clone = bg_padding_spin.clone();
            let bg_radius_spin_clone = bg_radius_spin.clone();
            let on_change_bg_grad = on_change.clone();
            bg_gradient_editor.set_on_change(move || {
                let grad = bg_gradient_editor_clone.get_gradient();
                let mut lines_ref = lines_clone.borrow_mut();
                if let Some(line) = lines_ref.get_mut(list_index) {
                    line.text_background = TextBackgroundConfig {
                        background: TextBackgroundType::LinearGradient {
                            stops: grad.stops,
                            angle: grad.angle,
                        },
                        padding: bg_padding_spin_clone.value(),
                        corner_radius: bg_radius_spin_clone.value(),
                    };
                }
                drop(lines_ref);
                if let Some(ref callback) = *on_change_bg_grad.borrow() {
                    callback();
                }
            });
        }

        // Connect background padding change handler
        {
            let lines_clone = lines.clone();
            let bg_radius_spin_clone = bg_radius_spin.clone();
            let on_change_padding = on_change.clone();
            bg_padding_spin.connect_value_changed(move |spin| {
                let mut lines_ref = lines_clone.borrow_mut();
                if let Some(line) = lines_ref.get_mut(list_index) {
                    line.text_background.padding = spin.value();
                    // Keep corner radius in sync
                    line.text_background.corner_radius = bg_radius_spin_clone.value();
                }
                drop(lines_ref);
                if let Some(ref callback) = *on_change_padding.borrow() {
                    callback();
                }
            });
        }

        // Connect background corner radius change handler
        {
            let lines_clone = lines.clone();
            let bg_padding_spin_clone = bg_padding_spin.clone();
            let on_change_radius = on_change.clone();
            bg_radius_spin.connect_value_changed(move |spin| {
                let mut lines_ref = lines_clone.borrow_mut();
                if let Some(line) = lines_ref.get_mut(list_index) {
                    line.text_background.corner_radius = spin.value();
                    // Keep padding in sync
                    line.text_background.padding = bg_padding_spin_clone.value();
                }
                drop(lines_ref);
                if let Some(ref callback) = *on_change_radius.borrow() {
                    callback();
                }
            });
        }

        // Rotation angle row
        let rotation_box = GtkBox::new(Orientation::Horizontal, 6);
        rotation_box.append(&Label::new(Some("Rotation Angle:")));
        let angle_spin = SpinButton::with_range(-360.0, 360.0, 5.0);
        angle_spin.set_value(line_config.rotation_angle);
        rotation_box.append(&angle_spin);
        row_box.append(&rotation_box);

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

        // Combine direction handler
        {
            let lines_clone = lines.clone();
            let on_change_dir = on_change.clone();
            direction_combo.connect_selected_notify(move |combo| {
                let dir = match combo.selected() {
                    0 => CombineDirection::Horizontal,
                    1 => CombineDirection::Vertical,
                    _ => CombineDirection::Horizontal,
                };
                let mut lines_ref = lines_clone.borrow_mut();
                if let Some(line) = lines_ref.get_mut(list_index) {
                    line.combine_direction = dir;
                }
                drop(lines_ref);
                if let Some(ref callback) = *on_change_dir.borrow() {
                    callback();
                }
            });
        }

        // Combine alignment handler
        {
            let lines_clone = lines.clone();
            let on_change_align = on_change.clone();
            alignment_grid.set_on_change(move |new_align| {
                let mut lines_ref = lines_clone.borrow_mut();
                if let Some(line) = lines_ref.get_mut(list_index) {
                    line.combine_alignment = new_align;
                }
                drop(lines_ref);
                if let Some(ref callback) = *on_change_align.borrow() {
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
        log::debug!(
            "TextLineConfigWidget::set_config: received {} lines",
            config.lines.len()
        );
        // Update lines vector
        *self.lines.borrow_mut() = config.lines;

        // Rebuild UI
        self.rebuild_list();
    }

    /// Get the current configuration
    pub fn get_config(&self) -> TextDisplayerConfig {
        let lines = self.lines.borrow().clone();
        log::debug!(
            "TextLineConfigWidget::get_config: returning {} lines",
            lines.len()
        );
        TextDisplayerConfig { lines }
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
