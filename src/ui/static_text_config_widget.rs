//! Configuration widget for Static Text source

use gtk4::prelude::*;
use gtk4::{
    Adjustment, Box as GtkBox, Button, Entry, Frame, Label, ListBox, Orientation,
    ScrolledWindow, SpinButton,
};
use std::cell::RefCell;
use std::rc::Rc;

use crate::sources::{StaticTextLine, StaticTextSourceConfig};

/// Widget for configuring Static Text source
pub struct StaticTextConfigWidget {
    widget: GtkBox,
    config: Rc<RefCell<StaticTextSourceConfig>>,
    list_box: ListBox,
    update_interval_spin: SpinButton,
    custom_caption_entry: Entry,
}

impl StaticTextConfigWidget {
    pub fn new() -> Self {
        let widget = GtkBox::new(Orientation::Vertical, 12);
        widget.set_margin_start(12);
        widget.set_margin_end(12);
        widget.set_margin_top(12);
        widget.set_margin_bottom(12);

        let config = Rc::new(RefCell::new(StaticTextSourceConfig::default()));

        // Info label
        let info_label = Label::new(Some("Configure custom static text lines for display."));
        info_label.set_halign(gtk4::Align::Start);
        info_label.add_css_class("dim-label");
        widget.append(&info_label);

        // Custom caption
        let caption_box = GtkBox::new(Orientation::Horizontal, 6);
        caption_box.append(&Label::new(Some("Custom Caption:")));

        let custom_caption_entry = Entry::new();
        custom_caption_entry.set_placeholder_text(Some("(auto-generated from first line)"));
        custom_caption_entry.set_hexpand(true);
        caption_box.append(&custom_caption_entry);
        widget.append(&caption_box);

        // Update interval
        let interval_box = GtkBox::new(Orientation::Horizontal, 6);
        interval_box.append(&Label::new(Some("Update Interval (ms):")));

        let interval_adjustment = Adjustment::new(1000.0, 100.0, 60000.0, 100.0, 1000.0, 0.0);
        let update_interval_spin = SpinButton::new(Some(&interval_adjustment), 100.0, 0);
        update_interval_spin.set_hexpand(true);

        interval_box.append(&update_interval_spin);
        widget.append(&interval_box);

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

        // Create rebuild function for managing list
        let rebuild_fn: Rc<RefCell<Option<Rc<dyn Fn()>>>> = Rc::new(RefCell::new(None));

        let config_for_rebuild = config.clone();
        let list_box_for_rebuild = list_box.clone();
        let rebuild_fn_for_closure = rebuild_fn.clone();

        let rebuild_closure: Rc<dyn Fn()> = Rc::new(move || {
            // Clear list box
            while let Some(child) = list_box_for_rebuild.first_child() {
                list_box_for_rebuild.remove(&child);
            }

            // Rebuild all rows
            let lines = config_for_rebuild.borrow().lines.clone();
            let callback_to_pass = rebuild_fn_for_closure.borrow().clone();
            for (index, line) in lines.into_iter().enumerate() {
                Self::add_line_row(
                    &list_box_for_rebuild,
                    line,
                    config_for_rebuild.clone(),
                    index,
                    callback_to_pass.clone(),
                );
            }
        });

        // Store the callback so it can reference itself
        *rebuild_fn.borrow_mut() = Some(rebuild_closure.clone());

        // Add button handler
        let config_for_add = config.clone();
        let rebuild_for_add = rebuild_closure.clone();
        add_button.connect_clicked(move |_| {
            let new_line_num = {
                let cfg = config_for_add.borrow();
                cfg.lines.len() + 1
            };

            let new_line = StaticTextLine {
                field_id: format!("line{}", new_line_num),
                text: format!("Line {}", new_line_num),
                label: format!("Line {}", new_line_num),
            };

            config_for_add.borrow_mut().lines.push(new_line);
            rebuild_for_add();
        });

        // Wire up update interval handler
        let config_for_interval = config.clone();
        update_interval_spin.connect_value_changed(move |spin| {
            config_for_interval.borrow_mut().update_interval_ms = spin.value() as u64;
        });

        // Wire up custom caption handler
        let config_for_caption = config.clone();
        custom_caption_entry.connect_changed(move |entry| {
            let text = entry.text().to_string();
            config_for_caption.borrow_mut().custom_caption = if text.is_empty() {
                None
            } else {
                Some(text)
            };
        });

        // Build initial list
        rebuild_closure();

        Self {
            widget,
            config,
            list_box,
            update_interval_spin,
            custom_caption_entry,
        }
    }

