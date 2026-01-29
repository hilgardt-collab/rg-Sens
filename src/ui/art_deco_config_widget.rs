//! Art Deco configuration widget
//!
//! Provides a tabbed interface for configuring all aspects of the Art Deco display.

use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, CheckButton, DrawingArea, DropDown, Entry, Label, Notebook, Orientation,
    ScrolledWindow, SpinButton, StringList,
};
use std::cell::RefCell;
use std::rc::Rc;

use crate::core::FieldMetadata;
use crate::displayers::ArtDecoDisplayConfig;
use crate::ui::art_deco_display::{
    render_art_deco_frame, BackgroundPattern, BorderStyle, CornerStyle, DividerStyle, HeaderStyle,
};
use crate::ui::background::Color;
use crate::ui::color_button_widget::ColorButtonWidget;
use crate::ui::combo_config_base;
use crate::ui::lcars_display::SplitOrientation;
use crate::ui::shared_font_dialog::show_font_dialog;
use crate::ui::theme::{ColorStopSource, ComboThemeConfig, LinearGradientSourceConfig};
use crate::ui::theme_color_selector::ThemeColorSelector;
use crate::ui::widget_builder::{
    create_page_container, create_section_header, ConfigWidgetBuilder,
};
use crate::ui::{GradientEditor, ThemeFontSelector};

/// Art Deco color preset names
const ART_DECO_PRESETS: &[&str] = &[
    "Gold & Black",
    "Copper & Teal",
    "Silver & Navy",
    "Bronze & Burgundy",
    "Emerald & Gold",
    "Custom",
];

/// Get theme colors for a preset index
fn get_preset_theme(preset_idx: u32) -> Option<ComboThemeConfig> {
    match preset_idx {
        0 => Some(ComboThemeConfig {
            // Gold & Black (default)
            color1: Color::new(0.831, 0.686, 0.216, 1.0), // Gold #D4AF37
            color2: Color::new(0.722, 0.451, 0.200, 1.0), // Copper #B87333
            color3: Color::new(0.804, 0.608, 0.114, 1.0), // Brass #CD9B1D
            color4: Color::new(0.102, 0.102, 0.102, 1.0), // Dark charcoal #1A1A1A
            gradient: LinearGradientSourceConfig {
                angle: 180.0,
                stops: vec![
                    ColorStopSource::theme(0.0, 1),
                    ColorStopSource::theme(1.0, 2),
                ],
            },
            font1_family: "Sans Bold".to_string(),
            font1_size: 14.0,
            font2_family: "Sans".to_string(),
            font2_size: 11.0,
        }),
        1 => Some(ComboThemeConfig {
            // Copper & Teal
            color1: Color::new(0.722, 0.451, 0.200, 1.0), // Copper #B87333
            color2: Color::new(0.0, 0.502, 0.502, 1.0),   // Teal #008080
            color3: Color::new(0.545, 0.271, 0.075, 1.0), // Saddle brown #8B4513
            color4: Color::new(0.067, 0.094, 0.106, 1.0), // Dark teal background #111821
            gradient: LinearGradientSourceConfig {
                angle: 180.0,
                stops: vec![
                    ColorStopSource::theme(0.0, 1),
                    ColorStopSource::theme(1.0, 2),
                ],
            },
            font1_family: "Sans Bold".to_string(),
            font1_size: 14.0,
            font2_family: "Sans".to_string(),
            font2_size: 11.0,
        }),
        2 => Some(ComboThemeConfig {
            // Silver & Navy
            color1: Color::new(0.753, 0.753, 0.753, 1.0), // Silver #C0C0C0
            color2: Color::new(0.0, 0.0, 0.502, 1.0),     // Navy #000080
            color3: Color::new(0.467, 0.533, 0.600, 1.0), // Slate gray #778899
            color4: Color::new(0.059, 0.071, 0.118, 1.0), // Dark navy #0F121E
            gradient: LinearGradientSourceConfig {
                angle: 180.0,
                stops: vec![
                    ColorStopSource::theme(0.0, 1),
                    ColorStopSource::theme(1.0, 3),
                ],
            },
            font1_family: "Sans Bold".to_string(),
            font1_size: 14.0,
            font2_family: "Sans".to_string(),
            font2_size: 11.0,
        }),
        3 => Some(ComboThemeConfig {
            // Bronze & Burgundy
            color1: Color::new(0.804, 0.498, 0.196, 1.0), // Bronze #CD7F32
            color2: Color::new(0.502, 0.0, 0.125, 1.0),   // Burgundy #800020
            color3: Color::new(0.647, 0.165, 0.165, 1.0), // Brown #A52A2A
            color4: Color::new(0.122, 0.063, 0.075, 1.0), // Dark burgundy #1F1013
            gradient: LinearGradientSourceConfig {
                angle: 180.0,
                stops: vec![
                    ColorStopSource::theme(0.0, 1),
                    ColorStopSource::theme(1.0, 2),
                ],
            },
            font1_family: "Sans Bold".to_string(),
            font1_size: 14.0,
            font2_family: "Sans".to_string(),
            font2_size: 11.0,
        }),
        4 => Some(ComboThemeConfig {
            // Emerald & Gold
            color1: Color::new(0.314, 0.784, 0.471, 1.0), // Emerald #50C878
            color2: Color::new(0.831, 0.686, 0.216, 1.0), // Gold #D4AF37
            color3: Color::new(0.0, 0.392, 0.0, 1.0),     // Dark green #006400
            color4: Color::new(0.059, 0.094, 0.067, 1.0), // Dark green background #0F1811
            gradient: LinearGradientSourceConfig {
                angle: 180.0,
                stops: vec![
                    ColorStopSource::theme(0.0, 1),
                    ColorStopSource::theme(1.0, 2),
                ],
            },
            font1_family: "Sans Bold".to_string(),
            font1_size: 14.0,
            font2_family: "Sans".to_string(),
            font2_size: 11.0,
        }),
        _ => None, // Custom - don't change colors
    }
}

