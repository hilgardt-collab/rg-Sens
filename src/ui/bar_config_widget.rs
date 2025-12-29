//! Bar display configuration widget with tabbed interface

use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, CheckButton, DrawingArea, DropDown, Label,
    Notebook, Orientation, SpinButton, Stack, StringList,
};
use std::cell::RefCell;
use std::rc::Rc;

use crate::ui::bar_display::{
    BarBackgroundType, BarDisplayConfig, BarFillDirection, BarFillType, BarOrientation,
    BarStyle, BarTaperAlignment, BarTaperStyle, render_bar,
};
use crate::ui::background::{Color, ColorStop};
use crate::ui::render_utils::render_checkerboard;
use crate::ui::GradientEditor;
use crate::ui::TextLineConfigWidget;
use crate::ui::theme::{ColorSource, ColorStopSource, ComboThemeConfig};
use crate::ui::theme_color_selector::ThemeColorSelector;
use crate::core::FieldMetadata;

/// Bar configuration widget
pub struct BarConfigWidget {
    container: GtkBox,
    config: Rc<RefCell<BarDisplayConfig>>,
    theme: Rc<RefCell<ComboThemeConfig>>,
    preview: DrawingArea,
    on_change: Rc<RefCell<Option<Box<dyn Fn()>>>>,

    // Bar style UI elements
    style_dropdown: DropDown,
    orientation_dropdown: DropDown,
    direction_dropdown: DropDown,
    style_stack: Stack,

    // Foreground UI elements
    fg_solid_radio: CheckButton,
    fg_gradient_radio: CheckButton,
    fg_color_widget: Rc<ThemeColorSelector>,
    fg_gradient_editor: Rc<GradientEditor>,

    // Background UI elements
    bg_solid_radio: CheckButton,
    bg_gradient_radio: CheckButton,
    bg_transparent_radio: CheckButton,
    bg_color_widget: Rc<ThemeColorSelector>,
    bg_gradient_editor: Rc<GradientEditor>,

    // Border color widget (for theme updates)
    border_color_widget: Rc<ThemeColorSelector>,

    // Rectangle options UI elements
    rect_width_spin: SpinButton,
    rect_height_spin: SpinButton,
    corner_radius_spin: SpinButton,
    padding_spin: SpinButton,

    // Segmented options UI elements
    segment_count_spin: SpinButton,
    segment_spacing_spin: SpinButton,
    segment_width_spin: SpinButton,
    segment_height_spin: SpinButton,
    segment_corner_radius_spin: SpinButton,

    // Taper UI elements
    taper_style_dropdown: DropDown,
    taper_amount_spin: SpinButton,
    taper_alignment_dropdown: DropDown,

    // Border UI elements
    border_check: CheckButton,
    border_width_spin: SpinButton,

    // Animation UI elements
    animate_check: CheckButton,
    animation_speed_spin: SpinButton,

    // Text overlay
    text_config_widget: Option<Rc<TextLineConfigWidget>>,
}

