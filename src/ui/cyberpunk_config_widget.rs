//! Cyberpunk HUD configuration widget
//!
//! Provides a tabbed interface for configuring all aspects of the Cyberpunk display.

use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, CheckButton, DrawingArea, DropDown, Entry, Label, Notebook, Orientation, Scale,
    ScrolledWindow, SpinButton, StringList,
};
use std::cell::RefCell;
use std::rc::Rc;

use crate::core::FieldMetadata;
use crate::displayers::CyberpunkDisplayConfig;
use crate::ui::combo_config_base;
use crate::ui::cyberpunk_display::{CornerStyle, DividerStyle, HeaderStyle};
use crate::ui::lcars_display::SplitOrientation;
use crate::ui::theme::ColorSource;
use crate::ui::theme_color_selector::ThemeColorSelector;
use crate::ui::theme_font_selector::ThemeFontSelector;
use crate::ui::widget_builder::{
    create_section_header, ConfigWidgetBuilder, ConfigWidgetBuilderThemeSelectorExt,
};

/// Holds references to Frame tab widgets for updating when config changes
struct FrameWidgets {
    border_width_spin: SpinButton,
    border_color_widget: Rc<ThemeColorSelector>,
    glow_spin: SpinButton,
    corner_style_dropdown: DropDown,
    corner_size_spin: SpinButton,
    bg_color_widget: Rc<ThemeColorSelector>,
    padding_spin: SpinButton,
}

/// Holds references to Effects tab widgets
struct EffectsWidgets {
    show_grid_check: CheckButton,
    grid_color_widget: Rc<ThemeColorSelector>,
    grid_spacing_spin: SpinButton,
    show_scanlines_check: CheckButton,
    scanline_opacity_spin: SpinButton,
    item_frame_check: CheckButton,
    item_frame_color_widget: Rc<ThemeColorSelector>,
    item_glow_check: CheckButton,
}

/// Holds references to Header tab widgets
struct HeaderWidgets {
    show_header_check: CheckButton,
    header_text_entry: Entry,
    header_style_dropdown: DropDown,
    header_color_widget: Rc<ThemeColorSelector>,
    header_font_selector: Rc<ThemeFontSelector>,
}

/// Holds references to Layout tab widgets
struct LayoutWidgets {
    orientation_dropdown: DropDown,
    divider_style_dropdown: DropDown,
    divider_color_widget: Rc<ThemeColorSelector>,
    divider_width_spin: SpinButton,
    divider_padding_spin: SpinButton,
    group_settings_box: GtkBox,
}

/// Holds references to Animation tab widgets
struct AnimationWidgets {
    enable_check: CheckButton,
    speed_scale: Scale,
}

/// Holds references to Theme tab widgets
#[allow(dead_code)]
struct ThemeWidgets {
    common: combo_config_base::CommonThemeWidgets,
}

/// Cyberpunk configuration widget
pub struct CyberpunkConfigWidget {
    container: GtkBox,
    config: Rc<RefCell<CyberpunkDisplayConfig>>,
    on_change: Rc<RefCell<Option<Box<dyn Fn()>>>>,
    preview: DrawingArea,
    /// Notebook for per-slot content configuration
    content_notebook: Rc<RefCell<Notebook>>,
    /// Source summaries for labeling tabs (slot_name, summary, group_num, item_idx)
    source_summaries: Rc<RefCell<Vec<(String, String, usize, u32)>>>,
    /// Available fields from the source for text overlay configuration
    available_fields: Rc<RefCell<Vec<FieldMetadata>>>,
    /// Frame tab widgets
    frame_widgets: Rc<RefCell<Option<FrameWidgets>>>,
    /// Effects tab widgets
    effects_widgets: Rc<RefCell<Option<EffectsWidgets>>>,
    /// Header tab widgets
    header_widgets: Rc<RefCell<Option<HeaderWidgets>>>,
    /// Layout tab widgets
    layout_widgets: Rc<RefCell<Option<LayoutWidgets>>>,
    /// Animation tab widgets
    animation_widgets: Rc<RefCell<Option<AnimationWidgets>>>,
    /// Theme tab widgets
    #[allow(dead_code)]
    theme_widgets: Rc<RefCell<Option<ThemeWidgets>>>,
    /// Callbacks to refresh theme reference sections
    #[allow(dead_code)] // Kept for Rc ownership; callbacks are invoked via clones
    theme_ref_refreshers: Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
    /// Cleanup callbacks for Lazy*ConfigWidget instances in content tabs
    content_cleanup_callbacks: Rc<RefCell<Vec<combo_config_base::CleanupCallback>>>,
}