/// Holds references to Theme tab widgets
#[allow(dead_code)]
struct ThemeWidgets {
    preset_dropdown: DropDown,
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

/// Holds references to Frame tab widgets
struct FrameWidgets {
    border_style_dropdown: DropDown,
    border_width_spin: SpinButton,
    corner_style_dropdown: DropDown,
    corner_size_spin: SpinButton,
    accent_width_spin: SpinButton,
}

/// Holds references to Background tab widgets
struct BackgroundWidgets {
    pattern_dropdown: DropDown,
    pattern_spacing_spin: SpinButton,
    sunburst_rays_spin: SpinButton,
}

/// Holds references to Header tab widgets
#[allow(dead_code)]
struct HeaderWidgets {
    show_header_check: CheckButton,
    header_text_entry: Entry,
    header_style_dropdown: DropDown,
    header_font_selector: Rc<ThemeFontSelector>,
}

/// Holds references to Layout tab widgets
struct LayoutWidgets {
    split_orientation_dropdown: DropDown,
    content_padding_spin: SpinButton,
    divider_style_dropdown: DropDown,
    divider_width_spin: SpinButton,
    divider_padding_spin: SpinButton,
    group_settings_box: GtkBox,
}

/// Holds references to Animation tab widgets
struct AnimationWidgets {
    enable_check: CheckButton,
    speed_spin: SpinButton,
}

/// Art Deco configuration widget
pub struct ArtDecoConfigWidget {
    container: GtkBox,
    config: Rc<RefCell<ArtDecoDisplayConfig>>,
    on_change: Rc<RefCell<Option<Box<dyn Fn()>>>>,
    preview: DrawingArea,
    content_notebook: Rc<RefCell<Notebook>>,
    source_summaries: Rc<RefCell<Vec<(String, String, usize, u32)>>>,
    available_fields: Rc<RefCell<Vec<FieldMetadata>>>,
    theme_widgets: Rc<RefCell<Option<ThemeWidgets>>>,
    frame_widgets: Rc<RefCell<Option<FrameWidgets>>>,
    background_widgets: Rc<RefCell<Option<BackgroundWidgets>>>,
    header_widgets: Rc<RefCell<Option<HeaderWidgets>>>,
    layout_widgets: Rc<RefCell<Option<LayoutWidgets>>>,
    animation_widgets: Rc<RefCell<Option<AnimationWidgets>>>,
    theme_ref_refreshers: Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
}

impl ArtDecoConfigWidget {
    pub fn new(available_fields: Vec<FieldMetadata>) -> Self {
        let container = GtkBox::new(Orientation::Vertical, 12);
        let config = Rc::new(RefCell::new(ArtDecoDisplayConfig::default()));
        let on_change: Rc<RefCell<Option<Box<dyn Fn()>>>> = Rc::new(RefCell::new(None));
        let source_summaries: Rc<RefCell<Vec<(String, String, usize, u32)>>> =
            Rc::new(RefCell::new(Vec::new()));
        let available_fields: Rc<RefCell<Vec<FieldMetadata>>> =
            Rc::new(RefCell::new(available_fields));
        let theme_widgets: Rc<RefCell<Option<ThemeWidgets>>> = Rc::new(RefCell::new(None));
        let frame_widgets: Rc<RefCell<Option<FrameWidgets>>> = Rc::new(RefCell::new(None));
        let background_widgets: Rc<RefCell<Option<BackgroundWidgets>>> =
            Rc::new(RefCell::new(None));
        let header_widgets: Rc<RefCell<Option<HeaderWidgets>>> = Rc::new(RefCell::new(None));
        let layout_widgets: Rc<RefCell<Option<LayoutWidgets>>> = Rc::new(RefCell::new(None));
        let animation_widgets: Rc<RefCell<Option<AnimationWidgets>>> = Rc::new(RefCell::new(None));
        let theme_ref_refreshers: Rc<RefCell<Vec<Rc<dyn Fn()>>>> =
            Rc::new(RefCell::new(Vec::new()));

        // Preview at the top
        let preview = DrawingArea::new();
        preview.set_content_height(180);
        preview.set_content_width(200); // Min width 200px
        preview.set_hexpand(true);
        preview.set_halign(gtk4::Align::Fill);
        preview.set_vexpand(false);

        let config_clone = config.clone();
        preview.set_draw_func(move |_, cr, width, height| {
            if width < 10 || height < 10 {
                return;
            }

            let cfg = config_clone.borrow();
            let _ = render_art_deco_frame(cr, &cfg.frame, width as f64, height as f64);
        });

        // Theme reference section
        let (theme_ref_section, main_theme_refresh_cb) =
            combo_config_base::create_theme_reference_section(&config, |cfg| {
                cfg.frame.theme.clone()
            });

        // Push main theme callback FIRST so it's preserved when rebuild_content_tabs clears
        theme_ref_refreshers
            .borrow_mut()
            .push(main_theme_refresh_cb);

        // Main tabbed notebook
        let notebook = Notebook::new();
        notebook.set_vexpand(true);

        // Content notebook (for dynamic tabs based on source data)
        let content_notebook = Rc::new(RefCell::new(Notebook::new()));

        // Create pages - Theme first
        let theme_page = Self::create_theme_page(
            &config,
            &on_change,
            &preview,
            &theme_widgets,
            &theme_ref_refreshers,
        );

        let frame_page = Self::create_frame_page(
            &config,
            &on_change,
            &preview,
            &frame_widgets,
            &theme_ref_refreshers,
        );

        let background_page = Self::create_background_page(
            &config,
            &on_change,
            &preview,
            &background_widgets,
            &theme_ref_refreshers,
        );

        let header_page = Self::create_header_page(
            &config,
            &on_change,
            &preview,
            &header_widgets,
            &theme_ref_refreshers,
        );

        let layout_page = Self::create_layout_page(
            &config,
            &on_change,
            &preview,
            &layout_widgets,
            &theme_ref_refreshers,
        );

        let content_page = Self::create_content_page(&content_notebook);

        let animation_page =
            Self::create_animation_page(&config, &on_change, &preview, &animation_widgets);

        // Add pages to notebook (Theme first)
        notebook.append_page(&theme_page, Some(&Label::new(Some("Theme"))));
        notebook.append_page(&frame_page, Some(&Label::new(Some("Frame"))));
        notebook.append_page(&background_page, Some(&Label::new(Some("Background"))));
        notebook.append_page(&header_page, Some(&Label::new(Some("Header"))));
        notebook.append_page(&layout_page, Some(&Label::new(Some("Layout"))));
        notebook.append_page(&content_page, Some(&Label::new(Some("Content"))));
        notebook.append_page(&animation_page, Some(&Label::new(Some("Animation"))));

        // Assemble container
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
            background_widgets,
            header_widgets,
            layout_widgets,
            animation_widgets,
            theme_ref_refreshers,
        }
    }

    pub fn widget(&self) -> &GtkBox {
        &self.container
    }

    pub fn get_config(&self) -> ArtDecoDisplayConfig {
        self.config.borrow().clone()
    }

    pub fn get_config_rc(&self) -> Rc<RefCell<ArtDecoDisplayConfig>> {
        self.config.clone()
    }

    pub fn set_config(&self, config: &ArtDecoDisplayConfig) {
        // IMPORTANT: Temporarily disable on_change callback to prevent signal cascade.
        let saved_callback = self.on_change.borrow_mut().take();

        *self.config.borrow_mut() = config.clone();
        self.refresh_all_widgets();

        // Restore the on_change callback now that widget updates are complete
        *self.on_change.borrow_mut() = saved_callback;
    }