impl BarConfigWidget {
    pub fn new(available_fields: Vec<FieldMetadata>) -> Self {
        let container = GtkBox::new(Orientation::Vertical, 12);
        container.set_margin_start(12);
        container.set_margin_end(12);
        container.set_margin_top(12);
        container.set_margin_bottom(12);

        let config = Rc::new(RefCell::new(BarDisplayConfig::default()));
        let theme = Rc::new(RefCell::new(ComboThemeConfig::default()));
        let on_change: Rc<RefCell<Option<Box<dyn Fn()>>>> = Rc::new(RefCell::new(None));

        // Create notebook for tabs
        let notebook = Notebook::new();
        notebook.set_vexpand(true);

        // === Tab 1: Bar Style ===
        let style_page = GtkBox::new(Orientation::Vertical, 12);
        style_page.set_margin_start(12);
        style_page.set_margin_end(12);
        style_page.set_margin_top(12);
        style_page.set_margin_bottom(12);

        // Style selector
        let style_box = GtkBox::new(Orientation::Horizontal, 6);
        style_box.append(&Label::new(Some("Style:")));
        let style_options = StringList::new(&["Full Panel", "Rectangle", "Segmented"]);
        let style_dropdown = DropDown::new(Some(style_options), Option::<gtk4::Expression>::None);
        style_dropdown.set_selected(0);
        style_dropdown.set_hexpand(true);
        style_box.append(&style_dropdown);
        style_page.append(&style_box);

        // Orientation selector
        let orientation_box = GtkBox::new(Orientation::Horizontal, 6);
        orientation_box.append(&Label::new(Some("Orientation:")));
        let orientation_options = StringList::new(&["Horizontal", "Vertical"]);
        let orientation_dropdown = DropDown::new(Some(orientation_options), Option::<gtk4::Expression>::None);
        orientation_dropdown.set_selected(0);
        orientation_dropdown.set_hexpand(true);
        orientation_box.append(&orientation_dropdown);
        style_page.append(&orientation_box);

        // Fill direction selector
        let direction_box = GtkBox::new(Orientation::Horizontal, 6);
        direction_box.append(&Label::new(Some("Fill Direction:")));
        let direction_options = StringList::new(&["Left to Right", "Right to Left", "Bottom to Top", "Top to Bottom"]);
        let direction_dropdown = DropDown::new(Some(direction_options), Option::<gtk4::Expression>::None);
        direction_dropdown.set_selected(0);
        direction_dropdown.set_hexpand(true);
        direction_box.append(&direction_dropdown);
        style_page.append(&direction_box);

        // Preview
        let preview = DrawingArea::new();
        preview.set_content_height(100);
        preview.set_hexpand(true);
        preview.set_vexpand(true);

        let config_clone = config.clone();
        let theme_clone = theme.clone();
        preview.set_draw_func(move |_, cr, width, height| {
            // Render checkerboard pattern to show transparency
            render_checkerboard(cr, width as f64, height as f64);

            let cfg = config_clone.borrow();
            let thm = theme_clone.borrow();
            let mut preview_values = std::collections::HashMap::new();
            preview_values.insert("value".to_string(), serde_json::json!(75.0));
            preview_values.insert("percent".to_string(), serde_json::json!(75.0));
            let _ = render_bar(cr, &cfg, &thm, 0.75, &preview_values, width as f64, height as f64);
        });

        style_page.append(&preview);

        notebook.append_page(&style_page, Some(&Label::new(Some("Bar Style"))));

        // === Tab 2: Foreground ===
        let fg_page = GtkBox::new(Orientation::Vertical, 12);
        fg_page.set_margin_start(12);
        fg_page.set_margin_end(12);
        fg_page.set_margin_top(12);
        fg_page.set_margin_bottom(12);

        let fg_type_box = GtkBox::new(Orientation::Horizontal, 12);
        let fg_solid_radio = CheckButton::with_label("Solid Color");
        fg_solid_radio.set_active(true);
        fg_type_box.append(&fg_solid_radio);

        let fg_gradient_radio = CheckButton::with_label("Gradient");
        fg_gradient_radio.set_group(Some(&fg_solid_radio));
        fg_type_box.append(&fg_gradient_radio);
        fg_page.append(&fg_type_box);

        // Foreground solid color - using ThemeColorSelector
        let fg_color_box = GtkBox::new(Orientation::Horizontal, 6);
        fg_color_box.append(&Label::new(Some("Solid Color:")));
        let initial_fg_source = if let BarFillType::Solid { color } = &config.borrow().foreground {
            color.clone()
        } else {
            ColorSource::Theme { index: 1 }
        };
        let fg_color_widget = Rc::new(ThemeColorSelector::new(initial_fg_source));
        fg_color_widget.set_theme_config(theme.borrow().clone());
        fg_color_box.append(fg_color_widget.widget());
        fg_page.append(&fg_color_box);

        // Copy/Paste gradient buttons for foreground
        let fg_copy_paste_box = GtkBox::new(Orientation::Horizontal, 6);
        let fg_copy_gradient_btn = Button::with_label("Copy Gradient");
        let fg_paste_gradient_btn = Button::with_label("Paste Gradient");
        fg_copy_paste_box.append(&fg_copy_gradient_btn);
        fg_copy_paste_box.append(&fg_paste_gradient_btn);
        fg_copy_paste_box.set_visible(false); // Hidden when solid is selected
        fg_page.append(&fg_copy_paste_box);

        let fg_gradient_editor = GradientEditor::new();
        fg_gradient_editor.widget().set_visible(false);
        fg_gradient_editor.widget().set_vexpand(true);
        fg_page.append(fg_gradient_editor.widget());
        let fg_gradient_editor = Rc::new(fg_gradient_editor);

        notebook.append_page(&fg_page, Some(&Label::new(Some("Foreground"))));

        // === Tab 3: Background ===
        let bg_page = GtkBox::new(Orientation::Vertical, 12);
        bg_page.set_margin_start(12);
        bg_page.set_margin_end(12);
        bg_page.set_margin_top(12);
        bg_page.set_margin_bottom(12);

        let bg_type_box = GtkBox::new(Orientation::Horizontal, 12);
        let bg_solid_radio = CheckButton::with_label("Solid Color");
        bg_solid_radio.set_active(true);
        bg_type_box.append(&bg_solid_radio);

        let bg_gradient_radio = CheckButton::with_label("Gradient");
        bg_gradient_radio.set_group(Some(&bg_solid_radio));
        bg_type_box.append(&bg_gradient_radio);

        let bg_transparent_radio = CheckButton::with_label("Transparent");
        bg_transparent_radio.set_group(Some(&bg_solid_radio));
        bg_type_box.append(&bg_transparent_radio);
        bg_page.append(&bg_type_box);

        // Background solid color - using ThemeColorSelector
        let bg_color_box = GtkBox::new(Orientation::Horizontal, 6);
        bg_color_box.append(&Label::new(Some("Solid Color:")));
        let initial_bg_source = if let BarBackgroundType::Solid { color } = &config.borrow().background {
            color.clone()
        } else {
            ColorSource::Custom { color: Color::new(0.15, 0.15, 0.15, 0.8) }
        };
        let bg_color_widget = Rc::new(ThemeColorSelector::new(initial_bg_source));
        bg_color_widget.set_theme_config(theme.borrow().clone());
        bg_color_box.append(bg_color_widget.widget());
        bg_page.append(&bg_color_box);

        // Copy/Paste gradient buttons for background
        let bg_copy_paste_box = GtkBox::new(Orientation::Horizontal, 6);
        let bg_copy_gradient_btn = Button::with_label("Copy Gradient");
        let bg_paste_gradient_btn = Button::with_label("Paste Gradient");
        bg_copy_paste_box.append(&bg_copy_gradient_btn);
        bg_copy_paste_box.append(&bg_paste_gradient_btn);
        bg_copy_paste_box.set_visible(false); // Hidden when solid/transparent is selected
        bg_page.append(&bg_copy_paste_box);

        let bg_gradient_editor = GradientEditor::new();
        bg_gradient_editor.widget().set_visible(false);
        bg_gradient_editor.widget().set_vexpand(true);
        bg_page.append(bg_gradient_editor.widget());
        let bg_gradient_editor = Rc::new(bg_gradient_editor);

        notebook.append_page(&bg_page, Some(&Label::new(Some("Background"))));

        // === Tab 4: Style Options ===
        let options_page = GtkBox::new(Orientation::Vertical, 12);
        options_page.set_margin_start(12);
        options_page.set_margin_end(12);
        options_page.set_margin_top(12);
        options_page.set_margin_bottom(12);

        // Taper style selector
        let taper_box = GtkBox::new(Orientation::Horizontal, 6);
        taper_box.append(&Label::new(Some("Taper Style:")));
        let taper_options = StringList::new(&["None", "Start", "End", "Both"]);
        let taper_style_dropdown = DropDown::new(Some(taper_options), Option::<gtk4::Expression>::None);
        let taper_index = match config.borrow().taper_style {
            BarTaperStyle::None => 0,
            BarTaperStyle::Start => 1,
            BarTaperStyle::End => 2,
            BarTaperStyle::Both => 3,
        };
        taper_style_dropdown.set_selected(taper_index);
        taper_style_dropdown.set_hexpand(true);
        taper_box.append(&taper_style_dropdown);
        options_page.append(&taper_box);

        // Taper amount
        let taper_amount_box = GtkBox::new(Orientation::Horizontal, 6);
        taper_amount_box.append(&Label::new(Some("Taper Amount (%):")));
        let taper_amount_spin = SpinButton::with_range(0.0, 100.0, 5.0);
        taper_amount_spin.set_value(config.borrow().taper_amount * 100.0);
        taper_amount_spin.set_hexpand(true);
        taper_amount_box.append(&taper_amount_spin);
        options_page.append(&taper_amount_box);

        // Taper alignment selector
        let taper_align_box = GtkBox::new(Orientation::Horizontal, 6);
        taper_align_box.append(&Label::new(Some("Taper Alignment:")));
        let taper_align_options = StringList::new(&["Top / Left", "Center", "Bottom / Right"]);
        let taper_alignment_dropdown = DropDown::new(Some(taper_align_options), Option::<gtk4::Expression>::None);
        let align_index = match config.borrow().taper_alignment {
            BarTaperAlignment::Start => 0,
            BarTaperAlignment::Center => 1,
            BarTaperAlignment::End => 2,
        };
        taper_alignment_dropdown.set_selected(align_index);
        taper_alignment_dropdown.set_hexpand(true);
        taper_align_box.append(&taper_alignment_dropdown);
        options_page.append(&taper_align_box);

        // Taper style change handler
        let config_clone = config.clone();
        let preview_clone = preview.clone();
        let on_change_clone = on_change.clone();
        taper_style_dropdown.connect_selected_notify(move |dropdown| {
            let style = match dropdown.selected() {
                0 => BarTaperStyle::None,
                1 => BarTaperStyle::Start,
                2 => BarTaperStyle::End,
                3 => BarTaperStyle::Both,
                _ => BarTaperStyle::None,
            };
            config_clone.borrow_mut().taper_style = style;
            preview_clone.queue_draw();
            if let Some(callback) = on_change_clone.borrow().as_ref() {
                callback();
            }
        });

        // Taper amount change handler
        let config_clone = config.clone();
        let preview_clone = preview.clone();
        let on_change_clone = on_change.clone();
        taper_amount_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().taper_amount = spin.value() / 100.0;
            preview_clone.queue_draw();
            if let Some(callback) = on_change_clone.borrow().as_ref() {
                callback();
            }
        });

        // Taper alignment change handler
        let config_clone = config.clone();
        let preview_clone = preview.clone();
        let on_change_clone = on_change.clone();
        taper_alignment_dropdown.connect_selected_notify(move |dropdown| {
            let alignment = match dropdown.selected() {
                0 => BarTaperAlignment::Start,
                1 => BarTaperAlignment::Center,
                2 => BarTaperAlignment::End,
                _ => BarTaperAlignment::Center,
            };
            config_clone.borrow_mut().taper_alignment = alignment;
            preview_clone.queue_draw();
            if let Some(callback) = on_change_clone.borrow().as_ref() {
                callback();
            }
        });

        let style_stack = Stack::new();
        style_stack.set_vexpand(true);

        let (rect_page, rect_width_spin, rect_height_spin, corner_radius_spin, padding_spin) =
            Self::create_rectangle_options(&config, &preview, &on_change);
        style_stack.add_named(&rect_page, Some("rectangle"));

        let (seg_page, segment_count_spin, segment_spacing_spin, segment_width_spin, segment_height_spin, segment_corner_radius_spin) =
            Self::create_segmented_options(&config, &preview, &on_change);
        style_stack.add_named(&seg_page, Some("segmented"));

        let empty_page = GtkBox::new(Orientation::Vertical, 0);
        style_stack.add_named(&empty_page, Some("full"));

        options_page.append(&style_stack);

        notebook.append_page(&options_page, Some(&Label::new(Some("Style Options"))));

        // === Tab 5: Text Overlay ===
        let text_page = GtkBox::new(Orientation::Vertical, 12);
        text_page.set_margin_start(12);
        text_page.set_margin_end(12);
        text_page.set_margin_top(12);
        text_page.set_margin_bottom(12);

        let text_check = CheckButton::with_label("Show Text Overlay");
        text_check.set_active(true);
        text_page.append(&text_check);

        let text_config_widget = TextLineConfigWidget::new(available_fields);
        text_config_widget.widget().set_vexpand(true);
        text_page.append(text_config_widget.widget());
        let text_config_widget = Rc::new(text_config_widget);

        // Connect text config widget changes to trigger on_change callback
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let config_clone = config.clone();
        let text_widget_for_change = text_config_widget.clone();
        text_config_widget.set_on_change(move || {
            // Update the stored text config when text widget changes
            config_clone.borrow_mut().text_overlay.text_config = text_widget_for_change.get_config();
            preview_clone.queue_draw();
            if let Some(callback) = on_change_clone.borrow().as_ref() {
                callback();
            }
        });

        notebook.append_page(&text_page, Some(&Label::new(Some("Text Overlay"))));

        // === Tab 6: Border ===
        let border_page = GtkBox::new(Orientation::Vertical, 12);
        border_page.set_margin_start(12);
        border_page.set_margin_end(12);
        border_page.set_margin_top(12);
        border_page.set_margin_bottom(12);

        let border_check = CheckButton::with_label("Show Border");
        border_page.append(&border_check);

        let border_width_box = GtkBox::new(Orientation::Horizontal, 6);
        border_width_box.append(&Label::new(Some("Width:")));
        let border_width_spin = SpinButton::with_range(1.0, 10.0, 0.5);
        border_width_spin.set_value(1.0);
        border_width_spin.set_hexpand(true);
        border_width_box.append(&border_width_spin);
        border_page.append(&border_width_box);

        // Border color - using ThemeColorSelector
        let border_color_box = GtkBox::new(Orientation::Horizontal, 6);
        border_color_box.append(&Label::new(Some("Color:")));
        let border_color_widget = Rc::new(ThemeColorSelector::new(config.borrow().border.color.clone()));
        border_color_widget.set_theme_config(theme.borrow().clone());
        border_color_box.append(border_color_widget.widget());
        border_page.append(&border_color_box);

        notebook.append_page(&border_page, Some(&Label::new(Some("Border"))));

        // === Tab 7: Animation ===
        let animation_page = GtkBox::new(Orientation::Vertical, 12);
        animation_page.set_margin_start(12);
        animation_page.set_margin_end(12);
        animation_page.set_margin_top(12);
        animation_page.set_margin_bottom(12);

        let animate_check = CheckButton::with_label("Enable smooth animation");
        animate_check.set_active(config.borrow().smooth_animation);
        animation_page.append(&animate_check);

        let speed_box = GtkBox::new(Orientation::Horizontal, 6);
        speed_box.append(&Label::new(Some("Animation Speed:")));
        let animation_speed_spin = SpinButton::with_range(0.1, 1.0, 0.1);
        animation_speed_spin.set_value(config.borrow().animation_speed);
        animation_speed_spin.set_digits(1);
        animation_speed_spin.set_hexpand(true);
        speed_box.append(&animation_speed_spin);
        animation_page.append(&speed_box);

        // Help text
        let help_label = Label::new(Some("Animation smoothly transitions the bar value.\nHigher speed = faster transition."));
        help_label.set_halign(gtk4::Align::Start);
        help_label.add_css_class("dim-label");
        animation_page.append(&help_label);

        notebook.append_page(&animation_page, Some(&Label::new(Some("Animation"))));

        // === Copy/Paste buttons for entire bar config ===
        let copy_paste_box = GtkBox::new(Orientation::Horizontal, 6);
        copy_paste_box.set_halign(gtk4::Align::End);
        copy_paste_box.set_margin_bottom(6);

        let copy_btn = Button::with_label("Copy Bar Config");
        let paste_btn = Button::with_label("Paste Bar Config");

        copy_paste_box.append(&copy_btn);
        copy_paste_box.append(&paste_btn);

        container.append(&copy_paste_box);
        container.append(&notebook);

        // === Event Handlers ===

        // Style change handler
        let config_clone = config.clone();
        let preview_clone = preview.clone();
        let on_change_clone = on_change.clone();
        let style_stack_clone = style_stack.clone();

        style_dropdown.connect_selected_notify(move |dropdown| {
            let selected = dropdown.selected();
            let (page_name, style) = match selected {
                0 => ("full", BarStyle::Full),
                1 => ("rectangle", BarStyle::Rectangle),
                2 => ("segmented", BarStyle::Segmented),
                _ => ("full", BarStyle::Full),
            };

            style_stack_clone.set_visible_child_name(page_name);

            let mut cfg = config_clone.borrow_mut();
            cfg.style = style;
            drop(cfg);

            preview_clone.queue_draw();

            if let Some(callback) = on_change_clone.borrow().as_ref() {
                callback();
            }
        });

        // Orientation change handler
        let config_clone = config.clone();
        let preview_clone = preview.clone();
        let on_change_clone = on_change.clone();

        orientation_dropdown.connect_selected_notify(move |dropdown| {
            let orientation = match dropdown.selected() {
                0 => BarOrientation::Horizontal,
                1 => BarOrientation::Vertical,
                _ => BarOrientation::Horizontal,
            };

            let mut cfg = config_clone.borrow_mut();
            cfg.orientation = orientation;
            drop(cfg);

            preview_clone.queue_draw();

            if let Some(callback) = on_change_clone.borrow().as_ref() {
                callback();
            }
        });

        // Fill direction change handler
        let config_clone = config.clone();
        let preview_clone = preview.clone();
        let on_change_clone = on_change.clone();

        direction_dropdown.connect_selected_notify(move |dropdown| {
            let direction = match dropdown.selected() {
                0 => BarFillDirection::LeftToRight,
                1 => BarFillDirection::RightToLeft,
                2 => BarFillDirection::BottomToTop,
                3 => BarFillDirection::TopToBottom,
                _ => BarFillDirection::LeftToRight,
            };

            let mut cfg = config_clone.borrow_mut();
            cfg.fill_direction = direction;
            drop(cfg);

            preview_clone.queue_draw();

            if let Some(callback) = on_change_clone.borrow().as_ref() {
                callback();
            }
        });

        // Foreground type toggle handlers
        Self::setup_fg_handlers(
            &config,
            &theme,
            &preview,
            &on_change,
            fg_solid_radio.clone(),
            fg_gradient_radio.clone(),
            fg_color_widget.clone(),
            fg_gradient_editor.clone(),
            fg_copy_paste_box.clone(),
            fg_copy_gradient_btn.clone(),
            fg_paste_gradient_btn.clone(),
        );

        // Background type toggle handlers
        Self::setup_bg_handlers(
            &config,
            &theme,
            &preview,
            &on_change,
            bg_solid_radio.clone(),
            bg_gradient_radio.clone(),
            bg_transparent_radio.clone(),
            bg_color_widget.clone(),
            bg_gradient_editor.clone(),
            bg_copy_paste_box.clone(),
            bg_copy_gradient_btn.clone(),
            bg_paste_gradient_btn.clone(),
        );

        // Text overlay checkbox handler
        let config_clone = config.clone();
        let preview_clone = preview.clone();
        let on_change_clone = on_change.clone();
        let text_widget_clone = text_config_widget.clone();

        text_check.connect_toggled(move |check| {
            let enabled = check.is_active();
            text_widget_clone.widget().set_sensitive(enabled);

            let mut cfg = config_clone.borrow_mut();
            cfg.text_overlay.enabled = enabled;
            drop(cfg);

            preview_clone.queue_draw();

            if let Some(callback) = on_change_clone.borrow().as_ref() {
                callback();
            }
        });

        // Border handlers
        let config_clone = config.clone();
        let preview_clone = preview.clone();
        let on_change_clone = on_change.clone();

        border_check.connect_toggled(move |check| {
            let mut cfg = config_clone.borrow_mut();
            cfg.border.enabled = check.is_active();
            drop(cfg);

            preview_clone.queue_draw();

            if let Some(callback) = on_change_clone.borrow().as_ref() {
                callback();
            }
        });

        let config_clone = config.clone();
        let preview_clone = preview.clone();
        let on_change_clone = on_change.clone();

        border_width_spin.connect_value_changed(move |spin| {
            let mut cfg = config_clone.borrow_mut();
            cfg.border.width = spin.value();
            drop(cfg);

            preview_clone.queue_draw();

            if let Some(callback) = on_change_clone.borrow().as_ref() {
                callback();
            }
        });

        // Border color handler - using ColorButtonWidget
        let config_clone = config.clone();
        let preview_clone = preview.clone();
        let on_change_clone = on_change.clone();
        border_color_widget.set_on_change(move |color_source| {
            config_clone.borrow_mut().border.color = color_source;
            preview_clone.queue_draw();
            if let Some(callback) = on_change_clone.borrow().as_ref() {
                callback();
            }
        });

        // Animation handlers
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        animate_check.connect_toggled(move |check| {
            config_clone.borrow_mut().smooth_animation = check.is_active();
            if let Some(callback) = on_change_clone.borrow().as_ref() {
                callback();
            }
        });

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        animation_speed_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().animation_speed = spin.value();
            if let Some(callback) = on_change_clone.borrow().as_ref() {
                callback();
            }
        });

        // Copy button handler
        let config_for_copy = config.clone();
        copy_btn.connect_clicked(move |_| {
            let cfg = config_for_copy.borrow().clone();
            if let Ok(mut clipboard) = crate::ui::clipboard::CLIPBOARD.lock() {
                clipboard.copy_bar_display(cfg);
            }
        });

        // Paste button handler
        let config_for_paste = config.clone();
        let preview_for_paste = preview.clone();
        let on_change_for_paste = on_change.clone();
        let style_dropdown_paste = style_dropdown.clone();
        let orientation_dropdown_paste = orientation_dropdown.clone();
        let direction_dropdown_paste = direction_dropdown.clone();
        let fg_solid_radio_paste = fg_solid_radio.clone();
        let fg_gradient_radio_paste = fg_gradient_radio.clone();
        let fg_color_widget_paste = fg_color_widget.clone();
        let fg_gradient_editor_paste = fg_gradient_editor.clone();
        let bg_solid_radio_paste = bg_solid_radio.clone();
        let bg_gradient_radio_paste = bg_gradient_radio.clone();
        let bg_transparent_radio_paste = bg_transparent_radio.clone();
        let bg_color_widget_paste = bg_color_widget.clone();
        let bg_gradient_editor_paste = bg_gradient_editor.clone();
        let rect_width_spin_paste = rect_width_spin.clone();
        let rect_height_spin_paste = rect_height_spin.clone();
        let corner_radius_spin_paste = corner_radius_spin.clone();
        let padding_spin_paste = padding_spin.clone();
        let segment_count_spin_paste = segment_count_spin.clone();
        let segment_spacing_spin_paste = segment_spacing_spin.clone();
        let segment_width_spin_paste = segment_width_spin.clone();
        let segment_height_spin_paste = segment_height_spin.clone();
        let taper_style_dropdown_paste = taper_style_dropdown.clone();
        let taper_amount_spin_paste = taper_amount_spin.clone();
        let taper_alignment_dropdown_paste = taper_alignment_dropdown.clone();
        let border_check_paste = border_check.clone();
        let border_width_spin_paste = border_width_spin.clone();
        let border_color_widget_paste = border_color_widget.clone();
        let text_widget_paste = text_config_widget.clone();
        let theme_for_paste = theme.clone();

        paste_btn.connect_clicked(move |_| {
            let pasted = if let Ok(clipboard) = crate::ui::clipboard::CLIPBOARD.lock() {
                clipboard.paste_bar_display()
            } else {
                None
            };

            if let Some(cfg) = pasted {
                // Update stored config
                *config_for_paste.borrow_mut() = cfg.clone();

                // Update UI elements
                style_dropdown_paste.set_selected(match cfg.style {
                    BarStyle::Full => 0,
                    BarStyle::Rectangle => 1,
                    BarStyle::Segmented => 2,
                });
                orientation_dropdown_paste.set_selected(match cfg.orientation {
                    BarOrientation::Horizontal => 0,
                    BarOrientation::Vertical => 1,
                });
                direction_dropdown_paste.set_selected(match cfg.fill_direction {
                    BarFillDirection::LeftToRight => 0,
                    BarFillDirection::RightToLeft => 1,
                    BarFillDirection::BottomToTop => 2,
                    BarFillDirection::TopToBottom => 3,
                });

                // Foreground
                let thm = theme_for_paste.borrow();
                match &cfg.foreground {
                    BarFillType::Solid { color } => {
                        fg_solid_radio_paste.set_active(true);
                        fg_color_widget_paste.set_source(color.clone());
                    }
                    BarFillType::Gradient { stops, .. } => {
                        fg_gradient_radio_paste.set_active(true);
                        fg_gradient_editor_paste.set_stops(stops.iter().map(|s| s.resolve(&thm)).collect());
                    }
                }

                // Background
                match &cfg.background {
                    BarBackgroundType::Solid { color } => {
                        bg_solid_radio_paste.set_active(true);
                        bg_color_widget_paste.set_source(color.clone());
                    }
                    BarBackgroundType::Gradient { stops, .. } => {
                        bg_gradient_radio_paste.set_active(true);
                        bg_gradient_editor_paste.set_stops(stops.iter().map(|s| s.resolve(&thm)).collect());
                    }
                    BarBackgroundType::Transparent => {
                        bg_transparent_radio_paste.set_active(true);
                    }
                }

                // Rectangle options (values stored as 0.0-1.0, displayed as percentages)
                rect_width_spin_paste.set_value(cfg.rectangle_width * 100.0);
                rect_height_spin_paste.set_value(cfg.rectangle_height * 100.0);
                corner_radius_spin_paste.set_value(cfg.corner_radius);
                padding_spin_paste.set_value(cfg.padding);

                // Segmented options (values stored as 0.0-1.0, displayed as percentages)
                segment_count_spin_paste.set_value(cfg.segment_count as f64);
                segment_spacing_spin_paste.set_value(cfg.segment_spacing);
                segment_width_spin_paste.set_value(cfg.segment_width * 100.0);
                segment_height_spin_paste.set_value(cfg.segment_height * 100.0);

                // Taper options
                taper_style_dropdown_paste.set_selected(match cfg.taper_style {
                    BarTaperStyle::None => 0,
                    BarTaperStyle::Start => 1,
                    BarTaperStyle::End => 2,
                    BarTaperStyle::Both => 3,
                });
                taper_amount_spin_paste.set_value(cfg.taper_amount * 100.0);
                taper_alignment_dropdown_paste.set_selected(match cfg.taper_alignment {
                    BarTaperAlignment::Start => 0,
                    BarTaperAlignment::Center => 1,
                    BarTaperAlignment::End => 2,
                });

                // Border
                border_check_paste.set_active(cfg.border.enabled);
                border_width_spin_paste.set_value(cfg.border.width);
                border_color_widget_paste.set_source(cfg.border.color.clone());

                // Text overlay
                text_widget_paste.set_config(cfg.text_overlay.text_config.clone());

                preview_for_paste.queue_draw();
                if let Some(callback) = on_change_for_paste.borrow().as_ref() {
                    callback();
                }
            }
        });

        Self {
            container,
            config,
            theme,
            preview,
            on_change,
            style_dropdown,
            orientation_dropdown,
            direction_dropdown,
            style_stack,
            fg_solid_radio,
            fg_gradient_radio,
            fg_color_widget,
            fg_gradient_editor,
            bg_solid_radio,
            bg_gradient_radio,
            bg_transparent_radio,
            bg_color_widget,
            bg_gradient_editor,
            border_color_widget,
            rect_width_spin,
            rect_height_spin,
            corner_radius_spin,
            padding_spin,
            segment_count_spin,
            segment_spacing_spin,
            segment_width_spin,
            segment_height_spin,
            segment_corner_radius_spin,
            taper_style_dropdown,
            taper_amount_spin,
            taper_alignment_dropdown,
            border_check,
            border_width_spin,
            animate_check,
            animation_speed_spin,
            text_config_widget: Some(text_config_widget),
        }
    }


    fn setup_fg_handlers(
        config: &Rc<RefCell<BarDisplayConfig>>,
        theme: &Rc<RefCell<ComboThemeConfig>>,
        preview: &DrawingArea,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        solid_radio: CheckButton,
        gradient_radio: CheckButton,
        color_widget: Rc<ThemeColorSelector>,
        gradient_editor: Rc<GradientEditor>,
        copy_paste_box: GtkBox,
        copy_btn: Button,
        paste_btn: Button,
    ) {
        // Toggle visibility
        let color_widget_clone = color_widget.widget().clone();
        let gradient_widget_clone = gradient_editor.widget().clone();
        let copy_paste_box_clone = copy_paste_box.clone();

        solid_radio.connect_toggled(move |check| {
            if check.is_active() {
                color_widget_clone.set_visible(true);
                gradient_widget_clone.set_visible(false);
                copy_paste_box_clone.set_visible(false);
            }
        });

        let color_widget_clone2 = color_widget.widget().clone();
        let gradient_widget_clone2 = gradient_editor.widget().clone();
        let copy_paste_box_clone2 = copy_paste_box.clone();

        gradient_radio.connect_toggled(move |check| {
            if check.is_active() {
                color_widget_clone2.set_visible(false);
                gradient_widget_clone2.set_visible(true);
                copy_paste_box_clone2.set_visible(true);
            }
        });

        // Color widget handler - using ThemeColorSelector
        let config_clone = config.clone();
        let preview_clone = preview.clone();
        let on_change_clone = on_change.clone();
        color_widget.set_on_change(move |color_source| {
            config_clone.borrow_mut().foreground = BarFillType::Solid { color: color_source };
            preview_clone.queue_draw();
            if let Some(callback) = on_change_clone.borrow().as_ref() {
                callback();
            }
        });

        // Gradient editor handler - now preserves theme color references
        let config_clone = config.clone();
        let preview_clone = preview.clone();
        let on_change_clone = on_change.clone();
        let gradient_editor_clone = gradient_editor.clone();

        gradient_editor.set_on_change(move || {
            // Get ColorStopSource directly to preserve theme references
            let stops_source = gradient_editor_clone.get_stops_source();
            let angle = gradient_editor_clone.get_gradient().angle;
            let mut cfg = config_clone.borrow_mut();
            cfg.foreground = BarFillType::Gradient { stops: stops_source, angle };
            drop(cfg);

            preview_clone.queue_draw();

            if let Some(callback) = on_change_clone.borrow().as_ref() {
                callback();
            }
        });

        // Copy gradient button handler
        let config_for_copy = config.clone();
        let theme_for_copy = theme.clone();
        copy_btn.connect_clicked(move |_| {
            use crate::ui::CLIPBOARD;

            let cfg = config_for_copy.borrow();
            let thm = theme_for_copy.borrow();
            if let BarFillType::Gradient { stops, .. } = &cfg.foreground {
                // Resolve to ColorStop for clipboard
                let resolved_stops: Vec<ColorStop> = stops.iter().map(|s| s.resolve(&thm)).collect();
                if let Ok(mut clipboard) = CLIPBOARD.lock() {
                    clipboard.copy_gradient_stops(resolved_stops);
                    log::info!("Bar foreground gradient copied to clipboard");
                }
            }
        });

        // Paste gradient button handler
        let config_for_paste = config.clone();
        let preview_for_paste = preview.clone();
        let on_change_for_paste = on_change.clone();
        let gradient_editor_for_paste = gradient_editor.clone();
        paste_btn.connect_clicked(move |_| {
            use crate::ui::CLIPBOARD;

            if let Ok(clipboard) = CLIPBOARD.lock() {
                if let Some(stops) = clipboard.paste_gradient_stops() {
                    // Get current angle from config or use default
                    let angle = if let BarFillType::Gradient { angle, .. } = config_for_paste.borrow().foreground {
                        angle
                    } else {
                        90.0
                    };

                    // Convert ColorStop to ColorStopSource (as custom colors)
                    let stops_source: Vec<ColorStopSource> = stops.iter()
                        .map(|s| ColorStopSource::custom(s.position, s.color))
                        .collect();

                    let mut cfg = config_for_paste.borrow_mut();
                    cfg.foreground = BarFillType::Gradient { stops: stops_source.clone(), angle };
                    drop(cfg);

                    // Update gradient editor with ColorStopSource
                    gradient_editor_for_paste.set_stops_source(stops_source);

                    preview_for_paste.queue_draw();

                    if let Some(callback) = on_change_for_paste.borrow().as_ref() {
                        callback();
                    }

                    log::info!("Bar foreground gradient pasted from clipboard");
                } else {
                    log::info!("No gradient in clipboard");
                }
            }
        });
    }

    fn setup_bg_handlers(
        config: &Rc<RefCell<BarDisplayConfig>>,
        theme: &Rc<RefCell<ComboThemeConfig>>,
        preview: &DrawingArea,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        solid_radio: CheckButton,
        gradient_radio: CheckButton,
        transparent_radio: CheckButton,
        color_widget: Rc<ThemeColorSelector>,
        gradient_editor: Rc<GradientEditor>,
        copy_paste_box: GtkBox,
        copy_btn: Button,
        paste_btn: Button,
    ) {
        // Toggle visibility
        let color_widget_clone = color_widget.widget().clone();
        let gradient_widget_clone = gradient_editor.widget().clone();
        let config_clone = config.clone();
        let copy_paste_box_clone = copy_paste_box.clone();

        solid_radio.connect_toggled(move |check| {
            if check.is_active() {
                color_widget_clone.set_visible(true);
                gradient_widget_clone.set_visible(false);
                copy_paste_box_clone.set_visible(false);
                let mut cfg = config_clone.borrow_mut();
                if !matches!(cfg.background, BarBackgroundType::Solid { .. }) {
                    cfg.background = BarBackgroundType::Solid {
                        color: ColorSource::custom(Color::new(0.15, 0.15, 0.15, 0.8)),
                    };
                }
            }
        });

        let color_widget_clone2 = color_widget.widget().clone();
        let gradient_widget_clone2 = gradient_editor.widget().clone();
        let config_clone2 = config.clone();
        let copy_paste_box_clone2 = copy_paste_box.clone();

        gradient_radio.connect_toggled(move |check| {
            if check.is_active() {
                color_widget_clone2.set_visible(false);
                gradient_widget_clone2.set_visible(true);
                copy_paste_box_clone2.set_visible(true);
                let mut cfg = config_clone2.borrow_mut();
                if !matches!(cfg.background, BarBackgroundType::Gradient { .. }) {
                    cfg.background = BarBackgroundType::Gradient {
                        stops: vec![
                            ColorStopSource::custom(0.0, Color::new(0.2, 0.2, 0.2, 1.0)),
                            ColorStopSource::custom(1.0, Color::new(0.1, 0.1, 0.1, 1.0)),
                        ],
                        angle: 90.0,
                    };
                }
            }
        });

        let color_widget_clone3 = color_widget.widget().clone();
        let gradient_widget_clone3 = gradient_editor.widget().clone();
        let config_clone3 = config.clone();
        let preview_clone = preview.clone();
        let on_change_clone = on_change.clone();
        let copy_paste_box_clone3 = copy_paste_box.clone();

        transparent_radio.connect_toggled(move |check| {
            if check.is_active() {
                color_widget_clone3.set_visible(false);
                gradient_widget_clone3.set_visible(false);
                copy_paste_box_clone3.set_visible(false);
                let mut cfg = config_clone3.borrow_mut();
                cfg.background = BarBackgroundType::Transparent;
                drop(cfg);

                preview_clone.queue_draw();

                if let Some(callback) = on_change_clone.borrow().as_ref() {
                    callback();
                }
            }
        });

        // Color widget handler - using ThemeColorSelector
        let config_clone = config.clone();
        let preview_clone = preview.clone();
        let on_change_clone = on_change.clone();
        color_widget.set_on_change(move |color_source| {
            config_clone.borrow_mut().background = BarBackgroundType::Solid { color: color_source };
            preview_clone.queue_draw();
            if let Some(callback) = on_change_clone.borrow().as_ref() {
                callback();
            }
        });

        // Gradient editor handler - now preserves theme color references
        let config_clone = config.clone();
        let preview_clone = preview.clone();
        let on_change_clone = on_change.clone();
        let gradient_editor_clone = gradient_editor.clone();

        gradient_editor.set_on_change(move || {
            // Get ColorStopSource directly to preserve theme references
            let stops_source = gradient_editor_clone.get_stops_source();
            let angle = gradient_editor_clone.get_gradient().angle;
            let mut cfg = config_clone.borrow_mut();
            cfg.background = BarBackgroundType::Gradient { stops: stops_source, angle };
            drop(cfg);

            preview_clone.queue_draw();

            if let Some(callback) = on_change_clone.borrow().as_ref() {
                callback();
            }
        });

        // Copy gradient button handler
        let config_for_copy = config.clone();
        let theme_for_copy = theme.clone();
        copy_btn.connect_clicked(move |_| {
            use crate::ui::CLIPBOARD;

            let cfg = config_for_copy.borrow();
            let thm = theme_for_copy.borrow();
            if let BarBackgroundType::Gradient { stops, .. } = &cfg.background {
                // Resolve to ColorStop for clipboard
                let resolved_stops: Vec<ColorStop> = stops.iter().map(|s| s.resolve(&thm)).collect();
                if let Ok(mut clipboard) = CLIPBOARD.lock() {
                    clipboard.copy_gradient_stops(resolved_stops);
                    log::info!("Bar background gradient copied to clipboard");
                }
            }
        });

        // Paste gradient button handler
        let config_for_paste = config.clone();
        let preview_for_paste = preview.clone();
        let on_change_for_paste = on_change.clone();
        let gradient_editor_for_paste = gradient_editor.clone();
        paste_btn.connect_clicked(move |_| {
            use crate::ui::CLIPBOARD;

            if let Ok(clipboard) = CLIPBOARD.lock() {
                if let Some(stops) = clipboard.paste_gradient_stops() {
                    // Get current angle from config or use default
                    let angle = if let BarBackgroundType::Gradient { angle, .. } = config_for_paste.borrow().background {
                        angle
                    } else {
                        90.0
                    };

                    // Convert ColorStop to ColorStopSource (as custom colors)
                    let stops_source: Vec<ColorStopSource> = stops.iter()
                        .map(|s| ColorStopSource::custom(s.position, s.color))
                        .collect();

                    let mut cfg = config_for_paste.borrow_mut();
                    cfg.background = BarBackgroundType::Gradient { stops: stops_source.clone(), angle };
                    drop(cfg);

                    // Update gradient editor with ColorStopSource
                    gradient_editor_for_paste.set_stops_source(stops_source);

                    preview_for_paste.queue_draw();

                    if let Some(callback) = on_change_for_paste.borrow().as_ref() {
                        callback();
                    }

                    log::info!("Bar background gradient pasted from clipboard");
                } else {
                    log::info!("No gradient in clipboard");
                }
            }
        });
    }

    fn create_rectangle_options(
        config: &Rc<RefCell<BarDisplayConfig>>,
        preview: &DrawingArea,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
    ) -> (GtkBox, SpinButton, SpinButton, SpinButton, SpinButton) {
        let page = GtkBox::new(Orientation::Vertical, 12);

        // Width
        let width_box = GtkBox::new(Orientation::Horizontal, 6);
        width_box.append(&Label::new(Some("Width (%):")));
        let width_spin = SpinButton::with_range(10.0, 100.0, 1.0);
        width_spin.set_value(80.0);
        width_spin.set_hexpand(true);
        width_box.append(&width_spin);
        page.append(&width_box);

        // Height
        let height_box = GtkBox::new(Orientation::Horizontal, 6);
        height_box.append(&Label::new(Some("Height (%):")));
        let height_spin = SpinButton::with_range(10.0, 100.0, 1.0);
        height_spin.set_value(60.0);
        height_spin.set_hexpand(true);
        height_box.append(&height_spin);
        page.append(&height_box);

        // Corner radius
        let radius_box = GtkBox::new(Orientation::Horizontal, 6);
        radius_box.append(&Label::new(Some("Corner Radius:")));
        let radius_spin = SpinButton::with_range(0.0, 50.0, 1.0);
        radius_spin.set_value(5.0);
        radius_spin.set_hexpand(true);
        radius_box.append(&radius_spin);
        page.append(&radius_box);

        // Padding
        let padding_box = GtkBox::new(Orientation::Horizontal, 6);
        padding_box.append(&Label::new(Some("Padding:")));
        let padding_spin = SpinButton::with_range(0.0, 50.0, 1.0);
        padding_spin.set_value(4.0);
        padding_spin.set_hexpand(true);
        padding_box.append(&padding_spin);
        page.append(&padding_box);

        // Handlers
        let config_clone = config.clone();
        let preview_clone = preview.clone();
        let on_change_clone = on_change.clone();
        width_spin.connect_value_changed(move |spin| {
            let mut cfg = config_clone.borrow_mut();
            cfg.rectangle_width = spin.value() / 100.0;
            drop(cfg);
            preview_clone.queue_draw();
            if let Some(callback) = on_change_clone.borrow().as_ref() {
                callback();
            }
        });

        let config_clone = config.clone();
        let preview_clone = preview.clone();
        let on_change_clone = on_change.clone();
        height_spin.connect_value_changed(move |spin| {
            let mut cfg = config_clone.borrow_mut();
            cfg.rectangle_height = spin.value() / 100.0;
            drop(cfg);
            preview_clone.queue_draw();
            if let Some(callback) = on_change_clone.borrow().as_ref() {
                callback();
            }
        });

        let config_clone = config.clone();
        let preview_clone = preview.clone();
        let on_change_clone = on_change.clone();
        radius_spin.connect_value_changed(move |spin| {
            let mut cfg = config_clone.borrow_mut();
            cfg.corner_radius = spin.value();
            drop(cfg);
            preview_clone.queue_draw();
            if let Some(callback) = on_change_clone.borrow().as_ref() {
                callback();
            }
        });

        let config_clone = config.clone();
        let preview_clone = preview.clone();
        let on_change_clone = on_change.clone();
        padding_spin.connect_value_changed(move |spin| {
            let mut cfg = config_clone.borrow_mut();
            cfg.padding = spin.value();
            drop(cfg);
            preview_clone.queue_draw();
            if let Some(callback) = on_change_clone.borrow().as_ref() {
                callback();
            }
        });

        (page, width_spin, height_spin, radius_spin, padding_spin)
    }

    fn create_segmented_options(
        config: &Rc<RefCell<BarDisplayConfig>>,
        preview: &DrawingArea,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
    ) -> (GtkBox, SpinButton, SpinButton, SpinButton, SpinButton, SpinButton) {
        let page = GtkBox::new(Orientation::Vertical, 12);

        // Segment count
        let count_box = GtkBox::new(Orientation::Horizontal, 6);
        count_box.append(&Label::new(Some("Segments:")));
        let count_spin = SpinButton::with_range(2.0, 100.0, 1.0);
        count_spin.set_value(10.0);
        count_spin.set_hexpand(true);
        count_box.append(&count_spin);
        page.append(&count_box);

        // Segment spacing
        let spacing_box = GtkBox::new(Orientation::Horizontal, 6);
        spacing_box.append(&Label::new(Some("Spacing:")));
        let spacing_spin = SpinButton::with_range(0.0, 20.0, 0.5);
        spacing_spin.set_value(2.0);
        spacing_spin.set_hexpand(true);
        spacing_box.append(&spacing_spin);
        page.append(&spacing_box);

        // Width
        let width_box = GtkBox::new(Orientation::Horizontal, 6);
        width_box.append(&Label::new(Some("Width (%):")));
        let width_spin = SpinButton::with_range(10.0, 100.0, 1.0);
        width_spin.set_value(90.0);
        width_spin.set_hexpand(true);
        width_box.append(&width_spin);
        page.append(&width_box);

        // Height
        let height_box = GtkBox::new(Orientation::Horizontal, 6);
        height_box.append(&Label::new(Some("Height (%):")));
        let height_spin = SpinButton::with_range(10.0, 100.0, 1.0);
        height_spin.set_value(80.0);
        height_spin.set_hexpand(true);
        height_box.append(&height_spin);
        page.append(&height_box);

        // Corner radius
        let corner_radius_box = GtkBox::new(Orientation::Horizontal, 6);
        corner_radius_box.append(&Label::new(Some("Corner Radius:")));
        let corner_radius_spin = SpinButton::with_range(0.0, 50.0, 1.0);
        corner_radius_spin.set_value(5.0);
        corner_radius_spin.set_hexpand(true);
        corner_radius_box.append(&corner_radius_spin);
        page.append(&corner_radius_box);

        // Handlers
        let config_clone = config.clone();
        let preview_clone = preview.clone();
        let on_change_clone = on_change.clone();
        count_spin.connect_value_changed(move |spin| {
            let mut cfg = config_clone.borrow_mut();
            cfg.segment_count = spin.value() as u32;
            drop(cfg);
            preview_clone.queue_draw();
            if let Some(callback) = on_change_clone.borrow().as_ref() {
                callback();
            }
        });

        let config_clone = config.clone();
        let preview_clone = preview.clone();
        let on_change_clone = on_change.clone();
        spacing_spin.connect_value_changed(move |spin| {
            let mut cfg = config_clone.borrow_mut();
            cfg.segment_spacing = spin.value();
            drop(cfg);
            preview_clone.queue_draw();
            if let Some(callback) = on_change_clone.borrow().as_ref() {
                callback();
            }
        });

        let config_clone = config.clone();
        let preview_clone = preview.clone();
        let on_change_clone = on_change.clone();
        width_spin.connect_value_changed(move |spin| {
            let mut cfg = config_clone.borrow_mut();
            cfg.segment_width = spin.value() / 100.0;
            drop(cfg);
            preview_clone.queue_draw();
            if let Some(callback) = on_change_clone.borrow().as_ref() {
                callback();
            }
        });

        let config_clone = config.clone();
        let preview_clone = preview.clone();
        let on_change_clone = on_change.clone();
        height_spin.connect_value_changed(move |spin| {
            let mut cfg = config_clone.borrow_mut();
            cfg.segment_height = spin.value() / 100.0;
            drop(cfg);
            preview_clone.queue_draw();
            if let Some(callback) = on_change_clone.borrow().as_ref() {
                callback();
            }
        });

        let config_clone = config.clone();
        let preview_clone = preview.clone();
        let on_change_clone = on_change.clone();
        corner_radius_spin.connect_value_changed(move |spin| {
            let mut cfg = config_clone.borrow_mut();
            cfg.corner_radius = spin.value();
            drop(cfg);
            preview_clone.queue_draw();
            if let Some(callback) = on_change_clone.borrow().as_ref() {
                callback();
            }
        });

        (page, count_spin, spacing_spin, width_spin, height_spin, corner_radius_spin)
    }

    pub fn widget(&self) -> &GtkBox {
        &self.container
    }

    pub fn set_config(&self, new_config: BarDisplayConfig) {
        *self.config.borrow_mut() = new_config.clone();

        // Update bar style
        let style_index = match new_config.style {
            BarStyle::Full => 0,
            BarStyle::Rectangle => 1,
            BarStyle::Segmented => 2,
        };
        self.style_dropdown.set_selected(style_index);

        // Update style stack visibility
        let page_name = match new_config.style {
            BarStyle::Full => "full",
            BarStyle::Rectangle => "rectangle",
            BarStyle::Segmented => "segmented",
        };
        self.style_stack.set_visible_child_name(page_name);

        // Update orientation
        let orientation_index = match new_config.orientation {
            BarOrientation::Horizontal => 0,
            BarOrientation::Vertical => 1,
        };
        self.orientation_dropdown.set_selected(orientation_index);

        // Update fill direction
        let direction_index = match new_config.fill_direction {
            BarFillDirection::LeftToRight => 0,
            BarFillDirection::RightToLeft => 1,
            BarFillDirection::BottomToTop => 2,
            BarFillDirection::TopToBottom => 3,
        };
        self.direction_dropdown.set_selected(direction_index);

        // Update rectangle options
        self.rect_width_spin.set_value(new_config.rectangle_width * 100.0);
        self.rect_height_spin.set_value(new_config.rectangle_height * 100.0);
        self.corner_radius_spin.set_value(new_config.corner_radius);
        self.padding_spin.set_value(new_config.padding);

        // Update segmented options
        self.segment_count_spin.set_value(new_config.segment_count as f64);
        self.segment_spacing_spin.set_value(new_config.segment_spacing);
        self.segment_width_spin.set_value(new_config.segment_width * 100.0);
        self.segment_height_spin.set_value(new_config.segment_height * 100.0);
        self.segment_corner_radius_spin.set_value(new_config.corner_radius);

        // Update taper options
        let taper_index = match new_config.taper_style {
            BarTaperStyle::None => 0,
            BarTaperStyle::Start => 1,
            BarTaperStyle::End => 2,
            BarTaperStyle::Both => 3,
        };
        self.taper_style_dropdown.set_selected(taper_index);
        self.taper_amount_spin.set_value(new_config.taper_amount * 100.0);
        let align_index = match new_config.taper_alignment {
            BarTaperAlignment::Start => 0,
            BarTaperAlignment::Center => 1,
            BarTaperAlignment::End => 2,
        };
        self.taper_alignment_dropdown.set_selected(align_index);

        // Update border
        self.border_check.set_active(new_config.border.enabled);
        self.border_width_spin.set_value(new_config.border.width);

        // Update animation
        self.animate_check.set_active(new_config.smooth_animation);
        self.animation_speed_spin.set_value(new_config.animation_speed);

        // Update foreground UI - preserving theme color references
        match &new_config.foreground {
            BarFillType::Solid { color } => {
                self.fg_solid_radio.set_active(true);
                self.fg_color_widget.widget().set_visible(true);
                self.fg_color_widget.set_source(color.clone());
                self.fg_gradient_editor.widget().set_visible(false);
            }
            BarFillType::Gradient { stops, angle } => {
                self.fg_gradient_radio.set_active(true);
                self.fg_color_widget.widget().set_visible(false);
                self.fg_gradient_editor.widget().set_visible(true);
                // Load gradient with ColorStopSource to preserve theme references
                self.fg_gradient_editor.set_gradient_source(*angle, stops.clone());
            }
        }

        // Update background UI - preserving theme color references
        match &new_config.background {
            BarBackgroundType::Solid { color } => {
                self.bg_solid_radio.set_active(true);
                self.bg_color_widget.widget().set_visible(true);
                self.bg_color_widget.set_source(color.clone());
                self.bg_gradient_editor.widget().set_visible(false);
            }
            BarBackgroundType::Gradient { stops, angle } => {
                self.bg_gradient_radio.set_active(true);
                self.bg_color_widget.widget().set_visible(false);
                self.bg_gradient_editor.widget().set_visible(true);
                // Load gradient with ColorStopSource to preserve theme references
                self.bg_gradient_editor.set_gradient_source(*angle, stops.clone());
            }
            BarBackgroundType::Transparent => {
                self.bg_transparent_radio.set_active(true);
                self.bg_color_widget.widget().set_visible(false);
                self.bg_gradient_editor.widget().set_visible(false);
            }
        }

        // Update border color
        self.border_color_widget.set_source(new_config.border.color.clone());

        // Update text config widget
        if let Some(ref text_widget) = self.text_config_widget {
            text_widget.set_config(new_config.text_overlay.text_config);
        }

        self.preview.queue_draw();
    }

    pub fn get_config(&self) -> BarDisplayConfig {
        let mut config = self.config.borrow().clone();

        // Update text config from widget
        if let Some(ref text_widget) = self.text_config_widget {
            config.text_overlay.text_config = text_widget.get_config();
        }

        // Include current theme in config
        config.theme = self.theme.borrow().clone();

        config
    }

    /// Update the theme configuration and refresh the preview
    pub fn set_theme(&self, theme: ComboThemeConfig) {
        *self.theme.borrow_mut() = theme.clone();
        // Update all ThemeColorSelector widgets with new theme
        self.fg_color_widget.set_theme_config(theme.clone());
        self.bg_color_widget.set_theme_config(theme.clone());
        self.border_color_widget.set_theme_config(theme.clone());
        // Update gradient editors with new theme
        self.fg_gradient_editor.set_theme_config(theme.clone());
        self.bg_gradient_editor.set_theme_config(theme.clone());
        // Update text config widget with new theme (for font selectors)
        if let Some(ref text_widget) = self.text_config_widget {
            text_widget.set_theme(theme);
        }
        self.preview.queue_draw();
    }

    pub fn set_on_change<F: Fn() + 'static>(&self, callback: F) {
        *self.on_change.borrow_mut() = Some(Box::new(callback));
    }
}

impl Default for BarConfigWidget {
    fn default() -> Self {
        Self::new(vec![])
    }
}
