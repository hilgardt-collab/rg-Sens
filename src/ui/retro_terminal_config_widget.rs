//! Retro Terminal (CRT) configuration widget
//!
//! Provides a tabbed interface for configuring all aspects of the Retro Terminal display.

use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, CheckButton, DrawingArea, DropDown, Entry, Label, Notebook, Orientation,
    Scale, SpinButton, StringList, ScrolledWindow,
};
use std::cell::RefCell;
use std::rc::Rc;

use crate::ui::shared_font_dialog::shared_font_dialog;
use crate::ui::gradient_editor::GradientEditor;
use crate::ui::color_button_widget::ColorButtonWidget;
use crate::ui::retro_terminal_display::{
    render_retro_terminal_frame, PhosphorColor, BezelStyle,
    TerminalHeaderStyle, TerminalDividerStyle,
};
use crate::ui::lcars_display::SplitOrientation;
use crate::ui::background::Color;
use crate::displayers::RetroTerminalDisplayConfig;
use crate::core::FieldMetadata;
use crate::ui::combo_config_base;
use crate::ui::theme::{ColorSource, FontSource};
use crate::ui::theme_font_selector::ThemeFontSelector;

/// Holds references to Colors tab widgets
struct ColorsWidgets {
    phosphor_dropdown: DropDown,
    custom_phosphor_widget: Rc<ColorButtonWidget>,
    custom_phosphor_box: GtkBox,
    background_widget: Rc<ColorButtonWidget>,
    brightness_scale: Scale,
}

/// Holds references to CRT Effects tab widgets
struct EffectsWidgets {
    scanline_intensity_scale: Scale,
    scanline_spacing_spin: SpinButton,
    curvature_scale: Scale,
    vignette_scale: Scale,
    glow_scale: Scale,
    flicker_check: CheckButton,
}

/// Holds references to Bezel tab widgets
struct BezelWidgets {
    style_dropdown: DropDown,
    color_widget: Rc<ColorButtonWidget>,
    width_spin: SpinButton,
    show_led_check: CheckButton,
    led_color_widget: Rc<ColorButtonWidget>,
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
    divider_style_dropdown: DropDown,
    divider_padding_spin: SpinButton,
    group_weights_box: GtkBox,
    item_orientations_box: GtkBox,
}

/// Holds references to Animation tab widgets
struct AnimationWidgets {
    enable_check: CheckButton,
    cursor_blink_check: CheckButton,
    typewriter_check: CheckButton,
}

/// Holds references to Theme tab widgets
#[allow(dead_code)]
struct ThemeWidgets {
    theme_color1_widget: Rc<ColorButtonWidget>,
    theme_color2_widget: Rc<ColorButtonWidget>,
    theme_color3_widget: Rc<ColorButtonWidget>,
    theme_color4_widget: Rc<ColorButtonWidget>,
    theme_gradient_editor: Rc<GradientEditor>,
    font1_btn: Button,
    font1_size_spin: SpinButton,
    font2_btn: Button,
    font2_size_spin: SpinButton,
}

/// Retro Terminal configuration widget
pub struct RetroTerminalConfigWidget {
    container: GtkBox,
    config: Rc<RefCell<RetroTerminalDisplayConfig>>,
    on_change: Rc<RefCell<Option<Box<dyn Fn()>>>>,
    preview: DrawingArea,
    content_notebook: Rc<RefCell<Notebook>>,
    source_summaries: Rc<RefCell<Vec<(String, String, usize, u32)>>>,
    available_fields: Rc<RefCell<Vec<FieldMetadata>>>,
    colors_widgets: Rc<RefCell<Option<ColorsWidgets>>>,
    effects_widgets: Rc<RefCell<Option<EffectsWidgets>>>,
    bezel_widgets: Rc<RefCell<Option<BezelWidgets>>>,
    header_widgets: Rc<RefCell<Option<HeaderWidgets>>>,
    layout_widgets: Rc<RefCell<Option<LayoutWidgets>>>,
    animation_widgets: Rc<RefCell<Option<AnimationWidgets>>>,
    /// Theme tab widgets
    #[allow(dead_code)]
    theme_widgets: Rc<RefCell<Option<ThemeWidgets>>>,
    /// Callbacks to refresh theme reference sections
    #[allow(dead_code)] // Kept for Rc ownership; callbacks are invoked via clones
    theme_ref_refreshers: Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
}

