//! Speedometer gauge configuration widget

use gtk4::prelude::*;
use gtk4::{
    Adjustment, Box as GtkBox, Button, CheckButton, DrawingArea, DropDown, Label,
    Notebook, Orientation, Scale, SpinButton, StringList,
};
use std::cell::RefCell;
use std::rc::Rc;

use crate::ui::speedometer_display::{
    render_speedometer, NeedleStyle, NeedleTailStyle, SpeedometerConfig, TickStyle,
};
use crate::ui::color_button_widget::ColorButtonWidget;
use crate::ui::render_utils::render_checkerboard;
use crate::ui::GradientEditor;
use crate::displayers::FieldMetadata;
use crate::ui::text_line_config_widget::TextLineConfigWidget;

/// Speedometer gauge configuration widget
#[allow(dead_code)]
pub struct SpeedometerConfigWidget {
    container: GtkBox,
    config: Rc<RefCell<SpeedometerConfig>>,
    on_change: Rc<RefCell<Option<Box<dyn Fn()>>>>,
    preview: DrawingArea,

    // Geometry controls
    start_angle_spin: SpinButton,
    end_angle_spin: SpinButton,
    arc_width_scale: Scale,
    radius_scale: Scale,

    // Track controls
    show_track_check: CheckButton,
    track_color_widget: Rc<ColorButtonWidget>,
    gradient_editor: Rc<GradientEditor>,

    // Major ticks controls
    show_major_ticks_check: CheckButton,
    major_tick_count_spin: SpinButton,
    major_tick_length_scale: Scale,
    major_tick_width_spin: SpinButton,
    major_tick_color_widget: Rc<ColorButtonWidget>,
    major_tick_style_dropdown: DropDown,

    // Minor ticks controls
    show_minor_ticks_check: CheckButton,
    minor_ticks_per_major_spin: SpinButton,
    minor_tick_length_scale: Scale,
    minor_tick_width_spin: SpinButton,
    minor_tick_color_widget: Rc<ColorButtonWidget>,
    minor_tick_style_dropdown: DropDown,

    // Tick labels controls (using TickLabelConfig)
    show_tick_labels_check: CheckButton,
    tick_label_font_button: Button,
    tick_label_font_size_spin: SpinButton,
    tick_label_color_widget: Rc<ColorButtonWidget>,
    tick_label_bold_check: CheckButton,
    tick_label_italic_check: CheckButton,
    tick_label_use_percentage_check: CheckButton,

    // Needle controls
    show_needle_check: CheckButton,
    needle_style_dropdown: DropDown,
    needle_tail_style_dropdown: DropDown,
    needle_length_scale: Scale,
    needle_width_spin: SpinButton,
    needle_color_widget: Rc<ColorButtonWidget>,
    needle_shadow_check: CheckButton,

    // Center hub controls
    show_center_hub_check: CheckButton,
    center_hub_radius_scale: Scale,
    center_hub_color_widget: Rc<ColorButtonWidget>,
    center_hub_3d_check: CheckButton,

    // Bezel controls (using BackgroundConfig)
    show_bezel_check: CheckButton,
    bezel_width_scale: Scale,
    bezel_background_widget: Rc<crate::ui::BackgroundConfigWidget>,

    // Animation controls
    animate_check: CheckButton,
    animation_duration_spin: SpinButton,
    bounce_animation_check: CheckButton,

    // Text overlay
    enable_text_overlay_check: CheckButton,
    text_config_widget: Rc<TextLineConfigWidget>,
}

impl SpeedometerConfigWidget {
    pub fn new(available_fields: Vec<FieldMetadata>) -> Self {
        let container = GtkBox::new(Orientation::Vertical, 12);
        let config = Rc::new(RefCell::new(SpeedometerConfig::default()));
        let on_change: Rc<RefCell<Option<Box<dyn Fn()>>>> = Rc::new(RefCell::new(None));

        // Preview at the top
        let preview = DrawingArea::new();
        preview.set_content_height(250);
        preview.set_vexpand(false);

        let config_clone = config.clone();
        preview.set_draw_func(move |_, cr, width, height| {
            // Render checkerboard pattern to show transparency
            render_checkerboard(cr, width as f64, height as f64);

            let cfg = config_clone.borrow();
            let mut preview_values = std::collections::HashMap::new();
            preview_values.insert("value".to_string(), serde_json::json!(75.0));
            preview_values.insert("percent".to_string(), serde_json::json!(75.0));
            let _ = render_speedometer(cr, &cfg, 0.75, &preview_values, width as f64, height as f64);
        });

        // Create notebook for tabbed interface
        let notebook = Notebook::new();
        notebook.set_vexpand(true);

        // === Tab 1: Geometry ===
        let (geom_page, start_angle_spin, end_angle_spin, arc_width_scale, radius_scale) =
            Self::create_geometry_page(&config, &on_change, &preview);
        notebook.append_page(&geom_page, Some(&Label::new(Some("Geometry"))));

        // === Tab 2: Track ===
        let (track_page, show_track_check, track_color_widget, gradient_editor) =
            Self::create_track_page(&config, &on_change, &preview);
        notebook.append_page(&track_page, Some(&Label::new(Some("Track"))));

        // === Tab 3: Ticks ===
        let (ticks_page, show_major_ticks_check, major_tick_count_spin, major_tick_length_scale,
             major_tick_width_spin, major_tick_color_button, major_tick_style_dropdown,
             show_minor_ticks_check, minor_ticks_per_major_spin, minor_tick_length_scale,
             minor_tick_width_spin, minor_tick_color_button, minor_tick_style_dropdown,
             show_tick_labels_check, tick_label_font_button, tick_label_font_size_spin,
             tick_label_color_button, tick_label_bold_check, tick_label_italic_check,
             tick_label_use_percentage_check) =
            Self::create_ticks_page(&config, &on_change, &preview);
        notebook.append_page(&ticks_page, Some(&Label::new(Some("Ticks"))));

        // === Tab 4: Needle ===
        let (needle_page, show_needle_check, needle_style_dropdown, needle_tail_style_dropdown,
             needle_length_scale, needle_width_spin, needle_color_button, needle_shadow_check) =
            Self::create_needle_page(&config, &on_change, &preview);
        notebook.append_page(&needle_page, Some(&Label::new(Some("Needle"))));

        // === Tab 5: Bezel & Hub ===
        let (bezel_page, show_center_hub_check, center_hub_radius_scale, center_hub_color_button,
             center_hub_3d_check, show_bezel_check, bezel_width_scale, bezel_background_widget) =
            Self::create_bezel_page(&config, &on_change, &preview);
        notebook.append_page(&bezel_page, Some(&Label::new(Some("Bezel & Hub"))));

        // === Tab 6: Animation ===
        let (animation_page, animate_check, animation_duration_spin, bounce_animation_check) =
            Self::create_animation_page(&config, &on_change, &preview);
        notebook.append_page(&animation_page, Some(&Label::new(Some("Animation"))));

        // === Tab 7: Text Overlay ===
        let (text_page, enable_text_overlay_check, text_config_widget) =
            Self::create_text_overlay_page(&config, &on_change, &preview, available_fields);
        notebook.append_page(&text_page, Some(&Label::new(Some("Text Overlay"))));

        container.append(&preview);
        container.append(&notebook);

        Self {
            container,
            config,
            on_change,
            preview,
            start_angle_spin,
            end_angle_spin,
            arc_width_scale,
            radius_scale,
            show_track_check,
            track_color_widget,
            gradient_editor,
            show_major_ticks_check,
            major_tick_count_spin,
            major_tick_length_scale,
            major_tick_width_spin,
            major_tick_color_widget: major_tick_color_button,
            major_tick_style_dropdown,
            show_minor_ticks_check,
            minor_ticks_per_major_spin,
            minor_tick_length_scale,
            minor_tick_width_spin,
            minor_tick_color_widget: minor_tick_color_button,
            minor_tick_style_dropdown,
            show_tick_labels_check,
            tick_label_font_button,
            tick_label_font_size_spin,
            tick_label_color_widget: tick_label_color_button,
            tick_label_bold_check,
            tick_label_italic_check,
            tick_label_use_percentage_check,
            show_needle_check,
            needle_style_dropdown,
            needle_tail_style_dropdown,
            needle_length_scale,
            needle_width_spin,
            needle_color_widget: needle_color_button,
            needle_shadow_check,
            show_center_hub_check,
            center_hub_radius_scale,
            center_hub_color_widget: center_hub_color_button,
            center_hub_3d_check,
            show_bezel_check,
            bezel_width_scale,
            bezel_background_widget,
            animate_check,
            animation_duration_spin,
            bounce_animation_check,
            enable_text_overlay_check,
            text_config_widget,
        }
    }