    /// Add a row for a text line
    fn add_line_row(
        list_box: &ListBox,
        line_config: StaticTextLine,
        config: Rc<RefCell<StaticTextSourceConfig>>,
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

        // Field ID row
        let field_id_box = GtkBox::new(Orientation::Horizontal, 6);
        field_id_box.append(&Label::new(Some("Field ID:")));

        let field_id_entry = Entry::new();
        field_id_entry.set_text(&line_config.field_id);
        field_id_entry.set_hexpand(true);
        field_id_entry.set_placeholder_text(Some("e.g., line1"));
        field_id_box.append(&field_id_entry);
        row_box.append(&field_id_box);

        // Label row
        let label_box = GtkBox::new(Orientation::Horizontal, 6);
        label_box.append(&Label::new(Some("Label:")));

        let label_entry = Entry::new();
        label_entry.set_text(&line_config.label);
        label_entry.set_hexpand(true);
        label_entry.set_placeholder_text(Some("Human-readable name"));
        label_box.append(&label_entry);
        row_box.append(&label_box);

        // Text content row
        let text_box = GtkBox::new(Orientation::Horizontal, 6);
        text_box.append(&Label::new(Some("Text:")));

        let text_entry = Entry::new();
        text_entry.set_text(&line_config.text);
        text_entry.set_hexpand(true);
        text_entry.set_placeholder_text(Some("The text to display"));
        text_box.append(&text_entry);
        row_box.append(&text_box);

        // Wire up field_id change handler
        let config_for_field_id = config.clone();
        field_id_entry.connect_changed(move |entry| {
            let mut cfg = config_for_field_id.borrow_mut();
            if let Some(line) = cfg.lines.get_mut(list_index) {
                line.field_id = entry.text().to_string();
            }
        });

        // Wire up label change handler
        let config_for_label = config.clone();
        label_entry.connect_changed(move |entry| {
            let mut cfg = config_for_label.borrow_mut();
            if let Some(line) = cfg.lines.get_mut(list_index) {
                line.label = entry.text().to_string();
            }
        });

        // Wire up text change handler
        let config_for_text = config.clone();
        text_entry.connect_changed(move |entry| {
            let mut cfg = config_for_text.borrow_mut();
            if let Some(line) = cfg.lines.get_mut(list_index) {
                line.text = entry.text().to_string();
            }
        });

        // Delete button
        let delete_button = Button::with_label("Remove Line");
        delete_button.add_css_class("destructive-action");
        row_box.append(&delete_button);

        // Delete button handler
        let config_for_delete = config.clone();
        delete_button.connect_clicked(move |_| {
            {
                let mut cfg = config_for_delete.borrow_mut();
                if list_index < cfg.lines.len() {
                    cfg.lines.remove(list_index);
                }
            }
            // Call rebuild callback to refresh the entire list with correct indices
            if let Some(ref rebuild) = rebuild_callback {
                rebuild();
            }
        });

        list_box.append(&frame);
    }

    /// Rebuild the entire list UI from the current config
    fn rebuild_list(&self) {
        // Clear list box
        while let Some(child) = self.list_box.first_child() {
            self.list_box.remove(&child);
        }

        // Create rebuild callback
        let list_box_clone = self.list_box.clone();
        let config_clone = self.config.clone();

        let rebuild_fn: Rc<RefCell<Option<Rc<dyn Fn()>>>> = Rc::new(RefCell::new(None));
        let rebuild_fn_clone = rebuild_fn.clone();

        let rebuild_closure: Rc<dyn Fn()> = Rc::new(move || {
            // Clear list box
            while let Some(child) = list_box_clone.first_child() {
                list_box_clone.remove(&child);
            }

            // Rebuild all rows
            let lines = config_clone.borrow().lines.clone();
            let callback_to_pass = rebuild_fn_clone.borrow().clone();
            for (index, line) in lines.into_iter().enumerate() {
                Self::add_line_row(
                    &list_box_clone,
                    line,
                    config_clone.clone(),
                    index,
                    callback_to_pass.clone(),
                );
            }
        });

        // Store the callback
        *rebuild_fn.borrow_mut() = Some(rebuild_closure.clone());

        // Build all rows
        let lines = self.config.borrow().lines.clone();
        for (index, line) in lines.into_iter().enumerate() {
            Self::add_line_row(
                &self.list_box,
                line,
                self.config.clone(),
                index,
                Some(rebuild_closure.clone()),
            );
        }
    }

    pub fn widget(&self) -> &GtkBox {
        &self.widget
    }

    pub fn get_config(&self) -> StaticTextSourceConfig {
        self.config.borrow().clone()
    }

    pub fn set_config(&self, config: &StaticTextSourceConfig) {
        *self.config.borrow_mut() = config.clone();

        // Update UI
        self.update_interval_spin.set_value(config.update_interval_ms as f64);
        self.custom_caption_entry.set_text(
            config.custom_caption.as_deref().unwrap_or("")
        );

        // Rebuild list
        self.rebuild_list();
    }
}

impl Default for StaticTextConfigWidget {
    fn default() -> Self {
        Self::new()
    }
}
