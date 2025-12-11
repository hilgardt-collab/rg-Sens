//! Configuration widget for Analog Clock displayer

use gtk4::prelude::*;
use gtk4::{
    Adjustment, Box as GtkBox, Button, CheckButton, DropDown, Frame, Label, Orientation,
    ScrolledWindow, SpinButton, StringList,
};
use std::cell::RefCell;
use std::rc::Rc;

use crate::ui::background::Color;
use crate::ui::clock_display::{AnalogClockConfig, FaceStyle, HandStyle, TickStyle};
use crate::ui::shared_font_dialog::shared_font_dialog;
use crate::ui::BackgroundConfigWidget;
use crate::ui::ColorPickerDialog;

/// Widget for configuring Analog Clock displayer
pub struct ClockAnalogConfigWidget {
    widget: ScrolledWindow,
    config: Rc<RefCell<AnalogClockConfig>>,
    background_widget: BackgroundConfigWidget,
    // Font button and controls for updating UI on set_config
    number_font_button: Button,
    number_size_spin: SpinButton,
    number_bold_check: CheckButton,
    number_italic_check: CheckButton,
    // Style dropdowns for updating UI on set_config
    face_dropdown: DropDown,
    tick_dropdown: DropDown,
    hand_dropdown: DropDown,
    // Checkboxes for updating UI on set_config
    show_numbers_check: CheckButton,
    show_second_hand_check: CheckButton,
    smooth_seconds_check: CheckButton,
    show_center_hub_check: CheckButton,
    // Size spinners for updating UI on set_config
    border_width_spin: SpinButton,
    hour_width_spin: SpinButton,
    minute_width_spin: SpinButton,
    second_width_spin: SpinButton,
    // Color buttons for updating labels on set_config
    tick_color_btn: Button,
    border_color_btn: Button,
    number_color_btn: Button,
    hour_color_btn: Button,
    minute_color_btn: Button,
    second_color_btn: Button,
    // Icon config
    icon_text_entry: gtk4::Entry,
    icon_font_button: Button,
    icon_size_spin: SpinButton,
    icon_bold_check: CheckButton,
}