    fn queue_preview_redraw(preview: &DrawingArea, on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>) {
        preview.queue_draw();
        if let Some(callback) = on_change.borrow().as_ref() {
            callback();
        }
    }

    fn create_geometry_page(
        config: &Rc<RefCell<SpeedometerConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
    ) -> (GtkBox, SpinButton, SpinButton, Scale, Scale) {
        let page = GtkBox::new(Orientation::Vertical, 12);
        page.set_margin_start(12);
        page.set_margin_end(12);
        page.set_margin_top(12);
        page.set_margin_bottom(12);

        // Start angle
        let start_angle_box = GtkBox::new(Orientation::Horizontal, 6);
        start_angle_box.append(&Label::new(Some("Start Angle (°):")));
        let start_angle_adj = Adjustment::new(135.0, -360.0, 360.0, 1.0, 10.0, 0.0);
        let start_angle_spin = SpinButton::new(Some(&start_angle_adj), 1.0, 1);
        start_angle_spin.set_hexpand(true);
        start_angle_box.append(&start_angle_spin);
        page.append(&start_angle_box);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        start_angle_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().start_angle = spin.value();
            Self::queue_preview_redraw(&preview_clone, &on_change_clone);
        });

        // End angle
        let end_angle_box = GtkBox::new(Orientation::Horizontal, 6);
        end_angle_box.append(&Label::new(Some("End Angle (°):")));
        let end_angle_adj = Adjustment::new(45.0, -360.0, 360.0, 1.0, 10.0, 0.0);
        let end_angle_spin = SpinButton::new(Some(&end_angle_adj), 1.0, 1);
        end_angle_spin.set_hexpand(true);
        end_angle_box.append(&end_angle_spin);
        page.append(&end_angle_box);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        end_angle_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().end_angle = spin.value();
            Self::queue_preview_redraw(&preview_clone, &on_change_clone);
        });

        // Arc width
        let arc_width_box = GtkBox::new(Orientation::Vertical, 6);
        arc_width_box.append(&Label::new(Some("Arc Width:")));
        let arc_width_scale = Scale::with_range(Orientation::Horizontal, 0.05, 0.5, 0.01);
        arc_width_scale.set_value(0.15);
        arc_width_scale.set_draw_value(true);
        arc_width_box.append(&arc_width_scale);
        page.append(&arc_width_box);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        arc_width_scale.connect_value_changed(move |scale| {
            config_clone.borrow_mut().arc_width = scale.value();
            Self::queue_preview_redraw(&preview_clone, &on_change_clone);
        });

        // Radius
        let radius_box = GtkBox::new(Orientation::Vertical, 6);
        radius_box.append(&Label::new(Some("Radius (% of space):")));
        let radius_scale = Scale::with_range(Orientation::Horizontal, 0.5, 1.0, 0.01);
        radius_scale.set_value(0.85);
        radius_scale.set_draw_value(true);
        radius_box.append(&radius_scale);
        page.append(&radius_box);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        radius_scale.connect_value_changed(move |scale| {
            config_clone.borrow_mut().radius_percent = scale.value();
            Self::queue_preview_redraw(&preview_clone, &on_change_clone);
        });

        (
            page,
            start_angle_spin,
            end_angle_spin,
            arc_width_scale,
            radius_scale,
        )
    }

    fn create_track_page(
        config: &Rc<RefCell<SpeedometerConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
    ) -> (GtkBox, CheckButton, Rc<ColorButtonWidget>, Rc<GradientEditor>) {
        let page = GtkBox::new(Orientation::Vertical, 12);
        page.set_margin_start(12);
        page.set_margin_end(12);
        page.set_margin_top(12);
        page.set_margin_bottom(12);

        // Show track checkbox
        let show_track_check = CheckButton::with_label("Show Track");
        show_track_check.set_active(true);
        page.append(&show_track_check);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        show_track_check.connect_toggled(move |check| {
            config_clone.borrow_mut().show_track = check.is_active();
            Self::queue_preview_redraw(&preview_clone, &on_change_clone);
        });

        // Track color - using ColorButtonWidget
        let track_color_box = GtkBox::new(Orientation::Horizontal, 6);
        track_color_box.append(&Label::new(Some("Track Base Color:")));
        let track_color_widget = Rc::new(ColorButtonWidget::new(config.borrow().track_color));
        track_color_box.append(track_color_widget.widget());
        page.append(&track_color_box);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        track_color_widget.set_on_change(move |color| {
            config_clone.borrow_mut().track_color = color;
            Self::queue_preview_redraw(&preview_clone, &on_change_clone);
        });

        // Gradient editor for track color zones
        let gradient_editor = Rc::new(GradientEditor::new());

        // Copy/Paste gradient buttons (above the gradient editor)
        let copy_paste_box = GtkBox::new(Orientation::Horizontal, 6);
        let copy_gradient_btn = Button::with_label("Copy Gradient");
        let paste_gradient_btn = Button::with_label("Paste Gradient");

        let config_for_copy = config.clone();
        copy_gradient_btn.connect_clicked(move |_| {
            use crate::ui::CLIPBOARD;

            let cfg = config_for_copy.borrow();
            if let Ok(mut clipboard) = CLIPBOARD.lock() {
                clipboard.copy_gradient_stops(cfg.track_color_stops.clone());
                log::info!("Speedometer track gradient color stops copied to clipboard");
            }
        });

        let config_for_paste = config.clone();
        let preview_for_paste = preview.clone();
        let on_change_for_paste = on_change.clone();
        let gradient_editor_for_paste = gradient_editor.clone();
        paste_gradient_btn.connect_clicked(move |_| {
            use crate::ui::CLIPBOARD;

            if let Ok(clipboard) = CLIPBOARD.lock() {
                if let Some(stops) = clipboard.paste_gradient_stops() {
                    config_for_paste.borrow_mut().track_color_stops = stops.clone();

                    // Update the gradient editor widget
                    let gradient_config = crate::ui::LinearGradientConfig {
                        angle: 0.0,
                        stops,
                    };
                    gradient_editor_for_paste.set_gradient(&gradient_config);

                    Self::queue_preview_redraw(&preview_for_paste, &on_change_for_paste);
                    log::info!("Speedometer track gradient color stops pasted from clipboard");
                }
            }
        });

        copy_paste_box.append(&copy_gradient_btn);
        copy_paste_box.append(&paste_gradient_btn);
        page.append(&copy_paste_box);

        // Append gradient editor after copy/paste buttons
        page.append(gradient_editor.widget());

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let gradient_editor_clone = gradient_editor.clone();
        gradient_editor.set_on_change(move || {
            let gradient = gradient_editor_clone.get_gradient();
            config_clone.borrow_mut().track_color_stops = gradient.stops;
            Self::queue_preview_redraw(&preview_clone, &on_change_clone);
        });

        (page, show_track_check, track_color_widget, gradient_editor)
    }

    fn create_ticks_page(
        config: &Rc<RefCell<SpeedometerConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
    ) -> (
        GtkBox,
        CheckButton,
        SpinButton,
        Scale,
        SpinButton,
        Rc<ColorButtonWidget>,
        DropDown,
        CheckButton,
        SpinButton,
        Scale,
        SpinButton,
        Rc<ColorButtonWidget>,
        DropDown,
        CheckButton,
        Button,
        SpinButton,
        Rc<ColorButtonWidget>,
        CheckButton,
        CheckButton,
        CheckButton,
    ) {
        let page = GtkBox::new(Orientation::Vertical, 12);
        page.set_margin_start(12);
        page.set_margin_end(12);
        page.set_margin_top(12);
        page.set_margin_bottom(12);

        // === Major Ticks Section ===
        let major_label = Label::new(Some("Major Ticks"));
        major_label.add_css_class("heading");
        page.append(&major_label);

        let show_major_ticks_check = CheckButton::with_label("Show Major Ticks");
        show_major_ticks_check.set_active(true);
        page.append(&show_major_ticks_check);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        show_major_ticks_check.connect_toggled(move |check| {
            config_clone.borrow_mut().show_major_ticks = check.is_active();
            Self::queue_preview_redraw(&preview_clone, &on_change_clone);
        });

        // Major tick count
        let major_count_box = GtkBox::new(Orientation::Horizontal, 6);
        major_count_box.append(&Label::new(Some("Tick Count:")));
        let major_count_adj = Adjustment::new(10.0, 2.0, 50.0, 1.0, 5.0, 0.0);
        let major_tick_count_spin = SpinButton::new(Some(&major_count_adj), 1.0, 0);
        major_tick_count_spin.set_hexpand(true);
        major_count_box.append(&major_tick_count_spin);
        page.append(&major_count_box);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        major_tick_count_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().major_tick_count = spin.value() as u32;
            Self::queue_preview_redraw(&preview_clone, &on_change_clone);
        });

        // Major tick length
        let major_length_box = GtkBox::new(Orientation::Vertical, 6);
        major_length_box.append(&Label::new(Some("Tick Length (% of arc width):")));
        let major_tick_length_scale = Scale::with_range(Orientation::Horizontal, 0.05, 0.5, 0.01);
        major_tick_length_scale.set_value(0.15);
        major_tick_length_scale.set_draw_value(true);
        major_length_box.append(&major_tick_length_scale);
        page.append(&major_length_box);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        major_tick_length_scale.connect_value_changed(move |scale| {
            config_clone.borrow_mut().major_tick_length = scale.value();
            Self::queue_preview_redraw(&preview_clone, &on_change_clone);
        });

        // Major tick width
        let major_width_box = GtkBox::new(Orientation::Horizontal, 6);
        major_width_box.append(&Label::new(Some("Tick Width (px):")));
        let major_width_adj = Adjustment::new(2.0, 0.5, 10.0, 0.5, 1.0, 0.0);
        let major_tick_width_spin = SpinButton::new(Some(&major_width_adj), 0.5, 1);
        major_tick_width_spin.set_hexpand(true);
        major_width_box.append(&major_tick_width_spin);
        page.append(&major_width_box);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        major_tick_width_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().major_tick_width = spin.value();
            Self::queue_preview_redraw(&preview_clone, &on_change_clone);
        });

        // Major tick color - using ColorButtonWidget
        let major_tick_color_box = GtkBox::new(Orientation::Horizontal, 6);
        major_tick_color_box.append(&Label::new(Some("Color:")));
        let major_tick_color_button = Rc::new(ColorButtonWidget::new(config.borrow().major_tick_color));
        major_tick_color_box.append(major_tick_color_button.widget());
        page.append(&major_tick_color_box);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        major_tick_color_button.set_on_change(move |color| {
            config_clone.borrow_mut().major_tick_color = color;
            Self::queue_preview_redraw(&preview_clone, &on_change_clone);
        });

        // Major tick style
        let major_style_box = GtkBox::new(Orientation::Horizontal, 6);
        major_style_box.append(&Label::new(Some("Style:")));
        let major_styles = StringList::new(&["Line", "Wedge", "Dot"]);
        let major_tick_style_dropdown = DropDown::new(Some(major_styles), Option::<gtk4::Expression>::None);
        major_tick_style_dropdown.set_selected(0);
        major_style_box.append(&major_tick_style_dropdown);
        page.append(&major_style_box);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        major_tick_style_dropdown.connect_selected_notify(move |dropdown| {
            let style = match dropdown.selected() {
                0 => TickStyle::Line,
                1 => TickStyle::Wedge,
                2 => TickStyle::Dot,
                _ => TickStyle::Line,
            };
            config_clone.borrow_mut().major_tick_style = style;
            Self::queue_preview_redraw(&preview_clone, &on_change_clone);
        });

        // === Minor Ticks Section ===
        let minor_label = Label::new(Some("Minor Ticks"));
        minor_label.add_css_class("heading");
        page.append(&minor_label);

        let show_minor_ticks_check = CheckButton::with_label("Show Minor Ticks");
        show_minor_ticks_check.set_active(true);
        page.append(&show_minor_ticks_check);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        show_minor_ticks_check.connect_toggled(move |check| {
            config_clone.borrow_mut().show_minor_ticks = check.is_active();
            Self::queue_preview_redraw(&preview_clone, &on_change_clone);
        });

        // Minor ticks per major
        let minor_count_box = GtkBox::new(Orientation::Horizontal, 6);
        minor_count_box.append(&Label::new(Some("Ticks per Major:")));
        let minor_count_adj = Adjustment::new(5.0, 1.0, 10.0, 1.0, 2.0, 0.0);
        let minor_ticks_per_major_spin = SpinButton::new(Some(&minor_count_adj), 1.0, 0);
        minor_ticks_per_major_spin.set_hexpand(true);
        minor_count_box.append(&minor_ticks_per_major_spin);
        page.append(&minor_count_box);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        minor_ticks_per_major_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().minor_ticks_per_major = spin.value() as u32;
            Self::queue_preview_redraw(&preview_clone, &on_change_clone);
        });

        // Minor tick length
        let minor_length_box = GtkBox::new(Orientation::Vertical, 6);
        minor_length_box.append(&Label::new(Some("Tick Length (% of arc width):")));
        let minor_tick_length_scale = Scale::with_range(Orientation::Horizontal, 0.05, 0.3, 0.01);
        minor_tick_length_scale.set_value(0.08);
        minor_tick_length_scale.set_draw_value(true);
        minor_length_box.append(&minor_tick_length_scale);
        page.append(&minor_length_box);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        minor_tick_length_scale.connect_value_changed(move |scale| {
            config_clone.borrow_mut().minor_tick_length = scale.value();
            Self::queue_preview_redraw(&preview_clone, &on_change_clone);
        });

        // Minor tick width
        let minor_width_box = GtkBox::new(Orientation::Horizontal, 6);
        minor_width_box.append(&Label::new(Some("Tick Width (px):")));
        let minor_width_adj = Adjustment::new(1.0, 0.5, 5.0, 0.5, 1.0, 0.0);
        let minor_tick_width_spin = SpinButton::new(Some(&minor_width_adj), 0.5, 1);
        minor_tick_width_spin.set_hexpand(true);
        minor_width_box.append(&minor_tick_width_spin);
        page.append(&minor_width_box);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        minor_tick_width_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().minor_tick_width = spin.value();
            Self::queue_preview_redraw(&preview_clone, &on_change_clone);
        });

        // Minor tick color - using ColorButtonWidget
        let minor_tick_color_box = GtkBox::new(Orientation::Horizontal, 6);
        minor_tick_color_box.append(&Label::new(Some("Color:")));
        let minor_tick_color_button = Rc::new(ColorButtonWidget::new(config.borrow().minor_tick_color));
        minor_tick_color_box.append(minor_tick_color_button.widget());
        page.append(&minor_tick_color_box);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        minor_tick_color_button.set_on_change(move |color| {
            config_clone.borrow_mut().minor_tick_color = color;
            Self::queue_preview_redraw(&preview_clone, &on_change_clone);
        });

        // Minor tick style
        let minor_style_box = GtkBox::new(Orientation::Horizontal, 6);
        minor_style_box.append(&Label::new(Some("Style:")));
        let minor_styles = StringList::new(&["Line", "Wedge", "Dot"]);
        let minor_tick_style_dropdown = DropDown::new(Some(minor_styles), Option::<gtk4::Expression>::None);
        minor_tick_style_dropdown.set_selected(0);
        minor_style_box.append(&minor_tick_style_dropdown);
        page.append(&minor_style_box);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        minor_tick_style_dropdown.connect_selected_notify(move |dropdown| {
            let style = match dropdown.selected() {
                0 => TickStyle::Line,
                1 => TickStyle::Wedge,
                2 => TickStyle::Dot,
                _ => TickStyle::Line,
            };
            config_clone.borrow_mut().minor_tick_style = style;
            Self::queue_preview_redraw(&preview_clone, &on_change_clone);
        });

        // === Tick Labels Section ===
        let labels_label = Label::new(Some("Tick Labels"));
        labels_label.add_css_class("heading");
        page.append(&labels_label);

        let show_tick_labels_check = CheckButton::with_label("Show Tick Labels");
        show_tick_labels_check.set_active(true);
        page.append(&show_tick_labels_check);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        show_tick_labels_check.connect_toggled(move |check| {
            config_clone.borrow_mut().show_tick_labels = check.is_active();
            Self::queue_preview_redraw(&preview_clone, &on_change_clone);
        });

        // Font controls (using shared font dialog pattern)
        let font_box = GtkBox::new(Orientation::Horizontal, 6);
        font_box.append(&Label::new(Some("Font:")));

        // Font selection button
        let initial_font_label = format!("{} {:.0}",
            config.borrow().tick_label_config.font_family,
            config.borrow().tick_label_config.font_size
        );
        let tick_label_font_button = Button::with_label(&initial_font_label);
        tick_label_font_button.set_hexpand(true);
        font_box.append(&tick_label_font_button);

        // Font size spinner
        font_box.append(&Label::new(Some("Size:")));
        let tick_label_font_size_spin = SpinButton::with_range(6.0, 200.0, 1.0);
        tick_label_font_size_spin.set_value(config.borrow().tick_label_config.font_size);
        tick_label_font_size_spin.set_width_chars(4);
        font_box.append(&tick_label_font_size_spin);

        // Update font size when spinner changes
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let font_button_clone = tick_label_font_button.clone();
        tick_label_font_size_spin.connect_value_changed(move |spin| {
            let new_size = spin.value();
            config_clone.borrow_mut().tick_label_config.font_size = new_size;
            // Update button label
            let family = config_clone.borrow().tick_label_config.font_family.clone();
            font_button_clone.set_label(&format!("{} {:.0}", family, new_size));
            Self::queue_preview_redraw(&preview_clone, &on_change_clone);
        });

        // Copy font button
        let copy_font_btn = Button::with_label("Copy");
        let config_clone_copy = config.clone();
        copy_font_btn.connect_clicked(move |_| {
            let cfg = config_clone_copy.borrow();
            if let Ok(mut clipboard) = crate::ui::clipboard::CLIPBOARD.lock() {
                clipboard.copy_font(
                    cfg.tick_label_config.font_family.clone(),
                    cfg.tick_label_config.font_size,
                    cfg.tick_label_config.bold,
                    cfg.tick_label_config.italic,
                );
            }
        });
        font_box.append(&copy_font_btn);

        // Paste font button
        let paste_font_btn = Button::with_label("Paste");
        let config_clone_paste = config.clone();
        let on_change_clone_paste = on_change.clone();
        let preview_clone_paste = preview.clone();
        let font_button_clone_paste = tick_label_font_button.clone();
        let size_spin_clone = tick_label_font_size_spin.clone();
        paste_font_btn.connect_clicked(move |_| {
            if let Ok(clipboard) = crate::ui::clipboard::CLIPBOARD.lock() {
                if let Some((family, size, bold, italic)) = clipboard.paste_font() {
                    let mut cfg = config_clone_paste.borrow_mut();
                    cfg.tick_label_config.font_family = family.clone();
                    cfg.tick_label_config.font_size = size;
                    cfg.tick_label_config.bold = bold;
                    cfg.tick_label_config.italic = italic;
                    drop(cfg);
                    // Update button label and size spinner
                    font_button_clone_paste.set_label(&format!("{} {:.0}", family, size));
                    size_spin_clone.set_value(size);
                    Self::queue_preview_redraw(&preview_clone_paste, &on_change_clone_paste);
                }
            }
        });
        font_box.append(&paste_font_btn);

        page.append(&font_box);

        // Font button click handler - opens font dialog
        let config_clone_font = config.clone();
        let on_change_clone_font = on_change.clone();
        let preview_clone_font = preview.clone();
        let font_button_clone_font = tick_label_font_button.clone();
        let size_spin_clone_font = tick_label_font_size_spin.clone();
        tick_label_font_button.connect_clicked(move |btn| {
            let window = btn.root().and_then(|root| root.downcast::<gtk4::Window>().ok());

            // Get current font description
            let current_font = {
                let cfg = config_clone_font.borrow();
                let font_str = format!("{} {}", cfg.tick_label_config.font_family, cfg.tick_label_config.font_size as i32);
                gtk4::pango::FontDescription::from_string(&font_str)
            };

            let config_clone2 = config_clone_font.clone();
            let on_change_clone2 = on_change_clone_font.clone();
            let preview_clone2 = preview_clone_font.clone();
            let font_button_clone2 = font_button_clone_font.clone();
            let size_spin_clone2 = size_spin_clone_font.clone();

            // Use callback-based API for font selection with shared dialog
            crate::ui::shared_font_dialog::shared_font_dialog().choose_font(
                window.as_ref(),
                Some(&current_font),
                gtk4::gio::Cancellable::NONE,
                move |result| {
                    if let Ok(font_desc) = result {
                        // Extract family and size from font description
                        let family = font_desc.family().map(|s| s.to_string()).unwrap_or_else(|| "Sans".to_string());
                        let size = font_desc.size() as f64 / gtk4::pango::SCALE as f64;

                        config_clone2.borrow_mut().tick_label_config.font_family = family.clone();
                        config_clone2.borrow_mut().tick_label_config.font_size = size;

                        // Update button label and size spinner
                        font_button_clone2.set_label(&format!("{} {:.0}", family, size));
                        size_spin_clone2.set_value(size);
                        Self::queue_preview_redraw(&preview_clone2, &on_change_clone2);
                    }
                },
            );
        });

        // Label color - using ColorButtonWidget
        let tick_label_color_box = GtkBox::new(Orientation::Horizontal, 6);
        tick_label_color_box.append(&Label::new(Some("Color:")));
        let tick_label_color_button = Rc::new(ColorButtonWidget::new(config.borrow().tick_label_config.color));
        tick_label_color_box.append(tick_label_color_button.widget());
        page.append(&tick_label_color_box);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        tick_label_color_button.set_on_change(move |color| {
            config_clone.borrow_mut().tick_label_config.color = color;
            Self::queue_preview_redraw(&preview_clone, &on_change_clone);
        });

        // Bold checkbox
        let tick_label_bold_check = CheckButton::with_label("Bold");
        tick_label_bold_check.set_active(false);
        page.append(&tick_label_bold_check);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        tick_label_bold_check.connect_toggled(move |check| {
            config_clone.borrow_mut().tick_label_config.bold = check.is_active();
            Self::queue_preview_redraw(&preview_clone, &on_change_clone);
        });

        // Italic checkbox
        let tick_label_italic_check = CheckButton::with_label("Italic");
        tick_label_italic_check.set_active(false);
        page.append(&tick_label_italic_check);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        tick_label_italic_check.connect_toggled(move |check| {
            config_clone.borrow_mut().tick_label_config.italic = check.is_active();
            Self::queue_preview_redraw(&preview_clone, &on_change_clone);
        });

        // Use percentage
        let tick_label_use_percentage_check = CheckButton::with_label("Show as 0-100% instead of actual values");
        tick_label_use_percentage_check.set_active(false);
        page.append(&tick_label_use_percentage_check);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        tick_label_use_percentage_check.connect_toggled(move |check| {
            config_clone.borrow_mut().tick_label_config.use_percentage = check.is_active();
            Self::queue_preview_redraw(&preview_clone, &on_change_clone);
        });

        (
            page,
            show_major_ticks_check,
            major_tick_count_spin,
            major_tick_length_scale,
            major_tick_width_spin,
            major_tick_color_button,
            major_tick_style_dropdown,
            show_minor_ticks_check,
            minor_ticks_per_major_spin,
            minor_tick_length_scale,
            minor_tick_width_spin,
            minor_tick_color_button,
            minor_tick_style_dropdown,
            show_tick_labels_check,
            tick_label_font_button,
            tick_label_font_size_spin,
            tick_label_color_button,
            tick_label_bold_check,
            tick_label_italic_check,
            tick_label_use_percentage_check,
        )
    }

    fn create_needle_page(
        config: &Rc<RefCell<SpeedometerConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
    ) -> (
        GtkBox,
        CheckButton,
        DropDown,
        DropDown,
        Scale,
        SpinButton,
        Rc<ColorButtonWidget>,
        CheckButton,
    ) {
        let page = GtkBox::new(Orientation::Vertical, 12);
        page.set_margin_start(12);
        page.set_margin_end(12);
        page.set_margin_top(12);
        page.set_margin_bottom(12);

        // Show needle
        let show_needle_check = CheckButton::with_label("Show Needle");
        show_needle_check.set_active(true);
        page.append(&show_needle_check);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        show_needle_check.connect_toggled(move |check| {
            config_clone.borrow_mut().show_needle = check.is_active();
            Self::queue_preview_redraw(&preview_clone, &on_change_clone);
        });

        // Needle style
        let style_box = GtkBox::new(Orientation::Horizontal, 6);
        style_box.append(&Label::new(Some("Needle Style:")));
        let styles = StringList::new(&["Arrow", "Line", "Tapered", "Triangle"]);
        let needle_style_dropdown = DropDown::new(Some(styles), Option::<gtk4::Expression>::None);
        needle_style_dropdown.set_selected(0);
        style_box.append(&needle_style_dropdown);
        page.append(&style_box);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        needle_style_dropdown.connect_selected_notify(move |dropdown| {
            let style = match dropdown.selected() {
                0 => NeedleStyle::Arrow,
                1 => NeedleStyle::Line,
                2 => NeedleStyle::Tapered,
                3 => NeedleStyle::Triangle,
                _ => NeedleStyle::Arrow,
            };
            config_clone.borrow_mut().needle_style = style;
            Self::queue_preview_redraw(&preview_clone, &on_change_clone);
        });

        // Needle tail style
        let tail_box = GtkBox::new(Orientation::Horizontal, 6);
        tail_box.append(&Label::new(Some("Needle Tail:")));
        let tails = StringList::new(&["None", "Short", "Balanced"]);
        let needle_tail_style_dropdown = DropDown::new(Some(tails), Option::<gtk4::Expression>::None);
        needle_tail_style_dropdown.set_selected(1);
        tail_box.append(&needle_tail_style_dropdown);
        page.append(&tail_box);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        needle_tail_style_dropdown.connect_selected_notify(move |dropdown| {
            let style = match dropdown.selected() {
                0 => NeedleTailStyle::None,
                1 => NeedleTailStyle::Short,
                2 => NeedleTailStyle::Balanced,
                _ => NeedleTailStyle::Short,
            };
            config_clone.borrow_mut().needle_tail_style = style;
            Self::queue_preview_redraw(&preview_clone, &on_change_clone);
        });

        // Needle length
        let length_box = GtkBox::new(Orientation::Vertical, 6);
        length_box.append(&Label::new(Some("Needle Length (% of radius):")));
        let needle_length_scale = Scale::with_range(Orientation::Horizontal, 0.5, 1.0, 0.01);
        needle_length_scale.set_value(0.85);
        needle_length_scale.set_draw_value(true);
        length_box.append(&needle_length_scale);
        page.append(&length_box);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        needle_length_scale.connect_value_changed(move |scale| {
            config_clone.borrow_mut().needle_length = scale.value();
            Self::queue_preview_redraw(&preview_clone, &on_change_clone);
        });

        // Needle width
        let width_box = GtkBox::new(Orientation::Horizontal, 6);
        width_box.append(&Label::new(Some("Needle Width (px):")));
        let width_adj = Adjustment::new(3.0, 1.0, 20.0, 0.5, 1.0, 0.0);
        let needle_width_spin = SpinButton::new(Some(&width_adj), 0.5, 1);
        needle_width_spin.set_hexpand(true);
        width_box.append(&needle_width_spin);
        page.append(&width_box);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        needle_width_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().needle_width = spin.value();
            Self::queue_preview_redraw(&preview_clone, &on_change_clone);
        });

        // Needle color - using ColorButtonWidget
        let needle_color_box = GtkBox::new(Orientation::Horizontal, 6);
        needle_color_box.append(&Label::new(Some("Needle Color:")));
        let needle_color_button = Rc::new(ColorButtonWidget::new(config.borrow().needle_color));
        needle_color_box.append(needle_color_button.widget());
        page.append(&needle_color_box);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        needle_color_button.set_on_change(move |color| {
            config_clone.borrow_mut().needle_color = color;
            Self::queue_preview_redraw(&preview_clone, &on_change_clone);
        });

        // Needle shadow
        let needle_shadow_check = CheckButton::with_label("Add Shadow");
        needle_shadow_check.set_active(false);
        page.append(&needle_shadow_check);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        needle_shadow_check.connect_toggled(move |check| {
            config_clone.borrow_mut().needle_shadow = check.is_active();
            Self::queue_preview_redraw(&preview_clone, &on_change_clone);
        });

        (
            page,
            show_needle_check,
            needle_style_dropdown,
            needle_tail_style_dropdown,
            needle_length_scale,
            needle_width_spin,
            needle_color_button,
            needle_shadow_check,
        )
    }

    fn create_bezel_page(
        config: &Rc<RefCell<SpeedometerConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
    ) -> (
        GtkBox,
        CheckButton,
        Scale,
        Rc<ColorButtonWidget>,
        CheckButton,
        CheckButton,
        Scale,
        Rc<crate::ui::BackgroundConfigWidget>,
    ) {
        let page = GtkBox::new(Orientation::Vertical, 12);
        page.set_margin_start(12);
        page.set_margin_end(12);
        page.set_margin_top(12);
        page.set_margin_bottom(12);

        // === Center Hub Section ===
        let hub_label = Label::new(Some("Center Hub"));
        hub_label.add_css_class("heading");
        page.append(&hub_label);

        let show_center_hub_check = CheckButton::with_label("Show Center Hub");
        show_center_hub_check.set_active(true);
        page.append(&show_center_hub_check);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        show_center_hub_check.connect_toggled(move |check| {
            config_clone.borrow_mut().show_center_hub = check.is_active();
            Self::queue_preview_redraw(&preview_clone, &on_change_clone);
        });

        // Hub radius
        let hub_radius_box = GtkBox::new(Orientation::Vertical, 6);
        hub_radius_box.append(&Label::new(Some("Hub Radius (% of radius):")));
        let center_hub_radius_scale = Scale::with_range(Orientation::Horizontal, 0.02, 0.15, 0.01);
        center_hub_radius_scale.set_value(0.06);
        center_hub_radius_scale.set_draw_value(true);
        hub_radius_box.append(&center_hub_radius_scale);
        page.append(&hub_radius_box);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        center_hub_radius_scale.connect_value_changed(move |scale| {
            config_clone.borrow_mut().center_hub_radius = scale.value();
            Self::queue_preview_redraw(&preview_clone, &on_change_clone);
        });

        // Hub color - using ColorButtonWidget
        let center_hub_color_box = GtkBox::new(Orientation::Horizontal, 6);
        center_hub_color_box.append(&Label::new(Some("Hub Color:")));
        let center_hub_color_button = Rc::new(ColorButtonWidget::new(config.borrow().center_hub_color));
        center_hub_color_box.append(center_hub_color_button.widget());
        page.append(&center_hub_color_box);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        center_hub_color_button.set_on_change(move |color| {
            config_clone.borrow_mut().center_hub_color = color;
            Self::queue_preview_redraw(&preview_clone, &on_change_clone);
        });

        // Hub 3D effect
        let center_hub_3d_check = CheckButton::with_label("3D Effect");
        center_hub_3d_check.set_active(false);
        page.append(&center_hub_3d_check);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        center_hub_3d_check.connect_toggled(move |check| {
            config_clone.borrow_mut().center_hub_3d = check.is_active();
            Self::queue_preview_redraw(&preview_clone, &on_change_clone);
        });

        // === Bezel Section ===
        let bezel_label = Label::new(Some("Bezel"));
        bezel_label.add_css_class("heading");
        page.append(&bezel_label);

        // Show bezel
        let show_bezel_check = CheckButton::with_label("Show Bezel");
        show_bezel_check.set_active(true);
        page.append(&show_bezel_check);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        show_bezel_check.connect_toggled(move |check| {
            config_clone.borrow_mut().show_bezel = check.is_active();
            Self::queue_preview_redraw(&preview_clone, &on_change_clone);
        });

        // Bezel width (0-100% of available space)
        let bezel_width_box = GtkBox::new(Orientation::Vertical, 6);
        bezel_width_box.append(&Label::new(Some("Bezel Width (% of available space, 0-100%):")));
        let bezel_width_scale = Scale::with_range(Orientation::Horizontal, 0.0, 1.0, 0.01);
        bezel_width_scale.set_value(0.05);
        bezel_width_scale.set_draw_value(true);
        bezel_width_box.append(&bezel_width_scale);
        page.append(&bezel_width_box);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        bezel_width_scale.connect_value_changed(move |scale| {
            config_clone.borrow_mut().bezel_width = scale.value();
            Self::queue_preview_redraw(&preview_clone, &on_change_clone);
        });

        // Bezel background (full background config widget)
        let bezel_background_label = Label::new(Some("Bezel Background:"));
        page.append(&bezel_background_label);

        let bezel_background_widget = Rc::new(crate::ui::BackgroundConfigWidget::new());
        page.append(bezel_background_widget.widget());

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let bezel_background_widget_clone = bezel_background_widget.clone();
        bezel_background_widget.set_on_change(Box::new(move || {
            config_clone.borrow_mut().bezel_background = bezel_background_widget_clone.get_config();
            Self::queue_preview_redraw(&preview_clone, &on_change_clone);
        }));

        (
            page,
            show_center_hub_check,
            center_hub_radius_scale,
            center_hub_color_button,
            center_hub_3d_check,
            show_bezel_check,
            bezel_width_scale,
            bezel_background_widget,
        )
    }

    fn create_animation_page(
        config: &Rc<RefCell<SpeedometerConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
    ) -> (GtkBox, CheckButton, SpinButton, CheckButton) {
        let page = GtkBox::new(Orientation::Vertical, 12);
        page.set_margin_start(12);
        page.set_margin_end(12);
        page.set_margin_top(12);
        page.set_margin_bottom(12);

        // Animate checkbox
        let animate_check = CheckButton::with_label("Enable Animation");
        animate_check.set_active(true);
        page.append(&animate_check);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        animate_check.connect_toggled(move |check| {
            config_clone.borrow_mut().animate = check.is_active();
            Self::queue_preview_redraw(&preview_clone, &on_change_clone);
        });

        // Animation duration
        let duration_box = GtkBox::new(Orientation::Horizontal, 6);
        duration_box.append(&Label::new(Some("Animation Duration (seconds):")));
        let duration_adj = Adjustment::new(1.0, 0.1, 10.0, 0.1, 1.0, 0.0);
        let animation_duration_spin = SpinButton::new(Some(&duration_adj), 0.1, 2);
        animation_duration_spin.set_hexpand(true);
        duration_box.append(&animation_duration_spin);
        page.append(&duration_box);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        animation_duration_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().animation_duration = spin.value();
            Self::queue_preview_redraw(&preview_clone, &on_change_clone);
        });

        // Bounce animation
        let bounce_animation_check = CheckButton::with_label("Bounce Animation");
        bounce_animation_check.set_active(false);
        page.append(&bounce_animation_check);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        bounce_animation_check.connect_toggled(move |check| {
            config_clone.borrow_mut().bounce_animation = check.is_active();
            Self::queue_preview_redraw(&preview_clone, &on_change_clone);
        });

        (
            page,
            animate_check,
            animation_duration_spin,
            bounce_animation_check,
        )
    }

    fn create_text_overlay_page(
        config: &Rc<RefCell<SpeedometerConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        available_fields: Vec<FieldMetadata>,
    ) -> (GtkBox, CheckButton, Rc<TextLineConfigWidget>) {
        let page = GtkBox::new(Orientation::Vertical, 12);
        page.set_margin_start(12);
        page.set_margin_end(12);
        page.set_margin_top(12);
        page.set_margin_bottom(12);

        // Enable text overlay
        let enable_check = CheckButton::with_label("Enable Text Overlay");
        enable_check.set_active(config.borrow().text_overlay.enabled);
        page.append(&enable_check);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        enable_check.connect_toggled(move |check| {
            config_clone.borrow_mut().text_overlay.enabled = check.is_active();
            Self::queue_preview_redraw(&preview_clone, &on_change_clone);
        });

        // Text configuration widget
        let text_config_widget = TextLineConfigWidget::new(available_fields);
        text_config_widget.set_config(config.borrow().text_overlay.text_config.clone());

        // Connect text widget on_change to update config and trigger parent callback
        let config_for_text = config.clone();
        let on_change_for_text = on_change.clone();
        let preview_for_text = preview.clone();
        let text_widget_rc = Rc::new(text_config_widget);
        let text_widget_for_callback = text_widget_rc.clone();
        text_widget_rc.set_on_change(move || {
            // Update config with new text settings
            config_for_text.borrow_mut().text_overlay.text_config = text_widget_for_callback.get_config();
            // Trigger parent on_change
            Self::queue_preview_redraw(&preview_for_text, &on_change_for_text);
        });

        page.append(text_widget_rc.widget());

        (page, enable_check, text_widget_rc)
    }

    pub fn widget(&self) -> &GtkBox {
        &self.container
    }

    pub fn get_config(&self) -> SpeedometerConfig {
        let mut config = self.config.borrow().clone();

        // Update text overlay config from widget
        config.text_overlay.enabled = self.enable_text_overlay_check.is_active();
        config.text_overlay.text_config = self.text_config_widget.get_config();

        config
    }

    pub fn set_config(&self, new_config: &SpeedometerConfig) {
        *self.config.borrow_mut() = new_config.clone();

        // Update all UI controls
        self.start_angle_spin.set_value(new_config.start_angle);
        self.end_angle_spin.set_value(new_config.end_angle);
        self.arc_width_scale.set_value(new_config.arc_width);
        self.radius_scale.set_value(new_config.radius_percent);

        self.show_track_check.set_active(new_config.show_track);
        let gradient_config = crate::ui::LinearGradientConfig {
            angle: 0.0, // Horizontal gradient
            stops: new_config.track_color_stops.clone(),
        };
        self.gradient_editor.set_gradient(&gradient_config);

        self.show_major_ticks_check.set_active(new_config.show_major_ticks);
        self.major_tick_count_spin.set_value(new_config.major_tick_count as f64);
        self.major_tick_length_scale.set_value(new_config.major_tick_length);
        self.major_tick_width_spin.set_value(new_config.major_tick_width);

        self.show_minor_ticks_check.set_active(new_config.show_minor_ticks);
        self.minor_ticks_per_major_spin.set_value(new_config.minor_ticks_per_major as f64);
        self.minor_tick_length_scale.set_value(new_config.minor_tick_length);
        self.minor_tick_width_spin.set_value(new_config.minor_tick_width);

        self.show_tick_labels_check.set_active(new_config.show_tick_labels);
        self.tick_label_font_button.set_label(&format!("{} {:.0}",
            new_config.tick_label_config.font_family,
            new_config.tick_label_config.font_size
        ));
        self.tick_label_font_size_spin.set_value(new_config.tick_label_config.font_size);
        self.tick_label_bold_check.set_active(new_config.tick_label_config.bold);
        self.tick_label_italic_check.set_active(new_config.tick_label_config.italic);
        self.tick_label_use_percentage_check.set_active(new_config.tick_label_config.use_percentage);

        self.show_needle_check.set_active(new_config.show_needle);
        self.needle_length_scale.set_value(new_config.needle_length);
        self.needle_width_spin.set_value(new_config.needle_width);
        self.needle_shadow_check.set_active(new_config.needle_shadow);

        self.show_center_hub_check.set_active(new_config.show_center_hub);
        self.center_hub_radius_scale.set_value(new_config.center_hub_radius);
        self.center_hub_3d_check.set_active(new_config.center_hub_3d);

        self.show_bezel_check.set_active(new_config.show_bezel);
        self.bezel_width_scale.set_value(new_config.bezel_width);
        self.bezel_background_widget.set_config(new_config.bezel_background.clone());

        self.animate_check.set_active(new_config.animate);
        self.animation_duration_spin.set_value(new_config.animation_duration);
        self.bounce_animation_check.set_active(new_config.bounce_animation);

        self.enable_text_overlay_check.set_active(new_config.text_overlay.enabled);
        self.text_config_widget.set_config(new_config.text_overlay.text_config.clone());

        self.preview.queue_draw();
    }

    pub fn set_on_change(&self, callback: Box<dyn Fn()>) {
        *self.on_change.borrow_mut() = Some(callback);
    }
}
