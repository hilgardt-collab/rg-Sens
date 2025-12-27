//! Synthwave/Outrun configuration widget
//!
//! Provides a tabbed interface for configuring all aspects of the Synthwave display.

use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, CheckButton, DrawingArea, DropDown, Entry, Label, Notebook, Orientation,
    Scale, SpinButton, StringList, ScrolledWindow,
};
use std::cell::RefCell;
use std::rc::Rc;

use crate::ui::shared_font_dialog::shared_font_dialog;
use crate::ui::color_button_widget::ColorButtonWidget;
use crate::ui::synthwave_display::{
    render_synthwave_frame, SynthwaveFrameStyle,
    GridStyle, SynthwaveHeaderStyle, SynthwaveDividerStyle, SynthwaveColorScheme,
};
use crate::ui::lcars_display::{ContentDisplayType, ContentItemConfig, SplitOrientation, StaticDisplayConfig};
use crate::ui::{
    BarConfigWidget, GraphConfigWidget, TextLineConfigWidget, CoreBarsConfigWidget,
    BackgroundConfigWidget, ArcConfigWidget, SpeedometerConfigWidget, GradientEditor,
    ThemeFontSelector,
};
use crate::ui::theme::FontSource;
use crate::ui::combo_config_base;
use crate::displayers::SynthwaveDisplayConfig;
use crate::core::{FieldMetadata, FieldType, FieldPurpose};

/// Holds references to Theme tab widgets
struct ThemeWidgets {
    // Color scheme preset dropdown
    color_scheme_dropdown: DropDown,
    // Theme colors
    theme_color1_widget: Rc<ColorButtonWidget>,
    theme_color2_widget: Rc<ColorButtonWidget>,
    theme_color3_widget: Rc<ColorButtonWidget>,
    theme_color4_widget: Rc<ColorButtonWidget>,
    // Theme gradient
    theme_gradient_editor: Rc<GradientEditor>,
    // Theme fonts
    font1_btn: Button,
    font1_size_spin: SpinButton,
    font2_btn: Button,
    font2_size_spin: SpinButton,
    // Neon glow (keep this for synthwave-specific effect)
    glow_scale: Scale,
}

/// Holds references to Frame tab widgets
struct FrameWidgets {
    style_dropdown: DropDown,
    frame_width_spin: SpinButton,
    corner_radius_spin: SpinButton,
}

/// Holds references to Grid tab widgets
struct GridWidgets {
    show_grid_check: CheckButton,
    grid_style_dropdown: DropDown,
    grid_spacing_spin: SpinButton,
    grid_line_width_spin: SpinButton,
    horizon_scale: Scale,
    perspective_scale: Scale,
    show_sun_check: CheckButton,
    sun_position_scale: Scale,
}

/// Holds references to Header tab widgets
struct HeaderWidgets {
    show_header_check: CheckButton,
    header_text_entry: Entry,
    header_style_dropdown: DropDown,
    header_height_spin: SpinButton,
    header_font_selector: Rc<ThemeFontSelector>,
}

/// Holds references to Layout tab widgets
struct LayoutWidgets {
    split_orientation_dropdown: DropDown,
    content_padding_spin: SpinButton,
    item_spacing_spin: SpinButton,
    divider_style_dropdown: DropDown,
    divider_padding_spin: SpinButton,
    group_weights_box: GtkBox,
}

/// Holds references to Animation tab widgets
struct AnimationWidgets {
    enable_check: CheckButton,
    speed_spin: SpinButton,
    scanline_check: CheckButton,
}

/// Synthwave configuration widget
pub struct SynthwaveConfigWidget {
    container: GtkBox,
    config: Rc<RefCell<SynthwaveDisplayConfig>>,
    on_change: Rc<RefCell<Option<Box<dyn Fn()>>>>,
    preview: DrawingArea,
    content_notebook: Rc<RefCell<Notebook>>,
    source_summaries: Rc<RefCell<Vec<(String, String, usize, u32)>>>,
    available_fields: Rc<RefCell<Vec<FieldMetadata>>>,
    theme_widgets: Rc<RefCell<Option<ThemeWidgets>>>,
    frame_widgets: Rc<RefCell<Option<FrameWidgets>>>,
    grid_widgets: Rc<RefCell<Option<GridWidgets>>>,
    header_widgets: Rc<RefCell<Option<HeaderWidgets>>>,
    layout_widgets: Rc<RefCell<Option<LayoutWidgets>>>,
    animation_widgets: Rc<RefCell<Option<AnimationWidgets>>>,
    /// Callbacks to refresh theme reference sections when theme changes
    #[allow(dead_code)] // Kept for Rc ownership; callbacks are invoked via clones
    theme_ref_refreshers: Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
}

impl SynthwaveConfigWidget {
    pub fn new(available_fields: Vec<FieldMetadata>) -> Self {
        let container = GtkBox::new(Orientation::Vertical, 12);
        let config = Rc::new(RefCell::new(SynthwaveDisplayConfig::default()));
        let on_change: Rc<RefCell<Option<Box<dyn Fn()>>>> = Rc::new(RefCell::new(None));
        let source_summaries: Rc<RefCell<Vec<(String, String, usize, u32)>>> = Rc::new(RefCell::new(Vec::new()));
        let available_fields: Rc<RefCell<Vec<FieldMetadata>>> = Rc::new(RefCell::new(available_fields));
        let theme_widgets: Rc<RefCell<Option<ThemeWidgets>>> = Rc::new(RefCell::new(None));
        let frame_widgets: Rc<RefCell<Option<FrameWidgets>>> = Rc::new(RefCell::new(None));
        let grid_widgets: Rc<RefCell<Option<GridWidgets>>> = Rc::new(RefCell::new(None));
        let header_widgets: Rc<RefCell<Option<HeaderWidgets>>> = Rc::new(RefCell::new(None));
        let layout_widgets: Rc<RefCell<Option<LayoutWidgets>>> = Rc::new(RefCell::new(None));
        let animation_widgets: Rc<RefCell<Option<AnimationWidgets>>> = Rc::new(RefCell::new(None));
        let theme_ref_refreshers: Rc<RefCell<Vec<Rc<dyn Fn()>>>> = Rc::new(RefCell::new(Vec::new()));

        // Preview at the top
        let preview = DrawingArea::new();
        preview.set_content_height(180);
        preview.set_vexpand(false);

        let config_clone = config.clone();
        preview.set_draw_func(move |_, cr, width, height| {
            if width < 10 || height < 10 {
                return;
            }

            let cfg = config_clone.borrow();
            let _ = render_synthwave_frame(cr, &cfg.frame, width as f64, height as f64);
        });

        // Theme reference section - placed under preview for easy access from all tabs
        let (theme_ref_section, main_theme_refresh_cb) = combo_config_base::create_theme_reference_section(
            &config,
            |cfg| cfg.frame.theme.clone(),
        );
        theme_ref_refreshers.borrow_mut().push(main_theme_refresh_cb);

        // Create notebook for tabbed interface
        let notebook = Notebook::new();
        notebook.set_vexpand(true);

        // Tab 1: Theme
        let theme_page = Self::create_theme_page(&config, &on_change, &preview, &theme_widgets, &theme_ref_refreshers);
        notebook.append_page(&theme_page, Some(&Label::new(Some("Theme"))));

        // Tab 2: Frame
        let frame_page = Self::create_frame_page(&config, &on_change, &preview, &frame_widgets);
        notebook.append_page(&frame_page, Some(&Label::new(Some("Frame"))));

        // Tab 3: Grid
        let grid_page = Self::create_grid_page(&config, &on_change, &preview, &grid_widgets);
        notebook.append_page(&grid_page, Some(&Label::new(Some("Grid"))));

        // Tab 4: Header
        let header_page = Self::create_header_page(&config, &on_change, &preview, &header_widgets);
        notebook.append_page(&header_page, Some(&Label::new(Some("Header"))));

        // Tab 5: Layout
        let layout_page = Self::create_layout_page(&config, &on_change, &preview, &layout_widgets);
        notebook.append_page(&layout_page, Some(&Label::new(Some("Layout"))));

        // Tab 6: Content
        let content_notebook = Rc::new(RefCell::new(Notebook::new()));
        let content_page = combo_config_base::create_content_page(
            &config,
            &on_change,
            &preview,
            &content_notebook,
            &source_summaries,
            &available_fields,
            |cfg| &cfg.frame.content_items,
        );
        notebook.append_page(&content_page, Some(&Label::new(Some("Content"))));

        // Tab 7: Animation
        let animation_page = Self::create_animation_page(&config, &on_change, &animation_widgets);
        notebook.append_page(&animation_page, Some(&Label::new(Some("Animation"))));

        container.append(&preview);
        container.append(&theme_ref_section);
        container.append(&notebook);

        Self {
            container,
            config,
            on_change,
            preview,
            content_notebook,
            source_summaries,
            available_fields,
            theme_widgets,
            frame_widgets,
            grid_widgets,
            header_widgets,
            layout_widgets,
            animation_widgets,
            theme_ref_refreshers,
        }
    }

    fn set_page_margins(page: &GtkBox) {
        combo_config_base::set_page_margins(page);
    }

    fn queue_redraw(
        preview: &DrawingArea,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
    ) {
        combo_config_base::queue_redraw(preview, on_change);
    }

    /// Refresh all theme reference sections
    fn refresh_theme_refs(refreshers: &Rc<RefCell<Vec<Rc<dyn Fn()>>>>) {
        combo_config_base::refresh_theme_refs(refreshers);
    }

