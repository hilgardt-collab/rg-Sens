//! Core bars configuration widget with tabbed interface

use gtk4::prelude::*;
use gtk4::{
    Adjustment, Box as GtkBox, Button, CheckButton, DrawingArea, DropDown, Entry, Label,
    Notebook, Orientation, Scale, SpinButton, StringList,
};
use std::cell::RefCell;
use std::rc::Rc;

use crate::core::FieldMetadata;
use crate::ui::background::Color;
use crate::ui::bar_display::{BarBackgroundType, BarFillDirection, BarFillType, BarOrientation, BarStyle};
use crate::ui::render_utils::render_checkerboard;
use crate::ui::clipboard::CLIPBOARD;
use crate::ui::core_bars_display::{CoreBarsConfig, LabelPosition, render_core_bars};
use crate::ui::shared_font_dialog::show_font_dialog;
use crate::ui::theme::{ColorSource, ColorStopSource, ComboThemeConfig};
use crate::ui::theme_color_selector::ThemeColorSelector;
use crate::ui::GradientEditor;
use crate::ui::TextOverlayConfigWidget;

/// Core bars configuration widget
pub struct CoreBarsConfigWidget {
    container: GtkBox,
    config: Rc<RefCell<CoreBarsConfig>>,
    theme: Rc<RefCell<ComboThemeConfig>>,
    preview: DrawingArea,
    on_change: Rc<RefCell<Option<Box<dyn Fn()>>>>,

    // Core selection
    start_core_spin: SpinButton,
    end_core_spin: SpinButton,

    // Padding
    padding_top_spin: SpinButton,
    padding_bottom_spin: SpinButton,
    padding_left_spin: SpinButton,
    padding_right_spin: SpinButton,

    // Bar style
    style_dropdown: DropDown,
    orientation_dropdown: DropDown,
    fill_direction_dropdown: DropDown,
    corner_radius_spin: SpinButton,
    bar_spacing_spin: SpinButton,

    // Segmented options
    segment_count_spin: SpinButton,
    segment_spacing_spin: SpinButton,

    // Colors
    fg_solid_radio: CheckButton,
    fg_gradient_radio: CheckButton,
    fg_color_widget: Rc<ThemeColorSelector>,
    fg_gradient_editor: Rc<GradientEditor>,
    bg_solid_radio: CheckButton,
    bg_gradient_radio: CheckButton,
    bg_transparent_radio: CheckButton,
    bg_color_widget: Rc<ThemeColorSelector>,
    bg_gradient_editor: Rc<GradientEditor>,

    // Border
    border_check: CheckButton,
    border_width_spin: SpinButton,
    border_color_widget: Rc<ThemeColorSelector>,

    // Labels
    show_labels_check: CheckButton,
    label_prefix_entry: Entry,
    label_position_dropdown: DropDown,
    label_font_button: Button,
    label_size_spin: SpinButton,
    label_color_widget: Rc<ThemeColorSelector>,
    label_bold_check: CheckButton,

    // Animation
    animate_check: CheckButton,
    animation_speed_scale: Scale,

    // Gradient spans bars option
    gradient_spans_bars_check: CheckButton,

    // Text overlay
    text_overlay_widget: Rc<TextOverlayConfigWidget>,
}

impl CoreBarsConfigWidget {
    pub fn new() -> Self {
        Self::with_fields(&[])
    }

    /// Create widget with available fields for text overlay
    pub fn with_fields(available_fields: &[FieldMetadata]) -> Self {
        // If no fields provided, create default fields useful for combo panel core bars
        let fields: Vec<FieldMetadata> = if available_fields.is_empty() {
            Self::default_combo_fields()
        } else {
            available_fields.to_vec()
        };

        let container = GtkBox::new(Orientation::Vertical, 8);
        container.set_margin_start(8);
        container.set_margin_end(8);
        container.set_margin_top(8);
        container.set_margin_bottom(8);

        let config = Rc::new(RefCell::new(CoreBarsConfig::default()));
        let theme = Rc::new(RefCell::new(ComboThemeConfig::default()));
        let on_change: Rc<RefCell<Option<Box<dyn Fn()>>>> = Rc::new(RefCell::new(None));

        // Create notebook for tabs
        let notebook = Notebook::new();
        notebook.set_vexpand(true);

        // === Tab 1: Cores ===
        let (cores_page, start_core_spin, end_core_spin, padding_top_spin, padding_bottom_spin, padding_left_spin, padding_right_spin) =
            Self::create_cores_page(&config, &on_change);
        notebook.append_page(&cores_page, Some(&Label::new(Some("Cores"))));

        // === Tab 2: Bar Style ===
        let (style_page, style_dropdown, orientation_dropdown, fill_direction_dropdown,
             corner_radius_spin, bar_spacing_spin, segment_count_spin, segment_spacing_spin) =
            Self::create_style_page(&config, &on_change);
        notebook.append_page(&style_page, Some(&Label::new(Some("Style"))));

        // === Tab 3: Colors ===
        let (colors_page, fg_solid_radio, fg_gradient_radio, fg_color_widget, fg_gradient_editor,
             bg_solid_radio, bg_gradient_radio, bg_transparent_radio, bg_color_widget, bg_gradient_editor,
             border_check, border_width_spin, border_color_widget, gradient_spans_bars_check) =
            Self::create_colors_page(&config, &theme, &on_change);
        notebook.append_page(&colors_page, Some(&Label::new(Some("Colors"))));

        // === Tab 4: Labels ===
        let (labels_page, show_labels_check, label_prefix_entry, label_position_dropdown,
             label_font_button, label_size_spin, label_color_widget, label_bold_check) =
            Self::create_labels_page(&config, &theme, &on_change);
        notebook.append_page(&labels_page, Some(&Label::new(Some("Labels"))));

        // === Tab 5: Animation ===
        let (animation_page, animate_check, animation_speed_scale) =
            Self::create_animation_page(&config, &on_change);
        notebook.append_page(&animation_page, Some(&Label::new(Some("Animation"))));

        // Create preview early so it can be used in text overlay tab
        let preview = DrawingArea::new();

        // === Tab 6: Text Overlay ===
        // Use TextOverlayConfigWidget for consistent text overlay handling
        let text_overlay_widget = Rc::new(TextOverlayConfigWidget::new(fields.clone()));
        text_overlay_widget.set_config(config.borrow().text_overlay.clone());
        text_overlay_widget.set_theme(theme.borrow().clone());
        text_overlay_widget.widget().set_vexpand(true);

        // Connect text overlay change
        let config_for_text = config.clone();
        let preview_for_text = preview.clone();
        let on_change_for_text = on_change.clone();
        let text_overlay_for_change = text_overlay_widget.clone();
        text_overlay_widget.set_on_change(move || {
            config_for_text.borrow_mut().text_overlay = text_overlay_for_change.get_config();
            preview_for_text.queue_draw();
            if let Some(callback) = on_change_for_text.borrow().as_ref() {
                callback();
            }
        });

        notebook.append_page(text_overlay_widget.widget(), Some(&Label::new(Some("Text"))));

        // Preview configuration
        preview.set_content_height(150);
        preview.set_hexpand(true);
        preview.set_vexpand(false);

        let config_for_preview = config.clone();
        let theme_for_preview = theme.clone();
        preview.set_draw_func(move |_, cr, width, height| {
            // Checkerboard background
            render_checkerboard(cr, width as f64, height as f64);

            let cfg = config_for_preview.borrow();
            let thm = theme_for_preview.borrow();
            // Sample values for preview
            let sample_values: Vec<f64> = (0..cfg.core_count().min(8))
                .map(|i| 0.2 + (i as f64 * 0.1))
                .collect();

            if !sample_values.is_empty() {
                let _ = render_core_bars(cr, &cfg, &thm, &sample_values, width as f64, height as f64);
            }
        });

        container.append(&notebook);
        container.append(&preview);

        Self {
            container,
            config,
            theme,
            preview,
            on_change,
            start_core_spin,
            end_core_spin,
            padding_top_spin,
            padding_bottom_spin,
            padding_left_spin,
            padding_right_spin,
            style_dropdown,
            orientation_dropdown,
            fill_direction_dropdown,
            corner_radius_spin,
            bar_spacing_spin,
            segment_count_spin,
            segment_spacing_spin,
            fg_solid_radio,
            fg_gradient_radio,
            fg_color_widget,
            fg_gradient_editor,
            bg_solid_radio,
            bg_gradient_radio,
            bg_transparent_radio,
            bg_color_widget,
            bg_gradient_editor,
            border_check,
            border_width_spin,
            border_color_widget,
            show_labels_check,
            label_prefix_entry,
            label_position_dropdown,
            label_font_button,
            label_size_spin,
            label_color_widget,
            label_bold_check,
            animate_check,
            animation_speed_scale,
            gradient_spans_bars_check,
            text_overlay_widget,
        }
    }

