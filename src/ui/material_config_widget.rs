//! Material Cards configuration widget
//!
//! Provides a tabbed interface for configuring all aspects of the Material display.

use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, CheckButton, DrawingArea, DropDown, Entry, Label, Notebook, Orientation,
    Scale, SpinButton, StringList, ScrolledWindow,
};
use std::cell::RefCell;
use std::rc::Rc;

use crate::ui::shared_font_dialog::shared_font_dialog;
use crate::ui::color_button_widget::ColorButtonWidget;
use crate::ui::material_display::{
    render_material_frame, CardElevation, HeaderStyle, HeaderAlignment, DividerStyle, ThemeVariant,
};
use crate::ui::lcars_display::SplitOrientation;
use crate::displayers::MaterialDisplayConfig;
use crate::core::FieldMetadata;
use crate::ui::combo_config_base;
use crate::ui::theme::{ColorSource, ComboThemeConfig, FontSource};
use crate::ui::theme_font_selector::ThemeFontSelector;
use crate::ui::theme_color_selector::ThemeColorSelector;
use crate::ui::widget_builder::{ConfigWidgetBuilder, ConfigWidgetBuilderColorExt, ConfigWidgetBuilderThemeSelectorExt, create_section_header};

/// Holds references to Theme tab widgets (combo theme colors, fonts, gradient, and appearance)
#[allow(dead_code)]
struct ThemeWidgets {
    theme_variant_dropdown: DropDown,
    theme_color1_widget: Rc<ColorButtonWidget>,
    theme_color2_widget: Rc<ColorButtonWidget>,
    theme_color3_widget: Rc<ColorButtonWidget>,
    theme_color4_widget: Rc<ColorButtonWidget>,
    theme_gradient_editor: Rc<crate::ui::gradient_editor::GradientEditor>,
    font1_btn: Button,
    font1_size_spin: SpinButton,
    font2_btn: Button,
    font2_size_spin: SpinButton,
    surface_light_widget: Rc<ColorButtonWidget>,
    surface_dark_widget: Rc<ColorButtonWidget>,
    bg_light_widget: Rc<ColorButtonWidget>,
    bg_dark_widget: Rc<ColorButtonWidget>,
}

/// Holds references to Card tab widgets
struct CardWidgets {
    elevation_dropdown: DropDown,
    corner_radius_spin: SpinButton,
    card_padding_spin: SpinButton,
    shadow_blur_spin: SpinButton,
    shadow_offset_spin: SpinButton,
    shadow_color_widget: Rc<ColorButtonWidget>,
}

/// Holds references to Header tab widgets
struct HeaderWidgets {
    show_header_check: CheckButton,
    header_text_entry: Entry,
    header_style_dropdown: DropDown,
    header_alignment_dropdown: DropDown,
    header_color_selector: Rc<ThemeColorSelector>,
    header_height_spin: SpinButton,
    header_font_selector: Rc<ThemeFontSelector>,
}

/// Holds references to Layout tab widgets
struct LayoutWidgets {
    orientation_dropdown: DropDown,
    content_padding_spin: SpinButton,
    item_spacing_spin: SpinButton,
    divider_style_dropdown: DropDown,
    divider_spacing_spin: SpinButton,
    divider_color_widget: Rc<ThemeColorSelector>,
    group_weights_box: GtkBox,
    item_orientations_box: GtkBox,
}

/// Holds references to Animation tab widgets
struct AnimationWidgets {
    enable_check: CheckButton,
    speed_scale: Scale,
}

/// Material configuration widget
pub struct MaterialConfigWidget {
    container: GtkBox,
    config: Rc<RefCell<MaterialDisplayConfig>>,
    on_change: Rc<RefCell<Option<Box<dyn Fn()>>>>,
    preview: DrawingArea,
    content_notebook: Rc<RefCell<Notebook>>,
    source_summaries: Rc<RefCell<Vec<(String, String, usize, u32)>>>,
    available_fields: Rc<RefCell<Vec<FieldMetadata>>>,
    card_widgets: Rc<RefCell<Option<CardWidgets>>>,
    header_widgets: Rc<RefCell<Option<HeaderWidgets>>>,
    layout_widgets: Rc<RefCell<Option<LayoutWidgets>>>,
    animation_widgets: Rc<RefCell<Option<AnimationWidgets>>>,
    /// Theme tab widgets (includes appearance settings)
    theme_widgets: Rc<RefCell<Option<ThemeWidgets>>>,
    /// Callbacks to refresh theme reference sections
    #[allow(dead_code)] // Kept for Rc ownership; callbacks are invoked via clones
    theme_ref_refreshers: Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
}

