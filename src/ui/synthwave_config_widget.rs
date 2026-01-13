//! Synthwave/Outrun configuration widget
//!
//! Provides a tabbed interface for configuring all aspects of the Synthwave display.

use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, CheckButton, DrawingArea, DropDown, Entry, Label, Notebook, Orientation,
    Scale, SpinButton, StringList,
};
use std::cell::RefCell;
use std::rc::Rc;

use crate::ui::synthwave_display::{
    render_synthwave_frame, SynthwaveFrameStyle,
    GridStyle, SynthwaveHeaderStyle, SynthwaveDividerStyle, SynthwaveColorScheme,
};
use crate::ui::lcars_display::SplitOrientation;
use crate::ui::ThemeFontSelector;
use crate::ui::theme::FontSource;
use crate::ui::combo_config_base;
use crate::ui::widget_builder::{ConfigWidgetBuilder, create_section_header};
use crate::displayers::SynthwaveDisplayConfig;
use crate::core::FieldMetadata;

/// Holds references to Theme tab widgets
struct ThemeWidgets {
    common: combo_config_base::CommonThemeWidgets,
    // Color scheme preset dropdown
    color_scheme_dropdown: DropDown,
    // Neon glow (synthwave-specific effect)
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
    group_settings_box: GtkBox,
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
        preview.set_content_width(100);
        preview.set_hexpand(true);
        preview.set_halign(gtk4::Align::Fill);
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
            |cfg, slot_name, item| {
                cfg.frame.content_items.insert(slot_name.to_string(), item);
            },
            &theme_ref_refreshers,
            |cfg| cfg.frame.theme.clone(),
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

