//! Arc gauge configuration widget

use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, CheckButton, DrawingArea, DropDown, Label, Notebook, Orientation,
    Scale, SpinButton, StringList,
};
use std::cell::RefCell;
use std::rc::Rc;

use crate::ui::arc_display::{
    render_arc, ArcCapStyle, ArcDisplayConfig, ArcTaperStyle, ColorApplicationMode,
    ColorTransitionStyle,
};
use crate::ui::clipboard::CLIPBOARD;
use crate::ui::render_utils::render_checkerboard;
use crate::ui::theme::{ColorSource, ColorStopSource, ComboThemeConfig};
use crate::ui::theme_color_selector::ThemeColorSelector;
use crate::ui::background::ColorStop;
use crate::ui::GradientEditor;
use crate::displayers::FieldMetadata;
use crate::ui::text_line_config_widget::TextLineConfigWidget;

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
    text_config_widget: Option<Rc<TextLineConfigWidget>>,
}

/// Convert ColorStopSource to ColorStop for gradient editor display
fn color_stop_sources_to_stops(sources: &[ColorStopSource]) -> Vec<ColorStop> {
    // Use a default theme for resolving theme colors in the config widget
    let theme = ComboThemeConfig::default();
    sources.iter().map(|s| s.resolve(&theme)).collect()
}

/// Convert ColorStop back to ColorStopSource (as Custom colors)
fn stops_to_color_stop_sources(stops: &[ColorStop]) -> Vec<ColorStopSource> {
    stops.iter().map(|s| ColorStopSource {
        position: s.position,
        color: ColorSource::Custom { color: s.color },
    }).collect()
}


