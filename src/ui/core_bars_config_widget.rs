//! Core bars configuration widget with tabbed interface

use gtk4::prelude::*;
use gtk4::{
    Adjustment, Box as GtkBox, Button, CheckButton, DrawingArea, DropDown, Entry, Label,
    Notebook, Orientation, Scale, SpinButton, StringList,
};
use std::cell::RefCell;
use std::rc::Rc;

use crate::ui::background::Color;
use crate::ui::bar_display::{BarBackgroundType, BarFillDirection, BarFillType, BarOrientation, BarStyle};
use crate::ui::clipboard::CLIPBOARD;
use crate::ui::color_button_widget::ColorButtonWidget;
use crate::ui::core_bars_display::{CoreBarsConfig, LabelPosition, render_core_bars};
use crate::ui::shared_font_dialog::shared_font_dialog;
use crate::ui::GradientEditor;

/// Core bars configuration widget
pub struct CoreBarsConfigWidget {
    container: GtkBox,
    config: Rc<RefCell<CoreBarsConfig>>,
    preview: DrawingArea,
    on_change: Rc<RefCell<Option<Box<dyn Fn()>>>>,

    // Core selection
    start_core_spin: SpinButton,
    end_core_spin: SpinButton,

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
    fg_color_widget: Rc<ColorButtonWidget>,
    #[allow(dead_code)]
    fg_gradient_editor: Rc<GradientEditor>,
    bg_solid_radio: CheckButton,
    bg_gradient_radio: CheckButton,
    bg_transparent_radio: CheckButton,
    bg_color_widget: Rc<ColorButtonWidget>,
    #[allow(dead_code)]
    bg_gradient_editor: Rc<GradientEditor>,

    // Border
    border_check: CheckButton,
    border_width_spin: SpinButton,
    border_color_widget: Rc<ColorButtonWidget>,

    // Labels
    show_labels_check: CheckButton,
    label_prefix_entry: Entry,
    label_position_dropdown: DropDown,
    label_font_button: Button,
    label_size_spin: SpinButton,
    label_color_widget: Rc<ColorButtonWidget>,
    label_bold_check: CheckButton,

    // Animation
    animate_check: CheckButton,
    animation_speed_scale: Scale,
}

impl CoreBarsConfigWidget {
    pub fn new() -> Self {
        let container = GtkBox::new(Orientation::Vertical, 8);
        container.set_margin_start(8);
        container.set_margin_end(8);
        container.set_margin_top(8);
        container.set_margin_bottom(8);

        let config = Rc::new(RefCell::new(CoreBarsConfig::default()));
        let on_change: Rc<RefCell<Option<Box<dyn Fn()>>>> = Rc::new(RefCell::new(None));

        // Create notebook for tabs
        let notebook = Notebook::new();
        notebook.set_vexpand(true);

        // === Tab 1: Cores ===
        let (cores_page, start_core_spin, end_core_spin) =
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
             border_check, border_width_spin, border_color_widget) =
            Self::create_colors_page(&config, &on_change);
        notebook.append_page(&colors_page, Some(&Label::new(Some("Colors"))));

        // === Tab 4: Labels ===
        let (labels_page, show_labels_check, label_prefix_entry, label_position_dropdown,
             label_font_button, label_size_spin, label_color_widget, label_bold_check) =
            Self::create_labels_page(&config, &on_change);
        notebook.append_page(&labels_page, Some(&Label::new(Some("Labels"))));

        // === Tab 5: Animation ===
        let (animation_page, animate_check, animation_speed_scale) =
            Self::create_animation_page(&config, &on_change);
        notebook.append_page(&animation_page, Some(&Label::new(Some("Animation"))));

        // Preview at bottom
        let preview = DrawingArea::new();
        preview.set_content_height(150);
        preview.set_vexpand(false);

        let config_for_preview = config.clone();
        preview.set_draw_func(move |_, cr, width, height| {
            // Checkerboard background
            Self::render_checkerboard(cr, width as f64, height as f64);

            let cfg = config_for_preview.borrow();
            // Sample values for preview
            let sample_values: Vec<f64> = (0..cfg.core_count().min(8))
                .map(|i| 0.2 + (i as f64 * 0.1))
                .collect();

            if !sample_values.is_empty() {
                let _ = render_core_bars(cr, &cfg, &sample_values, width as f64, height as f64);
            }
        });

        container.append(&notebook);
        container.append(&preview);

