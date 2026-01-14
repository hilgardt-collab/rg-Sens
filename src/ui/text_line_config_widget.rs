//! Widget for configuring text displayer lines

use crate::core::FieldMetadata;
use crate::displayers::{
    CombineDirection, TextBackgroundConfig, TextBackgroundType, TextDisplayerConfig, TextFillType,
    TextLineConfig,
};
use crate::ui::background::{Color, ColorStop, LinearGradientConfig};
use crate::ui::gradient_editor::GradientEditor;
use crate::ui::position_grid_widget::PositionGridWidget;
use crate::ui::theme::{ColorSource, ColorStopSource, ComboThemeConfig, FontSource};
use crate::ui::theme_color_selector::ThemeColorSelector;
use crate::ui::theme_font_selector::ThemeFontSelector;
use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, CheckButton, DropDown, Entry, Frame, Label, Orientation, ScrolledWindow,
    SpinButton, Stack, StackSidebar, StringList, Widget,
};
use std::cell::RefCell;
use std::rc::Rc;

/// Widget for configuring text displayer lines
pub struct TextLineConfigWidget {
    widget: GtkBox,
    lines: Rc<RefCell<Vec<TextLineConfig>>>,
    stack: Stack,
    #[allow(dead_code)] // Kept alive for GTK widget lifetime
    sidebar: StackSidebar,
    available_fields: Vec<FieldMetadata>,
    on_change: Rc<RefCell<Option<Box<dyn Fn()>>>>,
    theme: Rc<RefCell<ComboThemeConfig>>,
    fill_color_selectors: Rc<RefCell<Vec<Rc<ThemeColorSelector>>>>,
    fill_gradient_editors: Rc<RefCell<Vec<Rc<GradientEditor>>>>,
    font_selectors: Rc<RefCell<Vec<Rc<ThemeFontSelector>>>>,
}