impl ArcConfigWidget {
    pub fn new(available_fields: Vec<FieldMetadata>) -> Self {
        let container = GtkBox::new(Orientation::Vertical, 12);
        let config = Rc::new(RefCell::new(ArcDisplayConfig::default()));
        let theme = Rc::new(RefCell::new(ComboThemeConfig::default()));
        let on_change: Rc<RefCell<Option<Box<dyn Fn()>>>> = Rc::new(RefCell::new(None));

        // Preview at the top
        let preview = DrawingArea::new();
        preview.set_content_height(200);
        preview.set_hexpand(true);
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
            let _ = render_arc(cr, &cfg, &thm, 0.75, &preview_values, width as f64, height as f64);
        });

        // Create notebook for tabbed interface
        let notebook = Notebook::new();
        notebook.set_vexpand(true);

        // === Tab 1: Geometry ===
        let (geom_page, start_angle_spin, end_angle_spin, arc_width_scale, radius_scale,
             segmented_check, segment_count_spin, segment_spacing_spin) =
            Self::create_geometry_page(&config, &on_change, &preview);
        notebook.append_page(&geom_page, Some(&Label::new(Some("Geometry"))));

        // === Tab 2: Style ===
        let (style_page, cap_style_dropdown, taper_style_dropdown, taper_amount_spin,
             show_bg_arc_check, overlay_bg_check, bg_color_widget,
             animate_check, animation_duration_spin) =
            Self::create_style_page(&config, &theme, &on_change, &preview);
        notebook.append_page(&style_page, Some(&Label::new(Some("Style"))));

        // === Tab 3: Colors ===
        let (color_page, color_transition_dropdown, color_mode_dropdown, gradient_editor) =
            Self::create_color_page(&config, &on_change, &preview);
        notebook.append_page(&color_page, Some(&Label::new(Some("Colors"))));

        // === Tab 4: Text Overlay ===
        let (text_page, text_config_widget) = Self::create_text_page(&config, &on_change, available_fields);
        notebook.append_page(&text_page, Some(&Label::new(Some("Text"))));

        container.append(&preview);

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
        let text_config_widget_paste = text_config_widget.clone();

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
                    if let Some(ref text_widget) = text_config_widget_paste {
                        text_widget.set_config(new_config.text_overlay.text_config.clone());
                    }

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
            text_config_widget,
        }
    }

    fn create_geometry_page(
        config: &Rc<RefCell<ArcDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
    ) -> (GtkBox, SpinButton, SpinButton, Scale, Scale, CheckButton, SpinButton, SpinButton) {
        let page = GtkBox::new(Orientation::Vertical, 12);
        page.set_margin_start(12);
        page.set_margin_end(12);
        page.set_margin_top(12);
        page.set_margin_bottom(12);

        // Start angle
        let start_box = GtkBox::new(Orientation::Horizontal, 6);
        start_box.append(&Label::new(Some("Start Angle:")));
        let start_spin = SpinButton::with_range(-360.0, 360.0, 1.0);
        start_spin.set_value(config.borrow().start_angle);
        start_spin.set_hexpand(true);
        start_box.append(&start_spin);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        start_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().start_angle = spin.value();
            preview_clone.queue_draw();
            if let Some(cb) = on_change_clone.borrow().as_ref() {
                cb();
            }
        });

        page.append(&start_box);

        // End angle
        let end_box = GtkBox::new(Orientation::Horizontal, 6);
        end_box.append(&Label::new(Some("End Angle:")));
        let end_spin = SpinButton::with_range(-360.0, 360.0, 1.0);
        end_spin.set_value(config.borrow().end_angle);
        end_spin.set_hexpand(true);
        end_box.append(&end_spin);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        end_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().end_angle = spin.value();
            preview_clone.queue_draw();
            if let Some(cb) = on_change_clone.borrow().as_ref() {
                cb();
            }
        });

        page.append(&end_box);

        // Arc width
        let width_box = GtkBox::new(Orientation::Vertical, 6);
        width_box.append(&Label::new(Some("Arc Width (% of radius):")));
        let width_scale = Scale::with_range(Orientation::Horizontal, 0.05, 0.5, 0.01);
        width_scale.set_value(config.borrow().arc_width);
        width_scale.set_draw_value(true);
        width_scale.set_value_pos(gtk4::PositionType::Right);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        width_scale.connect_value_changed(move |scale| {
            config_clone.borrow_mut().arc_width = scale.value();
            preview_clone.queue_draw();
            if let Some(cb) = on_change_clone.borrow().as_ref() {
                cb();
            }
        });

        width_box.append(&width_scale);
        page.append(&width_box);

        // Radius
        let radius_box = GtkBox::new(Orientation::Vertical, 6);
        radius_box.append(&Label::new(Some("Radius (% of space):")));
        let radius_scale = Scale::with_range(Orientation::Horizontal, 0.3, 1.0, 0.05);
        radius_scale.set_value(config.borrow().radius_percent);
        radius_scale.set_draw_value(true);
        radius_scale.set_value_pos(gtk4::PositionType::Right);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        radius_scale.connect_value_changed(move |scale| {
            config_clone.borrow_mut().radius_percent = scale.value();
            preview_clone.queue_draw();
            if let Some(cb) = on_change_clone.borrow().as_ref() {
                cb();
            }
        });

        radius_box.append(&radius_scale);
        page.append(&radius_box);

        // Segmentation
        let seg_check = CheckButton::with_label("Segmented");
        seg_check.set_active(config.borrow().segmented);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        seg_check.connect_toggled(move |check| {
            config_clone.borrow_mut().segmented = check.is_active();
            preview_clone.queue_draw();
            if let Some(cb) = on_change_clone.borrow().as_ref() {
                cb();
            }
        });

        page.append(&seg_check);

        // Segment count
        let count_box = GtkBox::new(Orientation::Horizontal, 6);
        count_box.append(&Label::new(Some("Segment Count:")));
        let count_spin = SpinButton::with_range(5.0, 50.0, 1.0);
        count_spin.set_value(config.borrow().segment_count as f64);
        count_spin.set_hexpand(true);
        count_box.append(&count_spin);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        count_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().segment_count = spin.value() as u32;
            if let Some(cb) = on_change_clone.borrow().as_ref() {
                cb();
            }
        });

        page.append(&count_box);

        // Segment spacing
        let spacing_box = GtkBox::new(Orientation::Horizontal, 6);
        spacing_box.append(&Label::new(Some("Segment Spacing (degrees):")));
        let spacing_spin = SpinButton::with_range(0.0, 10.0, 0.5);
        spacing_spin.set_value(config.borrow().segment_spacing);
        spacing_spin.set_hexpand(true);
        spacing_box.append(&spacing_spin);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        spacing_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().segment_spacing = spin.value();
            if let Some(cb) = on_change_clone.borrow().as_ref() {
                cb();
            }
        });

        page.append(&spacing_box);

        (page, start_spin, end_spin, width_scale, radius_scale,
         seg_check, count_spin, spacing_spin)
    }

    fn create_style_page(
        config: &Rc<RefCell<ArcDisplayConfig>>,
        theme: &Rc<RefCell<ComboThemeConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
    ) -> (GtkBox, DropDown, DropDown, SpinButton, CheckButton, CheckButton, Rc<ThemeColorSelector>, CheckButton, SpinButton) {
        let page = GtkBox::new(Orientation::Vertical, 12);
        page.set_margin_start(12);
        page.set_margin_end(12);
        page.set_margin_top(12);
        page.set_margin_bottom(12);

        // Cap style
        let cap_box = GtkBox::new(Orientation::Horizontal, 6);
        cap_box.append(&Label::new(Some("End Cap Style:")));
        let cap_options = StringList::new(&["Butt", "Round", "Pointed"]);
        let cap_dropdown = DropDown::new(Some(cap_options), Option::<gtk4::Expression>::None);

        let cap_index = match config.borrow().cap_style {
            ArcCapStyle::Butt => 0,
            ArcCapStyle::Round => 1,
            ArcCapStyle::Pointed => 2,
        };
        cap_dropdown.set_selected(cap_index);
        cap_dropdown.set_hexpand(true);
        cap_box.append(&cap_dropdown);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        cap_dropdown.connect_selected_notify(move |dropdown| {
            let selected = dropdown.selected();
            if selected == gtk4::INVALID_LIST_POSITION {
                return;
            }
            let style = match selected {
                0 => ArcCapStyle::Butt,
                1 => ArcCapStyle::Round,
                2 => ArcCapStyle::Pointed,
                _ => ArcCapStyle::Round,
            };
            config_clone.borrow_mut().cap_style = style;
            preview_clone.queue_draw();
            if let Some(cb) = on_change_clone.borrow().as_ref() {
                cb();
            }
        });

        page.append(&cap_box);

        // Taper style
        let taper_box = GtkBox::new(Orientation::Horizontal, 6);
        taper_box.append(&Label::new(Some("Taper Style:")));
        let taper_options = StringList::new(&["None", "Start", "End", "Both"]);
        let taper_dropdown = DropDown::new(Some(taper_options), Option::<gtk4::Expression>::None);

        let taper_index = match config.borrow().taper_style {
            ArcTaperStyle::None => 0,
            ArcTaperStyle::Start => 1,
            ArcTaperStyle::End => 2,
            ArcTaperStyle::Both => 3,
        };
        taper_dropdown.set_selected(taper_index);
        taper_dropdown.set_hexpand(true);
        taper_box.append(&taper_dropdown);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        taper_dropdown.connect_selected_notify(move |dropdown| {
            let selected = dropdown.selected();
            if selected == gtk4::INVALID_LIST_POSITION {
                return;
            }
            let style = match selected {
                0 => ArcTaperStyle::None,
                1 => ArcTaperStyle::Start,
                2 => ArcTaperStyle::End,
                3 => ArcTaperStyle::Both,
                _ => ArcTaperStyle::None,
            };
            config_clone.borrow_mut().taper_style = style;
            preview_clone.queue_draw();
            if let Some(cb) = on_change_clone.borrow().as_ref() {
                cb();
            }
        });

        page.append(&taper_box);

        // Taper amount
        let amount_box = GtkBox::new(Orientation::Horizontal, 6);
        amount_box.append(&Label::new(Some("Taper Amount:")));
        let amount_spin = SpinButton::with_range(0.0, 100.0, 5.0);
        amount_spin.set_value(config.borrow().taper_amount * 100.0); // Convert to percentage
        amount_spin.set_digits(0);
        amount_spin.set_width_request(80);
        let percent_label = Label::new(Some("%"));

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        amount_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().taper_amount = spin.value() / 100.0; // Convert from percentage
            preview_clone.queue_draw();
            if let Some(cb) = on_change_clone.borrow().as_ref() {
                cb();
            }
        });

        amount_box.append(&amount_spin);
        amount_box.append(&percent_label);
        page.append(&amount_box);

        // Background arc
        let bg_check = CheckButton::with_label("Show Background Arc");
        bg_check.set_active(config.borrow().show_background_arc);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        bg_check.connect_toggled(move |check| {
            config_clone.borrow_mut().show_background_arc = check.is_active();
            preview_clone.queue_draw();
            if let Some(cb) = on_change_clone.borrow().as_ref() {
                cb();
            }
        });

        page.append(&bg_check);

        // Overlay background checkbox
        let overlay_check = CheckButton::with_label("Overlay Background");
        overlay_check.set_active(config.borrow().overlay_background);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        overlay_check.connect_toggled(move |check| {
            config_clone.borrow_mut().overlay_background = check.is_active();
            preview_clone.queue_draw();
            if let Some(cb) = on_change_clone.borrow().as_ref() {
                cb();
            }
        });

        page.append(&overlay_check);

        // Background arc color - using ThemeColorSelector
        let bg_color_box = GtkBox::new(Orientation::Horizontal, 6);
        bg_color_box.append(&Label::new(Some("Background Arc Color:")));
        let bg_color_widget = Rc::new(ThemeColorSelector::new(config.borrow().background_color.clone()));
        bg_color_widget.set_theme_config(theme.borrow().clone());
        bg_color_box.append(bg_color_widget.widget());
        page.append(&bg_color_box);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
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

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        animate_check.connect_toggled(move |check| {
            config_clone.borrow_mut().animate = check.is_active();
            preview_clone.queue_draw();
            if let Some(cb) = on_change_clone.borrow().as_ref() {
                cb();
            }
        });

        page.append(&animate_check);

        // Animation duration
        let duration_box = GtkBox::new(Orientation::Horizontal, 6);
        duration_box.append(&Label::new(Some("Animation Duration (ms):")));
        let duration_spin = SpinButton::with_range(50.0, 2000.0, 50.0);
        duration_spin.set_value(config.borrow().animation_duration * 1000.0); // Convert to ms
        duration_spin.set_digits(0);
        duration_spin.set_width_request(100);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        duration_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().animation_duration = spin.value() / 1000.0; // Convert from ms
            preview_clone.queue_draw();
            if let Some(cb) = on_change_clone.borrow().as_ref() {
                cb();
            }
        });

        duration_box.append(&duration_spin);
        page.append(&duration_box);

        (page, cap_dropdown, taper_dropdown, amount_spin, bg_check, overlay_check, bg_color_widget,
         animate_check, duration_spin)
    }

    fn create_color_page(
        config: &Rc<RefCell<ArcDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
    ) -> (GtkBox, DropDown, DropDown, Rc<GradientEditor>) {
        let page = GtkBox::new(Orientation::Vertical, 12);
        page.set_margin_start(12);
        page.set_margin_end(12);
        page.set_margin_top(12);
        page.set_margin_bottom(12);

        // Color transition style
        let transition_box = GtkBox::new(Orientation::Horizontal, 6);
        transition_box.append(&Label::new(Some("Color Transition:")));
        let transition_options = StringList::new(&["Smooth", "Abrupt"]);
        let transition_dropdown = DropDown::new(Some(transition_options), Option::<gtk4::Expression>::None);

        let trans_index = match config.borrow().color_transition {
            ColorTransitionStyle::Smooth => 0,
            ColorTransitionStyle::Abrupt => 1,
        };
        transition_dropdown.set_selected(trans_index);
        transition_dropdown.set_hexpand(true);
        transition_box.append(&transition_dropdown);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        transition_dropdown.connect_selected_notify(move |dropdown| {
            let selected = dropdown.selected();
            if selected == gtk4::INVALID_LIST_POSITION {
                return;
            }
            let style = match selected {
                0 => ColorTransitionStyle::Smooth,
                1 => ColorTransitionStyle::Abrupt,
                _ => ColorTransitionStyle::Abrupt,
            };
            config_clone.borrow_mut().color_transition = style;
            if let Some(cb) = on_change_clone.borrow().as_ref() {
                cb();
            }
        });

        page.append(&transition_box);

        // Color application mode
        let mode_box = GtkBox::new(Orientation::Horizontal, 6);
        mode_box.append(&Label::new(Some("Color Mode:")));
        let mode_options = StringList::new(&["Progressive", "Segments"]);
        let mode_dropdown = DropDown::new(Some(mode_options), Option::<gtk4::Expression>::None);

        let mode_index = match config.borrow().color_mode {
            ColorApplicationMode::Progressive => 0,
            ColorApplicationMode::Segments => 1,
        };
        mode_dropdown.set_selected(mode_index);
        mode_dropdown.set_hexpand(true);
        mode_box.append(&mode_dropdown);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        mode_dropdown.connect_selected_notify(move |dropdown| {
            let selected = dropdown.selected();
            if selected == gtk4::INVALID_LIST_POSITION {
                return;
            }
            let mode = match selected {
                0 => ColorApplicationMode::Progressive,
                1 => ColorApplicationMode::Segments,
                _ => ColorApplicationMode::Progressive,
            };
            config_clone.borrow_mut().color_mode = mode;
            if let Some(cb) = on_change_clone.borrow().as_ref() {
                cb();
            }
        });

        page.append(&mode_box);

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

        (page, transition_dropdown, mode_dropdown, gradient_editor_ref)
    }

    fn create_text_page(
        config: &Rc<RefCell<ArcDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        available_fields: Vec<FieldMetadata>,
    ) -> (GtkBox, Option<Rc<TextLineConfigWidget>>) {
        let page = GtkBox::new(Orientation::Vertical, 12);
        page.set_margin_start(12);
        page.set_margin_end(12);
        page.set_margin_top(12);
        page.set_margin_bottom(12);

        // Enable text overlay checkbox
        let text_check = CheckButton::with_label("Show Text Overlay");
        text_check.set_active(config.borrow().text_overlay.enabled);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        text_check.connect_toggled(move |check| {
            config_clone.borrow_mut().text_overlay.enabled = check.is_active();
            if let Some(cb) = on_change_clone.borrow().as_ref() {
                cb();
            }
        });

        page.append(&text_check);

        // Text configuration widget
        let text_widget = TextLineConfigWidget::new(available_fields);
        text_widget.set_config(config.borrow().text_overlay.text_config.clone());

        // Connect text widget on_change to update config and trigger parent callback
        let config_for_text = config.clone();
        let on_change_for_text = on_change.clone();
        let text_widget_rc = Rc::new(text_widget);
        let text_widget_for_callback = text_widget_rc.clone();
        text_widget_rc.set_on_change(move || {
            // Update config with new text settings
            config_for_text.borrow_mut().text_overlay.text_config = text_widget_for_callback.get_config();
            // Trigger parent on_change
            if let Some(cb) = on_change_for_text.borrow().as_ref() {
                cb();
            }
        });

        page.append(text_widget_rc.widget());

        // Return the widget so set_config can update it when loading saved configs
        (page, Some(text_widget_rc))
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
        self.segment_count_spin.set_value(new_config.segment_count as f64);
        self.segment_spacing_spin.set_value(new_config.segment_spacing);

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

        self.taper_amount_spin.set_value(new_config.taper_amount * 100.0);

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

        self.show_bg_arc_check.set_active(new_config.show_background_arc);
        self.overlay_bg_check.set_active(new_config.overlay_background);

        self.animate_check.set_active(new_config.animate);
        self.animation_duration_spin.set_value(new_config.animation_duration * 1000.0);

        // Update gradient editor with theme-aware color stops
        self.gradient_editor.set_stops_source(new_config.color_stops.clone());

        if let Some(text_widget) = &self.text_config_widget {
            text_widget.set_config(new_config.text_overlay.text_config);
        }

        // Update preview
        self.preview.queue_draw();
    }

    pub fn get_config(&self) -> ArcDisplayConfig {
        let mut config = self.config.borrow().clone();

        // Update color stops from gradient editor (preserves theme references)
        config.color_stops = self.gradient_editor.get_stops_source();

        // Update text config from widget if available
        if let Some(text_widget) = &self.text_config_widget {
            config.text_overlay.text_config = text_widget.get_config();
        }

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
        if let Some(text_widget) = &self.text_config_widget {
            text_widget.set_theme(new_theme);
        }
        self.preview.queue_draw();
    }
}

impl Default for ArcConfigWidget {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}