impl MaterialConfigWidget {
    pub fn new(available_fields: Vec<FieldMetadata>) -> Self {
        let container = GtkBox::new(Orientation::Vertical, 12);
        let config = Rc::new(RefCell::new(MaterialDisplayConfig::default()));
        let on_change: Rc<RefCell<Option<Box<dyn Fn()>>>> = Rc::new(RefCell::new(None));
        let source_summaries: Rc<RefCell<Vec<(String, String, usize, u32)>>> = Rc::new(RefCell::new(Vec::new()));
        let available_fields: Rc<RefCell<Vec<FieldMetadata>>> = Rc::new(RefCell::new(available_fields));
        let card_widgets: Rc<RefCell<Option<CardWidgets>>> = Rc::new(RefCell::new(None));
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
            let _ = render_material_frame(cr, &cfg.frame, width as f64, height as f64);
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

        // Tab 1: Theme (combo theme colors, fonts, gradient, appearance) - first for visibility
        let theme_page = Self::create_theme_page(&config, &on_change, &preview, &theme_widgets, &theme_ref_refreshers);
        notebook.append_page(&theme_page, Some(&Label::new(Some("Theme"))));

        // Tab 2: Card
        let card_page = Self::create_card_page(&config, &on_change, &preview, &card_widgets);
        notebook.append_page(&card_page, Some(&Label::new(Some("Card"))));

        // Tab 3: Header
        let header_page = Self::create_header_page(&config, &on_change, &preview, &header_widgets, &theme_ref_refreshers);
        notebook.append_page(&header_page, Some(&Label::new(Some("Header"))));

        // Tab 4: Layout
        let layout_page = Self::create_layout_page(&config, &on_change, &preview, &layout_widgets, &theme_ref_refreshers);
        notebook.append_page(&layout_page, Some(&Label::new(Some("Layout"))));

        // Tab 5: Content - with dynamic per-slot notebook
        let content_notebook = Rc::new(RefCell::new(Notebook::new()));
        let content_page = Self::create_content_page(&config, &on_change, &preview, &content_notebook, &source_summaries, &available_fields, &theme_ref_refreshers);
        notebook.append_page(&content_page, Some(&Label::new(Some("Content"))));

        // Tab 6: Animation
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
            card_widgets,
            header_widgets,
            layout_widgets,
            animation_widgets,
            theme_widgets,
            theme_ref_refreshers,
        }
    }

    /// Create the Theme configuration page (combo theme colors, fonts, gradient, appearance)
    fn create_theme_page(
        config: &Rc<RefCell<MaterialDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        theme_widgets_out: &Rc<RefCell<Option<ThemeWidgets>>>,
        theme_ref_refreshers: &Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
    ) -> GtkBox {
        use crate::ui::gradient_editor::GradientEditor;

        let page = GtkBox::new(Orientation::Vertical, 8);
        combo_config_base::set_page_margins(&page);

        let scroll = ScrolledWindow::new();
        scroll.set_vexpand(true);
        scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);

        let inner_box = GtkBox::new(Orientation::Vertical, 8);

        // Info label
        let info_label = Label::new(Some("Configure theme variant, colors, gradient, and fonts.\nThese can be referenced in content items for consistent styling."));
        info_label.set_halign(gtk4::Align::Start);
        info_label.add_css_class("dim-label");
        info_label.set_wrap(true);
        inner_box.append(&info_label);

        // Theme Variant section (Light/Dark)
        let variant_frame = gtk4::Frame::new(Some("Theme Variant"));
        let variant_box = GtkBox::new(Orientation::Vertical, 6);
        variant_box.set_margin_start(8);
        variant_box.set_margin_end(8);
        variant_box.set_margin_top(8);
        variant_box.set_margin_bottom(8);

        let variant_row = GtkBox::new(Orientation::Horizontal, 8);
        variant_row.append(&Label::new(Some("Variant:")));
        let variant_list = StringList::new(&["Light", "Dark"]);
        let theme_variant_dropdown = DropDown::new(Some(variant_list), None::<gtk4::Expression>);
        let variant_idx = match config.borrow().frame.theme_variant {
            ThemeVariant::Light => 0,
            ThemeVariant::Dark => 1,
        };
        theme_variant_dropdown.set_selected(variant_idx);
        theme_variant_dropdown.set_hexpand(true);
        variant_row.append(&theme_variant_dropdown);
        variant_box.append(&variant_row);

        // Info about default colors
        let variant_info = Label::new(Some("Changing the variant will reset theme colors to defaults."));
        variant_info.set_halign(gtk4::Align::Start);
        variant_info.add_css_class("dim-label");
        variant_box.append(&variant_info);

        variant_frame.set_child(Some(&variant_box));
        inner_box.append(&variant_frame);

        // Theme Colors section - 2x2 grid layout
        let colors_frame = gtk4::Frame::new(Some("Theme Colors"));
        let colors_grid = gtk4::Grid::new();
        colors_grid.set_row_spacing(6);
        colors_grid.set_column_spacing(8);
        colors_grid.set_margin_start(8);
        colors_grid.set_margin_end(8);
        colors_grid.set_margin_top(8);
        colors_grid.set_margin_bottom(8);

        let mut color_widgets: Vec<Rc<ColorButtonWidget>> = Vec::new();

        // Color 1 (Primary) - row 0, col 0-1
        let color1_label = Label::new(Some("C1 (Primary):"));
        color1_label.set_halign(gtk4::Align::End);
        color1_label.set_width_chars(14);
        colors_grid.attach(&color1_label, 0, 0, 1, 1);
        let color1_widget = Rc::new(ColorButtonWidget::new(config.borrow().frame.theme.color1));
        colors_grid.attach(color1_widget.widget(), 1, 0, 1, 1);

        // Color 2 (Secondary) - row 0, col 2-3
        let color2_label = Label::new(Some("C2 (Secondary):"));
        color2_label.set_halign(gtk4::Align::End);
        color2_label.set_width_chars(14);
        color2_label.set_margin_start(12);
        colors_grid.attach(&color2_label, 2, 0, 1, 1);
        let color2_widget = Rc::new(ColorButtonWidget::new(config.borrow().frame.theme.color2));
        colors_grid.attach(color2_widget.widget(), 3, 0, 1, 1);

        // Color 3 (Accent) - row 1, col 0-1
        let color3_label = Label::new(Some("C3 (Accent):"));
        color3_label.set_halign(gtk4::Align::End);
        color3_label.set_width_chars(14);
        colors_grid.attach(&color3_label, 0, 1, 1, 1);
        let color3_widget = Rc::new(ColorButtonWidget::new(config.borrow().frame.theme.color3));
        colors_grid.attach(color3_widget.widget(), 1, 1, 1, 1);

        // Color 4 (Highlight) - row 1, col 2-3
        let color4_label = Label::new(Some("C4 (Highlight):"));
        color4_label.set_halign(gtk4::Align::End);
        color4_label.set_width_chars(14);
        color4_label.set_margin_start(12);
        colors_grid.attach(&color4_label, 2, 1, 1, 1);
        let color4_widget = Rc::new(ColorButtonWidget::new(config.borrow().frame.theme.color4));
        colors_grid.attach(color4_widget.widget(), 3, 1, 1, 1);

        // Connect callbacks for each color
        let config_c1 = config.clone();
        let on_change_c1 = on_change.clone();
        let preview_c1 = preview.clone();
        let refreshers_c1 = theme_ref_refreshers.clone();
        color1_widget.set_on_change(move |new_color| {
            config_c1.borrow_mut().frame.theme.color1 = new_color;
            combo_config_base::refresh_theme_refs(&refreshers_c1);
            combo_config_base::queue_redraw(&preview_c1, &on_change_c1);
        });

        let config_c2 = config.clone();
        let on_change_c2 = on_change.clone();
        let preview_c2 = preview.clone();
        let refreshers_c2 = theme_ref_refreshers.clone();
        color2_widget.set_on_change(move |new_color| {
            config_c2.borrow_mut().frame.theme.color2 = new_color;
            combo_config_base::refresh_theme_refs(&refreshers_c2);
            combo_config_base::queue_redraw(&preview_c2, &on_change_c2);
        });

        let config_c3 = config.clone();
        let on_change_c3 = on_change.clone();
        let preview_c3 = preview.clone();
        let refreshers_c3 = theme_ref_refreshers.clone();
        color3_widget.set_on_change(move |new_color| {
            config_c3.borrow_mut().frame.theme.color3 = new_color;
            combo_config_base::refresh_theme_refs(&refreshers_c3);
            combo_config_base::queue_redraw(&preview_c3, &on_change_c3);
        });

        let config_c4 = config.clone();
        let on_change_c4 = on_change.clone();
        let preview_c4 = preview.clone();
        let refreshers_c4 = theme_ref_refreshers.clone();
        color4_widget.set_on_change(move |new_color| {
            config_c4.borrow_mut().frame.theme.color4 = new_color;
            combo_config_base::refresh_theme_refs(&refreshers_c4);
            combo_config_base::queue_redraw(&preview_c4, &on_change_c4);
        });

        color_widgets.push(color1_widget);
        color_widgets.push(color2_widget);
        color_widgets.push(color3_widget);
        color_widgets.push(color4_widget);

        colors_frame.set_child(Some(&colors_grid));
        inner_box.append(&colors_frame);

        // Theme Gradient section
        let gradient_frame = gtk4::Frame::new(Some("Theme Gradient"));
        let gradient_box = GtkBox::new(Orientation::Vertical, 6);
        gradient_box.set_margin_start(8);
        gradient_box.set_margin_end(8);
        gradient_box.set_margin_top(8);
        gradient_box.set_margin_bottom(8);

        let gradient_editor = Rc::new(GradientEditor::new());
        gradient_editor.set_gradient_source_config(&config.borrow().frame.theme.gradient);
        gradient_editor.set_theme_config(config.borrow().frame.theme.clone());

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let refreshers_clone = theme_ref_refreshers.clone();
        let gradient_editor_for_cb = gradient_editor.clone();
        gradient_editor.set_on_change(move || {
            let gradient_config = gradient_editor_for_cb.get_gradient_source_config();
            config_clone.borrow_mut().frame.theme.gradient = gradient_config;
            combo_config_base::refresh_theme_refs(&refreshers_clone);
            combo_config_base::queue_redraw(&preview_clone, &on_change_clone);
        });

        // Register theme refresh callback for gradient editor
        let gradient_editor_for_refresh = gradient_editor.clone();
        let config_for_gradient_refresh = config.clone();
        theme_ref_refreshers.borrow_mut().push(Rc::new(move || {
            let cfg = config_for_gradient_refresh.borrow();
            gradient_editor_for_refresh.set_theme_config(cfg.frame.theme.clone());
        }));

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
                                .unwrap_or_else(|| "Roboto".to_string());
                            config_c.borrow_mut().frame.theme.font1_family = family.clone();
                            btn_c.set_label(&family);
                            combo_config_base::refresh_theme_refs(&refreshers_c);
                            combo_config_base::queue_redraw(&preview_c, &on_change_c);
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
            combo_config_base::refresh_theme_refs(&refreshers_for_font1_size);
            combo_config_base::queue_redraw(&preview_for_font1_size, &on_change_for_font1_size);
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
                                .unwrap_or_else(|| "Roboto".to_string());
                            config_c.borrow_mut().frame.theme.font2_family = family.clone();
                            btn_c.set_label(&family);
                            combo_config_base::refresh_theme_refs(&refreshers_c);
                            combo_config_base::queue_redraw(&preview_c, &on_change_c);
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
            combo_config_base::refresh_theme_refs(&refreshers_for_font2_size);
            combo_config_base::queue_redraw(&preview_for_font2_size, &on_change_for_font2_size);
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
            "Material (Default)",
            "Synthwave",
            "LCARS",
            "Industrial",
            "Cyberpunk",
            "Retro Terminal",
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
                0 => ComboThemeConfig::default_for_material(),
                1 => ComboThemeConfig::default_for_synthwave(),
                2 => ComboThemeConfig::default_for_lcars(),
                3 => ComboThemeConfig::default_for_industrial(),
                4 => ComboThemeConfig::default_for_cyberpunk(),
                5 => ComboThemeConfig::default_for_retro_terminal(),
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
            combo_config_base::refresh_theme_refs(&refreshers_for_preset);
            combo_config_base::queue_redraw(&preview_for_preset, &on_change_for_preset);
        });
        preset_row.append(&preset_dropdown);
        preset_box.append(&preset_row);
        preset_frame.set_child(Some(&preset_box));
        inner_box.append(&preset_frame);

        // Surface Colors section (moved from Appearance tab)
        let surface_frame = gtk4::Frame::new(Some("Surface Colors (Card Background)"));
        let surface_box = GtkBox::new(Orientation::Vertical, 6);
        surface_box.set_margin_start(8);
        surface_box.set_margin_end(8);
        surface_box.set_margin_top(8);
        surface_box.set_margin_bottom(8);

        let surface_light_row = GtkBox::new(Orientation::Horizontal, 8);
        surface_light_row.append(&Label::new(Some("Light Theme:")));
        let surface_light_widget = Rc::new(ColorButtonWidget::new(config.borrow().frame.surface_color_light));
        surface_light_row.append(surface_light_widget.widget());
        surface_light_widget.widget().set_hexpand(true);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        surface_light_widget.set_on_change(move |color| {
            config_clone.borrow_mut().frame.surface_color_light = color;
            combo_config_base::queue_redraw(&preview_clone, &on_change_clone);
        });
        surface_box.append(&surface_light_row);

        let surface_dark_row = GtkBox::new(Orientation::Horizontal, 8);
        surface_dark_row.append(&Label::new(Some("Dark Theme:")));
        let surface_dark_widget = Rc::new(ColorButtonWidget::new(config.borrow().frame.surface_color_dark));
        surface_dark_row.append(surface_dark_widget.widget());
        surface_dark_widget.widget().set_hexpand(true);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        surface_dark_widget.set_on_change(move |color| {
            config_clone.borrow_mut().frame.surface_color_dark = color;
            combo_config_base::queue_redraw(&preview_clone, &on_change_clone);
        });
        surface_box.append(&surface_dark_row);

        surface_frame.set_child(Some(&surface_box));
        inner_box.append(&surface_frame);

        // Background Colors section (moved from Appearance tab)
        let bg_frame = gtk4::Frame::new(Some("Background Colors"));
        let bg_box = GtkBox::new(Orientation::Vertical, 6);
        bg_box.set_margin_start(8);
        bg_box.set_margin_end(8);
        bg_box.set_margin_top(8);
        bg_box.set_margin_bottom(8);

        let bg_light_row = GtkBox::new(Orientation::Horizontal, 8);
        bg_light_row.append(&Label::new(Some("Light Theme:")));
        let bg_light_widget = Rc::new(ColorButtonWidget::new(config.borrow().frame.background_color_light));
        bg_light_row.append(bg_light_widget.widget());
        bg_light_widget.widget().set_hexpand(true);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        bg_light_widget.set_on_change(move |color| {
            config_clone.borrow_mut().frame.background_color_light = color;
            combo_config_base::queue_redraw(&preview_clone, &on_change_clone);
        });
        bg_box.append(&bg_light_row);

        let bg_dark_row = GtkBox::new(Orientation::Horizontal, 8);
        bg_dark_row.append(&Label::new(Some("Dark Theme:")));
        let bg_dark_widget = Rc::new(ColorButtonWidget::new(config.borrow().frame.background_color_dark));
        bg_dark_row.append(bg_dark_widget.widget());
        bg_dark_widget.widget().set_hexpand(true);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        bg_dark_widget.set_on_change(move |color| {
            config_clone.borrow_mut().frame.background_color_dark = color;
            combo_config_base::queue_redraw(&preview_clone, &on_change_clone);
        });
        bg_box.append(&bg_dark_row);

        bg_frame.set_child(Some(&bg_box));
        inner_box.append(&bg_frame);

        // Connect theme variant dropdown - sets theme defaults when changed
        let config_for_variant = config.clone();
        let on_change_for_variant = on_change.clone();
        let preview_for_variant = preview.clone();
        let refreshers_for_variant = theme_ref_refreshers.clone();
        let color_widgets_for_variant = color_widgets.clone();
        let gradient_editor_for_variant = gradient_editor.clone();
        let font1_btn_for_variant = font1_btn.clone();
        let font1_size_spin_for_variant = font1_size_spin.clone();
        let font2_btn_for_variant = font2_btn.clone();
        let font2_size_spin_for_variant = font2_size_spin.clone();
        theme_variant_dropdown.connect_selected_notify(move |dropdown| {
            let selected = dropdown.selected();
            if selected == gtk4::INVALID_LIST_POSITION {
                return;
            }
            // Set theme variant and default colors
            let (variant, theme) = match selected {
                0 => (ThemeVariant::Light, ComboThemeConfig::default_for_material_light()),
                _ => (ThemeVariant::Dark, ComboThemeConfig::default_for_material_dark()),
            };
            {
                let mut cfg = config_for_variant.borrow_mut();
                cfg.frame.theme_variant = variant;
                cfg.frame.theme = theme.clone();
            }
            // Update theme color widgets
            if color_widgets_for_variant.len() >= 4 {
                color_widgets_for_variant[0].set_color(theme.color1);
                color_widgets_for_variant[1].set_color(theme.color2);
                color_widgets_for_variant[2].set_color(theme.color3);
                color_widgets_for_variant[3].set_color(theme.color4);
            }
            gradient_editor_for_variant.set_gradient_source_config(&theme.gradient);
            gradient_editor_for_variant.set_theme_config(theme.clone());
            font1_btn_for_variant.set_label(&theme.font1_family);
            font1_size_spin_for_variant.set_value(theme.font1_size);
            font2_btn_for_variant.set_label(&theme.font2_family);
            font2_size_spin_for_variant.set_value(theme.font2_size);
            combo_config_base::refresh_theme_refs(&refreshers_for_variant);
            combo_config_base::queue_redraw(&preview_for_variant, &on_change_for_variant);
        });

        scroll.set_child(Some(&inner_box));
        page.append(&scroll);

        // Store theme widgets for later updates
        *theme_widgets_out.borrow_mut() = Some(ThemeWidgets {
            theme_variant_dropdown: theme_variant_dropdown.clone(),
            theme_color1_widget: color_widgets[0].clone(),
            theme_color2_widget: color_widgets[1].clone(),
            theme_color3_widget: color_widgets[2].clone(),
            theme_color4_widget: color_widgets[3].clone(),
            theme_gradient_editor: gradient_editor,
            font1_btn: font1_btn.clone(),
            font1_size_spin: font1_size_spin.clone(),
            font2_btn: font2_btn.clone(),
            font2_size_spin: font2_size_spin.clone(),
            surface_light_widget,
            surface_dark_widget,
            bg_light_widget,
            bg_dark_widget,
        });

        page
    }

    /// Create a theme reference section showing current theme colors and fonts with copy buttons
    #[allow(dead_code)]
    fn create_theme_reference_section(
        config: &Rc<RefCell<MaterialDisplayConfig>>,
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

    fn create_card_page(
        config: &Rc<RefCell<MaterialDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        card_widgets_out: &Rc<RefCell<Option<CardWidgets>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        combo_config_base::set_page_margins(&page);

        let builder = ConfigWidgetBuilder::new(config, preview, on_change);

        // Elevation dropdown
        let elevation_idx = match config.borrow().frame.elevation {
            CardElevation::Flat => 0,
            CardElevation::Low => 1,
            CardElevation::Medium => 2,
            CardElevation::High => 3,
        };
        let elevation_dropdown = builder.dropdown_row(
            &page,
            "Card Elevation:",
            &["Flat", "Low", "Medium", "High"],
            elevation_idx,
            |cfg, idx| {
                cfg.frame.elevation = match idx {
                    0 => CardElevation::Flat,
                    1 => CardElevation::Low,
                    2 => CardElevation::Medium,
                    _ => CardElevation::High,
                };
            },
        );

        // Corner radius
        let corner_radius_spin = builder.spin_row(
            &page, "Corner Radius:", 0.0, 32.0, 2.0,
            config.borrow().frame.corner_radius,
            |cfg, val| cfg.frame.corner_radius = val,
        );

        // Card padding
        let card_padding_spin = builder.spin_row(
            &page, "Card Padding:", 4.0, 48.0, 2.0,
            config.borrow().frame.card_padding,
            |cfg, val| cfg.frame.card_padding = val,
        );

        // Shadow section header
        let shadow_label = create_section_header("Shadow Settings");
        shadow_label.set_margin_top(12);
        page.append(&shadow_label);

        // Shadow blur
        let shadow_blur_spin = builder.spin_row(
            &page, "Shadow Blur:", 0.0, 32.0, 2.0,
            config.borrow().frame.shadow_blur,
            |cfg, val| cfg.frame.shadow_blur = val,
        );

        // Shadow offset
        let shadow_offset_spin = builder.spin_row(
            &page, "Shadow Offset Y:", 0.0, 16.0, 1.0,
            config.borrow().frame.shadow_offset_y,
            |cfg, val| cfg.frame.shadow_offset_y = val,
        );

        // Shadow color
        let shadow_color_widget = builder.color_row(
            &page, "Shadow Color:",
            config.borrow().frame.shadow_color,
            |cfg, color| cfg.frame.shadow_color = color,
        );

        // Store widget refs
        *card_widgets_out.borrow_mut() = Some(CardWidgets {
            elevation_dropdown,
            corner_radius_spin,
            card_padding_spin,
            shadow_blur_spin,
            shadow_offset_spin,
            shadow_color_widget,
        });

        page
    }

    fn create_header_page(
        config: &Rc<RefCell<MaterialDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        header_widgets_out: &Rc<RefCell<Option<HeaderWidgets>>>,
        theme_ref_refreshers: &Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        combo_config_base::set_page_margins(&page);

        let builder = ConfigWidgetBuilder::new(config, preview, on_change);

        // Show header checkbox
        let show_header_check = builder.check_button(
            &page, "Show Header",
            config.borrow().frame.show_header,
            |cfg, active| cfg.frame.show_header = active,
        );

        // Header text entry
        let header_text_entry = builder.entry_row(
            &page, "Header Text:",
            &config.borrow().frame.header_text,
            |cfg, text| cfg.frame.header_text = text,
        );

        // Header style dropdown
        let style_idx = match config.borrow().frame.header_style {
            HeaderStyle::ColorBar => 0,
            HeaderStyle::Filled => 1,
            HeaderStyle::TextOnly => 2,
            HeaderStyle::None => 3,
        };
        let header_style_dropdown = builder.dropdown_row(
            &page, "Style:",
            &["Color Bar", "Filled", "Text Only", "None"],
            style_idx,
            |cfg, idx| {
                cfg.frame.header_style = match idx {
                    0 => HeaderStyle::ColorBar,
                    1 => HeaderStyle::Filled,
                    2 => HeaderStyle::TextOnly,
                    _ => HeaderStyle::None,
                };
            },
        );

        // Header alignment dropdown
        let align_idx = match config.borrow().frame.header_alignment {
            HeaderAlignment::Left => 0,
            HeaderAlignment::Center => 1,
            HeaderAlignment::Right => 2,
        };
        let header_alignment_dropdown = builder.dropdown_row(
            &page, "Alignment:",
            &["Left", "Center", "Right"],
            align_idx,
            |cfg, idx| {
                cfg.frame.header_alignment = match idx {
                    0 => HeaderAlignment::Left,
                    1 => HeaderAlignment::Center,
                    _ => HeaderAlignment::Right,
                };
            },
        );

        // Header color (theme-aware)
        let header_color_selector = builder.theme_color_selector_row(
            &page, "Header Color:",
            config.borrow().frame.accent_color.clone(),
            config.borrow().frame.theme.clone(),
            |cfg, source| cfg.frame.accent_color = source,
        );

        // Register theme refresh callback for header color
        let header_color_for_refresh = header_color_selector.clone();
        let config_for_color_refresh = config.clone();
        theme_ref_refreshers.borrow_mut().push(Rc::new(move || {
            header_color_for_refresh.set_theme_config(config_for_color_refresh.borrow().frame.theme.clone());
        }));

        // Header height
        let header_height_spin = builder.spin_row(
            &page, "Header Height:", 24.0, 72.0, 4.0,
            config.borrow().frame.header_height,
            |cfg, val| cfg.frame.header_height = val,
        );

        // Header font selector (theme-aware)
        let header_font_selector = builder.theme_font_selector_row(
            &page, "Header Font:",
            config.borrow().frame.header_font.clone(),
            config.borrow().frame.theme.clone(),
            |cfg, source| cfg.frame.header_font = source,
        );

        // Register theme refresh callback for the font selector
        let selector_for_refresh = header_font_selector.clone();
        let config_for_refresh = config.clone();
        theme_ref_refreshers.borrow_mut().push(Rc::new(move || {
            selector_for_refresh.set_theme_config(config_for_refresh.borrow().frame.theme.clone());
        }));

        // Store widget refs
        *header_widgets_out.borrow_mut() = Some(HeaderWidgets {
            show_header_check,
            header_text_entry,
            header_style_dropdown,
            header_alignment_dropdown,
            header_color_selector,
            header_height_spin,
            header_font_selector,
        });

        page
    }

    fn create_layout_page(
        config: &Rc<RefCell<MaterialDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        layout_widgets_out: &Rc<RefCell<Option<LayoutWidgets>>>,
        theme_ref_refreshers: &Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        combo_config_base::set_page_margins(&page);

        let builder = ConfigWidgetBuilder::new(config, preview, on_change);

        // Split direction dropdown
        let orient_idx = match config.borrow().frame.split_orientation {
            SplitOrientation::Vertical => 0,
            SplitOrientation::Horizontal => 1,
        };
        let orientation_dropdown = builder.dropdown_row(
            &page, "Split Direction:",
            &["Vertical", "Horizontal"],
            orient_idx,
            |cfg, idx| {
                cfg.frame.split_orientation = match idx {
                    0 => SplitOrientation::Vertical,
                    _ => SplitOrientation::Horizontal,
                };
            },
        );

        // Content padding
        let content_padding_spin = builder.spin_row(
            &page, "Content Padding:", 8.0, 48.0, 4.0,
            config.borrow().frame.content_padding,
            |cfg, val| cfg.frame.content_padding = val,
        );

        // Item spacing
        let item_spacing_spin = builder.spin_row(
            &page, "Item Spacing:", 4.0, 32.0, 2.0,
            config.borrow().frame.item_spacing,
            |cfg, val| cfg.frame.item_spacing = val,
        );

        // Dividers section header
        let dividers_label = create_section_header("Dividers");
        dividers_label.set_margin_top(12);
        page.append(&dividers_label);

        // Divider style dropdown
        let div_style_idx = match config.borrow().frame.divider_style {
            DividerStyle::Space => 0,
            DividerStyle::Line => 1,
            DividerStyle::Fade => 2,
        };
        let divider_style_dropdown = builder.dropdown_row(
            &page, "Style:",
            &["Space", "Line", "Fade"],
            div_style_idx,
            |cfg, idx| {
                cfg.frame.divider_style = match idx {
                    0 => DividerStyle::Space,
                    1 => DividerStyle::Line,
                    _ => DividerStyle::Fade,
                };
            },
        );

        // Divider spacing
        let divider_spacing_spin = builder.spin_row(
            &page, "Spacing:", 8.0, 48.0, 4.0,
            config.borrow().frame.divider_spacing,
            |cfg, val| cfg.frame.divider_spacing = val,
        );

        // Divider color (theme-aware)
        let divider_color_widget = builder.theme_color_selector_row(
            &page, "Color:",
            config.borrow().frame.divider_color.clone(),
            config.borrow().frame.theme.clone(),
            |cfg, source| cfg.frame.divider_color = source,
        );

        // Register theme refresh callback
        let divider_color_for_refresh = divider_color_widget.clone();
        let config_for_refresh = config.clone();
        theme_ref_refreshers.borrow_mut().push(Rc::new(move || {
            divider_color_for_refresh.set_theme_config(config_for_refresh.borrow().frame.theme.clone());
        }));

        // Group weights section
        let weights_label = create_section_header("Group Size Weights");
        weights_label.set_margin_top(12);
        page.append(&weights_label);

        let group_weights_box = GtkBox::new(Orientation::Vertical, 4);
        page.append(&group_weights_box);

        Self::rebuild_group_spinners(config, on_change, preview, &group_weights_box);

        // Item Orientations section
        let item_orient_label = create_section_header("Item Orientation per Group");
        item_orient_label.set_margin_top(12);
        page.append(&item_orient_label);

        let item_orient_info = Label::new(Some("Choose how items within each group are arranged"));
        item_orient_info.set_halign(gtk4::Align::Start);
        item_orient_info.add_css_class("dim-label");
        page.append(&item_orient_info);

        let item_orientations_box = GtkBox::new(Orientation::Vertical, 4);
        item_orientations_box.set_margin_top(4);
        combo_config_base::rebuild_item_orientation_dropdowns(
            &item_orientations_box, config,
            |c: &mut MaterialDisplayConfig| &mut c.frame,
            on_change, preview,
        );
        page.append(&item_orientations_box);

        // Store widget refs
        *layout_widgets_out.borrow_mut() = Some(LayoutWidgets {
            orientation_dropdown,
            content_padding_spin,
            item_spacing_spin,
            divider_style_dropdown,
            divider_spacing_spin,
            divider_color_widget,
            group_weights_box,
            item_orientations_box,
        });

        page
    }

    fn rebuild_group_spinners(
        config: &Rc<RefCell<MaterialDisplayConfig>>,
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
                combo_config_base::queue_redraw(&preview_clone, &on_change_clone);
            });

            weights_box.append(&row);
        }
    }

    fn create_content_page(
        config: &Rc<RefCell<MaterialDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        content_notebook: &Rc<RefCell<Notebook>>,
        source_summaries: &Rc<RefCell<Vec<(String, String, usize, u32)>>>,
        available_fields: &Rc<RefCell<Vec<FieldMetadata>>>,
        theme_ref_refreshers: &Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        combo_config_base::set_page_margins(&page);

        let info_label = Label::new(Some("Configure content items for each slot.\nSlots are named: group1_1, group1_2, group2_1, etc."));
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
        config: &Rc<RefCell<MaterialDisplayConfig>>,
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

    /// Default bar config with Material Design colors
    #[allow(dead_code, clippy::field_reassign_with_default)]
    fn default_bar_config_material() -> crate::ui::BarDisplayConfig {
        use crate::ui::bar_display::{BarDisplayConfig, BarStyle, BarOrientation, BarFillDirection, BarFillType, BarBackgroundType, BorderConfig};
        use crate::ui::background::Color;

        let mut config = BarDisplayConfig::default();
        config.style = BarStyle::Full;
        config.orientation = BarOrientation::Horizontal;
        config.fill_direction = BarFillDirection::LeftToRight;

        // Material Blue 500
        config.foreground = BarFillType::Solid {
            color: crate::ui::theme::ColorSource::custom(Color { r: 0.24, g: 0.47, b: 0.96, a: 1.0 })
        };
        config.background = BarBackgroundType::Solid {
            color: crate::ui::theme::ColorSource::custom(Color { r: 0.9, g: 0.9, b: 0.9, a: 1.0 })
        };
        config.border = BorderConfig {
            enabled: false,
            color: crate::ui::theme::ColorSource::custom(Color { r: 0.8, g: 0.8, b: 0.8, a: 1.0 }),
            width: 1.0,
        };
        config.corner_radius = 4.0;

        config
    }

    /// Default graph config with Material Design colors
    #[allow(dead_code, clippy::field_reassign_with_default)]
    fn default_graph_config_material() -> crate::ui::GraphDisplayConfig {
        use crate::ui::graph_display::{GraphDisplayConfig, GraphType, LineStyle, FillMode};
        use crate::ui::background::Color;

        let mut config = GraphDisplayConfig::default();
        config.graph_type = GraphType::Line;
        config.line_style = LineStyle::Solid;
        config.line_width = 2.0;
        config.line_color = ColorSource::custom(Color { r: 0.24, g: 0.47, b: 0.96, a: 1.0 });  // Material Blue
        config.fill_mode = FillMode::Gradient;
        config.fill_gradient_start = ColorSource::custom(Color { r: 0.24, g: 0.47, b: 0.96, a: 0.3 });
        config.fill_gradient_end = ColorSource::custom(Color { r: 0.24, g: 0.47, b: 0.96, a: 0.0 });
        config.background_color = Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
        config.plot_background_color = Color { r: 0.98, g: 0.98, b: 0.98, a: 1.0 };
        config.x_axis.show_grid = true;
        config.x_axis.grid_color = ColorSource::custom(Color { r: 0.9, g: 0.9, b: 0.9, a: 1.0 });
        config.y_axis.show_grid = true;
        config.y_axis.grid_color = ColorSource::custom(Color { r: 0.9, g: 0.9, b: 0.9, a: 1.0 });

        config
    }

    fn create_animation_page(
        config: &Rc<RefCell<MaterialDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        animation_widgets_out: &Rc<RefCell<Option<AnimationWidgets>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        combo_config_base::set_page_margins(&page);

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

        // Animation speed
        let speed_box = GtkBox::new(Orientation::Horizontal, 6);
        speed_box.append(&Label::new(Some("Animation Speed:")));

        let speed_scale = Scale::with_range(Orientation::Horizontal, 1.0, 20.0, 0.5);
        speed_scale.set_value(config.borrow().animation_speed);
        speed_scale.set_hexpand(true);
        speed_scale.set_draw_value(true);
        speed_box.append(&speed_scale);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        speed_scale.connect_value_changed(move |scale| {
            config_clone.borrow_mut().animation_speed = scale.value();
            if let Some(cb) = on_change_clone.borrow().as_ref() {
                cb();
            }
        });
        page.append(&speed_box);

        // Store widget refs
        *animation_widgets_out.borrow_mut() = Some(AnimationWidgets {
            enable_check,
            speed_scale,
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

    pub fn get_config(&self) -> MaterialDisplayConfig {
        self.config.borrow().clone()
    }

    /// Get a reference to the internal config Rc for use in callbacks
    pub fn get_config_rc(&self) -> Rc<RefCell<MaterialDisplayConfig>> {
        self.config.clone()
    }

    pub fn set_config(&self, config: &MaterialDisplayConfig) {
        // IMPORTANT: Temporarily disable on_change callback to prevent signal cascade.
        // When we call set_value() on widgets, their signal handlers fire and call on_change.
        // This causes redundant updates since we're setting the config directly anyway.
        let saved_callback = self.on_change.borrow_mut().take();

        *self.config.borrow_mut() = config.clone();

        // Update Theme widgets (theme variant, combo theme, surface/background colors)
        if let Some(widgets) = self.theme_widgets.borrow().as_ref() {
            // Theme variant dropdown
            widgets.theme_variant_dropdown.set_selected(match config.frame.theme_variant {
                ThemeVariant::Light => 0,
                ThemeVariant::Dark => 1,
            });
            // Theme colors
            widgets.theme_color1_widget.set_color(config.frame.theme.color1);
            widgets.theme_color2_widget.set_color(config.frame.theme.color2);
            widgets.theme_color3_widget.set_color(config.frame.theme.color3);
            widgets.theme_color4_widget.set_color(config.frame.theme.color4);
            widgets.theme_gradient_editor.set_gradient_source_config(&config.frame.theme.gradient);
            widgets.theme_gradient_editor.set_theme_config(config.frame.theme.clone());
            widgets.font1_btn.set_label(&config.frame.theme.font1_family);
            widgets.font1_size_spin.set_value(config.frame.theme.font1_size);
            widgets.font2_btn.set_label(&config.frame.theme.font2_family);
            widgets.font2_size_spin.set_value(config.frame.theme.font2_size);
            // Surface and background colors
            widgets.surface_light_widget.set_color(config.frame.surface_color_light);
            widgets.surface_dark_widget.set_color(config.frame.surface_color_dark);
            widgets.bg_light_widget.set_color(config.frame.background_color_light);
            widgets.bg_dark_widget.set_color(config.frame.background_color_dark);
        }

        // Update Card widgets
        if let Some(widgets) = self.card_widgets.borrow().as_ref() {
            widgets.elevation_dropdown.set_selected(match config.frame.elevation {
                CardElevation::Flat => 0,
                CardElevation::Low => 1,
                CardElevation::Medium => 2,
                CardElevation::High => 3,
            });
            widgets.corner_radius_spin.set_value(config.frame.corner_radius);
            widgets.card_padding_spin.set_value(config.frame.card_padding);
            widgets.shadow_blur_spin.set_value(config.frame.shadow_blur);
            widgets.shadow_offset_spin.set_value(config.frame.shadow_offset_y);
            widgets.shadow_color_widget.set_color(config.frame.shadow_color);
        }

        // Update Header widgets
        if let Some(widgets) = self.header_widgets.borrow().as_ref() {
            widgets.show_header_check.set_active(config.frame.show_header);
            widgets.header_text_entry.set_text(&config.frame.header_text);
            widgets.header_style_dropdown.set_selected(match config.frame.header_style {
                HeaderStyle::ColorBar => 0,
                HeaderStyle::Filled => 1,
                HeaderStyle::TextOnly => 2,
                HeaderStyle::None => 3,
            });
            widgets.header_alignment_dropdown.set_selected(match config.frame.header_alignment {
                HeaderAlignment::Left => 0,
                HeaderAlignment::Center => 1,
                HeaderAlignment::Right => 2,
            });
            widgets.header_color_selector.set_source(config.frame.accent_color.clone());
            widgets.header_height_spin.set_value(config.frame.header_height);
            widgets.header_font_selector.set_source(config.frame.header_font.clone());
        }

        // Update Layout widgets
        if let Some(widgets) = self.layout_widgets.borrow().as_ref() {
            widgets.orientation_dropdown.set_selected(match config.frame.split_orientation {
                SplitOrientation::Vertical => 0,
                SplitOrientation::Horizontal => 1,
            });
            widgets.content_padding_spin.set_value(config.frame.content_padding);
            widgets.item_spacing_spin.set_value(config.frame.item_spacing);
            widgets.divider_style_dropdown.set_selected(match config.frame.divider_style {
                DividerStyle::Space => 0,
                DividerStyle::Line => 1,
                DividerStyle::Fade => 2,
            });
            widgets.divider_spacing_spin.set_value(config.frame.divider_spacing);
            widgets.divider_color_widget.set_source(config.frame.divider_color.clone());

            Self::rebuild_group_spinners(
                &self.config,
                &self.on_change,
                &self.preview,
                &widgets.group_weights_box,
            );
            combo_config_base::rebuild_item_orientation_dropdowns(
                &widgets.item_orientations_box,
                &self.config,
                |c: &mut MaterialDisplayConfig| &mut c.frame,
                &self.on_change,
                &self.preview,
            );
        }

        // Update Animation widgets
        if let Some(widgets) = self.animation_widgets.borrow().as_ref() {
            widgets.enable_check.set_active(config.animation_enabled);
            widgets.speed_scale.set_value(config.animation_speed);
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
                |c: &mut MaterialDisplayConfig| &mut c.frame,
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

    pub fn set_available_fields(&self, fields: Vec<FieldMetadata>) {
        *self.available_fields.borrow_mut() = fields;
    }

    /// Extract transferable configuration (layout, content items, animation settings).
    pub fn get_transferable_config(&self) -> crate::ui::combo_config_base::TransferableComboConfig {
        let config = self.config.borrow();
        crate::ui::combo_config_base::TransferableComboConfig {
            group_count: config.frame.group_count,
            group_item_counts: config.frame.group_item_counts.iter().map(|&x| x as u32).collect(),
            group_size_weights: config.frame.group_size_weights.clone(),
            group_item_orientations: config.frame.group_item_orientations.clone(),
            layout_orientation: config.frame.split_orientation.clone(),
            content_items: config.frame.content_items.clone(),
            content_padding: config.frame.content_padding,
            item_spacing: config.frame.item_spacing,
            animation_enabled: config.animation_enabled,
            animation_speed: config.animation_speed,
        }
    }

    /// Apply transferable configuration from another combo panel.
    pub fn apply_transferable_config(&self, transfer: &crate::ui::combo_config_base::TransferableComboConfig) {
        {
            let mut config = self.config.borrow_mut();
            config.frame.group_count = transfer.group_count;
            config.frame.group_item_counts = transfer.group_item_counts.iter().map(|&x| x as usize).collect();
            config.frame.group_size_weights = transfer.group_size_weights.clone();
            config.frame.group_item_orientations = transfer.group_item_orientations.clone();
            config.frame.split_orientation = transfer.layout_orientation.clone();
            config.frame.content_items = transfer.content_items.clone();
            config.frame.content_padding = transfer.content_padding;
            config.frame.item_spacing = transfer.item_spacing;
            config.animation_enabled = transfer.animation_enabled;
            config.animation_speed = transfer.animation_speed;
        }
        self.preview.queue_draw();
    }
}

impl Default for MaterialConfigWidget {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}