        Self {
            container,
            config,
            preview,
            on_change,
            start_core_spin,
            end_core_spin,
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
        }
    }

    fn create_cores_page(
        config: &Rc<RefCell<CoreBarsConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
    ) -> (GtkBox, SpinButton, SpinButton) {
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

        (page, start_core_spin, end_core_spin)
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
            let style = match dd.selected() {
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
            let orientation = match dd.selected() {
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
            let is_horizontal = orientation_dropdown_clone.selected() == 0;
            let direction = if is_horizontal {
                match dd.selected() {
                    0 => BarFillDirection::LeftToRight,
                    _ => BarFillDirection::RightToLeft,
                }
            } else {
                match dd.selected() {
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
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
    ) -> (GtkBox, CheckButton, CheckButton, Rc<ColorButtonWidget>, Rc<GradientEditor>,
          CheckButton, CheckButton, CheckButton, Rc<ColorButtonWidget>, Rc<GradientEditor>,
          CheckButton, SpinButton, Rc<ColorButtonWidget>) {
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

        let initial_fg_color = match &config.borrow().foreground {
            BarFillType::Solid { color } => *color,
            _ => Color::new(0.2, 0.6, 1.0, 1.0),
        };
        let fg_color_widget = Rc::new(ColorButtonWidget::new(initial_fg_color));
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

        let bg_color_widget = Rc::new(ColorButtonWidget::new(Color::new(0.2, 0.2, 0.2, 0.5)));
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

        let border_color_widget = Rc::new(ColorButtonWidget::new(Color::new(0.5, 0.5, 0.5, 1.0)));
        page.append(border_color_widget.widget());

        // Connect foreground signals
        let fg_color_widget_vis = fg_color_widget.widget().clone();
        let fg_gradient_editor_vis = fg_gradient_editor.widget().clone();
        let fg_copy_paste_box_vis = fg_copy_paste_box.clone();
        let config_for_fg_solid = config.clone();
        let on_change_for_fg_solid = on_change.clone();
        let fg_color_widget_for_solid = fg_color_widget.clone();
        fg_solid_radio.connect_toggled(move |radio| {
            fg_color_widget_vis.set_visible(radio.is_active());
            fg_gradient_editor_vis.set_visible(!radio.is_active());
            fg_copy_paste_box_vis.set_visible(!radio.is_active());
            if radio.is_active() {
                let color = fg_color_widget_for_solid.color();
                config_for_fg_solid.borrow_mut().foreground = BarFillType::Solid { color };
                if let Some(ref cb) = *on_change_for_fg_solid.borrow() {
                    cb();
                }
            }
        });

        let fg_color_widget_vis2 = fg_color_widget.widget().clone();
        let fg_gradient_editor_vis2 = fg_gradient_editor.widget().clone();
        let fg_copy_paste_box_vis2 = fg_copy_paste_box.clone();
        let config_for_fg_gradient = config.clone();
        let on_change_for_fg_gradient_radio = on_change.clone();
        let fg_gradient_editor_for_radio = fg_gradient_editor.clone();
        fg_gradient_radio.connect_toggled(move |radio| {
            fg_color_widget_vis2.set_visible(!radio.is_active());
            fg_gradient_editor_vis2.set_visible(radio.is_active());
            fg_copy_paste_box_vis2.set_visible(radio.is_active());
            if radio.is_active() {
                let gradient = fg_gradient_editor_for_radio.get_gradient();
                config_for_fg_gradient.borrow_mut().foreground = BarFillType::Gradient {
                    stops: gradient.stops,
                    angle: gradient.angle
                };
                if let Some(ref cb) = *on_change_for_fg_gradient_radio.borrow() {
                    cb();
                }
            }
        });

        let config_clone2 = config.clone();
        let on_change_clone2 = on_change.clone();
        fg_color_widget.set_on_change(move |color| {
            config_clone2.borrow_mut().foreground = BarFillType::Solid { color };
            if let Some(ref cb) = *on_change_clone2.borrow() {
                cb();
            }
        });

        let config_for_fg_gradient = config.clone();
        let on_change_for_fg_gradient = on_change.clone();
        let fg_gradient_editor_for_change = fg_gradient_editor.clone();
        fg_gradient_editor.set_on_change(move || {
            let gradient = fg_gradient_editor_for_change.get_gradient();
            config_for_fg_gradient.borrow_mut().foreground = BarFillType::Gradient {
                stops: gradient.stops,
                angle: gradient.angle
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
                let color = bg_color_widget_for_solid.color();
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
                config_for_bg_gradient.borrow_mut().background = BarBackgroundType::Gradient {
                    stops: gradient.stops,
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
        bg_color_widget.set_on_change(move |color| {
            config_clone6.borrow_mut().background = BarBackgroundType::Solid { color };
            if let Some(ref cb) = *on_change_clone6.borrow() {
                cb();
            }
        });

        let config_for_bg_gradient = config.clone();
        let on_change_for_bg_gradient = on_change.clone();
        let bg_gradient_editor_for_change = bg_gradient_editor.clone();
        bg_gradient_editor.set_on_change(move || {
            let gradient = bg_gradient_editor_for_change.get_gradient();
            config_for_bg_gradient.borrow_mut().background = BarBackgroundType::Gradient {
                stops: gradient.stops,
                angle: gradient.angle
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
                if let Some(stops) = clipboard.paste_gradient_stops() {
                    fg_gradient_editor_for_paste.set_stops(stops.clone());
                    let gradient = fg_gradient_editor_for_paste.get_gradient();
                    config_for_fg_paste.borrow_mut().foreground = BarFillType::Gradient {
                        stops: gradient.stops,
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
                if let Some(stops) = clipboard.paste_gradient_stops() {
                    bg_gradient_editor_for_paste.set_stops(stops.clone());
                    let gradient = bg_gradient_editor_for_paste.get_gradient();
                    config_for_bg_paste.borrow_mut().background = BarBackgroundType::Gradient {
                        stops: gradient.stops,
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
        border_color_widget.set_on_change(move |color| {
            config_clone9.borrow_mut().border.color = color;
            if let Some(ref cb) = *on_change_clone9.borrow() {
                cb();
            }
        });

        (page, fg_solid_radio, fg_gradient_radio, fg_color_widget, fg_gradient_editor,
         bg_solid_radio, bg_gradient_radio, bg_transparent_radio, bg_color_widget, bg_gradient_editor,
         border_check, border_width_spin, border_color_widget)
    }

    fn create_labels_page(
        config: &Rc<RefCell<CoreBarsConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
    ) -> (GtkBox, CheckButton, Entry, DropDown, Button, SpinButton, Rc<ColorButtonWidget>, CheckButton) {
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
        let label_color_widget = Rc::new(ColorButtonWidget::new(config.borrow().label_color));
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
            let pos = match dd.selected() {
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
                let font_dialog = shared_font_dialog();
                gtk4::glib::MainContext::default().spawn_local(async move {
                    match font_dialog.choose_font_future(Some(&win), None::<&gtk4::pango::FontDescription>).await {
                        Ok(font_desc) => {
                            let family = font_desc.family().map(|f| f.to_string()).unwrap_or_else(|| "Sans".to_string());
                            config_clone2.borrow_mut().label_font = family.clone();
                            let size = size_spin.value();
                            font_btn.set_label(&format!("{} {:.0}", family, size));
                            if let Some(ref cb) = *on_change_clone2.borrow() {
                                cb();
                            }
                        }
                        Err(_) => {}
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
        label_color_widget.set_on_change(move |color| {
            config_clone.borrow_mut().label_color = color;
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

    fn render_checkerboard(cr: &gtk4::cairo::Context, width: f64, height: f64) {
        let checker_size = 10.0;
        for y in 0..(height / checker_size).ceil() as i32 {
            for x in 0..(width / checker_size).ceil() as i32 {
                if (x + y) % 2 == 0 {
                    cr.set_source_rgb(0.3, 0.3, 0.3);
                } else {
                    cr.set_source_rgb(0.2, 0.2, 0.2);
                }
                cr.rectangle(
                    x as f64 * checker_size,
                    y as f64 * checker_size,
                    checker_size,
                    checker_size,
                );
                let _ = cr.fill();
            }
        }
    }

    /// Get the container widget
    pub fn widget(&self) -> &GtkBox {
        &self.container
    }

    /// Get the current configuration
    pub fn get_config(&self) -> CoreBarsConfig {
        self.config.borrow().clone()
    }

    /// Set the configuration
    pub fn set_config(&self, config: CoreBarsConfig) {
        // Update UI elements
        self.start_core_spin.set_value(config.start_core as f64);
        self.end_core_spin.set_value(config.end_core as f64);

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
                self.fg_color_widget.set_color(*color);
            }
            BarFillType::Gradient { stops, angle } => {
                self.fg_gradient_editor.set_gradient(&crate::ui::background::LinearGradientConfig {
                    stops: stops.clone(),
                    angle: *angle,
                });
                self.fg_gradient_radio.set_active(true);
            }
        }

        match &config.background {
            BarBackgroundType::Solid { color } => {
                self.bg_solid_radio.set_active(true);
                self.bg_color_widget.set_color(*color);
            }
            BarBackgroundType::Gradient { stops, angle } => {
                self.bg_gradient_editor.set_gradient(&crate::ui::background::LinearGradientConfig {
                    stops: stops.clone(),
                    angle: *angle,
                });
                self.bg_gradient_radio.set_active(true);
            }
            BarBackgroundType::Transparent => {
                self.bg_transparent_radio.set_active(true);
            }
        }

        self.border_check.set_active(config.border.enabled);
        self.border_width_spin.set_value(config.border.width);
        self.border_color_widget.set_color(config.border.color);

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
        self.label_color_widget.set_color(config.label_color);
        self.label_bold_check.set_active(config.label_bold);

        // Animation
        self.animate_check.set_active(config.animate);
        self.animation_speed_scale.set_value(config.animation_speed);

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
}

impl Default for CoreBarsConfigWidget {
    fn default() -> Self {
        Self::new()
    }
}