    fn create_theme_page(
        config: &Rc<RefCell<SynthwaveDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        theme_widgets_out: &Rc<RefCell<Option<ThemeWidgets>>>,
        theme_ref_refreshers: &Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        Self::set_page_margins(&page);

        // Color Scheme Preset dropdown
        let scheme_box = GtkBox::new(Orientation::Horizontal, 6);
        scheme_box.append(&Label::new(Some("Color Scheme:")));
        let scheme_list = StringList::new(&["Classic", "Sunset", "Night Drive", "Miami", "Custom"]);
        let color_scheme_dropdown = DropDown::new(Some(scheme_list), None::<gtk4::Expression>);
        let scheme_idx = match &config.borrow().frame.color_scheme {
            SynthwaveColorScheme::Classic => 0,
            SynthwaveColorScheme::Sunset => 1,
            SynthwaveColorScheme::NightDrive => 2,
            SynthwaveColorScheme::Miami => 3,
            SynthwaveColorScheme::Custom { .. } => 4,
        };
        color_scheme_dropdown.set_selected(scheme_idx);
        color_scheme_dropdown.set_hexpand(true);
        scheme_box.append(&color_scheme_dropdown);
        page.append(&scheme_box);

        // Theme Colors section
        let colors_label = Label::new(Some("Theme Colors"));
        colors_label.set_halign(gtk4::Align::Start);
        colors_label.add_css_class("heading");
        colors_label.set_margin_top(8);
        page.append(&colors_label);

        // Color 1 (Primary)
        let color1_box = GtkBox::new(Orientation::Horizontal, 6);
        color1_box.append(&Label::new(Some("Color 1 (Primary):")));
        let theme_color1_widget = Rc::new(ColorButtonWidget::new(config.borrow().frame.theme.color1));
        color1_box.append(theme_color1_widget.widget());
        page.append(&color1_box);

        // Color 2 (Secondary)
        let color2_box = GtkBox::new(Orientation::Horizontal, 6);
        color2_box.append(&Label::new(Some("Color 2 (Secondary):")));
        let theme_color2_widget = Rc::new(ColorButtonWidget::new(config.borrow().frame.theme.color2));
        color2_box.append(theme_color2_widget.widget());
        page.append(&color2_box);

        // Color 3 (Accent)
        let color3_box = GtkBox::new(Orientation::Horizontal, 6);
        color3_box.append(&Label::new(Some("Color 3 (Accent):")));
        let theme_color3_widget = Rc::new(ColorButtonWidget::new(config.borrow().frame.theme.color3));
        color3_box.append(theme_color3_widget.widget());
        page.append(&color3_box);

        // Color 4 (Highlight)
        let color4_box = GtkBox::new(Orientation::Horizontal, 6);
        color4_box.append(&Label::new(Some("Color 4 (Highlight):")));
        let theme_color4_widget = Rc::new(ColorButtonWidget::new(config.borrow().frame.theme.color4));
        color4_box.append(theme_color4_widget.widget());
        page.append(&color4_box);

        // Connect color widget callbacks
        let config_c1 = config.clone();
        let on_change_c1 = on_change.clone();
        let preview_c1 = preview.clone();
        let refreshers_c1 = theme_ref_refreshers.clone();
        theme_color1_widget.set_on_change(move |color| {
            config_c1.borrow_mut().frame.theme.color1 = color;
            Self::queue_redraw(&preview_c1, &on_change_c1);
            Self::refresh_theme_refs(&refreshers_c1);
        });

        let config_c2 = config.clone();
        let on_change_c2 = on_change.clone();
        let preview_c2 = preview.clone();
        let refreshers_c2 = theme_ref_refreshers.clone();
        theme_color2_widget.set_on_change(move |color| {
            config_c2.borrow_mut().frame.theme.color2 = color;
            Self::queue_redraw(&preview_c2, &on_change_c2);
            Self::refresh_theme_refs(&refreshers_c2);
        });

        let config_c3 = config.clone();
        let on_change_c3 = on_change.clone();
        let preview_c3 = preview.clone();
        let refreshers_c3 = theme_ref_refreshers.clone();
        theme_color3_widget.set_on_change(move |color| {
            config_c3.borrow_mut().frame.theme.color3 = color;
            Self::queue_redraw(&preview_c3, &on_change_c3);
            Self::refresh_theme_refs(&refreshers_c3);
        });

        let config_c4 = config.clone();
        let on_change_c4 = on_change.clone();
        let preview_c4 = preview.clone();
        let refreshers_c4 = theme_ref_refreshers.clone();
        theme_color4_widget.set_on_change(move |color| {
            config_c4.borrow_mut().frame.theme.color4 = color;
            Self::queue_redraw(&preview_c4, &on_change_c4);
            Self::refresh_theme_refs(&refreshers_c4);
        });

        // Connect color scheme dropdown - auto-populate theme colors when preset selected
        let config_scheme = config.clone();
        let on_change_scheme = on_change.clone();
        let preview_scheme = preview.clone();
        let refreshers_scheme = theme_ref_refreshers.clone();
        let color1_widget_for_scheme = theme_color1_widget.clone();
        let color2_widget_for_scheme = theme_color2_widget.clone();
        let color3_widget_for_scheme = theme_color3_widget.clone();
        let color4_widget_for_scheme = theme_color4_widget.clone();
        color_scheme_dropdown.connect_selected_notify(move |dropdown| {
            let selected = dropdown.selected();
            if selected == gtk4::INVALID_LIST_POSITION {
                return;
            }

            // Get colors from the selected scheme
            let scheme = match selected {
                0 => SynthwaveColorScheme::Classic,
                1 => SynthwaveColorScheme::Sunset,
                2 => SynthwaveColorScheme::NightDrive,
                3 => SynthwaveColorScheme::Miami,
                _ => {
                    // Custom - don't auto-populate, just set the scheme
                    let current = config_scheme.borrow();
                    let primary = current.frame.theme.color1;
                    let secondary = current.frame.theme.color2;
                    let accent = current.frame.theme.color3;
                    drop(current);
                    config_scheme.borrow_mut().frame.color_scheme = SynthwaveColorScheme::Custom {
                        primary,
                        secondary,
                        accent,
                    };
                    Self::queue_redraw(&preview_scheme, &on_change_scheme);
                    Self::refresh_theme_refs(&refreshers_scheme);
                    return;
                }
            };

            // Auto-populate theme colors from the selected scheme
            let primary = scheme.primary();
            let secondary = scheme.secondary();
            let accent = scheme.accent();
            let (bg_top, _bg_bottom) = scheme.background_gradient();

            // Update the theme colors
            {
                let mut cfg = config_scheme.borrow_mut();
                cfg.frame.color_scheme = scheme;
                cfg.frame.theme.color1 = primary;
                cfg.frame.theme.color2 = secondary;
                cfg.frame.theme.color3 = accent;
                cfg.frame.theme.color4 = bg_top; // Use background top color as highlight
            }

            // Update color widget displays
            color1_widget_for_scheme.set_color(primary);
            color2_widget_for_scheme.set_color(secondary);
            color3_widget_for_scheme.set_color(accent);
            color4_widget_for_scheme.set_color(bg_top);

            Self::queue_redraw(&preview_scheme, &on_change_scheme);
            Self::refresh_theme_refs(&refreshers_scheme);
        });

        // Theme Gradient section
        let gradient_label = Label::new(Some("Theme Gradient"));
        gradient_label.set_halign(gtk4::Align::Start);
        gradient_label.add_css_class("heading");
        gradient_label.set_margin_top(12);
        page.append(&gradient_label);

        let theme_gradient_editor = Rc::new(GradientEditor::new());
        theme_gradient_editor.set_gradient(&config.borrow().frame.theme.gradient);
        page.append(theme_gradient_editor.widget());

        let config_grad = config.clone();
        let on_change_grad = on_change.clone();
        let preview_grad = preview.clone();
        let refreshers_grad = theme_ref_refreshers.clone();
        let gradient_editor_clone = theme_gradient_editor.clone();
        theme_gradient_editor.set_on_change(move || {
            config_grad.borrow_mut().frame.theme.gradient = gradient_editor_clone.get_gradient();
            Self::queue_redraw(&preview_grad, &on_change_grad);
            Self::refresh_theme_refs(&refreshers_grad);
        });

        // Theme Fonts section
        let fonts_label = Label::new(Some("Theme Fonts"));
        fonts_label.set_halign(gtk4::Align::Start);
        fonts_label.add_css_class("heading");
        fonts_label.set_margin_top(12);
        page.append(&fonts_label);

        // Font 1
        let font1_box = GtkBox::new(Orientation::Horizontal, 6);
        font1_box.append(&Label::new(Some("Font 1:")));
        let font1_btn = Button::with_label(&config.borrow().frame.theme.font1_family);
        font1_btn.set_hexpand(true);
        font1_box.append(&font1_btn);
        font1_box.append(&Label::new(Some("Size:")));
        let font1_size_spin = SpinButton::with_range(6.0, 72.0, 1.0);
        font1_size_spin.set_value(config.borrow().frame.theme.font1_size);
        font1_box.append(&font1_size_spin);
        page.append(&font1_box);

        // Font 1 button click handler
        let config_f1 = config.clone();
        let on_change_f1 = on_change.clone();
        let preview_f1 = preview.clone();
        let refreshers_f1 = theme_ref_refreshers.clone();
        let font1_btn_clone = font1_btn.clone();
        font1_btn.connect_clicked(move |button| {
            let config_for_cb = config_f1.clone();
            let on_change_for_cb = on_change_f1.clone();
            let preview_for_cb = preview_f1.clone();
            let refreshers_for_cb = refreshers_f1.clone();
            let font_btn_for_cb = font1_btn_clone.clone();
            if let Some(window) = button.root().and_then(|r| r.downcast::<gtk4::Window>().ok()) {
                let current_font = config_for_cb.borrow().frame.theme.font1_family.clone();
                let font_desc = gtk4::pango::FontDescription::from_string(&current_font);
                shared_font_dialog().choose_font(
                    Some(&window),
                    Some(&font_desc),
                    gtk4::gio::Cancellable::NONE,
                    move |result| {
                        if let Ok(font_desc) = result {
                            let family = font_desc.family()
                                .map(|s| s.to_string())
                                .unwrap_or_else(|| "sans-serif".to_string());
                            config_for_cb.borrow_mut().frame.theme.font1_family = family.clone();
                            font_btn_for_cb.set_label(&family);
                            Self::queue_redraw(&preview_for_cb, &on_change_for_cb);
                            Self::refresh_theme_refs(&refreshers_for_cb);
                        }
                    },
                );
            }
        });

        // Font 1 size spin handler
        let config_f1s = config.clone();
        let on_change_f1s = on_change.clone();
        let preview_f1s = preview.clone();
        let refreshers_f1s = theme_ref_refreshers.clone();
        font1_size_spin.connect_value_changed(move |spin| {
            config_f1s.borrow_mut().frame.theme.font1_size = spin.value();
            Self::queue_redraw(&preview_f1s, &on_change_f1s);
            Self::refresh_theme_refs(&refreshers_f1s);
        });

        // Font 2
        let font2_box = GtkBox::new(Orientation::Horizontal, 6);
        font2_box.append(&Label::new(Some("Font 2:")));
        let font2_btn = Button::with_label(&config.borrow().frame.theme.font2_family);
        font2_btn.set_hexpand(true);
        font2_box.append(&font2_btn);
        font2_box.append(&Label::new(Some("Size:")));
        let font2_size_spin = SpinButton::with_range(6.0, 72.0, 1.0);
        font2_size_spin.set_value(config.borrow().frame.theme.font2_size);
        font2_box.append(&font2_size_spin);
        page.append(&font2_box);

        // Font 2 button click handler
        let config_f2 = config.clone();
        let on_change_f2 = on_change.clone();
        let preview_f2 = preview.clone();
        let refreshers_f2 = theme_ref_refreshers.clone();
        let font2_btn_clone = font2_btn.clone();
        font2_btn.connect_clicked(move |button| {
            let config_for_cb = config_f2.clone();
            let on_change_for_cb = on_change_f2.clone();
            let preview_for_cb = preview_f2.clone();
            let refreshers_for_cb = refreshers_f2.clone();
            let font_btn_for_cb = font2_btn_clone.clone();
            if let Some(window) = button.root().and_then(|r| r.downcast::<gtk4::Window>().ok()) {
                let current_font = config_for_cb.borrow().frame.theme.font2_family.clone();
                let font_desc = gtk4::pango::FontDescription::from_string(&current_font);
                shared_font_dialog().choose_font(
                    Some(&window),
                    Some(&font_desc),
                    gtk4::gio::Cancellable::NONE,
                    move |result| {
                        if let Ok(font_desc) = result {
                            let family = font_desc.family()
                                .map(|s| s.to_string())
                                .unwrap_or_else(|| "sans-serif".to_string());
                            config_for_cb.borrow_mut().frame.theme.font2_family = family.clone();
                            font_btn_for_cb.set_label(&family);
                            Self::queue_redraw(&preview_for_cb, &on_change_for_cb);
                            Self::refresh_theme_refs(&refreshers_for_cb);
                        }
                    },
                );
            }
        });

        // Font 2 size spin handler
        let config_f2s = config.clone();
        let on_change_f2s = on_change.clone();
        let preview_f2s = preview.clone();
        let refreshers_f2s = theme_ref_refreshers.clone();
        font2_size_spin.connect_value_changed(move |spin| {
            config_f2s.borrow_mut().frame.theme.font2_size = spin.value();
            Self::queue_redraw(&preview_f2s, &on_change_f2s);
            Self::refresh_theme_refs(&refreshers_f2s);
        });

        // Effects section (Neon glow - Synthwave-specific)
        let effects_label = Label::new(Some("Effects"));
        effects_label.set_halign(gtk4::Align::Start);
        effects_label.add_css_class("heading");
        effects_label.set_margin_top(12);
        page.append(&effects_label);

        let glow_box = GtkBox::new(Orientation::Horizontal, 6);
        glow_box.append(&Label::new(Some("Neon Glow:")));
        let glow_scale = Scale::with_range(Orientation::Horizontal, 0.0, 1.0, 0.05);
        glow_scale.set_value(config.borrow().frame.neon_glow_intensity);
        glow_scale.set_hexpand(true);
        glow_scale.set_draw_value(true);
        glow_box.append(&glow_scale);

        let config_glow = config.clone();
        let on_change_glow = on_change.clone();
        let preview_glow = preview.clone();
        glow_scale.connect_value_changed(move |scale| {
            config_glow.borrow_mut().frame.neon_glow_intensity = scale.value();
            Self::queue_redraw(&preview_glow, &on_change_glow);
        });
        page.append(&glow_box);

        // Store widget refs
        *theme_widgets_out.borrow_mut() = Some(ThemeWidgets {
            color_scheme_dropdown,
            theme_color1_widget,
            theme_color2_widget,
            theme_color3_widget,
            theme_color4_widget,
            theme_gradient_editor,
            font1_btn,
            font1_size_spin,
            font2_btn,
            font2_size_spin,
            glow_scale,
        });

        page
    }

