//! Configuration widget for Digital Clock displayer

use gtk4::prelude::*;
use gtk4::{
    Adjustment, Box as GtkBox, Button, CheckButton, DropDown, Frame, Label, Orientation,
    ScrolledWindow, SpinButton, StringList,
};
use std::cell::RefCell;
use std::rc::Rc;

use crate::displayers::{DigitalClockConfig, DigitalStyle};
use crate::ui::color_button_widget::ColorButtonWidget;
use crate::ui::shared_font_dialog::show_font_dialog;

/// Widget for configuring Digital Clock displayer
pub struct ClockDigitalConfigWidget {
    widget: ScrolledWindow,
    config: Rc<RefCell<DigitalClockConfig>>,
    // UI elements for updating on set_config
    style_dropdown: DropDown,
    show_date_check: CheckButton,
    show_day_name_check: CheckButton,
    show_timer_check: CheckButton,
    show_alarm_check: CheckButton,
    blink_colon_check: CheckButton,
    // Time font controls
    time_font_button: Button,
    time_size_spin: SpinButton,
    time_bold_check: CheckButton,
    time_italic_check: CheckButton,
    time_color_widget: Rc<ColorButtonWidget>,
    // Date font controls
    date_font_button: Button,
    date_size_spin: SpinButton,
    date_bold_check: CheckButton,
    date_italic_check: CheckButton,
    date_color_widget: Rc<ColorButtonWidget>,
    // Other color buttons
    timer_color_widget: Rc<ColorButtonWidget>,
    alarm_color_widget: Rc<ColorButtonWidget>,
    // Icon config
    show_icon_check: CheckButton,
    icon_text_entry: gtk4::Entry,
    icon_font_button: Button,
    icon_size_spin: SpinButton,
    icon_bold_check: CheckButton,
}

