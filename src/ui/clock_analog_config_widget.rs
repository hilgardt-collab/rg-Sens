//! Configuration widget for Analog Clock displayer

use gtk4::prelude::*;
use gtk4::{
    Adjustment, Box as GtkBox, Button, CheckButton, DropDown, Frame, Grid, Label, Notebook, Orientation,
    SpinButton, StringList,
};
use std::cell::RefCell;
use std::rc::Rc;

use crate::ui::clock_display::{AnalogClockConfig, FaceStyle, HandStyle, TickStyle};
use crate::ui::shared_font_dialog::show_font_dialog;
use crate::ui::BackgroundConfigWidget;
use crate::ui::theme::ComboThemeConfig;
use crate::ui::theme_color_selector::ThemeColorSelector;
use crate::ui::theme_font_selector::ThemeFontSelector;

/// Widget for configuring Analog Clock displayer
pub struct ClockAnalogConfigWidget {
    widget: Notebook,
    config: Rc<RefCell<AnalogClockConfig>>,
    theme_config: Rc<RefCell<ComboThemeConfig>>,
    background_widget: BackgroundConfigWidget,
    // Numbers tab
    face_dropdown: DropDown,
    number_font_selector: Rc<ThemeFontSelector>,
    number_bold_check: CheckButton,
    number_italic_check: CheckButton,
    number_color_selector: Rc<ThemeColorSelector>,
    show_numbers_check: CheckButton,
    // Ticks tab - hour ticks
    hour_tick_style_dropdown: DropDown,
    hour_tick_color_selector: Rc<ThemeColorSelector>,
    hour_tick_outer_spin: SpinButton,
    hour_tick_inner_spin: SpinButton,
    // Ticks tab - minute ticks
    minute_tick_style_dropdown: DropDown,
    minute_tick_color_selector: Rc<ThemeColorSelector>,
    minute_tick_outer_spin: SpinButton,
    minute_tick_inner_spin: SpinButton,
    // Hands tab
    hand_dropdown: DropDown,
    show_second_hand_check: CheckButton,
    smooth_seconds_check: CheckButton,
    show_center_hub_check: CheckButton,
    border_color_selector: Rc<ThemeColorSelector>,
    border_width_spin: SpinButton,
    hour_color_selector: Rc<ThemeColorSelector>,
    hour_width_spin: SpinButton,
    minute_color_selector: Rc<ThemeColorSelector>,
    minute_width_spin: SpinButton,
    second_color_selector: Rc<ThemeColorSelector>,
    second_width_spin: SpinButton,
    center_hub_color_selector: Rc<ThemeColorSelector>,
    center_hub_size_spin: SpinButton,
    // Icon tab
    show_icon_check: CheckButton,
    icon_text_entry: gtk4::Entry,
    icon_font_button: Button,
    icon_size_spin: SpinButton,
    icon_bold_check: CheckButton,
    center_indicator_check: CheckButton,
    shrink_for_indicator_check: CheckButton,
}