    fn create_theme_page(
        config: &Rc<RefCell<SynthwaveDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        theme_widgets_out: &Rc<RefCell<Option<ThemeWidgets>>>,
        theme_ref_refreshers: &Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        combo_config_base::set_page_margins(&page);

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

        // Create an inner box for common theme widgets
        let inner_box = GtkBox::new(Orientation::Vertical, 8);
        page.append(&inner_box);

        // Create clones for callbacks
        let config_for_change = config.clone();
        let preview_for_redraw = preview.clone();
        let on_change_for_redraw = on_change.clone();
        let refreshers_for_redraw = theme_ref_refreshers.clone();

        // Use shared helper for common theme widgets
        let common = combo_config_base::create_common_theme_widgets(
            &inner_box,
            &config.borrow().frame.theme,
            move |mutator| {
                mutator(&mut config_for_change.borrow_mut().frame.theme);
            },
            move || {
                combo_config_base::queue_redraw(&preview_for_redraw, &on_change_for_redraw);
                combo_config_base::refresh_theme_refs(&refreshers_for_redraw);
            },
        );

        // Connect color scheme dropdown - auto-populate theme colors when preset selected
        let config_scheme = config.clone();
        let on_change_scheme = on_change.clone();
        let preview_scheme = preview.clone();
        let refreshers_scheme = theme_ref_refreshers.clone();
        let common_for_scheme = common.clone();
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
                    combo_config_base::queue_redraw(&preview_scheme, &on_change_scheme);
                    combo_config_base::refresh_theme_refs(&refreshers_scheme);
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

            // Update all theme color widgets and internal state using the common helper
            common_for_scheme.apply_theme_colors(primary, secondary, accent, bg_top);

            // Refresh all theme-linked widgets (theme reference section, etc.)
            combo_config_base::refresh_theme_refs(&refreshers_scheme);
            combo_config_base::queue_redraw(&preview_scheme, &on_change_scheme);
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
            combo_config_base::queue_redraw(&preview_glow, &on_change_glow);
        });
        page.append(&glow_box);

        // Store widget refs
        *theme_widgets_out.borrow_mut() = Some(ThemeWidgets {
            common,
            color_scheme_dropdown,
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
        combo_config_base::set_page_margins(&page);

        let builder = ConfigWidgetBuilder::new(config, preview, on_change);

        let style_idx = match config.borrow().frame.frame_style {
            SynthwaveFrameStyle::NeonBorder => 0,
            SynthwaveFrameStyle::Chrome => 1,
            SynthwaveFrameStyle::Minimal => 2,
            SynthwaveFrameStyle::RetroDouble => 3,
            SynthwaveFrameStyle::None => 4,
        };
        let style_dropdown = builder.dropdown_row(
            &page, "Frame Style:", &["Neon Border", "Chrome", "Minimal", "Retro Double", "None"], style_idx,
            |cfg, idx| cfg.frame.frame_style = match idx {
                0 => SynthwaveFrameStyle::NeonBorder,
                1 => SynthwaveFrameStyle::Chrome,
                2 => SynthwaveFrameStyle::Minimal,
                3 => SynthwaveFrameStyle::RetroDouble,
                _ => SynthwaveFrameStyle::None,
            },
        );

        let frame_width_spin = builder.spin_row(
            &page, "Frame Width:", 0.5, 6.0, 0.5, config.borrow().frame.frame_width,
            |cfg, v| cfg.frame.frame_width = v,
        );

        let corner_radius_spin = builder.spin_row(
            &page, "Corner Radius:", 0.0, 30.0, 2.0, config.borrow().frame.corner_radius,
            |cfg, v| cfg.frame.corner_radius = v,
        );

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
        combo_config_base::set_page_margins(&page);

        let builder = ConfigWidgetBuilder::new(config, preview, on_change);

        let show_grid_check = builder.check_button(
            &page, "Show Grid", config.borrow().frame.show_grid,
            |cfg, v| cfg.frame.show_grid = v,
        );

        let style_idx = match config.borrow().frame.grid_style {
            GridStyle::Perspective => 0,
            GridStyle::Flat => 1,
            GridStyle::Hexagon => 2,
            GridStyle::Scanlines => 3,
            GridStyle::None => 4,
        };
        let grid_style_dropdown = builder.dropdown_row(
            &page, "Grid Style:", &["Perspective", "Flat", "Hexagon", "Scanlines", "None"], style_idx,
            |cfg, idx| cfg.frame.grid_style = match idx {
                0 => GridStyle::Perspective,
                1 => GridStyle::Flat,
                2 => GridStyle::Hexagon,
                3 => GridStyle::Scanlines,
                _ => GridStyle::None,
            },
        );

        let grid_spacing_spin = builder.spin_row(
            &page, "Grid Spacing:", 10.0, 100.0, 5.0, config.borrow().frame.grid_spacing,
            |cfg, v| cfg.frame.grid_spacing = v,
        );

        let grid_line_width_spin = builder.spin_row(
            &page, "Line Width:", 0.5, 4.0, 0.5, config.borrow().frame.grid_line_width,
            |cfg, v| cfg.frame.grid_line_width = v,
        );

        let horizon_scale = builder.scale_row(
            &page, "Horizon:", 0.1, 0.9, 0.05, config.borrow().frame.grid_horizon,
            |cfg, v| cfg.frame.grid_horizon = v,
        );

        let perspective_scale = builder.scale_row(
            &page, "Perspective:", 0.0, 1.0, 0.1, config.borrow().frame.grid_perspective,
            |cfg, v| cfg.frame.grid_perspective = v,
        );

        // Sun section
        let sun_label = create_section_header("Sun Effect");
        sun_label.set_margin_top(12);
        page.append(&sun_label);

        let show_sun_check = builder.check_button(
            &page, "Show Sun", config.borrow().frame.show_sun,
            |cfg, v| cfg.frame.show_sun = v,
        );

        let sun_position_scale = builder.scale_row(
            &page, "Sun Position:", 0.0, 1.0, 0.1, config.borrow().frame.sun_position,
            |cfg, v| cfg.frame.sun_position = v,
        );

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
        combo_config_base::set_page_margins(&page);

        let builder = ConfigWidgetBuilder::new(config, preview, on_change);

        let show_header_check = builder.check_button(
            &page, "Show Header", config.borrow().frame.show_header,
            |cfg, v| cfg.frame.show_header = v,
        );

        let header_text_entry = builder.entry_row(
            &page, "Header Text:", &config.borrow().frame.header_text,
            |cfg, v| cfg.frame.header_text = v,
        );

        let style_idx = match config.borrow().frame.header_style {
            SynthwaveHeaderStyle::Chrome => 0,
            SynthwaveHeaderStyle::Neon => 1,
            SynthwaveHeaderStyle::Outline => 2,
            SynthwaveHeaderStyle::Simple => 3,
            SynthwaveHeaderStyle::None => 4,
        };
        let header_style_dropdown = builder.dropdown_row(
            &page, "Header Style:", &["Chrome", "Neon", "Outline", "Simple", "None"], style_idx,
            |cfg, idx| cfg.frame.header_style = match idx {
                0 => SynthwaveHeaderStyle::Chrome,
                1 => SynthwaveHeaderStyle::Neon,
                2 => SynthwaveHeaderStyle::Outline,
                3 => SynthwaveHeaderStyle::Simple,
                _ => SynthwaveHeaderStyle::None,
            },
        );

        let header_height_spin = builder.spin_row(
            &page, "Header Height:", 20.0, 60.0, 2.0, config.borrow().frame.header_height,
            |cfg, v| cfg.frame.header_height = v,
        );

        // Font section
        let font_label = create_section_header("Font");
        font_label.set_margin_top(12);
        page.append(&font_label);

        let current_font = config.borrow().frame.header_font.clone();
        let current_size = config.borrow().frame.header_font_size;
        let header_font_selector = Rc::new(ThemeFontSelector::new(
            FontSource::Custom { family: current_font, size: current_size }
        ));
        header_font_selector.set_theme_config(config.borrow().frame.theme.clone());

        let font_box = GtkBox::new(Orientation::Horizontal, 6);
        font_box.append(header_font_selector.widget());
        page.append(&font_box);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        header_font_selector.set_on_change(move |source| {
            let (family, size) = match &source {
                FontSource::Theme { index, size } => {
                    let cfg = config_clone.borrow();
                    let (family, _) = cfg.frame.theme.get_font(*index);
                    (family, *size)
                }
                FontSource::Custom { family, size } => (family.clone(), *size),
            };
            {
                let mut cfg = config_clone.borrow_mut();
                cfg.frame.header_font = family;
                cfg.frame.header_font_size = size;
            }
            combo_config_base::queue_redraw(&preview_clone, &on_change_clone);
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

        let item_spacing_spin = builder.spin_row(
            &page, "Item Spacing:", 0.0, 20.0, 1.0, config.borrow().frame.item_spacing,
            |cfg, v| cfg.frame.item_spacing = v,
        );

        // Divider section
        let divider_label = create_section_header("Dividers");
        divider_label.set_margin_top(12);
        page.append(&divider_label);

        let div_style_idx = match config.borrow().frame.divider_style {
            SynthwaveDividerStyle::NeonLine => 0,
            SynthwaveDividerStyle::Gradient => 1,
            SynthwaveDividerStyle::NeonDots => 2,
            SynthwaveDividerStyle::Line => 3,
            SynthwaveDividerStyle::None => 4,
        };
        let divider_style_dropdown = builder.dropdown_row(
            &page, "Divider Style:", &["Neon Line", "Gradient", "Neon Dots", "Line", "None"], div_style_idx,
            |cfg, idx| cfg.frame.divider_style = match idx {
                0 => SynthwaveDividerStyle::NeonLine,
                1 => SynthwaveDividerStyle::Gradient,
                2 => SynthwaveDividerStyle::NeonDots,
                3 => SynthwaveDividerStyle::Line,
                _ => SynthwaveDividerStyle::None,
            },
        );

        let divider_padding_spin = builder.spin_row(
            &page, "Divider Padding:", 2.0, 20.0, 1.0, config.borrow().frame.divider_padding,
            |cfg, v| cfg.frame.divider_padding = v,
        );

        // Combined group settings section (weight + orientation per group)
        let group_settings_box = combo_config_base::create_combined_group_settings_section(&page);
        combo_config_base::rebuild_combined_group_settings(
            &group_settings_box,
            config,
            |c: &mut SynthwaveDisplayConfig| &mut c.frame,
            on_change,
            preview,
        );

        // Store widget refs
        *layout_widgets_out.borrow_mut() = Some(LayoutWidgets {
            split_orientation_dropdown,
            content_padding_spin,
            item_spacing_spin,
            divider_style_dropdown,
            divider_padding_spin,
            group_settings_box,
        });

        page
    }

    fn create_animation_page(
        config: &Rc<RefCell<SynthwaveDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        animation_widgets_out: &Rc<RefCell<Option<AnimationWidgets>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        combo_config_base::set_page_margins(&page);

        // Animation page doesn't need preview, use simpler callback pattern
        let enable_check = CheckButton::with_label("Enable Animations");
        enable_check.set_active(config.borrow().animation_enabled);
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        enable_check.connect_toggled(move |check| {
            config_clone.borrow_mut().animation_enabled = check.is_active();
            if let Some(cb) = on_change_clone.borrow().as_ref() { cb(); }
        });
        page.append(&enable_check);

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
            if let Some(cb) = on_change_clone.borrow().as_ref() { cb(); }
        });
        page.append(&speed_box);

        let scanline_check = CheckButton::with_label("Scanline Effect");
        scanline_check.set_active(config.borrow().frame.scanline_effect);
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        scanline_check.connect_toggled(move |check| {
            config_clone.borrow_mut().frame.scanline_effect = check.is_active();
            if let Some(cb) = on_change_clone.borrow().as_ref() { cb(); }
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
    // Public API
    pub fn widget(&self) -> &GtkBox {
        &self.container
    }

    pub fn get_config(&self) -> SynthwaveDisplayConfig {
        self.config.borrow().clone()
    }

    /// Get a reference to the internal config Rc for use in callbacks
    pub fn get_config_rc(&self) -> Rc<RefCell<SynthwaveDisplayConfig>> {
        self.config.clone()
    }

    pub fn set_config(&self, config: SynthwaveDisplayConfig) {
        // IMPORTANT: Temporarily disable on_change callback to prevent signal cascade.
        let saved_callback = self.on_change.borrow_mut().take();

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
            // Apply the complete theme using the common helper
            let full_theme = crate::ui::theme::ComboThemeConfig {
                color1: theme_color1,
                color2: theme_color2,
                color3: theme_color3,
                color4: theme_color4,
                gradient: theme_gradient.clone(),
                font1_family: theme_font1_family.clone(),
                font1_size: theme_font1_size,
                font2_family: theme_font2_family.clone(),
                font2_size: theme_font2_size,
            };
            widgets.common.apply_theme_preset(&full_theme);
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

        // Trigger all theme refreshers to update child widgets with the new theme
        for refresher in self.theme_ref_refreshers.borrow().iter() {
            refresher();
        }

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
        self.config.borrow_mut().frame.theme = theme.clone();
        // Update header font selector with new theme
        if let Some(ref widgets) = *self.header_widgets.borrow() {
            widgets.header_font_selector.set_theme_config(theme.clone());
        }
        // Trigger all theme refreshers to update child widgets
        for refresher in self.theme_ref_refreshers.borrow().iter() {
            refresher();
        }
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

        // Rebuild combined group settings in Layout tab
        if let Some(ref widgets) = *self.layout_widgets.borrow() {
            combo_config_base::rebuild_combined_group_settings(
                &widgets.group_settings_box,
                &self.config,
                |c: &mut SynthwaveDisplayConfig| &mut c.frame,
                &self.on_change,
                &self.preview,
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
            |cfg, slot_name, item| {
                cfg.frame.content_items.insert(slot_name.to_string(), item);
            },
            &self.theme_ref_refreshers,
            |cfg| cfg.frame.theme.clone(),
        );

        // Notify that config changed
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
            layout_orientation: config.frame.split_orientation,
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
            config.frame.split_orientation = transfer.layout_orientation;
            config.frame.content_items = transfer.content_items.clone();
            config.frame.content_padding = transfer.content_padding;
            config.frame.item_spacing = transfer.item_spacing;
            config.animation_enabled = transfer.animation_enabled;
            config.animation_speed = transfer.animation_speed;
        }
        self.preview.queue_draw();
    }
}

impl Default for SynthwaveConfigWidget {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}