impl ClockDigitalConfigWidget {
    pub fn new() -> Self {
        let config = Rc::new(RefCell::new(DigitalClockConfig::default()));

        let main_box = GtkBox::new(Orientation::Vertical, 8);
        main_box.set_margin_start(8);
        main_box.set_margin_end(8);
        main_box.set_margin_top(8);
        main_box.set_margin_bottom(8);

        // Display Style
        let style_box = GtkBox::new(Orientation::Horizontal, 6);
        style_box.append(&Label::new(Some("Display Style:")));
        let style_options = StringList::new(&["Simple", "Segment (LED)", "LCD"]);
        let style_dropdown = DropDown::new(Some(style_options), Option::<gtk4::Expression>::None);
        style_dropdown.set_selected(0); // Simple
        style_dropdown.set_hexpand(true);
        style_box.append(&style_dropdown);
        main_box.append(&style_box);

        // Display options
        let options_frame = Frame::new(Some("Display Options"));
        let options_box = GtkBox::new(Orientation::Vertical, 4);
        options_box.set_margin_start(8);
        options_box.set_margin_end(8);
        options_box.set_margin_top(8);
        options_box.set_margin_bottom(8);

        let show_date_check = CheckButton::with_label("Show Date");
        show_date_check.set_active(true);
        options_box.append(&show_date_check);

        let show_day_name_check = CheckButton::with_label("Show Day Name");
        show_day_name_check.set_active(false);
        options_box.append(&show_day_name_check);

        let show_timer_check = CheckButton::with_label("Show Timer");
        show_timer_check.set_active(false);
        options_box.append(&show_timer_check);

        let show_alarm_check = CheckButton::with_label("Show Alarm Indicator");
        show_alarm_check.set_active(true);
        options_box.append(&show_alarm_check);

        let blink_colon_check = CheckButton::with_label("Blinking Colon");
        blink_colon_check.set_active(true);
        options_box.append(&blink_colon_check);

        options_frame.set_child(Some(&options_box));
        main_box.append(&options_frame);

        // ============ Time Font Section ============
        let time_frame = Frame::new(Some("Time Display"));
        let time_box = GtkBox::new(Orientation::Vertical, 6);
        time_box.set_margin_start(8);
        time_box.set_margin_end(8);
        time_box.set_margin_top(8);
        time_box.set_margin_bottom(8);

        // Font row: Font Button + Size + Bold/Italic + Copy/Paste
        let time_font_row = GtkBox::new(Orientation::Horizontal, 6);
        time_font_row.append(&Label::new(Some("Font:")));

        let initial_time_font = config.borrow().time_font.clone();
        let initial_time_size = config.borrow().time_size;
        let time_font_button = Button::with_label(&format!("{} {:.0}", initial_time_font, initial_time_size));
        time_font_button.set_hexpand(true);
        time_font_row.append(&time_font_button);

        time_font_row.append(&Label::new(Some("Size:")));
        let time_size_adj = Adjustment::new(48.0, 12.0, 200.0, 2.0, 10.0, 0.0);
        let time_size_spin = SpinButton::new(Some(&time_size_adj), 2.0, 0);
        time_size_spin.set_width_chars(4);
        time_font_row.append(&time_size_spin);

        let time_bold_check = CheckButton::with_label("B");
        time_bold_check.set_tooltip_text(Some("Bold"));
        time_bold_check.set_active(true);
        time_font_row.append(&time_bold_check);

        let time_italic_check = CheckButton::with_label("I");
        time_italic_check.set_tooltip_text(Some("Italic"));
        time_italic_check.set_active(false);
        time_font_row.append(&time_italic_check);

        let time_copy_btn = Button::with_label("Copy");
        time_copy_btn.set_tooltip_text(Some("Copy font settings"));
        time_font_row.append(&time_copy_btn);

        let time_paste_btn = Button::with_label("Paste");
        time_paste_btn.set_tooltip_text(Some("Paste font settings"));
        time_font_row.append(&time_paste_btn);

        time_box.append(&time_font_row);

        // Time color
        let time_color_row = GtkBox::new(Orientation::Horizontal, 6);
        time_color_row.append(&Label::new(Some("Color:")));
        let time_color_widget = Rc::new(ColorButtonWidget::new(config.borrow().time_color));
        time_color_row.append(time_color_widget.widget());
        time_box.append(&time_color_row);

        time_frame.set_child(Some(&time_box));
        main_box.append(&time_frame);

        // ============ Date Font Section ============
        let date_frame = Frame::new(Some("Date Display"));
        let date_box = GtkBox::new(Orientation::Vertical, 6);
        date_box.set_margin_start(8);
        date_box.set_margin_end(8);
        date_box.set_margin_top(8);
        date_box.set_margin_bottom(8);

        // Font row: Font Button + Size + Bold/Italic + Copy/Paste
        let date_font_row = GtkBox::new(Orientation::Horizontal, 6);
        date_font_row.append(&Label::new(Some("Font:")));

        let initial_date_font = config.borrow().date_font.clone();
        let initial_date_size = config.borrow().date_size;
        let date_font_button = Button::with_label(&format!("{} {:.0}", initial_date_font, initial_date_size));
        date_font_button.set_hexpand(true);
        date_font_row.append(&date_font_button);

        date_font_row.append(&Label::new(Some("Size:")));
        let date_size_adj = Adjustment::new(16.0, 8.0, 100.0, 1.0, 5.0, 0.0);
        let date_size_spin = SpinButton::new(Some(&date_size_adj), 1.0, 0);
        date_size_spin.set_width_chars(4);
        date_font_row.append(&date_size_spin);

        let date_bold_check = CheckButton::with_label("B");
        date_bold_check.set_tooltip_text(Some("Bold"));
        date_bold_check.set_active(false);
        date_font_row.append(&date_bold_check);

        let date_italic_check = CheckButton::with_label("I");
        date_italic_check.set_tooltip_text(Some("Italic"));
        date_italic_check.set_active(false);
        date_font_row.append(&date_italic_check);

        let date_copy_btn = Button::with_label("Copy");
        date_copy_btn.set_tooltip_text(Some("Copy font settings"));
        date_font_row.append(&date_copy_btn);

        let date_paste_btn = Button::with_label("Paste");
        date_paste_btn.set_tooltip_text(Some("Paste font settings"));
        date_font_row.append(&date_paste_btn);

        date_box.append(&date_font_row);

        // Date color
        let date_color_row = GtkBox::new(Orientation::Horizontal, 6);
        date_color_row.append(&Label::new(Some("Color:")));
        let date_color_widget = Rc::new(ColorButtonWidget::new(config.borrow().date_color));
        date_color_row.append(date_color_widget.widget());
        date_box.append(&date_color_row);

        date_frame.set_child(Some(&date_box));
        main_box.append(&date_frame);

        // ============ Other Colors ============
        let colors_frame = Frame::new(Some("Other Colors"));
        let colors_box = GtkBox::new(Orientation::Vertical, 6);
        colors_box.set_margin_start(8);
        colors_box.set_margin_end(8);
        colors_box.set_margin_top(8);
        colors_box.set_margin_bottom(8);

        // Timer color
        let timer_color_row = GtkBox::new(Orientation::Horizontal, 6);
        timer_color_row.append(&Label::new(Some("Timer Color:")));
        let timer_color_widget = Rc::new(ColorButtonWidget::new(config.borrow().timer_color));
        timer_color_row.append(timer_color_widget.widget());
        colors_box.append(&timer_color_row);

        // Alarm color
        let alarm_color_row = GtkBox::new(Orientation::Horizontal, 6);
        alarm_color_row.append(&Label::new(Some("Alarm Color:")));
        let alarm_color_widget = Rc::new(ColorButtonWidget::new(config.borrow().alarm_color));
        alarm_color_row.append(alarm_color_widget.widget());
        colors_box.append(&alarm_color_row);

        colors_frame.set_child(Some(&colors_box));
        main_box.append(&colors_frame);

        // ============ Icon Section ============
        let icon_frame = Frame::new(Some("Alarm/Timer Icon"));
        let icon_box = GtkBox::new(Orientation::Vertical, 6);
        icon_box.set_margin_start(8);
        icon_box.set_margin_end(8);
        icon_box.set_margin_top(8);
        icon_box.set_margin_bottom(8);

        // Show icon checkbox
        let show_icon_check = CheckButton::with_label("Show Icon");
        show_icon_check.set_active(config.borrow().show_icon);
        icon_box.append(&show_icon_check);

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
        let icon_font_button = Button::with_label(&format!("{} {:.0}px", config.borrow().icon_font, config.borrow().icon_size));
        icon_font_button.set_hexpand(true);
        icon_font_row.append(&icon_font_button);
        icon_box.append(&icon_font_row);

        // Icon size
        let icon_size_row = GtkBox::new(Orientation::Horizontal, 6);
        icon_size_row.append(&Label::new(Some("Icon Size (px):")));
        let icon_size_adj = Adjustment::new(config.borrow().icon_size, 8.0, 48.0, 1.0, 5.0, 0.0);
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

        // Style dropdown
        let config_for_style = config.clone();
        style_dropdown.connect_selected_notify(move |dropdown| {
            let mut cfg = config_for_style.borrow_mut();
            cfg.style = match dropdown.selected() {
                0 => DigitalStyle::Simple,
                1 => DigitalStyle::Segment,
                _ => DigitalStyle::LCD,
            };
        });

        // Display options checkboxes
        let config_for_show_date = config.clone();
        show_date_check.connect_toggled(move |check| {
            config_for_show_date.borrow_mut().show_date = check.is_active();
        });

        let config_for_show_day = config.clone();
        show_day_name_check.connect_toggled(move |check| {
            config_for_show_day.borrow_mut().show_day_name = check.is_active();
        });

        let config_for_show_timer = config.clone();
        show_timer_check.connect_toggled(move |check| {
            config_for_show_timer.borrow_mut().show_timer = check.is_active();
        });

        let config_for_show_alarm = config.clone();
        show_alarm_check.connect_toggled(move |check| {
            config_for_show_alarm.borrow_mut().show_alarm_indicator = check.is_active();
        });

        let config_for_blink = config.clone();
        blink_colon_check.connect_toggled(move |check| {
            config_for_blink.borrow_mut().blink_colon = check.is_active();
        });

        // Time font button - opens font dialog
        let config_for_time_font = config.clone();
        let time_font_btn_clone = time_font_button.clone();
        let time_size_spin_clone = time_size_spin.clone();
        time_font_button.connect_clicked(move |btn| {
            let window = btn.root().and_then(|r| r.downcast::<gtk4::Window>().ok());
            let config_clone = config_for_time_font.clone();
            let font_btn = time_font_btn_clone.clone();
            let size_spin = time_size_spin_clone.clone();

            if let Some(win) = window {
                show_font_dialog(Some(&win), None, move |font_desc| {
                    let family = font_desc.family().map(|f| f.to_string()).unwrap_or_else(|| "Monospace".to_string());
                    config_clone.borrow_mut().time_font = family.clone();
                    let size = size_spin.value();
                    font_btn.set_label(&format!("{} {:.0}", family, size));
                });
            }
        });

        // Time size spinner
        let config_for_time_size = config.clone();
        let time_font_btn_for_size = time_font_button.clone();
        time_size_spin.connect_value_changed(move |spin| {
            let size = spin.value();
            let mut cfg = config_for_time_size.borrow_mut();
            cfg.time_size = size;
            let family = cfg.time_font.clone();
            drop(cfg);
            time_font_btn_for_size.set_label(&format!("{} {:.0}", family, size));
        });

        // Time bold checkbox
        let config_for_time_bold = config.clone();
        time_bold_check.connect_toggled(move |check| {
            config_for_time_bold.borrow_mut().time_bold = check.is_active();
        });

        // Time italic checkbox
        let config_for_time_italic = config.clone();
        time_italic_check.connect_toggled(move |check| {
            config_for_time_italic.borrow_mut().time_italic = check.is_active();
        });

        // Time copy font button
        let config_for_time_copy = config.clone();
        time_copy_btn.connect_clicked(move |_| {
            let cfg = config_for_time_copy.borrow();
            if let Ok(mut clipboard) = crate::ui::clipboard::CLIPBOARD.lock() {
                clipboard.copy_font(
                    cfg.time_font.clone(),
                    cfg.time_size,
                    cfg.time_bold,
                    cfg.time_italic,
                );
            }
        });

        // Time paste font button
        let config_for_time_paste = config.clone();
        let time_font_btn_for_paste = time_font_button.clone();
        let time_size_spin_for_paste = time_size_spin.clone();
        let time_bold_check_for_paste = time_bold_check.clone();
        let time_italic_check_for_paste = time_italic_check.clone();
        time_paste_btn.connect_clicked(move |_| {
            if let Ok(clipboard) = crate::ui::clipboard::CLIPBOARD.lock() {
                if let Some((family, size, bold, italic)) = clipboard.paste_font() {
                    let mut cfg = config_for_time_paste.borrow_mut();
                    cfg.time_font = family.clone();
                    cfg.time_size = size;
                    cfg.time_bold = bold;
                    cfg.time_italic = italic;
                    drop(cfg);

                    time_font_btn_for_paste.set_label(&format!("{} {:.0}", family, size));
                    time_size_spin_for_paste.set_value(size);
                    time_bold_check_for_paste.set_active(bold);
                    time_italic_check_for_paste.set_active(italic);
                }
            }
        });

        // Time color widget callback
        let config_for_time_color = config.clone();
        time_color_widget.set_on_change(move |color| {
            config_for_time_color.borrow_mut().time_color = color;
        });

        // Date font button - opens font dialog
        let config_for_date_font = config.clone();
        let date_font_btn_clone = date_font_button.clone();
        let date_size_spin_clone = date_size_spin.clone();
        date_font_button.connect_clicked(move |btn| {
            let window = btn.root().and_then(|r| r.downcast::<gtk4::Window>().ok());
            let config_clone = config_for_date_font.clone();
            let font_btn = date_font_btn_clone.clone();
            let size_spin = date_size_spin_clone.clone();

            if let Some(win) = window {
                show_font_dialog(Some(&win), None, move |font_desc| {
                    let family = font_desc.family().map(|f| f.to_string()).unwrap_or_else(|| "Sans".to_string());
                    config_clone.borrow_mut().date_font = family.clone();
                    let size = size_spin.value();
                    font_btn.set_label(&format!("{} {:.0}", family, size));
                });
            }
        });

        // Date size spinner
        let config_for_date_size = config.clone();
        let date_font_btn_for_size = date_font_button.clone();
        date_size_spin.connect_value_changed(move |spin| {
            let size = spin.value();
            let mut cfg = config_for_date_size.borrow_mut();
            cfg.date_size = size;
            let family = cfg.date_font.clone();
            drop(cfg);
            date_font_btn_for_size.set_label(&format!("{} {:.0}", family, size));
        });

        // Date bold checkbox
        let config_for_date_bold = config.clone();
        date_bold_check.connect_toggled(move |check| {
            config_for_date_bold.borrow_mut().date_bold = check.is_active();
        });

        // Date italic checkbox
        let config_for_date_italic = config.clone();
        date_italic_check.connect_toggled(move |check| {
            config_for_date_italic.borrow_mut().date_italic = check.is_active();
        });

        // Date copy font button
        let config_for_date_copy = config.clone();
        date_copy_btn.connect_clicked(move |_| {
            let cfg = config_for_date_copy.borrow();
            if let Ok(mut clipboard) = crate::ui::clipboard::CLIPBOARD.lock() {
                clipboard.copy_font(
                    cfg.date_font.clone(),
                    cfg.date_size,
                    cfg.date_bold,
                    cfg.date_italic,
                );
            }
        });

        // Date paste font button
        let config_for_date_paste = config.clone();
        let date_font_btn_for_paste = date_font_button.clone();
        let date_size_spin_for_paste = date_size_spin.clone();
        let date_bold_check_for_paste = date_bold_check.clone();
        let date_italic_check_for_paste = date_italic_check.clone();
        date_paste_btn.connect_clicked(move |_| {
            if let Ok(clipboard) = crate::ui::clipboard::CLIPBOARD.lock() {
                if let Some((family, size, bold, italic)) = clipboard.paste_font() {
                    let mut cfg = config_for_date_paste.borrow_mut();
                    cfg.date_font = family.clone();
                    cfg.date_size = size;
                    cfg.date_bold = bold;
                    cfg.date_italic = italic;
                    drop(cfg);

                    date_font_btn_for_paste.set_label(&format!("{} {:.0}", family, size));
                    date_size_spin_for_paste.set_value(size);
                    date_bold_check_for_paste.set_active(bold);
                    date_italic_check_for_paste.set_active(italic);
                }
            }
        });

        // Date color widget callback
        let config_for_date_color = config.clone();
        date_color_widget.set_on_change(move |color| {
            config_for_date_color.borrow_mut().date_color = color;
        });

        // Timer color widget callback
        let config_for_timer_color = config.clone();
        timer_color_widget.set_on_change(move |color| {
            config_for_timer_color.borrow_mut().timer_color = color;
        });

        // Alarm color widget callback
        let config_for_alarm_color = config.clone();
        alarm_color_widget.set_on_change(move |color| {
            config_for_alarm_color.borrow_mut().alarm_color = color;
        });

        // Show icon checkbox
        let config_for_show_icon = config.clone();
        show_icon_check.connect_toggled(move |check| {
            config_for_show_icon.borrow_mut().show_icon = check.is_active();
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
                show_font_dialog(Some(&win), None, move |font_desc| {
                    let family = font_desc.family().map(|f| f.to_string()).unwrap_or_else(|| "Sans".to_string());
                    config_clone.borrow_mut().icon_font = family.clone();
                    let size_px = size_spin.value();
                    font_btn.set_label(&format!("{} {:.0}px", family, size_px));
                });
            }
        });