    fn create_cores_page(
        config: &Rc<RefCell<CoreBarsConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
    ) -> (GtkBox, SpinButton, SpinButton, SpinButton, SpinButton, SpinButton, SpinButton) {
        let page = GtkBox::new(Orientation::Vertical, 8);
        page.set_margin_start(8);
        page.set_margin_end(8);
        page.set_margin_top(8);
        page.set_margin_bottom(8);

        // Start core
        let start_row = GtkBox::new(Orientation::Horizontal, 8);
        start_row.append(&Label::new(Some("Start Core:")));
        let start_adj = Adjustment::new(0.0, 0.0, 127.0, 1.0, 4.0, 0.0);
        let start_core_spin = SpinButton::new(Some(&start_adj), 1.0, 0);
        start_core_spin.set_hexpand(true);
        start_row.append(&start_core_spin);
        page.append(&start_row);

        // End core
        let end_row = GtkBox::new(Orientation::Horizontal, 8);
        end_row.append(&Label::new(Some("End Core:")));
        let end_adj = Adjustment::new(15.0, 0.0, 127.0, 1.0, 4.0, 0.0);
        let end_core_spin = SpinButton::new(Some(&end_adj), 1.0, 0);
        end_core_spin.set_hexpand(true);
        end_row.append(&end_core_spin);
        page.append(&end_row);

        // Info label
        let info_label = Label::new(Some("Select which CPU cores to display (0-based index)"));
        info_label.set_xalign(0.0);
        info_label.add_css_class("dim-label");
        page.append(&info_label);

        // Padding section
        let padding_label = Label::new(Some("Padding"));
        padding_label.set_xalign(0.0);
        padding_label.add_css_class("heading");
        padding_label.set_margin_top(12);
        page.append(&padding_label);

        // Top/Bottom padding row
        let tb_row = GtkBox::new(Orientation::Horizontal, 8);
        tb_row.append(&Label::new(Some("Top:")));
        let padding_top_adj = Adjustment::new(0.0, 0.0, 100.0, 1.0, 5.0, 0.0);
        let padding_top_spin = SpinButton::new(Some(&padding_top_adj), 1.0, 0);
        padding_top_spin.set_hexpand(true);
        tb_row.append(&padding_top_spin);
        tb_row.append(&Label::new(Some("Bottom:")));
        let padding_bottom_adj = Adjustment::new(0.0, 0.0, 100.0, 1.0, 5.0, 0.0);
        let padding_bottom_spin = SpinButton::new(Some(&padding_bottom_adj), 1.0, 0);
        padding_bottom_spin.set_hexpand(true);
        tb_row.append(&padding_bottom_spin);
        page.append(&tb_row);

        // Left/Right padding row
        let lr_row = GtkBox::new(Orientation::Horizontal, 8);
        lr_row.append(&Label::new(Some("Left:")));
        let padding_left_adj = Adjustment::new(0.0, 0.0, 100.0, 1.0, 5.0, 0.0);
        let padding_left_spin = SpinButton::new(Some(&padding_left_adj), 1.0, 0);
        padding_left_spin.set_hexpand(true);
        lr_row.append(&padding_left_spin);
        lr_row.append(&Label::new(Some("Right:")));
        let padding_right_adj = Adjustment::new(0.0, 0.0, 100.0, 1.0, 5.0, 0.0);
        let padding_right_spin = SpinButton::new(Some(&padding_right_adj), 1.0, 0);
        padding_right_spin.set_hexpand(true);
        lr_row.append(&padding_right_spin);
        page.append(&lr_row);

        // Connect signals
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        start_core_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().start_core = spin.value() as usize;
            if let Some(ref cb) = *on_change_clone.borrow() {
                cb();
            }
        });

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        end_core_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().end_core = spin.value() as usize;
            if let Some(ref cb) = *on_change_clone.borrow() {
                cb();
            }
        });

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        padding_top_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().padding_top = spin.value();
            if let Some(ref cb) = *on_change_clone.borrow() {
                cb();
            }
        });

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        padding_bottom_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().padding_bottom = spin.value();
            if let Some(ref cb) = *on_change_clone.borrow() {
                cb();
            }
        });

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        padding_left_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().padding_left = spin.value();
            if let Some(ref cb) = *on_change_clone.borrow() {
                cb();
            }
        });

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        padding_right_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().padding_right = spin.value();
            if let Some(ref cb) = *on_change_clone.borrow() {
                cb();
            }
        });

        (page, start_core_spin, end_core_spin, padding_top_spin, padding_bottom_spin, padding_left_spin, padding_right_spin)
    }

    fn create_style_page(
        config: &Rc<RefCell<CoreBarsConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
    ) -> (GtkBox, DropDown, DropDown, DropDown, SpinButton, SpinButton, SpinButton, SpinButton) {
        let page = GtkBox::new(Orientation::Vertical, 8);
        page.set_margin_start(8);
        page.set_margin_end(8);
        page.set_margin_top(8);
        page.set_margin_bottom(8);

        // Style dropdown
        let style_row = GtkBox::new(Orientation::Horizontal, 8);
        style_row.append(&Label::new(Some("Style:")));
        let style_options = StringList::new(&["Full", "Rectangle", "Segmented"]);
        let style_dropdown = DropDown::new(Some(style_options), Option::<gtk4::Expression>::None);
        style_dropdown.set_selected(0);
        style_dropdown.set_hexpand(true);
        style_row.append(&style_dropdown);
        page.append(&style_row);

        // Orientation dropdown
        let orient_row = GtkBox::new(Orientation::Horizontal, 8);
        orient_row.append(&Label::new(Some("Orientation:")));
        let orient_options = StringList::new(&["Horizontal", "Vertical"]);
        let orientation_dropdown = DropDown::new(Some(orient_options), Option::<gtk4::Expression>::None);
        orientation_dropdown.set_selected(0);
        orientation_dropdown.set_hexpand(true);
        orient_row.append(&orientation_dropdown);
        page.append(&orient_row);

        // Fill direction dropdown (will be populated based on orientation)
        let direction_row = GtkBox::new(Orientation::Horizontal, 8);
        direction_row.append(&Label::new(Some("Fill Direction:")));
        let direction_options = StringList::new(&["Left to Right", "Right to Left"]); // Initial horizontal options
        let fill_direction_dropdown = DropDown::new(Some(direction_options), Option::<gtk4::Expression>::None);
        fill_direction_dropdown.set_selected(0);
        fill_direction_dropdown.set_hexpand(true);
        direction_row.append(&fill_direction_dropdown);
        page.append(&direction_row);

        // Corner radius
        let radius_row = GtkBox::new(Orientation::Horizontal, 8);
        radius_row.append(&Label::new(Some("Corner Radius:")));
        let radius_adj = Adjustment::new(3.0, 0.0, 20.0, 1.0, 2.0, 0.0);
        let corner_radius_spin = SpinButton::new(Some(&radius_adj), 1.0, 0);
        corner_radius_spin.set_hexpand(true);
        radius_row.append(&corner_radius_spin);
        page.append(&radius_row);

        // Bar spacing
        let spacing_row = GtkBox::new(Orientation::Horizontal, 8);
        spacing_row.append(&Label::new(Some("Bar Spacing:")));
        let spacing_adj = Adjustment::new(4.0, 0.0, 20.0, 1.0, 2.0, 0.0);
        let bar_spacing_spin = SpinButton::new(Some(&spacing_adj), 1.0, 0);
        bar_spacing_spin.set_hexpand(true);
        spacing_row.append(&bar_spacing_spin);
        page.append(&spacing_row);

        // Segmented options
        let seg_label = Label::new(Some("Segmented Options:"));
        seg_label.set_xalign(0.0);
        page.append(&seg_label);

        let seg_count_row = GtkBox::new(Orientation::Horizontal, 8);
        seg_count_row.append(&Label::new(Some("Segment Count:")));
        let seg_count_adj = Adjustment::new(10.0, 2.0, 50.0, 1.0, 5.0, 0.0);
        let segment_count_spin = SpinButton::new(Some(&seg_count_adj), 1.0, 0);
        segment_count_spin.set_hexpand(true);
        seg_count_row.append(&segment_count_spin);
        page.append(&seg_count_row);

        let seg_spacing_row = GtkBox::new(Orientation::Horizontal, 8);
        seg_spacing_row.append(&Label::new(Some("Segment Spacing:")));
        let seg_spacing_adj = Adjustment::new(1.0, 0.0, 10.0, 0.5, 1.0, 0.0);
        let segment_spacing_spin = SpinButton::new(Some(&seg_spacing_adj), 0.5, 1);
        segment_spacing_spin.set_hexpand(true);
        seg_spacing_row.append(&segment_spacing_spin);
        page.append(&seg_spacing_row);

        // Connect signals
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        style_dropdown.connect_selected_notify(move |dd| {
            let selected = dd.selected();
            if selected == gtk4::INVALID_LIST_POSITION {
                return;
            }
            let style = match selected {
                0 => BarStyle::Full,
                1 => BarStyle::Rectangle,
                _ => BarStyle::Segmented,
            };
            config_clone.borrow_mut().bar_style = style;
            if let Some(ref cb) = *on_change_clone.borrow() {
                cb();
            }
        });

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let fill_direction_dropdown_clone = fill_direction_dropdown.clone();
        orientation_dropdown.connect_selected_notify(move |dd| {
            let selected = dd.selected();
            if selected == gtk4::INVALID_LIST_POSITION {
                return;
            }
            let orientation = match selected {
                0 => BarOrientation::Horizontal,
                _ => BarOrientation::Vertical,
            };

            // Update fill direction dropdown options based on orientation
            let new_options = if matches!(orientation, BarOrientation::Horizontal) {
                StringList::new(&["Left to Right", "Right to Left"])
            } else {
                StringList::new(&["Bottom to Top", "Top to Bottom"])
            };
            fill_direction_dropdown_clone.set_model(Some(&new_options));

            // Set appropriate default
            fill_direction_dropdown_clone.set_selected(0);

            config_clone.borrow_mut().orientation = orientation;
            config_clone.borrow_mut().fill_direction = if matches!(orientation, BarOrientation::Horizontal) {
                BarFillDirection::LeftToRight
            } else {
                BarFillDirection::BottomToTop
            };

            if let Some(ref cb) = *on_change_clone.borrow() {
                cb();
            }
        });

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let orientation_dropdown_clone = orientation_dropdown.clone();
        fill_direction_dropdown.connect_selected_notify(move |dd| {
            let selected = dd.selected();
            if selected == gtk4::INVALID_LIST_POSITION {
                return;
            }
            let orient_selected = orientation_dropdown_clone.selected();
            if orient_selected == gtk4::INVALID_LIST_POSITION {
                return;
            }
            let is_horizontal = orient_selected == 0;
            let direction = if is_horizontal {
                match selected {
                    0 => BarFillDirection::LeftToRight,
                    _ => BarFillDirection::RightToLeft,
                }
            } else {
                match selected {
                    0 => BarFillDirection::BottomToTop,
                    _ => BarFillDirection::TopToBottom,
                }
            };
            config_clone.borrow_mut().fill_direction = direction;
            if let Some(ref cb) = *on_change_clone.borrow() {
                cb();
            }
        });

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        corner_radius_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().corner_radius = spin.value();
            if let Some(ref cb) = *on_change_clone.borrow() {
                cb();
            }
        });

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        bar_spacing_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().bar_spacing = spin.value();
            if let Some(ref cb) = *on_change_clone.borrow() {
                cb();
            }
        });

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        segment_count_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().segment_count = spin.value() as u32;
            if let Some(ref cb) = *on_change_clone.borrow() {
                cb();
            }
        });

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        segment_spacing_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().segment_spacing = spin.value();
            if let Some(ref cb) = *on_change_clone.borrow() {
                cb();
            }
        });

        (page, style_dropdown, orientation_dropdown, fill_direction_dropdown, corner_radius_spin,
         bar_spacing_spin, segment_count_spin, segment_spacing_spin)
    }

    fn create_colors_page(
        config: &Rc<RefCell<CoreBarsConfig>>,
        theme: &Rc<RefCell<ComboThemeConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
    ) -> (GtkBox, CheckButton, CheckButton, Rc<ThemeColorSelector>, Rc<GradientEditor>,
          CheckButton, CheckButton, CheckButton, Rc<ThemeColorSelector>, Rc<GradientEditor>,
          CheckButton, SpinButton, Rc<ThemeColorSelector>, CheckButton) {
        let page = GtkBox::new(Orientation::Vertical, 8);
        page.set_margin_start(8);
        page.set_margin_end(8);
        page.set_margin_top(8);
        page.set_margin_bottom(8);

        // Foreground section
        let fg_label = Label::new(Some("Foreground:"));
        fg_label.set_xalign(0.0);
        page.append(&fg_label);

        let fg_type_row = GtkBox::new(Orientation::Horizontal, 8);
        let fg_solid_radio = CheckButton::with_label("Solid");
        fg_solid_radio.set_active(true);
        fg_type_row.append(&fg_solid_radio);
        let fg_gradient_radio = CheckButton::with_label("Gradient");
        fg_gradient_radio.set_group(Some(&fg_solid_radio));
        fg_type_row.append(&fg_gradient_radio);
        page.append(&fg_type_row);

        let initial_fg_source = {
            let cfg = config.borrow();
            match &cfg.foreground {
                BarFillType::Solid { color } => color.clone(),
                _ => ColorSource::custom(Color::new(0.2, 0.6, 1.0, 1.0)),
            }
        };
        let fg_color_widget = Rc::new(ThemeColorSelector::new(initial_fg_source));
        fg_color_widget.set_theme_config(theme.borrow().clone());
        page.append(fg_color_widget.widget());

        let fg_gradient_editor = Rc::new(GradientEditor::new());
        fg_gradient_editor.widget().set_visible(false);
        page.append(fg_gradient_editor.widget());

        // Foreground gradient copy/paste buttons
        let fg_copy_paste_box = GtkBox::new(Orientation::Horizontal, 8);
        let fg_copy_gradient_btn = Button::with_label("Copy Gradient");
        let fg_paste_gradient_btn = Button::with_label("Paste Gradient");
        fg_copy_paste_box.append(&fg_copy_gradient_btn);
        fg_copy_paste_box.append(&fg_paste_gradient_btn);
        fg_copy_paste_box.set_visible(false);
        page.append(&fg_copy_paste_box);

        // Gradient spans bars option
        let gradient_spans_bars_check = CheckButton::with_label("Gradient spans across bars");
        gradient_spans_bars_check.set_tooltip_text(Some("Each bar shows a single color sampled from the gradient based on its position"));
        gradient_spans_bars_check.set_active(config.borrow().gradient_spans_bars);
        gradient_spans_bars_check.set_visible(false); // Only visible when gradient is selected
        page.append(&gradient_spans_bars_check);

        // Background section
        let bg_label = Label::new(Some("Background:"));
        bg_label.set_xalign(0.0);
        page.append(&bg_label);

        let bg_type_row = GtkBox::new(Orientation::Horizontal, 8);
        let bg_solid_radio = CheckButton::with_label("Solid");
        bg_type_row.append(&bg_solid_radio);
        let bg_gradient_radio = CheckButton::with_label("Gradient");
        bg_gradient_radio.set_group(Some(&bg_solid_radio));
        bg_type_row.append(&bg_gradient_radio);
        let bg_transparent_radio = CheckButton::with_label("Transparent");
        bg_transparent_radio.set_group(Some(&bg_solid_radio));
        bg_transparent_radio.set_active(true);
        bg_type_row.append(&bg_transparent_radio);
        page.append(&bg_type_row);

        let initial_bg_source = {
            let cfg = config.borrow();
            match &cfg.background {
                BarBackgroundType::Solid { color } => color.clone(),
                _ => ColorSource::custom(Color::new(0.2, 0.2, 0.2, 0.5)),
            }
        };
        let bg_color_widget = Rc::new(ThemeColorSelector::new(initial_bg_source));
        bg_color_widget.set_theme_config(theme.borrow().clone());
        bg_color_widget.widget().set_visible(false);
        page.append(bg_color_widget.widget());

        let bg_gradient_editor = Rc::new(GradientEditor::new());
        bg_gradient_editor.widget().set_visible(false);
        page.append(bg_gradient_editor.widget());

        // Background gradient copy/paste buttons
        let bg_copy_paste_box = GtkBox::new(Orientation::Horizontal, 8);
        let bg_copy_gradient_btn = Button::with_label("Copy Gradient");
        let bg_paste_gradient_btn = Button::with_label("Paste Gradient");
        bg_copy_paste_box.append(&bg_copy_gradient_btn);
        bg_copy_paste_box.append(&bg_paste_gradient_btn);
        bg_copy_paste_box.set_visible(false);
        page.append(&bg_copy_paste_box);

        // Border section
        let border_label = Label::new(Some("Border:"));
        border_label.set_xalign(0.0);
        page.append(&border_label);

        let border_row = GtkBox::new(Orientation::Horizontal, 8);
        let border_check = CheckButton::with_label("Enable Border");
        border_row.append(&border_check);
        border_row.append(&Label::new(Some("Width:")));
        let border_adj = Adjustment::new(1.0, 0.5, 5.0, 0.5, 1.0, 0.0);
        let border_width_spin = SpinButton::new(Some(&border_adj), 0.5, 1);
        border_row.append(&border_width_spin);
        page.append(&border_row);

        let initial_border_source = {
            let cfg = config.borrow();
            cfg.border.color.clone()
        };
        let border_color_widget = Rc::new(ThemeColorSelector::new(initial_border_source));
        border_color_widget.set_theme_config(theme.borrow().clone());
        page.append(border_color_widget.widget());

        // Connect foreground signals
        let fg_color_widget_vis = fg_color_widget.widget().clone();
        let fg_gradient_editor_vis = fg_gradient_editor.widget().clone();
        let fg_copy_paste_box_vis = fg_copy_paste_box.clone();
        let gradient_spans_bars_check_vis = gradient_spans_bars_check.clone();
        let config_for_fg_solid = config.clone();
        let on_change_for_fg_solid = on_change.clone();
        let fg_color_widget_for_solid = fg_color_widget.clone();
        fg_solid_radio.connect_toggled(move |radio| {
            fg_color_widget_vis.set_visible(radio.is_active());
            fg_gradient_editor_vis.set_visible(!radio.is_active());
            fg_copy_paste_box_vis.set_visible(!radio.is_active());
            gradient_spans_bars_check_vis.set_visible(!radio.is_active());
            if radio.is_active() {
                let color = fg_color_widget_for_solid.source();
                config_for_fg_solid.borrow_mut().foreground = BarFillType::Solid { color };
                if let Some(ref cb) = *on_change_for_fg_solid.borrow() {
                    cb();
                }
            }
        });

        let fg_color_widget_vis2 = fg_color_widget.widget().clone();
        let fg_gradient_editor_vis2 = fg_gradient_editor.widget().clone();
        let fg_copy_paste_box_vis2 = fg_copy_paste_box.clone();
        let gradient_spans_bars_check_vis2 = gradient_spans_bars_check.clone();
        let config_for_fg_gradient = config.clone();
        let on_change_for_fg_gradient_radio = on_change.clone();
        let fg_gradient_editor_for_radio = fg_gradient_editor.clone();
        fg_gradient_radio.connect_toggled(move |radio| {
            fg_color_widget_vis2.set_visible(!radio.is_active());
            fg_gradient_editor_vis2.set_visible(radio.is_active());
            fg_copy_paste_box_vis2.set_visible(radio.is_active());
            gradient_spans_bars_check_vis2.set_visible(radio.is_active());
            if radio.is_active() {
                let gradient = fg_gradient_editor_for_radio.get_gradient();
                let stops: Vec<ColorStopSource> = gradient.stops.iter()
                    .map(|s| ColorStopSource::custom(s.position, s.color))
                    .collect();
                config_for_fg_gradient.borrow_mut().foreground = BarFillType::Gradient {
                    stops,
                    angle: gradient.angle
                };
                if let Some(ref cb) = *on_change_for_fg_gradient_radio.borrow() {
                    cb();
                }
            }
        });

        let config_clone2 = config.clone();
        let on_change_clone2 = on_change.clone();
        fg_color_widget.set_on_change(move |color_source| {
            config_clone2.borrow_mut().foreground = BarFillType::Solid { color: color_source };
            if let Some(ref cb) = *on_change_clone2.borrow() {
                cb();
            }
        });

        let config_for_fg_gradient = config.clone();
        let on_change_for_fg_gradient = on_change.clone();
        let fg_gradient_editor_for_change = fg_gradient_editor.clone();
        fg_gradient_editor.set_on_change(move || {
            let gradient_source = fg_gradient_editor_for_change.get_gradient_source_config();
            config_for_fg_gradient.borrow_mut().foreground = BarFillType::Gradient {
                stops: gradient_source.stops,
                angle: gradient_source.angle,
            };
            if let Some(ref cb) = *on_change_for_fg_gradient.borrow() {
                cb();
            }
        });

        // Connect background signals
        let bg_color_widget_vis = bg_color_widget.widget().clone();
        let bg_gradient_editor_vis = bg_gradient_editor.widget().clone();
        let bg_copy_paste_box_vis = bg_copy_paste_box.clone();
        let config_for_bg_solid = config.clone();
        let on_change_for_bg_solid = on_change.clone();
        let bg_color_widget_for_solid = bg_color_widget.clone();
        bg_solid_radio.connect_toggled(move |radio| {
            bg_color_widget_vis.set_visible(radio.is_active());
            bg_gradient_editor_vis.set_visible(false);
            bg_copy_paste_box_vis.set_visible(false);
            if radio.is_active() {
                let color = bg_color_widget_for_solid.source();
                config_for_bg_solid.borrow_mut().background = BarBackgroundType::Solid { color };
                if let Some(ref cb) = *on_change_for_bg_solid.borrow() {
                    cb();
                }
            }
        });

        let bg_color_widget_vis2 = bg_color_widget.widget().clone();
        let bg_gradient_editor_vis2 = bg_gradient_editor.widget().clone();
        let bg_copy_paste_box_vis2 = bg_copy_paste_box.clone();
        let config_for_bg_gradient = config.clone();
        let on_change_for_bg_gradient_radio = on_change.clone();
        let bg_gradient_editor_for_radio = bg_gradient_editor.clone();
        bg_gradient_radio.connect_toggled(move |radio| {
            bg_color_widget_vis2.set_visible(false);
            bg_gradient_editor_vis2.set_visible(radio.is_active());
            bg_copy_paste_box_vis2.set_visible(radio.is_active());
            if radio.is_active() {
                let gradient = bg_gradient_editor_for_radio.get_gradient();
                let stops: Vec<ColorStopSource> = gradient.stops.iter()
                    .map(|s| ColorStopSource::custom(s.position, s.color))
                    .collect();
                config_for_bg_gradient.borrow_mut().background = BarBackgroundType::Gradient {
                    stops,
                    angle: gradient.angle
                };
                if let Some(ref cb) = *on_change_for_bg_gradient_radio.borrow() {
                    cb();
                }
            }
        });

        let bg_color_widget_vis3 = bg_color_widget.widget().clone();
        let bg_gradient_editor_vis3 = bg_gradient_editor.widget().clone();
        let bg_copy_paste_box_vis3 = bg_copy_paste_box.clone();
        let config_clone5 = config.clone();
        let on_change_clone5 = on_change.clone();
        bg_transparent_radio.connect_toggled(move |radio| {
            bg_color_widget_vis3.set_visible(false);
            bg_gradient_editor_vis3.set_visible(false);
            bg_copy_paste_box_vis3.set_visible(false);
            if radio.is_active() {
                config_clone5.borrow_mut().background = BarBackgroundType::Transparent;
                if let Some(ref cb) = *on_change_clone5.borrow() {
                    cb();
                }
            }
        });

        let config_clone6 = config.clone();
        let on_change_clone6 = on_change.clone();
        bg_color_widget.set_on_change(move |color_source| {
            config_clone6.borrow_mut().background = BarBackgroundType::Solid { color: color_source };
            if let Some(ref cb) = *on_change_clone6.borrow() {
                cb();
            }
        });

        let config_for_bg_gradient = config.clone();
        let on_change_for_bg_gradient = on_change.clone();
        let bg_gradient_editor_for_change = bg_gradient_editor.clone();
        bg_gradient_editor.set_on_change(move || {
            let gradient_source = bg_gradient_editor_for_change.get_gradient_source_config();
            config_for_bg_gradient.borrow_mut().background = BarBackgroundType::Gradient {
                stops: gradient_source.stops,
                angle: gradient_source.angle,
            };
            if let Some(ref cb) = *on_change_for_bg_gradient.borrow() {
                cb();
            }
        });

        // Connect foreground gradient copy/paste buttons
        let fg_gradient_editor_for_copy = fg_gradient_editor.clone();
        fg_copy_gradient_btn.connect_clicked(move |_| {
            let gradient = fg_gradient_editor_for_copy.get_gradient();
            if let Ok(mut clipboard) = CLIPBOARD.lock() {
                clipboard.copy_gradient_stops(gradient.stops);
            }
        });

        let fg_gradient_editor_for_paste = fg_gradient_editor.clone();
        let config_for_fg_paste = config.clone();
        let on_change_for_fg_paste = on_change.clone();
        fg_paste_gradient_btn.connect_clicked(move |_| {
            if let Ok(clipboard) = CLIPBOARD.lock() {
                if let Some(pasted_stops) = clipboard.paste_gradient_stops() {
                    fg_gradient_editor_for_paste.set_stops(pasted_stops.clone());
                    let gradient = fg_gradient_editor_for_paste.get_gradient();
                    let stops: Vec<ColorStopSource> = gradient.stops.iter()
                        .map(|s| ColorStopSource::custom(s.position, s.color))
                        .collect();
                    config_for_fg_paste.borrow_mut().foreground = BarFillType::Gradient {
                        stops,
                        angle: gradient.angle
                    };
                    if let Some(ref cb) = *on_change_for_fg_paste.borrow() {
                        cb();
                    }
                }
            }
        });

        // Connect background gradient copy/paste buttons
        let bg_gradient_editor_for_copy = bg_gradient_editor.clone();
        bg_copy_gradient_btn.connect_clicked(move |_| {
            let gradient = bg_gradient_editor_for_copy.get_gradient();
            if let Ok(mut clipboard) = CLIPBOARD.lock() {
                clipboard.copy_gradient_stops(gradient.stops);
            }
        });

        let bg_gradient_editor_for_paste = bg_gradient_editor.clone();
        let config_for_bg_paste = config.clone();
        let on_change_for_bg_paste = on_change.clone();
        bg_paste_gradient_btn.connect_clicked(move |_| {
            if let Ok(clipboard) = CLIPBOARD.lock() {
                if let Some(pasted_stops) = clipboard.paste_gradient_stops() {
                    bg_gradient_editor_for_paste.set_stops(pasted_stops.clone());
                    let gradient = bg_gradient_editor_for_paste.get_gradient();
                    let stops: Vec<ColorStopSource> = gradient.stops.iter()
                        .map(|s| ColorStopSource::custom(s.position, s.color))
                        .collect();
                    config_for_bg_paste.borrow_mut().background = BarBackgroundType::Gradient {
                        stops,
                        angle: gradient.angle
                    };
                    if let Some(ref cb) = *on_change_for_bg_paste.borrow() {
                        cb();
                    }
                }
            }
        });

        // Connect border signals
        let config_clone7 = config.clone();
        let on_change_clone7 = on_change.clone();
        border_check.connect_toggled(move |check| {
            config_clone7.borrow_mut().border.enabled = check.is_active();
            if let Some(ref cb) = *on_change_clone7.borrow() {
                cb();
            }
        });

        let config_clone8 = config.clone();
        let on_change_clone8 = on_change.clone();
        border_width_spin.connect_value_changed(move |spin| {
            config_clone8.borrow_mut().border.width = spin.value();
            if let Some(ref cb) = *on_change_clone8.borrow() {
                cb();
            }
        });

        let config_clone9 = config.clone();
        let on_change_clone9 = on_change.clone();
        border_color_widget.set_on_change(move |color_source| {
            config_clone9.borrow_mut().border.color = color_source;
            if let Some(ref cb) = *on_change_clone9.borrow() {
                cb();
            }
        });

        // Connect gradient spans bars checkbox
        let config_for_gradient_spans = config.clone();
        let on_change_for_gradient_spans = on_change.clone();
        gradient_spans_bars_check.connect_toggled(move |check| {
            config_for_gradient_spans.borrow_mut().gradient_spans_bars = check.is_active();
            if let Some(ref cb) = *on_change_for_gradient_spans.borrow() {
                cb();
            }
        });

        (page, fg_solid_radio, fg_gradient_radio, fg_color_widget, fg_gradient_editor,
         bg_solid_radio, bg_gradient_radio, bg_transparent_radio, bg_color_widget, bg_gradient_editor,
         border_check, border_width_spin, border_color_widget, gradient_spans_bars_check)
    }

    fn create_labels_page(
        config: &Rc<RefCell<CoreBarsConfig>>,
        theme: &Rc<RefCell<ComboThemeConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
    ) -> (GtkBox, CheckButton, Entry, DropDown, Button, SpinButton, Rc<ThemeColorSelector>, CheckButton) {
        let page = GtkBox::new(Orientation::Vertical, 8);
        page.set_margin_start(8);
        page.set_margin_end(8);
        page.set_margin_top(8);
        page.set_margin_bottom(8);

        // Show labels checkbox
        let show_labels_check = CheckButton::with_label("Show Labels");
        show_labels_check.set_active(config.borrow().show_labels);
        page.append(&show_labels_check);

        // Label prefix
        let prefix_row = GtkBox::new(Orientation::Horizontal, 8);
        prefix_row.append(&Label::new(Some("Prefix:")));
        let label_prefix_entry = Entry::new();
        label_prefix_entry.set_text(&config.borrow().label_prefix);
        label_prefix_entry.set_placeholder_text(Some("e.g., Core , CPU , #"));
        label_prefix_entry.set_hexpand(true);
        prefix_row.append(&label_prefix_entry);
        page.append(&prefix_row);

        // Label position
        let pos_row = GtkBox::new(Orientation::Horizontal, 8);
        pos_row.append(&Label::new(Some("Position:")));
        let pos_options = StringList::new(&["Start", "End", "Inside"]);
        let label_position_dropdown = DropDown::new(Some(pos_options), Option::<gtk4::Expression>::None);
        label_position_dropdown.set_selected(0);
        label_position_dropdown.set_hexpand(true);
        pos_row.append(&label_position_dropdown);
        page.append(&pos_row);

        // Font button with copy/paste
        let font_row = GtkBox::new(Orientation::Horizontal, 8);
        font_row.append(&Label::new(Some("Font:")));
        let cfg = config.borrow();
        let label_font_button = Button::with_label(&format!("{} {:.0}", cfg.label_font, cfg.label_size));
        label_font_button.set_hexpand(true);
        font_row.append(&label_font_button);
        drop(cfg);

        // Copy font button
        let copy_font_btn = Button::with_label("Copy");
        copy_font_btn.set_tooltip_text(Some("Copy font settings"));
        font_row.append(&copy_font_btn);

        // Paste font button
        let paste_font_btn = Button::with_label("Paste");
        paste_font_btn.set_tooltip_text(Some("Paste font settings"));
        font_row.append(&paste_font_btn);

        page.append(&font_row);

        // Font size
        let size_row = GtkBox::new(Orientation::Horizontal, 8);
        size_row.append(&Label::new(Some("Size:")));
        let size_adj = Adjustment::new(10.0, 6.0, 32.0, 1.0, 2.0, 0.0);
        let label_size_spin = SpinButton::new(Some(&size_adj), 1.0, 0);
        label_size_spin.set_value(config.borrow().label_size);
        label_size_spin.set_hexpand(true);
        size_row.append(&label_size_spin);
        page.append(&size_row);

        // Font color
        let color_row = GtkBox::new(Orientation::Horizontal, 8);
        color_row.append(&Label::new(Some("Color:")));
        let label_color_widget = Rc::new(ThemeColorSelector::new(config.borrow().label_color.clone()));
        label_color_widget.set_theme_config(theme.borrow().clone());
        color_row.append(label_color_widget.widget());
        page.append(&color_row);

        // Bold checkbox
        let label_bold_check = CheckButton::with_label("Bold");
        label_bold_check.set_active(config.borrow().label_bold);
        page.append(&label_bold_check);

        // Connect signals
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        show_labels_check.connect_toggled(move |check| {
            config_clone.borrow_mut().show_labels = check.is_active();
            if let Some(ref cb) = *on_change_clone.borrow() {
                cb();
            }
        });

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        label_prefix_entry.connect_changed(move |entry| {
            config_clone.borrow_mut().label_prefix = entry.text().to_string();
            if let Some(ref cb) = *on_change_clone.borrow() {
                cb();
            }
        });

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        label_position_dropdown.connect_selected_notify(move |dd| {
            let selected = dd.selected();
            if selected == gtk4::INVALID_LIST_POSITION {
                return;
            }
            let pos = match selected {
                0 => LabelPosition::Start,
                1 => LabelPosition::End,
                _ => LabelPosition::Inside,
            };
            config_clone.borrow_mut().label_position = pos;
            if let Some(ref cb) = *on_change_clone.borrow() {
                cb();
            }
        });

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let label_font_btn_clone = label_font_button.clone();
        let label_size_spin_clone = label_size_spin.clone();
        label_font_button.connect_clicked(move |btn| {
            let window = btn.root().and_then(|r| r.downcast::<gtk4::Window>().ok());
            let config_clone2 = config_clone.clone();
            let font_btn = label_font_btn_clone.clone();
            let size_spin = label_size_spin_clone.clone();
            let on_change_clone2 = on_change_clone.clone();

            if let Some(win) = window {
                show_font_dialog(Some(&win), None, move |font_desc| {
                    let family = font_desc.family().map(|f| f.to_string()).unwrap_or_else(|| "Sans".to_string());
                    config_clone2.borrow_mut().label_font = family.clone();
                    let size = size_spin.value();
                    font_btn.set_label(&format!("{} {:.0}", family, size));
                    if let Some(ref cb) = *on_change_clone2.borrow() {
                        cb();
                    }
                });
            }
        });

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let label_font_btn_clone2 = label_font_button.clone();
        label_size_spin.connect_value_changed(move |spin| {
            let size = spin.value();
            let mut cfg = config_clone.borrow_mut();
            cfg.label_size = size;
            let family = cfg.label_font.clone();
            drop(cfg);
            label_font_btn_clone2.set_label(&format!("{} {:.0}", family, size));
            if let Some(ref cb) = *on_change_clone.borrow() {
                cb();
            }
        });

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        label_color_widget.set_on_change(move |color_source| {
            config_clone.borrow_mut().label_color = color_source;
            if let Some(ref cb) = *on_change_clone.borrow() {
                cb();
            }
        });

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        label_bold_check.connect_toggled(move |check| {
            config_clone.borrow_mut().label_bold = check.is_active();
            if let Some(ref cb) = *on_change_clone.borrow() {
                cb();
            }
        });

        // Copy font handler
        let config_for_copy = config.clone();
        copy_font_btn.connect_clicked(move |_| {
            let cfg = config_for_copy.borrow();
            if let Ok(mut clipboard) = CLIPBOARD.lock() {
                clipboard.copy_font(cfg.label_font.clone(), cfg.label_size, cfg.label_bold, false);
            }
        });

        // Paste font handler
        let config_for_paste = config.clone();
        let on_change_for_paste = on_change.clone();
        let font_btn_for_paste = label_font_button.clone();
        let size_spin_for_paste = label_size_spin.clone();
        let bold_check_for_paste = label_bold_check.clone();
        paste_font_btn.connect_clicked(move |_| {
            let Ok(clipboard) = CLIPBOARD.lock() else { return };
            if let Some((family, size, bold, _italic)) = clipboard.paste_font() {
                let mut cfg = config_for_paste.borrow_mut();
                cfg.label_font = family.clone();
                cfg.label_size = size;
                cfg.label_bold = bold;
                drop(cfg);
                font_btn_for_paste.set_label(&format!("{} {:.0}", family, size));
                size_spin_for_paste.set_value(size);
                bold_check_for_paste.set_active(bold);
                if let Some(ref cb) = *on_change_for_paste.borrow() {
                    cb();
                }
            }
        });

        (page, show_labels_check, label_prefix_entry, label_position_dropdown,
         label_font_button, label_size_spin, label_color_widget, label_bold_check)
    }

    fn create_animation_page(
        config: &Rc<RefCell<CoreBarsConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
    ) -> (GtkBox, CheckButton, Scale) {
        let page = GtkBox::new(Orientation::Vertical, 8);
        page.set_margin_start(8);
        page.set_margin_end(8);
        page.set_margin_top(8);
        page.set_margin_bottom(8);

        // Animate checkbox
        let animate_check = CheckButton::with_label("Enable Animation");
        animate_check.set_active(config.borrow().animate);
        page.append(&animate_check);

        // Animation speed
        let speed_row = GtkBox::new(Orientation::Horizontal, 8);
        speed_row.append(&Label::new(Some("Speed:")));
        let animation_speed_scale = Scale::with_range(Orientation::Horizontal, 1.0, 20.0, 0.5);
        animation_speed_scale.set_value(config.borrow().animation_speed);
        animation_speed_scale.set_hexpand(true);
        speed_row.append(&animation_speed_scale);
        page.append(&speed_row);

        // Connect signals
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        animate_check.connect_toggled(move |check| {
            config_clone.borrow_mut().animate = check.is_active();
            if let Some(ref cb) = *on_change_clone.borrow() {
                cb();
            }
        });

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        animation_speed_scale.connect_value_changed(move |scale| {
            config_clone.borrow_mut().animation_speed = scale.value();
            if let Some(ref cb) = *on_change_clone.borrow() {
                cb();
            }
        });

        (page, animate_check, animation_speed_scale)
    }

    /// Get the container widget
    pub fn widget(&self) -> &GtkBox {
        &self.container
    }

    /// Get the current configuration
    pub fn get_config(&self) -> CoreBarsConfig {
        let mut config = self.config.borrow().clone();

        // Update text overlay from widget
        config.text_overlay = self.text_overlay_widget.get_config();

        config
    }

    /// Set the configuration
    pub fn set_config(&self, config: CoreBarsConfig) {
        // Update UI elements
        self.start_core_spin.set_value(config.start_core as f64);
        self.end_core_spin.set_value(config.end_core as f64);

        // Padding
        self.padding_top_spin.set_value(config.padding_top);
        self.padding_bottom_spin.set_value(config.padding_bottom);
        self.padding_left_spin.set_value(config.padding_left);
        self.padding_right_spin.set_value(config.padding_right);

        self.style_dropdown.set_selected(match config.bar_style {
            BarStyle::Full => 0,
            BarStyle::Rectangle => 1,
            BarStyle::Segmented => 2,
        });

        self.orientation_dropdown.set_selected(match config.orientation {
            BarOrientation::Horizontal => 0,
            BarOrientation::Vertical => 1,
        });

        // Update fill direction dropdown options based on orientation
        let direction_options = if matches!(config.orientation, BarOrientation::Horizontal) {
            StringList::new(&["Left to Right", "Right to Left"])
        } else {
            StringList::new(&["Bottom to Top", "Top to Bottom"])
        };
        self.fill_direction_dropdown.set_model(Some(&direction_options));

        // Set the correct selection based on orientation
        let selection = match (&config.orientation, &config.fill_direction) {
            (BarOrientation::Horizontal, BarFillDirection::LeftToRight) => 0,
            (BarOrientation::Horizontal, BarFillDirection::RightToLeft) => 1,
            (BarOrientation::Vertical, BarFillDirection::BottomToTop) => 0,
            (BarOrientation::Vertical, BarFillDirection::TopToBottom) => 1,
            // Fallback to defaults if orientation/direction mismatch
            (BarOrientation::Horizontal, _) => 0, // Default to LeftToRight
            (BarOrientation::Vertical, _) => 0,   // Default to BottomToTop
        };
        self.fill_direction_dropdown.set_selected(selection);

        self.corner_radius_spin.set_value(config.corner_radius);
        self.bar_spacing_spin.set_value(config.bar_spacing);
        self.segment_count_spin.set_value(config.segment_count as f64);
        self.segment_spacing_spin.set_value(config.segment_spacing);

        // Colors
        match &config.foreground {
            BarFillType::Solid { color } => {
                self.fg_solid_radio.set_active(true);
                self.fg_color_widget.set_source(color.clone());
            }
            BarFillType::Gradient { stops, angle } => {
                self.fg_gradient_editor.set_gradient_source(*angle, stops.clone());
                self.fg_gradient_radio.set_active(true);
            }
        }

        match &config.background {
            BarBackgroundType::Solid { color } => {
                self.bg_solid_radio.set_active(true);
                self.bg_color_widget.set_source(color.clone());
            }
            BarBackgroundType::Gradient { stops, angle } => {
                self.bg_gradient_editor.set_gradient_source(*angle, stops.clone());
                self.bg_gradient_radio.set_active(true);
            }
            BarBackgroundType::Transparent => {
                self.bg_transparent_radio.set_active(true);
            }
        }

        self.border_check.set_active(config.border.enabled);
        self.border_width_spin.set_value(config.border.width);
        self.border_color_widget.set_source(config.border.color.clone());

        // Labels
        self.show_labels_check.set_active(config.show_labels);
        self.label_prefix_entry.set_text(&config.label_prefix);
        self.label_position_dropdown.set_selected(match config.label_position {
            LabelPosition::Start => 0,
            LabelPosition::End => 1,
            LabelPosition::Inside => 2,
        });
        self.label_font_button.set_label(&format!("{} {:.0}", config.label_font, config.label_size));
        self.label_size_spin.set_value(config.label_size);
        self.label_color_widget.set_source(config.label_color.clone());
        self.label_bold_check.set_active(config.label_bold);

        // Animation
        self.animate_check.set_active(config.animate);
        self.animation_speed_scale.set_value(config.animation_speed);

        // Gradient spans bars
        self.gradient_spans_bars_check.set_active(config.gradient_spans_bars);
        // Update visibility based on foreground type
        let is_gradient = matches!(config.foreground, BarFillType::Gradient { .. });
        self.gradient_spans_bars_check.set_visible(is_gradient);

        // Text overlay
        self.text_overlay_widget.set_config(config.text_overlay.clone());

        // Store config
        *self.config.borrow_mut() = config;

        // Refresh preview
        self.preview.queue_draw();
    }

    /// Set callback for config changes
    pub fn set_on_change<F: Fn() + 'static>(&self, callback: F) {
        let preview = self.preview.clone();
        *self.on_change.borrow_mut() = Some(Box::new(move || {
            preview.queue_draw();
            callback();
        }));
    }

    /// Set the maximum core count (call this when CPU source is available)
    pub fn set_max_cores(&self, max_cores: usize) {
        if max_cores > 0 {
            self.end_core_spin.adjustment().set_upper((max_cores - 1) as f64);
            self.start_core_spin.adjustment().set_upper((max_cores - 1) as f64);
        }
    }

    /// Update the theme configuration and refresh the preview
    pub fn set_theme(&self, theme: ComboThemeConfig) {
        *self.theme.borrow_mut() = theme.clone();
        // Update all color selectors with the new theme
        self.fg_color_widget.set_theme_config(theme.clone());
        self.fg_gradient_editor.set_theme_config(theme.clone());
        self.bg_color_widget.set_theme_config(theme.clone());
        self.bg_gradient_editor.set_theme_config(theme.clone());
        self.border_color_widget.set_theme_config(theme.clone());
        self.label_color_widget.set_theme_config(theme.clone());
        // Update text overlay widget
        self.text_overlay_widget.set_theme(theme);
        self.preview.queue_draw();
        // Notify parent to refresh with new theme colors
        if let Some(callback) = self.on_change.borrow().as_ref() {
            callback();
        }
    }

    /// Create default fields for combo panel core bars slots
    fn default_combo_fields() -> Vec<FieldMetadata> {
        use crate::core::{FieldType, FieldPurpose};

        let mut fields = vec![
            // Common slot fields
            FieldMetadata::new("caption", "Caption", "Slot caption text", FieldType::Text, FieldPurpose::Caption),
            FieldMetadata::new("value", "Value", "Formatted value string", FieldType::Text, FieldPurpose::Value),
            FieldMetadata::new("unit", "Unit", "Value unit", FieldType::Text, FieldPurpose::Unit),
            FieldMetadata::new("numerical_value", "Numerical Value", "Raw numerical value", FieldType::Numerical, FieldPurpose::Value),
        ];

        // Add core usage fields (core0 through core31 to cover most systems)
        for i in 0..32 {
            fields.push(FieldMetadata::new(
                format!("core{}_usage", i),
                format!("Core {} Usage", i),
                format!("CPU core {} usage percentage", i),
                FieldType::Numerical,
                FieldPurpose::Value,
            ));
        }

        fields
    }
}

