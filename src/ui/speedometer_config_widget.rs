//! Speedometer gauge configuration widget

use gtk4::prelude::*;
use gtk4::{
    Adjustment, Box as GtkBox, Button, CheckButton, DrawingArea, DropDown, Label, Notebook,
    Orientation, Scale, SpinButton, StringList,
};
use std::cell::RefCell;
use std::rc::Rc;

use crate::displayers::FieldMetadata;
use crate::ui::clipboard::CLIPBOARD;
use crate::ui::config::{ConfigWidget, LazyConfigWidget};
use crate::ui::render_utils::render_checkerboard;
use crate::ui::speedometer_display::{
    render_speedometer_with_theme, NeedleStyle, NeedleTailStyle, SpeedometerConfig, TickStyle,
};
use crate::ui::text_overlay_config_widget::TextOverlayConfigWidget;
use crate::ui::theme::ComboThemeConfig;
use crate::ui::theme_color_selector::ThemeColorSelector;
use crate::ui::widget_builder::{
    create_dropdown_row, create_page_container, create_spin_row_with_value, SpinChangeHandler,
};
use crate::ui::GradientEditor;

/// Speedometer gauge configuration widget
#[allow(dead_code)]
pub struct SpeedometerConfigWidget {
    container: GtkBox,
    config: Rc<RefCell<SpeedometerConfig>>,
    on_change: Rc<RefCell<Option<Box<dyn Fn()>>>>,
    preview: DrawingArea,
    theme: Rc<RefCell<ComboThemeConfig>>,

    // Geometry controls
    start_angle_spin: SpinButton,
    end_angle_spin: SpinButton,
    arc_width_scale: Scale,
    radius_scale: Scale,

    // Track controls
    show_track_check: CheckButton,
    track_color_widget: Rc<ThemeColorSelector>,
    gradient_editor: Rc<GradientEditor>,

    // Major ticks controls
    show_major_ticks_check: CheckButton,
    major_tick_count_spin: SpinButton,
    major_tick_length_scale: Scale,
    major_tick_width_spin: SpinButton,
    major_tick_color_widget: Rc<ThemeColorSelector>,
    major_tick_style_dropdown: DropDown,

    // Minor ticks controls
    show_minor_ticks_check: CheckButton,
    minor_ticks_per_major_spin: SpinButton,
    minor_tick_length_scale: Scale,
    minor_tick_width_spin: SpinButton,
    minor_tick_color_widget: Rc<ThemeColorSelector>,
    minor_tick_style_dropdown: DropDown,

    // Tick labels controls (using TickLabelConfig)
    show_tick_labels_check: CheckButton,
    tick_label_font_button: Button,
    tick_label_font_size_spin: SpinButton,
    tick_label_color_widget: Rc<ThemeColorSelector>,
    tick_label_bold_check: CheckButton,
    tick_label_italic_check: CheckButton,
    tick_label_use_percentage_check: CheckButton,

    // Needle controls
    show_needle_check: CheckButton,
    needle_style_dropdown: DropDown,
    needle_tail_style_dropdown: DropDown,
    needle_length_scale: Scale,
    needle_width_spin: SpinButton,
    needle_color_widget: Rc<ThemeColorSelector>,
    needle_shadow_check: CheckButton,

    // Center hub controls
    show_center_hub_check: CheckButton,
    center_hub_radius_scale: Scale,
    center_hub_color_widget: Rc<ThemeColorSelector>,
    center_hub_3d_check: CheckButton,

    // Bezel controls (using BackgroundConfig)
    show_bezel_check: CheckButton,
    bezel_width_scale: Scale,
    bezel_solid_color_widget: Rc<ThemeColorSelector>,
    bezel_background_widget: Rc<crate::ui::BackgroundConfigWidget>,

    // Animation controls
    animate_check: CheckButton,
    animation_duration_spin: SpinButton,
    bounce_animation_check: CheckButton,

    // Text overlay
    text_overlay_widget: Rc<TextOverlayConfigWidget>,
}