        // Icon size spinner
        let config_for_icon_size = config.clone();
        let icon_font_btn_for_size = icon_font_button.clone();
        icon_size_spin.connect_value_changed(move |spin| {
            let size_px = spin.value();
            let mut cfg = config_for_icon_size.borrow_mut();
            cfg.icon_size = size_px;
            let family = cfg.icon_font.clone();
            drop(cfg);
            icon_font_btn_for_size.set_label(&format!("{} {:.0}px", family, size_px));
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
            style_dropdown,
            show_date_check,
            show_day_name_check,
            show_timer_check,
            show_alarm_check,
            blink_colon_check,
            time_font_button,
            time_size_spin,
            time_bold_check,
            time_italic_check,
            time_color_widget,
            date_font_button,
            date_size_spin,
            date_bold_check,
            date_italic_check,
            date_color_widget,
            timer_color_widget,
            alarm_color_widget,
            show_icon_check,
            icon_text_entry,
            icon_font_button,
            icon_size_spin,
            icon_bold_check,
        }
    }

    pub fn widget(&self) -> &ScrolledWindow {
        &self.widget
    }

    pub fn get_config(&self) -> DigitalClockConfig {
        self.config.borrow().clone()
    }

    pub fn set_config(&self, config: DigitalClockConfig) {
        // Update UI to match config
        self.style_dropdown.set_selected(match config.style {
            DigitalStyle::Simple => 0,
            DigitalStyle::Segment => 1,
            DigitalStyle::LCD => 2,
        });

        self.show_date_check.set_active(config.show_date);
        self.show_day_name_check.set_active(config.show_day_name);
        self.show_timer_check.set_active(config.show_timer);
        self.show_alarm_check.set_active(config.show_alarm_indicator);
        self.blink_colon_check.set_active(config.blink_colon);

        // Time font
        self.time_font_button.set_label(&format!("{} {:.0}", config.time_font, config.time_size));
        self.time_size_spin.set_value(config.time_size);
        self.time_bold_check.set_active(config.time_bold);
        self.time_italic_check.set_active(config.time_italic);
        self.time_color_widget.set_color(config.time_color);

        // Date font
        self.date_font_button.set_label(&format!("{} {:.0}", config.date_font, config.date_size));
        self.date_size_spin.set_value(config.date_size);
        self.date_bold_check.set_active(config.date_bold);
        self.date_italic_check.set_active(config.date_italic);
        self.date_color_widget.set_color(config.date_color);

        // Other colors
        self.timer_color_widget.set_color(config.timer_color);
        self.alarm_color_widget.set_color(config.alarm_color);

        // Icon config
        self.show_icon_check.set_active(config.show_icon);
        self.icon_text_entry.set_text(&config.icon_text);
        self.icon_font_button.set_label(&format!("{} {:.0}px", config.icon_font, config.icon_size));
        self.icon_size_spin.set_value(config.icon_size);
        self.icon_bold_check.set_active(config.icon_bold);

        // Store config
        *self.config.borrow_mut() = config;
    }
}

impl Default for ClockDigitalConfigWidget {
    fn default() -> Self {
        Self::new()
    }
}