    pub fn set_on_change<F: Fn() + 'static>(&self, callback: F) {
        *self.on_change.borrow_mut() = Some(Box::new(callback));
    }

    pub fn set_theme(&self, theme: crate::ui::theme::ComboThemeConfig) {
        self.config.borrow_mut().frame.theme = theme;
        combo_config_base::refresh_theme_refs(&self.theme_ref_refreshers);
        self.preview.queue_draw();
    }

    fn create_theme_page(
        config: &Rc<RefCell<ArtDecoDisplayConfig>>,
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

        // Color Preset dropdown
        let preset_box = GtkBox::new(Orientation::Horizontal, 6);
        preset_box.append(&Label::new(Some("Color Preset:")));
        let preset_list = StringList::new(ART_DECO_PRESETS);
        let preset_dropdown = DropDown::new(Some(preset_list), Option::<gtk4::Expression>::None);
        preset_dropdown.set_selected(ART_DECO_PRESETS.len() as u32 - 1); // Default to "Custom"
        preset_dropdown.set_hexpand(true);
        preset_box.append(&preset_dropdown);
        inner_box.append(&preset_box);

        // Info label
        let info_label = Label::new(Some("Select a preset or customize colors below.\nThese can be referenced in content items for consistent styling."));
        info_label.set_halign(gtk4::Align::Start);
        info_label.add_css_class("dim-label");
        inner_box.append(&info_label);

        // Theme colors section - 2x2 grid layout
        let colors_label = Label::new(Some("Theme Colors"));
        colors_label.set_halign(gtk4::Align::Start);
        colors_label.add_css_class("heading");
        colors_label.set_margin_top(8);
        inner_box.append(&colors_label);

        let colors_grid = gtk4::Grid::new();
        colors_grid.set_row_spacing(6);
        colors_grid.set_column_spacing(8);
        colors_grid.set_margin_start(6);

        // Color 1 (Primary) - row 0, col 0-1
        let color1_label = Label::new(Some("C1 (Primary):"));
        color1_label.set_halign(gtk4::Align::End);
        color1_label.set_width_chars(14);
        colors_grid.attach(&color1_label, 0, 0, 1, 1);
        let theme_color1_widget =
            Rc::new(ColorButtonWidget::new(config.borrow().frame.theme.color1));
        colors_grid.attach(theme_color1_widget.widget(), 1, 0, 1, 1);

        // Color 2 (Secondary) - row 0, col 2-3
        let color2_label = Label::new(Some("C2 (Secondary):"));
        color2_label.set_halign(gtk4::Align::End);
        color2_label.set_width_chars(14);
        color2_label.set_margin_start(12);
        colors_grid.attach(&color2_label, 2, 0, 1, 1);
        let theme_color2_widget =
            Rc::new(ColorButtonWidget::new(config.borrow().frame.theme.color2));
        colors_grid.attach(theme_color2_widget.widget(), 3, 0, 1, 1);

        // Color 3 (Tertiary) - row 1, col 0-1
        let color3_label = Label::new(Some("C3 (Tertiary):"));
        color3_label.set_halign(gtk4::Align::End);
        color3_label.set_width_chars(14);
        colors_grid.attach(&color3_label, 0, 1, 1, 1);
        let theme_color3_widget =
            Rc::new(ColorButtonWidget::new(config.borrow().frame.theme.color3));
        colors_grid.attach(theme_color3_widget.widget(), 1, 1, 1, 1);

        // Color 4 (Background) - row 1, col 2-3
        let color4_label = Label::new(Some("C4 (Background):"));
        color4_label.set_halign(gtk4::Align::End);
        color4_label.set_width_chars(14);
        color4_label.set_margin_start(12);
        colors_grid.attach(&color4_label, 2, 1, 1, 1);
        let theme_color4_widget =
            Rc::new(ColorButtonWidget::new(config.borrow().frame.theme.color4));
        colors_grid.attach(theme_color4_widget.widget(), 3, 1, 1, 1);

        inner_box.append(&colors_grid);

        // Connect callbacks for each color (switch to Custom when manually changing)
        let config_c1 = config.clone();
        let on_change_c1 = on_change.clone();
        let preview_c1 = preview.clone();
        let refreshers_c1 = theme_ref_refreshers.clone();
        let preset_dropdown_c1 = preset_dropdown.clone();
        theme_color1_widget.set_on_change(move |color| {
            config_c1.borrow_mut().frame.theme.color1 = color;
            preset_dropdown_c1.set_selected(ART_DECO_PRESETS.len() as u32 - 1);
            combo_config_base::refresh_theme_refs(&refreshers_c1);
            combo_config_base::queue_redraw(&preview_c1, &on_change_c1);
        });

        let config_c2 = config.clone();
        let on_change_c2 = on_change.clone();
        let preview_c2 = preview.clone();
        let refreshers_c2 = theme_ref_refreshers.clone();
        let preset_dropdown_c2 = preset_dropdown.clone();
        theme_color2_widget.set_on_change(move |color| {
            config_c2.borrow_mut().frame.theme.color2 = color;
            preset_dropdown_c2.set_selected(ART_DECO_PRESETS.len() as u32 - 1);
            combo_config_base::refresh_theme_refs(&refreshers_c2);
            combo_config_base::queue_redraw(&preview_c2, &on_change_c2);
        });

        let config_c3 = config.clone();
        let on_change_c3 = on_change.clone();
        let preview_c3 = preview.clone();
        let refreshers_c3 = theme_ref_refreshers.clone();
        let preset_dropdown_c3 = preset_dropdown.clone();
        theme_color3_widget.set_on_change(move |color| {
            config_c3.borrow_mut().frame.theme.color3 = color;
            preset_dropdown_c3.set_selected(ART_DECO_PRESETS.len() as u32 - 1);
            combo_config_base::refresh_theme_refs(&refreshers_c3);
            combo_config_base::queue_redraw(&preview_c3, &on_change_c3);
        });

        let config_c4 = config.clone();
        let on_change_c4 = on_change.clone();
        let preview_c4 = preview.clone();
        let refreshers_c4 = theme_ref_refreshers.clone();
        let preset_dropdown_c4 = preset_dropdown.clone();
        theme_color4_widget.set_on_change(move |color| {
            config_c4.borrow_mut().frame.theme.color4 = color;
            preset_dropdown_c4.set_selected(ART_DECO_PRESETS.len() as u32 - 1);
            combo_config_base::refresh_theme_refs(&refreshers_c4);
            combo_config_base::queue_redraw(&preview_c4, &on_change_c4);
        });

        // Preset dropdown change handler - updates all colors
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let refreshers_clone = theme_ref_refreshers.clone();
        let color1_widget_clone = theme_color1_widget.clone();
        let color2_widget_clone = theme_color2_widget.clone();
        let color3_widget_clone = theme_color3_widget.clone();
        let color4_widget_clone = theme_color4_widget.clone();
        let gradient_editor_for_preset: Rc<RefCell<Option<Rc<GradientEditor>>>> =
            Rc::new(RefCell::new(None));
        let gradient_editor_for_preset_clone = gradient_editor_for_preset.clone();
        preset_dropdown.connect_selected_notify(move |dropdown| {
            let selected = dropdown.selected();
            if let Some(preset_theme) = get_preset_theme(selected) {
                // Update config
                {
                    let mut cfg = config_clone.borrow_mut();
                    cfg.frame.theme.color1 = preset_theme.color1;
                    cfg.frame.theme.color2 = preset_theme.color2;
                    cfg.frame.theme.color3 = preset_theme.color3;
                    cfg.frame.theme.color4 = preset_theme.color4;
                    cfg.frame.theme.gradient = preset_theme.gradient.clone();
                }
                // Update color button widgets
                color1_widget_clone.set_color(preset_theme.color1);
                color2_widget_clone.set_color(preset_theme.color2);
                color3_widget_clone.set_color(preset_theme.color3);
                color4_widget_clone.set_color(preset_theme.color4);
                // Update gradient editor if available
                if let Some(ref editor) = *gradient_editor_for_preset_clone.borrow() {
                    editor.set_gradient_source_config(&preset_theme.gradient);
                    editor.set_theme_config(preset_theme.clone());
                    // Also explicitly update the gradient editor's internal theme colors
                    editor.update_theme_colors(
                        preset_theme.color1,
                        preset_theme.color2,
                        preset_theme.color3,
                        preset_theme.color4,
                    );
                }
                // Refresh all theme-linked widgets (theme reference section, etc.)
                combo_config_base::refresh_theme_refs(&refreshers_clone);
                combo_config_base::queue_redraw(&preview_clone, &on_change_clone);
            }
        });

        // Theme gradient section
        let gradient_label = Label::new(Some("Theme Gradient"));
        gradient_label.set_halign(gtk4::Align::Start);
        gradient_label.add_css_class("heading");
        gradient_label.set_margin_top(12);
        inner_box.append(&gradient_label);

        let theme_gradient_editor = Rc::new(GradientEditor::new());
        theme_gradient_editor.set_gradient_source_config(&config.borrow().frame.theme.gradient);
        theme_gradient_editor.set_theme_config(config.borrow().frame.theme.clone());
        inner_box.append(theme_gradient_editor.widget());

        // Store gradient editor reference for preset callback to use
        *gradient_editor_for_preset.borrow_mut() = Some(theme_gradient_editor.clone());

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let refreshers_clone = theme_ref_refreshers.clone();
        let gradient_editor_clone = theme_gradient_editor.clone();
        let preset_dropdown_clone = preset_dropdown.clone();
        theme_gradient_editor.set_on_change(move || {
            config_clone.borrow_mut().frame.theme.gradient =
                gradient_editor_clone.get_gradient_source_config();
            // Switch to Custom when manually changing gradient
            preset_dropdown_clone.set_selected(ART_DECO_PRESETS.len() as u32 - 1);
            combo_config_base::refresh_theme_refs(&refreshers_clone);
            combo_config_base::queue_redraw(&preview_clone, &on_change_clone);
        });

        // Register theme refresh callback for gradient editor
        let gradient_editor_for_refresh = theme_gradient_editor.clone();
        let config_for_gradient_refresh = config.clone();
        theme_ref_refreshers.borrow_mut().push(Rc::new(move || {
            let cfg = config_for_gradient_refresh.borrow();
            gradient_editor_for_refresh.set_theme_config(cfg.frame.theme.clone());
        }));

        // Theme fonts section
        let fonts_label = Label::new(Some("Theme Fonts"));
        fonts_label.set_halign(gtk4::Align::Start);
        fonts_label.add_css_class("heading");
        fonts_label.set_margin_top(12);
        inner_box.append(&fonts_label);

        // Font 1
        let font1_box = GtkBox::new(Orientation::Horizontal, 6);
        font1_box.append(&Label::new(Some("Font 1:")));
        let font1_btn = Button::with_label(&config.borrow().frame.theme.font1_family);
        font1_btn.set_hexpand(true);
        font1_box.append(&font1_btn);

        let font1_size_spin = SpinButton::with_range(6.0, 72.0, 1.0);
        font1_size_spin.set_value(config.borrow().frame.theme.font1_size);
        font1_box.append(&font1_size_spin);
        inner_box.append(&font1_box);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let refreshers_clone = theme_ref_refreshers.clone();
        let font1_btn_clone = font1_btn.clone();
        font1_btn.connect_clicked(move |btn| {
            let Some(window) = btn.root().and_then(|r| r.downcast::<gtk4::Window>().ok()) else {
                return;
            };
            let config_for_cb = config_clone.clone();
            let on_change_for_cb = on_change_clone.clone();
            let preview_for_cb = preview_clone.clone();
            let refreshers_for_cb = refreshers_clone.clone();
            let font_btn_for_cb = font1_btn_clone.clone();
            let current_font = config_clone.borrow().frame.theme.font1_family.clone();
            let font_desc = gtk4::pango::FontDescription::from_string(&current_font);
            show_font_dialog(Some(&window), Some(&font_desc), move |font_desc| {
                let family = font_desc
                    .family()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "sans-serif".to_string());
                config_for_cb.borrow_mut().frame.theme.font1_family = family.clone();
                font_btn_for_cb.set_label(&family);
                combo_config_base::refresh_theme_refs(&refreshers_for_cb);
                combo_config_base::queue_redraw(&preview_for_cb, &on_change_for_cb);
            });
        });

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let refreshers_clone = theme_ref_refreshers.clone();
        font1_size_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().frame.theme.font1_size = spin.value();
            combo_config_base::refresh_theme_refs(&refreshers_clone);
            combo_config_base::queue_redraw(&preview_clone, &on_change_clone);
        });

        // Font 2
        let font2_box = GtkBox::new(Orientation::Horizontal, 6);
        font2_box.append(&Label::new(Some("Font 2:")));
        let font2_btn = Button::with_label(&config.borrow().frame.theme.font2_family);
        font2_btn.set_hexpand(true);
        font2_box.append(&font2_btn);

        let font2_size_spin = SpinButton::with_range(6.0, 72.0, 1.0);
        font2_size_spin.set_value(config.borrow().frame.theme.font2_size);
        font2_box.append(&font2_size_spin);
        inner_box.append(&font2_box);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let refreshers_clone = theme_ref_refreshers.clone();
        let font2_btn_clone = font2_btn.clone();
        font2_btn.connect_clicked(move |btn| {
            let Some(window) = btn.root().and_then(|r| r.downcast::<gtk4::Window>().ok()) else {
                return;
            };
            let config_for_cb = config_clone.clone();
            let on_change_for_cb = on_change_clone.clone();
            let preview_for_cb = preview_clone.clone();
            let refreshers_for_cb = refreshers_clone.clone();
            let font_btn_for_cb = font2_btn_clone.clone();
            let current_font = config_clone.borrow().frame.theme.font2_family.clone();
            let font_desc = gtk4::pango::FontDescription::from_string(&current_font);
            show_font_dialog(Some(&window), Some(&font_desc), move |font_desc| {
                let family = font_desc
                    .family()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "sans-serif".to_string());
                config_for_cb.borrow_mut().frame.theme.font2_family = family.clone();
                font_btn_for_cb.set_label(&family);
                combo_config_base::refresh_theme_refs(&refreshers_for_cb);
                combo_config_base::queue_redraw(&preview_for_cb, &on_change_for_cb);
            });
        });

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let refreshers_clone = theme_ref_refreshers.clone();
        font2_size_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().frame.theme.font2_size = spin.value();
            combo_config_base::refresh_theme_refs(&refreshers_clone);
            combo_config_base::queue_redraw(&preview_clone, &on_change_clone);
        });

        scroll.set_child(Some(&inner_box));
        page.append(&scroll);

        // Store widget refs
        *theme_widgets_out.borrow_mut() = Some(ThemeWidgets {
            preset_dropdown,
            theme_color1_widget,
            theme_color2_widget,
            theme_color3_widget,
            theme_color4_widget,
            theme_gradient_editor,
            font1_btn,
            font1_size_spin,
            font2_btn,
            font2_size_spin,
        });

        page
    }

    fn create_frame_page(
        config: &Rc<RefCell<ArtDecoDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        frame_widgets_out: &Rc<RefCell<Option<FrameWidgets>>>,
        theme_ref_refreshers: &Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        combo_config_base::set_page_margins(&page);

        let builder = ConfigWidgetBuilder::new(config, preview, on_change);

        let style_idx = match config.borrow().frame.border_style {
            BorderStyle::Sunburst => 0,
            BorderStyle::Chevron => 1,
            BorderStyle::Stepped => 2,
            BorderStyle::Geometric => 3,
            BorderStyle::Ornate => 4,
        };
        let border_style_dropdown = builder.dropdown_row(
            &page,
            "Border Style:",
            &["Sunburst", "Chevron", "Stepped", "Geometric", "Ornate"],
            style_idx,
            |cfg, idx| {
                cfg.frame.border_style = match idx {
                    0 => BorderStyle::Sunburst,
                    1 => BorderStyle::Chevron,
                    2 => BorderStyle::Stepped,
                    3 => BorderStyle::Geometric,
                    _ => BorderStyle::Ornate,
                }
            },
        );

        let border_width_spin = builder.spin_row(
            &page,
            "Border Width:",
            1.0,
            10.0,
            0.5,
            config.borrow().frame.border_width,
            |cfg, v| cfg.frame.border_width = v,
        );

        // Border color (theme-aware)
        let border_color_box = GtkBox::new(Orientation::Horizontal, 6);
        border_color_box.append(&Label::new(Some("Border Color:")));
        let border_color_widget = Rc::new(ThemeColorSelector::new(
            config.borrow().frame.border_color.clone(),
        ));
        border_color_widget.set_theme_config(config.borrow().frame.theme.clone());
        border_color_box.append(border_color_widget.widget());

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        border_color_widget.set_on_change(move |new_source| {
            config_clone.borrow_mut().frame.border_color = new_source;
            combo_config_base::queue_redraw(&preview_clone, &on_change_clone);
        });

        let widget_for_refresh = border_color_widget.clone();
        let config_for_refresh = config.clone();
        theme_ref_refreshers.borrow_mut().push(Rc::new(move || {
            widget_for_refresh.set_theme_config(config_for_refresh.borrow().frame.theme.clone());
        }));
        page.append(&border_color_box);

        // Corner style section
        page.append(&create_section_header("Corner Decorations"));

        let corner_idx = match config.borrow().frame.corner_style {
            CornerStyle::Fan => 0,
            CornerStyle::Ziggurat => 1,
            CornerStyle::Diamond => 2,
            CornerStyle::Bracket => 3,
            CornerStyle::Hexagon => 4,
            CornerStyle::Octagon => 5,
            CornerStyle::Circle => 6,
            CornerStyle::DoubleBracket => 7,
            CornerStyle::GeometricStack => 8,
            CornerStyle::None => 9,
        };
        let corner_style_dropdown = builder.dropdown_row(
            &page,
            "Style:",
            &["Fan", "Ziggurat", "Diamond", "Bracket", "Hexagon", "Octagon", "Circle", "Double Bracket", "Geometric Stack", "None"],
            corner_idx,
            |cfg, idx| {
                cfg.frame.corner_style = match idx {
                    0 => CornerStyle::Fan,
                    1 => CornerStyle::Ziggurat,
                    2 => CornerStyle::Diamond,
                    3 => CornerStyle::Bracket,
                    4 => CornerStyle::Hexagon,
                    5 => CornerStyle::Octagon,
                    6 => CornerStyle::Circle,
                    7 => CornerStyle::DoubleBracket,
                    8 => CornerStyle::GeometricStack,
                    _ => CornerStyle::None,
                }
            },
        );

        let corner_size_spin = builder.spin_row(
            &page,
            "Size:",
            8.0,
            64.0,
            2.0,
            config.borrow().frame.corner_size,
            |cfg, v| cfg.frame.corner_size = v,
        );

        let accent_width_spin = builder.spin_row(
            &page,
            "Accent Width:",
            0.5,
            5.0,
            0.5,
            config.borrow().frame.accent_width,
            |cfg, v| cfg.frame.accent_width = v,
        );

        // Accent color (theme-aware)
        let accent_color_box = GtkBox::new(Orientation::Horizontal, 6);
        accent_color_box.append(&Label::new(Some("Accent Color:")));
        let accent_color_widget = Rc::new(ThemeColorSelector::new(
            config.borrow().frame.accent_color.clone(),
        ));
        accent_color_widget.set_theme_config(config.borrow().frame.theme.clone());
        accent_color_box.append(accent_color_widget.widget());

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        accent_color_widget.set_on_change(move |new_source| {
            config_clone.borrow_mut().frame.accent_color = new_source;
            combo_config_base::queue_redraw(&preview_clone, &on_change_clone);
        });

        let widget_for_refresh = accent_color_widget.clone();
        let config_for_refresh = config.clone();
        theme_ref_refreshers.borrow_mut().push(Rc::new(move || {
            widget_for_refresh.set_theme_config(config_for_refresh.borrow().frame.theme.clone());
        }));
        page.append(&accent_color_box);

        // Store widget refs
        *frame_widgets_out.borrow_mut() = Some(FrameWidgets {
            border_style_dropdown,
            border_width_spin,
            corner_style_dropdown,
            corner_size_spin,
            accent_width_spin,
        });

        page
    }

    fn create_background_page(
        config: &Rc<RefCell<ArtDecoDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        background_widgets_out: &Rc<RefCell<Option<BackgroundWidgets>>>,
        theme_ref_refreshers: &Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        combo_config_base::set_page_margins(&page);

        let builder = ConfigWidgetBuilder::new(config, preview, on_change);

        // Background color (theme-aware) - keep manual for ThemeColorSelector
        let bg_color_box = GtkBox::new(Orientation::Horizontal, 6);
        bg_color_box.append(&Label::new(Some("Background Color:")));
        let bg_color_widget = Rc::new(ThemeColorSelector::new(
            config.borrow().frame.background_color.clone(),
        ));
        bg_color_widget.set_theme_config(config.borrow().frame.theme.clone());
        bg_color_box.append(bg_color_widget.widget());

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        bg_color_widget.set_on_change(move |new_source| {
            config_clone.borrow_mut().frame.background_color = new_source;
            combo_config_base::queue_redraw(&preview_clone, &on_change_clone);
        });

        let widget_for_refresh = bg_color_widget.clone();
        let config_for_refresh = config.clone();
        theme_ref_refreshers.borrow_mut().push(Rc::new(move || {
            widget_for_refresh.set_theme_config(config_for_refresh.borrow().frame.theme.clone());
        }));
        page.append(&bg_color_box);

        // Pattern section
        page.append(&create_section_header("Background Pattern"));

        let pattern_idx = match config.borrow().frame.background_pattern {
            BackgroundPattern::Solid => 0,
            BackgroundPattern::VerticalLines => 1,
            BackgroundPattern::DiamondGrid => 2,
            BackgroundPattern::Sunburst => 3,
            BackgroundPattern::Chevrons => 4,
        };
        let pattern_dropdown = builder.dropdown_row(
            &page,
            "Pattern:",
            &[
                "Solid",
                "Vertical Lines",
                "Diamond Grid",
                "Sunburst",
                "Chevrons",
            ],
            pattern_idx,
            |cfg, idx| {
                cfg.frame.background_pattern = match idx {
                    0 => BackgroundPattern::Solid,
                    1 => BackgroundPattern::VerticalLines,
                    2 => BackgroundPattern::DiamondGrid,
                    3 => BackgroundPattern::Sunburst,
                    _ => BackgroundPattern::Chevrons,
                }
            },
        );

        let pattern_spacing_spin = builder.spin_row(
            &page,
            "Pattern Spacing:",
            8.0,
            64.0,
            2.0,
            config.borrow().frame.pattern_spacing,
            |cfg, v| cfg.frame.pattern_spacing = v,
        );

        let sunburst_rays_spin = builder.spin_row(
            &page,
            "Sunburst Rays:",
            6.0,
            36.0,
            2.0,
            config.borrow().frame.sunburst_rays as f64,
            |cfg, v| cfg.frame.sunburst_rays = v as usize,
        );

        // Pattern color (theme-aware)
        let pattern_color_box = GtkBox::new(Orientation::Horizontal, 6);
        pattern_color_box.append(&Label::new(Some("Pattern Color:")));
        let pattern_color_widget = Rc::new(ThemeColorSelector::new(
            config.borrow().frame.pattern_color.clone(),
        ));
        pattern_color_widget.set_theme_config(config.borrow().frame.theme.clone());
        pattern_color_box.append(pattern_color_widget.widget());

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        pattern_color_widget.set_on_change(move |new_source| {
            config_clone.borrow_mut().frame.pattern_color = new_source;
            combo_config_base::queue_redraw(&preview_clone, &on_change_clone);
        });

        let widget_for_refresh = pattern_color_widget.clone();
        let config_for_refresh = config.clone();
        theme_ref_refreshers.borrow_mut().push(Rc::new(move || {
            widget_for_refresh.set_theme_config(config_for_refresh.borrow().frame.theme.clone());
        }));
        page.append(&pattern_color_box);

        // Store widget refs
        *background_widgets_out.borrow_mut() = Some(BackgroundWidgets {
            pattern_dropdown,
            pattern_spacing_spin,
            sunburst_rays_spin,
        });

        page
    }

    fn create_header_page(
        config: &Rc<RefCell<ArtDecoDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        header_widgets_out: &Rc<RefCell<Option<HeaderWidgets>>>,
        theme_ref_refreshers: &Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        combo_config_base::set_page_margins(&page);

        let builder = ConfigWidgetBuilder::new(config, preview, on_change);

        let show_header_check = builder.check_button(
            &page,
            "Show Header",
            config.borrow().frame.show_header,
            |cfg, v| cfg.frame.show_header = v,
        );

        let header_text_entry = builder.entry_row(
            &page,
            "Header Text:",
            &config.borrow().frame.header_text.clone(),
            |cfg, s| cfg.frame.header_text = s,
        );

        let style_idx = match config.borrow().frame.header_style {
            HeaderStyle::Centered => 0,
            HeaderStyle::Banner => 1,
            HeaderStyle::Stepped => 2,
            HeaderStyle::None => 3,
        };
        let header_style_dropdown = builder.dropdown_row(
            &page,
            "Style:",
            &["Centered", "Banner", "Stepped", "None"],
            style_idx,
            |cfg, idx| {
                cfg.frame.header_style = match idx {
                    0 => HeaderStyle::Centered,
                    1 => HeaderStyle::Banner,
                    2 => HeaderStyle::Stepped,
                    _ => HeaderStyle::None,
                }
            },
        );

        // Header font
        let font_box = GtkBox::new(Orientation::Horizontal, 6);
        font_box.append(&Label::new(Some("Font:")));
        let header_font_selector = Rc::new(ThemeFontSelector::new(
            config.borrow().frame.header_font.clone(),
        ));
        header_font_selector.set_theme_config(config.borrow().frame.theme.clone());
        font_box.append(header_font_selector.widget());

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        header_font_selector.set_on_change(move |new_source| {
            config_clone.borrow_mut().frame.header_font = new_source;
            combo_config_base::queue_redraw(&preview_clone, &on_change_clone);
        });

        let widget_for_refresh = header_font_selector.clone();
        let config_for_refresh = config.clone();
        theme_ref_refreshers.borrow_mut().push(Rc::new(move || {
            widget_for_refresh.set_theme_config(config_for_refresh.borrow().frame.theme.clone());
        }));
        page.append(&font_box);

        // Header color (theme-aware)
        let color_box = GtkBox::new(Orientation::Horizontal, 6);
        color_box.append(&Label::new(Some("Header Color:")));
        let header_color_widget = Rc::new(ThemeColorSelector::new(
            config.borrow().frame.header_color.clone(),
        ));
        header_color_widget.set_theme_config(config.borrow().frame.theme.clone());
        color_box.append(header_color_widget.widget());

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        header_color_widget.set_on_change(move |new_source| {
            config_clone.borrow_mut().frame.header_color = new_source;
            combo_config_base::queue_redraw(&preview_clone, &on_change_clone);
        });

        let widget_for_refresh = header_color_widget.clone();
        let config_for_refresh = config.clone();
        theme_ref_refreshers.borrow_mut().push(Rc::new(move || {
            widget_for_refresh.set_theme_config(config_for_refresh.borrow().frame.theme.clone());
        }));
        page.append(&color_box);

        // Store widget refs
        *header_widgets_out.borrow_mut() = Some(HeaderWidgets {
            show_header_check,
            header_text_entry,
            header_style_dropdown,
            header_font_selector,
        });

        page
    }

    fn create_layout_page(
        config: &Rc<RefCell<ArtDecoDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        layout_widgets_out: &Rc<RefCell<Option<LayoutWidgets>>>,
        theme_ref_refreshers: &Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        combo_config_base::set_page_margins(&page);

        let builder = ConfigWidgetBuilder::new(config, preview, on_change);

        let orient_idx = match config.borrow().frame.split_orientation {
            SplitOrientation::Horizontal => 0,
            SplitOrientation::Vertical => 1,
        };
        let split_orientation_dropdown = builder.dropdown_row(
            &page,
            "Group Direction:",
            &["Horizontal", "Vertical"],
            orient_idx,
            |cfg, idx| {
                cfg.frame.split_orientation = match idx {
                    0 => SplitOrientation::Horizontal,
                    _ => SplitOrientation::Vertical,
                }
            },
        );

        let content_padding_spin = builder.spin_row(
            &page,
            "Content Padding:",
            4.0,
            48.0,
            2.0,
            config.borrow().frame.content_padding,
            |cfg, v| cfg.frame.content_padding = v,
        );

        // Divider section
        page.append(&create_section_header("Dividers"));

        let div_style_idx = match config.borrow().frame.divider_style {
            DividerStyle::Chevron => 0,
            DividerStyle::DoubleLine => 1,
            DividerStyle::Line => 2,
            DividerStyle::Stepped => 3,
            DividerStyle::DiamondCluster => 4,
            DividerStyle::Crescent => 5,
            DividerStyle::ArrowDiamond => 6,
            DividerStyle::CircleChain => 7,
            DividerStyle::CrossedLines => 8,
            DividerStyle::FleurDeLis => 9,
            DividerStyle::Heartbeat => 10,
            DividerStyle::DiamondGrid => 11,
            DividerStyle::None => 12,
        };
        let divider_style_dropdown = builder.dropdown_row(
            &page,
            "Style:",
            &["Chevron", "Double Line", "Line", "Stepped", "Diamond Cluster", "Crescent", "Arrow Diamond", "Circle Chain", "Crossed Lines", "Fleur de Lis", "Heartbeat", "Diamond Grid", "None"],
            div_style_idx,
            |cfg, idx| {
                cfg.frame.divider_style = match idx {
                    0 => DividerStyle::Chevron,
                    1 => DividerStyle::DoubleLine,
                    2 => DividerStyle::Line,
                    3 => DividerStyle::Stepped,
                    4 => DividerStyle::DiamondCluster,
                    5 => DividerStyle::Crescent,
                    6 => DividerStyle::ArrowDiamond,
                    7 => DividerStyle::CircleChain,
                    8 => DividerStyle::CrossedLines,
                    9 => DividerStyle::FleurDeLis,
                    10 => DividerStyle::Heartbeat,
                    11 => DividerStyle::DiamondGrid,
                    _ => DividerStyle::None,
                }
            },
        );

        let divider_width_spin = builder.spin_row(
            &page,
            "Width:",
            1.0,
            8.0,
            0.5,
            config.borrow().frame.divider_width,
            |cfg, v| cfg.frame.divider_width = v,
        );

        let divider_padding_spin = builder.spin_row(
            &page,
            "Padding:",
            2.0,
            24.0,
            2.0,
            config.borrow().frame.divider_padding,
            |cfg, v| cfg.frame.divider_padding = v,
        );

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
            combo_config_base::queue_redraw(&preview_clone, &on_change_clone);
        });

        let widget_for_refresh = divider_color_widget.clone();
        let config_for_refresh = config.clone();
        theme_ref_refreshers.borrow_mut().push(Rc::new(move || {
            widget_for_refresh.set_theme_config(config_for_refresh.borrow().frame.theme.clone());
        }));
        page.append(&div_color_box);

        // Combined group settings section (weight + orientation per group)
        let group_settings_box = combo_config_base::create_combined_group_settings_section(&page);
        combo_config_base::rebuild_combined_group_settings(
            &group_settings_box,
            config,
            |c: &mut ArtDecoDisplayConfig| &mut c.frame,
            on_change,
            preview,
        );

        // Store widget refs
        *layout_widgets_out.borrow_mut() = Some(LayoutWidgets {
            split_orientation_dropdown,
            content_padding_spin,
            divider_style_dropdown,
            divider_width_spin,
            divider_padding_spin,
            group_settings_box,
        });

        page
    }

    fn create_content_page(content_notebook: &Rc<RefCell<Notebook>>) -> GtkBox {
        let page = create_page_container();

        let info_label = Label::new(Some(
            "Content configuration will appear here based on the connected data source.",
        ));
        info_label.set_halign(gtk4::Align::Start);
        info_label.add_css_class("dim-label");
        page.append(&info_label);

        let notebook = content_notebook.borrow();
        page.append(&*notebook);

        page
    }

    fn create_animation_page(
        config: &Rc<RefCell<ArtDecoDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        animation_widgets_out: &Rc<RefCell<Option<AnimationWidgets>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        combo_config_base::set_page_margins(&page);

        let builder = ConfigWidgetBuilder::new(config, preview, on_change);

        let enable_check = builder.check_button(
            &page,
            "Enable Animation",
            config.borrow().animation_enabled,
            |cfg, v| cfg.animation_enabled = v,
        );

        let speed_spin = builder.spin_row(
            &page,
            "Animation Speed:",
            1.0,
            20.0,
            0.5,
            config.borrow().animation_speed,
            |cfg, v| cfg.animation_speed = v,
        );

        // Store widget refs
        *animation_widgets_out.borrow_mut() = Some(AnimationWidgets {
            enable_check,
            speed_spin,
        });

        page
    }

    fn refresh_all_widgets(&self) {
        // Clone config to avoid holding a borrow while setting widget values
        // (setting values triggers callbacks that need to borrow_mut the config)
        let config = self.config.borrow().clone();

        // Update Theme widgets
        if let Some(ref widgets) = *self.theme_widgets.borrow() {
            widgets
                .theme_color1_widget
                .set_color(config.frame.theme.color1);
            widgets
                .theme_color2_widget
                .set_color(config.frame.theme.color2);
            widgets
                .theme_color3_widget
                .set_color(config.frame.theme.color3);
            widgets
                .theme_color4_widget
                .set_color(config.frame.theme.color4);
            widgets
                .theme_gradient_editor
                .set_theme_config(config.frame.theme.clone());
            widgets
                .theme_gradient_editor
                .set_gradient_source_config(&config.frame.theme.gradient);
            widgets
                .font1_btn
                .set_label(&config.frame.theme.font1_family);
            widgets
                .font1_size_spin
                .set_value(config.frame.theme.font1_size);
            widgets
                .font2_btn
                .set_label(&config.frame.theme.font2_family);
            widgets
                .font2_size_spin
                .set_value(config.frame.theme.font2_size);
        }

        // Update Frame widgets
        if let Some(ref widgets) = *self.frame_widgets.borrow() {
            widgets
                .border_style_dropdown
                .set_selected(match config.frame.border_style {
                    BorderStyle::Sunburst => 0,
                    BorderStyle::Chevron => 1,
                    BorderStyle::Stepped => 2,
                    BorderStyle::Geometric => 3,
                    BorderStyle::Ornate => 4,
                });
            widgets
                .border_width_spin
                .set_value(config.frame.border_width);
            widgets
                .corner_style_dropdown
                .set_selected(match config.frame.corner_style {
                    CornerStyle::Fan => 0,
                    CornerStyle::Ziggurat => 1,
                    CornerStyle::Diamond => 2,
                    CornerStyle::Bracket => 3,
                    CornerStyle::Hexagon => 4,
                    CornerStyle::Octagon => 5,
                    CornerStyle::Circle => 6,
                    CornerStyle::DoubleBracket => 7,
                    CornerStyle::GeometricStack => 8,
                    CornerStyle::None => 9,
                });
            widgets.corner_size_spin.set_value(config.frame.corner_size);
            widgets
                .accent_width_spin
                .set_value(config.frame.accent_width);
        }

        // Update Background widgets
        if let Some(ref widgets) = *self.background_widgets.borrow() {
            widgets
                .pattern_dropdown
                .set_selected(match config.frame.background_pattern {
                    BackgroundPattern::Solid => 0,
                    BackgroundPattern::VerticalLines => 1,
                    BackgroundPattern::DiamondGrid => 2,
                    BackgroundPattern::Sunburst => 3,
                    BackgroundPattern::Chevrons => 4,
                });
            widgets
                .pattern_spacing_spin
                .set_value(config.frame.pattern_spacing);
            widgets
                .sunburst_rays_spin
                .set_value(config.frame.sunburst_rays as f64);
        }

        // Update Header widgets
        if let Some(ref widgets) = *self.header_widgets.borrow() {
            widgets
                .show_header_check
                .set_active(config.frame.show_header);
            widgets
                .header_text_entry
                .set_text(&config.frame.header_text);
            widgets
                .header_style_dropdown
                .set_selected(match config.frame.header_style {
                    HeaderStyle::Centered => 0,
                    HeaderStyle::Banner => 1,
                    HeaderStyle::Stepped => 2,
                    HeaderStyle::None => 3,
                });
        }

        // Update Layout widgets
        if let Some(ref widgets) = *self.layout_widgets.borrow() {
            widgets
                .split_orientation_dropdown
                .set_selected(match config.frame.split_orientation {
                    SplitOrientation::Horizontal => 0,
                    SplitOrientation::Vertical => 1,
                });
            widgets
                .content_padding_spin
                .set_value(config.frame.content_padding);
            widgets
                .divider_style_dropdown
                .set_selected(match config.frame.divider_style {
                    DividerStyle::Chevron => 0,
                    DividerStyle::DoubleLine => 1,
                    DividerStyle::Line => 2,
                    DividerStyle::Stepped => 3,
                    DividerStyle::DiamondCluster => 4,
                    DividerStyle::Crescent => 5,
                    DividerStyle::ArrowDiamond => 6,
                    DividerStyle::CircleChain => 7,
                    DividerStyle::CrossedLines => 8,
                    DividerStyle::FleurDeLis => 9,
                    DividerStyle::Heartbeat => 10,
                    DividerStyle::DiamondGrid => 11,
                    DividerStyle::None => 12,
                });
            widgets
                .divider_width_spin
                .set_value(config.frame.divider_width);
            widgets
                .divider_padding_spin
                .set_value(config.frame.divider_padding);

            combo_config_base::rebuild_combined_group_settings(
                &widgets.group_settings_box,
                &self.config,
                |c: &mut ArtDecoDisplayConfig| &mut c.frame,
                &self.on_change,
                &self.preview,
            );
        }

        // Update Animation widgets
        if let Some(ref widgets) = *self.animation_widgets.borrow() {
            widgets.enable_check.set_active(config.animation_enabled);
            widgets.speed_spin.set_value(config.animation_speed);
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

        // Update Theme Reference section with new theme colors
        combo_config_base::refresh_theme_refs(&self.theme_ref_refreshers);

        self.preview.queue_draw();
    }

    pub fn set_source_summaries(&self, summaries: Vec<(String, String, usize, u32)>) {
        // Extract group configuration from summaries
        let mut group_item_counts: std::collections::HashMap<usize, u32> =
            std::collections::HashMap::new();
        for (_, _, group_num, item_idx) in &summaries {
            let current_max = group_item_counts.entry(*group_num).or_insert(0);
            if *item_idx > *current_max {
                *current_max = *item_idx;
            }
        }

        let mut group_nums: Vec<usize> = group_item_counts.keys().cloned().collect();
        group_nums.sort();
        let group_counts: Vec<usize> = group_nums
            .iter()
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

        // Rebuild combined group settings in Layout tab
        if let Some(widgets) = self.layout_widgets.borrow().as_ref() {
            combo_config_base::rebuild_combined_group_settings(
                &widgets.group_settings_box,
                &self.config,
                |c: &mut ArtDecoDisplayConfig| &mut c.frame,
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

    /// Extract transferable configuration that can be applied to another combo panel type.
    pub fn get_transferable_config(&self) -> crate::ui::combo_config_base::TransferableComboConfig {
        let config = self.config.borrow();
        crate::ui::combo_config_base::TransferableComboConfig {
            group_count: config.frame.group_count,
            group_item_counts: config
                .frame
                .group_item_counts
                .iter()
                .map(|&x| x as u32)
                .collect(),
            group_size_weights: config.frame.group_size_weights.clone(),
            group_item_orientations: config.frame.group_item_orientations.clone(),
            layout_orientation: config.frame.split_orientation,
            content_items: config.frame.content_items.clone(),
            content_padding: config.frame.content_padding,
            item_spacing: 8.0, // Not configurable in Art Deco, use default
            animation_enabled: config.animation_enabled,
            animation_speed: config.animation_speed,
        }
    }

    /// Apply transferable configuration from another combo panel.
    pub fn apply_transferable_config(
        &self,
        transfer: &crate::ui::combo_config_base::TransferableComboConfig,
    ) {
        {
            let mut config = self.config.borrow_mut();
            config.frame.group_count = transfer.group_count;
            config.frame.group_item_counts = transfer
                .group_item_counts
                .iter()
                .map(|&x| x as usize)
                .collect();
            config.frame.group_size_weights = transfer.group_size_weights.clone();
            config.frame.group_item_orientations = transfer.group_item_orientations.clone();
            config.frame.split_orientation = transfer.layout_orientation;
            config.frame.content_items = transfer.content_items.clone();
            config.frame.content_padding = transfer.content_padding;
            // item_spacing not configurable in Art Deco
            config.animation_enabled = transfer.animation_enabled;
            config.animation_speed = transfer.animation_speed;
        }
        self.preview.queue_draw();
    }

    fn rebuild_content_tabs(
        config: &Rc<RefCell<ArtDecoDisplayConfig>>,
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

    /// Cleanup method to break reference cycles and allow garbage collection.
    pub fn cleanup(&self) {
        log::debug!("ArtDecoConfigWidget::cleanup() - breaking reference cycles");
        combo_config_base::cleanup_common_fields(&self.on_change, &self.theme_ref_refreshers);
        *self.theme_widgets.borrow_mut() = None;
        *self.frame_widgets.borrow_mut() = None;
        *self.background_widgets.borrow_mut() = None;
        *self.header_widgets.borrow_mut() = None;
        *self.layout_widgets.borrow_mut() = None;
        *self.animation_widgets.borrow_mut() = None;
    }
}
