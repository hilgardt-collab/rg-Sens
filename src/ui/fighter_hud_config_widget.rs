//! Fighter Jet HUD configuration widget
//!
//! Provides a tabbed interface for configuring all aspects of the Fighter HUD display.

use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, CheckButton, DrawingArea, DropDown, Entry, Label, Notebook, Orientation,
    Scale, SpinButton, StringList, ScrolledWindow,
};
use std::cell::RefCell;
use std::rc::Rc;

use crate::ui::shared_font_dialog::shared_font_dialog;
use crate::ui::color_button_widget::ColorButtonWidget;
use crate::ui::fighter_hud_display::{
    render_fighter_hud_frame, HudColorPreset, HudFrameStyle,
    HudHeaderStyle, HudDividerStyle,
};
use crate::ui::lcars_display::SplitOrientation;
use crate::ui::background::Color;
use crate::displayers::FighterHudDisplayConfig;
use crate::core::FieldMetadata;
use crate::ui::combo_config_base;
use crate::ui::widget_builder::{ConfigWidgetBuilder, create_section_header};
use crate::ui::theme::FontSource;
use crate::ui::theme_color_selector::ThemeColorSelector;
use crate::ui::theme_font_selector::ThemeFontSelector;

/// Holds references to Frame tab widgets
struct FrameWidgets {
    style_dropdown: DropDown,
    line_width_spin: SpinButton,
    bracket_size_spin: SpinButton,
    bracket_thickness_spin: SpinButton,
    show_reticle_check: CheckButton,
    reticle_size_spin: SpinButton,
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
    tick_spacing_spin: SpinButton,
    group_weights_box: GtkBox,
    item_orientations_box: GtkBox,
}

/// Holds references to Animation tab widgets
struct AnimationWidgets {
    enable_check: CheckButton,
    speed_spin: SpinButton,
    scan_line_check: CheckButton,
}

/// Holds references to Theme tab widgets
#[allow(dead_code)]
struct ThemeWidgets {
    // HUD scheme widgets
    hud_color_dropdown: DropDown,
    custom_hud_color_widget: Rc<ColorButtonWidget>,
    custom_color_row: GtkBox,
    background_widget: Rc<ColorButtonWidget>,
    glow_scale: Scale,
    // Theme color widgets
    theme_color1_widget: Rc<ColorButtonWidget>,
    theme_color2_widget: Rc<ColorButtonWidget>,
    theme_color3_widget: Rc<ColorButtonWidget>,
    theme_color4_widget: Rc<ColorButtonWidget>,
    theme_gradient_editor: Rc<crate::ui::gradient_editor::GradientEditor>,
    font1_btn: Button,
    font1_size_spin: SpinButton,
    font2_btn: Button,
    font2_size_spin: SpinButton,
}

