//! Arc gauge configuration widget

use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, CheckButton, DrawingArea, DropDown, Label, Notebook, Orientation, Scale,
    SpinButton,
};
use std::cell::RefCell;
use std::rc::Rc;

use crate::displayers::FieldMetadata;
use crate::ui::arc_display::{
    render_arc, ArcCapStyle, ArcDisplayConfig, ArcTaperStyle, ColorApplicationMode,
    ColorTransitionStyle,
};
use crate::ui::background::ColorStop;
use crate::ui::clipboard::CLIPBOARD;
use crate::ui::render_utils::render_checkerboard;
use crate::ui::text_overlay_config_widget::TextOverlayConfigWidget;
use crate::ui::theme::{ColorSource, ColorStopSource, ComboThemeConfig};
use crate::ui::theme_color_selector::ThemeColorSelector;
use crate::ui::widget_builder::{
    create_dropdown_row, create_labeled_row, create_page_container, create_spin_row_with_value,
    SpinChangeHandler,
};
use crate::ui::GradientEditor;

/// Arc gauge configuration widget
pub struct ArcConfigWidget {
    container: GtkBox,
    config: Rc<RefCell<ArcDisplayConfig>>,
    theme: Rc<RefCell<ComboThemeConfig>>,
    on_change: Rc<RefCell<Option<Box<dyn Fn()>>>>,
    preview: DrawingArea,

    // Geometry controls
    start_angle_spin: SpinButton,
    end_angle_spin: SpinButton,
    arc_width_scale: Scale,
    radius_scale: Scale,

    // Segmentation controls
    segmented_check: CheckButton,
    segment_count_spin: SpinButton,
    segment_spacing_spin: SpinButton,

    // Style controls
    cap_style_dropdown: DropDown,
    taper_style_dropdown: DropDown,
    taper_amount_spin: SpinButton,

    // Color controls
    color_transition_dropdown: DropDown,
    color_mode_dropdown: DropDown,
    gradient_editor: Rc<GradientEditor>,

    // Background arc controls
    show_bg_arc_check: CheckButton,
    overlay_bg_check: CheckButton,
    bg_color_widget: Rc<ThemeColorSelector>,

    // Animation controls
    animate_check: CheckButton,
    animation_duration_spin: SpinButton,

    // Text overlay
    text_overlay_widget: Rc<TextOverlayConfigWidget>,
}

/// Convert ColorStopSource to ColorStop for gradient editor display
fn color_stop_sources_to_stops(sources: &[ColorStopSource]) -> Vec<ColorStop> {
    // Use a default theme for resolving theme colors in the config widget
    let theme = ComboThemeConfig::default();
    sources.iter().map(|s| s.resolve(&theme)).collect()
}

/// Convert ColorStop back to ColorStopSource (as Custom colors)
fn stops_to_color_stop_sources(stops: &[ColorStop]) -> Vec<ColorStopSource> {
    stops
        .iter()
        .map(|s| ColorStopSource {
            position: s.position,
            color: ColorSource::Custom { color: s.color },
        })
        .collect()
}