impl TextLineConfigWidget {
    /// Create a new text line configuration widget
    pub fn new(available_fields: Vec<FieldMetadata>) -> Self {
        log::debug!(
            "TextLineConfigWidget::new() called with {} available fields",
            available_fields.len()
        );

        // Main vertical container
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

        // Horizontal container for sidebar + stack
        let content_box = GtkBox::new(Orientation::Horizontal, 6);
        content_box.set_vexpand(true);

        // Create Stack for content pages
        let stack = Stack::new();
        stack.set_hexpand(true);
        stack.set_vexpand(true);
        stack.set_transition_type(gtk4::StackTransitionType::Crossfade);
        stack.set_transition_duration(150);

        // Create StackSidebar for navigation
        let sidebar = StackSidebar::new();
        sidebar.set_stack(&stack);
        sidebar.set_size_request(120, -1);

        // Add sidebar (left) and stack (right) to content box
        content_box.append(&sidebar);

        // Wrap stack in scrolled window for long content
        let stack_scrolled = ScrolledWindow::new();
        stack_scrolled.set_hexpand(true);
        stack_scrolled.set_vexpand(true);
        stack_scrolled.set_child(Some(&stack));
        content_box.append(&stack_scrolled);

        widget.append(&content_box);

        let lines = Rc::new(RefCell::new(Vec::new()));
        let on_change: Rc<RefCell<Option<Box<dyn Fn()>>>> = Rc::new(RefCell::new(None));
        let theme = Rc::new(RefCell::new(ComboThemeConfig::default()));
        let fill_color_selectors: Rc<RefCell<Vec<Rc<ThemeColorSelector>>>> =
            Rc::new(RefCell::new(Vec::new()));
        let fill_gradient_editors: Rc<RefCell<Vec<Rc<GradientEditor>>>> =
            Rc::new(RefCell::new(Vec::new()));
        let font_selectors: Rc<RefCell<Vec<Rc<ThemeFontSelector>>>> =
            Rc::new(RefCell::new(Vec::new()));

        // Set up add button - uses a self-contained rebuild callback
        let lines_for_add = lines.clone();
        let stack_for_add = stack.clone();
        let fields_for_add = available_fields.clone();
        let on_change_for_rebuild = on_change.clone();

        // Create self-referential rebuild callback for add
        let rebuild_fn: Rc<RefCell<Option<Rc<dyn Fn()>>>> = Rc::new(RefCell::new(None));

        let rebuild_closure: Rc<dyn Fn()> = Rc::new({
            let lines_inner = lines_for_add.clone();
            let stack_inner = stack_for_add.clone();
            let fields_inner = fields_for_add.clone();
            let rebuild_fn_inner = rebuild_fn.clone();
            let on_change_inner = on_change_for_rebuild.clone();
            let theme_inner = theme.clone();
            let font_selectors_inner = font_selectors.clone();
            let fill_color_selectors_inner = fill_color_selectors.clone();
            let fill_gradient_editors_inner = fill_gradient_editors.clone();

            move || {
                // Save current visible page name before rebuild
                let current_page = stack_inner.visible_child_name().map(|s| s.to_string());

                // Clear all stack pages
                while let Some(child) = stack_inner.first_child() {
                    stack_inner.remove(&child);
                }

                // Clear font and color selectors when rebuilding
                font_selectors_inner.borrow_mut().clear();
                fill_color_selectors_inner.borrow_mut().clear();
                fill_gradient_editors_inner.borrow_mut().clear();

                // Rebuild all pages with the same rebuild callback
                let lines_data = lines_inner.borrow().clone();
                let callback_to_pass = rebuild_fn_inner.borrow().clone();
                for (index, line) in lines_data.into_iter().enumerate() {
                    Self::add_line_page(
                        &stack_inner,
                        line,
                        &fields_inner,
                        lines_inner.clone(),
                        index,
                        callback_to_pass.clone(),
                        on_change_inner.clone(),
                        theme_inner.clone(),
                        font_selectors_inner.clone(),
                        fill_color_selectors_inner.clone(),
                        fill_gradient_editors_inner.clone(),
                    );
                }

                // Restore the previously visible page if it still exists, otherwise stay on current
                if let Some(page_name) = current_page {
                    // Check if the page still exists (it might have been deleted)
                    if stack_inner.child_by_name(&page_name).is_some() {
                        stack_inner.set_visible_child_name(&page_name);
                    }
                }
            }
        });

        // Store the callback so it can reference itself
        *rebuild_fn.borrow_mut() = Some(rebuild_closure.clone());

        let on_change_for_add = on_change.clone();
        let stack_for_add_btn = stack.clone();
        add_button.connect_clicked(move |_| {
            log::info!("=== TextLineConfigWidget: Add Line clicked ===");
            // Add new line to data
            let new_line_index = {
                let mut lines = lines_for_add.borrow_mut();
                let new_line = TextLineConfig::default();
                log::info!(
                    "    Adding new line with default field_id='{}'",
                    new_line.field_id
                );
                lines.push(new_line);
                log::info!("    Total lines now: {}", lines.len());
                lines.len() - 1
            };
            // Trigger full rebuild so the new line has the rebuild callback
            rebuild_closure();
            // Select the newly added page
            let page_name = format!("line_{}", new_line_index);
            stack_for_add_btn.set_visible_child_name(&page_name);
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

        // Paste button handler - needs to rebuild stack
        let lines_for_paste = lines.clone();
        let stack_for_paste = stack.clone();
        let fields_for_paste = available_fields.clone();
        let on_change_for_paste = on_change.clone();
        let rebuild_fn_for_paste = rebuild_fn.clone();
        let theme_for_paste = theme.clone();
        let font_selectors_for_paste = font_selectors.clone();
        let fill_color_selectors_for_paste = fill_color_selectors.clone();
        let fill_gradient_editors_for_paste = fill_gradient_editors.clone();
        paste_btn.connect_clicked(move |_| {
            let pasted = if let Ok(clipboard) = crate::ui::clipboard::CLIPBOARD.lock() {
                clipboard.paste_text_display()
            } else {
                None
            };

            if let Some(config) = pasted {
                // Update lines
                *lines_for_paste.borrow_mut() = config.lines;

                // Clear font and color selectors when rebuilding
                font_selectors_for_paste.borrow_mut().clear();
                fill_color_selectors_for_paste.borrow_mut().clear();
                fill_gradient_editors_for_paste.borrow_mut().clear();

                // Rebuild stack
                while let Some(child) = stack_for_paste.first_child() {
                    stack_for_paste.remove(&child);
                }
                let lines_data = lines_for_paste.borrow().clone();
                let callback_to_pass = rebuild_fn_for_paste.borrow().clone();
                for (index, line) in lines_data.into_iter().enumerate() {
                    Self::add_line_page(
                        &stack_for_paste,
                        line,
                        &fields_for_paste,
                        lines_for_paste.clone(),
                        index,
                        callback_to_pass.clone(),
                        on_change_for_paste.clone(),
                        theme_for_paste.clone(),
                        font_selectors_for_paste.clone(),
                        fill_color_selectors_for_paste.clone(),
                        fill_gradient_editors_for_paste.clone(),
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
            stack,
            sidebar,
            available_fields,
            on_change,
            theme,
            fill_color_selectors,
            fill_gradient_editors,
            font_selectors,
        }
    }

    /// Add a page for a text line to the stack
    fn add_line_page(
        stack: &Stack,
        line_config: TextLineConfig,
        fields: &[FieldMetadata],
        lines: Rc<RefCell<Vec<TextLineConfig>>>,
        list_index: usize,
        rebuild_callback: Option<Rc<dyn Fn()>>,
        on_change: Rc<RefCell<Option<Box<dyn Fn()>>>>,
        theme: Rc<RefCell<ComboThemeConfig>>,
        font_selectors: Rc<RefCell<Vec<Rc<ThemeFontSelector>>>>,
        fill_color_selectors: Rc<RefCell<Vec<Rc<ThemeColorSelector>>>>,
        fill_gradient_editors: Rc<RefCell<Vec<Rc<GradientEditor>>>>,
    ) {
        let row_box = GtkBox::new(Orientation::Vertical, 6);
        row_box.set_margin_top(6);
        row_box.set_margin_bottom(6);
        row_box.set_margin_start(6);
        row_box.set_margin_end(6);

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
        let rebuild_pos = rebuild_callback.clone();
        position_grid.set_on_change(move |new_pos| {
            let mut lines_ref = lines_clone_pos.borrow_mut();
            if let Some(line) = lines_ref.get_mut(list_index) {
                line.position = new_pos;
            }
            drop(lines_ref);
            // Rebuild to update direction/alignment visibility for all lines
            if let Some(ref rebuild_fn) = rebuild_pos {
                rebuild_fn();
            }
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
        // 2. Must be the FIRST line in the group with the same position
        // 3. There must be 2+ lines in the group with the SAME position
        let should_show_dir_align = {
            let all_lines = lines.borrow();
            // Use current data from lines, not the stale line_config copy
            let current_line = all_lines.get(list_index);
            if let Some(current_line) = current_line {
                if !current_line.is_combined || current_line.group_id.is_none() {
                    false
                } else {
                    let group_id = current_line.group_id.as_ref().unwrap();
                    let this_position = current_line.position;

                    // Find first line in group with same position and count lines in group with same position
                    let mut first_index_in_group_with_pos: Option<usize> = None;
                    let mut group_count_with_same_pos = 0;

                    for (i, line) in all_lines.iter().enumerate() {
                        if line.is_combined
                            && line.group_id.as_ref() == Some(group_id)
                            && line.position == this_position
                        {
                            // Track first line in group with same position
                            if first_index_in_group_with_pos.is_none() {
                                first_index_in_group_with_pos = Some(i);
                            }
                            group_count_with_same_pos += 1;
                        }
                    }

                    // Show only if this is the first line in group with same position AND there are 2+ such lines
                    first_index_in_group_with_pos == Some(list_index)
                        && group_count_with_same_pos >= 2
                }
            } else {
                false
            }
        };

        direction_align_box.set_visible(should_show_dir_align);
        row_box.append(&direction_align_box);

        // Font controls using ThemeFontSelector
        let font_box = GtkBox::new(Orientation::Horizontal, 6);
        font_box.append(&Label::new(Some("Font:")));

        // Create ThemeFontSelector with current font_source
        let initial_font_source = line_config.get_font_source();
        let font_selector = Rc::new(ThemeFontSelector::new(initial_font_source));

        // Set theme config if available
        font_selector.set_theme_config(theme.borrow().clone());

        font_box.append(font_selector.widget());

        // Store in font_selectors for theme updates
        font_selectors.borrow_mut().push(font_selector.clone());

        // Connect font selector callback to update font_source
        let lines_clone_font = lines.clone();
        let on_change_font = on_change.clone();
        font_selector.set_on_change(move |source| {
            let mut lines_ref = lines_clone_font.borrow_mut();
            if let Some(line) = lines_ref.get_mut(list_index) {
                // Update font_source
                line.font_source = Some(source.clone());
                // Also update legacy fields for backward compatibility
                match &source {
                    FontSource::Custom { family, size } => {
                        line.font_family = family.clone();
                        line.font_size = *size;
                    }
                    FontSource::Theme { .. } => {
                        // Keep legacy fields as-is for theme fonts
                    }
                }
            }
            drop(lines_ref);
            if let Some(ref callback) = *on_change_font.borrow() {
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

        // Copy font button - preserves FontSource (Theme or Custom)
        let copy_font_btn = Button::with_label("Copy");
        let lines_clone_copy_font = lines.clone();
        copy_font_btn.connect_clicked(move |_| {
            let lines_ref = lines_clone_copy_font.borrow();
            if let Some(line) = lines_ref.get(list_index) {
                if let Ok(mut clipboard) = crate::ui::clipboard::CLIPBOARD.lock() {
                    // Copy FontSource to preserve theme reference
                    if let Some(ref source) = line.font_source {
                        clipboard.copy_font_source(source.clone(), line.bold, line.italic);
                    } else {
                        // Fallback to legacy fields as Custom
                        let source = FontSource::Custom {
                            family: line.font_family.clone(),
                            size: line.font_size,
                        };
                        clipboard.copy_font_source(source, line.bold, line.italic);
                    }
                }
            }
        });
        font_box.append(&copy_font_btn);

        // Paste font button - preserves FontSource (Theme or Custom)
        let paste_font_btn = Button::with_label("Paste");
        let lines_clone_paste_font = lines.clone();
        let font_selector_clone = font_selector.clone();
        let bold_check_clone = bold_check.clone();
        let italic_check_clone = italic_check.clone();
        let on_change_paste = on_change.clone();
        let theme_paste = theme.clone();
        paste_font_btn.connect_clicked(move |_| {
            if let Ok(clipboard) = crate::ui::clipboard::CLIPBOARD.lock() {
                if let Some((source, bold, italic)) = clipboard.paste_font_source() {
                    let mut lines_ref = lines_clone_paste_font.borrow_mut();
                    if let Some(line) = lines_ref.get_mut(list_index) {
                        line.font_source = Some(source.clone());
                        // Update legacy fields for compatibility
                        let (family, size) = source.resolve(&theme_paste.borrow());
                        line.font_family = family;
                        line.font_size = size;
                        line.bold = bold;
                        line.italic = italic;
                    }
                    drop(lines_ref);

                    // Update font selector and bold/italic checks
                    font_selector_clone.set_source(source);
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
        let initial_color_source = line_config.fill.color_source();
        let fill_color_widget = Rc::new(ThemeColorSelector::new(initial_color_source.clone()));

        // Set theme config and store in fill_color_selectors for theme updates
        fill_color_widget.set_theme_config(theme.borrow().clone());
        fill_color_selectors
            .borrow_mut()
            .push(fill_color_widget.clone());

        solid_fill_box.append(fill_color_widget.widget());
        solid_fill_box.set_visible(initial_fill_type_index == 0);
        fill_box.append(&solid_fill_box);

        // Gradient fill container
        let gradient_fill_box = GtkBox::new(Orientation::Vertical, 6);
        let fill_gradient_editor = Rc::new(GradientEditor::new());
        // Set theme config on gradient editor so theme colors show correctly
        fill_gradient_editor.set_theme_config(theme.borrow().clone());
        // Store in fill_gradient_editors for theme updates
        fill_gradient_editors
            .borrow_mut()
            .push(fill_gradient_editor.clone());
        // Initialize gradient editor with current value or defaults
        match &line_config.fill {
            TextFillType::LinearGradient { stops, angle } => {
                fill_gradient_editor.set_gradient(&LinearGradientConfig {
                    stops: stops.clone(),
                    angle: *angle,
                });
            }
            TextFillType::Solid { .. } => {
                // Default 2-stop gradient using the resolved solid color
                let resolved_color = line_config.fill.primary_color(None);
                fill_gradient_editor.set_gradient(&LinearGradientConfig {
                    stops: vec![
                        ColorStop::new(0.0, resolved_color),
                        ColorStop::new(
                            1.0,
                            Color::new(
                                resolved_color.r * 0.5,
                                resolved_color.g * 0.5,
                                resolved_color.b * 0.5,
                                resolved_color.a,
                            ),
                        ),
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
                            let color = fill_color_widget_clone.source();
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

        // Background solid color container (theme-aware)
        let bg_solid_box = GtkBox::new(Orientation::Horizontal, 6);
        bg_solid_box.append(&Label::new(Some("Color:")));
        let bg_color_source = match &line_config.text_background.background {
            TextBackgroundType::Solid { color } => color.clone(),
            _ => ColorSource::Custom {
                color: Color::new(0.0, 0.0, 0.0, 0.5),
            },
        };
        let bg_color_widget = Rc::new(ThemeColorSelector::new(bg_color_source));
        // Set theme config and store in fill_color_selectors for theme updates
        bg_color_widget.set_theme_config(theme.borrow().clone());
        fill_color_selectors
            .borrow_mut()
            .push(bg_color_widget.clone());
        bg_solid_box.append(bg_color_widget.widget());
        bg_solid_box.set_visible(initial_bg_type_index == 1);
        bg_box.append(&bg_solid_box);

        // Background gradient container
        let bg_gradient_box = GtkBox::new(Orientation::Vertical, 6);
        let bg_gradient_editor = Rc::new(GradientEditor::new());
        // Set theme config on gradient editor so theme colors show correctly
        bg_gradient_editor.set_theme_config(theme.borrow().clone());
        // Store in fill_gradient_editors for theme updates (also used for background gradients)
        fill_gradient_editors
            .borrow_mut()
            .push(bg_gradient_editor.clone());
        match &line_config.text_background.background {
            TextBackgroundType::LinearGradient { stops, angle } => {
                // Use set_gradient_source to preserve theme color references
                bg_gradient_editor.set_gradient_source(*angle, stops.clone());
            }
            _ => {
                // Default gradient with custom colors
                bg_gradient_editor.set_stops_source(vec![
                    ColorStopSource::custom(0.0, Color::new(0.0, 0.0, 0.0, 0.7)),
                    ColorStopSource::custom(1.0, Color::new(0.2, 0.2, 0.2, 0.7)),
                ]);
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
                            color: bg_color_widget_clone.source(),
                        },
                        2 => {
                            // Use get_gradient_source_config to preserve theme color references
                            let grad = bg_gradient_editor_clone.get_gradient_source_config();
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

        // Connect background color change handler (theme-aware)
        {
            let lines_clone = lines.clone();
            let bg_padding_spin_clone = bg_padding_spin.clone();
            let bg_radius_spin_clone = bg_radius_spin.clone();
            let on_change_bg_color = on_change.clone();
            bg_color_widget.set_on_change(move |new_source| {
                let mut lines_ref = lines_clone.borrow_mut();
                if let Some(line) = lines_ref.get_mut(list_index) {
                    line.text_background = TextBackgroundConfig {
                        background: TextBackgroundType::Solid { color: new_source },
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
                // Use get_gradient_source_config to preserve theme color references
                let grad = bg_gradient_editor_clone.get_gradient_source_config();
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
            "Group 1", "Group 2", "Group 3", "Group 4", "Group 5", "Group 6", "Group 7", "Group 8",
            "Custom",
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
            // Default to Group 1 and set it in the data so matching works
            let mut lines_ref = lines.borrow_mut();
            if let Some(line) = lines_ref.get_mut(list_index) {
                line.group_id = Some("Group 1".to_string());
            }
            drop(lines_ref);
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

        // Helper function to check if this is the first line in a group with same position
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

            // Check if there's any earlier line with the same group_id AND same position
            let group_id = current_line.group_id.as_ref().unwrap();
            let position = current_line.position;
            for (i, line) in lines.iter().enumerate() {
                if i >= index {
                    break;
                }
                if line.is_combined
                    && line.group_id.as_ref() == Some(group_id)
                    && line.position == position
                {
                    return false; // Found an earlier line in the same group at same position
                }
            }
            true // This is the first line in the group at this position
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

            // Call rebuild callback to refresh the entire stack with correct indices
            if let Some(ref rebuild) = rebuild_callback {
                rebuild();
            }
            if let Some(ref callback) = *on_change_delete.borrow() {
                callback();
            }
        });

        // Add page to stack with unique name and display title
        let page_name = format!("line_{}", list_index);
        let page_title = format!("Line {}", list_index + 1);
        stack.add_titled(&row_box, Some(&page_name), &page_title);
    }

    /// Rebuild the entire stack UI from the current lines data
    fn rebuild_stack(&self) {
        // Clear all stack pages
        while let Some(child) = self.stack.first_child() {
            self.stack.remove(&child);
        }

        // Clear font and color selectors when rebuilding
        self.font_selectors.borrow_mut().clear();
        self.fill_color_selectors.borrow_mut().clear();
        self.fill_gradient_editors.borrow_mut().clear();

        // Create rebuild callback for delete buttons
        let stack_clone = self.stack.clone();
        let lines_clone = self.lines.clone();
        let fields_clone = self.available_fields.clone();
        let on_change_clone = self.on_change.clone();
        let theme_clone = self.theme.clone();
        let font_selectors_clone = self.font_selectors.clone();
        let fill_color_selectors_clone = self.fill_color_selectors.clone();
        let fill_gradient_editors_clone = self.fill_gradient_editors.clone();

        // Create rebuild function as Rc<RefCell> to allow self-reference
        let rebuild_fn: Rc<RefCell<Option<Rc<dyn Fn()>>>> = Rc::new(RefCell::new(None));
        let rebuild_fn_clone = rebuild_fn.clone();
        let on_change_for_rebuild = on_change_clone.clone();
        let theme_for_rebuild = theme_clone.clone();
        let font_selectors_for_rebuild = font_selectors_clone.clone();
        let fill_color_selectors_for_rebuild = fill_color_selectors_clone.clone();
        let fill_gradient_editors_for_rebuild = fill_gradient_editors_clone.clone();

        let rebuild_closure: Rc<dyn Fn()> = Rc::new(move || {
            // Save current visible page name before rebuild
            let current_page = stack_clone.visible_child_name().map(|s| s.to_string());

            // Clear all stack pages
            while let Some(child) = stack_clone.first_child() {
                stack_clone.remove(&child);
            }

            // Clear font and color selectors when rebuilding
            font_selectors_for_rebuild.borrow_mut().clear();
            fill_color_selectors_for_rebuild.borrow_mut().clear();
            fill_gradient_editors_for_rebuild.borrow_mut().clear();

            // Rebuild all pages with the same rebuild callback
            let lines_data = lines_clone.borrow().clone();
            let callback_to_pass = rebuild_fn_clone.borrow().clone();
            for (index, line) in lines_data.into_iter().enumerate() {
                Self::add_line_page(
                    &stack_clone,
                    line,
                    &fields_clone,
                    lines_clone.clone(),
                    index,
                    callback_to_pass.clone(),
                    on_change_for_rebuild.clone(),
                    theme_for_rebuild.clone(),
                    font_selectors_for_rebuild.clone(),
                    fill_color_selectors_for_rebuild.clone(),
                    fill_gradient_editors_for_rebuild.clone(),
                );
            }

            // Restore the previously visible page if it still exists
            if let Some(page_name) = current_page {
                if stack_clone.child_by_name(&page_name).is_some() {
                    stack_clone.set_visible_child_name(&page_name);
                }
            }
        });

        // Store the callback in the RefCell
        *rebuild_fn.borrow_mut() = Some(rebuild_closure.clone());

        // Rebuild all pages with correct indices
        let lines = self.lines.borrow().clone();
        for (index, line) in lines.into_iter().enumerate() {
            Self::add_line_page(
                &self.stack,
                line,
                &self.available_fields,
                self.lines.clone(),
                index,
                Some(rebuild_closure.clone()),
                on_change_clone.clone(),
                theme_clone.clone(),
                font_selectors_clone.clone(),
                fill_color_selectors_clone.clone(),
                fill_gradient_editors_clone.clone(),
            );
        }

        // Select first page if available
        if !self.lines.borrow().is_empty() {
            self.stack.set_visible_child_name("line_0");
        }
    }

    /// Set the configuration
    pub fn set_config(&self, config: TextDisplayerConfig) {
        // Update lines vector
        *self.lines.borrow_mut() = config.lines;

        // Rebuild UI
        self.rebuild_stack();
    }

    /// Get the current configuration
    pub fn get_config(&self) -> TextDisplayerConfig {
        let lines = self.lines.borrow().clone();
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

    /// Set the theme config for resolving theme colors and fonts
    pub fn set_theme(&self, theme: ComboThemeConfig) {
        *self.theme.borrow_mut() = theme.clone();
        // Update all fill color selectors
        for selector in self.fill_color_selectors.borrow().iter() {
            selector.set_theme_config(theme.clone());
        }
        // Update all fill gradient editors
        for editor in self.fill_gradient_editors.borrow().iter() {
            editor.set_theme_config(theme.clone());
        }
        // Update all font selectors
        for selector in self.font_selectors.borrow().iter() {
            selector.set_theme_config(theme.clone());
        }
        // Notify parent to refresh with new theme colors/fonts
        if let Some(callback) = self.on_change.borrow().as_ref() {
            callback();
        }
    }
}

// =============================================================================
// LazyTextLineConfigWidget - Delays creation of TextLineConfigWidget until needed
// =============================================================================

/// A lazy-loading wrapper for TextLineConfigWidget that defers expensive widget creation
/// until the user actually clicks to expand/configure the text lines.
///
/// This significantly improves dialog open time for combo panels with many slots,
/// as TextLineConfigWidget creation is deferred until needed.
pub struct LazyTextLineConfigWidget {
    /// Container that holds either the placeholder or the actual widget
    container: GtkBox,
    /// The actual widget, created lazily on first expand
    inner_widget: Rc<RefCell<Option<TextLineConfigWidget>>>,
    /// Deferred config to apply when widget is created
    deferred_config: Rc<RefCell<TextDisplayerConfig>>,
    /// Deferred theme to apply when widget is created
    deferred_theme: Rc<RefCell<ComboThemeConfig>>,
    /// Available fields for the widget
    available_fields: Vec<FieldMetadata>,
    /// Callback to invoke on config changes
    on_change: Rc<RefCell<Option<Box<dyn Fn()>>>>,
}

impl LazyTextLineConfigWidget {
    /// Create a new lazy text line config widget
    ///
    /// The actual TextLineConfigWidget is NOT created here - it's created automatically
    /// when the widget becomes visible (mapped), or when explicitly initialized.
    pub fn new(available_fields: Vec<FieldMetadata>) -> Self {
        let container = GtkBox::new(Orientation::Vertical, 0);
        let inner_widget: Rc<RefCell<Option<TextLineConfigWidget>>> = Rc::new(RefCell::new(None));
        let deferred_config = Rc::new(RefCell::new(TextDisplayerConfig::default()));
        let deferred_theme = Rc::new(RefCell::new(ComboThemeConfig::default()));
        let on_change: Rc<RefCell<Option<Box<dyn Fn()>>>> = Rc::new(RefCell::new(None));

        // Create placeholder with loading indicator
        let placeholder = GtkBox::new(Orientation::Vertical, 8);
        placeholder.set_margin_top(12);
        placeholder.set_margin_bottom(12);
        placeholder.set_margin_start(12);
        placeholder.set_margin_end(12);

        let info_label = Label::new(Some("Loading text configuration..."));
        info_label.add_css_class("dim-label");
        placeholder.append(&info_label);
        container.append(&placeholder);

        // Create a shared initialization closure
        let init_widget = {
            let container_clone = container.clone();
            let inner_widget_clone = inner_widget.clone();
            let deferred_config_clone = deferred_config.clone();
            let deferred_theme_clone = deferred_theme.clone();
            let available_fields_clone = available_fields.clone();
            let on_change_clone = on_change.clone();

            Rc::new(move || {
                // Only create if not already created
                if inner_widget_clone.borrow().is_none() {
                    log::info!(
                        "LazyTextLineConfigWidget: Creating actual TextLineConfigWidget on map"
                    );

                    // Create the actual widget
                    let widget = TextLineConfigWidget::new(available_fields_clone.clone());

                    // Apply deferred theme first (before config, as config may trigger UI updates)
                    widget.set_theme(deferred_theme_clone.borrow().clone());

                    // Apply deferred config
                    widget.set_config(deferred_config_clone.borrow().clone());

                    // Connect on_change callback
                    let on_change_inner = on_change_clone.clone();
                    widget.set_on_change(move || {
                        if let Some(ref callback) = *on_change_inner.borrow() {
                            callback();
                        }
                    });

                    // Remove placeholder and add actual widget
                    while let Some(child) = container_clone.first_child() {
                        container_clone.remove(&child);
                    }
                    container_clone.append(widget.widget());

                    // Store the widget
                    *inner_widget_clone.borrow_mut() = Some(widget);
                }
            })
        };

        // Auto-initialize when the widget becomes visible (mapped)
        {
            let init_widget_clone = init_widget.clone();
            container.connect_map(move |_| {
                init_widget_clone();
            });
        }

        Self {
            container,
            inner_widget,
            deferred_config,
            deferred_theme,
            available_fields,
            on_change,
        }
    }

    /// Get the widget container
    pub fn widget(&self) -> &Widget {
        self.container.upcast_ref()
    }

    /// Set the text configuration
    ///
    /// If the inner widget hasn't been created yet, this stores the config
    /// to be applied when it is created. Otherwise, it's applied immediately.
    pub fn set_config(&self, config: TextDisplayerConfig) {
        *self.deferred_config.borrow_mut() = config.clone();
        if let Some(ref widget) = *self.inner_widget.borrow() {
            widget.set_config(config);
        }
    }

    /// Get the current text configuration
    ///
    /// Returns the deferred config if the inner widget hasn't been created yet.
    pub fn get_config(&self) -> TextDisplayerConfig {
        if let Some(ref widget) = *self.inner_widget.borrow() {
            widget.get_config()
        } else {
            self.deferred_config.borrow().clone()
        }
    }

    /// Set the theme for the text widget
    pub fn set_theme(&self, theme: ComboThemeConfig) {
        *self.deferred_theme.borrow_mut() = theme.clone();
        // Note: TextDisplayerConfig doesn't have a theme field - theme is stored separately
        // in deferred_theme and applied to individual lines when widget is created

        if let Some(ref widget) = *self.inner_widget.borrow() {
            widget.set_theme(theme);
        } else {
            // Even if inner widget doesn't exist, trigger on_change so stored config is updated
            if let Some(ref callback) = *self.on_change.borrow() {
                callback();
            }
        }
    }

    /// Set the on_change callback
    pub fn set_on_change<F: Fn() + 'static>(&self, callback: F) {
        *self.on_change.borrow_mut() = Some(Box::new(callback));
        // If widget already exists, connect it
        if let Some(ref widget) = *self.inner_widget.borrow() {
            let on_change_inner = self.on_change.clone();
            widget.set_on_change(move || {
                if let Some(ref cb) = *on_change_inner.borrow() {
                    cb();
                }
            });
        }
    }

    /// Check if the actual widget has been created
    pub fn is_initialized(&self) -> bool {
        self.inner_widget.borrow().is_some()
    }

    /// Force initialization of the inner widget (for cases where it must exist)
    pub fn ensure_initialized(&self) {
        if self.inner_widget.borrow().is_none() {
            log::info!("LazyTextLineConfigWidget: Force-initializing TextLineConfigWidget");

            let widget = TextLineConfigWidget::new(self.available_fields.clone());
            widget.set_theme(self.deferred_theme.borrow().clone());
            widget.set_config(self.deferred_config.borrow().clone());

            // Connect on_change
            let on_change_inner = self.on_change.clone();
            widget.set_on_change(move || {
                if let Some(ref callback) = *on_change_inner.borrow() {
                    callback();
                }
            });

            // Remove placeholder and add actual widget
            while let Some(child) = self.container.first_child() {
                self.container.remove(&child);
            }
            self.container.append(widget.widget());

            *self.inner_widget.borrow_mut() = Some(widget);
        }
    }
}