/// Fighter HUD configuration widget
pub struct FighterHudConfigWidget {
    container: GtkBox,
    config: Rc<RefCell<FighterHudDisplayConfig>>,
    on_change: Rc<RefCell<Option<Box<dyn Fn()>>>>,
    preview: DrawingArea,
    content_notebook: Rc<RefCell<Notebook>>,
    source_summaries: Rc<RefCell<Vec<(String, String, usize, u32)>>>,
    available_fields: Rc<RefCell<Vec<FieldMetadata>>>,
    frame_widgets: Rc<RefCell<Option<FrameWidgets>>>,
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

impl FighterHudConfigWidget {
    pub fn new(available_fields: Vec<FieldMetadata>) -> Self {
        let container = GtkBox::new(Orientation::Vertical, 12);
        let config = Rc::new(RefCell::new(FighterHudDisplayConfig::default()));
        let on_change: Rc<RefCell<Option<Box<dyn Fn()>>>> = Rc::new(RefCell::new(None));
        let source_summaries: Rc<RefCell<Vec<(String, String, usize, u32)>>> = Rc::new(RefCell::new(Vec::new()));
        let available_fields: Rc<RefCell<Vec<FieldMetadata>>> = Rc::new(RefCell::new(available_fields));
        let frame_widgets: Rc<RefCell<Option<FrameWidgets>>> = Rc::new(RefCell::new(None));
        let header_widgets: Rc<RefCell<Option<HeaderWidgets>>> = Rc::new(RefCell::new(None));
        let layout_widgets: Rc<RefCell<Option<LayoutWidgets>>> = Rc::new(RefCell::new(None));
        let animation_widgets: Rc<RefCell<Option<AnimationWidgets>>> = Rc::new(RefCell::new(None));
        let theme_widgets: Rc<RefCell<Option<ThemeWidgets>>> = Rc::new(RefCell::new(None));
        let theme_ref_refreshers: Rc<RefCell<Vec<Rc<dyn Fn()>>>> = Rc::new(RefCell::new(Vec::new()));

        // Preview at the top
        let preview = DrawingArea::new();
        preview.set_content_height(180);
        preview.set_hexpand(true);
        preview.set_vexpand(false);

        let config_clone = config.clone();
        preview.set_draw_func(move |_, cr, width, height| {
            if width < 10 || height < 10 {
                return;
            }

            let cfg = config_clone.borrow();
            let _ = render_fighter_hud_frame(cr, &cfg.frame, width as f64, height as f64);
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

        // Tab 1: Theme (includes HUD color scheme, theme colors, and fonts)
        let theme_page = Self::create_theme_page(&config, &on_change, &preview, &theme_widgets, &theme_ref_refreshers);
        notebook.append_page(&theme_page, Some(&Label::new(Some("Theme"))));

        // Tab 2: Frame
        let frame_page = Self::create_frame_page(&config, &on_change, &preview, &frame_widgets, &theme_ref_refreshers);
        notebook.append_page(&frame_page, Some(&Label::new(Some("Frame"))));

        // Tab 4: Header
        let header_page = Self::create_header_page(&config, &on_change, &preview, &header_widgets, &theme_ref_refreshers);
        notebook.append_page(&header_page, Some(&Label::new(Some("Header"))));

        // Tab 5: Layout
        let layout_page = Self::create_layout_page(&config, &on_change, &preview, &layout_widgets);
        notebook.append_page(&layout_page, Some(&Label::new(Some("Layout"))));

        // Tab 6: Content
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
            frame_widgets,
            header_widgets,
            layout_widgets,
            animation_widgets,
            theme_widgets,
            theme_ref_refreshers,
        }
    }

    fn create_frame_page(
        config: &Rc<RefCell<FighterHudDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        frame_widgets_out: &Rc<RefCell<Option<FrameWidgets>>>,
        theme_ref_refreshers: &Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        combo_config_base::set_page_margins(&page);

        let builder = ConfigWidgetBuilder::new(config, preview, on_change);

        let style_idx = match config.borrow().frame.frame_style {
            HudFrameStyle::CornerBrackets => 0,
            HudFrameStyle::TargetingReticle => 1,
            HudFrameStyle::TacticalBox => 2,
            HudFrameStyle::Minimal => 3,
            HudFrameStyle::None => 4,
        };
        let style_dropdown = builder.dropdown_row(
            &page, "Frame Style:", &["Corner Brackets", "Targeting Reticle", "Tactical Box", "Minimal", "None"], style_idx,
            |cfg, idx| cfg.frame.frame_style = match idx {
                0 => HudFrameStyle::CornerBrackets,
                1 => HudFrameStyle::TargetingReticle,
                2 => HudFrameStyle::TacticalBox,
                3 => HudFrameStyle::Minimal,
                _ => HudFrameStyle::None,
            },
        );

        let line_width_spin = builder.spin_row(
            &page, "Line Width:", 0.5, 5.0, 0.5, config.borrow().frame.line_width,
            |cfg, v| cfg.frame.line_width = v,
        );

        let bracket_size_spin = builder.spin_row(
            &page, "Bracket Size:", 10.0, 60.0, 2.0, config.borrow().frame.bracket_size,
            |cfg, v| cfg.frame.bracket_size = v,
        );

        let bracket_thickness_spin = builder.spin_row(
            &page, "Bracket Thickness:", 1.0, 6.0, 0.5, config.borrow().frame.bracket_thickness,
            |cfg, v| cfg.frame.bracket_thickness = v,
        );

        // Center reticle section
        let reticle_label = create_section_header("Center Reticle");
        reticle_label.set_margin_top(12);
        page.append(&reticle_label);

        let show_reticle_check = builder.check_button(
            &page, "Show Center Reticle", config.borrow().frame.show_center_reticle,
            |cfg, v| cfg.frame.show_center_reticle = v,
        );

        let reticle_size_spin = builder.spin_row(
            &page, "Reticle Size:", 0.05, 0.5, 0.05, config.borrow().frame.reticle_size,
            |cfg, v| cfg.frame.reticle_size = v,
        );

        // Reticle color (theme-aware)
        let reticle_color_box = GtkBox::new(Orientation::Horizontal, 6);
        reticle_color_box.append(&Label::new(Some("Reticle Color:")));
        let reticle_color_selector = ThemeColorSelector::new(config.borrow().frame.reticle_color.clone());
        reticle_color_selector.set_theme_config(config.borrow().frame.theme.clone());
        reticle_color_selector.widget().set_hexpand(true);
        reticle_color_box.append(reticle_color_selector.widget());
        page.append(&reticle_color_box);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        reticle_color_selector.set_on_change(move |color_source| {
            config_clone.borrow_mut().frame.reticle_color = color_source;
            combo_config_base::queue_redraw(&preview_clone, &on_change_clone);
        });

        let reticle_color_selector_rc = Rc::new(reticle_color_selector);
        let reticle_color_selector_for_refresh = reticle_color_selector_rc.clone();
        let config_for_reticle_refresh = config.clone();
        theme_ref_refreshers.borrow_mut().push(Rc::new(move || {
            reticle_color_selector_for_refresh.set_theme_config(config_for_reticle_refresh.borrow().frame.theme.clone());
        }));

        // Store widget refs
        *frame_widgets_out.borrow_mut() = Some(FrameWidgets {
            style_dropdown,
            line_width_spin,
            bracket_size_spin,
            bracket_thickness_spin,
            show_reticle_check,
            reticle_size_spin,
        });

        page
    }

    fn create_header_page(
        config: &Rc<RefCell<FighterHudDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        header_widgets_out: &Rc<RefCell<Option<HeaderWidgets>>>,
        theme_ref_refreshers: &Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        combo_config_base::set_page_margins(&page);

        // Show header
        let show_header_check = CheckButton::with_label("Show Header");
        show_header_check.set_active(config.borrow().frame.show_header);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        show_header_check.connect_toggled(move |check| {
            config_clone.borrow_mut().frame.show_header = check.is_active();
            combo_config_base::queue_redraw(&preview_clone, &on_change_clone);
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
            combo_config_base::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&text_box);

        // Header style
        let style_box = GtkBox::new(Orientation::Horizontal, 6);
        style_box.append(&Label::new(Some("Header Style:")));
        let style_list = StringList::new(&["Status Bar", "Mission Callout", "System ID", "None"]);
        let header_style_dropdown = DropDown::new(Some(style_list), None::<gtk4::Expression>);
        let style_idx = match config.borrow().frame.header_style {
            HudHeaderStyle::StatusBar => 0,
            HudHeaderStyle::MissionCallout => 1,
            HudHeaderStyle::SystemId => 2,
            HudHeaderStyle::None => 3,
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
                0 => HudHeaderStyle::StatusBar,
                1 => HudHeaderStyle::MissionCallout,
                2 => HudHeaderStyle::SystemId,
                _ => HudHeaderStyle::None,
            };
            combo_config_base::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&style_box);

        // Header height
        let height_box = GtkBox::new(Orientation::Horizontal, 6);
        height_box.append(&Label::new(Some("Header Height:")));
        let header_height_spin = SpinButton::with_range(16.0, 48.0, 2.0);
        header_height_spin.set_value(config.borrow().frame.header_height);
        header_height_spin.set_hexpand(true);
        height_box.append(&header_height_spin);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        header_height_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().frame.header_height = spin.value();
            combo_config_base::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&height_box);

        // Font section (theme-aware)
        let font_label = Label::new(Some("Font"));
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
            combo_config_base::queue_redraw(&preview_clone, &on_change_clone);
        });