    fn create_frame_page(
        config: &Rc<RefCell<SynthwaveDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        frame_widgets_out: &Rc<RefCell<Option<FrameWidgets>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        Self::set_page_margins(&page);

        // Frame style
        let style_box = GtkBox::new(Orientation::Horizontal, 6);
        style_box.append(&Label::new(Some("Frame Style:")));
        let style_list = StringList::new(&["Neon Border", "Chrome", "Minimal", "Retro Double", "None"]);
        let style_dropdown = DropDown::new(Some(style_list), None::<gtk4::Expression>);
        let style_idx = match config.borrow().frame.frame_style {
            SynthwaveFrameStyle::NeonBorder => 0,
            SynthwaveFrameStyle::Chrome => 1,
            SynthwaveFrameStyle::Minimal => 2,
            SynthwaveFrameStyle::RetroDouble => 3,
            SynthwaveFrameStyle::None => 4,
        };
        style_dropdown.set_selected(style_idx);
        style_dropdown.set_hexpand(true);
        style_box.append(&style_dropdown);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        style_dropdown.connect_selected_notify(move |dropdown| {
            let selected = dropdown.selected();
            if selected == gtk4::INVALID_LIST_POSITION {
                return;
            }
            config_clone.borrow_mut().frame.frame_style = match selected {
                0 => SynthwaveFrameStyle::NeonBorder,
                1 => SynthwaveFrameStyle::Chrome,
                2 => SynthwaveFrameStyle::Minimal,
                3 => SynthwaveFrameStyle::RetroDouble,
                _ => SynthwaveFrameStyle::None,
            };
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&style_box);

        // Frame width
        let width_box = GtkBox::new(Orientation::Horizontal, 6);
        width_box.append(&Label::new(Some("Frame Width:")));
        let frame_width_spin = SpinButton::with_range(0.5, 6.0, 0.5);
        frame_width_spin.set_value(config.borrow().frame.frame_width);
        frame_width_spin.set_hexpand(true);
        width_box.append(&frame_width_spin);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        frame_width_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().frame.frame_width = spin.value();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&width_box);

        // Corner radius
        let radius_box = GtkBox::new(Orientation::Horizontal, 6);
        radius_box.append(&Label::new(Some("Corner Radius:")));
        let corner_radius_spin = SpinButton::with_range(0.0, 30.0, 2.0);
        corner_radius_spin.set_value(config.borrow().frame.corner_radius);
        corner_radius_spin.set_hexpand(true);
        radius_box.append(&corner_radius_spin);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        corner_radius_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().frame.corner_radius = spin.value();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&radius_box);

        // Store widget refs
        *frame_widgets_out.borrow_mut() = Some(FrameWidgets {
            style_dropdown,
            frame_width_spin,
            corner_radius_spin,
        });