impl Default for CoreBarsConfigWidget {
    fn default() -> Self {
        Self::new()
    }
}

/// Lazy wrapper for CoreBarsConfigWidget to defer expensive widget creation
///
/// The actual CoreBarsConfigWidget (with preview, notebook pages, etc.) is only created
/// when the widget becomes visible (mapped), saving significant memory when many
/// content items are created but only one display type is active.
pub struct LazyCoreBarsConfigWidget {
    /// Container that holds either the placeholder or the actual widget
    container: GtkBox,
    /// The actual widget, created lazily on first map
    inner_widget: Rc<RefCell<Option<CoreBarsConfigWidget>>>,
    /// Deferred config to apply when widget is created
    deferred_config: Rc<RefCell<CoreBarsConfig>>,
    /// Deferred theme to apply when widget is created
    deferred_theme: Rc<RefCell<ComboThemeConfig>>,
    /// Deferred max_cores to apply when widget is created
    deferred_max_cores: Rc<RefCell<Option<usize>>>,
    /// Callback to invoke on config changes
    on_change: Rc<RefCell<Option<Box<dyn Fn()>>>>,
}

impl LazyCoreBarsConfigWidget {
    /// Create a new lazy core bars config widget
    ///
    /// The actual CoreBarsConfigWidget is NOT created here - it's created automatically
    /// when the widget becomes visible (mapped).
    pub fn new() -> Self {
        let container = GtkBox::new(Orientation::Vertical, 0);
        let inner_widget: Rc<RefCell<Option<CoreBarsConfigWidget>>> = Rc::new(RefCell::new(None));
        let deferred_config = Rc::new(RefCell::new(CoreBarsConfig::default()));
        let deferred_theme = Rc::new(RefCell::new(ComboThemeConfig::default()));
        let deferred_max_cores = Rc::new(RefCell::new(None));
        let on_change: Rc<RefCell<Option<Box<dyn Fn()>>>> = Rc::new(RefCell::new(None));

        // Create placeholder with loading indicator
        let placeholder = GtkBox::new(Orientation::Vertical, 8);
        placeholder.set_margin_top(12);
        placeholder.set_margin_bottom(12);
        placeholder.set_margin_start(12);
        placeholder.set_margin_end(12);

        let info_label = Label::new(Some("Loading core bars configuration..."));
        info_label.add_css_class("dim-label");
        placeholder.append(&info_label);
        container.append(&placeholder);

        // Create a shared initialization closure
        let init_widget = {
            let container_clone = container.clone();
            let inner_widget_clone = inner_widget.clone();
            let deferred_config_clone = deferred_config.clone();
            let deferred_theme_clone = deferred_theme.clone();
            let deferred_max_cores_clone = deferred_max_cores.clone();
            let on_change_clone = on_change.clone();

            Rc::new(move || {
                // Only create if not already created
                if inner_widget_clone.borrow().is_none() {
                    log::info!("LazyCoreBarsConfigWidget: Creating actual CoreBarsConfigWidget on map");

                    // Create the actual widget
                    let widget = CoreBarsConfigWidget::new();

                    // Apply deferred theme first (before config, as config may trigger UI updates)
                    widget.set_theme(deferred_theme_clone.borrow().clone());

                    // Apply deferred config
                    widget.set_config(deferred_config_clone.borrow().clone());

                    // Apply deferred max_cores if set
                    if let Some(max_cores) = *deferred_max_cores_clone.borrow() {
                        widget.set_max_cores(max_cores);
                    }

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
            deferred_max_cores,
            on_change,
        }
    }

    /// Get the widget container
    pub fn widget(&self) -> &GtkBox {
        &self.container
    }

    /// Set the core bars configuration
    pub fn set_config(&self, config: CoreBarsConfig) {
        *self.deferred_config.borrow_mut() = config.clone();
        if let Some(ref widget) = *self.inner_widget.borrow() {
            widget.set_config(config);
        }
    }

    /// Get the current core bars configuration
    pub fn get_config(&self) -> CoreBarsConfig {
        if let Some(ref widget) = *self.inner_widget.borrow() {
            widget.get_config()
        } else {
            self.deferred_config.borrow().clone()
        }
    }

    /// Set the theme for the core bars widget
    pub fn set_theme(&self, theme: ComboThemeConfig) {
        *self.deferred_theme.borrow_mut() = theme.clone();
        if let Some(ref widget) = *self.inner_widget.borrow() {
            widget.set_theme(theme);
        }
    }

    /// Set the maximum number of CPU cores
    pub fn set_max_cores(&self, max_cores: usize) {
        *self.deferred_max_cores.borrow_mut() = Some(max_cores);
        if let Some(ref widget) = *self.inner_widget.borrow() {
            widget.set_max_cores(max_cores);
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
}

impl Default for LazyCoreBarsConfigWidget {
    fn default() -> Self {
        Self::new()
    }
}