        // Register theme refresh callback for header font
        let header_font_selector_for_theme = header_font_selector.clone();
        let config_for_header_theme = config.clone();
        let header_font_refresh: Rc<dyn Fn()> = Rc::new(move || {
            let theme = config_for_header_theme.borrow().frame.theme.clone();
            header_font_selector_for_theme.set_theme_config(theme);
        });
        theme_ref_refreshers.borrow_mut().push(header_font_refresh);

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
        config: &Rc<RefCell<FighterHudDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        layout_widgets_out: &Rc<RefCell<Option<LayoutWidgets>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        combo_config_base::set_page_margins(&page);

        let builder = ConfigWidgetBuilder::new(config, preview, on_change);

        let orient_idx = match config.borrow().frame.split_orientation {
            SplitOrientation::Vertical => 0,
            SplitOrientation::Horizontal => 1,
        };
        let split_orientation_dropdown = builder.dropdown_row(
            &page, "Split Orientation:", &["Vertical", "Horizontal"], orient_idx,
            |cfg, idx| cfg.frame.split_orientation = if idx == 0 { SplitOrientation::Vertical } else { SplitOrientation::Horizontal },
        );

        let content_padding_spin = builder.spin_row(
            &page, "Content Padding:", 4.0, 32.0, 2.0, config.borrow().frame.content_padding,
            |cfg, v| cfg.frame.content_padding = v,
        );