impl ArcConfigWidget {
    pub fn new(available_fields: Vec<FieldMetadata>) -> Self {
        let container = GtkBox::new(Orientation::Vertical, 12);
        let config = Rc::new(RefCell::new(ArcDisplayConfig::default()));
        let theme = Rc::new(RefCell::new(ComboThemeConfig::default()));
        let on_change: Rc<RefCell<Option<Box<dyn Fn()>>>> = Rc::new(RefCell::new(None));

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
            let _ = render_arc(
                cr,
                &cfg,
                &thm,
                0.75,
                &preview_values,
                width as f64,
                height as f64,
            );
        });

        // Create notebook for tabbed interface
        let notebook = Notebook::new();
        notebook.set_vexpand(true);

        // === Tab 1: Geometry ===
        let (
            geom_page,
            start_angle_spin,
            end_angle_spin,
            arc_width_scale,
            radius_scale,
            segmented_check,
            segment_count_spin,
            segment_spacing_spin,
        ) = Self::create_geometry_page(&config, &on_change, &preview);
        notebook.append_page(&geom_page, Some(&Label::new(Some("Geometry"))));

        // === Tab 2: Style ===
        let (
            style_page,
            cap_style_dropdown,
            taper_style_dropdown,
            taper_amount_spin,
            show_bg_arc_check,
            overlay_bg_check,
            bg_color_widget,
            animate_check,
            animation_duration_spin,
        ) = Self::create_style_page(&config, &theme, &on_change, &preview);
        notebook.append_page(&style_page, Some(&Label::new(Some("Style"))));

        // === Tab 3: Colors ===
        let (color_page, color_transition_dropdown, color_mode_dropdown, gradient_editor) =
            Self::create_color_page(&config, &on_change, &preview);
        notebook.append_page(&color_page, Some(&Label::new(Some("Colors"))));

        // === Tab 4: Text Overlay ===
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
            Some(&Label::new(Some("Text"))),
        );

        preview.set_visible(false); container.append(&preview);

        // Copy/Paste buttons for the entire arc config
        let copy_paste_box = GtkBox::new(Orientation::Horizontal, 6);
        copy_paste_box.set_halign(gtk4::Align::End);
        copy_paste_box.set_margin_bottom(6);

        let copy_btn = Button::with_label("Copy Arc Config");
        let paste_btn = Button::with_label("Paste Arc Config");

        copy_paste_box.append(&copy_btn);
        copy_paste_box.append(&paste_btn);
        container.append(&copy_paste_box);

        container.append(&notebook);

        // Connect copy button
        let config_for_copy = config.clone();
        copy_btn.connect_clicked(move |_| {
            if let Ok(mut clipboard) = CLIPBOARD.lock() {
                clipboard.copy_arc_display(config_for_copy.borrow().clone());
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
        let segmented_check_paste = segmented_check.clone();
        let segment_count_spin_paste = segment_count_spin.clone();
        let segment_spacing_spin_paste = segment_spacing_spin.clone();
        let cap_style_dropdown_paste = cap_style_dropdown.clone();
        let taper_style_dropdown_paste = taper_style_dropdown.clone();
        let taper_amount_spin_paste = taper_amount_spin.clone();
        let color_transition_dropdown_paste = color_transition_dropdown.clone();
        let color_mode_dropdown_paste = color_mode_dropdown.clone();
        let gradient_editor_paste = gradient_editor.clone();
        let show_bg_arc_check_paste = show_bg_arc_check.clone();
        let overlay_bg_check_paste = overlay_bg_check.clone();
        let animate_check_paste = animate_check.clone();
        let animation_duration_spin_paste = animation_duration_spin.clone();
        let text_overlay_widget_paste = text_overlay_widget.clone();

        paste_btn.connect_clicked(move |_| {
            if let Ok(clipboard) = CLIPBOARD.lock() {
                if let Some(new_config) = clipboard.paste_arc_display() {
                    *config_for_paste.borrow_mut() = new_config.clone();

                    // Update all UI controls
                    start_angle_spin_paste.set_value(new_config.start_angle);
                    end_angle_spin_paste.set_value(new_config.end_angle);
                    arc_width_scale_paste.set_value(new_config.arc_width);
                    radius_scale_paste.set_value(new_config.radius_percent);
                    segmented_check_paste.set_active(new_config.segmented);
                    segment_count_spin_paste.set_value(new_config.segment_count as f64);
                    segment_spacing_spin_paste.set_value(new_config.segment_spacing);

                    let cap_index = match new_config.cap_style {
                        ArcCapStyle::Butt => 0,
                        ArcCapStyle::Round => 1,
                        ArcCapStyle::Pointed => 2,
                    };
                    cap_style_dropdown_paste.set_selected(cap_index);

                    let taper_index = match new_config.taper_style {
                        ArcTaperStyle::None => 0,
                        ArcTaperStyle::Start => 1,
                        ArcTaperStyle::End => 2,
                        ArcTaperStyle::Both => 3,
                    };
                    taper_style_dropdown_paste.set_selected(taper_index);

                    taper_amount_spin_paste.set_value(new_config.taper_amount * 100.0);

                    let trans_index = match new_config.color_transition {
                        ColorTransitionStyle::Smooth => 0,
                        ColorTransitionStyle::Abrupt => 1,
                    };
                    color_transition_dropdown_paste.set_selected(trans_index);

                    let mode_index = match new_config.color_mode {
                        ColorApplicationMode::Progressive => 0,
                        ColorApplicationMode::Segments => 1,
                    };
                    color_mode_dropdown_paste.set_selected(mode_index);

                    show_bg_arc_check_paste.set_active(new_config.show_background_arc);
                    overlay_bg_check_paste.set_active(new_config.overlay_background);
                    animate_check_paste.set_active(new_config.animate);
                    animation_duration_spin_paste.set_value(new_config.animation_duration * 1000.0);

                    // Update gradient editor with theme-aware color stops
                    gradient_editor_paste.set_stops_source(new_config.color_stops.clone());

                    // Update text overlay config widget
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
            theme,
            on_change,
            preview,
            start_angle_spin,
            end_angle_spin,
            arc_width_scale,
            radius_scale,
            segmented_check,
            segment_count_spin,
            segment_spacing_spin,
            cap_style_dropdown,
            taper_style_dropdown,
            taper_amount_spin,
            color_transition_dropdown,
            color_mode_dropdown,
            gradient_editor,
            show_bg_arc_check,
            overlay_bg_check,
            bg_color_widget,
            animate_check,
            animation_duration_spin,
            text_overlay_widget,
        }
    }

    fn create_geometry_page(
        config: &Rc<RefCell<ArcDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
    ) -> (
        GtkBox,
        SpinButton,
        SpinButton,
        Scale,
        Scale,
        CheckButton,
        SpinButton,
        SpinButton,
    ) {
        let page = create_page_container();
        let handler = SpinChangeHandler::new(config.clone(), preview.clone(), on_change.clone());

        // Start angle
        let (start_row, start_spin) = create_spin_row_with_value(
            "Start Angle:",
            -360.0,
            360.0,
            1.0,
            config.borrow().start_angle,
        );
        page.append(&start_row);
        handler.connect_spin(&start_spin, |cfg, val| cfg.start_angle = val);

        // End angle
        let (end_row, end_spin) =
            create_spin_row_with_value("End Angle:", -360.0, 360.0, 1.0, config.borrow().end_angle);
        page.append(&end_row);
        handler.connect_spin(&end_spin, |cfg, val| cfg.end_angle = val);

        // Arc width
        let width_box = GtkBox::new(Orientation::Vertical, 6);
        width_box.append(&Label::new(Some("Arc Width (% of radius):")));
        let width_scale = Scale::with_range(Orientation::Horizontal, 0.05, 0.5, 0.01);
        width_scale.set_value(config.borrow().arc_width);
        width_scale.set_draw_value(true);
        width_scale.set_value_pos(gtk4::PositionType::Right);
        width_box.append(&width_scale);
        page.append(&width_box);

        let config_clone = config.clone();
        let preview_clone = preview.clone();
        let on_change_clone = on_change.clone();
        width_scale.connect_value_changed(move |scale| {
            config_clone.borrow_mut().arc_width = scale.value();
            preview_clone.queue_draw();
            if let Some(cb) = on_change_clone.borrow().as_ref() {
                cb();
            }
        });

        // Radius
        let radius_box = GtkBox::new(Orientation::Vertical, 6);
        radius_box.append(&Label::new(Some("Radius (% of space):")));
        let radius_scale = Scale::with_range(Orientation::Horizontal, 0.3, 1.0, 0.05);
        radius_scale.set_value(config.borrow().radius_percent);
        radius_scale.set_draw_value(true);
        radius_scale.set_value_pos(gtk4::PositionType::Right);
        radius_box.append(&radius_scale);
        page.append(&radius_box);

        let config_clone = config.clone();
        let preview_clone = preview.clone();
        let on_change_clone = on_change.clone();
        radius_scale.connect_value_changed(move |scale| {
            config_clone.borrow_mut().radius_percent = scale.value();
            preview_clone.queue_draw();
            if let Some(cb) = on_change_clone.borrow().as_ref() {
                cb();
            }
        });

        // Segmentation
        let seg_check = CheckButton::with_label("Segmented");
        seg_check.set_active(config.borrow().segmented);
        page.append(&seg_check);
        handler.connect_check(&seg_check, |cfg, val| cfg.segmented = val);

        // Segment count
        let (count_row, count_spin) = create_spin_row_with_value(
            "Segment Count:",
            5.0,
            50.0,
            1.0,
            config.borrow().segment_count as f64,
        );
        page.append(&count_row);
        handler.connect_spin_int(&count_spin, |cfg, val| cfg.segment_count = val as u32);

        // Segment spacing
        let (spacing_row, spacing_spin) = create_spin_row_with_value(
            "Segment Spacing (degrees):",
            0.0,
            10.0,
            0.5,
            config.borrow().segment_spacing,
        );
        page.append(&spacing_row);
        handler.connect_spin(&spacing_spin, |cfg, val| cfg.segment_spacing = val);

        (
            page,
            start_spin,
            end_spin,
            width_scale,
            radius_scale,
            seg_check,
            count_spin,
            spacing_spin,
        )
    }

    fn create_style_page(
        config: &Rc<RefCell<ArcDisplayConfig>>,
        theme: &Rc<RefCell<ComboThemeConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
    ) -> (
        GtkBox,
        DropDown,
        DropDown,
        SpinButton,
        CheckButton,
        CheckButton,
        Rc<ThemeColorSelector>,
        CheckButton,
        SpinButton,
    ) {
        let page = create_page_container();
        let handler = SpinChangeHandler::new(config.clone(), preview.clone(), on_change.clone());

        // Cap style
        let cap_index = match config.borrow().cap_style {
            ArcCapStyle::Butt => 0,
            ArcCapStyle::Round => 1,
            ArcCapStyle::Pointed => 2,
        };
        let (cap_row, cap_dropdown) =
            create_dropdown_row("End Cap Style:", &["Butt", "Round", "Pointed"]);
        cap_dropdown.set_selected(cap_index);
        page.append(&cap_row);

        handler.connect_dropdown(&cap_dropdown, |cfg, sel| {
            cfg.cap_style = match sel {
                0 => ArcCapStyle::Butt,
                1 => ArcCapStyle::Round,
                _ => ArcCapStyle::Pointed,
            };
        });

        // Taper style
        let taper_index = match config.borrow().taper_style {
            ArcTaperStyle::None => 0,
            ArcTaperStyle::Start => 1,
            ArcTaperStyle::End => 2,
            ArcTaperStyle::Both => 3,
        };
        let (taper_row, taper_dropdown) =
            create_dropdown_row("Taper Style:", &["None", "Start", "End", "Both"]);
        taper_dropdown.set_selected(taper_index);
        page.append(&taper_row);

        handler.connect_dropdown(&taper_dropdown, |cfg, sel| {
            cfg.taper_style = match sel {
                0 => ArcTaperStyle::None,
                1 => ArcTaperStyle::Start,
                2 => ArcTaperStyle::End,
                _ => ArcTaperStyle::Both,
            };
        });

        // Taper amount
        let (amount_row, amount_spin) = create_spin_row_with_value(
            "Taper Amount (%):",
            0.0,
            100.0,
            5.0,
            config.borrow().taper_amount * 100.0,
        );
        amount_spin.set_digits(0);
        page.append(&amount_row);
        handler.connect_spin_percent(&amount_spin, |cfg, val| cfg.taper_amount = val);

        // Background arc
        let bg_check = CheckButton::with_label("Show Background Arc");
        bg_check.set_active(config.borrow().show_background_arc);
        page.append(&bg_check);
        handler.connect_check(&bg_check, |cfg, val| cfg.show_background_arc = val);

        // Overlay background checkbox
        let overlay_check = CheckButton::with_label("Overlay Background");
        overlay_check.set_active(config.borrow().overlay_background);
        page.append(&overlay_check);
        handler.connect_check(&overlay_check, |cfg, val| cfg.overlay_background = val);

        // Background arc color - using ThemeColorSelector
        let bg_color_widget = Rc::new(ThemeColorSelector::new(
            config.borrow().background_color.clone(),
        ));
        bg_color_widget.set_theme_config(theme.borrow().clone());
        let bg_color_row = create_labeled_row("Background Arc Color:", bg_color_widget.widget());
        page.append(&bg_color_row);

        let config_clone = config.clone();
        let preview_clone = preview.clone();
        let on_change_clone = on_change.clone();
        bg_color_widget.set_on_change(move |color_source| {
            config_clone.borrow_mut().background_color = color_source;
            preview_clone.queue_draw();
            if let Some(cb) = on_change_clone.borrow().as_ref() {
                cb();
            }
        });

        // Animation
        let animate_check = CheckButton::with_label("Animate");
        animate_check.set_active(config.borrow().animate);
        page.append(&animate_check);
        handler.connect_check(&animate_check, |cfg, val| cfg.animate = val);

        // Animation duration
        let (duration_row, duration_spin) = create_spin_row_with_value(
            "Animation Duration (ms):",
            50.0,
            2000.0,
            50.0,
            config.borrow().animation_duration * 1000.0,
        );
        duration_spin.set_digits(0);
        page.append(&duration_row);

        let config_clone = config.clone();
        let preview_clone = preview.clone();
        let on_change_clone = on_change.clone();
        duration_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().animation_duration = spin.value() / 1000.0;
            preview_clone.queue_draw();
            if let Some(cb) = on_change_clone.borrow().as_ref() {
                cb();
            }
        });

        (
            page,
            cap_dropdown,
            taper_dropdown,
            amount_spin,
            bg_check,
            overlay_check,
            bg_color_widget,
            animate_check,
            duration_spin,
        )
    }

    fn create_color_page(
        config: &Rc<RefCell<ArcDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
    ) -> (GtkBox, DropDown, DropDown, Rc<GradientEditor>) {
        let page = create_page_container();
        let handler = SpinChangeHandler::new(config.clone(), preview.clone(), on_change.clone());

        // Color transition style
        let trans_index = match config.borrow().color_transition {
            ColorTransitionStyle::Smooth => 0,
            ColorTransitionStyle::Abrupt => 1,
        };
        let (transition_row, transition_dropdown) =
            create_dropdown_row("Color Transition:", &["Smooth", "Abrupt"]);
        transition_dropdown.set_selected(trans_index);
        page.append(&transition_row);

        handler.connect_dropdown(&transition_dropdown, |cfg, sel| {
            cfg.color_transition = match sel {
                0 => ColorTransitionStyle::Smooth,
                _ => ColorTransitionStyle::Abrupt,
            };
        });

        // Color application mode
        let mode_index = match config.borrow().color_mode {
            ColorApplicationMode::Progressive => 0,
            ColorApplicationMode::Segments => 1,
        };
        let (mode_row, mode_dropdown) =
            create_dropdown_row("Color Mode:", &["Progressive", "Segments"]);
        mode_dropdown.set_selected(mode_index);
        page.append(&mode_row);

        handler.connect_dropdown(&mode_dropdown, |cfg, sel| {
            cfg.color_mode = match sel {
                0 => ColorApplicationMode::Progressive,
                _ => ColorApplicationMode::Segments,
            };
        });

        // Color stops editor using GradientEditor
        page.append(&Label::new(Some("Color Stops:")));

        // Create gradient editor first so we can reference it in paste handler
        let gradient_editor = GradientEditor::new();

        // Initialize gradient editor with current color stops (using ColorStopSource)
        gradient_editor.set_stops_source(config.borrow().color_stops.clone());

        let gradient_editor_ref = Rc::new(gradient_editor);

        // Copy/Paste gradient buttons
        let copy_paste_box = GtkBox::new(Orientation::Horizontal, 6);
        let copy_gradient_btn = Button::with_label("Copy Gradient");
        let paste_gradient_btn = Button::with_label("Paste Gradient");

        let config_for_copy = config.clone();
        copy_gradient_btn.connect_clicked(move |_| {
            use crate::ui::CLIPBOARD;

            let cfg = config_for_copy.borrow();
            if let Ok(mut clipboard) = CLIPBOARD.lock() {
                clipboard.copy_gradient_stops(color_stop_sources_to_stops(&cfg.color_stops));
                log::info!("Arc gradient color stops copied to clipboard");
            }
        });

        let config_for_paste = config.clone();
        let preview_for_paste = preview.clone();
        let on_change_for_paste = on_change.clone();
        let gradient_editor_for_paste = gradient_editor_ref.clone();
        paste_gradient_btn.connect_clicked(move |_| {
            use crate::ui::CLIPBOARD;

            if let Ok(clipboard) = CLIPBOARD.lock() {
                if let Some(stops) = clipboard.paste_gradient_stops() {
                    config_for_paste.borrow_mut().color_stops = stops_to_color_stop_sources(&stops);

                    // Update the gradient editor widget
                    gradient_editor_for_paste.set_stops(stops);

                    preview_for_paste.queue_draw();

                    if let Some(callback) = on_change_for_paste.borrow().as_ref() {
                        callback();
                    }

                    log::info!("Arc gradient color stops pasted from clipboard");
                } else {
                    log::info!("No gradient color stops in clipboard");
                }
            }
        });

        copy_paste_box.append(&copy_gradient_btn);
        copy_paste_box.append(&paste_gradient_btn);
        page.append(&copy_paste_box);

        // Set up change handler for gradient editor
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let gradient_editor_clone = gradient_editor_ref.clone();

        gradient_editor_ref.set_on_change(move || {
            config_clone.borrow_mut().color_stops = gradient_editor_clone.get_stops_source();
            preview_clone.queue_draw();
            if let Some(cb) = on_change_clone.borrow().as_ref() {
                cb();
            }
        });

        // Make gradient editor expand with dialog
        gradient_editor_ref.widget().set_vexpand(true);
        page.append(gradient_editor_ref.widget());

        (
            page,
            transition_dropdown,
            mode_dropdown,
            gradient_editor_ref,
        )
    }

    pub fn widget(&self) -> &GtkBox {
        &self.container
    }

    pub fn set_config(&self, new_config: ArcDisplayConfig) {
        *self.config.borrow_mut() = new_config.clone();

        // Update all controls to reflect new config
        self.start_angle_spin.set_value(new_config.start_angle);
        self.end_angle_spin.set_value(new_config.end_angle);
        self.arc_width_scale.set_value(new_config.arc_width);
        self.radius_scale.set_value(new_config.radius_percent);
        self.segmented_check.set_active(new_config.segmented);
        self.segment_count_spin
            .set_value(new_config.segment_count as f64);
        self.segment_spacing_spin
            .set_value(new_config.segment_spacing);

        let cap_index = match new_config.cap_style {
            ArcCapStyle::Butt => 0,
            ArcCapStyle::Round => 1,
            ArcCapStyle::Pointed => 2,
        };
        self.cap_style_dropdown.set_selected(cap_index);

        let taper_index = match new_config.taper_style {
            ArcTaperStyle::None => 0,
            ArcTaperStyle::Start => 1,
            ArcTaperStyle::End => 2,
            ArcTaperStyle::Both => 3,
        };
        self.taper_style_dropdown.set_selected(taper_index);

        self.taper_amount_spin
            .set_value(new_config.taper_amount * 100.0);

        let trans_index = match new_config.color_transition {
            ColorTransitionStyle::Smooth => 0,
            ColorTransitionStyle::Abrupt => 1,
        };
        self.color_transition_dropdown.set_selected(trans_index);

        let mode_index = match new_config.color_mode {
            ColorApplicationMode::Progressive => 0,
            ColorApplicationMode::Segments => 1,
        };
        self.color_mode_dropdown.set_selected(mode_index);

        self.show_bg_arc_check
            .set_active(new_config.show_background_arc);
        self.overlay_bg_check
            .set_active(new_config.overlay_background);

        self.animate_check.set_active(new_config.animate);
        self.animation_duration_spin
            .set_value(new_config.animation_duration * 1000.0);

        // Update gradient editor with theme-aware color stops
        self.gradient_editor
            .set_stops_source(new_config.color_stops.clone());

        // Update text overlay widget
        self.text_overlay_widget.set_config(new_config.text_overlay);

        // Update preview
        self.preview.queue_draw();
    }

    pub fn get_config(&self) -> ArcDisplayConfig {
        let mut config = self.config.borrow().clone();

        // Update color stops from gradient editor (preserves theme references)
        config.color_stops = self.gradient_editor.get_stops_source();

        // Update text overlay from widget
        config.text_overlay = self.text_overlay_widget.get_config();

        // Include current theme in config
        config.theme = self.theme.borrow().clone();

        config
    }

    pub fn set_on_change<F: Fn() + 'static>(&self, callback: F) {
        *self.on_change.borrow_mut() = Some(Box::new(callback));
    }

    /// Update the theme configuration and refresh the preview
    pub fn set_theme(&self, new_theme: ComboThemeConfig) {
        *self.theme.borrow_mut() = new_theme.clone();
        // Update ThemeColorSelector widgets with new theme
        self.bg_color_widget.set_theme_config(new_theme.clone());
        // Update gradient editor with new theme
        self.gradient_editor.set_theme_config(new_theme.clone());
        // Update text overlay config widget with new theme
        self.text_overlay_widget.set_theme(new_theme);
        self.preview.queue_draw();
        // Notify parent to refresh with new theme colors
        if let Some(callback) = self.on_change.borrow().as_ref() {
            callback();
        }
    }
}

impl Default for ArcConfigWidget {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

/// Lazy wrapper for ArcConfigWidget to defer expensive widget creation
///
/// The actual ArcConfigWidget (with preview, notebook pages, etc.) is only created
/// when the widget becomes visible (mapped), saving significant memory when many
/// content items are created but only one display type is active.
pub struct LazyArcConfigWidget {
    /// Container that holds either the placeholder or the actual widget
    container: GtkBox,
    /// The actual widget, created lazily on first map
    inner_widget: Rc<RefCell<Option<ArcConfigWidget>>>,
    /// Deferred config to apply when widget is created
    deferred_config: Rc<RefCell<ArcDisplayConfig>>,
    /// Deferred theme to apply when widget is created
    deferred_theme: Rc<RefCell<ComboThemeConfig>>,
    /// Available fields for the widget (used in init closure)
    #[allow(dead_code)]
    available_fields: Vec<FieldMetadata>,
    /// Callback to invoke on config changes
    on_change: Rc<RefCell<Option<Box<dyn Fn()>>>>,
    /// Signal handler ID for map callback, stored to disconnect during cleanup
    map_handler_id: Rc<RefCell<Option<gtk4::glib::SignalHandlerId>>>,
}

impl LazyArcConfigWidget {
    /// Create a new lazy arc config widget
    ///
    /// The actual ArcConfigWidget is NOT created here - it's created automatically
    /// when the widget becomes visible (mapped).
    pub fn new(available_fields: Vec<FieldMetadata>) -> Self {
        let container = GtkBox::new(Orientation::Vertical, 0);
        let inner_widget: Rc<RefCell<Option<ArcConfigWidget>>> = Rc::new(RefCell::new(None));
        let deferred_config = Rc::new(RefCell::new(ArcDisplayConfig::default()));
        let deferred_theme = Rc::new(RefCell::new(ComboThemeConfig::default()));
        let on_change: Rc<RefCell<Option<Box<dyn Fn()>>>> = Rc::new(RefCell::new(None));

        // Create placeholder with loading indicator
        let placeholder = GtkBox::new(Orientation::Vertical, 8);
        placeholder.set_margin_top(12);
        placeholder.set_margin_bottom(12);
        placeholder.set_margin_start(12);
        placeholder.set_margin_end(12);

        let info_label = Label::new(Some("Loading arc configuration..."));
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
                    log::info!("LazyArcConfigWidget: Creating actual ArcConfigWidget on map");

                    // Create the actual widget
                    let widget = ArcConfigWidget::new(available_fields_clone.clone());

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
        // Store the handler ID so we can disconnect during cleanup to break the cycle
        let map_handler_id: Rc<RefCell<Option<gtk4::glib::SignalHandlerId>>> =
            Rc::new(RefCell::new(None));
        {
            let init_widget_clone = init_widget.clone();
            let handler_id = container.connect_map(move |_| {
                init_widget_clone();
            });
            *map_handler_id.borrow_mut() = Some(handler_id);
        }

        Self {
            container,
            inner_widget,
            deferred_config,
            deferred_theme,
            available_fields,
            on_change,
            map_handler_id,
        }
    }

    /// Get the widget container
    pub fn widget(&self) -> &GtkBox {
        &self.container
    }

    /// Set the arc configuration
    pub fn set_config(&self, config: ArcDisplayConfig) {
        *self.deferred_config.borrow_mut() = config.clone();
        if let Some(ref widget) = *self.inner_widget.borrow() {
            widget.set_config(config);
        }
    }

    /// Get the current arc configuration
    pub fn get_config(&self) -> ArcDisplayConfig {
        if let Some(ref widget) = *self.inner_widget.borrow() {
            widget.get_config()
        } else {
            self.deferred_config.borrow().clone()
        }
    }

    /// Set the theme for the arc widget
    pub fn set_theme(&self, theme: ComboThemeConfig) {
        *self.deferred_theme.borrow_mut() = theme.clone();
        if let Some(ref widget) = *self.inner_widget.borrow() {
            widget.set_theme(theme);
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

    /// Cleanup method to break reference cycles and allow garbage collection.
    /// This clears the on_change callback which may hold Rc references to this widget.
    pub fn cleanup(&self) {
        log::debug!("LazyArcConfigWidget::cleanup() - breaking reference cycles");
        // Disconnect the map signal handler to break the cycle
        if let Some(handler_id) = self.map_handler_id.borrow_mut().take() {
            self.container.disconnect(handler_id);
        }
        *self.on_change.borrow_mut() = None;
        *self.inner_widget.borrow_mut() = None;
    }
}