impl ClockAnalogConfigWidget {
    pub fn new() -> Self {
        let config = Rc::new(RefCell::new(AnalogClockConfig::default()));

        let main_box = GtkBox::new(Orientation::Vertical, 8);
        main_box.set_margin_start(8);
        main_box.set_margin_end(8);
        main_box.set_margin_top(8);
        main_box.set_margin_bottom(8);

        // ============ Face Background Section ============
        let face_frame = Frame::new(Some("Clock Face Background"));
        let background_widget = BackgroundConfigWidget::new();
        face_frame.set_child(Some(background_widget.widget()));
        main_box.append(&face_frame);

        // Connect background changes to config (no-op, retrieved via get_config)

        // ============ Style Section ============
        let style_frame = Frame::new(Some("Styles"));
        let style_box = GtkBox::new(Orientation::Vertical, 6);
        style_box.set_margin_start(8);
        style_box.set_margin_end(8);
        style_box.set_margin_top(8);
        style_box.set_margin_bottom(8);

        // Face Style
        let face_box = GtkBox::new(Orientation::Horizontal, 6);
        face_box.append(&Label::new(Some("Face Style:")));
        let face_options = StringList::new(&["Minimal", "Classic", "Modern", "Roman", "Numbers"]);
        let face_dropdown = DropDown::new(Some(face_options), Option::<gtk4::Expression>::None);
        face_dropdown.set_selected(1); // Classic
        face_dropdown.set_hexpand(true);
        face_box.append(&face_dropdown);
        style_box.append(&face_box);

        // Tick Style
        let tick_box = GtkBox::new(Orientation::Horizontal, 6);
        tick_box.append(&Label::new(Some("Tick Style:")));
        let tick_options = StringList::new(&["None", "Dots", "Lines", "Mixed"]);
        let tick_dropdown = DropDown::new(Some(tick_options), Option::<gtk4::Expression>::None);
        tick_dropdown.set_selected(2); // Lines
        tick_dropdown.set_hexpand(true);
        tick_box.append(&tick_dropdown);
        style_box.append(&tick_box);

        // Hand Style
        let hand_box = GtkBox::new(Orientation::Horizontal, 6);
        hand_box.append(&Label::new(Some("Hand Style:")));
        let hand_options = StringList::new(&["Line", "Arrow", "Sword", "Fancy"]);
        let hand_dropdown = DropDown::new(Some(hand_options), Option::<gtk4::Expression>::None);
        hand_dropdown.set_selected(0); // Line
        hand_dropdown.set_hexpand(true);
        hand_box.append(&hand_dropdown);
        style_box.append(&hand_box);

        style_frame.set_child(Some(&style_box));
        main_box.append(&style_frame);

        // ============ Number Font Section ============
        let number_frame = Frame::new(Some("Clock Numbers"));
        let number_box = GtkBox::new(Orientation::Vertical, 6);
        number_box.set_margin_start(8);
        number_box.set_margin_end(8);
        number_box.set_margin_top(8);
        number_box.set_margin_bottom(8);

        // Show Numbers checkbox
        let show_numbers_check = CheckButton::with_label("Show Numbers");
        show_numbers_check.set_active(true);
        number_box.append(&show_numbers_check);

        // Font row: Font Button + Size + Bold/Italic + Copy/Paste
        let font_box = GtkBox::new(Orientation::Horizontal, 6);
        font_box.append(&Label::new(Some("Font:")));

        let initial_font = config.borrow().number_font.clone();
        let initial_size = config.borrow().number_size;
        let number_font_button = Button::with_label(&format!("{} {:.0}%", initial_font, initial_size * 100.0));
        number_font_button.set_hexpand(true);
        font_box.append(&number_font_button);

        // Size spinner (as percentage of clock radius)
        font_box.append(&Label::new(Some("Size:")));
        let number_size_adj = Adjustment::new(12.0, 5.0, 30.0, 1.0, 5.0, 0.0);
        let number_size_spin = SpinButton::new(Some(&number_size_adj), 1.0, 0);
        number_size_spin.set_width_chars(4);
        font_box.append(&number_size_spin);
        font_box.append(&Label::new(Some("%")));

        // Bold checkbox
        let number_bold_check = CheckButton::with_label("B");
        number_bold_check.set_tooltip_text(Some("Bold"));
        number_bold_check.set_active(true);
        font_box.append(&number_bold_check);

        // Italic checkbox
        let number_italic_check = CheckButton::with_label("I");
        number_italic_check.set_tooltip_text(Some("Italic"));
        number_italic_check.set_active(false);
        font_box.append(&number_italic_check);

        // Copy font button
        let copy_font_btn = Button::with_label("Copy");
        copy_font_btn.set_tooltip_text(Some("Copy font settings"));
        font_box.append(&copy_font_btn);

        // Paste font button
        let paste_font_btn = Button::with_label("Paste");
        paste_font_btn.set_tooltip_text(Some("Paste font settings"));
        font_box.append(&paste_font_btn);

        number_box.append(&font_box);

        // Number color
        let color_box = GtkBox::new(Orientation::Horizontal, 6);
        color_box.append(&Label::new(Some("Color:")));
        let number_color_btn = Button::with_label("■");
        number_color_btn.set_hexpand(true);
        color_box.append(&number_color_btn);
        number_box.append(&color_box);

        number_frame.set_child(Some(&number_box));
        main_box.append(&number_frame);

        // ============ Hands Section ============
        let hands_frame = Frame::new(Some("Clock Hands"));
        let hands_box = GtkBox::new(Orientation::Vertical, 6);
        hands_box.set_margin_start(8);
        hands_box.set_margin_end(8);
        hands_box.set_margin_top(8);
        hands_box.set_margin_bottom(8);

        // Show checkboxes
        let show_second_hand_check = CheckButton::with_label("Show Second Hand");
        show_second_hand_check.set_active(true);
        hands_box.append(&show_second_hand_check);

        let smooth_seconds_check = CheckButton::with_label("Smooth Second Hand");
        smooth_seconds_check.set_active(true);
        hands_box.append(&smooth_seconds_check);

        let show_center_hub_check = CheckButton::with_label("Show Center Hub");
        show_center_hub_check.set_active(true);
        hands_box.append(&show_center_hub_check);

        // Tick color
        let tick_row = GtkBox::new(Orientation::Horizontal, 6);
        tick_row.append(&Label::new(Some("Tick Marks:")));
        let tick_color_btn = Button::with_label("■");
        tick_color_btn.set_hexpand(true);
        tick_row.append(&tick_color_btn);
        hands_box.append(&tick_row);

        // Border color and width
        let border_row = GtkBox::new(Orientation::Horizontal, 6);
        border_row.append(&Label::new(Some("Border:")));
        let border_color_btn = Button::with_label("■");
        border_row.append(&border_color_btn);
        border_row.append(&Label::new(Some("Width:")));
        let border_width_adj = Adjustment::new(3.0, 0.0, 20.0, 0.5, 1.0, 0.0);
        let border_width_spin = SpinButton::new(Some(&border_width_adj), 0.5, 1);
        border_width_spin.set_hexpand(true);
        border_row.append(&border_width_spin);
        hands_box.append(&border_row);

        // Hour hand
        let hour_row = GtkBox::new(Orientation::Horizontal, 6);
        hour_row.append(&Label::new(Some("Hour Hand:")));
        let hour_color_btn = Button::with_label("■");
        hour_row.append(&hour_color_btn);
        hour_row.append(&Label::new(Some("Width:")));
        let hour_width_adj = Adjustment::new(6.0, 1.0, 20.0, 0.5, 1.0, 0.0);
        let hour_width_spin = SpinButton::new(Some(&hour_width_adj), 0.5, 1);
        hour_width_spin.set_hexpand(true);
        hour_row.append(&hour_width_spin);
        hands_box.append(&hour_row);

        // Minute hand
        let minute_row = GtkBox::new(Orientation::Horizontal, 6);
        minute_row.append(&Label::new(Some("Minute Hand:")));
        let minute_color_btn = Button::with_label("■");
        minute_row.append(&minute_color_btn);
        minute_row.append(&Label::new(Some("Width:")));
        let minute_width_adj = Adjustment::new(4.0, 1.0, 20.0, 0.5, 1.0, 0.0);
        let minute_width_spin = SpinButton::new(Some(&minute_width_adj), 0.5, 1);
        minute_width_spin.set_hexpand(true);
        minute_row.append(&minute_width_spin);
        hands_box.append(&minute_row);

        // Second hand
        let second_row = GtkBox::new(Orientation::Horizontal, 6);
        second_row.append(&Label::new(Some("Second Hand:")));
        let second_color_btn = Button::with_label("■");
        second_row.append(&second_color_btn);
        second_row.append(&Label::new(Some("Width:")));
        let second_width_adj = Adjustment::new(2.0, 0.5, 10.0, 0.5, 1.0, 0.0);
        let second_width_spin = SpinButton::new(Some(&second_width_adj), 0.5, 1);
        second_width_spin.set_hexpand(true);
        second_row.append(&second_width_spin);
        hands_box.append(&second_row);

        hands_frame.set_child(Some(&hands_box));
        main_box.append(&hands_frame);

        // ============ Icon Section ============
        let icon_frame = Frame::new(Some("Alarm/Timer Icon"));
        let icon_box = GtkBox::new(Orientation::Vertical, 6);
        icon_box.set_margin_start(8);
        icon_box.set_margin_end(8);
        icon_box.set_margin_top(8);
        icon_box.set_margin_bottom(8);

        // Icon text (emoji/character)
        let icon_text_row = GtkBox::new(Orientation::Horizontal, 6);
        icon_text_row.append(&Label::new(Some("Icon Text:")));
        let icon_text_entry = gtk4::Entry::new();
        icon_text_entry.set_text(&config.borrow().icon_text);
        icon_text_entry.set_max_width_chars(8);
        icon_text_entry.set_hexpand(true);
        icon_text_row.append(&icon_text_entry);
        icon_box.append(&icon_text_row);

        // Icon font
        let icon_font_row = GtkBox::new(Orientation::Horizontal, 6);
        icon_font_row.append(&Label::new(Some("Icon Font:")));
        let icon_font_button = Button::with_label(&format!("{} {:.0}%", config.borrow().icon_font, config.borrow().icon_size));
        icon_font_button.set_hexpand(true);
        icon_font_row.append(&icon_font_button);
        icon_box.append(&icon_font_row);

        // Icon size
        let icon_size_row = GtkBox::new(Orientation::Horizontal, 6);
        icon_size_row.append(&Label::new(Some("Icon Size (%):")));
        let icon_size_adj = Adjustment::new(config.borrow().icon_size, 5.0, 30.0, 1.0, 5.0, 0.0);
        let icon_size_spin = SpinButton::new(Some(&icon_size_adj), 1.0, 0);
        icon_size_spin.set_hexpand(true);
        icon_size_row.append(&icon_size_spin);
        icon_box.append(&icon_size_row);

        // Icon bold checkbox
        let icon_bold_check = CheckButton::with_label("Bold");
        icon_bold_check.set_active(config.borrow().icon_bold);
        icon_box.append(&icon_bold_check);

        icon_frame.set_child(Some(&icon_box));
        main_box.append(&icon_frame);

        // ============ Connect Signals ============

        // Font button - opens font dialog
        let config_for_font = config.clone();
        let font_btn_clone = number_font_button.clone();
        let size_spin_clone = number_size_spin.clone();
        number_font_button.connect_clicked(move |btn| {
            let window = btn.root().and_then(|r| r.downcast::<gtk4::Window>().ok());
            let _current_font = config_for_font.borrow().number_font.clone();
            let config_clone = config_for_font.clone();
            let font_btn = font_btn_clone.clone();
            let size_spin = size_spin_clone.clone();

            if let Some(win) = window {
                let font_dialog = shared_font_dialog();
                gtk4::glib::MainContext::default().spawn_local(async move {
                    match font_dialog.choose_font_future(Some(&win), None::<&gtk4::pango::FontDescription>).await {
                        Ok(font_desc) => {
                            let family = font_desc.family().map(|f| f.to_string()).unwrap_or_else(|| "Sans".to_string());
                            config_clone.borrow_mut().number_font = family.clone();
                            let size_percent = size_spin.value();
                            font_btn.set_label(&format!("{} {:.0}%", family, size_percent));
                        }
                        Err(_) => {} // User cancelled
                    }
                });
            }
        });

        // Size spinner
        let config_for_size = config.clone();
        let font_btn_for_size = number_font_button.clone();
        number_size_spin.connect_value_changed(move |spin| {
            let size_percent = spin.value();
            let mut cfg = config_for_size.borrow_mut();
            cfg.number_size = size_percent / 100.0; // Convert to fraction
            let family = cfg.number_font.clone();
            drop(cfg);
            font_btn_for_size.set_label(&format!("{} {:.0}%", family, size_percent));
        });

        // Bold checkbox
        let config_for_bold = config.clone();
        number_bold_check.connect_toggled(move |check| {
            config_for_bold.borrow_mut().number_bold = check.is_active();
        });

        // Italic checkbox
        let config_for_italic = config.clone();
        number_italic_check.connect_toggled(move |check| {
            config_for_italic.borrow_mut().number_italic = check.is_active();
        });

        // Copy font button
        let config_for_copy = config.clone();
        copy_font_btn.connect_clicked(move |_| {
            let cfg = config_for_copy.borrow();
            if let Ok(mut clipboard) = crate::ui::clipboard::CLIPBOARD.lock() {
                clipboard.copy_font(
                    cfg.number_font.clone(),
                    cfg.number_size * 100.0, // Store as percentage
                    cfg.number_bold,
                    cfg.number_italic,
                );
            }
        });

        // Paste font button
        let config_for_paste = config.clone();
        let font_btn_for_paste = number_font_button.clone();
        let size_spin_for_paste = number_size_spin.clone();
        let bold_check_for_paste = number_bold_check.clone();
        let italic_check_for_paste = number_italic_check.clone();
        paste_font_btn.connect_clicked(move |_| {
            if let Ok(clipboard) = crate::ui::clipboard::CLIPBOARD.lock() {
                if let Some((family, size, bold, italic)) = clipboard.paste_font() {
                    let mut cfg = config_for_paste.borrow_mut();
                    cfg.number_font = family.clone();
                    cfg.number_size = size / 100.0; // Convert from percentage to fraction
                    cfg.number_bold = bold;
                    cfg.number_italic = italic;
                    drop(cfg);

                    font_btn_for_paste.set_label(&format!("{} {:.0}%", family, size));
                    size_spin_for_paste.set_value(size);
                    bold_check_for_paste.set_active(bold);
                    italic_check_for_paste.set_active(italic);
                }
            }
        });

        // Number color button
        let config_for_num_color = config.clone();
        let number_color_btn_clone = number_color_btn.clone();
        number_color_btn.connect_clicked(move |btn| {
            let window = btn.root().and_then(|r| r.downcast::<gtk4::Window>().ok());
            let current = config_for_num_color.borrow().number_color;
            let config_clone = config_for_num_color.clone();
            let btn_clone = number_color_btn_clone.clone();
            gtk4::glib::MainContext::default().spawn_local(async move {
                if let Some(color) = ColorPickerDialog::pick_color(window.as_ref(), current).await {
                    config_clone.borrow_mut().number_color = color;
                    btn_clone.set_label(&format!("■ ({:.0},{:.0},{:.0})", color.r * 255.0, color.g * 255.0, color.b * 255.0));
                }
            });
        });

        // Style dropdowns
        let config_for_face = config.clone();
        face_dropdown.connect_selected_notify(move |dropdown| {
            let mut cfg = config_for_face.borrow_mut();
            cfg.face_style = match dropdown.selected() {
                0 => FaceStyle::Minimal,
                1 => FaceStyle::Classic,
                2 => FaceStyle::Modern,
                3 => FaceStyle::Roman,
                _ => FaceStyle::Numbers,
            };
        });

        let config_for_tick = config.clone();
        tick_dropdown.connect_selected_notify(move |dropdown| {
            let mut cfg = config_for_tick.borrow_mut();
            cfg.tick_style = match dropdown.selected() {
                0 => TickStyle::None,
                1 => TickStyle::Dots,
                2 => TickStyle::Lines,
                _ => TickStyle::Mixed,
            };
        });

        let config_for_hand = config.clone();
        hand_dropdown.connect_selected_notify(move |dropdown| {
            let mut cfg = config_for_hand.borrow_mut();
            let style = match dropdown.selected() {
                0 => HandStyle::Line,
                1 => HandStyle::Arrow,
                2 => HandStyle::Sword,
                _ => HandStyle::Fancy,
            };
            cfg.hour_hand_style = style;
            cfg.minute_hand_style = style;
            cfg.second_hand_style = style;
        });

        // Show checkboxes
        let config_for_show_nums = config.clone();
        show_numbers_check.connect_toggled(move |check| {
            config_for_show_nums.borrow_mut().show_numbers = check.is_active();
        });

        let config_for_show_sec = config.clone();
        show_second_hand_check.connect_toggled(move |check| {
            config_for_show_sec.borrow_mut().show_second_hand = check.is_active();
        });

        let config_for_smooth = config.clone();
        smooth_seconds_check.connect_toggled(move |check| {
            config_for_smooth.borrow_mut().smooth_seconds = check.is_active();
        });

        let config_for_hub = config.clone();
        show_center_hub_check.connect_toggled(move |check| {
            config_for_hub.borrow_mut().show_center_hub = check.is_active();
        });

        // Size spinners
        let config_for_border_w = config.clone();
        border_width_spin.connect_value_changed(move |spin| {
            config_for_border_w.borrow_mut().border_width = spin.value();
        });

        let config_for_hour_w = config.clone();
        hour_width_spin.connect_value_changed(move |spin| {
            config_for_hour_w.borrow_mut().hour_hand_width = spin.value();
        });

        let config_for_minute_w = config.clone();
        minute_width_spin.connect_value_changed(move |spin| {
            config_for_minute_w.borrow_mut().minute_hand_width = spin.value();
        });

        let config_for_second_w = config.clone();
        second_width_spin.connect_value_changed(move |spin| {
            config_for_second_w.borrow_mut().second_hand_width = spin.value();
        });

        // Color buttons
        let config_for_tick_c = config.clone();
        let tick_color_btn_clone = tick_color_btn.clone();
        tick_color_btn.connect_clicked(move |btn| {
            let window = btn.root().and_then(|r| r.downcast::<gtk4::Window>().ok());
            let current = config_for_tick_c.borrow().tick_color;
            let config_clone = config_for_tick_c.clone();
            let btn_clone = tick_color_btn_clone.clone();
            gtk4::glib::MainContext::default().spawn_local(async move {
                if let Some(color) = ColorPickerDialog::pick_color(window.as_ref(), current).await {
                    config_clone.borrow_mut().tick_color = color;
                    btn_clone.set_label(&format!("■ ({:.0},{:.0},{:.0})", color.r * 255.0, color.g * 255.0, color.b * 255.0));
                }
            });
        });

        let config_for_border_c = config.clone();
        let border_color_btn_clone = border_color_btn.clone();
        border_color_btn.connect_clicked(move |btn| {
            let window = btn.root().and_then(|r| r.downcast::<gtk4::Window>().ok());
            let current = config_for_border_c.borrow().border_color;
            let config_clone = config_for_border_c.clone();
            let btn_clone = border_color_btn_clone.clone();
            gtk4::glib::MainContext::default().spawn_local(async move {
                if let Some(color) = ColorPickerDialog::pick_color(window.as_ref(), current).await {
                    config_clone.borrow_mut().border_color = color;
                    btn_clone.set_label(&format!("■ ({:.0},{:.0},{:.0})", color.r * 255.0, color.g * 255.0, color.b * 255.0));
                }
            });
        });

        let config_for_hour_c = config.clone();
        let hour_color_btn_clone = hour_color_btn.clone();
        hour_color_btn.connect_clicked(move |btn| {
            let window = btn.root().and_then(|r| r.downcast::<gtk4::Window>().ok());
            let current = config_for_hour_c.borrow().hour_hand_color;
            let config_clone = config_for_hour_c.clone();
            let btn_clone = hour_color_btn_clone.clone();
            gtk4::glib::MainContext::default().spawn_local(async move {
                if let Some(color) = ColorPickerDialog::pick_color(window.as_ref(), current).await {
                    config_clone.borrow_mut().hour_hand_color = color;
                    btn_clone.set_label(&format!("■ ({:.0},{:.0},{:.0})", color.r * 255.0, color.g * 255.0, color.b * 255.0));
                }
            });
        });

        let config_for_minute_c = config.clone();
        let minute_color_btn_clone = minute_color_btn.clone();
        minute_color_btn.connect_clicked(move |btn| {
            let window = btn.root().and_then(|r| r.downcast::<gtk4::Window>().ok());
            let current = config_for_minute_c.borrow().minute_hand_color;
            let config_clone = config_for_minute_c.clone();
            let btn_clone = minute_color_btn_clone.clone();
            gtk4::glib::MainContext::default().spawn_local(async move {
                if let Some(color) = ColorPickerDialog::pick_color(window.as_ref(), current).await {
                    config_clone.borrow_mut().minute_hand_color = color;
                    btn_clone.set_label(&format!("■ ({:.0},{:.0},{:.0})", color.r * 255.0, color.g * 255.0, color.b * 255.0));
                }
            });
        });

        let config_for_second_c = config.clone();
        let second_color_btn_clone = second_color_btn.clone();
        second_color_btn.connect_clicked(move |btn| {
            let window = btn.root().and_then(|r| r.downcast::<gtk4::Window>().ok());
            let current = config_for_second_c.borrow().second_hand_color;
            let config_clone = config_for_second_c.clone();
            let btn_clone = second_color_btn_clone.clone();
            gtk4::glib::MainContext::default().spawn_local(async move {
                if let Some(color) = ColorPickerDialog::pick_color(window.as_ref(), current).await {
                    config_clone.borrow_mut().second_hand_color = color;
                    btn_clone.set_label(&format!("■ ({:.0},{:.0},{:.0})", color.r * 255.0, color.g * 255.0, color.b * 255.0));
                }
            });
        });

        // Icon text entry
        let config_for_icon_text = config.clone();
        icon_text_entry.connect_changed(move |entry| {
            config_for_icon_text.borrow_mut().icon_text = entry.text().to_string();
        });

        // Icon font button
        let config_for_icon_font = config.clone();
        let icon_font_btn_clone = icon_font_button.clone();
        let icon_size_spin_clone = icon_size_spin.clone();
        icon_font_button.connect_clicked(move |btn| {
            let window = btn.root().and_then(|r| r.downcast::<gtk4::Window>().ok());
            let config_clone = config_for_icon_font.clone();
            let font_btn = icon_font_btn_clone.clone();
            let size_spin = icon_size_spin_clone.clone();

            if let Some(win) = window {
                let font_dialog = shared_font_dialog();
                gtk4::glib::MainContext::default().spawn_local(async move {
                    match font_dialog.choose_font_future(Some(&win), None::<&gtk4::pango::FontDescription>).await {
                        Ok(font_desc) => {
                            let family = font_desc.family().map(|f| f.to_string()).unwrap_or_else(|| "Sans".to_string());
                            config_clone.borrow_mut().icon_font = family.clone();
                            let size_pct = size_spin.value();
                            font_btn.set_label(&format!("{} {:.0}%", family, size_pct));
                        }
                        Err(_) => {} // User cancelled
                    }
                });
            }
        });

        // Icon size spinner
        let config_for_icon_size = config.clone();
        let icon_font_btn_for_size = icon_font_button.clone();
        icon_size_spin.connect_value_changed(move |spin| {
            let size_pct = spin.value();
            let mut cfg = config_for_icon_size.borrow_mut();
            cfg.icon_size = size_pct;
            let family = cfg.icon_font.clone();
            drop(cfg);
            icon_font_btn_for_size.set_label(&format!("{} {:.0}%", family, size_pct));
        });

        // Icon bold checkbox
        let config_for_icon_bold = config.clone();
        icon_bold_check.connect_toggled(move |check| {
            config_for_icon_bold.borrow_mut().icon_bold = check.is_active();
        });

        // Wrap in scrolled window
        let scrolled = ScrolledWindow::new();
        scrolled.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);
        scrolled.set_child(Some(&main_box));
        scrolled.set_vexpand(true);