        page
    }

    fn create_grid_page(
        config: &Rc<RefCell<SynthwaveDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        grid_widgets_out: &Rc<RefCell<Option<GridWidgets>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        Self::set_page_margins(&page);

        // Show grid
        let show_grid_check = CheckButton::with_label("Show Grid");
        show_grid_check.set_active(config.borrow().frame.show_grid);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        show_grid_check.connect_toggled(move |check| {
            config_clone.borrow_mut().frame.show_grid = check.is_active();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&show_grid_check);

        // Grid style
        let style_box = GtkBox::new(Orientation::Horizontal, 6);
        style_box.append(&Label::new(Some("Grid Style:")));
        let style_list = StringList::new(&["Perspective", "Flat", "Hexagon", "Scanlines", "None"]);
        let grid_style_dropdown = DropDown::new(Some(style_list), None::<gtk4::Expression>);
        let style_idx = match config.borrow().frame.grid_style {
            GridStyle::Perspective => 0,
            GridStyle::Flat => 1,
            GridStyle::Hexagon => 2,
            GridStyle::Scanlines => 3,
            GridStyle::None => 4,
        };
        grid_style_dropdown.set_selected(style_idx);
        grid_style_dropdown.set_hexpand(true);
        style_box.append(&grid_style_dropdown);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        grid_style_dropdown.connect_selected_notify(move |dropdown| {
            let selected = dropdown.selected();
            if selected == gtk4::INVALID_LIST_POSITION {
                return;
            }
            config_clone.borrow_mut().frame.grid_style = match selected {
                0 => GridStyle::Perspective,
                1 => GridStyle::Flat,
                2 => GridStyle::Hexagon,
                3 => GridStyle::Scanlines,
                _ => GridStyle::None,
            };
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&style_box);

        // Grid spacing
        let spacing_box = GtkBox::new(Orientation::Horizontal, 6);
        spacing_box.append(&Label::new(Some("Grid Spacing:")));
        let grid_spacing_spin = SpinButton::with_range(10.0, 100.0, 5.0);
        grid_spacing_spin.set_value(config.borrow().frame.grid_spacing);
        grid_spacing_spin.set_hexpand(true);
        spacing_box.append(&grid_spacing_spin);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        grid_spacing_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().frame.grid_spacing = spin.value();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&spacing_box);

        // Grid line width
        let line_box = GtkBox::new(Orientation::Horizontal, 6);
        line_box.append(&Label::new(Some("Line Width:")));
        let grid_line_width_spin = SpinButton::with_range(0.5, 4.0, 0.5);
        grid_line_width_spin.set_value(config.borrow().frame.grid_line_width);
        grid_line_width_spin.set_hexpand(true);
        line_box.append(&grid_line_width_spin);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        grid_line_width_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().frame.grid_line_width = spin.value();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&line_box);

        // Horizon position
        let horizon_box = GtkBox::new(Orientation::Horizontal, 6);
        horizon_box.append(&Label::new(Some("Horizon:")));
        let horizon_scale = Scale::with_range(Orientation::Horizontal, 0.1, 0.9, 0.05);
        horizon_scale.set_value(config.borrow().frame.grid_horizon);
        horizon_scale.set_hexpand(true);
        horizon_scale.set_draw_value(true);
        horizon_box.append(&horizon_scale);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        horizon_scale.connect_value_changed(move |scale| {
            config_clone.borrow_mut().frame.grid_horizon = scale.value();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&horizon_box);

        // Perspective intensity
        let perspective_box = GtkBox::new(Orientation::Horizontal, 6);
        perspective_box.append(&Label::new(Some("Perspective:")));
        let perspective_scale = Scale::with_range(Orientation::Horizontal, 0.0, 1.0, 0.1);
        perspective_scale.set_value(config.borrow().frame.grid_perspective);
        perspective_scale.set_hexpand(true);
        perspective_scale.set_draw_value(true);
        perspective_box.append(&perspective_scale);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        perspective_scale.connect_value_changed(move |scale| {
            config_clone.borrow_mut().frame.grid_perspective = scale.value();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&perspective_box);

        // Sun section
        let sun_label = Label::new(Some("Sun Effect"));
        sun_label.set_halign(gtk4::Align::Start);
        sun_label.add_css_class("heading");
        sun_label.set_margin_top(12);
        page.append(&sun_label);

        let show_sun_check = CheckButton::with_label("Show Sun");
        show_sun_check.set_active(config.borrow().frame.show_sun);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        show_sun_check.connect_toggled(move |check| {
            config_clone.borrow_mut().frame.show_sun = check.is_active();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&show_sun_check);

        // Sun position
        let sun_pos_box = GtkBox::new(Orientation::Horizontal, 6);
        sun_pos_box.append(&Label::new(Some("Sun Position:")));
        let sun_position_scale = Scale::with_range(Orientation::Horizontal, 0.0, 1.0, 0.1);
        sun_position_scale.set_value(config.borrow().frame.sun_position);
        sun_position_scale.set_hexpand(true);
        sun_position_scale.set_draw_value(true);
        sun_pos_box.append(&sun_position_scale);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        sun_position_scale.connect_value_changed(move |scale| {
            config_clone.borrow_mut().frame.sun_position = scale.value();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&sun_pos_box);

        // Store widget refs
        *grid_widgets_out.borrow_mut() = Some(GridWidgets {
            show_grid_check,
            grid_style_dropdown,
            grid_spacing_spin,
            grid_line_width_spin,
            horizon_scale,
            perspective_scale,
            show_sun_check,
            sun_position_scale,
        });

        page
    }

    fn create_header_page(
        config: &Rc<RefCell<SynthwaveDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        header_widgets_out: &Rc<RefCell<Option<HeaderWidgets>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        Self::set_page_margins(&page);

        // Show header
        let show_header_check = CheckButton::with_label("Show Header");
        show_header_check.set_active(config.borrow().frame.show_header);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        show_header_check.connect_toggled(move |check| {
            config_clone.borrow_mut().frame.show_header = check.is_active();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&show_header_check);

        // Header text
        let text_box = GtkBox::new(Orientation::Horizontal, 6);
        text_box.append(&Label::new(Some("Header Text:")));
        let header_text_entry = Entry::new();
        header_text_entry.set_text(&config.borrow().frame.header_text);
        header_text_entry.set_hexpand(true);
        text_box.append(&header_text_entry);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        header_text_entry.connect_changed(move |entry| {
            config_clone.borrow_mut().frame.header_text = entry.text().to_string();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&text_box);

        // Header style
        let style_box = GtkBox::new(Orientation::Horizontal, 6);
        style_box.append(&Label::new(Some("Header Style:")));
        let style_list = StringList::new(&["Chrome", "Neon", "Outline", "Simple", "None"]);
        let header_style_dropdown = DropDown::new(Some(style_list), None::<gtk4::Expression>);
        let style_idx = match config.borrow().frame.header_style {
            SynthwaveHeaderStyle::Chrome => 0,
            SynthwaveHeaderStyle::Neon => 1,
            SynthwaveHeaderStyle::Outline => 2,
            SynthwaveHeaderStyle::Simple => 3,
            SynthwaveHeaderStyle::None => 4,
        };
        header_style_dropdown.set_selected(style_idx);
        header_style_dropdown.set_hexpand(true);
        style_box.append(&header_style_dropdown);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        header_style_dropdown.connect_selected_notify(move |dropdown| {
            let selected = dropdown.selected();
            if selected == gtk4::INVALID_LIST_POSITION {
                return;
            }
            config_clone.borrow_mut().frame.header_style = match selected {
                0 => SynthwaveHeaderStyle::Chrome,
                1 => SynthwaveHeaderStyle::Neon,
                2 => SynthwaveHeaderStyle::Outline,
                3 => SynthwaveHeaderStyle::Simple,
                _ => SynthwaveHeaderStyle::None,
            };
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&style_box);

        // Header height
        let height_box = GtkBox::new(Orientation::Horizontal, 6);
        height_box.append(&Label::new(Some("Header Height:")));
        let header_height_spin = SpinButton::with_range(20.0, 60.0, 2.0);
        header_height_spin.set_value(config.borrow().frame.header_height);
        header_height_spin.set_hexpand(true);
        height_box.append(&header_height_spin);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        header_height_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().frame.header_height = spin.value();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&height_box);

        // Font section with theme selector
        let font_label = Label::new(Some("Font"));
        font_label.set_halign(gtk4::Align::Start);
        font_label.add_css_class("heading");
        font_label.set_margin_top(12);
        page.append(&font_label);

        // Create ThemeFontSelector with current font as custom
        let current_font = config.borrow().frame.header_font.clone();
        let current_size = config.borrow().frame.header_font_size;
        let header_font_selector = Rc::new(ThemeFontSelector::new(
            FontSource::Custom { family: current_font, size: current_size }
        ));

        // Set theme config so selector can show theme font names
        header_font_selector.set_theme_config(config.borrow().frame.theme.clone());

        let font_box = GtkBox::new(Orientation::Horizontal, 6);
        font_box.append(header_font_selector.widget());
        page.append(&font_box);

        // Connect font selector callback
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        header_font_selector.set_on_change(move |source| {
            let (family, size) = match &source {
                FontSource::Theme { index } => {
                    let cfg = config_clone.borrow();
                    cfg.frame.theme.get_font(*index)
                }
                FontSource::Custom { family, size } => (family.clone(), *size),
            };
            {
                let mut cfg = config_clone.borrow_mut();
                cfg.frame.header_font = family;
                cfg.frame.header_font_size = size;
            }
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });

        // Store widget refs
        *header_widgets_out.borrow_mut() = Some(HeaderWidgets {
            show_header_check,
            header_text_entry,
            header_style_dropdown,
            header_height_spin,
            header_font_selector,
        });

        page
    }

    fn create_layout_page(
        config: &Rc<RefCell<SynthwaveDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        layout_widgets_out: &Rc<RefCell<Option<LayoutWidgets>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        Self::set_page_margins(&page);

        // Split orientation
        let orient_box = GtkBox::new(Orientation::Horizontal, 6);
        orient_box.append(&Label::new(Some("Split Orientation:")));
        let orient_list = StringList::new(&["Vertical", "Horizontal"]);
        let split_orientation_dropdown = DropDown::new(Some(orient_list), None::<gtk4::Expression>);
        let orient_idx = match config.borrow().frame.split_orientation {
            SplitOrientation::Vertical => 0,
            SplitOrientation::Horizontal => 1,
        };
        split_orientation_dropdown.set_selected(orient_idx);
        split_orientation_dropdown.set_hexpand(true);
        orient_box.append(&split_orientation_dropdown);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        split_orientation_dropdown.connect_selected_notify(move |dropdown| {
            let selected = dropdown.selected();
            if selected == gtk4::INVALID_LIST_POSITION {
                return;
            }
            config_clone.borrow_mut().frame.split_orientation = match selected {
                0 => SplitOrientation::Vertical,
                _ => SplitOrientation::Horizontal,
            };
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&orient_box);

        // Content padding
        let padding_box = GtkBox::new(Orientation::Horizontal, 6);
        padding_box.append(&Label::new(Some("Content Padding:")));
        let content_padding_spin = SpinButton::with_range(4.0, 32.0, 2.0);
        content_padding_spin.set_value(config.borrow().frame.content_padding);
        content_padding_spin.set_hexpand(true);
        padding_box.append(&content_padding_spin);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        content_padding_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().frame.content_padding = spin.value();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&padding_box);

        // Item spacing
        let spacing_box = GtkBox::new(Orientation::Horizontal, 6);
        spacing_box.append(&Label::new(Some("Item Spacing:")));
        let item_spacing_spin = SpinButton::with_range(0.0, 20.0, 1.0);
        item_spacing_spin.set_value(config.borrow().frame.item_spacing);
        item_spacing_spin.set_hexpand(true);
        spacing_box.append(&item_spacing_spin);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        item_spacing_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().frame.item_spacing = spin.value();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&spacing_box);

        // Divider section
        let divider_label = Label::new(Some("Dividers"));
        divider_label.set_halign(gtk4::Align::Start);
        divider_label.add_css_class("heading");
        divider_label.set_margin_top(12);
        page.append(&divider_label);

        // Divider style
        let div_style_box = GtkBox::new(Orientation::Horizontal, 6);
        div_style_box.append(&Label::new(Some("Divider Style:")));
        let div_style_list = StringList::new(&["Neon Line", "Gradient", "Neon Dots", "Line", "None"]);
        let divider_style_dropdown = DropDown::new(Some(div_style_list), None::<gtk4::Expression>);
        let div_style_idx = match config.borrow().frame.divider_style {
            SynthwaveDividerStyle::NeonLine => 0,
            SynthwaveDividerStyle::Gradient => 1,
            SynthwaveDividerStyle::NeonDots => 2,
            SynthwaveDividerStyle::Line => 3,
            SynthwaveDividerStyle::None => 4,
        };
        divider_style_dropdown.set_selected(div_style_idx);
        divider_style_dropdown.set_hexpand(true);
        div_style_box.append(&divider_style_dropdown);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        divider_style_dropdown.connect_selected_notify(move |dropdown| {
            let selected = dropdown.selected();
            if selected == gtk4::INVALID_LIST_POSITION {
                return;
            }
            config_clone.borrow_mut().frame.divider_style = match selected {
                0 => SynthwaveDividerStyle::NeonLine,
                1 => SynthwaveDividerStyle::Gradient,
                2 => SynthwaveDividerStyle::NeonDots,
                3 => SynthwaveDividerStyle::Line,
                _ => SynthwaveDividerStyle::None,
            };
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&div_style_box);

        // Divider padding
        let div_padding_box = GtkBox::new(Orientation::Horizontal, 6);
        div_padding_box.append(&Label::new(Some("Divider Padding:")));
        let divider_padding_spin = SpinButton::with_range(2.0, 20.0, 1.0);
        divider_padding_spin.set_value(config.borrow().frame.divider_padding);
        divider_padding_spin.set_hexpand(true);
        div_padding_box.append(&divider_padding_spin);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        divider_padding_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().frame.divider_padding = spin.value();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&div_padding_box);

        // Group weights section
        let weights_label = Label::new(Some("Group Size Weights"));
        weights_label.set_halign(gtk4::Align::Start);
        weights_label.add_css_class("heading");
        weights_label.set_margin_top(12);
        page.append(&weights_label);

        let group_weights_box = GtkBox::new(Orientation::Vertical, 4);
        page.append(&group_weights_box);

        // Store widget refs
        *layout_widgets_out.borrow_mut() = Some(LayoutWidgets {
            split_orientation_dropdown,
            content_padding_spin,
            item_spacing_spin,
            divider_style_dropdown,
            divider_padding_spin,
            group_weights_box,
        });

        page
    }

    fn create_animation_page(
        config: &Rc<RefCell<SynthwaveDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        animation_widgets_out: &Rc<RefCell<Option<AnimationWidgets>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        Self::set_page_margins(&page);

        // Enable animation
        let enable_check = CheckButton::with_label("Enable Animations");
        enable_check.set_active(config.borrow().animation_enabled);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        enable_check.connect_toggled(move |check| {
            config_clone.borrow_mut().animation_enabled = check.is_active();
            if let Some(cb) = on_change_clone.borrow().as_ref() {
                cb();
            }
        });
        page.append(&enable_check);

        // Animation speed
        let speed_box = GtkBox::new(Orientation::Horizontal, 6);
        speed_box.append(&Label::new(Some("Animation Speed:")));
        let speed_spin = SpinButton::with_range(1.0, 20.0, 1.0);
        speed_spin.set_value(config.borrow().animation_speed);
        speed_spin.set_hexpand(true);
        speed_box.append(&speed_spin);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        speed_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().animation_speed = spin.value();
            if let Some(cb) = on_change_clone.borrow().as_ref() {
                cb();
            }
        });
        page.append(&speed_box);

        // Scanline effect
        let scanline_check = CheckButton::with_label("Scanline Effect");
        scanline_check.set_active(config.borrow().frame.scanline_effect);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        scanline_check.connect_toggled(move |check| {
            config_clone.borrow_mut().frame.scanline_effect = check.is_active();
            if let Some(cb) = on_change_clone.borrow().as_ref() {
                cb();
            }
        });
        page.append(&scanline_check);

        // Store widget refs
        *animation_widgets_out.borrow_mut() = Some(AnimationWidgets {
            enable_check,
            speed_spin,
            scanline_check,
        });

        page
    }

    fn rebuild_group_spinners(
        config: &Rc<RefCell<SynthwaveDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        group_weights_box: &GtkBox,
    ) {
        // Clear existing spinners
        while let Some(child) = group_weights_box.first_child() {
            group_weights_box.remove(&child);
        }

        let cfg = config.borrow();
        let group_count = cfg.frame.group_count;

        if group_count <= 1 {
            let label = Label::new(Some("Group weights not applicable for single group."));
            label.add_css_class("dim-label");
            group_weights_box.append(&label);
            return;
        }

        // Create spinners for each group
        for i in 0..group_count {
            let row = GtkBox::new(Orientation::Horizontal, 6);
            row.append(&Label::new(Some(&format!("Group {} Weight:", i + 1))));

            let weight = cfg.frame.group_size_weights.get(i).copied().unwrap_or(1.0);
            let spin = SpinButton::with_range(0.1, 10.0, 0.1);
            spin.set_value(weight);
            spin.set_hexpand(true);
            row.append(&spin);

            let config_clone = config.clone();
            let on_change_clone = on_change.clone();
            let preview_clone = preview.clone();
            let idx = i;
            spin.connect_value_changed(move |spin| {
                let mut cfg = config_clone.borrow_mut();
                while cfg.frame.group_size_weights.len() <= idx {
                    cfg.frame.group_size_weights.push(1.0);
                }
                cfg.frame.group_size_weights[idx] = spin.value();
                Self::queue_redraw(&preview_clone, &on_change_clone);
            });

            group_weights_box.append(&row);
        }
    }

    /// Create a theme reference section showing current theme colors and fonts with copy buttons
    /// Returns the frame and a refresh callback that should be called when theme changes
    #[allow(dead_code)] // Deprecated: use combo_config_base::create_theme_reference_section
    fn create_theme_reference_section(
        config: &Rc<RefCell<SynthwaveDisplayConfig>>,
    ) -> (gtk4::Frame, Rc<dyn Fn()>) {
        use crate::ui::clipboard::CLIPBOARD;

        let frame = gtk4::Frame::new(Some("Theme Reference"));
        frame.set_margin_top(8);

        let content_box = GtkBox::new(Orientation::Vertical, 6);
        content_box.set_margin_start(8);
        content_box.set_margin_end(8);
        content_box.set_margin_top(8);
        content_box.set_margin_bottom(8);

        // Colors row
        let colors_box = GtkBox::new(Orientation::Horizontal, 8);
        colors_box.append(&Label::new(Some("Colors:")));

        // Store swatches for refresh
        let color_swatches: Rc<RefCell<Vec<DrawingArea>>> = Rc::new(RefCell::new(Vec::new()));

        let color_indices = [1u8, 2, 3, 4];
        let color_tooltips = ["Color 1 (Primary)", "Color 2 (Secondary)", "Color 3 (Accent)", "Color 4 (Highlight)"];

        for (idx, tooltip) in color_indices.iter().zip(color_tooltips.iter()) {
            let item_box = GtkBox::new(Orientation::Horizontal, 2);

            // Color swatch - reads from config dynamically
            let swatch = DrawingArea::new();
            swatch.set_size_request(20, 20);
            let config_for_draw = config.clone();
            let color_idx = *idx;
            swatch.set_draw_func(move |_, cr, width, height| {
                let c = config_for_draw.borrow().frame.theme.get_color(color_idx);
                // Draw checkerboard for transparency
                let checker_size = 4.0;
                for y in 0..(height as f64 / checker_size).ceil() as i32 {
                    for x in 0..(width as f64 / checker_size).ceil() as i32 {
                        if (x + y) % 2 == 0 {
                            cr.set_source_rgb(0.8, 0.8, 0.8);
                        } else {
                            cr.set_source_rgb(0.6, 0.6, 0.6);
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
                // Draw color
                cr.set_source_rgba(c.r, c.g, c.b, c.a);
                cr.rectangle(0.0, 0.0, width as f64, height as f64);
                let _ = cr.fill();
                // Border
                cr.set_source_rgb(0.3, 0.3, 0.3);
                cr.set_line_width(1.0);
                cr.rectangle(0.5, 0.5, width as f64 - 1.0, height as f64 - 1.0);
                let _ = cr.stroke();
            });
            color_swatches.borrow_mut().push(swatch.clone());
            item_box.append(&swatch);

            // Copy button with icon - reads from config dynamically
            let copy_btn = Button::from_icon_name("edit-copy-symbolic");
            copy_btn.set_tooltip_text(Some(&format!("Copy {} to clipboard", tooltip)));
            let config_for_copy = config.clone();
            let color_idx_for_copy = *idx;
            let tooltip_for_log = tooltip.to_string();
            copy_btn.connect_clicked(move |_| {
                let c = config_for_copy.borrow().frame.theme.get_color(color_idx_for_copy);
                if let Ok(mut clipboard) = CLIPBOARD.lock() {
                    clipboard.copy_color(c.r, c.g, c.b, c.a);
                    log::info!("Theme {} copied to clipboard", tooltip_for_log);
                }
            });
            item_box.append(&copy_btn);

            colors_box.append(&item_box);
        }

        content_box.append(&colors_box);

        // Gradient row
        let gradient_box = GtkBox::new(Orientation::Horizontal, 8);
        gradient_box.append(&Label::new(Some("Gradient:")));

        // Gradient preview swatch - reads from config dynamically
        let gradient_swatch = DrawingArea::new();
        gradient_swatch.set_size_request(60, 20);
        let config_for_gradient = config.clone();
        gradient_swatch.set_draw_func(move |_, cr, width, height| {
            let gradient_config = config_for_gradient.borrow().frame.theme.gradient.clone();
            // Render linear gradient
            let w = width as f64;
            let h = height as f64;
            let angle_rad = gradient_config.angle.to_radians();
            let (dx, dy) = (angle_rad.sin(), -angle_rad.cos());
            let length = (w * dx.abs() + h * dy.abs()) / 2.0;
            let (cx, cy) = (w / 2.0, h / 2.0);
            let (x0, y0) = (cx - dx * length, cy - dy * length);
            let (x1, y1) = (cx + dx * length, cy + dy * length);

            let gradient = gtk4::cairo::LinearGradient::new(x0, y0, x1, y1);
            for stop in &gradient_config.stops {
                gradient.add_color_stop_rgba(stop.position, stop.color.r, stop.color.g, stop.color.b, stop.color.a);
            }
            let _ = cr.set_source(&gradient);
            cr.rectangle(0.0, 0.0, w, h);
            let _ = cr.fill();

            // Border
            cr.set_source_rgb(0.3, 0.3, 0.3);
            cr.set_line_width(1.0);
            cr.rectangle(0.5, 0.5, w - 1.0, h - 1.0);
            let _ = cr.stroke();
        });
        gradient_box.append(&gradient_swatch);

        // Gradient copy button - reads from config dynamically
        let gradient_copy_btn = Button::from_icon_name("edit-copy-symbolic");
        gradient_copy_btn.set_tooltip_text(Some("Copy Theme Gradient to clipboard"));
        let config_for_gradient_copy = config.clone();
        gradient_copy_btn.connect_clicked(move |_| {
            let stops = config_for_gradient_copy.borrow().frame.theme.gradient.stops.clone();
            if let Ok(mut clipboard) = CLIPBOARD.lock() {
                clipboard.copy_gradient_stops(stops);
                log::info!("Theme gradient copied to clipboard");
            }
        });
        gradient_box.append(&gradient_copy_btn);

        content_box.append(&gradient_box);

        // Fonts row
        let fonts_box = GtkBox::new(Orientation::Horizontal, 8);
        fonts_box.append(&Label::new(Some("Fonts:")));

        // Store font labels for refresh
        let font_labels: Rc<RefCell<Vec<Label>>> = Rc::new(RefCell::new(Vec::new()));

        let font_indices = [1u8, 2];
        let font_tooltips = ["Font 1 (Headers)", "Font 2 (Content)"];

        for (idx, tooltip) in font_indices.iter().zip(font_tooltips.iter()) {
            let item_box = GtkBox::new(Orientation::Horizontal, 4);

            // Font info label - will be updated on refresh
            let (family, size) = config.borrow().frame.theme.get_font(*idx);
            let info = Label::new(Some(&format!("{} {}pt", family, size as i32)));
            info.add_css_class("dim-label");
            font_labels.borrow_mut().push(info.clone());
            item_box.append(&info);

            // Copy button with icon - reads from config dynamically
            let copy_btn = Button::from_icon_name("edit-copy-symbolic");
            copy_btn.set_tooltip_text(Some(&format!("Copy {} to clipboard", tooltip)));
            let config_for_copy = config.clone();
            let font_idx = *idx;
            let tooltip_for_log = tooltip.to_string();
            copy_btn.connect_clicked(move |_| {
                let (family, size) = config_for_copy.borrow().frame.theme.get_font(font_idx);
                if let Ok(mut clipboard) = CLIPBOARD.lock() {
                    clipboard.copy_font(family, size, false, false);
                    log::info!("Theme {} copied to clipboard", tooltip_for_log);
                }
            });
            item_box.append(&copy_btn);

            fonts_box.append(&item_box);
        }

        content_box.append(&fonts_box);
        frame.set_child(Some(&content_box));

        // Create refresh callback
        let config_for_refresh = config.clone();
        let gradient_swatch_for_refresh = gradient_swatch.clone();
        let refresh_callback: Rc<dyn Fn()> = Rc::new(move || {
            // Refresh color swatches
            for swatch in color_swatches.borrow().iter() {
                swatch.queue_draw();
            }
            // Refresh gradient swatch
            gradient_swatch_for_refresh.queue_draw();
            // Refresh font labels
            let cfg = config_for_refresh.borrow();
            let labels = font_labels.borrow();
            if labels.len() >= 2 {
                let (family1, size1) = cfg.frame.theme.get_font(1);
                labels[0].set_text(&format!("{} {}pt", family1, size1 as i32));
                let (family2, size2) = cfg.frame.theme.get_font(2);
                labels[1].set_text(&format!("{} {}pt", family2, size2 as i32));
            }
        });

        (frame, refresh_callback)
    }

    fn create_content_item_config(
        config: &Rc<RefCell<SynthwaveDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        slot_name: &str,
        available_fields: Vec<FieldMetadata>,
    ) -> GtkBox {
        // Ensure this slot exists in content_items
        {
            let mut cfg = config.borrow_mut();
            if !cfg.frame.content_items.contains_key(slot_name) {
                cfg.frame.content_items.insert(slot_name.to_string(), ContentItemConfig::default());
            }
        }

        let tab = GtkBox::new(Orientation::Vertical, 8);
        tab.set_margin_start(12);
        tab.set_margin_end(12);
        tab.set_margin_top(12);
        tab.set_margin_bottom(12);

        let scroll = ScrolledWindow::new();
        scroll.set_vexpand(true);
        scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);

        let inner_box = GtkBox::new(Orientation::Vertical, 8);

        // Display type dropdown
        let type_box = GtkBox::new(Orientation::Horizontal, 6);
        type_box.append(&Label::new(Some("Display As:")));
        let type_list = StringList::new(&["Bar", "Text", "Graph", "Core Bars", "Static", "Arc", "Speedometer"]);
        let type_dropdown = DropDown::new(Some(type_list), None::<gtk4::Expression>);
        type_dropdown.set_hexpand(true);

        let current_type = {
            let cfg = config.borrow();
            cfg.frame.content_items
                .get(slot_name)
                .map(|item| item.display_as)
                .unwrap_or_default()
        };
        let type_idx = match current_type {
            ContentDisplayType::Bar | ContentDisplayType::LevelBar => 0,
            ContentDisplayType::Text => 1,
            ContentDisplayType::Graph => 2,
            ContentDisplayType::CoreBars => 3,
            ContentDisplayType::Static => 4,
            ContentDisplayType::Arc => 5,
            ContentDisplayType::Speedometer => 6,
        };
        type_dropdown.set_selected(type_idx);
        type_box.append(&type_dropdown);
        inner_box.append(&type_box);

        // Auto height checkbox
        let auto_height_check = CheckButton::with_label("Auto-adjust height");
        let current_auto_height = {
            let cfg = config.borrow();
            cfg.frame.content_items
                .get(slot_name)
                .map(|item| item.auto_height)
                .unwrap_or(true)
        };
        auto_height_check.set_active(current_auto_height);
        inner_box.append(&auto_height_check);

        // Item height
        let height_box = GtkBox::new(Orientation::Horizontal, 6);
        height_box.append(&Label::new(Some("Item Height:")));
        let height_spin = SpinButton::with_range(20.0, 300.0, 5.0);
        let current_height = {
            let cfg = config.borrow();
            cfg.frame.content_items
                .get(slot_name)
                .map(|item| item.item_height)
                .unwrap_or(60.0)
        };
        height_spin.set_value(current_height);
        height_spin.set_hexpand(true);
        height_spin.set_sensitive(!current_auto_height);
        height_box.append(&height_spin);
        inner_box.append(&height_box);

        // Connect auto-height checkbox
        let height_spin_clone = height_spin.clone();
        let slot_name_clone = slot_name.to_string();
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        auto_height_check.connect_toggled(move |check| {
            let is_auto = check.is_active();
            height_spin_clone.set_sensitive(!is_auto);
            let mut cfg = config_clone.borrow_mut();
            let item = cfg.frame.content_items
                .entry(slot_name_clone.clone())
                .or_default();
            item.auto_height = is_auto;
            drop(cfg);
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });

        // Connect height spinner
        let slot_name_clone = slot_name.to_string();
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        height_spin.connect_value_changed(move |spin| {
            let mut cfg = config_clone.borrow_mut();
            let item = cfg.frame.content_items
                .entry(slot_name_clone.clone())
                .or_default();
            item.item_height = spin.value();
            drop(cfg);
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });

        // Get available fields for this slot
        let slot_prefix = format!("{}_", slot_name);
        let mut slot_fields: Vec<FieldMetadata> = available_fields.iter()
            .filter(|f| f.id.starts_with(&slot_prefix))
            .map(|f| {
                let short_id = f.id.strip_prefix(&slot_prefix).unwrap_or(&f.id);
                FieldMetadata::new(
                    short_id,
                    &f.name,
                    &f.description,
                    f.field_type.clone(),
                    f.purpose.clone(),
                )
            })
            .collect();

        if slot_fields.is_empty() {
            slot_fields = vec![
                FieldMetadata::new("caption", "Caption", "Label text", FieldType::Text, FieldPurpose::Caption),
                FieldMetadata::new("value", "Value", "Current value", FieldType::Text, FieldPurpose::Value),
                FieldMetadata::new("unit", "Unit", "Unit of measurement", FieldType::Text, FieldPurpose::Unit),
                FieldMetadata::new("numerical_value", "Numeric Value", "Raw numeric value", FieldType::Numerical, FieldPurpose::Value),
            ];
        }

        // === Bar Configuration Section ===
        let bar_config_frame = gtk4::Frame::new(Some("Bar Configuration"));
        bar_config_frame.set_margin_top(12);

        let bar_widget = BarConfigWidget::new(slot_fields.clone());
        let current_bar_config = {
            let cfg = config.borrow();
            cfg.frame.content_items
                .get(slot_name)
                .map(|item| item.bar_config.clone())
                .unwrap_or_default()
        };
        bar_widget.set_config(current_bar_config);

        let slot_name_clone = slot_name.to_string();
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let bar_widget_rc = Rc::new(bar_widget);
        let bar_widget_for_callback = bar_widget_rc.clone();
        bar_widget_rc.set_on_change(move || {
            let bar_config = bar_widget_for_callback.get_config();
            let mut cfg = config_clone.borrow_mut();
            let item = cfg.frame.content_items
                .entry(slot_name_clone.clone())
                .or_default();
            item.bar_config = bar_config;
            drop(cfg);
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });

        bar_config_frame.set_child(Some(bar_widget_rc.widget()));
        inner_box.append(&bar_config_frame);

        // === Graph Configuration Section ===
        let graph_config_frame = gtk4::Frame::new(Some("Graph Configuration"));
        graph_config_frame.set_margin_top(12);

        let graph_widget = GraphConfigWidget::new(slot_fields.clone());
        let current_graph_config = {
            let cfg = config.borrow();
            cfg.frame.content_items
                .get(slot_name)
                .map(|item| item.graph_config.clone())
                .unwrap_or_default()
        };
        graph_widget.set_config(current_graph_config);

        let slot_name_clone = slot_name.to_string();
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let graph_widget_rc = Rc::new(graph_widget);
        let graph_widget_for_callback = graph_widget_rc.clone();
        graph_widget_rc.set_on_change(move || {
            let graph_config = graph_widget_for_callback.get_config();
            let mut cfg = config_clone.borrow_mut();
            let item = cfg.frame.content_items
                .entry(slot_name_clone.clone())
                .or_default();
            item.graph_config = graph_config;
            drop(cfg);
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });

        graph_config_frame.set_child(Some(graph_widget_rc.widget()));
        inner_box.append(&graph_config_frame);

        // === Text Configuration Section ===
        let text_config_frame = gtk4::Frame::new(Some("Text Configuration"));
        text_config_frame.set_margin_top(12);

        let text_widget = TextLineConfigWidget::new(slot_fields.clone());
        let current_text_config = {
            let cfg = config.borrow();
            cfg.frame.content_items
                .get(slot_name)
                .map(|item| item.bar_config.text_overlay.text_config.clone())
                .unwrap_or_default()
        };
        text_widget.set_config(current_text_config);

        let slot_name_clone = slot_name.to_string();
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let text_widget_rc = Rc::new(text_widget);
        let text_widget_for_callback = text_widget_rc.clone();
        text_widget_rc.set_on_change(move || {
            let text_config = text_widget_for_callback.get_config();
            let mut cfg = config_clone.borrow_mut();
            let item = cfg.frame.content_items
                .entry(slot_name_clone.clone())
                .or_default();
            item.bar_config.text_overlay.enabled = true;
            item.bar_config.text_overlay.text_config = text_config;
            drop(cfg);
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });

        text_config_frame.set_child(Some(text_widget_rc.widget()));
        inner_box.append(&text_config_frame);

        // === Core Bars Configuration Section ===
        let core_bars_config_frame = gtk4::Frame::new(Some("Core Bars Configuration"));
        core_bars_config_frame.set_margin_top(12);

        let core_bars_widget = CoreBarsConfigWidget::new();
        let current_core_bars_config = {
            let cfg = config.borrow();
            cfg.frame.content_items
                .get(slot_name)
                .map(|item| item.core_bars_config.clone())
                .unwrap_or_default()
        };
        core_bars_widget.set_config(current_core_bars_config);

        let slot_name_clone = slot_name.to_string();
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let core_bars_widget_rc = Rc::new(core_bars_widget);
        let core_bars_widget_for_callback = core_bars_widget_rc.clone();
        core_bars_widget_rc.set_on_change(move || {
            let core_bars_config = core_bars_widget_for_callback.get_config();
            let mut cfg = config_clone.borrow_mut();
            let item = cfg.frame.content_items
                .entry(slot_name_clone.clone())
                .or_default();
            item.core_bars_config = core_bars_config;
            drop(cfg);
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });

        core_bars_config_frame.set_child(Some(core_bars_widget_rc.widget()));
        inner_box.append(&core_bars_config_frame);

        // === Static Configuration Section ===
        let static_config_frame = gtk4::Frame::new(Some("Static Background Configuration"));
        static_config_frame.set_margin_top(12);

        let static_bg_widget = BackgroundConfigWidget::new();
        let current_static_config = {
            let cfg = config.borrow();
            cfg.frame.content_items
                .get(slot_name)
                .map(|item| item.static_config.background.clone())
                .unwrap_or_default()
        };
        static_bg_widget.set_config(current_static_config);

        let slot_name_clone = slot_name.to_string();
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let static_bg_widget_rc = Rc::new(static_bg_widget);
        let static_bg_widget_for_callback = static_bg_widget_rc.clone();
        static_bg_widget_rc.set_on_change(move || {
            let bg_config = static_bg_widget_for_callback.get_config();
            let mut cfg = config_clone.borrow_mut();
            let item = cfg.frame.content_items
                .entry(slot_name_clone.clone())
                .or_default();
            item.static_config = StaticDisplayConfig { background: bg_config };
            drop(cfg);
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });

        static_config_frame.set_child(Some(static_bg_widget_rc.widget()));
        inner_box.append(&static_config_frame);

        // === Arc Configuration Section ===
        let arc_config_frame = gtk4::Frame::new(Some("Arc Gauge Configuration"));
        arc_config_frame.set_margin_top(12);

        let arc_widget = ArcConfigWidget::new(slot_fields.clone());
        let current_arc_config = {
            let cfg = config.borrow();
            cfg.frame.content_items
                .get(slot_name)
                .map(|item| item.arc_config.clone())
                .unwrap_or_default()
        };
        arc_widget.set_config(current_arc_config);

        let slot_name_clone = slot_name.to_string();
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let arc_widget_rc = Rc::new(arc_widget);
        let arc_widget_for_callback = arc_widget_rc.clone();
        arc_widget_rc.set_on_change(move || {
            let arc_config = arc_widget_for_callback.get_config();
            let mut cfg = config_clone.borrow_mut();
            let item = cfg.frame.content_items
                .entry(slot_name_clone.clone())
                .or_default();
            item.arc_config = arc_config;
            drop(cfg);
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });

        arc_config_frame.set_child(Some(arc_widget_rc.widget()));
        inner_box.append(&arc_config_frame);

        // === Speedometer Configuration Section ===
        let speedometer_config_frame = gtk4::Frame::new(Some("Speedometer Configuration"));
        speedometer_config_frame.set_margin_top(12);

        let speedometer_widget = SpeedometerConfigWidget::new(slot_fields.clone());
        let current_speedometer_config = {
            let cfg = config.borrow();
            cfg.frame.content_items
                .get(slot_name)
                .map(|item| item.speedometer_config.clone())
                .unwrap_or_default()
        };
        speedometer_widget.set_config(&current_speedometer_config);

        let slot_name_clone = slot_name.to_string();
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let speedometer_widget_rc = Rc::new(speedometer_widget);
        let speedometer_widget_for_callback = speedometer_widget_rc.clone();
        speedometer_widget_rc.set_on_change(Box::new(move || {
            let speedometer_config = speedometer_widget_for_callback.get_config();
            let mut cfg = config_clone.borrow_mut();
            let item = cfg.frame.content_items
                .entry(slot_name_clone.clone())
                .or_default();
            item.speedometer_config = speedometer_config;
            drop(cfg);
            Self::queue_redraw(&preview_clone, &on_change_clone);
        }));

        speedometer_config_frame.set_child(Some(speedometer_widget_rc.widget()));
        inner_box.append(&speedometer_config_frame);

        // Show/hide sections based on display type
        let show_bar = matches!(current_type, ContentDisplayType::Bar | ContentDisplayType::LevelBar);
        let show_text = matches!(current_type, ContentDisplayType::Text | ContentDisplayType::Static);
        bar_config_frame.set_visible(show_bar);
        text_config_frame.set_visible(show_text);
        graph_config_frame.set_visible(current_type == ContentDisplayType::Graph);
        core_bars_config_frame.set_visible(current_type == ContentDisplayType::CoreBars);
        static_config_frame.set_visible(current_type == ContentDisplayType::Static);
        arc_config_frame.set_visible(current_type == ContentDisplayType::Arc);
        speedometer_config_frame.set_visible(current_type == ContentDisplayType::Speedometer);

        scroll.set_child(Some(&inner_box));
        tab.append(&scroll);

        // Display type change handler - show/hide relevant config sections
        let slot_name_clone = slot_name.to_string();
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let bar_config_frame_clone = bar_config_frame.clone();
        let text_config_frame_clone = text_config_frame.clone();
        let graph_config_frame_clone = graph_config_frame.clone();
        let core_bars_config_frame_clone = core_bars_config_frame.clone();
        let static_config_frame_clone = static_config_frame.clone();
        let arc_config_frame_clone = arc_config_frame.clone();
        let speedometer_config_frame_clone = speedometer_config_frame.clone();
        type_dropdown.connect_selected_notify(move |dropdown| {
            let selected = dropdown.selected();
            if selected == gtk4::INVALID_LIST_POSITION {
                return;
            }
            let display_type = match selected {
                0 => ContentDisplayType::Bar,
                1 => ContentDisplayType::Text,
                2 => ContentDisplayType::Graph,
                3 => ContentDisplayType::CoreBars,
                4 => ContentDisplayType::Static,
                5 => ContentDisplayType::Arc,
                _ => ContentDisplayType::Speedometer,
            };

            // Update config
            {
                let mut cfg = config_clone.borrow_mut();
                let item = cfg.frame.content_items
                    .entry(slot_name_clone.clone())
                    .or_default();
                item.display_as = display_type;
            }

            // Show/hide relevant frames
            let show_bar = matches!(display_type, ContentDisplayType::Bar | ContentDisplayType::LevelBar);
            let show_text = matches!(display_type, ContentDisplayType::Text | ContentDisplayType::Static);
            bar_config_frame_clone.set_visible(show_bar);
            text_config_frame_clone.set_visible(show_text);
            graph_config_frame_clone.set_visible(display_type == ContentDisplayType::Graph);
            core_bars_config_frame_clone.set_visible(display_type == ContentDisplayType::CoreBars);
            static_config_frame_clone.set_visible(display_type == ContentDisplayType::Static);
            arc_config_frame_clone.set_visible(display_type == ContentDisplayType::Arc);
            speedometer_config_frame_clone.set_visible(display_type == ContentDisplayType::Speedometer);

            Self::queue_redraw(&preview_clone, &on_change_clone);
        });

        tab
    }

    // Public API
    pub fn widget(&self) -> &GtkBox {
        &self.container
    }

    pub fn get_config(&self) -> SynthwaveDisplayConfig {
        self.config.borrow().clone()
    }

    pub fn set_config(&self, config: SynthwaveDisplayConfig) {
        // Extract all values we need BEFORE updating the config
        // Color scheme
        let color_scheme_idx = match &config.frame.color_scheme {
            SynthwaveColorScheme::Classic => 0,
            SynthwaveColorScheme::Sunset => 1,
            SynthwaveColorScheme::NightDrive => 2,
            SynthwaveColorScheme::Miami => 3,
            SynthwaveColorScheme::Custom { .. } => 4,
        };
        // Theme values
        let theme_color1 = config.frame.theme.color1;
        let theme_color2 = config.frame.theme.color2;
        let theme_color3 = config.frame.theme.color3;
        let theme_color4 = config.frame.theme.color4;
        let theme_gradient = config.frame.theme.gradient.clone();
        let theme_font1_family = config.frame.theme.font1_family.clone();
        let theme_font1_size = config.frame.theme.font1_size;
        let theme_font2_family = config.frame.theme.font2_family.clone();
        let theme_font2_size = config.frame.theme.font2_size;
        let neon_glow = config.frame.neon_glow_intensity;

        let frame_style_idx = match config.frame.frame_style {
            SynthwaveFrameStyle::NeonBorder => 0,
            SynthwaveFrameStyle::Chrome => 1,
            SynthwaveFrameStyle::Minimal => 2,
            SynthwaveFrameStyle::RetroDouble => 3,
            SynthwaveFrameStyle::None => 4,
        };
        let frame_width = config.frame.frame_width;
        let corner_radius = config.frame.corner_radius;

        let show_grid = config.frame.show_grid;
        let grid_style_idx = match config.frame.grid_style {
            GridStyle::Perspective => 0,
            GridStyle::Flat => 1,
            GridStyle::Hexagon => 2,
            GridStyle::Scanlines => 3,
            GridStyle::None => 4,
        };
        let grid_spacing = config.frame.grid_spacing;
        let grid_line_width = config.frame.grid_line_width;
        let horizon = config.frame.grid_horizon;
        let perspective = config.frame.grid_perspective;
        let show_sun = config.frame.show_sun;
        let sun_position = config.frame.sun_position;

        let show_header = config.frame.show_header;
        let header_text = config.frame.header_text.clone();
        let header_style_idx = match config.frame.header_style {
            SynthwaveHeaderStyle::Chrome => 0,
            SynthwaveHeaderStyle::Neon => 1,
            SynthwaveHeaderStyle::Outline => 2,
            SynthwaveHeaderStyle::Simple => 3,
            SynthwaveHeaderStyle::None => 4,
        };
        let header_height = config.frame.header_height;
        let header_font = config.frame.header_font.clone();
        let header_font_size = config.frame.header_font_size;

        let orient_idx = match config.frame.split_orientation {
            SplitOrientation::Vertical => 0,
            SplitOrientation::Horizontal => 1,
        };
        let content_padding = config.frame.content_padding;
        let item_spacing = config.frame.item_spacing;
        let div_style_idx = match config.frame.divider_style {
            SynthwaveDividerStyle::NeonLine => 0,
            SynthwaveDividerStyle::Gradient => 1,
            SynthwaveDividerStyle::NeonDots => 2,
            SynthwaveDividerStyle::Line => 3,
            SynthwaveDividerStyle::None => 4,
        };
        let divider_padding = config.frame.divider_padding;

        let animation_enabled = config.animation_enabled;
        let animation_speed = config.animation_speed;
        let scanline_effect = config.frame.scanline_effect;

        // Now update the config
        *self.config.borrow_mut() = config;

        // Update UI widgets
        if let Some(ref widgets) = *self.theme_widgets.borrow() {
            widgets.color_scheme_dropdown.set_selected(color_scheme_idx);
            widgets.theme_color1_widget.set_color(theme_color1);
            widgets.theme_color2_widget.set_color(theme_color2);
            widgets.theme_color3_widget.set_color(theme_color3);
            widgets.theme_color4_widget.set_color(theme_color4);
            widgets.theme_gradient_editor.set_gradient(&theme_gradient);
            widgets.font1_btn.set_label(&theme_font1_family);
            widgets.font1_size_spin.set_value(theme_font1_size);
            widgets.font2_btn.set_label(&theme_font2_family);
            widgets.font2_size_spin.set_value(theme_font2_size);
            widgets.glow_scale.set_value(neon_glow);
        }

        if let Some(ref widgets) = *self.frame_widgets.borrow() {
            widgets.style_dropdown.set_selected(frame_style_idx);
            widgets.frame_width_spin.set_value(frame_width);
            widgets.corner_radius_spin.set_value(corner_radius);
        }

        if let Some(ref widgets) = *self.grid_widgets.borrow() {
            widgets.show_grid_check.set_active(show_grid);
            widgets.grid_style_dropdown.set_selected(grid_style_idx);
            widgets.grid_spacing_spin.set_value(grid_spacing);
            widgets.grid_line_width_spin.set_value(grid_line_width);
            widgets.horizon_scale.set_value(horizon);
            widgets.perspective_scale.set_value(perspective);
            widgets.show_sun_check.set_active(show_sun);
            widgets.sun_position_scale.set_value(sun_position);
        }

        if let Some(ref widgets) = *self.header_widgets.borrow() {
            widgets.show_header_check.set_active(show_header);
            widgets.header_text_entry.set_text(&header_text);
            widgets.header_style_dropdown.set_selected(header_style_idx);
            widgets.header_height_spin.set_value(header_height);
            // Update font selector with current font as custom
            widgets.header_font_selector.set_source(FontSource::Custom {
                family: header_font,
                size: header_font_size,
            });
        }

        if let Some(ref widgets) = *self.layout_widgets.borrow() {
            widgets.split_orientation_dropdown.set_selected(orient_idx);
            widgets.content_padding_spin.set_value(content_padding);
            widgets.item_spacing_spin.set_value(item_spacing);
            widgets.divider_style_dropdown.set_selected(div_style_idx);
            widgets.divider_padding_spin.set_value(divider_padding);
        }

        if let Some(ref widgets) = *self.animation_widgets.borrow() {
            widgets.enable_check.set_active(animation_enabled);
            widgets.speed_spin.set_value(animation_speed);
            widgets.scanline_check.set_active(scanline_effect);
        }

        self.preview.queue_draw();
    }

    pub fn set_on_change(&self, callback: impl Fn() + 'static) {
        *self.on_change.borrow_mut() = Some(Box::new(callback));
    }

    pub fn set_source_summaries(&self, summaries: Vec<(String, String, usize, u32)>) {
        // Extract group configuration from summaries
        let mut group_item_counts: std::collections::HashMap<usize, u32> = std::collections::HashMap::new();
        for (_, _, group_num, item_idx) in &summaries {
            let current_max = group_item_counts.entry(*group_num).or_insert(0);
            if *item_idx > *current_max {
                *current_max = *item_idx;
            }
        }

        // Convert to sorted vec (same approach as LCARS)
        let mut group_nums: Vec<usize> = group_item_counts.keys().cloned().collect();
        group_nums.sort();
        let group_counts: Vec<usize> = group_nums.iter()
            .map(|n| *group_item_counts.get(n).unwrap_or(&0) as usize)
            .collect();

        // Update config with group info
        {
            let mut cfg = self.config.borrow_mut();
            let new_group_count = group_nums.len().max(1);
            cfg.frame.group_count = new_group_count;
            cfg.frame.group_item_counts = group_counts;

            // Ensure weights are set
            while cfg.frame.group_size_weights.len() < new_group_count {
                cfg.frame.group_size_weights.push(1.0);
            }
            // Trim if we have fewer groups now
            cfg.frame.group_size_weights.truncate(new_group_count);
        }

        *self.source_summaries.borrow_mut() = summaries;

        // Rebuild group spinners in Layout tab
        if let Some(ref widgets) = *self.layout_widgets.borrow() {
            Self::rebuild_group_spinners(
                &self.config,
                &self.on_change,
                &self.preview,
                &widgets.group_weights_box,
            );
        }

        // Rebuild content tabs
        combo_config_base::rebuild_content_tabs(
            &self.config,
            &self.on_change,
            &self.preview,
            &self.content_notebook,
            &self.source_summaries,
            &self.available_fields,
            |cfg| &cfg.frame.content_items,
        );

        // Notify that config changed
        if let Some(cb) = self.on_change.borrow().as_ref() {
            cb();
        }
    }

    pub fn set_available_fields(&self, fields: Vec<FieldMetadata>) {
        *self.available_fields.borrow_mut() = fields;
    }
}

impl Default for SynthwaveConfigWidget {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}