impl RetroTerminalConfigWidget {
    pub fn new(available_fields: Vec<FieldMetadata>) -> Self {
        let container = GtkBox::new(Orientation::Vertical, 12);
        let config = Rc::new(RefCell::new(RetroTerminalDisplayConfig::default()));
        let on_change: Rc<RefCell<Option<Box<dyn Fn()>>>> = Rc::new(RefCell::new(None));
        let source_summaries: Rc<RefCell<Vec<(String, String, usize, u32)>>> = Rc::new(RefCell::new(Vec::new()));
        let available_fields: Rc<RefCell<Vec<FieldMetadata>>> = Rc::new(RefCell::new(available_fields));
        let colors_widgets: Rc<RefCell<Option<ColorsWidgets>>> = Rc::new(RefCell::new(None));
        let effects_widgets: Rc<RefCell<Option<EffectsWidgets>>> = Rc::new(RefCell::new(None));
        let bezel_widgets: Rc<RefCell<Option<BezelWidgets>>> = Rc::new(RefCell::new(None));
        let header_widgets: Rc<RefCell<Option<HeaderWidgets>>> = Rc::new(RefCell::new(None));
        let layout_widgets: Rc<RefCell<Option<LayoutWidgets>>> = Rc::new(RefCell::new(None));
        let animation_widgets: Rc<RefCell<Option<AnimationWidgets>>> = Rc::new(RefCell::new(None));
        let theme_widgets: Rc<RefCell<Option<ThemeWidgets>>> = Rc::new(RefCell::new(None));
        let theme_ref_refreshers: Rc<RefCell<Vec<Rc<dyn Fn()>>>> = Rc::new(RefCell::new(Vec::new()));

        // Preview at the top
        let preview = DrawingArea::new();
        preview.set_content_height(200);
        preview.set_hexpand(true);
        preview.set_vexpand(false);

        let config_clone = config.clone();
        preview.set_draw_func(move |_, cr, width, height| {
            if width < 10 || height < 10 {
                return;
            }

            let cfg = config_clone.borrow();
            let _ = render_retro_terminal_frame(cr, &cfg.frame, width as f64, height as f64);
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

        // Tab 1: Theme (consolidated with Colors)
        let theme_page = Self::create_theme_page(&config, &on_change, &preview, &theme_widgets, &theme_ref_refreshers, &colors_widgets);
        notebook.append_page(&theme_page, Some(&Label::new(Some("Theme"))));

        // Tab 2: CRT Effects
        let effects_page = Self::create_effects_page(&config, &on_change, &preview, &effects_widgets);
        notebook.append_page(&effects_page, Some(&Label::new(Some("CRT Effects"))));

        // Tab 3: Bezel
        let bezel_page = Self::create_bezel_page(&config, &on_change, &preview, &bezel_widgets, &theme_ref_refreshers);
        notebook.append_page(&bezel_page, Some(&Label::new(Some("Bezel"))));

        // Tab 4: Header
        let header_page = Self::create_header_page(&config, &on_change, &preview, &header_widgets, &theme_ref_refreshers);
        notebook.append_page(&header_page, Some(&Label::new(Some("Header"))));

        // Tab 5: Layout
        let layout_page = Self::create_layout_page(&config, &on_change, &preview, &layout_widgets);
        notebook.append_page(&layout_page, Some(&Label::new(Some("Layout"))));

        // Tab 6: Content - with dynamic per-slot notebook
        let content_notebook = Rc::new(RefCell::new(Notebook::new()));
        let content_page = Self::create_content_page(&config, &on_change, &preview, &content_notebook, &source_summaries, &available_fields, &theme_ref_refreshers);
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
            colors_widgets,
            effects_widgets,
            bezel_widgets,
            header_widgets,
            layout_widgets,
            animation_widgets,
            theme_widgets,
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

    fn create_effects_page(
        config: &Rc<RefCell<RetroTerminalDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        effects_widgets_out: &Rc<RefCell<Option<EffectsWidgets>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        Self::set_page_margins(&page);

        // Scanlines section
        let scanline_label = Label::new(Some("Scanlines"));
        scanline_label.set_halign(gtk4::Align::Start);
        scanline_label.add_css_class("heading");
        page.append(&scanline_label);

        // Scanline intensity
        let intensity_box = GtkBox::new(Orientation::Horizontal, 6);
        intensity_box.append(&Label::new(Some("Intensity:")));
        let scanline_intensity_scale = Scale::with_range(Orientation::Horizontal, 0.0, 1.0, 0.05);
        scanline_intensity_scale.set_value(config.borrow().frame.scanline_intensity);
        scanline_intensity_scale.set_hexpand(true);
        scanline_intensity_scale.set_draw_value(true);
        intensity_box.append(&scanline_intensity_scale);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        scanline_intensity_scale.connect_value_changed(move |scale| {
            config_clone.borrow_mut().frame.scanline_intensity = scale.value();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&intensity_box);

        // Scanline spacing
        let spacing_box = GtkBox::new(Orientation::Horizontal, 6);
        spacing_box.append(&Label::new(Some("Spacing:")));
        let scanline_spacing_spin = SpinButton::with_range(1.0, 8.0, 0.5);
        scanline_spacing_spin.set_value(config.borrow().frame.scanline_spacing);
        scanline_spacing_spin.set_hexpand(true);
        spacing_box.append(&scanline_spacing_spin);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        scanline_spacing_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().frame.scanline_spacing = spin.value();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&spacing_box);

        // CRT Effects section
        let crt_label = Label::new(Some("CRT Effects"));
        crt_label.set_halign(gtk4::Align::Start);
        crt_label.add_css_class("heading");
        crt_label.set_margin_top(12);
        page.append(&crt_label);

        // Curvature
        let curvature_box = GtkBox::new(Orientation::Horizontal, 6);
        curvature_box.append(&Label::new(Some("Curvature:")));
        let curvature_scale = Scale::with_range(Orientation::Horizontal, 0.0, 0.1, 0.01);
        curvature_scale.set_value(config.borrow().frame.curvature_amount);
        curvature_scale.set_hexpand(true);
        curvature_scale.set_draw_value(true);
        curvature_box.append(&curvature_scale);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        curvature_scale.connect_value_changed(move |scale| {
            config_clone.borrow_mut().frame.curvature_amount = scale.value();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&curvature_box);

        // Vignette
        let vignette_box = GtkBox::new(Orientation::Horizontal, 6);
        vignette_box.append(&Label::new(Some("Vignette:")));
        let vignette_scale = Scale::with_range(Orientation::Horizontal, 0.0, 1.0, 0.05);
        vignette_scale.set_value(config.borrow().frame.vignette_intensity);
        vignette_scale.set_hexpand(true);
        vignette_scale.set_draw_value(true);
        vignette_box.append(&vignette_scale);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        vignette_scale.connect_value_changed(move |scale| {
            config_clone.borrow_mut().frame.vignette_intensity = scale.value();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&vignette_box);

        // Screen glow
        let glow_box = GtkBox::new(Orientation::Horizontal, 6);
        glow_box.append(&Label::new(Some("Screen Glow:")));
        let glow_scale = Scale::with_range(Orientation::Horizontal, 0.0, 1.0, 0.05);
        glow_scale.set_value(config.borrow().frame.screen_glow);
        glow_scale.set_hexpand(true);
        glow_scale.set_draw_value(true);
        glow_box.append(&glow_scale);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        glow_scale.connect_value_changed(move |scale| {
            config_clone.borrow_mut().frame.screen_glow = scale.value();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&glow_box);

        // Flicker
        let flicker_check = CheckButton::with_label("Enable Screen Flicker");
        flicker_check.set_active(config.borrow().frame.flicker_enabled);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        flicker_check.connect_toggled(move |check| {
            config_clone.borrow_mut().frame.flicker_enabled = check.is_active();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&flicker_check);

        // Store widget refs
        *effects_widgets_out.borrow_mut() = Some(EffectsWidgets {
            scanline_intensity_scale,
            scanline_spacing_spin,
            curvature_scale,
            vignette_scale,
            glow_scale,
            flicker_check,
        });

        page
    }

    fn create_bezel_page(
        config: &Rc<RefCell<RetroTerminalDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        bezel_widgets_out: &Rc<RefCell<Option<BezelWidgets>>>,
        _theme_ref_refreshers: &Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        Self::set_page_margins(&page);

        // Bezel style
        let style_box = GtkBox::new(Orientation::Horizontal, 6);
        style_box.append(&Label::new(Some("Bezel Style:")));
        let style_list = StringList::new(&["Classic CRT", "Slim", "Industrial", "None"]);
        let style_dropdown = DropDown::new(Some(style_list), None::<gtk4::Expression>);
        let style_idx = match config.borrow().frame.bezel_style {
            BezelStyle::Classic => 0,
            BezelStyle::Slim => 1,
            BezelStyle::Industrial => 2,
            BezelStyle::None => 3,
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
            config_clone.borrow_mut().frame.bezel_style = match selected {
                0 => BezelStyle::Classic,
                1 => BezelStyle::Slim,
                2 => BezelStyle::Industrial,
                _ => BezelStyle::None,
            };
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&style_box);

        // Bezel color
        let color_box = GtkBox::new(Orientation::Horizontal, 6);
        color_box.append(&Label::new(Some("Bezel Color:")));
        let color_widget = Rc::new(ColorButtonWidget::new(config.borrow().frame.bezel_color));
        color_box.append(color_widget.widget());

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        color_widget.set_on_change(move |color| {
            config_clone.borrow_mut().frame.bezel_color = color;
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&color_box);

        // Bezel width
        let width_box = GtkBox::new(Orientation::Horizontal, 6);
        width_box.append(&Label::new(Some("Bezel Width:")));
        let width_spin = SpinButton::with_range(4.0, 48.0, 2.0);
        width_spin.set_value(config.borrow().frame.bezel_width);
        width_spin.set_hexpand(true);
        width_box.append(&width_spin);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        width_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().frame.bezel_width = spin.value();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&width_box);

        // Power LED section
        let led_label = Label::new(Some("Power LED"));
        led_label.set_halign(gtk4::Align::Start);
        led_label.add_css_class("heading");
        led_label.set_margin_top(12);
        page.append(&led_label);

        // Show LED
        let show_led_check = CheckButton::with_label("Show Power LED");
        show_led_check.set_active(config.borrow().frame.show_power_led);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        show_led_check.connect_toggled(move |check| {
            config_clone.borrow_mut().frame.show_power_led = check.is_active();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&show_led_check);

        // LED color
        let led_color_box = GtkBox::new(Orientation::Horizontal, 6);
        led_color_box.append(&Label::new(Some("LED Color:")));
        let led_color_widget = Rc::new(ColorButtonWidget::new(config.borrow().frame.power_led_color));
        led_color_box.append(led_color_widget.widget());

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        led_color_widget.set_on_change(move |color| {
            config_clone.borrow_mut().frame.power_led_color = color;
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&led_color_box);

        // Store widget refs
        *bezel_widgets_out.borrow_mut() = Some(BezelWidgets {
            style_dropdown,
            color_widget,
            width_spin,
            show_led_check,
            led_color_widget,
        });

        page
    }

    fn create_header_page(
        config: &Rc<RefCell<RetroTerminalDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        header_widgets_out: &Rc<RefCell<Option<HeaderWidgets>>>,
        theme_ref_refreshers: &Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
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
        style_box.append(&Label::new(Some("Style:")));
        let style_list = StringList::new(&["Title Bar", "Status Line", "Prompt", "None"]);
        let header_style_dropdown = DropDown::new(Some(style_list), None::<gtk4::Expression>);
        let style_idx = match config.borrow().frame.header_style {
            TerminalHeaderStyle::TitleBar => 0,
            TerminalHeaderStyle::StatusLine => 1,
            TerminalHeaderStyle::Prompt => 2,
            TerminalHeaderStyle::None => 3,
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
                0 => TerminalHeaderStyle::TitleBar,
                1 => TerminalHeaderStyle::StatusLine,
                2 => TerminalHeaderStyle::Prompt,
                _ => TerminalHeaderStyle::None,
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

        // Typography section
        let font_label = Label::new(Some("Typography"));
        font_label.set_halign(gtk4::Align::Start);
        font_label.add_css_class("heading");
        font_label.set_margin_top(12);
        page.append(&font_label);

        // Header font selector (theme-aware)
        let header_font_selector = Rc::new(ThemeFontSelector::new(config.borrow().frame.header_font.clone()));
        header_font_selector.set_theme_config(config.borrow().frame.theme.clone());
        page.append(header_font_selector.widget());

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        header_font_selector.set_on_change(move |font_source| {
            config_clone.borrow_mut().frame.header_font = font_source;
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });

        // Register theme refresh callback for the font selector
        let selector_for_refresh = header_font_selector.clone();
        let config_for_refresh = config.clone();
        let font_refresh: Rc<dyn Fn()> = Rc::new(move || {
            let theme = config_for_refresh.borrow().frame.theme.clone();
            selector_for_refresh.set_theme_config(theme);
        });
        theme_ref_refreshers.borrow_mut().push(font_refresh);

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
        config: &Rc<RefCell<RetroTerminalDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        layout_widgets_out: &Rc<RefCell<Option<LayoutWidgets>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        Self::set_page_margins(&page);

        // Split orientation
        let orientation_box = GtkBox::new(Orientation::Horizontal, 6);
        orientation_box.append(&Label::new(Some("Group Direction:")));
        let orientation_list = StringList::new(&["Horizontal (side by side)", "Vertical (stacked)"]);
        let split_orientation_dropdown = DropDown::new(Some(orientation_list), None::<gtk4::Expression>);
        let orient_idx = match config.borrow().frame.split_orientation {
            SplitOrientation::Horizontal => 0,
            SplitOrientation::Vertical => 1,
        };
        split_orientation_dropdown.set_selected(orient_idx);
        split_orientation_dropdown.set_hexpand(true);
        orientation_box.append(&split_orientation_dropdown);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        split_orientation_dropdown.connect_selected_notify(move |dropdown| {
            let selected = dropdown.selected();
            if selected == gtk4::INVALID_LIST_POSITION {
                return;
            }
            config_clone.borrow_mut().frame.split_orientation = match selected {
                0 => SplitOrientation::Horizontal,
                _ => SplitOrientation::Vertical,
            };
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&orientation_box);

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

        // Dividers section
        let dividers_label = Label::new(Some("Dividers"));
        dividers_label.set_halign(gtk4::Align::Start);
        dividers_label.add_css_class("heading");
        dividers_label.set_margin_top(12);
        page.append(&dividers_label);

        // Divider style
        let div_style_box = GtkBox::new(Orientation::Horizontal, 6);
        div_style_box.append(&Label::new(Some("Style:")));
        let div_style_list = StringList::new(&["Dashed ------", "Solid ======", "Box Drawing", "Pipe |||", "ASCII ====", "None"]);
        let divider_style_dropdown = DropDown::new(Some(div_style_list), None::<gtk4::Expression>);
        let div_style_idx = match config.borrow().frame.divider_style {
            TerminalDividerStyle::Dashed => 0,
            TerminalDividerStyle::Solid => 1,
            TerminalDividerStyle::BoxDrawing => 2,
            TerminalDividerStyle::Pipe => 3,
            TerminalDividerStyle::Ascii => 4,
            TerminalDividerStyle::None => 5,
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
                0 => TerminalDividerStyle::Dashed,
                1 => TerminalDividerStyle::Solid,
                2 => TerminalDividerStyle::BoxDrawing,
                3 => TerminalDividerStyle::Pipe,
                4 => TerminalDividerStyle::Ascii,
                _ => TerminalDividerStyle::None,
            };
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&div_style_box);

        // Divider padding
        let div_padding_box = GtkBox::new(Orientation::Horizontal, 6);
        div_padding_box.append(&Label::new(Some("Divider Padding:")));
        let divider_padding_spin = SpinButton::with_range(0.0, 16.0, 1.0);
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

        Self::rebuild_group_spinners(config, on_change, preview, &group_weights_box);

        // Item Orientations section
        let item_orient_label = Label::new(Some("Item Orientation per Group"));
        item_orient_label.set_halign(gtk4::Align::Start);
        item_orient_label.add_css_class("heading");
        item_orient_label.set_margin_top(12);
        page.append(&item_orient_label);

        let item_orient_info = Label::new(Some(
            "Choose how items within each group are arranged",
        ));
        item_orient_info.set_halign(gtk4::Align::Start);
        item_orient_info.add_css_class("dim-label");
        page.append(&item_orient_info);

        let item_orientations_box = GtkBox::new(Orientation::Vertical, 4);
        item_orientations_box.set_margin_top(4);
        combo_config_base::rebuild_item_orientation_dropdowns(
            &item_orientations_box,
            config,
            |c: &mut RetroTerminalDisplayConfig| &mut c.frame,
            on_change,
            preview,
        );
        page.append(&item_orientations_box);

        // Store widget refs
        *layout_widgets_out.borrow_mut() = Some(LayoutWidgets {
            split_orientation_dropdown,
            content_padding_spin,
            divider_style_dropdown,
            divider_padding_spin,
            group_weights_box,
            item_orientations_box,
        });

        page
    }

    fn rebuild_group_spinners(
        config: &Rc<RefCell<RetroTerminalDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        weights_box: &GtkBox,
    ) {
        while let Some(child) = weights_box.first_child() {
            weights_box.remove(&child);
        }

        let cfg = config.borrow();
        let count = cfg.frame.group_count;

        if count == 0 {
            let placeholder = Label::new(Some("No groups configured."));
            placeholder.set_halign(gtk4::Align::Start);
            placeholder.add_css_class("dim-label");
            weights_box.append(&placeholder);
            return;
        }

        for i in 0..count {
            let row = GtkBox::new(Orientation::Horizontal, 6);
            row.append(&Label::new(Some(&format!("Group {}:", i + 1))));

            let weight_spin = SpinButton::with_range(0.1, 10.0, 0.1);
            weight_spin.set_digits(1);
            weight_spin.set_value(cfg.frame.group_size_weights.get(i).copied().unwrap_or(1.0));
            weight_spin.set_hexpand(true);
            row.append(&weight_spin);

            let config_clone = config.clone();
            let on_change_clone = on_change.clone();
            let preview_clone = preview.clone();
            weight_spin.connect_value_changed(move |spin| {
                let mut cfg = config_clone.borrow_mut();
                if i < cfg.frame.group_size_weights.len() {
                    cfg.frame.group_size_weights[i] = spin.value();
                }
                drop(cfg);
                Self::queue_redraw(&preview_clone, &on_change_clone);
            });

            weights_box.append(&row);
        }
    }

    /// Helper function to refresh all theme reference sections
    fn refresh_theme_refs(refreshers: &Rc<RefCell<Vec<Rc<dyn Fn()>>>>) {
        combo_config_base::refresh_theme_refs(refreshers);
    }

    /// Get theme colors for a given phosphor color preset
    fn theme_colors_for_phosphor(phosphor: &PhosphorColor) -> [Color; 4] {
        match phosphor {
            PhosphorColor::Green => [
                Color { r: 0.2, g: 1.0, b: 0.2, a: 1.0 },   // Primary: bright green
                Color { r: 0.1, g: 0.6, b: 0.1, a: 1.0 },   // Secondary: dark green
                Color { r: 0.3, g: 0.8, b: 0.3, a: 1.0 },   // Accent: medium green
                Color { r: 0.5, g: 1.0, b: 0.5, a: 1.0 },   // Highlight: light green
            ],
            PhosphorColor::Amber => [
                Color { r: 1.0, g: 0.69, b: 0.0, a: 1.0 },  // Primary: bright amber
                Color { r: 0.6, g: 0.4, b: 0.0, a: 1.0 },   // Secondary: dark amber
                Color { r: 0.8, g: 0.55, b: 0.0, a: 1.0 },  // Accent: medium amber
                Color { r: 1.0, g: 0.8, b: 0.3, a: 1.0 },   // Highlight: light amber
            ],
            PhosphorColor::White => [
                Color { r: 0.9, g: 0.9, b: 0.85, a: 1.0 },  // Primary: white
                Color { r: 0.5, g: 0.5, b: 0.48, a: 1.0 },  // Secondary: gray
                Color { r: 0.7, g: 0.7, b: 0.67, a: 1.0 },  // Accent: light gray
                Color { r: 1.0, g: 1.0, b: 0.95, a: 1.0 },  // Highlight: bright white
            ],
            PhosphorColor::Blue => [
                Color { r: 0.4, g: 0.6, b: 1.0, a: 1.0 },   // Primary: bright blue
                Color { r: 0.2, g: 0.3, b: 0.6, a: 1.0 },   // Secondary: dark blue
                Color { r: 0.3, g: 0.5, b: 0.8, a: 1.0 },   // Accent: medium blue
                Color { r: 0.6, g: 0.8, b: 1.0, a: 1.0 },   // Highlight: light blue
            ],
            PhosphorColor::Custom(c) => {
                // For custom, generate variations based on the custom color
                let dim = Color { r: c.r * 0.6, g: c.g * 0.6, b: c.b * 0.6, a: c.a };
                let accent = Color { r: c.r * 0.8, g: c.g * 0.8, b: c.b * 0.8, a: c.a };
                let highlight = Color {
                    r: (c.r * 1.2).min(1.0),
                    g: (c.g * 1.2).min(1.0),
                    b: (c.b * 1.2).min(1.0),
                    a: c.a
                };
                [*c, dim, accent, highlight]
            }
        }
    }

    /// Create the Theme configuration page
    fn create_theme_page(
        config: &Rc<RefCell<RetroTerminalDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        theme_widgets_out: &Rc<RefCell<Option<ThemeWidgets>>>,
        theme_ref_refreshers: &Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
        colors_widgets_out: &Rc<RefCell<Option<ColorsWidgets>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        Self::set_page_margins(&page);

        let scroll = ScrolledWindow::new();
        scroll.set_vexpand(true);
        scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);

        let inner_box = GtkBox::new(Orientation::Vertical, 8);

        // Shared reference for theme color widgets (populated later, accessed by phosphor callback)
        let theme_color_widgets: Rc<RefCell<Vec<Rc<ColorButtonWidget>>>> = Rc::new(RefCell::new(Vec::new()));

        // Info label
        let info_label = Label::new(Some("Configure terminal and theme colors, gradient, and fonts.\nThese can be referenced in content items for consistent styling."));
        info_label.set_halign(gtk4::Align::Start);
        info_label.add_css_class("dim-label");
        info_label.set_wrap(true);
        inner_box.append(&info_label);

        // ========== Terminal Colors section (merged from Colors tab) ==========
        let terminal_colors_frame = gtk4::Frame::new(Some("Terminal Colors"));
        let terminal_colors_box = GtkBox::new(Orientation::Vertical, 6);
        terminal_colors_box.set_margin_start(8);
        terminal_colors_box.set_margin_end(8);
        terminal_colors_box.set_margin_top(8);
        terminal_colors_box.set_margin_bottom(8);

        // Phosphor color preset
        let phosphor_box = GtkBox::new(Orientation::Horizontal, 6);
        phosphor_box.append(&Label::new(Some("Phosphor Color:")));
        let phosphor_list = StringList::new(&["Green (P1)", "Amber (P3)", "White (P4)", "Blue", "Custom"]);
        let phosphor_dropdown = DropDown::new(Some(phosphor_list), None::<gtk4::Expression>);
        let phosphor_idx = match &config.borrow().frame.phosphor_color {
            PhosphorColor::Green => 0,
            PhosphorColor::Amber => 1,
            PhosphorColor::White => 2,
            PhosphorColor::Blue => 3,
            PhosphorColor::Custom(_) => 4,
        };
        phosphor_dropdown.set_selected(phosphor_idx);
        phosphor_dropdown.set_hexpand(true);
        phosphor_box.append(&phosphor_dropdown);
        terminal_colors_box.append(&phosphor_box);

        // Custom phosphor color (shown only when Custom is selected)
        let custom_phosphor_box = GtkBox::new(Orientation::Horizontal, 6);
        custom_phosphor_box.append(&Label::new(Some("Custom Color:")));
        let custom_color = if let PhosphorColor::Custom(c) = &config.borrow().frame.phosphor_color {
            *c
        } else {
            Color { r: 0.2, g: 1.0, b: 0.2, a: 1.0 }
        };
        let custom_phosphor_widget = Rc::new(ColorButtonWidget::new(custom_color));
        custom_phosphor_box.append(custom_phosphor_widget.widget());
        custom_phosphor_box.set_visible(phosphor_idx == 4);
        terminal_colors_box.append(&custom_phosphor_box);

        // Connect phosphor dropdown
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let custom_box_clone = custom_phosphor_box.clone();
        let custom_widget_clone = custom_phosphor_widget.clone();
        let theme_color_widgets_clone = theme_color_widgets.clone();
        let theme_ref_refreshers_clone = theme_ref_refreshers.clone();
        phosphor_dropdown.connect_selected_notify(move |dropdown| {
            let selected = dropdown.selected();
            if selected == gtk4::INVALID_LIST_POSITION {
                return;
            }
            custom_box_clone.set_visible(selected == 4);
            let phosphor_color = match selected {
                0 => PhosphorColor::Green,
                1 => PhosphorColor::Amber,
                2 => PhosphorColor::White,
                3 => PhosphorColor::Blue,
                _ => PhosphorColor::Custom(custom_widget_clone.color()),
            };

            // Get the corresponding theme colors for this phosphor
            let theme_colors = Self::theme_colors_for_phosphor(&phosphor_color);

            // Update the config
            {
                let mut cfg = config_clone.borrow_mut();
                cfg.frame.phosphor_color = phosphor_color;
                cfg.frame.theme.color1 = theme_colors[0];
                cfg.frame.theme.color2 = theme_colors[1];
                cfg.frame.theme.color3 = theme_colors[2];
                cfg.frame.theme.color4 = theme_colors[3];
            }

            // Update the theme color widgets if they exist
            let widgets = theme_color_widgets_clone.borrow();
            if widgets.len() == 4 {
                widgets[0].set_color(theme_colors[0]);
                widgets[1].set_color(theme_colors[1]);
                widgets[2].set_color(theme_colors[2]);
                widgets[3].set_color(theme_colors[3]);
            }
            drop(widgets);

            // Refresh all theme-linked widgets
            Self::refresh_theme_refs(&theme_ref_refreshers_clone);

            Self::queue_redraw(&preview_clone, &on_change_clone);
        });

        // Connect custom color widget
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let theme_color_widgets_clone = theme_color_widgets.clone();
        let theme_ref_refreshers_clone = theme_ref_refreshers.clone();
        custom_phosphor_widget.set_on_change(move |color| {
            let phosphor_color = PhosphorColor::Custom(color);

            // Get the corresponding theme colors for this phosphor
            let theme_colors = Self::theme_colors_for_phosphor(&phosphor_color);

            // Update the config
            {
                let mut cfg = config_clone.borrow_mut();
                cfg.frame.phosphor_color = phosphor_color;
                cfg.frame.theme.color1 = theme_colors[0];
                cfg.frame.theme.color2 = theme_colors[1];
                cfg.frame.theme.color3 = theme_colors[2];
                cfg.frame.theme.color4 = theme_colors[3];
            }

            // Update the theme color widgets if they exist
            let widgets = theme_color_widgets_clone.borrow();
            if widgets.len() == 4 {
                widgets[0].set_color(theme_colors[0]);
                widgets[1].set_color(theme_colors[1]);
                widgets[2].set_color(theme_colors[2]);
                widgets[3].set_color(theme_colors[3]);
            }
            drop(widgets);

            // Refresh all theme-linked widgets
            Self::refresh_theme_refs(&theme_ref_refreshers_clone);

            Self::queue_redraw(&preview_clone, &on_change_clone);
        });

        // Background color
        let bg_box = GtkBox::new(Orientation::Horizontal, 6);
        bg_box.append(&Label::new(Some("Screen Background:")));
        let background_widget = Rc::new(ColorButtonWidget::new(config.borrow().frame.background_color));
        bg_box.append(background_widget.widget());
        terminal_colors_box.append(&bg_box);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        background_widget.set_on_change(move |color| {
            config_clone.borrow_mut().frame.background_color = color;
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });

        // Text brightness
        let brightness_box = GtkBox::new(Orientation::Horizontal, 6);
        brightness_box.append(&Label::new(Some("Text Brightness:")));
        let brightness_scale = Scale::with_range(Orientation::Horizontal, 0.3, 1.0, 0.05);
        brightness_scale.set_value(config.borrow().frame.text_brightness);
        brightness_scale.set_hexpand(true);
        brightness_scale.set_draw_value(true);
        brightness_box.append(&brightness_scale);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        brightness_scale.connect_value_changed(move |scale| {
            config_clone.borrow_mut().frame.text_brightness = scale.value();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        terminal_colors_box.append(&brightness_box);

        terminal_colors_frame.set_child(Some(&terminal_colors_box));
        inner_box.append(&terminal_colors_frame);

        // Store colors widgets
        *colors_widgets_out.borrow_mut() = Some(ColorsWidgets {
            phosphor_dropdown: phosphor_dropdown.clone(),
            custom_phosphor_widget: custom_phosphor_widget.clone(),
            custom_phosphor_box: custom_phosphor_box.clone(),
            background_widget: background_widget.clone(),
            brightness_scale: brightness_scale.clone(),
        });

        // Theme Colors section
        let colors_frame = gtk4::Frame::new(Some("Theme Colors"));
        let colors_box = GtkBox::new(Orientation::Vertical, 6);
        colors_box.set_margin_start(8);
        colors_box.set_margin_end(8);
        colors_box.set_margin_top(8);
        colors_box.set_margin_bottom(8);

        let color_labels = ["Color 1 (Primary):", "Color 2 (Secondary):", "Color 3 (Accent):", "Color 4 (Highlight):"];
        let mut color_widgets: Vec<Rc<ColorButtonWidget>> = Vec::new();

        for (i, label_text) in color_labels.iter().enumerate() {
            let row = GtkBox::new(Orientation::Horizontal, 8);
            row.append(&Label::new(Some(label_text)));

            let color = match i {
                0 => config.borrow().frame.theme.color1,
                1 => config.borrow().frame.theme.color2,
                2 => config.borrow().frame.theme.color3,
                _ => config.borrow().frame.theme.color4,
            };

            let color_widget = Rc::new(ColorButtonWidget::new(color));
            color_widget.widget().set_hexpand(true);

            let config_clone = config.clone();
            let on_change_clone = on_change.clone();
            let preview_clone = preview.clone();
            let refreshers_clone = theme_ref_refreshers.clone();
            let color_idx = i;
            color_widget.set_on_change(move |new_color| {
                let mut cfg = config_clone.borrow_mut();
                match color_idx {
                    0 => cfg.frame.theme.color1 = new_color,
                    1 => cfg.frame.theme.color2 = new_color,
                    2 => cfg.frame.theme.color3 = new_color,
                    _ => cfg.frame.theme.color4 = new_color,
                }
                drop(cfg);
                Self::refresh_theme_refs(&refreshers_clone);
                Self::queue_redraw(&preview_clone, &on_change_clone);
            });

            row.append(color_widget.widget());
            color_widgets.push(color_widget);
            colors_box.append(&row);
        }

        // Populate the shared theme_color_widgets for phosphor dropdown access
        *theme_color_widgets.borrow_mut() = color_widgets.clone();

        colors_frame.set_child(Some(&colors_box));
        inner_box.append(&colors_frame);

        // Theme Gradient section
        let gradient_frame = gtk4::Frame::new(Some("Theme Gradient"));
        let gradient_box = GtkBox::new(Orientation::Vertical, 6);
        gradient_box.set_margin_start(8);
        gradient_box.set_margin_end(8);
        gradient_box.set_margin_top(8);
        gradient_box.set_margin_bottom(8);

        let gradient_editor = Rc::new(GradientEditor::new());
        gradient_editor.set_theme_config(config.borrow().frame.theme.clone());
        gradient_editor.set_gradient_source_config(&config.borrow().frame.theme.gradient);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let refreshers_clone = theme_ref_refreshers.clone();
        let gradient_editor_for_cb = gradient_editor.clone();
        gradient_editor.set_on_change(move || {
            let gradient_config = gradient_editor_for_cb.get_gradient_source_config();
            config_clone.borrow_mut().frame.theme.gradient = gradient_config;
            Self::refresh_theme_refs(&refreshers_clone);
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });

        // Register a theme refresh callback for the gradient editor
        let gradient_editor_for_refresh = gradient_editor.clone();
        let config_for_gradient_refresh = config.clone();
        let gradient_refresh: Rc<dyn Fn()> = Rc::new(move || {
            let theme = config_for_gradient_refresh.borrow().frame.theme.clone();
            gradient_editor_for_refresh.set_theme_config(theme);
        });
        theme_ref_refreshers.borrow_mut().push(gradient_refresh);

        gradient_box.append(gradient_editor.widget());
        gradient_frame.set_child(Some(&gradient_box));
        inner_box.append(&gradient_frame);

        // Theme Fonts section
        let fonts_frame = gtk4::Frame::new(Some("Theme Fonts"));
        let fonts_box = GtkBox::new(Orientation::Vertical, 6);
        fonts_box.set_margin_start(8);
        fonts_box.set_margin_end(8);
        fonts_box.set_margin_top(8);
        fonts_box.set_margin_bottom(8);

        // Font 1
        let font1_row = GtkBox::new(Orientation::Horizontal, 8);
        font1_row.append(&Label::new(Some("Font 1 (Headers):")));
        let font1_btn = Button::with_label(&config.borrow().frame.theme.font1_family);
        font1_btn.set_hexpand(true);
        let config_for_font1 = config.clone();
        let on_change_for_font1 = on_change.clone();
        let preview_for_font1 = preview.clone();
        let refreshers_for_font1 = theme_ref_refreshers.clone();
        let font1_btn_clone = font1_btn.clone();
        font1_btn.connect_clicked(move |btn| {
            if let Some(window) = btn.root().and_then(|r| r.downcast::<gtk4::Window>().ok()) {
                let current_font = config_for_font1.borrow().frame.theme.font1_family.clone();
                let font_desc = gtk4::pango::FontDescription::from_string(&current_font);
                let config_c = config_for_font1.clone();
                let on_change_c = on_change_for_font1.clone();
                let preview_c = preview_for_font1.clone();
                let btn_c = font1_btn_clone.clone();
                let refreshers_c = refreshers_for_font1.clone();
                shared_font_dialog().choose_font(
                    Some(&window),
                    Some(&font_desc),
                    gtk4::gio::Cancellable::NONE,
                    move |result| {
                        if let Ok(font_desc) = result {
                            let family = font_desc.family()
                                .map(|s| s.to_string())
                                .unwrap_or_else(|| "monospace".to_string());
                            config_c.borrow_mut().frame.theme.font1_family = family.clone();
                            btn_c.set_label(&family);
                            Self::refresh_theme_refs(&refreshers_c);
                            Self::queue_redraw(&preview_c, &on_change_c);
                        }
                    },
                );
            }
        });
        font1_row.append(&font1_btn);

        font1_row.append(&Label::new(Some("Size:")));
        let font1_size_spin = SpinButton::with_range(6.0, 72.0, 1.0);
        font1_size_spin.set_value(config.borrow().frame.theme.font1_size);
        let config_for_font1_size = config.clone();
        let on_change_for_font1_size = on_change.clone();
        let preview_for_font1_size = preview.clone();
        let refreshers_for_font1_size = theme_ref_refreshers.clone();
        font1_size_spin.connect_value_changed(move |spin| {
            config_for_font1_size.borrow_mut().frame.theme.font1_size = spin.value();
            Self::refresh_theme_refs(&refreshers_for_font1_size);
            Self::queue_redraw(&preview_for_font1_size, &on_change_for_font1_size);
        });
        font1_row.append(&font1_size_spin);
        fonts_box.append(&font1_row);

        // Font 2
        let font2_row = GtkBox::new(Orientation::Horizontal, 8);
        font2_row.append(&Label::new(Some("Font 2 (Content):")));
        let font2_btn = Button::with_label(&config.borrow().frame.theme.font2_family);
        font2_btn.set_hexpand(true);
        let config_for_font2 = config.clone();
        let on_change_for_font2 = on_change.clone();
        let preview_for_font2 = preview.clone();
        let refreshers_for_font2 = theme_ref_refreshers.clone();
        let font2_btn_clone = font2_btn.clone();
        font2_btn.connect_clicked(move |btn| {
            if let Some(window) = btn.root().and_then(|r| r.downcast::<gtk4::Window>().ok()) {
                let current_font = config_for_font2.borrow().frame.theme.font2_family.clone();
                let font_desc = gtk4::pango::FontDescription::from_string(&current_font);
                let config_c = config_for_font2.clone();
                let on_change_c = on_change_for_font2.clone();
                let preview_c = preview_for_font2.clone();
                let btn_c = font2_btn_clone.clone();
                let refreshers_c = refreshers_for_font2.clone();
                shared_font_dialog().choose_font(
                    Some(&window),
                    Some(&font_desc),
                    gtk4::gio::Cancellable::NONE,
                    move |result| {
                        if let Ok(font_desc) = result {
                            let family = font_desc.family()
                                .map(|s| s.to_string())
                                .unwrap_or_else(|| "monospace".to_string());
                            config_c.borrow_mut().frame.theme.font2_family = family.clone();
                            btn_c.set_label(&family);
                            Self::refresh_theme_refs(&refreshers_c);
                            Self::queue_redraw(&preview_c, &on_change_c);
                        }
                    },
                );
            }
        });
        font2_row.append(&font2_btn);

        font2_row.append(&Label::new(Some("Size:")));
        let font2_size_spin = SpinButton::with_range(6.0, 72.0, 1.0);
        font2_size_spin.set_value(config.borrow().frame.theme.font2_size);
        let config_for_font2_size = config.clone();
        let on_change_for_font2_size = on_change.clone();
        let preview_for_font2_size = preview.clone();
        let refreshers_for_font2_size = theme_ref_refreshers.clone();
        font2_size_spin.connect_value_changed(move |spin| {
            config_for_font2_size.borrow_mut().frame.theme.font2_size = spin.value();
            Self::refresh_theme_refs(&refreshers_for_font2_size);
            Self::queue_redraw(&preview_for_font2_size, &on_change_for_font2_size);
        });
        font2_row.append(&font2_size_spin);
        fonts_box.append(&font2_row);

        fonts_frame.set_child(Some(&fonts_box));
        inner_box.append(&fonts_frame);

        // Color Scheme presets section
        let preset_frame = gtk4::Frame::new(Some("Color Scheme Presets"));
        let preset_box = GtkBox::new(Orientation::Vertical, 6);
        preset_box.set_margin_start(8);
        preset_box.set_margin_end(8);
        preset_box.set_margin_top(8);
        preset_box.set_margin_bottom(8);

        let preset_row = GtkBox::new(Orientation::Horizontal, 8);
        preset_row.append(&Label::new(Some("Apply Preset:")));
        let preset_list = StringList::new(&[
            "Retro Terminal (Default)",
            "Synthwave",
            "LCARS",
            "Industrial",
            "Material",
            "Cyberpunk",
            "Fighter HUD",
        ]);
        let preset_dropdown = DropDown::new(Some(preset_list), None::<gtk4::Expression>);
        preset_dropdown.set_hexpand(true);
        preset_dropdown.set_selected(gtk4::INVALID_LIST_POSITION);

        let config_for_preset = config.clone();
        let on_change_for_preset = on_change.clone();
        let preview_for_preset = preview.clone();
        let refreshers_for_preset = theme_ref_refreshers.clone();
        let color_widgets_clone = color_widgets.clone();
        let gradient_editor_for_preset = gradient_editor.clone();
        let font1_btn_for_preset = font1_btn.clone();
        let font1_size_spin_for_preset = font1_size_spin.clone();
        let font2_btn_for_preset = font2_btn.clone();
        let font2_size_spin_for_preset = font2_size_spin.clone();
        preset_dropdown.connect_selected_notify(move |dropdown| {
            use crate::ui::theme::ComboThemeConfig;
            let theme = match dropdown.selected() {
                0 => ComboThemeConfig::default_for_retro_terminal(),
                1 => ComboThemeConfig::default_for_synthwave(),
                2 => ComboThemeConfig::default_for_lcars(),
                3 => ComboThemeConfig::default_for_industrial(),
                4 => ComboThemeConfig::default_for_material(),
                5 => ComboThemeConfig::default_for_cyberpunk(),
                6 => ComboThemeConfig::default_for_fighter_hud(),
                _ => return,
            };
            config_for_preset.borrow_mut().frame.theme = theme.clone();
            // Update UI widgets
            if color_widgets_clone.len() >= 4 {
                color_widgets_clone[0].set_color(theme.color1);
                color_widgets_clone[1].set_color(theme.color2);
                color_widgets_clone[2].set_color(theme.color3);
                color_widgets_clone[3].set_color(theme.color4);
            }
            gradient_editor_for_preset.set_gradient_source_config(&theme.gradient);
            font1_btn_for_preset.set_label(&theme.font1_family);
            font1_size_spin_for_preset.set_value(theme.font1_size);
            font2_btn_for_preset.set_label(&theme.font2_family);
            font2_size_spin_for_preset.set_value(theme.font2_size);
            Self::refresh_theme_refs(&refreshers_for_preset);
            Self::queue_redraw(&preview_for_preset, &on_change_for_preset);
        });
        preset_row.append(&preset_dropdown);
        preset_box.append(&preset_row);
        preset_frame.set_child(Some(&preset_box));
        inner_box.append(&preset_frame);

        scroll.set_child(Some(&inner_box));
        page.append(&scroll);

        // Store theme widgets for later updates
        *theme_widgets_out.borrow_mut() = Some(ThemeWidgets {
            theme_color1_widget: color_widgets[0].clone(),
            theme_color2_widget: color_widgets[1].clone(),
            theme_color3_widget: color_widgets[2].clone(),
            theme_color4_widget: color_widgets[3].clone(),
            theme_gradient_editor: gradient_editor,
            font1_btn: font1_btn.clone(),
            font1_size_spin: font1_size_spin.clone(),
            font2_btn: font2_btn.clone(),
            font2_size_spin: font2_size_spin.clone(),
        });

        page
    }

    /// Create a theme reference section showing current theme colors and fonts with copy buttons
    #[allow(dead_code)]
    fn create_theme_reference_section(
        config: &Rc<RefCell<RetroTerminalDisplayConfig>>,
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

            let swatch = DrawingArea::new();
            swatch.set_size_request(20, 20);
            let config_for_draw = config.clone();
            let color_idx = *idx;
            swatch.set_draw_func(move |_, cr, width, height| {
                let c = config_for_draw.borrow().frame.theme.get_color(color_idx);
                let checker_size = 4.0;
                for y in 0..(height as f64 / checker_size).ceil() as i32 {
                    for x in 0..(width as f64 / checker_size).ceil() as i32 {
                        if (x + y) % 2 == 0 {
                            cr.set_source_rgb(0.8, 0.8, 0.8);
                        } else {
                            cr.set_source_rgb(0.6, 0.6, 0.6);
                        }
                        cr.rectangle(x as f64 * checker_size, y as f64 * checker_size, checker_size, checker_size);
                        let _ = cr.fill();
                    }
                }
                cr.set_source_rgba(c.r, c.g, c.b, c.a);
                cr.rectangle(0.0, 0.0, width as f64, height as f64);
                let _ = cr.fill();
                cr.set_source_rgb(0.3, 0.3, 0.3);
                cr.set_line_width(1.0);
                cr.rectangle(0.5, 0.5, width as f64 - 1.0, height as f64 - 1.0);
                let _ = cr.stroke();
            });
            color_swatches.borrow_mut().push(swatch.clone());
            item_box.append(&swatch);

            let copy_btn = Button::from_icon_name("edit-copy-symbolic");
            copy_btn.set_tooltip_text(Some(&format!("Copy {} to clipboard", tooltip)));
            let config_for_copy = config.clone();
            let color_idx_for_copy = *idx;
            copy_btn.connect_clicked(move |_| {
                let c = config_for_copy.borrow().frame.theme.get_color(color_idx_for_copy);
                if let Ok(mut clipboard) = CLIPBOARD.lock() {
                    clipboard.copy_color(c.r, c.g, c.b, c.a);
                }
            });
            item_box.append(&copy_btn);
            colors_box.append(&item_box);
        }
        content_box.append(&colors_box);

        // Gradient row
        let gradient_box = GtkBox::new(Orientation::Horizontal, 8);
        gradient_box.append(&Label::new(Some("Gradient:")));

        let gradient_swatch = DrawingArea::new();
        gradient_swatch.set_size_request(60, 20);
        let config_for_gradient = config.clone();
        gradient_swatch.set_draw_func(move |_, cr, width, height| {
            let cfg = config_for_gradient.borrow();
            let gradient_config = cfg.frame.theme.gradient.resolve(&cfg.frame.theme);
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
            cr.set_source_rgb(0.3, 0.3, 0.3);
            cr.set_line_width(1.0);
            cr.rectangle(0.5, 0.5, w - 1.0, h - 1.0);
            let _ = cr.stroke();
        });
        gradient_box.append(&gradient_swatch);

        let gradient_copy_btn = Button::from_icon_name("edit-copy-symbolic");
        gradient_copy_btn.set_tooltip_text(Some("Copy Theme Gradient to clipboard"));
        let config_for_gradient_copy = config.clone();
        gradient_copy_btn.connect_clicked(move |_| {
            let cfg = config_for_gradient_copy.borrow();
            let resolved_gradient = cfg.frame.theme.gradient.resolve(&cfg.frame.theme);
            if let Ok(mut clipboard) = CLIPBOARD.lock() {
                clipboard.copy_gradient_stops(resolved_gradient.stops);
            }
        });
        gradient_box.append(&gradient_copy_btn);
        content_box.append(&gradient_box);

        // Fonts row
        let fonts_box = GtkBox::new(Orientation::Horizontal, 8);
        fonts_box.append(&Label::new(Some("Fonts:")));

        let font_labels: Rc<RefCell<Vec<Label>>> = Rc::new(RefCell::new(Vec::new()));
        let font_indices = [1u8, 2];
        let font_tooltips = ["Font 1 (Headers)", "Font 2 (Content)"];

        for (idx, tooltip) in font_indices.iter().zip(font_tooltips.iter()) {
            let item_box = GtkBox::new(Orientation::Horizontal, 4);
            let (family, size) = config.borrow().frame.theme.get_font(*idx);
            let info = Label::new(Some(&format!("{} {}pt", family, size as i32)));
            info.add_css_class("dim-label");
            font_labels.borrow_mut().push(info.clone());
            item_box.append(&info);

            let copy_btn = Button::from_icon_name("edit-copy-symbolic");
            copy_btn.set_tooltip_text(Some(&format!("Copy {} to clipboard", tooltip)));
            let font_idx = *idx;
            copy_btn.connect_clicked(move |_| {
                if let Ok(mut clipboard) = CLIPBOARD.lock() {
                    clipboard.copy_font_source(FontSource::Theme { index: font_idx, size: 14.0 }, false, false);
                }
            });
            item_box.append(&copy_btn);
            fonts_box.append(&item_box);
        }
        content_box.append(&fonts_box);
        frame.set_child(Some(&content_box));

        let config_for_refresh = config.clone();
        let gradient_swatch_for_refresh = gradient_swatch.clone();
        let refresh_callback: Rc<dyn Fn()> = Rc::new(move || {
            for swatch in color_swatches.borrow().iter() {
                swatch.queue_draw();
            }
            gradient_swatch_for_refresh.queue_draw();
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

    fn create_content_page(
        config: &Rc<RefCell<RetroTerminalDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        content_notebook: &Rc<RefCell<Notebook>>,
        source_summaries: &Rc<RefCell<Vec<(String, String, usize, u32)>>>,
        available_fields: &Rc<RefCell<Vec<FieldMetadata>>>,
        theme_ref_refreshers: &Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        Self::set_page_margins(&page);

        let info_label = Label::new(Some("Configure content items for each slot."));
        info_label.set_halign(gtk4::Align::Start);
        info_label.add_css_class("dim-label");
        page.append(&info_label);

        let scrolled = ScrolledWindow::new();
        scrolled.set_vexpand(true);
        scrolled.set_hexpand(true);
        scrolled.set_min_content_height(300);

        let notebook = content_notebook.borrow();
        scrolled.set_child(Some(&*notebook));
        page.append(&scrolled);

        drop(notebook);
        Self::rebuild_content_tabs(config, on_change, preview, content_notebook, source_summaries, available_fields, theme_ref_refreshers);

        page
    }

    fn rebuild_content_tabs(
        config: &Rc<RefCell<RetroTerminalDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        content_notebook: &Rc<RefCell<Notebook>>,
        source_summaries: &Rc<RefCell<Vec<(String, String, usize, u32)>>>,
        available_fields: &Rc<RefCell<Vec<FieldMetadata>>>,
        theme_ref_refreshers: &Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
    ) {
        combo_config_base::rebuild_content_tabs(
            config,
            on_change,
            preview,
            content_notebook,
            source_summaries,
            available_fields,
            |cfg| &cfg.frame.content_items,
            |cfg, slot_name, item| {
                cfg.frame.content_items.insert(slot_name.to_string(), item);
            },
            theme_ref_refreshers,
            |cfg| cfg.frame.theme.clone(),
        );
    }

    /// Default bar config with terminal/phosphor colors
    #[allow(dead_code)]
    #[allow(clippy::field_reassign_with_default)]
    fn default_bar_config_terminal() -> crate::ui::BarDisplayConfig {
        use crate::ui::bar_display::{BarDisplayConfig, BarStyle, BarOrientation, BarFillDirection, BarFillType, BarBackgroundType, BorderConfig};

        let mut config = BarDisplayConfig::default();
        config.style = BarStyle::Full;
        config.orientation = BarOrientation::Horizontal;
        config.fill_direction = BarFillDirection::LeftToRight;

        // Terminal green phosphor
        config.foreground = BarFillType::Solid {
            color: crate::ui::theme::ColorSource::custom(Color { r: 0.2, g: 1.0, b: 0.2, a: 1.0 })
        };
        config.background = BarBackgroundType::Solid {
            color: crate::ui::theme::ColorSource::custom(Color { r: 0.05, g: 0.1, b: 0.05, a: 1.0 })
        };
        config.border = BorderConfig {
            enabled: true,
            color: crate::ui::theme::ColorSource::custom(Color { r: 0.1, g: 0.5, b: 0.1, a: 1.0 }),
            width: 1.0,
        };
        config.corner_radius = 0.0;

        config
    }

    /// Default graph config with terminal colors
    #[allow(dead_code)]
    #[allow(clippy::field_reassign_with_default)]
    fn default_graph_config_terminal() -> crate::ui::GraphDisplayConfig {
        use crate::ui::graph_display::{GraphDisplayConfig, GraphType, LineStyle, FillMode};

        let mut config = GraphDisplayConfig::default();
        config.graph_type = GraphType::Line;
        config.line_style = LineStyle::Solid;
        config.line_width = 1.5;
        config.line_color = ColorSource::custom(Color { r: 0.2, g: 1.0, b: 0.2, a: 1.0 });  // Phosphor green
        config.fill_mode = FillMode::Gradient;
        config.fill_gradient_start = ColorSource::custom(Color { r: 0.1, g: 0.5, b: 0.1, a: 0.3 });
        config.fill_gradient_end = ColorSource::custom(Color { r: 0.05, g: 0.2, b: 0.05, a: 0.0 });
        config.background_color = Color { r: 0.02, g: 0.02, b: 0.02, a: 1.0 };
        config.plot_background_color = Color { r: 0.02, g: 0.05, b: 0.02, a: 1.0 };
        config.x_axis.show_grid = true;
        config.x_axis.grid_color = ColorSource::custom(Color { r: 0.05, g: 0.2, b: 0.05, a: 1.0 });
        config.y_axis.show_grid = true;
        config.y_axis.grid_color = ColorSource::custom(Color { r: 0.05, g: 0.2, b: 0.05, a: 1.0 });

        config
    }

    fn create_animation_page(
        config: &Rc<RefCell<RetroTerminalDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        animation_widgets_out: &Rc<RefCell<Option<AnimationWidgets>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        Self::set_page_margins(&page);

        // Enable animation
        let enable_check = CheckButton::with_label("Enable Animation");
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

        // Cursor blink
        let cursor_blink_check = CheckButton::with_label("Cursor Blink");
        cursor_blink_check.set_active(config.borrow().frame.cursor_blink);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        cursor_blink_check.connect_toggled(move |check| {
            config_clone.borrow_mut().frame.cursor_blink = check.is_active();
            if let Some(cb) = on_change_clone.borrow().as_ref() {
                cb();
            }
        });
        page.append(&cursor_blink_check);

        // Typewriter effect
        let typewriter_check = CheckButton::with_label("Typewriter Text Effect");
        typewriter_check.set_active(config.borrow().frame.typewriter_effect);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        typewriter_check.connect_toggled(move |check| {
            config_clone.borrow_mut().frame.typewriter_effect = check.is_active();
            if let Some(cb) = on_change_clone.borrow().as_ref() {
                cb();
            }
        });
        page.append(&typewriter_check);

        // Store widget refs
        *animation_widgets_out.borrow_mut() = Some(AnimationWidgets {
            enable_check,
            cursor_blink_check,
            typewriter_check,
        });

        page
    }

    // Public API

    pub fn widget(&self) -> &GtkBox {
        &self.container
    }

    pub fn set_on_change<F: Fn() + 'static>(&self, callback: F) {
        *self.on_change.borrow_mut() = Some(Box::new(callback));
    }

    /// Set the theme configuration. Call this BEFORE set_config to ensure
    /// font selectors have the correct theme when the UI is rebuilt.
    pub fn set_theme(&self, theme: crate::ui::theme::ComboThemeConfig) {
        self.config.borrow_mut().frame.theme = theme;
        // Trigger all theme refreshers to update child widgets
        for refresher in self.theme_ref_refreshers.borrow().iter() {
            refresher();
        }
    }

    pub fn get_config(&self) -> RetroTerminalDisplayConfig {
        self.config.borrow().clone()
    }

    /// Get a reference to the internal config Rc for use in callbacks
    pub fn get_config_rc(&self) -> Rc<RefCell<RetroTerminalDisplayConfig>> {
        self.config.clone()
    }

    pub fn set_config(&self, config: &RetroTerminalDisplayConfig) {
        // IMPORTANT: Temporarily disable on_change callback to prevent signal cascade.
        let saved_callback = self.on_change.borrow_mut().take();

        *self.config.borrow_mut() = config.clone();

        // Update Colors widgets
        if let Some(widgets) = self.colors_widgets.borrow().as_ref() {
            let phosphor_idx = match &config.frame.phosphor_color {
                PhosphorColor::Green => 0,
                PhosphorColor::Amber => 1,
                PhosphorColor::White => 2,
                PhosphorColor::Blue => 3,
                PhosphorColor::Custom(_) => 4,
            };
            widgets.phosphor_dropdown.set_selected(phosphor_idx);
            widgets.custom_phosphor_box.set_visible(phosphor_idx == 4);
            if let PhosphorColor::Custom(c) = &config.frame.phosphor_color {
                widgets.custom_phosphor_widget.set_color(*c);
            }
            widgets.background_widget.set_color(config.frame.background_color);
            widgets.brightness_scale.set_value(config.frame.text_brightness);
        }

        // Update Effects widgets
        if let Some(widgets) = self.effects_widgets.borrow().as_ref() {
            widgets.scanline_intensity_scale.set_value(config.frame.scanline_intensity);
            widgets.scanline_spacing_spin.set_value(config.frame.scanline_spacing);
            widgets.curvature_scale.set_value(config.frame.curvature_amount);
            widgets.vignette_scale.set_value(config.frame.vignette_intensity);
            widgets.glow_scale.set_value(config.frame.screen_glow);
            widgets.flicker_check.set_active(config.frame.flicker_enabled);
        }

        // Update Bezel widgets
        if let Some(widgets) = self.bezel_widgets.borrow().as_ref() {
            widgets.style_dropdown.set_selected(match config.frame.bezel_style {
                BezelStyle::Classic => 0,
                BezelStyle::Slim => 1,
                BezelStyle::Industrial => 2,
                BezelStyle::None => 3,
            });
            widgets.color_widget.set_color(config.frame.bezel_color);
            widgets.width_spin.set_value(config.frame.bezel_width);
            widgets.show_led_check.set_active(config.frame.show_power_led);
            widgets.led_color_widget.set_color(config.frame.power_led_color);
        }

        // Update Header widgets
        if let Some(widgets) = self.header_widgets.borrow().as_ref() {
            widgets.show_header_check.set_active(config.frame.show_header);
            widgets.header_text_entry.set_text(&config.frame.header_text);
            widgets.header_style_dropdown.set_selected(match config.frame.header_style {
                TerminalHeaderStyle::TitleBar => 0,
                TerminalHeaderStyle::StatusLine => 1,
                TerminalHeaderStyle::Prompt => 2,
                TerminalHeaderStyle::None => 3,
            });
            widgets.header_height_spin.set_value(config.frame.header_height);
            widgets.header_font_selector.set_source(config.frame.header_font.clone());
        }

        // Update Layout widgets
        if let Some(widgets) = self.layout_widgets.borrow().as_ref() {
            widgets.split_orientation_dropdown.set_selected(match config.frame.split_orientation {
                SplitOrientation::Horizontal => 0,
                SplitOrientation::Vertical => 1,
            });
            widgets.content_padding_spin.set_value(config.frame.content_padding);
            widgets.divider_style_dropdown.set_selected(match config.frame.divider_style {
                TerminalDividerStyle::Dashed => 0,
                TerminalDividerStyle::Solid => 1,
                TerminalDividerStyle::BoxDrawing => 2,
                TerminalDividerStyle::Pipe => 3,
                TerminalDividerStyle::Ascii => 4,
                TerminalDividerStyle::None => 5,
            });
            widgets.divider_padding_spin.set_value(config.frame.divider_padding);

            Self::rebuild_group_spinners(
                &self.config,
                &self.on_change,
                &self.preview,
                &widgets.group_weights_box,
            );
            combo_config_base::rebuild_item_orientation_dropdowns(
                &widgets.item_orientations_box,
                &self.config,
                |c: &mut RetroTerminalDisplayConfig| &mut c.frame,
                &self.on_change,
                &self.preview,
            );
        }

        // Update Animation widgets
        if let Some(widgets) = self.animation_widgets.borrow().as_ref() {
            widgets.enable_check.set_active(config.animation_enabled);
            widgets.cursor_blink_check.set_active(config.frame.cursor_blink);
            widgets.typewriter_check.set_active(config.frame.typewriter_effect);
        }

        // Update Theme widgets (fonts and colors)
        if let Some(ref widgets) = *self.theme_widgets.borrow() {
            widgets.theme_color1_widget.set_color(config.frame.theme.color1);
            widgets.theme_color2_widget.set_color(config.frame.theme.color2);
            widgets.theme_color3_widget.set_color(config.frame.theme.color3);
            widgets.theme_color4_widget.set_color(config.frame.theme.color4);
            widgets.theme_gradient_editor.set_gradient_source_config(&config.frame.theme.gradient);
            widgets.font1_btn.set_label(&config.frame.theme.font1_family);
            widgets.font1_size_spin.set_value(config.frame.theme.font1_size);
            widgets.font2_btn.set_label(&config.frame.theme.font2_family);
            widgets.font2_size_spin.set_value(config.frame.theme.font2_size);
        }

        // Rebuild content tabs
        Self::rebuild_content_tabs(
            &self.config,
            &self.on_change,
            &self.preview,
            &self.content_notebook,
            &self.source_summaries,
            &self.available_fields,
            &self.theme_ref_refreshers,
        );

        // Restore the on_change callback now that widget updates are complete
        *self.on_change.borrow_mut() = saved_callback;

        self.preview.queue_draw();
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

        let mut group_nums: Vec<usize> = group_item_counts.keys().cloned().collect();
        group_nums.sort();
        let group_counts: Vec<usize> = group_nums.iter()
            .map(|n| *group_item_counts.get(n).unwrap_or(&0) as usize)
            .collect();

        // Update the frame config with group information
        {
            let mut cfg = self.config.borrow_mut();
            let new_group_count = group_nums.len();
            cfg.frame.group_count = new_group_count;
            cfg.frame.group_item_counts = group_counts;

            while cfg.frame.group_size_weights.len() < new_group_count {
                cfg.frame.group_size_weights.push(1.0);
            }
            cfg.frame.group_size_weights.truncate(new_group_count);
        }

        *self.source_summaries.borrow_mut() = summaries;

        // Rebuild group weight spinners and item orientation dropdowns in Layout tab
        if let Some(widgets) = self.layout_widgets.borrow().as_ref() {
            Self::rebuild_group_spinners(
                &self.config,
                &self.on_change,
                &self.preview,
                &widgets.group_weights_box,
            );
            combo_config_base::rebuild_item_orientation_dropdowns(
                &widgets.item_orientations_box,
                &self.config,
                |c: &mut RetroTerminalDisplayConfig| &mut c.frame,
                &self.on_change,
                &self.preview,
            );
        }

        Self::rebuild_content_tabs(
            &self.config,
            &self.on_change,
            &self.preview,
            &self.content_notebook,
            &self.source_summaries,
            &self.available_fields,
            &self.theme_ref_refreshers,
        );

        // Notify that config has changed so displayer gets updated
        if let Some(cb) = self.on_change.borrow().as_ref() {
            cb();
        }
    }

    /// Update the available fields for content configuration.
    /// NOTE: This only stores the fields - it does NOT rebuild tabs.
    /// Call set_source_summaries() after this to trigger the rebuild.
    pub fn set_available_fields(&self, fields: Vec<FieldMetadata>) {
        *self.available_fields.borrow_mut() = fields;
        // Don't rebuild here - set_source_summaries() will be called next and will rebuild
    }
}

impl Default for RetroTerminalConfigWidget {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}