impl CyberpunkConfigWidget {
    pub fn new(available_fields: Vec<FieldMetadata>) -> Self {
        log::info!(
            "=== CyberpunkConfigWidget::new() called with {} fields ===",
            available_fields.len()
        );
        let container = GtkBox::new(Orientation::Vertical, 12);
        let config = Rc::new(RefCell::new(CyberpunkDisplayConfig::default()));
        let on_change: Rc<RefCell<Option<Box<dyn Fn()>>>> = Rc::new(RefCell::new(None));
        let source_summaries: Rc<RefCell<Vec<(String, String, usize, u32)>>> =
            Rc::new(RefCell::new(Vec::new()));
        let available_fields: Rc<RefCell<Vec<FieldMetadata>>> =
            Rc::new(RefCell::new(available_fields));
        let frame_widgets: Rc<RefCell<Option<FrameWidgets>>> = Rc::new(RefCell::new(None));
        let effects_widgets: Rc<RefCell<Option<EffectsWidgets>>> = Rc::new(RefCell::new(None));
        let header_widgets: Rc<RefCell<Option<HeaderWidgets>>> = Rc::new(RefCell::new(None));
        let layout_widgets: Rc<RefCell<Option<LayoutWidgets>>> = Rc::new(RefCell::new(None));
        let animation_widgets: Rc<RefCell<Option<AnimationWidgets>>> = Rc::new(RefCell::new(None));
        let theme_widgets: Rc<RefCell<Option<ThemeWidgets>>> = Rc::new(RefCell::new(None));
        let theme_ref_refreshers: Rc<RefCell<Vec<Rc<dyn Fn()>>>> =
            Rc::new(RefCell::new(Vec::new()));
        let content_cleanup_callbacks: Rc<RefCell<Vec<combo_config_base::CleanupCallback>>> =
            Rc::new(RefCell::new(Vec::new()));

        // Preview at the top
        let preview = DrawingArea::new();
        preview.set_content_height(200); // Max height 200px
        preview.set_content_width(200); // Min width 200px
        preview.set_hexpand(true);
        preview.set_halign(gtk4::Align::Fill);
        preview.set_vexpand(false);

        // Theme reference section - placed under preview for easy access from all tabs
        let (theme_ref_section, main_theme_refresh_cb) =
            combo_config_base::create_theme_reference_section(&config, |cfg| {
                cfg.frame.theme.clone()
            });
        theme_ref_refreshers
            .borrow_mut()
            .push(main_theme_refresh_cb);

        // Create notebook for tabbed interface
        let notebook = Notebook::new();
        notebook.set_vexpand(true);

        // Tab 1: Theme (first for easy access to theme colors/fonts)
        let theme_page = Self::create_theme_page(
            &config,
            &on_change,
            &preview,
            &theme_widgets,
            &theme_ref_refreshers,
        );
        notebook.append_page(&theme_page, Some(&Label::new(Some("Theme"))));

        // Tab 2: Frame
        let frame_page = Self::create_frame_page(
            &config,
            &on_change,
            &preview,
            &frame_widgets,
            &theme_ref_refreshers,
        );
        notebook.append_page(&frame_page, Some(&Label::new(Some("Frame"))));

        // Tab 3: Effects
        let effects_page = Self::create_effects_page(
            &config,
            &on_change,
            &preview,
            &effects_widgets,
            &theme_ref_refreshers,
        );
        notebook.append_page(&effects_page, Some(&Label::new(Some("Effects"))));

        // Tab 4: Header
        let header_page = Self::create_header_page(
            &config,
            &on_change,
            &preview,
            &header_widgets,
            &theme_ref_refreshers,
        );
        notebook.append_page(&header_page, Some(&Label::new(Some("Header"))));

        // Tab 5: Layout
        let layout_page = Self::create_layout_page(
            &config,
            &on_change,
            &preview,
            &layout_widgets,
            &theme_ref_refreshers,
        );
        notebook.append_page(&layout_page, Some(&Label::new(Some("Layout"))));

        // Tab 6: Content - with dynamic per-slot notebook
        let content_notebook = Rc::new(RefCell::new(Notebook::new()));
        let content_page = Self::create_content_page(
            &config,
            &on_change,
            &preview,
            &content_notebook,
            &source_summaries,
            &available_fields,
            &theme_ref_refreshers,
            &content_cleanup_callbacks,
        );
        notebook.append_page(&content_page, Some(&Label::new(Some("Content"))));

        // Tab 7: Animation
        let animation_page = Self::create_animation_page(&config, &on_change, &animation_widgets);
        notebook.append_page(&animation_page, Some(&Label::new(Some("Animation"))));

        preview.set_visible(false); container.append(&preview);
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
            frame_widgets,
            effects_widgets,
            header_widgets,
            layout_widgets,
            animation_widgets,
            theme_widgets,
            theme_ref_refreshers,
            content_cleanup_callbacks,
        }
    }

    fn create_frame_page(
        config: &Rc<RefCell<CyberpunkDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        frame_widgets_out: &Rc<RefCell<Option<FrameWidgets>>>,
        theme_ref_refreshers: &Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        combo_config_base::set_page_margins(&page);

        let builder = ConfigWidgetBuilder::new(config, preview, on_change);

        // Border width
        let border_width_spin = builder.spin_row(
            &page,
            "Border Width:",
            0.5,
            10.0,
            0.5,
            config.borrow().frame.border_width,
            |cfg, val| cfg.frame.border_width = val,
        );

        // Border color (theme-aware)
        let border_color_widget = builder.theme_color_selector_row(
            &page,
            "Border Color:",
            config.borrow().frame.border_color.clone(),
            config.borrow().frame.theme.clone(),
            |cfg, source| cfg.frame.border_color = source,
        );

        // Glow intensity
        let glow_spin = builder.spin_row(
            &page,
            "Glow Intensity:",
            0.0,
            1.0,
            0.1,
            config.borrow().frame.glow_intensity,
            |cfg, val| cfg.frame.glow_intensity = val,
        );
        glow_spin.set_digits(2);

        // Corner style dropdown
        let corner_idx = match config.borrow().frame.corner_style {
            CornerStyle::Chamfer => 0,
            CornerStyle::Bracket => 1,
            CornerStyle::Angular => 2,
        };
        let corner_style_dropdown = builder.dropdown_row(
            &page,
            "Corner Style:",
            &["Chamfer (45°)", "Bracket [ ]", "Angular"],
            corner_idx,
            |cfg, idx| {
                cfg.frame.corner_style = match idx {
                    0 => CornerStyle::Chamfer,
                    1 => CornerStyle::Bracket,
                    _ => CornerStyle::Angular,
                };
            },
        );

        // Corner size
        let corner_size_spin = builder.spin_row(
            &page,
            "Corner Size:",
            4.0,
            50.0,
            2.0,
            config.borrow().frame.corner_size,
            |cfg, val| cfg.frame.corner_size = val,
        );

        // Background color (theme-aware)
        let bg_color_widget = builder.theme_color_selector_row(
            &page,
            "Background:",
            config.borrow().frame.background_color.clone(),
            config.borrow().frame.theme.clone(),
            |cfg, source| cfg.frame.background_color = source,
        );

        // Content padding
        let padding_spin = builder.spin_row(
            &page,
            "Content Padding:",
            0.0,
            50.0,
            2.0,
            config.borrow().frame.content_padding,
            |cfg, val| cfg.frame.content_padding = val,
        );

        // Store widget refs
        *frame_widgets_out.borrow_mut() = Some(FrameWidgets {
            border_width_spin,
            border_color_widget: border_color_widget.clone(),
            glow_spin,
            corner_style_dropdown,
            corner_size_spin,
            bg_color_widget: bg_color_widget.clone(),
            padding_spin,
        });

        // Register theme refresh callbacks
        let border_color_for_refresh = border_color_widget.clone();
        let bg_color_for_refresh = bg_color_widget.clone();
        let config_for_refresh = config.clone();
        theme_ref_refreshers.borrow_mut().push(Rc::new(move || {
            let theme = config_for_refresh.borrow().frame.theme.clone();
            border_color_for_refresh.set_theme_config(theme.clone());
            bg_color_for_refresh.set_theme_config(theme);
        }));

        page
    }

    fn create_effects_page(
        config: &Rc<RefCell<CyberpunkDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        effects_widgets_out: &Rc<RefCell<Option<EffectsWidgets>>>,
        theme_ref_refreshers: &Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        combo_config_base::set_page_margins(&page);

        let builder = ConfigWidgetBuilder::new(config, preview, on_change);

        // Grid section
        page.append(&create_section_header("Grid Pattern"));

        let show_grid_check = builder.check_button(
            &page,
            "Show Grid",
            config.borrow().frame.show_grid,
            |cfg, active| cfg.frame.show_grid = active,
        );

        let grid_color_widget = builder.theme_color_selector_row(
            &page,
            "Grid Color:",
            config.borrow().frame.grid_color.clone(),
            config.borrow().frame.theme.clone(),
            |cfg, source| cfg.frame.grid_color = source,
        );

        let grid_spacing_spin = builder.spin_row(
            &page,
            "Grid Spacing:",
            5.0,
            100.0,
            5.0,
            config.borrow().frame.grid_spacing,
            |cfg, val| cfg.frame.grid_spacing = val,
        );

        // Scanlines section
        let scanline_label = create_section_header("Scanlines (CRT Effect)");
        scanline_label.set_margin_top(12);
        page.append(&scanline_label);

        let show_scanlines_check = builder.check_button(
            &page,
            "Show Scanlines",
            config.borrow().frame.show_scanlines,
            |cfg, active| cfg.frame.show_scanlines = active,
        );

        let scanline_opacity_spin = builder.spin_row(
            &page,
            "Opacity:",
            0.0,
            0.5,
            0.02,
            config.borrow().frame.scanline_opacity,
            |cfg, val| cfg.frame.scanline_opacity = val,
        );
        scanline_opacity_spin.set_digits(2);

        // Item frames section
        let item_frame_label = create_section_header("Content Item Frames");
        item_frame_label.set_margin_top(12);
        page.append(&item_frame_label);

        let item_frame_check = builder.check_button(
            &page,
            "Show Item Frames",
            config.borrow().frame.item_frame_enabled,
            |cfg, active| cfg.frame.item_frame_enabled = active,
        );

        let item_frame_color_widget = builder.theme_color_selector_row(
            &page,
            "Frame Color:",
            config.borrow().frame.item_frame_color.clone(),
            config.borrow().frame.theme.clone(),
            |cfg, source| cfg.frame.item_frame_color = source,
        );

        let item_glow_check = builder.check_button(
            &page,
            "Item Frame Glow",
            config.borrow().frame.item_glow_enabled,
            |cfg, active| cfg.frame.item_glow_enabled = active,
        );

        // Store widget refs
        *effects_widgets_out.borrow_mut() = Some(EffectsWidgets {
            show_grid_check,
            grid_color_widget: grid_color_widget.clone(),
            grid_spacing_spin,
            show_scanlines_check,
            scanline_opacity_spin,
            item_frame_check,
            item_frame_color_widget: item_frame_color_widget.clone(),
            item_glow_check,
        });

        // Register theme refresh callbacks
        let grid_color_for_refresh = grid_color_widget.clone();
        let item_frame_color_for_refresh = item_frame_color_widget.clone();
        let config_for_refresh = config.clone();
        theme_ref_refreshers.borrow_mut().push(Rc::new(move || {
            let theme = config_for_refresh.borrow().frame.theme.clone();
            grid_color_for_refresh.set_theme_config(theme.clone());
            item_frame_color_for_refresh.set_theme_config(theme);
        }));

        page
    }

    fn create_header_page(
        config: &Rc<RefCell<CyberpunkDisplayConfig>>,
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
            &page,
            "Show Header",
            config.borrow().frame.show_header,
            |cfg, active| cfg.frame.show_header = active,
        );

        // Header text entry
        let header_text_entry = builder.entry_row(
            &page,
            "Header Text:",
            &config.borrow().frame.header_text,
            |cfg, text| cfg.frame.header_text = text,
        );

        // Header style dropdown
        let style_idx = match config.borrow().frame.header_style {
            HeaderStyle::Brackets => 0,
            HeaderStyle::Underline => 1,
            HeaderStyle::Box => 2,
            HeaderStyle::None => 3,
        };
        let header_style_dropdown = builder.dropdown_row(
            &page,
            "Style:",
            &["Brackets", "Underline", "Box", "None"],
            style_idx,
            |cfg, idx| {
                cfg.frame.header_style = match idx {
                    0 => HeaderStyle::Brackets,
                    1 => HeaderStyle::Underline,
                    2 => HeaderStyle::Box,
                    _ => HeaderStyle::None,
                };
            },
        );

        // Header color (theme-aware)
        let header_color_widget = builder.theme_color_selector_row(
            &page,
            "Text Color:",
            config.borrow().frame.header_color.clone(),
            config.borrow().frame.theme.clone(),
            |cfg, source| cfg.frame.header_color = source,
        );

        // Header font (theme-aware)
        let header_font_selector = builder.theme_font_selector_row(
            &page,
            "Font:",
            config.borrow().frame.header_font.clone(),
            config.borrow().frame.theme.clone(),
            |cfg, source| cfg.frame.header_font = source,
        );

        // Store widget refs
        *header_widgets_out.borrow_mut() = Some(HeaderWidgets {
            show_header_check,
            header_text_entry,
            header_style_dropdown,
            header_color_widget: header_color_widget.clone(),
            header_font_selector: header_font_selector.clone(),
        });

        // Register theme refresh callbacks
        let header_color_for_refresh = header_color_widget.clone();
        let header_font_for_refresh = header_font_selector.clone();
        let config_for_refresh = config.clone();
        theme_ref_refreshers.borrow_mut().push(Rc::new(move || {
            let theme = config_for_refresh.borrow().frame.theme.clone();
            header_color_for_refresh.set_theme_config(theme.clone());
            header_font_for_refresh.set_theme_config(theme);
        }));

        page
    }

    fn create_layout_page(
        config: &Rc<RefCell<CyberpunkDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        layout_widgets_out: &Rc<RefCell<Option<LayoutWidgets>>>,
        theme_ref_refreshers: &Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        combo_config_base::set_page_margins(&page);

        let builder = ConfigWidgetBuilder::with_theme_refreshers(
            config,
            preview,
            on_change,
            theme_ref_refreshers,
        );

        // Layout section
        page.append(&create_section_header("Layout"));

        let orient_idx = match config.borrow().frame.split_orientation {
            SplitOrientation::Vertical => 0,
            SplitOrientation::Horizontal => 1,
        };
        let orientation_dropdown = builder.dropdown_row(
            &page,
            "Split Direction:",
            &["Vertical", "Horizontal"],
            orient_idx,
            |cfg, idx| {
                cfg.frame.split_orientation = if idx == 0 {
                    SplitOrientation::Vertical
                } else {
                    SplitOrientation::Horizontal
                }
            },
        );

        // Dividers section
        let dividers_header = create_section_header("Dividers");
        dividers_header.set_margin_top(12);
        page.append(&dividers_header);

        let div_style_idx = match config.borrow().frame.divider_style {
            DividerStyle::Line => 0,
            DividerStyle::Dashed => 1,
            DividerStyle::Glow => 2,
            DividerStyle::Dots => 3,
            DividerStyle::None => 4,
        };
        let divider_style_dropdown = builder.dropdown_row(
            &page,
            "Style:",
            &["Line", "Dashed", "Glow", "Dots", "None"],
            div_style_idx,
            |cfg, idx| {
                cfg.frame.divider_style = match idx {
                    0 => DividerStyle::Line,
                    1 => DividerStyle::Dashed,
                    2 => DividerStyle::Glow,
                    3 => DividerStyle::Dots,
                    _ => DividerStyle::None,
                }
            },
        );

        let divider_color_widget = builder.theme_color_selector_row(
            &page,
            "Color:",
            config.borrow().frame.divider_color.clone(),
            config.borrow().frame.theme.clone(),
            |cfg, color| cfg.frame.divider_color = color,
        );
        // Register theme refresh callback
        let divider_color_for_theme = divider_color_widget.clone();
        let config_for_theme = config.clone();
        theme_ref_refreshers.borrow_mut().push(Rc::new(move || {
            divider_color_for_theme.set_theme_config(config_for_theme.borrow().frame.theme.clone());
        }));

        let divider_width_spin = builder.spin_row(
            &page,
            "Width:",
            0.5,
            5.0,
            0.5,
            config.borrow().frame.divider_width,
            |cfg, v| cfg.frame.divider_width = v,
        );
        let divider_padding_spin = builder.spin_row(
            &page,
            "Padding:",
            0.0,
            20.0,
            1.0,
            config.borrow().frame.divider_padding,
            |cfg, v| cfg.frame.divider_padding = v,
        );

        // Combined group settings section (weight + orientation per group)
        let group_settings_box = combo_config_base::create_combined_group_settings_section(&page);
        combo_config_base::rebuild_combined_group_settings(
            &group_settings_box,
            config,
            |c: &mut CyberpunkDisplayConfig| &mut c.frame,
            on_change,
            preview,
        );

        // Store widget refs
        *layout_widgets_out.borrow_mut() = Some(LayoutWidgets {
            orientation_dropdown,
            divider_style_dropdown,
            divider_color_widget,
            divider_width_spin,
            divider_padding_spin,
            group_settings_box,
        });

        page
    }

    /// Create the Theme configuration page
    fn create_theme_page(
        config: &Rc<RefCell<CyberpunkDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        _preview: &DrawingArea,
        theme_widgets_out: &Rc<RefCell<Option<ThemeWidgets>>>,
        theme_ref_refreshers: &Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        combo_config_base::set_page_margins(&page);

        let scroll = ScrolledWindow::new();
        scroll.set_vexpand(true);
        scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);

        let inner_box = GtkBox::new(Orientation::Vertical, 8);

        // Info label
        let info_label = Label::new(Some("Configure theme colors, gradient, and fonts.\nThese can be referenced in content items for consistent styling."));
        info_label.set_halign(gtk4::Align::Start);
        info_label.add_css_class("dim-label");
        info_label.set_wrap(true);
        inner_box.append(&info_label);

        // Create common theme widgets using the shared helper
        let config_for_change = config.clone();
        let on_change_for_redraw = on_change.clone();
        let refreshers_for_redraw = theme_ref_refreshers.clone();

        let common = combo_config_base::create_common_theme_widgets(
            &inner_box,
            &config.borrow().frame.theme,
            move |mutator| {
                mutator(&mut config_for_change.borrow_mut().frame.theme);
            },
            move || {
                combo_config_base::notify_change(&on_change_for_redraw);
                combo_config_base::refresh_theme_refs(&refreshers_for_redraw);
            },
        );

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
            "Cyberpunk (Default)",
            "Synthwave",
            "LCARS",
            "Industrial",
            "Material",
            "Retro Terminal",
            "Fighter HUD",
        ]);
        let preset_dropdown = DropDown::new(Some(preset_list), None::<gtk4::Expression>);
        preset_dropdown.set_hexpand(true);
        preset_dropdown.set_selected(gtk4::INVALID_LIST_POSITION);

        let config_for_preset = config.clone();
        let on_change_for_preset = on_change.clone();
        let refreshers_for_preset = theme_ref_refreshers.clone();
        let common_for_preset = common.clone();
        preset_dropdown.connect_selected_notify(move |dropdown| {
            use crate::ui::theme::ComboThemeConfig;
            let theme = match dropdown.selected() {
                0 => ComboThemeConfig::default_for_cyberpunk(),
                1 => ComboThemeConfig::default_for_synthwave(),
                2 => ComboThemeConfig::default_for_lcars(),
                3 => ComboThemeConfig::default_for_industrial(),
                4 => ComboThemeConfig::default_for_material(),
                5 => ComboThemeConfig::default_for_retro_terminal(),
                6 => ComboThemeConfig::default_for_fighter_hud(),
                _ => return,
            };
            config_for_preset.borrow_mut().frame.theme = theme.clone();
            // Apply the complete theme preset using the common helper
            common_for_preset.apply_theme_preset(&theme);
            // Refresh all theme-linked widgets (theme reference section, etc.)
            combo_config_base::refresh_theme_refs(&refreshers_for_preset);
            combo_config_base::notify_change(&on_change_for_preset);
        });
        preset_row.append(&preset_dropdown);
        preset_box.append(&preset_row);
        preset_frame.set_child(Some(&preset_box));
        inner_box.append(&preset_frame);

        scroll.set_child(Some(&inner_box));
        page.append(&scroll);

        // Store theme widgets for later updates
        *theme_widgets_out.borrow_mut() = Some(ThemeWidgets { common });

        page
    }
    fn create_content_page(
        config: &Rc<RefCell<CyberpunkDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        content_notebook: &Rc<RefCell<Notebook>>,
        source_summaries: &Rc<RefCell<Vec<(String, String, usize, u32)>>>,
        available_fields: &Rc<RefCell<Vec<FieldMetadata>>>,
        theme_ref_refreshers: &Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
        content_cleanup_callbacks: &Rc<RefCell<Vec<combo_config_base::CleanupCallback>>>,
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

        // Initial build of content tabs
        drop(notebook);
        Self::rebuild_content_tabs(
            config,
            on_change,
            preview,
            content_notebook,
            source_summaries,
            available_fields,
            theme_ref_refreshers,
            content_cleanup_callbacks,
        );

        page
    }

    fn rebuild_content_tabs(
        config: &Rc<RefCell<CyberpunkDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        content_notebook: &Rc<RefCell<Notebook>>,
        source_summaries: &Rc<RefCell<Vec<(String, String, usize, u32)>>>,
        available_fields: &Rc<RefCell<Vec<FieldMetadata>>>,
        theme_ref_refreshers: &Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
        content_cleanup_callbacks: &Rc<RefCell<Vec<combo_config_base::CleanupCallback>>>,
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
            content_cleanup_callbacks,
        );
    }

    /// Default bar config with cyberpunk colors (cyan/magenta gradient)
    #[allow(dead_code)]
    #[allow(clippy::field_reassign_with_default)]
    fn default_bar_config_cyberpunk() -> crate::ui::BarDisplayConfig {
        use crate::ui::background::Color;
        use crate::ui::bar_display::{
            BarBackgroundType, BarDisplayConfig, BarFillDirection, BarFillType, BarOrientation,
            BarStyle, BorderConfig,
        };

        let mut config = BarDisplayConfig::default();
        config.style = BarStyle::Full;
        config.orientation = BarOrientation::Horizontal;
        config.fill_direction = BarFillDirection::LeftToRight;

        // Cyberpunk cyan to magenta gradient
        config.foreground = BarFillType::Gradient {
            stops: vec![
                crate::ui::theme::ColorStopSource::custom(
                    0.0,
                    Color {
                        r: 0.0,
                        g: 1.0,
                        b: 1.0,
                        a: 1.0,
                    },
                ), // Cyan
                crate::ui::theme::ColorStopSource::custom(
                    1.0,
                    Color {
                        r: 1.0,
                        g: 0.0,
                        b: 1.0,
                        a: 1.0,
                    },
                ), // Magenta
            ],
            angle: 0.0,
        };
        config.background = BarBackgroundType::Solid {
            color: crate::ui::theme::ColorSource::custom(Color {
                r: 0.1,
                g: 0.1,
                b: 0.15,
                a: 0.8,
            }), // Dark background
        };
        config.border = BorderConfig {
            enabled: true,
            color: crate::ui::theme::ColorSource::custom(Color {
                r: 0.0,
                g: 1.0,
                b: 1.0,
                a: 0.5,
            }), // Cyan border
            width: 1.0,
        };

        config
    }

    /// Default graph config with cyberpunk colors
    #[allow(dead_code)]
    #[allow(clippy::field_reassign_with_default)]
    fn default_graph_config_cyberpunk() -> crate::ui::GraphDisplayConfig {
        use crate::ui::background::Color;
        use crate::ui::graph_display::{FillMode, GraphDisplayConfig, GraphType, LineStyle};

        let mut config = GraphDisplayConfig::default();
        config.graph_type = GraphType::Line;
        config.line_style = LineStyle::Solid;
        config.line_width = 2.0;
        config.line_color = ColorSource::custom(Color {
            r: 0.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        }); // Cyan
        config.fill_mode = FillMode::Gradient;
        config.fill_gradient_start = ColorSource::custom(Color {
            r: 0.0,
            g: 1.0,
            b: 1.0,
            a: 0.4,
        }); // Cyan start
        config.fill_gradient_end = ColorSource::custom(Color {
            r: 0.0,
            g: 1.0,
            b: 1.0,
            a: 0.0,
        }); // Transparent end
        config.background_color = Color {
            r: 0.04,
            g: 0.06,
            b: 0.1,
            a: 0.8,
        };
        config.plot_background_color = Color {
            r: 0.04,
            g: 0.06,
            b: 0.1,
            a: 0.6,
        };
        config.x_axis.show_grid = true;
        config.x_axis.grid_color = ColorSource::custom(Color {
            r: 0.0,
            g: 0.5,
            b: 0.5,
            a: 0.3,
        }); // Cyan grid
        config.y_axis.show_grid = true;
        config.y_axis.grid_color = ColorSource::custom(Color {
            r: 0.0,
            g: 0.5,
            b: 0.5,
            a: 0.3,
        }); // Cyan grid

        config
    }

    /// Default core bars config with cyberpunk colors
    #[allow(dead_code)]
    #[allow(clippy::field_reassign_with_default)]
    fn default_core_bars_config_cyberpunk() -> crate::ui::CoreBarsConfig {
        use crate::ui::background::Color;
        use crate::ui::bar_display::{BarBackgroundType, BarFillType, BarStyle, BorderConfig};
        use crate::ui::core_bars_display::CoreBarsConfig;

        let mut config = CoreBarsConfig::default();
        config.bar_style = BarStyle::Full;

        // Cyberpunk cyan to yellow to magenta gradient
        config.foreground = BarFillType::Gradient {
            stops: vec![
                crate::ui::theme::ColorStopSource::custom(
                    0.0,
                    Color {
                        r: 0.0,
                        g: 1.0,
                        b: 1.0,
                        a: 1.0,
                    },
                ), // Cyan (low)
                crate::ui::theme::ColorStopSource::custom(
                    0.5,
                    Color {
                        r: 1.0,
                        g: 1.0,
                        b: 0.0,
                        a: 1.0,
                    },
                ), // Yellow (mid)
                crate::ui::theme::ColorStopSource::custom(
                    1.0,
                    Color {
                        r: 1.0,
                        g: 0.0,
                        b: 1.0,
                        a: 1.0,
                    },
                ), // Magenta (high)
            ],
            angle: 90.0,
        };
        config.background = BarBackgroundType::Solid {
            color: crate::ui::theme::ColorSource::custom(Color {
                r: 0.1,
                g: 0.1,
                b: 0.15,
                a: 0.6,
            }),
        };
        config.border = BorderConfig {
            enabled: true,
            color: crate::ui::theme::ColorSource::custom(Color {
                r: 0.0,
                g: 1.0,
                b: 1.0,
                a: 0.3,
            }),
            width: 1.0,
        };

        config
    }

    /// Default arc config with cyberpunk colors
    #[allow(dead_code)]
    #[allow(clippy::field_reassign_with_default)]
    fn default_arc_config_cyberpunk() -> crate::ui::ArcDisplayConfig {
        use crate::ui::arc_display::ArcDisplayConfig;
        use crate::ui::background::Color;
        use crate::ui::theme::{ColorSource, ColorStopSource};

        let mut config = ArcDisplayConfig::default();

        // Cyberpunk cyan to magenta gradient
        config.color_stops = vec![
            ColorStopSource {
                position: 0.0,
                color: ColorSource::Custom {
                    color: Color {
                        r: 0.0,
                        g: 1.0,
                        b: 1.0,
                        a: 1.0,
                    },
                },
            }, // Cyan
            ColorStopSource {
                position: 0.5,
                color: ColorSource::Custom {
                    color: Color {
                        r: 1.0,
                        g: 0.0,
                        b: 1.0,
                        a: 1.0,
                    },
                },
            }, // Magenta
            ColorStopSource {
                position: 1.0,
                color: ColorSource::Custom {
                    color: Color {
                        r: 1.0,
                        g: 1.0,
                        b: 0.0,
                        a: 1.0,
                    },
                },
            }, // Yellow
        ];
        config.show_background_arc = true;
        config.background_color = ColorSource::Custom {
            color: Color {
                r: 0.1,
                g: 0.1,
                b: 0.15,
                a: 0.6,
            },
        };
        config.animate = true;

        config
    }

    /// Default speedometer config with cyberpunk colors
    #[allow(dead_code)]
    #[allow(clippy::field_reassign_with_default)]
    fn default_speedometer_config_cyberpunk() -> crate::ui::SpeedometerConfig {
        use crate::ui::background::Color;
        use crate::ui::speedometer_display::SpeedometerConfig;
        use crate::ui::theme::{ColorSource, ColorStopSource};
        let mut config = SpeedometerConfig::default();

        // Cyberpunk colored track
        config.track_color = ColorSource::Custom {
            color: Color {
                r: 0.1,
                g: 0.1,
                b: 0.15,
                a: 0.6,
            },
        };
        config.track_color_stops = vec![
            ColorStopSource::custom(
                0.0,
                Color {
                    r: 0.0,
                    g: 1.0,
                    b: 1.0,
                    a: 1.0,
                },
            ), // Cyan (low)
            ColorStopSource::custom(
                0.6,
                Color {
                    r: 1.0,
                    g: 1.0,
                    b: 0.0,
                    a: 1.0,
                },
            ), // Yellow (mid)
            ColorStopSource::custom(
                1.0,
                Color {
                    r: 1.0,
                    g: 0.0,
                    b: 1.0,
                    a: 1.0,
                },
            ), // Magenta (high)
        ];

        // Cyberpunk tick colors
        config.major_tick_color = ColorSource::Custom {
            color: Color {
                r: 0.0,
                g: 1.0,
                b: 1.0,
                a: 0.8,
            },
        }; // Cyan
        config.minor_tick_color = ColorSource::Custom {
            color: Color {
                r: 0.0,
                g: 1.0,
                b: 1.0,
                a: 0.4,
            },
        }; // Dimmer cyan

        // Cyberpunk needle - magenta with glow effect
        config.needle_color = ColorSource::Custom {
            color: Color {
                r: 1.0,
                g: 0.0,
                b: 1.0,
                a: 1.0,
            },
        }; // Magenta

        // Center hub - dark with cyan accent
        config.center_hub_color = ColorSource::Custom {
            color: Color {
                r: 0.0,
                g: 0.4,
                b: 0.4,
                a: 1.0,
            },
        }; // Dark cyan
        config.center_hub_3d = true;

        config
    }

    fn create_animation_page(
        config: &Rc<RefCell<CyberpunkDisplayConfig>>,
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

    pub fn get_config(&self) -> CyberpunkDisplayConfig {
        self.config.borrow().clone()
    }

    /// Get a reference to the internal config Rc for use in callbacks
    pub fn get_config_rc(&self) -> Rc<RefCell<CyberpunkDisplayConfig>> {
        self.config.clone()
    }

    pub fn set_config(&self, config: &CyberpunkDisplayConfig) {
        log::debug!(
            "CyberpunkConfigWidget::set_config - loading {} groups, {} content_items",
            config.frame.group_count,
            config.frame.content_items.len()
        );
        for (slot_name, item_cfg) in &config.frame.content_items {
            log::debug!(
                "  loading content_item '{}': display_as={:?}",
                slot_name,
                item_cfg.display_as
            );
        }

        // IMPORTANT: Temporarily disable on_change callback to prevent signal cascade.
        // When we call set_value() on widgets, their signal handlers fire and call on_change.
        // This causes redundant updates since we're setting the config directly anyway.
        let saved_callback = self.on_change.borrow_mut().take();

        *self.config.borrow_mut() = config.clone();

        // Update Frame widgets
        if let Some(widgets) = self.frame_widgets.borrow().as_ref() {
            widgets
                .border_width_spin
                .set_value(config.frame.border_width);
            widgets
                .border_color_widget
                .set_source(config.frame.border_color.clone());
            widgets
                .border_color_widget
                .set_theme_config(config.frame.theme.clone());
            widgets.glow_spin.set_value(config.frame.glow_intensity);
            widgets
                .corner_style_dropdown
                .set_selected(match config.frame.corner_style {
                    CornerStyle::Chamfer => 0,
                    CornerStyle::Bracket => 1,
                    CornerStyle::Angular => 2,
                });
            widgets.corner_size_spin.set_value(config.frame.corner_size);
            widgets
                .bg_color_widget
                .set_source(config.frame.background_color.clone());
            widgets
                .bg_color_widget
                .set_theme_config(config.frame.theme.clone());
            widgets.padding_spin.set_value(config.frame.content_padding);
        }

        // Update Effects widgets
        if let Some(widgets) = self.effects_widgets.borrow().as_ref() {
            widgets.show_grid_check.set_active(config.frame.show_grid);
            widgets
                .grid_color_widget
                .set_source(config.frame.grid_color.clone());
            widgets
                .grid_color_widget
                .set_theme_config(config.frame.theme.clone());
            widgets
                .grid_spacing_spin
                .set_value(config.frame.grid_spacing);
            widgets
                .show_scanlines_check
                .set_active(config.frame.show_scanlines);
            widgets
                .scanline_opacity_spin
                .set_value(config.frame.scanline_opacity);
            widgets
                .item_frame_check
                .set_active(config.frame.item_frame_enabled);
            widgets
                .item_frame_color_widget
                .set_source(config.frame.item_frame_color.clone());
            widgets
                .item_frame_color_widget
                .set_theme_config(config.frame.theme.clone());
            widgets
                .item_glow_check
                .set_active(config.frame.item_glow_enabled);
        }

        // Update Header widgets
        if let Some(widgets) = self.header_widgets.borrow().as_ref() {
            widgets
                .show_header_check
                .set_active(config.frame.show_header);
            widgets
                .header_text_entry
                .set_text(&config.frame.header_text);
            widgets
                .header_style_dropdown
                .set_selected(match config.frame.header_style {
                    HeaderStyle::Brackets => 0,
                    HeaderStyle::Underline => 1,
                    HeaderStyle::Box => 2,
                    HeaderStyle::None => 3,
                });
            widgets
                .header_color_widget
                .set_source(config.frame.header_color.clone());
            widgets
                .header_color_widget
                .set_theme_config(config.frame.theme.clone());
            widgets
                .header_font_selector
                .set_source(config.frame.header_font.clone());
            widgets
                .header_font_selector
                .set_theme_config(config.frame.theme.clone());
        }

        // Update Layout widgets
        if let Some(widgets) = self.layout_widgets.borrow().as_ref() {
            widgets
                .orientation_dropdown
                .set_selected(match config.frame.split_orientation {
                    SplitOrientation::Vertical => 0,
                    SplitOrientation::Horizontal => 1,
                });
            widgets
                .divider_style_dropdown
                .set_selected(match config.frame.divider_style {
                    DividerStyle::Line => 0,
                    DividerStyle::Dashed => 1,
                    DividerStyle::Glow => 2,
                    DividerStyle::Dots => 3,
                    DividerStyle::None => 4,
                });
            widgets
                .divider_color_widget
                .set_source(config.frame.divider_color.clone());
            widgets
                .divider_color_widget
                .set_theme_config(config.frame.theme.clone());
            widgets
                .divider_width_spin
                .set_value(config.frame.divider_width);
            widgets
                .divider_padding_spin
                .set_value(config.frame.divider_padding);

            // Rebuild combined group settings
            combo_config_base::rebuild_combined_group_settings(
                &widgets.group_settings_box,
                &self.config,
                |c: &mut CyberpunkDisplayConfig| &mut c.frame,
                &self.on_change,
                &self.preview,
            );
        }

        // Update Animation widgets
        if let Some(widgets) = self.animation_widgets.borrow().as_ref() {
            widgets.enable_check.set_active(config.animation_enabled);
            widgets.speed_scale.set_value(config.animation_speed);
        }

        // Update Theme widgets (fonts and colors)
        if let Some(ref widgets) = *self.theme_widgets.borrow() {
            widgets
                .common
                .color1_widget
                .set_color(config.frame.theme.color1);
            widgets
                .common
                .color2_widget
                .set_color(config.frame.theme.color2);
            widgets
                .common
                .color3_widget
                .set_color(config.frame.theme.color3);
            widgets
                .common
                .color4_widget
                .set_color(config.frame.theme.color4);
            widgets
                .common
                .gradient_editor
                .set_theme_config(config.frame.theme.clone());
            widgets
                .common
                .gradient_editor
                .set_gradient_source_config(&config.frame.theme.gradient);
            widgets
                .common
                .font1_btn
                .set_label(&config.frame.theme.font1_family);
            widgets
                .common
                .font1_size_spin
                .set_value(config.frame.theme.font1_size);
            widgets
                .common
                .font2_btn
                .set_label(&config.frame.theme.font2_family);
            widgets
                .common
                .font2_size_spin
                .set_value(config.frame.theme.font2_size);
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
            &self.content_cleanup_callbacks,
        );

        // Restore the on_change callback now that widget updates are complete
        *self.on_change.borrow_mut() = saved_callback;

        // Update Theme Reference section with new theme colors
        combo_config_base::refresh_theme_refs(&self.theme_ref_refreshers);
    }

    pub fn set_source_summaries(&self, summaries: Vec<(String, String, usize, u32)>) {
        log::debug!(
            "CyberpunkConfigWidget::set_source_summaries - received {} summaries",
            summaries.len()
        );
        for (slot_name, _, group_num, item_idx) in &summaries {
            log::debug!(
                "  summary: slot='{}', group={}, item={}",
                slot_name,
                group_num,
                item_idx
            );
        }
        // Extract group configuration from summaries
        let mut group_item_counts: std::collections::HashMap<usize, u32> =
            std::collections::HashMap::new();
        for (_, _, group_num, item_idx) in &summaries {
            let current_max = group_item_counts.entry(*group_num).or_insert(0);
            if *item_idx > *current_max {
                *current_max = *item_idx;
            }
        }

        // Convert to sorted vec
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

            // Ensure group_size_weights has the right length
            while cfg.frame.group_size_weights.len() < new_group_count {
                cfg.frame.group_size_weights.push(1.0);
            }
            // Trim if we have fewer groups now
            cfg.frame.group_size_weights.truncate(new_group_count);
        }

        *self.source_summaries.borrow_mut() = summaries;

        // Rebuild combined group settings in Layout tab
        if let Some(widgets) = self.layout_widgets.borrow().as_ref() {
            combo_config_base::rebuild_combined_group_settings(
                &widgets.group_settings_box,
                &self.config,
                |c: &mut CyberpunkDisplayConfig| &mut c.frame,
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
            &self.content_cleanup_callbacks,
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
    /// This excludes theme-specific settings like colors, fonts, and frame styles.
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
            item_spacing: config.frame.grid_spacing,
            animation_enabled: config.animation_enabled,
            animation_speed: config.animation_speed,
        }
    }

    /// Apply transferable configuration from another combo panel.
    /// This preserves theme-specific settings while updating layout and content.
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
            config.frame.grid_spacing = transfer.item_spacing;
            config.animation_enabled = transfer.animation_enabled;
            config.animation_speed = transfer.animation_speed;
        }
    }

    /// Cleanup method to break reference cycles and allow garbage collection.
    pub fn cleanup(&self) {
        log::debug!("CyberpunkConfigWidget::cleanup() - breaking reference cycles");
        combo_config_base::cleanup_common_fields_with_content(
            &self.on_change,
            &self.theme_ref_refreshers,
            &self.content_cleanup_callbacks,
        );
        *self.frame_widgets.borrow_mut() = None;
        *self.effects_widgets.borrow_mut() = None;
        *self.header_widgets.borrow_mut() = None;
        *self.layout_widgets.borrow_mut() = None;
        *self.animation_widgets.borrow_mut() = None;
        *self.theme_widgets.borrow_mut() = None;
    }
}

impl Default for CyberpunkConfigWidget {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}