        Self {
            widget: scrolled,
            config,
            background_widget,
            number_font_button,
            number_size_spin,
            number_bold_check,
            number_italic_check,
            face_dropdown,
            tick_dropdown,
            hand_dropdown,
            show_numbers_check,
            show_second_hand_check,
            smooth_seconds_check,
            show_center_hub_check,
            border_width_spin,
            hour_width_spin,
            minute_width_spin,
            second_width_spin,
            tick_color_btn,
            border_color_btn,
            number_color_btn,
            hour_color_btn,
            minute_color_btn,
            second_color_btn,
            icon_text_entry,
            icon_font_button,
            icon_size_spin,
            icon_bold_check,
        }
    }

    pub fn widget(&self) -> &ScrolledWindow {
        &self.widget
    }

    pub fn get_config(&self) -> AnalogClockConfig {
        let mut cfg = self.config.borrow().clone();
        // Get background config from the BackgroundConfigWidget
        cfg.face_background = self.background_widget.get_config();
        cfg
    }

    pub fn set_config(&self, config: AnalogClockConfig) {
        // Set background widget
        self.background_widget.set_config(config.face_background.clone());

        // Update UI elements to match config
        self.face_dropdown.set_selected(match config.face_style {
            FaceStyle::Minimal => 0,
            FaceStyle::Classic => 1,
            FaceStyle::Modern => 2,
            FaceStyle::Roman => 3,
            FaceStyle::Numbers => 4,
        });

        self.tick_dropdown.set_selected(match config.tick_style {
            TickStyle::None => 0,
            TickStyle::Dots => 1,
            TickStyle::Lines => 2,
            TickStyle::Mixed => 3,
        });

        self.hand_dropdown.set_selected(match config.hour_hand_style {
            HandStyle::Line => 0,
            HandStyle::Arrow => 1,
            HandStyle::Sword => 2,
            HandStyle::Fancy => 3,
        });

        // Font settings
        let size_percent = config.number_size * 100.0;
        self.number_font_button.set_label(&format!("{} {:.0}%", config.number_font, size_percent));
        self.number_size_spin.set_value(size_percent);
        self.number_bold_check.set_active(config.number_bold);
        self.number_italic_check.set_active(config.number_italic);

        // Checkboxes
        self.show_numbers_check.set_active(config.show_numbers);
        self.show_second_hand_check.set_active(config.show_second_hand);
        self.smooth_seconds_check.set_active(config.smooth_seconds);
        self.show_center_hub_check.set_active(config.show_center_hub);

        // Size spinners
        self.border_width_spin.set_value(config.border_width);
        self.hour_width_spin.set_value(config.hour_hand_width);
        self.minute_width_spin.set_value(config.minute_hand_width);
        self.second_width_spin.set_value(config.second_hand_width);

        // Color buttons
        Self::update_color_button(&self.tick_color_btn, &config.tick_color);
        Self::update_color_button(&self.border_color_btn, &config.border_color);
        Self::update_color_button(&self.number_color_btn, &config.number_color);
        Self::update_color_button(&self.hour_color_btn, &config.hour_hand_color);
        Self::update_color_button(&self.minute_color_btn, &config.minute_hand_color);
        Self::update_color_button(&self.second_color_btn, &config.second_hand_color);

        // Icon config
        self.icon_text_entry.set_text(&config.icon_text);
        self.icon_font_button.set_label(&format!("{} {:.0}%", config.icon_font, config.icon_size));
        self.icon_size_spin.set_value(config.icon_size);
        self.icon_bold_check.set_active(config.icon_bold);

        // Store config
        *self.config.borrow_mut() = config;
    }

    fn update_color_button(btn: &Button, color: &Color) {
        btn.set_label(&format!("■ ({:.0},{:.0},{:.0})", color.r * 255.0, color.g * 255.0, color.b * 255.0));
    }
}

impl Default for ClockAnalogConfigWidget {
    fn default() -> Self {
        Self::new()
    }
}