        // Divider section
        let divider_label = create_section_header("Dividers");
        divider_label.set_margin_top(12);
        page.append(&divider_label);

        let div_style_idx = match config.borrow().frame.divider_style {
            HudDividerStyle::TickLadder => 0,
            HudDividerStyle::ArrowLine => 1,
            HudDividerStyle::TacticalDash => 2,
            HudDividerStyle::Fade => 3,
            HudDividerStyle::None => 4,
        };
        let divider_style_dropdown = builder.dropdown_row(
            &page, "Divider Style:", &["Tick Ladder", "Arrow Line", "Tactical Dash", "Fade", "None"], div_style_idx,
            |cfg, idx| cfg.frame.divider_style = match idx {
                0 => HudDividerStyle::TickLadder,
                1 => HudDividerStyle::ArrowLine,
                2 => HudDividerStyle::TacticalDash,
                3 => HudDividerStyle::Fade,
                _ => HudDividerStyle::None,
            },
        );

        let divider_padding_spin = builder.spin_row(
            &page, "Divider Padding:", 2.0, 20.0, 1.0, config.borrow().frame.divider_padding,
            |cfg, v| cfg.frame.divider_padding = v,
        );

        let tick_spacing_spin = builder.spin_row(
            &page, "Tick Spacing:", 4.0, 20.0, 1.0, config.borrow().frame.tick_spacing,
            |cfg, v| cfg.frame.tick_spacing = v,
        );

        // Group weights section
        let weights_label = create_section_header("Group Size Weights");
        weights_label.set_margin_top(12);
        page.append(&weights_label);