impl ClockAnalogConfigWidget {
    pub fn new() -> Self {
        let config = Rc::new(RefCell::new(AnalogClockConfig::default()));
        let theme_config = Rc::new(RefCell::new(ComboThemeConfig::default()));

        // Create notebook for tabbed interface
        let notebook = Notebook::new();
        notebook.set_vexpand(true);

        // ============ TAB 1: Appearance ============
        let appearance_box = GtkBox::new(Orientation::Vertical, 8);
        appearance_box.set_margin_start(8);
        appearance_box.set_margin_end(8);
        appearance_box.set_margin_top(8);
        appearance_box.set_margin_bottom(8);

        // Face Background Section
        let face_frame = Frame::new(Some("Clock Face Background"));
        let background_widget = BackgroundConfigWidget::new();
        face_frame.set_child(Some(background_widget.widget()));
        appearance_box.append(&face_frame);

        // Add Appearance tab to notebook
        notebook.append_page(&appearance_box, Some(&Label::new(Some("Appearance"))));

        // ============ TAB 2: Numbers ============
        let numbers_box = GtkBox::new(Orientation::Vertical, 8);
        numbers_box.set_margin_start(8);
        numbers_box.set_margin_end(8);
        numbers_box.set_margin_top(8);
        numbers_box.set_margin_bottom(8);

        // Number Style Section
        let number_frame = Frame::new(Some("Clock Numbers"));
        let number_box = GtkBox::new(Orientation::Vertical, 6);
        number_box.set_margin_start(8);
        number_box.set_margin_end(8);
        number_box.set_margin_top(8);
        number_box.set_margin_bottom(8);

        // Face Style (moved from Appearance)
        let face_box = GtkBox::new(Orientation::Horizontal, 6);
        face_box.append(&Label::new(Some("Number Style:")));
        let face_options = StringList::new(&["None", "Minimal", "Classic", "Roman"]);
        let face_dropdown = DropDown::new(Some(face_options), Option::<gtk4::Expression>::None);
        face_dropdown.set_selected(2); // Classic
        face_dropdown.set_hexpand(true);
        face_box.append(&face_dropdown);
        number_box.append(&face_box);

        // Show Numbers checkbox (depends on face style)
        let show_numbers_check = CheckButton::with_label("Show Numbers");
        show_numbers_check.set_active(true);
        number_box.append(&show_numbers_check);

        // Font selector (theme-aware)
        let font_row = GtkBox::new(Orientation::Horizontal, 6);
        font_row.append(&Label::new(Some("Font:")));

        let initial_font = config.borrow().number_font.clone();
        // Convert fraction to percentage for display (0.12 -> 12)
        let initial_font_for_selector = initial_font.with_size(initial_font.size() * 100.0);
        let number_font_selector = Rc::new(ThemeFontSelector::new(initial_font_for_selector));
        number_font_selector.set_theme_config(theme_config.borrow().clone());
        number_font_selector.widget().set_hexpand(true);
        font_row.append(number_font_selector.widget());

        // Bold checkbox
        let number_bold_check = CheckButton::with_label("B");
        number_bold_check.set_tooltip_text(Some("Bold"));
        number_bold_check.set_active(config.borrow().number_bold);
        font_row.append(&number_bold_check);

        // Italic checkbox
        let number_italic_check = CheckButton::with_label("I");
        number_italic_check.set_tooltip_text(Some("Italic"));
        number_italic_check.set_active(config.borrow().number_italic);
        font_row.append(&number_italic_check);

        number_box.append(&font_row);

        // Number color (theme-aware)
        let color_row = GtkBox::new(Orientation::Horizontal, 6);
        color_row.append(&Label::new(Some("Color:")));
        let initial_color = config.borrow().number_color.clone();
        let number_color_selector = Rc::new(ThemeColorSelector::new(initial_color));
        number_color_selector.set_theme_config(theme_config.borrow().clone());
        color_row.append(number_color_selector.widget());
        number_box.append(&color_row);

        // Connect font selector callback - convert percentage back to fraction for storage (12 -> 0.12)
        let config_for_font = config.clone();
        number_font_selector.set_on_change(move |font_source| {
            let font_as_fraction = font_source.with_size(font_source.size() / 100.0);
            config_for_font.borrow_mut().number_font = font_as_fraction;
        });

        // Connect color selector callback
        let config_for_num_color = config.clone();
        number_color_selector.set_on_change(move |color_source| {
            config_for_num_color.borrow_mut().number_color = color_source;
        });

        number_frame.set_child(Some(&number_box));
        numbers_box.append(&number_frame);

        // Add Numbers tab to notebook
        notebook.append_page(&numbers_box, Some(&Label::new(Some("Numbers"))));

        // ============ TAB 3: Ticks ============
        let ticks_box = GtkBox::new(Orientation::Vertical, 8);
        ticks_box.set_margin_start(8);
        ticks_box.set_margin_end(8);
        ticks_box.set_margin_top(8);
        ticks_box.set_margin_bottom(8);

        // Hour Ticks Section
        let hour_tick_frame = Frame::new(Some("Hour Ticks"));
        let hour_tick_box = GtkBox::new(Orientation::Vertical, 6);
        hour_tick_box.set_margin_start(8);
        hour_tick_box.set_margin_end(8);
        hour_tick_box.set_margin_top(8);
        hour_tick_box.set_margin_bottom(8);

        // Hour tick style
        let hour_style_row = GtkBox::new(Orientation::Horizontal, 6);
        hour_style_row.append(&Label::new(Some("Type:")));
        let hour_tick_options = StringList::new(&["None", "Squares", "Lines", "Dots", "Triangles"]);
        let hour_tick_style_dropdown = DropDown::new(Some(hour_tick_options), Option::<gtk4::Expression>::None);
        hour_tick_style_dropdown.set_selected(2); // Lines
        hour_tick_style_dropdown.set_hexpand(true);
        hour_style_row.append(&hour_tick_style_dropdown);
        hour_tick_box.append(&hour_style_row);

        // Hour tick color (theme-aware)
        let hour_color_row = GtkBox::new(Orientation::Horizontal, 6);
        hour_color_row.append(&Label::new(Some("Color:")));
        let hour_tick_color_selector = Rc::new(ThemeColorSelector::new(config.borrow().hour_tick_color.clone()));
        hour_tick_color_selector.set_theme_config(theme_config.borrow().clone());
        hour_color_row.append(hour_tick_color_selector.widget());
        hour_tick_box.append(&hour_color_row);

        // Hour tick radii
        let hour_outer_row = GtkBox::new(Orientation::Horizontal, 6);
        hour_outer_row.append(&Label::new(Some("Outer Radius %:")));
        let hour_outer_adj = Adjustment::new(95.0, 50.0, 100.0, 1.0, 5.0, 0.0);
        let hour_tick_outer_spin = SpinButton::new(Some(&hour_outer_adj), 1.0, 0);
        hour_tick_outer_spin.set_hexpand(true);
        hour_outer_row.append(&hour_tick_outer_spin);
        hour_tick_box.append(&hour_outer_row);

        let hour_inner_row = GtkBox::new(Orientation::Horizontal, 6);
        hour_inner_row.append(&Label::new(Some("Inner Radius %:")));
        let hour_inner_adj = Adjustment::new(85.0, 50.0, 100.0, 1.0, 5.0, 0.0);
        let hour_tick_inner_spin = SpinButton::new(Some(&hour_inner_adj), 1.0, 0);
        hour_tick_inner_spin.set_hexpand(true);
        hour_inner_row.append(&hour_tick_inner_spin);
        hour_tick_box.append(&hour_inner_row);

        hour_tick_frame.set_child(Some(&hour_tick_box));
        ticks_box.append(&hour_tick_frame);

        // Minute Ticks Section
        let minute_tick_frame = Frame::new(Some("Minute Ticks"));
        let minute_tick_box = GtkBox::new(Orientation::Vertical, 6);
        minute_tick_box.set_margin_start(8);
        minute_tick_box.set_margin_end(8);
        minute_tick_box.set_margin_top(8);
        minute_tick_box.set_margin_bottom(8);

        // Minute tick style
        let minute_style_row = GtkBox::new(Orientation::Horizontal, 6);
        minute_style_row.append(&Label::new(Some("Type:")));
        let minute_tick_options = StringList::new(&["None", "Squares", "Lines", "Dots", "Triangles"]);
        let minute_tick_style_dropdown = DropDown::new(Some(minute_tick_options), Option::<gtk4::Expression>::None);
        minute_tick_style_dropdown.set_selected(2); // Lines
        minute_tick_style_dropdown.set_hexpand(true);
        minute_style_row.append(&minute_tick_style_dropdown);
        minute_tick_box.append(&minute_style_row);

        // Minute tick color (theme-aware)
        let minute_color_row = GtkBox::new(Orientation::Horizontal, 6);
        minute_color_row.append(&Label::new(Some("Color:")));
        let minute_tick_color_selector = Rc::new(ThemeColorSelector::new(config.borrow().minute_tick_color.clone()));
        minute_tick_color_selector.set_theme_config(theme_config.borrow().clone());
        minute_color_row.append(minute_tick_color_selector.widget());
        minute_tick_box.append(&minute_color_row);

        // Minute tick radii
        let minute_outer_row = GtkBox::new(Orientation::Horizontal, 6);
        minute_outer_row.append(&Label::new(Some("Outer Radius %:")));
        let minute_outer_adj = Adjustment::new(95.0, 50.0, 100.0, 1.0, 5.0, 0.0);
        let minute_tick_outer_spin = SpinButton::new(Some(&minute_outer_adj), 1.0, 0);
        minute_tick_outer_spin.set_hexpand(true);
        minute_outer_row.append(&minute_tick_outer_spin);
        minute_tick_box.append(&minute_outer_row);

        let minute_inner_row = GtkBox::new(Orientation::Horizontal, 6);
        minute_inner_row.append(&Label::new(Some("Inner Radius %:")));
        let minute_inner_adj = Adjustment::new(90.0, 50.0, 100.0, 1.0, 5.0, 0.0);
        let minute_tick_inner_spin = SpinButton::new(Some(&minute_inner_adj), 1.0, 0);
        minute_tick_inner_spin.set_hexpand(true);
        minute_inner_row.append(&minute_tick_inner_spin);
        minute_tick_box.append(&minute_inner_row);

        minute_tick_frame.set_child(Some(&minute_tick_box));
        ticks_box.append(&minute_tick_frame);

        // Connect tick callbacks
        let config_for_hour_tick_style = config.clone();
        hour_tick_style_dropdown.connect_selected_notify(move |dropdown| {
            config_for_hour_tick_style.borrow_mut().hour_tick_style = tick_style_from_index(dropdown.selected());
        });

        let config_for_hour_tick_color = config.clone();
        hour_tick_color_selector.set_on_change(move |color_source| {
            config_for_hour_tick_color.borrow_mut().hour_tick_color = color_source;
        });

        let config_for_hour_tick_outer = config.clone();
        hour_tick_outer_spin.connect_value_changed(move |spin| {
            config_for_hour_tick_outer.borrow_mut().hour_tick_outer_percent = spin.value();
        });

        let config_for_hour_tick_inner = config.clone();
        hour_tick_inner_spin.connect_value_changed(move |spin| {
            config_for_hour_tick_inner.borrow_mut().hour_tick_inner_percent = spin.value();
        });

        let config_for_minute_tick_style = config.clone();
        minute_tick_style_dropdown.connect_selected_notify(move |dropdown| {
            config_for_minute_tick_style.borrow_mut().minute_tick_style = tick_style_from_index(dropdown.selected());
        });

        let config_for_minute_tick_color = config.clone();
        minute_tick_color_selector.set_on_change(move |color_source| {
            config_for_minute_tick_color.borrow_mut().minute_tick_color = color_source;
        });

        let config_for_minute_tick_outer = config.clone();
        minute_tick_outer_spin.connect_value_changed(move |spin| {
            config_for_minute_tick_outer.borrow_mut().minute_tick_outer_percent = spin.value();
        });

        let config_for_minute_tick_inner = config.clone();
        minute_tick_inner_spin.connect_value_changed(move |spin| {
            config_for_minute_tick_inner.borrow_mut().minute_tick_inner_percent = spin.value();
        });

        // Add Ticks tab to notebook
        notebook.append_page(&ticks_box, Some(&Label::new(Some("Ticks"))));

        // ============ TAB 4: Hands ============
        let hands_tab_box = GtkBox::new(Orientation::Vertical, 8);
        hands_tab_box.set_margin_start(8);
        hands_tab_box.set_margin_end(8);
        hands_tab_box.set_margin_top(8);
        hands_tab_box.set_margin_bottom(8);

        // Hand Style Section
        let style_frame = Frame::new(Some("Hand Style"));
        let style_box = GtkBox::new(Orientation::Vertical, 6);
        style_box.set_margin_start(8);
        style_box.set_margin_end(8);
        style_box.set_margin_top(8);
        style_box.set_margin_bottom(8);

        // Hand Style
        let hand_box = GtkBox::new(Orientation::Horizontal, 6);
        hand_box.append(&Label::new(Some("Hand Style:")));
        let hand_options = StringList::new(&["Line", "Arrow", "Sword", "Fancy"]);
        let hand_dropdown = DropDown::new(Some(hand_options), Option::<gtk4::Expression>::None);
        hand_dropdown.set_selected(0); // Line
        hand_dropdown.set_hexpand(true);
        hand_box.append(&hand_dropdown);
        style_box.append(&hand_box);

        // Show checkboxes
        let show_second_hand_check = CheckButton::with_label("Show Second Hand");
        show_second_hand_check.set_active(true);
        style_box.append(&show_second_hand_check);

        let smooth_seconds_check = CheckButton::with_label("Smooth Second Hand");
        smooth_seconds_check.set_active(true);
        style_box.append(&smooth_seconds_check);

        let show_center_hub_check = CheckButton::with_label("Show Center Hub");
        show_center_hub_check.set_active(true);
        style_box.append(&show_center_hub_check);

        style_frame.set_child(Some(&style_box));
        hands_tab_box.append(&style_frame);

        // Colors Section - using Grid for alignment
        let colors_frame = Frame::new(Some("Colors"));
        let colors_grid = Grid::new();
        colors_grid.set_row_spacing(6);
        colors_grid.set_column_spacing(8);
        colors_grid.set_margin_start(8);
        colors_grid.set_margin_end(8);
        colors_grid.set_margin_top(8);
        colors_grid.set_margin_bottom(8);

        // Column 0: Labels (right-aligned), Column 1: Color selectors, Column 2: "Width:" labels, Column 3: Spinners
        let label_width = 90;

        // Border row
        let border_label = Label::new(Some("Border:"));
        border_label.set_xalign(1.0);
        border_label.set_width_request(label_width);
        colors_grid.attach(&border_label, 0, 0, 1, 1);
        let border_color_selector = Rc::new(ThemeColorSelector::new(config.borrow().border_color.clone()));
        border_color_selector.set_theme_config(theme_config.borrow().clone());
        colors_grid.attach(border_color_selector.widget(), 1, 0, 1, 1);
        let border_width_label = Label::new(Some("Width:"));
        colors_grid.attach(&border_width_label, 2, 0, 1, 1);
        let border_width_adj = Adjustment::new(3.0, 0.0, 20.0, 0.5, 1.0, 0.0);
        let border_width_spin = SpinButton::new(Some(&border_width_adj), 0.5, 1);
        border_width_spin.set_hexpand(true);
        colors_grid.attach(&border_width_spin, 3, 0, 1, 1);

        // Hour hand row
        let hour_label = Label::new(Some("Hour Hand:"));
        hour_label.set_xalign(1.0);
        hour_label.set_width_request(label_width);
        colors_grid.attach(&hour_label, 0, 1, 1, 1);
        let hour_color_selector = Rc::new(ThemeColorSelector::new(config.borrow().hour_hand_color.clone()));
        hour_color_selector.set_theme_config(theme_config.borrow().clone());
        colors_grid.attach(hour_color_selector.widget(), 1, 1, 1, 1);
        let hour_width_label = Label::new(Some("Width:"));
        colors_grid.attach(&hour_width_label, 2, 1, 1, 1);
        let hour_width_adj = Adjustment::new(6.0, 1.0, 20.0, 0.5, 1.0, 0.0);
        let hour_width_spin = SpinButton::new(Some(&hour_width_adj), 0.5, 1);
        hour_width_spin.set_hexpand(true);
        colors_grid.attach(&hour_width_spin, 3, 1, 1, 1);

        // Minute hand row
        let minute_label = Label::new(Some("Minute Hand:"));
        minute_label.set_xalign(1.0);
        minute_label.set_width_request(label_width);
        colors_grid.attach(&minute_label, 0, 2, 1, 1);
        let minute_color_selector = Rc::new(ThemeColorSelector::new(config.borrow().minute_hand_color.clone()));
        minute_color_selector.set_theme_config(theme_config.borrow().clone());
        colors_grid.attach(minute_color_selector.widget(), 1, 2, 1, 1);
        let minute_width_label = Label::new(Some("Width:"));
        colors_grid.attach(&minute_width_label, 2, 2, 1, 1);
        let minute_width_adj = Adjustment::new(4.0, 1.0, 20.0, 0.5, 1.0, 0.0);
        let minute_width_spin = SpinButton::new(Some(&minute_width_adj), 0.5, 1);
        minute_width_spin.set_hexpand(true);
        colors_grid.attach(&minute_width_spin, 3, 2, 1, 1);

        // Second hand row
        let second_label = Label::new(Some("Second Hand:"));
        second_label.set_xalign(1.0);
        second_label.set_width_request(label_width);
        colors_grid.attach(&second_label, 0, 3, 1, 1);
        let second_color_selector = Rc::new(ThemeColorSelector::new(config.borrow().second_hand_color.clone()));
        second_color_selector.set_theme_config(theme_config.borrow().clone());
        colors_grid.attach(second_color_selector.widget(), 1, 3, 1, 1);
        let second_width_label = Label::new(Some("Width:"));
        colors_grid.attach(&second_width_label, 2, 3, 1, 1);
        let second_width_adj = Adjustment::new(2.0, 0.5, 10.0, 0.5, 1.0, 0.0);
        let second_width_spin = SpinButton::new(Some(&second_width_adj), 0.5, 1);
        second_width_spin.set_hexpand(true);
        colors_grid.attach(&second_width_spin, 3, 3, 1, 1);

        // Center hub row
        let hub_label = Label::new(Some("Center Hub:"));
        hub_label.set_xalign(1.0);
        hub_label.set_width_request(label_width);
        colors_grid.attach(&hub_label, 0, 4, 1, 1);
        let center_hub_color_selector = Rc::new(ThemeColorSelector::new(config.borrow().center_hub_color.clone()));
        center_hub_color_selector.set_theme_config(theme_config.borrow().clone());
        colors_grid.attach(center_hub_color_selector.widget(), 1, 4, 1, 1);
        let hub_size_label = Label::new(Some("Size %:"));
        colors_grid.attach(&hub_size_label, 2, 4, 1, 1);
        let hub_size_adj = Adjustment::new(config.borrow().center_hub_size * 100.0, 1.0, 20.0, 0.5, 1.0, 0.0);
        let center_hub_size_spin = SpinButton::new(Some(&hub_size_adj), 0.5, 1);
        center_hub_size_spin.set_hexpand(true);
        colors_grid.attach(&center_hub_size_spin, 3, 4, 1, 1);

        colors_frame.set_child(Some(&colors_grid));
        hands_tab_box.append(&colors_frame);

        // Connect hand color callbacks
        let config_for_border_c = config.clone();
        border_color_selector.set_on_change(move |color_source| {
            config_for_border_c.borrow_mut().border_color = color_source;
        });

        let config_for_hour_c = config.clone();
        hour_color_selector.set_on_change(move |color_source| {
            config_for_hour_c.borrow_mut().hour_hand_color = color_source;
        });

        let config_for_minute_c = config.clone();
        minute_color_selector.set_on_change(move |color_source| {
            config_for_minute_c.borrow_mut().minute_hand_color = color_source;
        });

        let config_for_second_c = config.clone();
        second_color_selector.set_on_change(move |color_source| {
            config_for_second_c.borrow_mut().second_hand_color = color_source;
        });

        let config_for_hub_c = config.clone();
        center_hub_color_selector.set_on_change(move |color_source| {
            config_for_hub_c.borrow_mut().center_hub_color = color_source;
        });

        let config_for_hub_size = config.clone();
        center_hub_size_spin.connect_value_changed(move |spin| {
            // Convert from percentage to fraction (5% -> 0.05)
            config_for_hub_size.borrow_mut().center_hub_size = spin.value() / 100.0;
        });

        // Add Hands tab to notebook
        notebook.append_page(&hands_tab_box, Some(&Label::new(Some("Hands"))));

        // ============ TAB 5: Icon ============
        let icon_tab_box = GtkBox::new(Orientation::Vertical, 8);
        icon_tab_box.set_margin_start(8);
        icon_tab_box.set_margin_end(8);
        icon_tab_box.set_margin_top(8);
        icon_tab_box.set_margin_bottom(8);

        // Icon Section
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

        // Layout options separator
        let layout_label = Label::new(Some("Layout Options:"));
        layout_label.set_xalign(0.0);
        layout_label.set_margin_top(8);
        icon_box.append(&layout_label);

        // Center indicator checkbox
        let center_indicator_check = CheckButton::with_label("Center indicator below clock");
        center_indicator_check.set_active(config.borrow().center_indicator);
        center_indicator_check.set_tooltip_text(Some("Place the alarm/timer indicator centered below the clock face"));
        icon_box.append(&center_indicator_check);

        // Shrink for indicator checkbox
        let shrink_for_indicator_check = CheckButton::with_label("Shrink clock when indicator visible");
        shrink_for_indicator_check.set_active(config.borrow().shrink_for_indicator);
        shrink_for_indicator_check.set_tooltip_text(Some("Reduce clock size to prevent overlap with indicator"));
        icon_box.append(&shrink_for_indicator_check);

        icon_frame.set_child(Some(&icon_box));
        icon_tab_box.append(&icon_frame);

        // Add Icon tab to notebook
        notebook.append_page(&icon_tab_box, Some(&Label::new(Some("Icon"))));

        // ============ Connect Signals ============

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

        // Face style dropdown (now in Numbers tab)
        let config_for_face = config.clone();
        face_dropdown.connect_selected_notify(move |dropdown| {
            let mut cfg = config_for_face.borrow_mut();
            cfg.face_style = match dropdown.selected() {
                0 => FaceStyle::Minimal, // None - shows no numbers
                1 => FaceStyle::Minimal,
                2 => FaceStyle::Classic,
                3 => FaceStyle::Roman,
                _ => FaceStyle::Classic,
            };
            // When "None" is selected, hide numbers
            cfg.show_numbers = dropdown.selected() != 0;
        });

        // Hand style dropdown
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
                    let family = font_desc.family().map(|f| f.to_string()).unwrap_or_else(|| "Noto Color Emoji".to_string());
                    config_clone.borrow_mut().icon_font = family.clone();
                    let size_pct = size_spin.value();
                    font_btn.set_label(&format!("{} {:.0}%", family, size_pct));
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

        // Center indicator checkbox
        let config_for_center = config.clone();
        center_indicator_check.connect_toggled(move |check| {
            config_for_center.borrow_mut().center_indicator = check.is_active();
        });

        // Shrink for indicator checkbox
        let config_for_shrink = config.clone();
        shrink_for_indicator_check.connect_toggled(move |check| {
            config_for_shrink.borrow_mut().shrink_for_indicator = check.is_active();
        });

        Self {
            widget: notebook,
            config,
            theme_config,
            background_widget,
            face_dropdown,
            number_font_selector,
            number_bold_check,
            number_italic_check,
            number_color_selector,
            show_numbers_check,
            hour_tick_style_dropdown,
            hour_tick_color_selector,
            hour_tick_outer_spin,
            hour_tick_inner_spin,
            minute_tick_style_dropdown,
            minute_tick_color_selector,
            minute_tick_outer_spin,
            minute_tick_inner_spin,
            hand_dropdown,
            show_second_hand_check,
            smooth_seconds_check,
            show_center_hub_check,
            border_color_selector,
            border_width_spin,
            hour_color_selector,
            hour_width_spin,
            minute_color_selector,
            minute_width_spin,
            second_color_selector,
            second_width_spin,
            center_hub_color_selector,
            center_hub_size_spin,
            show_icon_check,
            icon_text_entry,
            icon_font_button,
            icon_size_spin,
            icon_bold_check,
            center_indicator_check,
            shrink_for_indicator_check,
        }
    }

