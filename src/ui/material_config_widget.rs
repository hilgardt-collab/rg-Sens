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
    render_material_frame, CardElevation, HeaderStyle, DividerStyle, ThemeVariant,
};
use crate::ui::graph_config_widget::GraphConfigWidget;
use crate::ui::bar_config_widget::BarConfigWidget;
use crate::ui::core_bars_config_widget::CoreBarsConfigWidget;
use crate::ui::background_config_widget::BackgroundConfigWidget;
use crate::ui::text_line_config_widget::TextLineConfigWidget;
use crate::ui::arc_config_widget::ArcConfigWidget;
use crate::ui::speedometer_config_widget::SpeedometerConfigWidget;
use crate::ui::lcars_display::{ContentDisplayType, ContentItemConfig, SplitOrientation, StaticDisplayConfig};
use crate::displayers::MaterialDisplayConfig;
use crate::core::{FieldMetadata, FieldType, FieldPurpose};
use crate::ui::combo_config_base;
use crate::ui::theme::{ColorSource, ComboThemeConfig, FontSource};
use crate::ui::theme_font_selector::ThemeFontSelector;
use crate::ui::theme_color_selector::ThemeColorSelector;

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

    fn set_page_margins(page: &GtkBox) {
        combo_config_base::set_page_margins(page);
    }

    fn queue_redraw(
        preview: &DrawingArea,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
    ) {
        combo_config_base::queue_redraw(preview, on_change);
    }

    /// Helper function to refresh all theme reference sections
    fn refresh_theme_refs(refreshers: &Rc<RefCell<Vec<Rc<dyn Fn()>>>>) {
        combo_config_base::refresh_theme_refs(refreshers);
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
        Self::set_page_margins(&page);

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
            Self::refresh_theme_refs(&refreshers_clone);
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });

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
                                .unwrap_or_else(|| "Roboto".to_string());
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
            Self::refresh_theme_refs(&refreshers_for_preset);
            Self::queue_redraw(&preview_for_preset, &on_change_for_preset);
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
            Self::queue_redraw(&preview_clone, &on_change_clone);
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
            Self::queue_redraw(&preview_clone, &on_change_clone);
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
            Self::queue_redraw(&preview_clone, &on_change_clone);
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
            Self::queue_redraw(&preview_clone, &on_change_clone);
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
            Self::refresh_theme_refs(&refreshers_for_variant);
            Self::queue_redraw(&preview_for_variant, &on_change_for_variant);
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
        Self::set_page_margins(&page);

        // Elevation
        let elevation_box = GtkBox::new(Orientation::Horizontal, 6);
        elevation_box.append(&Label::new(Some("Card Elevation:")));
        let elevation_list = StringList::new(&["Flat", "Low", "Medium", "High"]);
        let elevation_dropdown = DropDown::new(Some(elevation_list), None::<gtk4::Expression>);
        let elevation_idx = match config.borrow().frame.elevation {
            CardElevation::Flat => 0,
            CardElevation::Low => 1,
            CardElevation::Medium => 2,
            CardElevation::High => 3,
        };
        elevation_dropdown.set_selected(elevation_idx);
        elevation_dropdown.set_hexpand(true);
        elevation_box.append(&elevation_dropdown);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        elevation_dropdown.connect_selected_notify(move |dropdown| {
            let selected = dropdown.selected();
            if selected == gtk4::INVALID_LIST_POSITION {
                return;
            }
            config_clone.borrow_mut().frame.elevation = match selected {
                0 => CardElevation::Flat,
                1 => CardElevation::Low,
                2 => CardElevation::Medium,
                _ => CardElevation::High,
            };
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&elevation_box);

        // Corner radius
        let radius_box = GtkBox::new(Orientation::Horizontal, 6);
        radius_box.append(&Label::new(Some("Corner Radius:")));
        let corner_radius_spin = SpinButton::with_range(0.0, 32.0, 2.0);
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

        // Card padding
        let padding_box = GtkBox::new(Orientation::Horizontal, 6);
        padding_box.append(&Label::new(Some("Card Padding:")));
        let card_padding_spin = SpinButton::with_range(4.0, 48.0, 2.0);
        card_padding_spin.set_value(config.borrow().frame.card_padding);
        card_padding_spin.set_hexpand(true);
        padding_box.append(&card_padding_spin);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        card_padding_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().frame.card_padding = spin.value();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&padding_box);

        // Shadow section
        let shadow_label = Label::new(Some("Shadow Settings"));
        shadow_label.set_halign(gtk4::Align::Start);
        shadow_label.add_css_class("heading");
        shadow_label.set_margin_top(12);
        page.append(&shadow_label);

        // Shadow blur
        let blur_box = GtkBox::new(Orientation::Horizontal, 6);
        blur_box.append(&Label::new(Some("Shadow Blur:")));
        let shadow_blur_spin = SpinButton::with_range(0.0, 32.0, 2.0);
        shadow_blur_spin.set_value(config.borrow().frame.shadow_blur);
        shadow_blur_spin.set_hexpand(true);
        blur_box.append(&shadow_blur_spin);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        shadow_blur_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().frame.shadow_blur = spin.value();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&blur_box);

        // Shadow offset
        let offset_box = GtkBox::new(Orientation::Horizontal, 6);
        offset_box.append(&Label::new(Some("Shadow Offset Y:")));
        let shadow_offset_spin = SpinButton::with_range(0.0, 16.0, 1.0);
        shadow_offset_spin.set_value(config.borrow().frame.shadow_offset_y);
        shadow_offset_spin.set_hexpand(true);
        offset_box.append(&shadow_offset_spin);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        shadow_offset_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().frame.shadow_offset_y = spin.value();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&offset_box);

        // Shadow color
        let shadow_color_box = GtkBox::new(Orientation::Horizontal, 6);
        shadow_color_box.append(&Label::new(Some("Shadow Color:")));
        let shadow_color_widget = Rc::new(ColorButtonWidget::new(config.borrow().frame.shadow_color));
        shadow_color_box.append(shadow_color_widget.widget());

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        shadow_color_widget.set_on_change(move |color| {
            config_clone.borrow_mut().frame.shadow_color = color;
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&shadow_color_box);

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
        let style_list = StringList::new(&["Color Bar", "Filled", "Text Only", "None"]);
        let header_style_dropdown = DropDown::new(Some(style_list), None::<gtk4::Expression>);
        let style_idx = match config.borrow().frame.header_style {
            HeaderStyle::ColorBar => 0,
            HeaderStyle::Filled => 1,
            HeaderStyle::TextOnly => 2,
            HeaderStyle::None => 3,
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
                0 => HeaderStyle::ColorBar,
                1 => HeaderStyle::Filled,
                2 => HeaderStyle::TextOnly,
                _ => HeaderStyle::None,
            };
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&style_box);

        // Header height
        let height_box = GtkBox::new(Orientation::Horizontal, 6);
        height_box.append(&Label::new(Some("Header Height:")));
        let header_height_spin = SpinButton::with_range(24.0, 72.0, 4.0);
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
        config: &Rc<RefCell<MaterialDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        layout_widgets_out: &Rc<RefCell<Option<LayoutWidgets>>>,
        theme_ref_refreshers: &Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        Self::set_page_margins(&page);

        // Orientation
        let orient_box = GtkBox::new(Orientation::Horizontal, 6);
        orient_box.append(&Label::new(Some("Split Direction:")));
        let orient_list = StringList::new(&["Vertical", "Horizontal"]);
        let orientation_dropdown = DropDown::new(Some(orient_list), None::<gtk4::Expression>);
        let orient_idx = match config.borrow().frame.split_orientation {
            SplitOrientation::Vertical => 0,
            SplitOrientation::Horizontal => 1,
        };
        orientation_dropdown.set_selected(orient_idx);
        orientation_dropdown.set_hexpand(true);
        orient_box.append(&orientation_dropdown);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        orientation_dropdown.connect_selected_notify(move |dropdown| {
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
        let content_padding_box = GtkBox::new(Orientation::Horizontal, 6);
        content_padding_box.append(&Label::new(Some("Content Padding:")));
        let content_padding_spin = SpinButton::with_range(8.0, 48.0, 4.0);
        content_padding_spin.set_value(config.borrow().frame.content_padding);
        content_padding_spin.set_hexpand(true);
        content_padding_box.append(&content_padding_spin);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        content_padding_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().frame.content_padding = spin.value();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&content_padding_box);

        // Item spacing
        let item_spacing_box = GtkBox::new(Orientation::Horizontal, 6);
        item_spacing_box.append(&Label::new(Some("Item Spacing:")));
        let item_spacing_spin = SpinButton::with_range(4.0, 32.0, 2.0);
        item_spacing_spin.set_value(config.borrow().frame.item_spacing);
        item_spacing_spin.set_hexpand(true);
        item_spacing_box.append(&item_spacing_spin);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        item_spacing_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().frame.item_spacing = spin.value();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&item_spacing_box);

        // Dividers section
        let dividers_label = Label::new(Some("Dividers"));
        dividers_label.set_halign(gtk4::Align::Start);
        dividers_label.add_css_class("heading");
        dividers_label.set_margin_top(12);
        page.append(&dividers_label);

        // Divider style
        let div_style_box = GtkBox::new(Orientation::Horizontal, 6);
        div_style_box.append(&Label::new(Some("Style:")));
        let div_style_list = StringList::new(&["Space", "Line", "Fade"]);
        let divider_style_dropdown = DropDown::new(Some(div_style_list), None::<gtk4::Expression>);
        let div_style_idx = match config.borrow().frame.divider_style {
            DividerStyle::Space => 0,
            DividerStyle::Line => 1,
            DividerStyle::Fade => 2,
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
                0 => DividerStyle::Space,
                1 => DividerStyle::Line,
                _ => DividerStyle::Fade,
            };
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&div_style_box);

        // Divider spacing
        let div_spacing_box = GtkBox::new(Orientation::Horizontal, 6);
        div_spacing_box.append(&Label::new(Some("Spacing:")));
        let divider_spacing_spin = SpinButton::with_range(8.0, 48.0, 4.0);
        divider_spacing_spin.set_value(config.borrow().frame.divider_spacing);
        divider_spacing_spin.set_hexpand(true);
        div_spacing_box.append(&divider_spacing_spin);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        divider_spacing_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().frame.divider_spacing = spin.value();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&div_spacing_box);

        // Divider color (theme-aware)
        let div_color_box = GtkBox::new(Orientation::Horizontal, 6);
        div_color_box.append(&Label::new(Some("Color:")));
        let divider_color_widget = Rc::new(ThemeColorSelector::new(
            config.borrow().frame.divider_color.clone(),
        ));
        divider_color_widget.set_theme_config(config.borrow().frame.theme.clone());
        div_color_box.append(divider_color_widget.widget());

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        divider_color_widget.set_on_change(move |new_source| {
            config_clone.borrow_mut().frame.divider_color = new_source;
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });

        // Register theme refresh callback
        let divider_color_widget_for_refresh = divider_color_widget.clone();
        let config_for_divider_refresh = config.clone();
        theme_ref_refreshers.borrow_mut().push(Rc::new(move || {
            let cfg = config_for_divider_refresh.borrow();
            divider_color_widget_for_refresh.set_theme_config(cfg.frame.theme.clone());
        }));

        page.append(&div_color_box);

        // Group weights section
        let weights_label = Label::new(Some("Group Size Weights"));
        weights_label.set_halign(gtk4::Align::Start);
        weights_label.add_css_class("heading");
        weights_label.set_margin_top(12);
        page.append(&weights_label);

        let group_weights_box = GtkBox::new(Orientation::Vertical, 4);
        page.append(&group_weights_box);

        Self::rebuild_group_spinners(config, on_change, preview, &group_weights_box);

        // Store widget refs
        *layout_widgets_out.borrow_mut() = Some(LayoutWidgets {
            orientation_dropdown,
            content_padding_spin,
            item_spacing_spin,
            divider_style_dropdown,
            divider_spacing_spin,
            divider_color_widget,
            group_weights_box,
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
                Self::queue_redraw(&preview_clone, &on_change_clone);
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
        Self::set_page_margins(&page);

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
        let notebook = content_notebook.borrow();

        // Clear existing tabs
        while notebook.n_pages() > 0 {
            notebook.remove_page(Some(0));
        }

        let summaries = source_summaries.borrow();

        if summaries.is_empty() {
            let placeholder = GtkBox::new(Orientation::Vertical, 8);
            placeholder.set_margin_start(12);
            placeholder.set_margin_end(12);
            placeholder.set_margin_top(12);
            let label = Label::new(Some("No sources configured.\nGo to 'Data Source' tab and select 'Combination' source to configure content."));
            label.set_halign(gtk4::Align::Start);
            placeholder.append(&label);
            notebook.append_page(&placeholder, Some(&Label::new(Some("No Sources"))));
            return;
        }

        // Group summaries by group number
        let mut groups: std::collections::HashMap<usize, Vec<(String, String, u32)>> = std::collections::HashMap::new();
        for (slot_name, summary, group_num, item_idx) in summaries.iter() {
            groups.entry(*group_num)
                .or_default()
                .push((slot_name.clone(), summary.clone(), *item_idx));
        }

        let mut group_nums: Vec<usize> = groups.keys().cloned().collect();
        group_nums.sort();

        for group_num in group_nums {
            if let Some(items) = groups.get(&group_num) {
                let group_box = GtkBox::new(Orientation::Vertical, 4);
                group_box.set_margin_start(4);
                group_box.set_margin_end(4);
                group_box.set_margin_top(4);

                let items_notebook = Notebook::new();
                items_notebook.set_scrollable(true);
                items_notebook.set_vexpand(true);

                let mut sorted_items = items.clone();
                sorted_items.sort_by_key(|(_, _, idx)| *idx);

                for (slot_name, summary, item_idx) in sorted_items {
                    let tab_label = format!("Item {} : {}", item_idx, summary);
                    let tab_box = Self::create_slot_config_tab(&slot_name, config, on_change, preview, available_fields, theme_ref_refreshers);
                    items_notebook.append_page(&tab_box, Some(&Label::new(Some(&tab_label))));
                }

                group_box.append(&items_notebook);
                notebook.append_page(&group_box, Some(&Label::new(Some(&format!("Group {}", group_num)))));
            }
        }
    }

    fn create_slot_config_tab(
        slot_name: &str,
        config: &Rc<RefCell<MaterialDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        available_fields: &Rc<RefCell<Vec<FieldMetadata>>>,
        theme_ref_refreshers: &Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
    ) -> GtkBox {
        // Get available fields for this slot (needed for smart defaults)
        let slot_prefix = format!("{}_", slot_name);
        let slot_fields_for_default: Vec<FieldMetadata> = available_fields.borrow().iter()
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

        // Ensure this slot exists in content_items with smart default display type
        {
            let mut cfg = config.borrow_mut();
            if !cfg.frame.content_items.contains_key(slot_name) {
                let item = ContentItemConfig {
                    display_as: ContentDisplayType::suggest_for_fields(&slot_fields_for_default),
                    ..Default::default()
                };
                cfg.frame.content_items.insert(slot_name.to_string(), item);
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

        // Get available fields
        let slot_prefix = format!("{}_", slot_name);
        let source_fields = available_fields.borrow();
        let mut slot_fields: Vec<FieldMetadata> = source_fields.iter()
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
        drop(source_fields);

        // === Bar Configuration Section ===
        let bar_config_frame = gtk4::Frame::new(Some("Bar Configuration"));
        bar_config_frame.set_margin_top(12);

        let bar_widget = BarConfigWidget::new(slot_fields.clone());
        let current_bar_config = {
            let cfg = config.borrow();
            cfg.frame.content_items
                .get(slot_name)
                .map(|item| item.bar_config.clone())
                .unwrap_or_else(Self::default_bar_config_material)
        };
        bar_widget.set_config(current_bar_config);
        // Set initial theme
        bar_widget.set_theme(config.borrow().frame.theme.clone());

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

        // Register theme refresh callback for bar widget
        let bar_widget_for_theme = bar_widget_rc.clone();
        let config_for_bar_theme = config.clone();
        let theme_refresh_callback: Rc<dyn Fn()> = Rc::new(move || {
            let theme = config_for_bar_theme.borrow().frame.theme.clone();
            bar_widget_for_theme.set_theme(theme);
        });
        theme_ref_refreshers.borrow_mut().push(theme_refresh_callback);

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
                .unwrap_or_else(Self::default_graph_config_material)
        };
        // Set theme BEFORE config, since set_config triggers UI rebuild that needs theme
        graph_widget.set_theme(config.borrow().frame.theme.clone());
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

        // Register theme refresh callback for graph widget
        let graph_widget_for_theme = graph_widget_rc.clone();
        let config_for_graph_theme = config.clone();
        let theme_refresh_callback: Rc<dyn Fn()> = Rc::new(move || {
            let theme = config_for_graph_theme.borrow().frame.theme.clone();
            graph_widget_for_theme.set_theme(theme);
        });
        theme_ref_refreshers.borrow_mut().push(theme_refresh_callback);

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
        // Set theme BEFORE config, since set_config triggers UI rebuild that needs theme
        text_widget.set_theme(config.borrow().frame.theme.clone());
        text_widget.set_config(current_text_config);

        let slot_name_clone = slot_name.to_string();
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        // Only save when Text display mode is active to avoid overwriting BarConfigWidget's changes
        let text_widget_rc = Rc::new(text_widget);
        let text_widget_for_callback = text_widget_rc.clone();
        text_widget_rc.set_on_change(move || {
            let mut cfg = config_clone.borrow_mut();
            let item = cfg.frame.content_items
                .entry(slot_name_clone.clone())
                .or_default();
            // Only update if Text mode is active (not Bar mode which has its own text widget)
            let is_text_mode = matches!(item.display_as, ContentDisplayType::Text | ContentDisplayType::Static);
            if is_text_mode {
                let text_config = text_widget_for_callback.get_config();
                item.bar_config.text_overlay.enabled = true;
                item.bar_config.text_overlay.text_config = text_config;
            }
            drop(cfg);
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });

        text_config_frame.set_child(Some(text_widget_rc.widget()));
        inner_box.append(&text_config_frame);

        // Register theme refresh callback for text widget
        let text_widget_for_theme = text_widget_rc.clone();
        let config_for_text_theme = config.clone();
        let theme_refresh_callback: Rc<dyn Fn()> = Rc::new(move || {
            let theme = config_for_text_theme.borrow().frame.theme.clone();
            text_widget_for_theme.set_theme(theme);
        });
        theme_ref_refreshers.borrow_mut().push(theme_refresh_callback);

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
        // Set theme BEFORE config, since set_config triggers UI rebuild that needs theme
        core_bars_widget.set_theme(config.borrow().frame.theme.clone());
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

        // Register theme refresh callback for core bars widget
        let core_bars_widget_for_theme = core_bars_widget_rc.clone();
        let config_for_core_bars_theme = config.clone();
        let theme_refresh_callback: Rc<dyn Fn()> = Rc::new(move || {
            let theme = config_for_core_bars_theme.borrow().frame.theme.clone();
            core_bars_widget_for_theme.set_theme(theme);
        });
        theme_ref_refreshers.borrow_mut().push(theme_refresh_callback);

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
        // Set theme BEFORE config for gradient editor theme colors
        static_bg_widget.set_theme_config(config.borrow().frame.theme.clone());
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

        // Register theme refresh callback for static background widget
        let static_bg_widget_for_theme = static_bg_widget_rc.clone();
        let config_for_static_theme = config.clone();
        let theme_refresh_callback: Rc<dyn Fn()> = Rc::new(move || {
            let theme = config_for_static_theme.borrow().frame.theme.clone();
            static_bg_widget_for_theme.set_theme_config(theme);
        });
        theme_ref_refreshers.borrow_mut().push(theme_refresh_callback);

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
        // Set initial theme
        arc_widget.set_theme(config.borrow().frame.theme.clone());

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

        // Register theme refresh callback for arc widget
        let arc_widget_for_theme = arc_widget_rc.clone();
        let config_for_arc_theme = config.clone();
        let theme_refresh_callback: Rc<dyn Fn()> = Rc::new(move || {
            let theme = config_for_arc_theme.borrow().frame.theme.clone();
            arc_widget_for_theme.set_theme(theme);
        });
        theme_ref_refreshers.borrow_mut().push(theme_refresh_callback);

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
        // Set theme BEFORE config, since set_config triggers UI rebuild that needs theme
        speedometer_widget.set_theme(config.borrow().frame.theme.clone());
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

        // Register theme refresh callback for speedometer widget
        let speedometer_widget_for_theme = speedometer_widget_rc.clone();
        let config_for_speedometer_theme = config.clone();
        let theme_refresh_callback: Rc<dyn Fn()> = Rc::new(move || {
            let theme = config_for_speedometer_theme.borrow().frame.theme.clone();
            speedometer_widget_for_theme.set_theme(theme);
        });
        theme_ref_refreshers.borrow_mut().push(theme_refresh_callback);

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

        // Display type change handler
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
            let show_bar = matches!(display_type, ContentDisplayType::Bar | ContentDisplayType::LevelBar);
            let show_text = matches!(display_type, ContentDisplayType::Text | ContentDisplayType::Static);
            bar_config_frame_clone.set_visible(show_bar);
            text_config_frame_clone.set_visible(show_text);
            graph_config_frame_clone.set_visible(display_type == ContentDisplayType::Graph);
            core_bars_config_frame_clone.set_visible(display_type == ContentDisplayType::CoreBars);
            static_config_frame_clone.set_visible(display_type == ContentDisplayType::Static);
            arc_config_frame_clone.set_visible(display_type == ContentDisplayType::Arc);
            speedometer_config_frame_clone.set_visible(display_type == ContentDisplayType::Speedometer);
            let mut cfg = config_clone.borrow_mut();
            let item = cfg.frame.content_items
                .entry(slot_name_clone.clone())
                .or_default();
            item.display_as = display_type;
            drop(cfg);
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });

        // Item height change handler
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

        tab
    }

    /// Default bar config with Material Design colors
    #[allow(clippy::field_reassign_with_default)]
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
    #[allow(clippy::field_reassign_with_default)]
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

        // Rebuild group weight spinners in Layout tab
        if let Some(widgets) = self.layout_widgets.borrow().as_ref() {
            Self::rebuild_group_spinners(
                &self.config,
                &self.on_change,
                &self.preview,
                &widgets.group_weights_box,
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
}

impl Default for MaterialConfigWidget {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}