        let group_weights_box = GtkBox::new(Orientation::Vertical, 4);
        page.append(&group_weights_box);

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
            &item_orientations_box,
            config,
            |c: &mut FighterHudDisplayConfig| &mut c.frame,
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
            tick_spacing_spin,
            group_weights_box,
            item_orientations_box,
        });

        page
    }

    /// Create the Theme configuration page
    fn create_theme_page(
        config: &Rc<RefCell<FighterHudDisplayConfig>>,
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
        let info_label = Label::new(Some("Configure HUD color scheme, theme colors, gradient, and fonts.\nThese can be referenced in content items for consistent styling."));
        info_label.set_halign(gtk4::Align::Start);
        info_label.add_css_class("dim-label");
        info_label.set_wrap(true);
        inner_box.append(&info_label);

        // === HUD Color Scheme Section ===
        let hud_scheme_frame = gtk4::Frame::new(Some("HUD Color Scheme"));
        let hud_scheme_box = GtkBox::new(Orientation::Vertical, 6);
        hud_scheme_box.set_margin_start(8);
        hud_scheme_box.set_margin_end(8);
        hud_scheme_box.set_margin_top(8);
        hud_scheme_box.set_margin_bottom(8);

        // HUD color preset dropdown
        let hud_color_row = GtkBox::new(Orientation::Horizontal, 8);
        hud_color_row.append(&Label::new(Some("HUD Color:")));
        let hud_color_list = StringList::new(&["Military Green", "Amber", "Cyan", "White", "Custom"]);
        let hud_color_dropdown = DropDown::new(Some(hud_color_list), None::<gtk4::Expression>);
        let hud_color_idx = match &config.borrow().frame.hud_color {
            HudColorPreset::MilitaryGreen => 0,
            HudColorPreset::Amber => 1,
            HudColorPreset::Cyan => 2,
            HudColorPreset::White => 3,
            HudColorPreset::Custom(_) => 4,
        };
        hud_color_dropdown.set_selected(hud_color_idx);
        hud_color_dropdown.set_hexpand(true);
        hud_color_row.append(&hud_color_dropdown);
        hud_scheme_box.append(&hud_color_row);

        // Custom color (shown only when Custom is selected)
        let custom_color_row = GtkBox::new(Orientation::Horizontal, 8);
        custom_color_row.append(&Label::new(Some("Custom Color:")));
        let custom_hud_color = if let HudColorPreset::Custom(c) = &config.borrow().frame.hud_color {
            *c
        } else {
            Color { r: 0.0, g: 0.9, b: 0.3, a: 1.0 }
        };
        let custom_hud_color_widget = Rc::new(ColorButtonWidget::new(custom_hud_color));
        custom_hud_color_widget.widget().set_hexpand(true);
        custom_color_row.append(custom_hud_color_widget.widget());
        custom_color_row.set_visible(hud_color_idx == 4);
        hud_scheme_box.append(&custom_color_row);

        // Background color
        let bg_row = GtkBox::new(Orientation::Horizontal, 8);
        bg_row.append(&Label::new(Some("Background:")));
        let background_widget = Rc::new(ColorButtonWidget::new(config.borrow().frame.background_color));
        background_widget.widget().set_hexpand(true);
        bg_row.append(background_widget.widget());
        hud_scheme_box.append(&bg_row);

        // Glow intensity
        let glow_row = GtkBox::new(Orientation::Horizontal, 8);
        glow_row.append(&Label::new(Some("Glow Intensity:")));
        let glow_scale = Scale::with_range(Orientation::Horizontal, 0.0, 1.0, 0.05);
        glow_scale.set_value(config.borrow().frame.glow_intensity);
        glow_scale.set_hexpand(true);
        glow_scale.set_draw_value(true);
        glow_row.append(&glow_scale);
        hud_scheme_box.append(&glow_row);

        hud_scheme_frame.set_child(Some(&hud_scheme_box));
        inner_box.append(&hud_scheme_frame);

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

        // === HUD Scheme Callbacks (now that color_widgets exist) ===

        // HUD color dropdown callback - updates hud_color and theme colors
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let custom_color_row_clone = custom_color_row.clone();
        let custom_hud_color_widget_clone = custom_hud_color_widget.clone();
        let color_widgets_for_hud = color_widgets.clone();
        let refreshers_for_hud = theme_ref_refreshers.clone();
        hud_color_dropdown.connect_selected_notify(move |dropdown| {
            let selected = dropdown.selected();
            if selected == gtk4::INVALID_LIST_POSITION {
                return;
            }
            custom_color_row_clone.set_visible(selected == 4);

            let preset = match selected {
                0 => HudColorPreset::MilitaryGreen,
                1 => HudColorPreset::Amber,
                2 => HudColorPreset::Cyan,
                3 => HudColorPreset::White,
                _ => HudColorPreset::Custom(custom_hud_color_widget_clone.color()),
            };

            // Update theme colors based on preset
            let (c1, c2, c3, c4) = preset.to_theme_colors();
            let mut cfg = config_clone.borrow_mut();
            cfg.frame.hud_color = preset;
            cfg.frame.theme.color1 = c1;
            cfg.frame.theme.color2 = c2;
            cfg.frame.theme.color3 = c3;
            cfg.frame.theme.color4 = c4;
            drop(cfg);

            // Update color widgets
            if color_widgets_for_hud.len() >= 4 {
                color_widgets_for_hud[0].set_color(c1);
                color_widgets_for_hud[1].set_color(c2);
                color_widgets_for_hud[2].set_color(c3);
                color_widgets_for_hud[3].set_color(c4);
            }

            combo_config_base::refresh_theme_refs(&refreshers_for_hud);
            combo_config_base::queue_redraw(&preview_clone, &on_change_clone);
        });

        // Custom HUD color callback
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let color_widgets_for_custom = color_widgets.clone();
        let refreshers_for_custom = theme_ref_refreshers.clone();
        custom_hud_color_widget.set_on_change(move |color| {
            let preset = HudColorPreset::Custom(color);
            let (c1, c2, c3, c4) = preset.to_theme_colors();
            let mut cfg = config_clone.borrow_mut();
            cfg.frame.hud_color = preset;
            cfg.frame.theme.color1 = c1;
            cfg.frame.theme.color2 = c2;
            cfg.frame.theme.color3 = c3;
            cfg.frame.theme.color4 = c4;
            drop(cfg);

            // Update color widgets
            if color_widgets_for_custom.len() >= 4 {
                color_widgets_for_custom[0].set_color(c1);
                color_widgets_for_custom[1].set_color(c2);
                color_widgets_for_custom[2].set_color(c3);
                color_widgets_for_custom[3].set_color(c4);
            }

            combo_config_base::refresh_theme_refs(&refreshers_for_custom);
            combo_config_base::queue_redraw(&preview_clone, &on_change_clone);
        });

        // Background color callback
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        background_widget.set_on_change(move |color| {
            config_clone.borrow_mut().frame.background_color = color;
            combo_config_base::queue_redraw(&preview_clone, &on_change_clone);
        });

        // Glow intensity callback
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        glow_scale.connect_value_changed(move |scale| {
            config_clone.borrow_mut().frame.glow_intensity = scale.value();
            combo_config_base::queue_redraw(&preview_clone, &on_change_clone);
        });

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
            combo_config_base::refresh_theme_refs(&refreshers_clone);
            combo_config_base::queue_redraw(&preview_clone, &on_change_clone);
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
                                .unwrap_or_else(|| "monospace".to_string());
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
            "Fighter HUD (Default)",
            "Cyberpunk",
            "Synthwave",
            "LCARS",
            "Industrial",
            "Material",
            "Retro Terminal",
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
                0 => ComboThemeConfig::default_for_fighter_hud(),
                1 => ComboThemeConfig::default_for_cyberpunk(),
                2 => ComboThemeConfig::default_for_synthwave(),
                3 => ComboThemeConfig::default_for_lcars(),
                4 => ComboThemeConfig::default_for_industrial(),
                5 => ComboThemeConfig::default_for_material(),
                6 => ComboThemeConfig::default_for_retro_terminal(),
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

        scroll.set_child(Some(&inner_box));
        page.append(&scroll);

        // Store theme widgets for later updates
        *theme_widgets_out.borrow_mut() = Some(ThemeWidgets {
            // HUD scheme widgets
            hud_color_dropdown,
            custom_hud_color_widget,
            custom_color_row,
            background_widget,
            glow_scale,
            // Theme color widgets
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
        config: &Rc<RefCell<FighterHudDisplayConfig>>,
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
        config: &Rc<RefCell<FighterHudDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        content_notebook: &Rc<RefCell<Notebook>>,
        source_summaries: &Rc<RefCell<Vec<(String, String, usize, u32)>>>,
        available_fields: &Rc<RefCell<Vec<FieldMetadata>>>,
        theme_ref_refreshers: &Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        combo_config_base::set_page_margins(&page);

        let info_label = Label::new(Some("Content items are configured per source slot.\nSelect a slot tab to configure its display type."));
        info_label.set_halign(gtk4::Align::Start);
        info_label.add_css_class("dim-label");
        page.append(&info_label);

        // Create scrolled window for content tabs
        let scroll = ScrolledWindow::new();
        scroll.set_vexpand(true);
        scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);

        {
            let notebook = content_notebook.borrow();
            scroll.set_child(Some(&*notebook));
        }

        page.append(&scroll);

        // Build initial content tabs
        Self::rebuild_content_tabs(config, on_change, preview, content_notebook, source_summaries, available_fields, theme_ref_refreshers);

        page
    }

    fn create_animation_page(
        config: &Rc<RefCell<FighterHudDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        animation_widgets_out: &Rc<RefCell<Option<AnimationWidgets>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        combo_config_base::set_page_margins(&page);

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

        // Scan line effect
        let scan_line_check = CheckButton::with_label("Scan Line Effect");
        scan_line_check.set_active(config.borrow().frame.scan_line_effect);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        scan_line_check.connect_toggled(move |check| {
            config_clone.borrow_mut().frame.scan_line_effect = check.is_active();
            if let Some(cb) = on_change_clone.borrow().as_ref() {
                cb();
            }
        });
        page.append(&scan_line_check);

        // Store widget refs
        *animation_widgets_out.borrow_mut() = Some(AnimationWidgets {
            enable_check,
            speed_spin,
            scan_line_check,
        });

        page
    }

    fn rebuild_group_spinners(
        config: &Rc<RefCell<FighterHudDisplayConfig>>,
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
                drop(cfg);
                combo_config_base::queue_redraw(&preview_clone, &on_change_clone);
            });

            group_weights_box.append(&row);
        }
    }

    fn rebuild_content_tabs(
        config: &Rc<RefCell<FighterHudDisplayConfig>>,
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

    // Public API
    pub fn widget(&self) -> &GtkBox {
        &self.container
    }

    pub fn get_config(&self) -> FighterHudDisplayConfig {
        self.config.borrow().clone()
    }

    /// Get a reference to the internal config Rc for use in callbacks
    pub fn get_config_rc(&self) -> Rc<RefCell<FighterHudDisplayConfig>> {
        self.config.clone()
    }

    pub fn set_config(&self, config: FighterHudDisplayConfig) {
        // IMPORTANT: Temporarily disable on_change callback to prevent signal cascade.
        let saved_callback = self.on_change.borrow_mut().take();

        // Extract all values we need BEFORE updating the config
        // This prevents RefCell borrow conflicts when GTK callbacks fire synchronously
        let color_idx = match &config.frame.hud_color {
            HudColorPreset::MilitaryGreen => 0,
            HudColorPreset::Amber => 1,
            HudColorPreset::Cyan => 2,
            HudColorPreset::White => 3,
            HudColorPreset::Custom(_) => 4,
        };
        let custom_color = if let HudColorPreset::Custom(c) = &config.frame.hud_color {
            Some(*c)
        } else {
            None
        };
        let background_color = config.frame.background_color;
        let glow_intensity = config.frame.glow_intensity;

        let frame_style_idx = match config.frame.frame_style {
            HudFrameStyle::CornerBrackets => 0,
            HudFrameStyle::TargetingReticle => 1,
            HudFrameStyle::TacticalBox => 2,
            HudFrameStyle::Minimal => 3,
            HudFrameStyle::None => 4,
        };
        let line_width = config.frame.line_width;
        let bracket_size = config.frame.bracket_size;
        let bracket_thickness = config.frame.bracket_thickness;
        let show_center_reticle = config.frame.show_center_reticle;
        let reticle_size = config.frame.reticle_size;

        let show_header = config.frame.show_header;
        let header_text = config.frame.header_text.clone();
        let header_style_idx = match config.frame.header_style {
            HudHeaderStyle::StatusBar => 0,
            HudHeaderStyle::MissionCallout => 1,
            HudHeaderStyle::SystemId => 2,
            HudHeaderStyle::None => 3,
        };
        let header_height = config.frame.header_height;
        let header_font = config.frame.header_font.clone();
        let theme = config.frame.theme.clone();

        let orient_idx = match config.frame.split_orientation {
            SplitOrientation::Vertical => 0,
            SplitOrientation::Horizontal => 1,
        };
        let content_padding = config.frame.content_padding;
        let div_style_idx = match config.frame.divider_style {
            HudDividerStyle::TickLadder => 0,
            HudDividerStyle::ArrowLine => 1,
            HudDividerStyle::TacticalDash => 2,
            HudDividerStyle::Fade => 3,
            HudDividerStyle::None => 4,
        };
        let divider_padding = config.frame.divider_padding;
        let tick_spacing = config.frame.tick_spacing;

        let animation_enabled = config.animation_enabled;
        let animation_speed = config.animation_speed;
        let scan_line_effect = config.frame.scan_line_effect;

        // Now update the config
        *self.config.borrow_mut() = config;

        // Update UI widgets - config borrow is dropped, so callbacks can safely borrow_mut
        // Update theme widgets (includes HUD scheme settings)
        if let Some(ref widgets) = *self.theme_widgets.borrow() {
            widgets.hud_color_dropdown.set_selected(color_idx);
            widgets.custom_color_row.set_visible(color_idx == 4);
            if let Some(c) = custom_color {
                widgets.custom_hud_color_widget.set_color(c);
            }
            widgets.background_widget.set_color(background_color);
            widgets.glow_scale.set_value(glow_intensity);
            // Update theme colors too
            widgets.theme_color1_widget.set_color(theme.color1);
            widgets.theme_color2_widget.set_color(theme.color2);
            widgets.theme_color3_widget.set_color(theme.color3);
            widgets.theme_color4_widget.set_color(theme.color4);
            widgets.theme_gradient_editor.set_gradient_source_config(&theme.gradient);
            widgets.font1_btn.set_label(&theme.font1_family);
            widgets.font1_size_spin.set_value(theme.font1_size);
            widgets.font2_btn.set_label(&theme.font2_family);
            widgets.font2_size_spin.set_value(theme.font2_size);
        }

        if let Some(ref widgets) = *self.frame_widgets.borrow() {
            widgets.style_dropdown.set_selected(frame_style_idx);
            widgets.line_width_spin.set_value(line_width);
            widgets.bracket_size_spin.set_value(bracket_size);
            widgets.bracket_thickness_spin.set_value(bracket_thickness);
            widgets.show_reticle_check.set_active(show_center_reticle);
            widgets.reticle_size_spin.set_value(reticle_size);
        }

        if let Some(ref widgets) = *self.header_widgets.borrow() {
            widgets.show_header_check.set_active(show_header);
            widgets.header_text_entry.set_text(&header_text);
            widgets.header_style_dropdown.set_selected(header_style_idx);
            widgets.header_height_spin.set_value(header_height);
            widgets.header_font_selector.set_source(header_font);
            widgets.header_font_selector.set_theme_config(theme.clone());
        }

        if let Some(ref widgets) = *self.layout_widgets.borrow() {
            widgets.split_orientation_dropdown.set_selected(orient_idx);
            widgets.content_padding_spin.set_value(content_padding);
            widgets.divider_style_dropdown.set_selected(div_style_idx);
            widgets.divider_padding_spin.set_value(divider_padding);
            widgets.tick_spacing_spin.set_value(tick_spacing);
        }

        if let Some(ref widgets) = *self.animation_widgets.borrow() {
            widgets.enable_check.set_active(animation_enabled);
            widgets.speed_spin.set_value(animation_speed);
            widgets.scan_line_check.set_active(scan_line_effect);
        }

        // Update Theme widgets (fonts and colors)
        if let Some(ref widgets) = *self.theme_widgets.borrow() {
            let cfg = self.config.borrow();
            widgets.theme_color1_widget.set_color(cfg.frame.theme.color1);
            widgets.theme_color2_widget.set_color(cfg.frame.theme.color2);
            widgets.theme_color3_widget.set_color(cfg.frame.theme.color3);
            widgets.theme_color4_widget.set_color(cfg.frame.theme.color4);
            widgets.theme_gradient_editor.set_gradient_source_config(&cfg.frame.theme.gradient);
            widgets.font1_btn.set_label(&cfg.frame.theme.font1_family);
            widgets.font1_size_spin.set_value(cfg.frame.theme.font1_size);
            widgets.font2_btn.set_label(&cfg.frame.theme.font2_family);
            widgets.font2_size_spin.set_value(cfg.frame.theme.font2_size);
        }

        // Rebuild content tabs to update theme reference sections
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

    pub fn set_on_change(&self, callback: impl Fn() + 'static) {
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

    pub fn set_source_summaries(&self, summaries: Vec<(String, String, usize, u32)>) {
        // Extract group configuration from summaries (lesson #1)
        let mut group_item_counts: std::collections::HashMap<usize, u32> = std::collections::HashMap::new();
        for (_, _, group_num, item_idx) in &summaries {
            let current_max = group_item_counts.entry(*group_num).or_insert(0);
            if *item_idx > *current_max {
                *current_max = *item_idx;
            }
        }

        // Update config with group info
        {
            let mut cfg = self.config.borrow_mut();
            let group_count = group_item_counts.len().max(1);
            cfg.frame.group_count = group_count;

            // Convert to Vec
            let mut counts_vec: Vec<usize> = vec![0; group_count];
            for (group_idx, max_item) in group_item_counts {
                if group_idx > 0 && group_idx <= group_count {
                    counts_vec[group_idx - 1] = max_item as usize;
                }
            }
            cfg.frame.group_item_counts = counts_vec;

            // Ensure weights are set
            while cfg.frame.group_size_weights.len() < group_count {
                cfg.frame.group_size_weights.push(1.0);
            }
        }

        *self.source_summaries.borrow_mut() = summaries;

        // Rebuild group spinners and item orientation dropdowns in Layout tab
        if let Some(ref widgets) = *self.layout_widgets.borrow() {
            Self::rebuild_group_spinners(
                &self.config,
                &self.on_change,
                &self.preview,
                &widgets.group_weights_box,
            );
            combo_config_base::rebuild_item_orientation_dropdowns(
                &widgets.item_orientations_box,
                &self.config,
                |c: &mut FighterHudDisplayConfig| &mut c.frame,
                &self.on_change,
                &self.preview,
            );
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

        // Notify that config changed (lesson #12)
        if let Some(cb) = self.on_change.borrow().as_ref() {
            cb();
        }
    }

    pub fn set_available_fields(&self, fields: Vec<FieldMetadata>) {
        *self.available_fields.borrow_mut() = fields;
    }

    /// Extract transferable configuration that can be applied to another combo panel type.
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
            item_spacing: 8.0, // Not configurable in Fighter HUD, use default
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
            // item_spacing not configurable in Fighter HUD
            config.animation_enabled = transfer.animation_enabled;
            config.animation_speed = transfer.animation_speed;
        }
        self.preview.queue_draw();
    }
}

impl Default for FighterHudConfigWidget {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}