    pub fn widget(&self) -> &Notebook {
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

        // Face style in Numbers tab - map to new options (None, Minimal, Classic, Roman)
        let face_index = match config.face_style {
            FaceStyle::Minimal => if config.show_numbers { 1 } else { 0 },
            FaceStyle::Classic => 2,
            FaceStyle::Roman => 3,
            FaceStyle::Modern => 2, // Map old Modern to Classic
            FaceStyle::Numbers => 2, // Map old Numbers to Classic
        };
        self.face_dropdown.set_selected(face_index);
        self.show_numbers_check.set_active(config.show_numbers);

        // Number font - convert fraction to percentage for display (0.12 -> 12)
        let font_as_percentage = config.number_font.with_size(config.number_font.size() * 100.0);
        self.number_font_selector.set_source(font_as_percentage);
        self.number_bold_check.set_active(config.number_bold);
        self.number_italic_check.set_active(config.number_italic);
        self.number_color_selector.set_source(config.number_color.clone());

        // Hour ticks
        self.hour_tick_style_dropdown.set_selected(tick_style_to_index(config.hour_tick_style));
        self.hour_tick_color_selector.set_source(config.hour_tick_color.clone());
        self.hour_tick_outer_spin.set_value(config.hour_tick_outer_percent);
        self.hour_tick_inner_spin.set_value(config.hour_tick_inner_percent);

        // Minute ticks
        self.minute_tick_style_dropdown.set_selected(tick_style_to_index(config.minute_tick_style));
        self.minute_tick_color_selector.set_source(config.minute_tick_color.clone());
        self.minute_tick_outer_spin.set_value(config.minute_tick_outer_percent);
        self.minute_tick_inner_spin.set_value(config.minute_tick_inner_percent);

        // Hand style
        self.hand_dropdown.set_selected(match config.hour_hand_style {
            HandStyle::Line => 0,
            HandStyle::Arrow => 1,
            HandStyle::Sword => 2,
            HandStyle::Fancy => 3,
        });

        // Checkboxes
        self.show_second_hand_check.set_active(config.show_second_hand);
        self.smooth_seconds_check.set_active(config.smooth_seconds);
        self.show_center_hub_check.set_active(config.show_center_hub);

        // Size spinners
        self.border_width_spin.set_value(config.border_width);
        self.hour_width_spin.set_value(config.hour_hand_width);
        self.minute_width_spin.set_value(config.minute_hand_width);
        self.second_width_spin.set_value(config.second_hand_width);

        // Color selectors
        self.border_color_selector.set_source(config.border_color.clone());
        self.hour_color_selector.set_source(config.hour_hand_color.clone());
        self.minute_color_selector.set_source(config.minute_hand_color.clone());
        self.second_color_selector.set_source(config.second_hand_color.clone());
        self.center_hub_color_selector.set_source(config.center_hub_color.clone());
        // Convert fraction to percentage (0.05 -> 5%)
        self.center_hub_size_spin.set_value(config.center_hub_size * 100.0);

        // Icon config
        self.show_icon_check.set_active(config.show_icon);
        self.icon_text_entry.set_text(&config.icon_text);
        self.icon_font_button.set_label(&format!("{} {:.0}%", config.icon_font, config.icon_size));
        self.icon_size_spin.set_value(config.icon_size);
        self.icon_bold_check.set_active(config.icon_bold);
        self.center_indicator_check.set_active(config.center_indicator);
        self.shrink_for_indicator_check.set_active(config.shrink_for_indicator);

        // Store config
        *self.config.borrow_mut() = config;
    }