impl SpeedometerConfigWidget {
    pub fn new(available_fields: Vec<FieldMetadata>) -> Self {
        let container = GtkBox::new(Orientation::Vertical, 12);
        let config = Rc::new(RefCell::new(SpeedometerConfig::default()));
        let on_change: Rc<RefCell<Option<Box<dyn Fn()>>>> = Rc::new(RefCell::new(None));
        let theme = Rc::new(RefCell::new(ComboThemeConfig::default()));

        // Preview at the top
        let preview = DrawingArea::new();
        preview.set_content_height(200); // Max height 200px
        preview.set_content_width(200); // Min width 200px
        preview.set_hexpand(true);
        preview.set_halign(gtk4::Align::Fill);
        preview.set_vexpand(false);

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
            let _ = render_speedometer_with_theme(
                cr,
                &cfg,
                0.75,
                &preview_values,
                width as f64,
                height as f64,
                &thm,
            );
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
            Self::create_track_page(&config, &on_change, &preview, &theme);
        notebook.append_page(&track_page, Some(&Label::new(Some("Track"))));

        // === Tab 3: Ticks ===
        let (
            ticks_page,
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
        ) = Self::create_ticks_page(&config, &on_change, &preview);
        notebook.append_page(&ticks_page, Some(&Label::new(Some("Ticks"))));

        // === Tab 4: Needle ===
        let (
            needle_page,
            show_needle_check,
            needle_style_dropdown,
            needle_tail_style_dropdown,
            needle_length_scale,
            needle_width_spin,
            needle_color_button,
            needle_shadow_check,
        ) = Self::create_needle_page(&config, &on_change, &preview);
        notebook.append_page(&needle_page, Some(&Label::new(Some("Needle"))));

        // === Tab 5: Bezel & Hub ===
        let (
            bezel_page,
            show_center_hub_check,
            center_hub_radius_scale,
            center_hub_color_button,
            center_hub_3d_check,
            show_bezel_check,
            bezel_width_scale,
            bezel_solid_color_widget,
            bezel_background_widget,
        ) = Self::create_bezel_page(&config, &on_change, &preview);
        notebook.append_page(&bezel_page, Some(&Label::new(Some("Bezel & Hub"))));

        // === Tab 6: Animation ===
        let (animation_page, animate_check, animation_duration_spin, bounce_animation_check) =
            Self::create_animation_page(&config, &on_change, &preview);
        notebook.append_page(&animation_page, Some(&Label::new(Some("Animation"))));

        // === Tab 7: Text Overlay ===
        let text_overlay_widget = Rc::new(TextOverlayConfigWidget::new(available_fields));
        text_overlay_widget.set_config(config.borrow().text_overlay.clone());
        {
            let config_clone = config.clone();
            let on_change_clone = on_change.clone();
            let preview_clone = preview.clone();
            let text_widget_for_callback = text_overlay_widget.clone();
            text_overlay_widget.set_on_change(move || {
                config_clone.borrow_mut().text_overlay = text_widget_for_callback.get_config();
                preview_clone.queue_draw();
                if let Some(cb) = on_change_clone.borrow().as_ref() {
                    cb();
                }
            });
        }
        notebook.append_page(
            text_overlay_widget.widget(),
            Some(&Label::new(Some("Text Overlay"))),
        );

        preview.set_visible(false);
        container.append(&preview);

        // Copy/Paste buttons for the entire speedometer config
        let copy_paste_box = GtkBox::new(Orientation::Horizontal, 6);
        copy_paste_box.set_halign(gtk4::Align::End);
        copy_paste_box.set_margin_bottom(6);

        let copy_btn = Button::with_label("Copy Speedometer Config");
        let paste_btn = Button::with_label("Paste Speedometer Config");

        copy_paste_box.append(&copy_btn);
        copy_paste_box.append(&paste_btn);
        container.append(&copy_paste_box);

        container.append(&notebook);

        // Connect copy button
        let config_for_copy = config.clone();
        copy_btn.connect_clicked(move |_| {
            if let Ok(mut clipboard) = CLIPBOARD.lock() {
                clipboard.copy_speedometer_display(config_for_copy.borrow().clone());
            }
        });

        // Connect paste button - needs to update all UI widgets
        let config_for_paste = config.clone();
        let preview_for_paste = preview.clone();
        let on_change_for_paste = on_change.clone();
        let start_angle_spin_paste = start_angle_spin.clone();
        let end_angle_spin_paste = end_angle_spin.clone();
        let arc_width_scale_paste = arc_width_scale.clone();
        let radius_scale_paste = radius_scale.clone();
        let show_track_check_paste = show_track_check.clone();
        let track_color_widget_paste = track_color_widget.clone();
        let gradient_editor_paste = gradient_editor.clone();
        let show_major_ticks_check_paste = show_major_ticks_check.clone();
        let major_tick_count_spin_paste = major_tick_count_spin.clone();
        let major_tick_length_scale_paste = major_tick_length_scale.clone();
        let major_tick_width_spin_paste = major_tick_width_spin.clone();
        let major_tick_color_button_paste = major_tick_color_button.clone();
        let major_tick_style_dropdown_paste = major_tick_style_dropdown.clone();
        let show_minor_ticks_check_paste = show_minor_ticks_check.clone();
        let minor_ticks_per_major_spin_paste = minor_ticks_per_major_spin.clone();
        let minor_tick_length_scale_paste = minor_tick_length_scale.clone();
        let minor_tick_width_spin_paste = minor_tick_width_spin.clone();
        let minor_tick_color_button_paste = minor_tick_color_button.clone();
        let minor_tick_style_dropdown_paste = minor_tick_style_dropdown.clone();
        let show_tick_labels_check_paste = show_tick_labels_check.clone();
        let tick_label_font_button_paste = tick_label_font_button.clone();
        let tick_label_font_size_spin_paste = tick_label_font_size_spin.clone();
        let tick_label_color_button_paste = tick_label_color_button.clone();
        let tick_label_bold_check_paste = tick_label_bold_check.clone();
        let tick_label_italic_check_paste = tick_label_italic_check.clone();
        let tick_label_use_percentage_check_paste = tick_label_use_percentage_check.clone();
        let show_needle_check_paste = show_needle_check.clone();
        let needle_style_dropdown_paste = needle_style_dropdown.clone();
        let needle_tail_style_dropdown_paste = needle_tail_style_dropdown.clone();
        let needle_length_scale_paste = needle_length_scale.clone();
        let needle_width_spin_paste = needle_width_spin.clone();
        let needle_color_button_paste = needle_color_button.clone();
        let needle_shadow_check_paste = needle_shadow_check.clone();
        let show_center_hub_check_paste = show_center_hub_check.clone();
        let center_hub_radius_scale_paste = center_hub_radius_scale.clone();
        let center_hub_color_button_paste = center_hub_color_button.clone();
        let center_hub_3d_check_paste = center_hub_3d_check.clone();
        let show_bezel_check_paste = show_bezel_check.clone();
        let bezel_width_scale_paste = bezel_width_scale.clone();
        let bezel_solid_color_widget_paste = bezel_solid_color_widget.clone();
        let bezel_background_widget_paste = bezel_background_widget.clone();
        let animate_check_paste = animate_check.clone();
        let animation_duration_spin_paste = animation_duration_spin.clone();
        let bounce_animation_check_paste = bounce_animation_check.clone();
        let text_overlay_widget_paste = text_overlay_widget.clone();

        paste_btn.connect_clicked(move |_| {
            if let Ok(clipboard) = CLIPBOARD.lock() {
                if let Some(new_config) = clipboard.paste_speedometer_display() {
                    *config_for_paste.borrow_mut() = new_config.clone();

                    // Update geometry
                    start_angle_spin_paste.set_value(new_config.start_angle);
                    end_angle_spin_paste.set_value(new_config.end_angle);
                    arc_width_scale_paste.set_value(new_config.arc_width);
                    radius_scale_paste.set_value(new_config.radius_percent);

                    // Update track
                    show_track_check_paste.set_active(new_config.show_track);
                    track_color_widget_paste.set_source(new_config.track_color.clone());
                    // Use theme-aware color stops
                    gradient_editor_paste.set_stops_source(new_config.track_color_stops.clone());

                    // Update major ticks
                    show_major_ticks_check_paste.set_active(new_config.show_major_ticks);
                    major_tick_count_spin_paste.set_value(new_config.major_tick_count as f64);
                    major_tick_length_scale_paste.set_value(new_config.major_tick_length);
                    major_tick_width_spin_paste.set_value(new_config.major_tick_width);
                    major_tick_color_button_paste.set_source(new_config.major_tick_color.clone());
                    let major_style_idx = match new_config.major_tick_style {
                        TickStyle::Line => 0,
                        TickStyle::Wedge => 1,
                        TickStyle::Dot => 2,
                    };
                    major_tick_style_dropdown_paste.set_selected(major_style_idx);

                    // Update minor ticks
                    show_minor_ticks_check_paste.set_active(new_config.show_minor_ticks);
                    minor_ticks_per_major_spin_paste
                        .set_value(new_config.minor_ticks_per_major as f64);
                    minor_tick_length_scale_paste.set_value(new_config.minor_tick_length);
                    minor_tick_width_spin_paste.set_value(new_config.minor_tick_width);
                    minor_tick_color_button_paste.set_source(new_config.minor_tick_color.clone());
                    let minor_style_idx = match new_config.minor_tick_style {
                        TickStyle::Line => 0,
                        TickStyle::Wedge => 1,
                        TickStyle::Dot => 2,
                    };
                    minor_tick_style_dropdown_paste.set_selected(minor_style_idx);

                    // Update tick labels
                    show_tick_labels_check_paste.set_active(new_config.show_tick_labels);
                    tick_label_font_button_paste.set_label(&format!(
                        "{} {:.0}",
                        new_config.tick_label_config.font_family,
                        new_config.tick_label_config.font_size
                    ));
                    tick_label_font_size_spin_paste
                        .set_value(new_config.tick_label_config.font_size);
                    tick_label_color_button_paste
                        .set_source(new_config.tick_label_config.color.clone());
                    tick_label_bold_check_paste.set_active(new_config.tick_label_config.bold);
                    tick_label_italic_check_paste.set_active(new_config.tick_label_config.italic);
                    tick_label_use_percentage_check_paste
                        .set_active(new_config.tick_label_config.use_percentage);

                    // Update needle
                    show_needle_check_paste.set_active(new_config.show_needle);
                    let needle_style_idx = match new_config.needle_style {
                        NeedleStyle::Arrow => 0,
                        NeedleStyle::Line => 1,
                        NeedleStyle::Tapered => 2,
                        NeedleStyle::Triangle => 3,
                    };
                    needle_style_dropdown_paste.set_selected(needle_style_idx);
                    let needle_tail_idx = match new_config.needle_tail_style {
                        NeedleTailStyle::None => 0,
                        NeedleTailStyle::Short => 1,
                        NeedleTailStyle::Balanced => 2,
                    };
                    needle_tail_style_dropdown_paste.set_selected(needle_tail_idx);
                    needle_length_scale_paste.set_value(new_config.needle_length);
                    needle_width_spin_paste.set_value(new_config.needle_width);
                    needle_color_button_paste.set_source(new_config.needle_color.clone());
                    needle_shadow_check_paste.set_active(new_config.needle_shadow);

                    // Update center hub
                    show_center_hub_check_paste.set_active(new_config.show_center_hub);
                    center_hub_radius_scale_paste.set_value(new_config.center_hub_radius);
                    center_hub_color_button_paste.set_source(new_config.center_hub_color.clone());
                    center_hub_3d_check_paste.set_active(new_config.center_hub_3d);

                    // Update bezel
                    show_bezel_check_paste.set_active(new_config.show_bezel);
                    bezel_width_scale_paste.set_value(new_config.bezel_width);
                    bezel_solid_color_widget_paste.set_source(new_config.bezel_solid_color.clone());
                    bezel_background_widget_paste.set_config(new_config.bezel_background.clone());

                    // Update animation
                    animate_check_paste.set_active(new_config.animate);
                    animation_duration_spin_paste.set_value(new_config.animation_duration);
                    bounce_animation_check_paste.set_active(new_config.bounce_animation);

                    // Update text overlay
                    text_overlay_widget_paste.set_config(new_config.text_overlay.clone());

                    preview_for_paste.queue_draw();
                    if let Some(cb) = on_change_for_paste.borrow().as_ref() {
                        cb();
                    }
                }
            }
        });

        Self {
            container,
            config,
            on_change,
            preview,
            theme,
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
            bezel_solid_color_widget,
            bezel_background_widget,
            animate_check,
            animation_duration_spin,
            bounce_animation_check,
            text_overlay_widget,
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
        let page = create_page_container();
        let handler = SpinChangeHandler::new(config.clone(), preview.clone(), on_change.clone());

        // Start angle
        let (start_row, start_angle_spin) =
            create_spin_row_with_value("Start Angle (°):", -360.0, 360.0, 1.0, 135.0);
        page.append(&start_row);
        handler.connect_spin(&start_angle_spin, |cfg, val| cfg.start_angle = val);

        // End angle
        let (end_row, end_angle_spin) =
            create_spin_row_with_value("End Angle (°):", -360.0, 360.0, 1.0, 45.0);
        page.append(&end_row);
        handler.connect_spin(&end_angle_spin, |cfg, val| cfg.end_angle = val);

        // Arc width
        let arc_width_box = GtkBox::new(Orientation::Vertical, 6);
        arc_width_box.append(&Label::new(Some("Arc Width:")));
        let arc_width_scale = Scale::with_range(Orientation::Horizontal, 0.05, 0.5, 0.01);
        arc_width_scale.set_value(0.15);
        arc_width_scale.set_draw_value(true);
        arc_width_box.append(&arc_width_scale);
        page.append(&arc_width_box);

        let config_clone = config.clone();
        let preview_clone = preview.clone();
        let on_change_clone = on_change.clone();
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
        theme: &Rc<RefCell<ComboThemeConfig>>,
    ) -> (
        GtkBox,
        CheckButton,
        Rc<ThemeColorSelector>,
        Rc<GradientEditor>,
    ) {
        let page = create_page_container();

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

        // Track color - using ThemeColorSelector for theme support
        let track_color_box = GtkBox::new(Orientation::Horizontal, 6);
        track_color_box.append(&Label::new(Some("Track Base Color:")));
        let track_color_widget =
            Rc::new(ThemeColorSelector::new(config.borrow().track_color.clone()));
        track_color_box.append(track_color_widget.widget());
        page.append(&track_color_box);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        track_color_widget.set_on_change(move |source| {
            config_clone.borrow_mut().track_color = source;
            Self::queue_preview_redraw(&preview_clone, &on_change_clone);
        });

        // Gradient editor for track color zones
        let gradient_editor = Rc::new(GradientEditor::new());

        // Copy/Paste gradient buttons (above the gradient editor)
        let copy_paste_box = GtkBox::new(Orientation::Horizontal, 6);
        let copy_gradient_btn = Button::with_label("Copy Gradient");
        let paste_gradient_btn = Button::with_label("Paste Gradient");

        let config_for_copy = config.clone();
        let theme_for_copy = theme.clone();
        copy_gradient_btn.connect_clicked(move |_| {
            use crate::ui::CLIPBOARD;

            let cfg = config_for_copy.borrow();
            let thm = theme_for_copy.borrow();
            // Resolve theme-aware stops to concrete colors for clipboard
            let resolved_stops: Vec<crate::ui::background::ColorStop> = cfg
                .track_color_stops
                .iter()
                .map(|stop| stop.resolve(&thm))
                .collect();
            if let Ok(mut clipboard) = CLIPBOARD.lock() {
                clipboard.copy_gradient_stops(resolved_stops);
                log::info!("Speedometer track gradient color stops copied to clipboard");
            }
        });

        let config_for_paste = config.clone();
        let preview_for_paste = preview.clone();
        let on_change_for_paste = on_change.clone();
        let gradient_editor_for_paste = gradient_editor.clone();
        paste_gradient_btn.connect_clicked(move |_| {
            use crate::ui::theme::ColorStopSource;
            use crate::ui::CLIPBOARD;

            if let Ok(clipboard) = CLIPBOARD.lock() {
                if let Some(stops) = clipboard.paste_gradient_stops() {
                    // Convert ColorStop to ColorStopSource (as custom colors)
                    let stops_source: Vec<ColorStopSource> = stops
                        .into_iter()
                        .map(|stop| ColorStopSource::custom(stop.position, stop.color))
                        .collect();

                    config_for_paste.borrow_mut().track_color_stops = stops_source.clone();

                    // Update the gradient editor widget with theme-aware stops
                    gradient_editor_for_paste.set_stops_source(stops_source);

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
            // Use theme-aware color stops
            let stops = gradient_editor_clone.get_stops_source();
            config_clone.borrow_mut().track_color_stops = stops;
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
        Rc<ThemeColorSelector>,
        DropDown,
        CheckButton,
        SpinButton,
        Scale,
        SpinButton,
        Rc<ThemeColorSelector>,
        DropDown,
        CheckButton,
        Button,
        SpinButton,
        Rc<ThemeColorSelector>,
        CheckButton,
        CheckButton,
        CheckButton,
    ) {
        let page = create_page_container();

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

        // Major tick color - using ThemeColorSelector
        let major_tick_color_box = GtkBox::new(Orientation::Horizontal, 6);
        major_tick_color_box.append(&Label::new(Some("Color:")));
        let major_tick_color_button = Rc::new(ThemeColorSelector::new(
            config.borrow().major_tick_color.clone(),
        ));
        major_tick_color_box.append(major_tick_color_button.widget());
        page.append(&major_tick_color_box);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        major_tick_color_button.set_on_change(move |source| {
            config_clone.borrow_mut().major_tick_color = source;
            Self::queue_preview_redraw(&preview_clone, &on_change_clone);
        });

        // Major tick style
        let major_style_box = GtkBox::new(Orientation::Horizontal, 6);
        major_style_box.append(&Label::new(Some("Style:")));
        let major_styles = StringList::new(&["Line", "Wedge", "Dot"]);
        let major_tick_style_dropdown =
            DropDown::new(Some(major_styles), Option::<gtk4::Expression>::None);
        major_tick_style_dropdown.set_selected(0);
        major_style_box.append(&major_tick_style_dropdown);
        page.append(&major_style_box);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        major_tick_style_dropdown.connect_selected_notify(move |dropdown| {
            let selected = dropdown.selected();
            if selected == gtk4::INVALID_LIST_POSITION {
                return;
            }
            let style = match selected {
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

        // Minor tick color - using ThemeColorSelector
        let minor_tick_color_box = GtkBox::new(Orientation::Horizontal, 6);
        minor_tick_color_box.append(&Label::new(Some("Color:")));
        let minor_tick_color_button = Rc::new(ThemeColorSelector::new(
            config.borrow().minor_tick_color.clone(),
        ));
        minor_tick_color_box.append(minor_tick_color_button.widget());
        page.append(&minor_tick_color_box);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        minor_tick_color_button.set_on_change(move |source| {
            config_clone.borrow_mut().minor_tick_color = source;
            Self::queue_preview_redraw(&preview_clone, &on_change_clone);
        });

        // Minor tick style
        let minor_style_box = GtkBox::new(Orientation::Horizontal, 6);
        minor_style_box.append(&Label::new(Some("Style:")));
        let minor_styles = StringList::new(&["Line", "Wedge", "Dot"]);
        let minor_tick_style_dropdown =
            DropDown::new(Some(minor_styles), Option::<gtk4::Expression>::None);
        minor_tick_style_dropdown.set_selected(0);
        minor_style_box.append(&minor_tick_style_dropdown);
        page.append(&minor_style_box);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        minor_tick_style_dropdown.connect_selected_notify(move |dropdown| {
            let selected = dropdown.selected();
            if selected == gtk4::INVALID_LIST_POSITION {
                return;
            }
            let style = match selected {
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
        let initial_font_label = format!(
            "{} {:.0}",
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
            let window = btn
                .root()
                .and_then(|root| root.downcast::<gtk4::Window>().ok());

            // Get current font description
            let current_font = {
                let cfg = config_clone_font.borrow();
                let font_str = format!(
                    "{} {}",
                    cfg.tick_label_config.font_family, cfg.tick_label_config.font_size as i32
                );
                gtk4::pango::FontDescription::from_string(&font_str)
            };

            let config_clone2 = config_clone_font.clone();
            let on_change_clone2 = on_change_clone_font.clone();
            let preview_clone2 = preview_clone_font.clone();
            let font_button_clone2 = font_button_clone_font.clone();
            let size_spin_clone2 = size_spin_clone_font.clone();

            // Use callback-based API for font selection with shared dialog
            crate::ui::shared_font_dialog::show_font_dialog(
                window.as_ref(),
                Some(&current_font),
                move |font_desc| {
                    // Extract family and size from font description
                    let family = font_desc
                        .family()
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "Sans".to_string());
                    let size = font_desc.size() as f64 / gtk4::pango::SCALE as f64;

                    config_clone2.borrow_mut().tick_label_config.font_family = family.clone();
                    config_clone2.borrow_mut().tick_label_config.font_size = size;

                    // Update button label and size spinner
                    font_button_clone2.set_label(&format!("{} {:.0}", family, size));
                    size_spin_clone2.set_value(size);
                    Self::queue_preview_redraw(&preview_clone2, &on_change_clone2);
                },
            );
        });

        // Label color - using ThemeColorSelector
        let tick_label_color_box = GtkBox::new(Orientation::Horizontal, 6);
        tick_label_color_box.append(&Label::new(Some("Color:")));
        let tick_label_color_button = Rc::new(ThemeColorSelector::new(
            config.borrow().tick_label_config.color.clone(),
        ));
        tick_label_color_box.append(tick_label_color_button.widget());
        page.append(&tick_label_color_box);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        tick_label_color_button.set_on_change(move |source| {
            config_clone.borrow_mut().tick_label_config.color = source;
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
        let tick_label_use_percentage_check =
            CheckButton::with_label("Show as 0-100% instead of actual values");
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
        Rc<ThemeColorSelector>,
        CheckButton,
    ) {
        let page = create_page_container();
        let handler = SpinChangeHandler::new(config.clone(), preview.clone(), on_change.clone());

        // Show needle
        let show_needle_check = CheckButton::with_label("Show Needle");
        show_needle_check.set_active(true);
        page.append(&show_needle_check);
        handler.connect_check(&show_needle_check, |cfg, val| cfg.show_needle = val);

        // Needle style
        let (style_row, needle_style_dropdown) =
            create_dropdown_row("Needle Style:", &["Arrow", "Line", "Tapered", "Triangle"]);
        page.append(&style_row);
        handler.connect_dropdown(&needle_style_dropdown, |cfg, sel| {
            cfg.needle_style = match sel {
                0 => NeedleStyle::Arrow,
                1 => NeedleStyle::Line,
                2 => NeedleStyle::Tapered,
                _ => NeedleStyle::Triangle,
            };
        });

        // Needle tail style
        let (tail_row, needle_tail_style_dropdown) =
            create_dropdown_row("Needle Tail:", &["None", "Short", "Balanced"]);
        needle_tail_style_dropdown.set_selected(1);
        page.append(&tail_row);
        handler.connect_dropdown(&needle_tail_style_dropdown, |cfg, sel| {
            cfg.needle_tail_style = match sel {
                0 => NeedleTailStyle::None,
                1 => NeedleTailStyle::Short,
                _ => NeedleTailStyle::Balanced,
            };
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
        let (width_row, needle_width_spin) =
            create_spin_row_with_value("Needle Width (px):", 1.0, 20.0, 0.5, 3.0);
        needle_width_spin.set_digits(1);
        page.append(&width_row);
        handler.connect_spin(&needle_width_spin, |cfg, val| cfg.needle_width = val);

        // Needle color - using ThemeColorSelector
        let needle_color_button = Rc::new(ThemeColorSelector::new(
            config.borrow().needle_color.clone(),
        ));
        let needle_color_row = crate::ui::widget_builder::create_labeled_row(
            "Needle Color:",
            needle_color_button.widget(),
        );
        page.append(&needle_color_row);

        let config_clone = config.clone();
        let preview_clone = preview.clone();
        let on_change_clone = on_change.clone();
        needle_color_button.set_on_change(move |source| {
            config_clone.borrow_mut().needle_color = source;
            Self::queue_preview_redraw(&preview_clone, &on_change_clone);
        });

        // Needle shadow
        let needle_shadow_check = CheckButton::with_label("Add Shadow");
        needle_shadow_check.set_active(false);
        page.append(&needle_shadow_check);
        handler.connect_check(&needle_shadow_check, |cfg, val| cfg.needle_shadow = val);

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
        Rc<ThemeColorSelector>,
        CheckButton,
        CheckButton,
        Scale,
        Rc<ThemeColorSelector>,
        Rc<crate::ui::BackgroundConfigWidget>,
    ) {
        let page = create_page_container();

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

        // Hub color - using ThemeColorSelector
        let center_hub_color_box = GtkBox::new(Orientation::Horizontal, 6);
        center_hub_color_box.append(&Label::new(Some("Hub Color:")));
        let center_hub_color_button = Rc::new(ThemeColorSelector::new(
            config.borrow().center_hub_color.clone(),
        ));
        center_hub_color_box.append(center_hub_color_button.widget());
        page.append(&center_hub_color_box);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        center_hub_color_button.set_on_change(move |source| {
            config_clone.borrow_mut().center_hub_color = source;
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
        bezel_width_box.append(&Label::new(Some(
            "Bezel Width (% of available space, 0-100%):",
        )));
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

        // Bezel solid color (theme-aware)
        let bezel_color_box = GtkBox::new(Orientation::Horizontal, 6);
        bezel_color_box.append(&Label::new(Some("Bezel Solid Color:")));
        let bezel_solid_color_widget = Rc::new(ThemeColorSelector::new(
            config.borrow().bezel_solid_color.clone(),
        ));
        bezel_color_box.append(bezel_solid_color_widget.widget());
        page.append(&bezel_color_box);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        bezel_solid_color_widget.set_on_change(move |color_source| {
            config_clone.borrow_mut().bezel_solid_color = color_source;
            Self::queue_preview_redraw(&preview_clone, &on_change_clone);
        });

        // Bezel background (full background config widget - for gradients/images)
        let bezel_background_label = Label::new(Some("Bezel Background (for gradients/images):"));
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
            bezel_solid_color_widget,
            bezel_background_widget,
        )
    }

    fn create_animation_page(
        config: &Rc<RefCell<SpeedometerConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
    ) -> (GtkBox, CheckButton, SpinButton, CheckButton) {
        let page = create_page_container();

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

    pub fn widget(&self) -> &GtkBox {
        &self.container
    }

    pub fn get_config(&self) -> SpeedometerConfig {
        let mut config = self.config.borrow().clone();

        // Update text overlay from widget
        config.text_overlay = self.text_overlay_widget.get_config();

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
        self.track_color_widget
            .set_source(new_config.track_color.clone());
        // Use theme-aware color stops
        self.gradient_editor
            .set_stops_source(new_config.track_color_stops.clone());

        self.show_major_ticks_check
            .set_active(new_config.show_major_ticks);
        self.major_tick_count_spin
            .set_value(new_config.major_tick_count as f64);
        self.major_tick_length_scale
            .set_value(new_config.major_tick_length);
        self.major_tick_width_spin
            .set_value(new_config.major_tick_width);
        self.major_tick_color_widget
            .set_source(new_config.major_tick_color.clone());

        self.show_minor_ticks_check
            .set_active(new_config.show_minor_ticks);
        self.minor_ticks_per_major_spin
            .set_value(new_config.minor_ticks_per_major as f64);
        self.minor_tick_length_scale
            .set_value(new_config.minor_tick_length);
        self.minor_tick_width_spin
            .set_value(new_config.minor_tick_width);
        self.minor_tick_color_widget
            .set_source(new_config.minor_tick_color.clone());

        self.show_tick_labels_check
            .set_active(new_config.show_tick_labels);
        self.tick_label_font_button.set_label(&format!(
            "{} {:.0}",
            new_config.tick_label_config.font_family, new_config.tick_label_config.font_size
        ));
        self.tick_label_font_size_spin
            .set_value(new_config.tick_label_config.font_size);
        self.tick_label_color_widget
            .set_source(new_config.tick_label_config.color.clone());
        self.tick_label_bold_check
            .set_active(new_config.tick_label_config.bold);
        self.tick_label_italic_check
            .set_active(new_config.tick_label_config.italic);
        self.tick_label_use_percentage_check
            .set_active(new_config.tick_label_config.use_percentage);

        self.show_needle_check.set_active(new_config.show_needle);
        self.needle_length_scale.set_value(new_config.needle_length);
        self.needle_width_spin.set_value(new_config.needle_width);
        self.needle_color_widget
            .set_source(new_config.needle_color.clone());
        self.needle_shadow_check
            .set_active(new_config.needle_shadow);

        self.show_center_hub_check
            .set_active(new_config.show_center_hub);
        self.center_hub_radius_scale
            .set_value(new_config.center_hub_radius);
        self.center_hub_color_widget
            .set_source(new_config.center_hub_color.clone());
        self.center_hub_3d_check
            .set_active(new_config.center_hub_3d);

        self.show_bezel_check.set_active(new_config.show_bezel);
        self.bezel_width_scale.set_value(new_config.bezel_width);
        self.bezel_solid_color_widget
            .set_source(new_config.bezel_solid_color.clone());
        self.bezel_background_widget
            .set_config(new_config.bezel_background.clone());

        self.animate_check.set_active(new_config.animate);
        self.animation_duration_spin
            .set_value(new_config.animation_duration);
        self.bounce_animation_check
            .set_active(new_config.bounce_animation);

        self.text_overlay_widget
            .set_config(new_config.text_overlay.clone());

        self.preview.queue_draw();
    }

    pub fn set_on_change<F: Fn() + 'static>(&self, callback: F) {
        *self.on_change.borrow_mut() = Some(Box::new(callback));
    }

    /// Set the theme configuration for theme-aware child widgets
    pub fn set_theme(&self, theme: crate::ui::theme::ComboThemeConfig) {
        // Update internal theme reference
        *self.theme.borrow_mut() = theme.clone();

        // Propagate theme to all ThemeColorSelector widgets
        self.track_color_widget.set_theme_config(theme.clone());
        self.major_tick_color_widget.set_theme_config(theme.clone());
        self.minor_tick_color_widget.set_theme_config(theme.clone());
        self.tick_label_color_widget.set_theme_config(theme.clone());
        self.needle_color_widget.set_theme_config(theme.clone());
        self.center_hub_color_widget.set_theme_config(theme.clone());
        self.bezel_solid_color_widget
            .set_theme_config(theme.clone());

        // Propagate theme to gradient editor for theme-aware color stops
        self.gradient_editor.set_theme_config(theme.clone());

        // Propagate theme to bezel background widget for theme-aware gradients
        self.bezel_background_widget.set_theme_config(theme.clone());

        // Propagate theme to text overlay widget for T1/T2 font selectors
        self.text_overlay_widget.set_theme(theme);

        // Redraw preview with new theme
        self.preview.queue_draw();

        // Notify parent to refresh with new theme colors
        if let Some(callback) = self.on_change.borrow().as_ref() {
            callback();
        }
    }

    /// Cleanup method to break reference cycles
    pub fn cleanup(&self) {
        *self.on_change.borrow_mut() = None;
    }
}

impl ConfigWidget for SpeedometerConfigWidget {
    type Config = SpeedometerConfig;

    fn new(available_fields: Vec<FieldMetadata>) -> Self {
        SpeedometerConfigWidget::new(available_fields)
    }

    fn widget(&self) -> &gtk4::Box {
        &self.container
    }

    fn set_config(&self, config: Self::Config) {
        SpeedometerConfigWidget::set_config(self, &config)
    }

    fn get_config(&self) -> Self::Config {
        SpeedometerConfigWidget::get_config(self)
    }

    fn set_on_change<F: Fn() + 'static>(&self, callback: F) {
        SpeedometerConfigWidget::set_on_change(self, callback)
    }

    fn set_theme(&self, theme: ComboThemeConfig) {
        SpeedometerConfigWidget::set_theme(self, theme)
    }

    fn cleanup(&self) {
        SpeedometerConfigWidget::cleanup(self)
    }
}

/// Lazy-loading wrapper for SpeedometerConfigWidget.
pub type LazySpeedometerConfigWidget = LazyConfigWidget<SpeedometerConfigWidget>;