    /// Set the theme configuration for theme-aware selectors
    pub fn set_theme(&self, theme: ComboThemeConfig) {
        *self.theme_config.borrow_mut() = theme.clone();
        self.background_widget.set_theme_config(theme.clone());
        self.number_font_selector.set_theme_config(theme.clone());
        self.number_color_selector.set_theme_config(theme.clone());
        self.hour_tick_color_selector.set_theme_config(theme.clone());
        self.minute_tick_color_selector.set_theme_config(theme.clone());
        self.border_color_selector.set_theme_config(theme.clone());
        self.hour_color_selector.set_theme_config(theme.clone());
        self.minute_color_selector.set_theme_config(theme.clone());
        self.second_color_selector.set_theme_config(theme.clone());
        self.center_hub_color_selector.set_theme_config(theme);
    }
}

impl Default for ClockAnalogConfigWidget {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert dropdown index to TickStyle
fn tick_style_from_index(index: u32) -> TickStyle {
    match index {
        0 => TickStyle::None,
        1 => TickStyle::Squares,
        2 => TickStyle::Lines,
        3 => TickStyle::Dots,
        4 => TickStyle::Triangles,
        _ => TickStyle::Lines,
    }
}

/// Convert TickStyle to dropdown index
fn tick_style_to_index(style: TickStyle) -> u32 {
    match style {
        TickStyle::None => 0,
        TickStyle::Squares => 1,
        TickStyle::Lines => 2,
        TickStyle::Dots => 3,
        TickStyle::Triangles => 4,
    }
}
