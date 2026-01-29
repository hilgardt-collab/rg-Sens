//! Industrial/Gauge Panel configuration widget
//!
//! Provides a tabbed interface for configuring all aspects of the Industrial display.

use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, CheckButton, DrawingArea, DropDown, Entry, Label, Notebook, Orientation,
    ScrolledWindow, SpinButton, StringList,
};
use std::cell::RefCell;
use std::rc::Rc;

use crate::core::FieldMetadata;
use crate::displayers::IndustrialDisplayConfig;
use crate::ui::color_button_widget::ColorButtonWidget;
use crate::ui::combo_config_base;
use crate::ui::industrial_display::{
    DividerStyle, HeaderStyle, RivetStyle, SurfaceTexture, WarningStripePosition,
};
use crate::ui::lcars_display::SplitOrientation;
use crate::ui::theme::{ColorSource, FontSource};
use crate::ui::theme_color_selector::ThemeColorSelector;
use crate::ui::theme_font_selector::ThemeFontSelector;
use crate::ui::widget_builder::{create_dropdown_row, create_section_header, ConfigWidgetBuilder};

/// Holds references to Surface tab widgets
struct SurfaceWidgets {
    texture_dropdown: DropDown,
    surface_color_widget: Rc<ColorButtonWidget>,
    surface_dark_widget: Rc<ColorButtonWidget>,
    highlight_color_widget: Rc<ColorButtonWidget>,
}

/// Holds references to Border tab widgets
struct BorderWidgets {
    show_border_check: CheckButton,
    border_width_spin: SpinButton,
    border_color_widget: Rc<ColorButtonWidget>,
    corner_radius_spin: SpinButton,
    show_bevel_check: CheckButton,
    bevel_width_spin: SpinButton,
}

/// Holds references to Rivet tab widgets
struct RivetWidgets {
    rivet_style_dropdown: DropDown,
    rivet_size_spin: SpinButton,
    rivet_color_widget: Rc<ColorButtonWidget>,
    rivet_spacing_spin: SpinButton,
    show_corner_rivets_check: CheckButton,
    show_edge_rivets_check: CheckButton,
}

/// Holds references to Warning tab widgets
struct WarningWidgets {
    position_dropdown: DropDown,
    stripe_width_spin: SpinButton,
    color1_widget: Rc<ColorButtonWidget>,
    color2_widget: Rc<ColorButtonWidget>,
    angle_spin: SpinButton,
}

/// Holds references to Header tab widgets
struct HeaderWidgets {
    show_header_check: CheckButton,
    header_text_entry: Entry,
    header_style_dropdown: DropDown,
    header_height_spin: SpinButton,
    header_font_btn: Button,
    header_font_size_spin: SpinButton,
    header_color_widget: Rc<ColorButtonWidget>,
}

/// Holds references to Layout tab widgets
struct LayoutWidgets {
    split_orientation_dropdown: DropDown,
    content_padding_spin: SpinButton,
    item_spacing_spin: SpinButton,
    divider_style_dropdown: DropDown,
    divider_width_spin: SpinButton,
    divider_color_widget: Rc<ThemeColorSelector>,
    group_settings_box: GtkBox,
}

/// Holds references to Animation tab widgets
struct AnimationWidgets {
    enable_check: CheckButton,
    speed_spin: SpinButton,
}

/// Holds references to Theme tab widgets
#[allow(dead_code)]
struct ThemeWidgets {
    common: combo_config_base::CommonThemeWidgets,
}

/// Industrial configuration widget
pub struct IndustrialConfigWidget {
    container: GtkBox,
    config: Rc<RefCell<IndustrialDisplayConfig>>,
    on_change: Rc<RefCell<Option<Box<dyn Fn()>>>>,
    preview: DrawingArea,
    content_notebook: Rc<RefCell<Notebook>>,
    source_summaries: Rc<RefCell<Vec<(String, String, usize, u32)>>>,
    available_fields: Rc<RefCell<Vec<FieldMetadata>>>,
    surface_widgets: Rc<RefCell<Option<SurfaceWidgets>>>,
    border_widgets: Rc<RefCell<Option<BorderWidgets>>>,
    rivet_widgets: Rc<RefCell<Option<RivetWidgets>>>,
    warning_widgets: Rc<RefCell<Option<WarningWidgets>>>,
    header_widgets: Rc<RefCell<Option<HeaderWidgets>>>,
    layout_widgets: Rc<RefCell<Option<LayoutWidgets>>>,
    animation_widgets: Rc<RefCell<Option<AnimationWidgets>>>,
    #[allow(dead_code)]
    theme_widgets: Rc<RefCell<Option<ThemeWidgets>>>,
    #[allow(dead_code)] // Kept for Rc ownership; callbacks are invoked via clones
    theme_ref_refreshers: Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
    /// Cleanup callbacks for Lazy*ConfigWidget instances in content tabs
    content_cleanup_callbacks: Rc<RefCell<Vec<combo_config_base::CleanupCallback>>>,
}

impl IndustrialConfigWidget {
    pub fn new(available_fields: Vec<FieldMetadata>) -> Self {
        let container = GtkBox::new(Orientation::Vertical, 12);
        let config = Rc::new(RefCell::new(IndustrialDisplayConfig::default()));
        let on_change: Rc<RefCell<Option<Box<dyn Fn()>>>> = Rc::new(RefCell::new(None));
        let source_summaries: Rc<RefCell<Vec<(String, String, usize, u32)>>> =
            Rc::new(RefCell::new(Vec::new()));
        let available_fields: Rc<RefCell<Vec<FieldMetadata>>> =
            Rc::new(RefCell::new(available_fields));
        let surface_widgets: Rc<RefCell<Option<SurfaceWidgets>>> = Rc::new(RefCell::new(None));
        let border_widgets: Rc<RefCell<Option<BorderWidgets>>> = Rc::new(RefCell::new(None));
        let rivet_widgets: Rc<RefCell<Option<RivetWidgets>>> = Rc::new(RefCell::new(None));
        let warning_widgets: Rc<RefCell<Option<WarningWidgets>>> = Rc::new(RefCell::new(None));
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

        // Tab 1: Theme (first for easy access)
        let theme_page = Self::create_theme_page(
            &config,
            &on_change,
            &preview,
            &theme_widgets,
            &theme_ref_refreshers,
        );
        notebook.append_page(&theme_page, Some(&Label::new(Some("Theme"))));

        // Tab 2: Surface
        let surface_page = Self::create_surface_page(
            &config,
            &on_change,
            &preview,
            &surface_widgets,
            &theme_ref_refreshers,
        );
        notebook.append_page(&surface_page, Some(&Label::new(Some("Surface"))));

        // Tab 3: Border
        let border_page = Self::create_border_page(
            &config,
            &on_change,
            &preview,
            &border_widgets,
            &theme_ref_refreshers,
        );
        notebook.append_page(&border_page, Some(&Label::new(Some("Border"))));

        // Tab 4: Rivets
        let rivet_page = Self::create_rivet_page(&config, &on_change, &preview, &rivet_widgets);
        notebook.append_page(&rivet_page, Some(&Label::new(Some("Rivets"))));

        // Tab 5: Warning Stripes
        let warning_page = Self::create_warning_page(
            &config,
            &on_change,
            &preview,
            &warning_widgets,
            &theme_ref_refreshers,
        );
        notebook.append_page(&warning_page, Some(&Label::new(Some("Warning"))));

        // Tab 6: Header
        let header_page = Self::create_header_page(
            &config,
            &on_change,
            &preview,
            &header_widgets,
            &theme_ref_refreshers,
        );
        notebook.append_page(&header_page, Some(&Label::new(Some("Header"))));

        // Tab 7: Layout
        let layout_page = Self::create_layout_page(
            &config,
            &on_change,
            &preview,
            &layout_widgets,
            &theme_ref_refreshers,
        );
        notebook.append_page(&layout_page, Some(&Label::new(Some("Layout"))));

        // Tab 8: Content
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

        // Tab 9: Animation
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
            surface_widgets,
            border_widgets,
            rivet_widgets,
            warning_widgets,
            header_widgets,
            layout_widgets,
            animation_widgets,
            theme_widgets,
            theme_ref_refreshers,
            content_cleanup_callbacks,
        }
    }

    fn create_surface_page(
        config: &Rc<RefCell<IndustrialDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        _preview: &DrawingArea,
        surface_widgets_out: &Rc<RefCell<Option<SurfaceWidgets>>>,
        theme_ref_refreshers: &Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        combo_config_base::set_page_margins(&page);

        // Surface texture
        let texture_idx = match config.borrow().frame.surface_texture {
            SurfaceTexture::BrushedMetal => 0,
            SurfaceTexture::CarbonFiber => 1,
            SurfaceTexture::DiamondPlate => 2,
            SurfaceTexture::Solid => 3,
        };
        let (texture_box, texture_dropdown) = create_dropdown_row(
            "Texture:",
            &["Brushed Metal", "Carbon Fiber", "Diamond Plate", "Solid"],
        );
        texture_dropdown.set_selected(texture_idx);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        texture_dropdown.connect_selected_notify(move |dropdown| {
            let selected = dropdown.selected();
            if selected == gtk4::INVALID_LIST_POSITION {
                return;
            }
            config_clone.borrow_mut().frame.surface_texture = match selected {
                0 => SurfaceTexture::BrushedMetal,
                1 => SurfaceTexture::CarbonFiber,
                2 => SurfaceTexture::DiamondPlate,
                _ => SurfaceTexture::Solid,
            };
            combo_config_base::notify_change(&on_change_clone);
        });
        page.append(&texture_box);

        // Colors section
        let colors_label = Label::new(Some("Surface Colors"));
        colors_label.set_halign(gtk4::Align::Start);
        colors_label.add_css_class("heading");
        colors_label.set_margin_top(12);
        page.append(&colors_label);

        // Surface color (theme-aware)
        let surface_box = GtkBox::new(Orientation::Horizontal, 6);
        surface_box.append(&Label::new(Some("Base Color:")));
        let surface_color_widget = Rc::new(ThemeColorSelector::new(ColorSource::custom(
            config.borrow().frame.surface_color,
        )));
        surface_color_widget.set_theme_config(config.borrow().frame.theme.clone());
        surface_box.append(surface_color_widget.widget());

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        surface_color_widget.set_on_change(move |new_source| {
            let color = new_source.resolve(&config_clone.borrow().frame.theme);
            config_clone.borrow_mut().frame.surface_color = color;
            combo_config_base::notify_change(&on_change_clone);
        });

        let widget_for_refresh = surface_color_widget.clone();
        let config_for_refresh = config.clone();
        theme_ref_refreshers.borrow_mut().push(Rc::new(move || {
            widget_for_refresh.set_theme_config(config_for_refresh.borrow().frame.theme.clone());
        }));
        page.append(&surface_box);

        // Dark surface color (theme-aware)
        let surface_dark_box = GtkBox::new(Orientation::Horizontal, 6);
        surface_dark_box.append(&Label::new(Some("Dark Color:")));
        let surface_dark_widget = Rc::new(ThemeColorSelector::new(ColorSource::custom(
            config.borrow().frame.surface_color_dark,
        )));
        surface_dark_widget.set_theme_config(config.borrow().frame.theme.clone());
        surface_dark_box.append(surface_dark_widget.widget());

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        surface_dark_widget.set_on_change(move |new_source| {
            let color = new_source.resolve(&config_clone.borrow().frame.theme);
            config_clone.borrow_mut().frame.surface_color_dark = color;
            combo_config_base::notify_change(&on_change_clone);
        });

        let widget_for_refresh = surface_dark_widget.clone();
        let config_for_refresh = config.clone();
        theme_ref_refreshers.borrow_mut().push(Rc::new(move || {
            widget_for_refresh.set_theme_config(config_for_refresh.borrow().frame.theme.clone());
        }));
        page.append(&surface_dark_box);

        // Highlight color (theme-aware)
        let highlight_box = GtkBox::new(Orientation::Horizontal, 6);
        highlight_box.append(&Label::new(Some("Highlight:")));
        let highlight_color_widget = Rc::new(ThemeColorSelector::new(ColorSource::custom(
            config.borrow().frame.highlight_color,
        )));
        highlight_color_widget.set_theme_config(config.borrow().frame.theme.clone());
        highlight_box.append(highlight_color_widget.widget());

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        highlight_color_widget.set_on_change(move |new_source| {
            let color = new_source.resolve(&config_clone.borrow().frame.theme);
            config_clone.borrow_mut().frame.highlight_color = color;
            combo_config_base::notify_change(&on_change_clone);
        });

        let widget_for_refresh = highlight_color_widget.clone();
        let config_for_refresh = config.clone();
        theme_ref_refreshers.borrow_mut().push(Rc::new(move || {
            widget_for_refresh.set_theme_config(config_for_refresh.borrow().frame.theme.clone());
        }));
        page.append(&highlight_box);

        // Store widget refs - need to convert ThemeColorSelector to ColorButtonWidget for struct
        // For now, we keep the struct as ColorButtonWidget but we're not storing them
        // A proper fix would update SurfaceWidgets to use ThemeColorSelector
        *surface_widgets_out.borrow_mut() = None; // TODO: Update SurfaceWidgets struct

        page
    }

    fn create_border_page(
        config: &Rc<RefCell<IndustrialDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        _preview: &DrawingArea,
        border_widgets_out: &Rc<RefCell<Option<BorderWidgets>>>,
        theme_ref_refreshers: &Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        combo_config_base::set_page_margins(&page);

        // Show border
        let show_border_check = CheckButton::with_label("Show Border");
        show_border_check.set_active(config.borrow().frame.show_border);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        show_border_check.connect_toggled(move |check| {
            config_clone.borrow_mut().frame.show_border = check.is_active();
            combo_config_base::notify_change(&on_change_clone);
        });
        page.append(&show_border_check);

        // Border width
        let width_box = GtkBox::new(Orientation::Horizontal, 6);
        width_box.append(&Label::new(Some("Border Width:")));
        let border_width_spin = SpinButton::with_range(1.0, 10.0, 1.0);
        border_width_spin.set_value(config.borrow().frame.border_width);
        border_width_spin.set_hexpand(true);
        width_box.append(&border_width_spin);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        border_width_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().frame.border_width = spin.value();
            combo_config_base::notify_change(&on_change_clone);
        });
        page.append(&width_box);

        // Border color (theme-aware)
        let color_box = GtkBox::new(Orientation::Horizontal, 6);
        color_box.append(&Label::new(Some("Border Color:")));
        let border_color_widget = Rc::new(ThemeColorSelector::new(ColorSource::custom(
            config.borrow().frame.border_color,
        )));
        border_color_widget.set_theme_config(config.borrow().frame.theme.clone());
        color_box.append(border_color_widget.widget());

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        border_color_widget.set_on_change(move |new_source| {
            let color = new_source.resolve(&config_clone.borrow().frame.theme);
            config_clone.borrow_mut().frame.border_color = color;
            combo_config_base::notify_change(&on_change_clone);
        });

        let widget_for_refresh = border_color_widget.clone();
        let config_for_refresh = config.clone();
        theme_ref_refreshers.borrow_mut().push(Rc::new(move || {
            widget_for_refresh.set_theme_config(config_for_refresh.borrow().frame.theme.clone());
        }));
        page.append(&color_box);

        // Corner radius
        let radius_box = GtkBox::new(Orientation::Horizontal, 6);
        radius_box.append(&Label::new(Some("Corner Radius:")));
        let corner_radius_spin = SpinButton::with_range(0.0, 32.0, 2.0);
        corner_radius_spin.set_value(config.borrow().frame.corner_radius);
        corner_radius_spin.set_hexpand(true);
        radius_box.append(&corner_radius_spin);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        corner_radius_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().frame.corner_radius = spin.value();
            combo_config_base::notify_change(&on_change_clone);
        });
        page.append(&radius_box);

        // Bevel section
        let bevel_label = Label::new(Some("Bevel Effect"));
        bevel_label.set_halign(gtk4::Align::Start);
        bevel_label.add_css_class("heading");
        bevel_label.set_margin_top(12);
        page.append(&bevel_label);

        // Show bevel
        let show_bevel_check = CheckButton::with_label("Show Beveled Edge");
        show_bevel_check.set_active(config.borrow().frame.show_beveled_edge);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        show_bevel_check.connect_toggled(move |check| {
            config_clone.borrow_mut().frame.show_beveled_edge = check.is_active();
            combo_config_base::notify_change(&on_change_clone);
        });
        page.append(&show_bevel_check);

        // Bevel width
        let bevel_width_box = GtkBox::new(Orientation::Horizontal, 6);
        bevel_width_box.append(&Label::new(Some("Bevel Width:")));
        let bevel_width_spin = SpinButton::with_range(1.0, 12.0, 1.0);
        bevel_width_spin.set_value(config.borrow().frame.bevel_width);
        bevel_width_spin.set_hexpand(true);
        bevel_width_box.append(&bevel_width_spin);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        bevel_width_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().frame.bevel_width = spin.value();
            combo_config_base::notify_change(&on_change_clone);
        });
        page.append(&bevel_width_box);

        // Store widget refs - set to None since we changed to ThemeColorSelector
        *border_widgets_out.borrow_mut() = None; // TODO: Update BorderWidgets struct

        page
    }

    fn create_rivet_page(
        config: &Rc<RefCell<IndustrialDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        _preview: &DrawingArea,
        rivet_widgets_out: &Rc<RefCell<Option<RivetWidgets>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        combo_config_base::set_page_margins(&page);

        // Rivet style
        let style_idx = match config.borrow().frame.rivet_style {
            RivetStyle::Hex => 0,
            RivetStyle::Phillips => 1,
            RivetStyle::Flat => 2,
            RivetStyle::None => 3,
        };
        let (style_box, rivet_style_dropdown) = create_dropdown_row(
            "Rivet Style:",
            &["Hex Bolt", "Phillips Screw", "Flat Rivet", "None"],
        );
        rivet_style_dropdown.set_selected(style_idx);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        rivet_style_dropdown.connect_selected_notify(move |dropdown| {
            let selected = dropdown.selected();
            if selected == gtk4::INVALID_LIST_POSITION {
                return;
            }
            config_clone.borrow_mut().frame.rivet_style = match selected {
                0 => RivetStyle::Hex,
                1 => RivetStyle::Phillips,
                2 => RivetStyle::Flat,
                _ => RivetStyle::None,
            };
            combo_config_base::notify_change(&on_change_clone);
        });
        page.append(&style_box);

        // Rivet size
        let size_box = GtkBox::new(Orientation::Horizontal, 6);
        size_box.append(&Label::new(Some("Size:")));
        let rivet_size_spin = SpinButton::with_range(4.0, 16.0, 1.0);
        rivet_size_spin.set_value(config.borrow().frame.rivet_size);
        rivet_size_spin.set_hexpand(true);
        size_box.append(&rivet_size_spin);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        rivet_size_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().frame.rivet_size = spin.value();
            combo_config_base::notify_change(&on_change_clone);
        });
        page.append(&size_box);

        // Rivet color
        let color_box = GtkBox::new(Orientation::Horizontal, 6);
        color_box.append(&Label::new(Some("Color:")));
        let rivet_color_widget = Rc::new(ColorButtonWidget::new(config.borrow().frame.rivet_color));
        color_box.append(rivet_color_widget.widget());

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        rivet_color_widget.set_on_change(move |color| {
            config_clone.borrow_mut().frame.rivet_color = color;
            combo_config_base::notify_change(&on_change_clone);
        });
        page.append(&color_box);

        // Rivet spacing
        let spacing_box = GtkBox::new(Orientation::Horizontal, 6);
        spacing_box.append(&Label::new(Some("Spacing:")));
        let rivet_spacing_spin = SpinButton::with_range(30.0, 120.0, 10.0);
        rivet_spacing_spin.set_value(config.borrow().frame.rivet_spacing);
        rivet_spacing_spin.set_hexpand(true);
        spacing_box.append(&rivet_spacing_spin);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        rivet_spacing_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().frame.rivet_spacing = spin.value();
            combo_config_base::notify_change(&on_change_clone);
        });
        page.append(&spacing_box);

        // Rivet placement
        let placement_label = Label::new(Some("Placement"));
        placement_label.set_halign(gtk4::Align::Start);
        placement_label.add_css_class("heading");
        placement_label.set_margin_top(12);
        page.append(&placement_label);

        // Corner rivets
        let show_corner_rivets_check = CheckButton::with_label("Show Corner Rivets");
        show_corner_rivets_check.set_active(config.borrow().frame.show_corner_rivets);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        show_corner_rivets_check.connect_toggled(move |check| {
            config_clone.borrow_mut().frame.show_corner_rivets = check.is_active();
            combo_config_base::notify_change(&on_change_clone);
        });
        page.append(&show_corner_rivets_check);

        // Edge rivets
        let show_edge_rivets_check = CheckButton::with_label("Show Edge Rivets");
        show_edge_rivets_check.set_active(config.borrow().frame.show_edge_rivets);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        show_edge_rivets_check.connect_toggled(move |check| {
            config_clone.borrow_mut().frame.show_edge_rivets = check.is_active();
            combo_config_base::notify_change(&on_change_clone);
        });
        page.append(&show_edge_rivets_check);

        // Store widget refs
        *rivet_widgets_out.borrow_mut() = Some(RivetWidgets {
            rivet_style_dropdown,
            rivet_size_spin,
            rivet_color_widget,
            rivet_spacing_spin,
            show_corner_rivets_check,
            show_edge_rivets_check,
        });

        page
    }

    fn create_warning_page(
        config: &Rc<RefCell<IndustrialDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        _preview: &DrawingArea,
        warning_widgets_out: &Rc<RefCell<Option<WarningWidgets>>>,
        theme_ref_refreshers: &Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        combo_config_base::set_page_margins(&page);

        // Position
        let position_box = GtkBox::new(Orientation::Horizontal, 6);
        position_box.append(&Label::new(Some("Position:")));
        let position_list =
            StringList::new(&["None", "Top", "Bottom", "Left", "Right", "All Edges"]);
        let position_dropdown = DropDown::new(Some(position_list), None::<gtk4::Expression>);
        let pos_idx = match config.borrow().frame.warning_stripe_position {
            WarningStripePosition::None => 0,
            WarningStripePosition::Top => 1,
            WarningStripePosition::Bottom => 2,
            WarningStripePosition::Left => 3,
            WarningStripePosition::Right => 4,
            WarningStripePosition::All => 5,
        };
        position_dropdown.set_selected(pos_idx);
        position_dropdown.set_hexpand(true);
        position_box.append(&position_dropdown);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        position_dropdown.connect_selected_notify(move |dropdown| {
            let selected = dropdown.selected();
            if selected == gtk4::INVALID_LIST_POSITION {
                return;
            }
            config_clone.borrow_mut().frame.warning_stripe_position = match selected {
                0 => WarningStripePosition::None,
                1 => WarningStripePosition::Top,
                2 => WarningStripePosition::Bottom,
                3 => WarningStripePosition::Left,
                4 => WarningStripePosition::Right,
                _ => WarningStripePosition::All,
            };
            combo_config_base::notify_change(&on_change_clone);
        });
        page.append(&position_box);

        // Stripe width
        let width_box = GtkBox::new(Orientation::Horizontal, 6);
        width_box.append(&Label::new(Some("Stripe Width:")));
        let stripe_width_spin = SpinButton::with_range(10.0, 50.0, 5.0);
        stripe_width_spin.set_value(config.borrow().frame.warning_stripe_width);
        stripe_width_spin.set_hexpand(true);
        width_box.append(&stripe_width_spin);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        stripe_width_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().frame.warning_stripe_width = spin.value();
            combo_config_base::notify_change(&on_change_clone);
        });
        page.append(&width_box);

        // Colors section
        let colors_label = Label::new(Some("Stripe Colors"));
        colors_label.set_halign(gtk4::Align::Start);
        colors_label.add_css_class("heading");
        colors_label.set_margin_top(12);
        page.append(&colors_label);

        // Color 1 (theme-aware)
        let color1_box = GtkBox::new(Orientation::Horizontal, 6);
        color1_box.append(&Label::new(Some("Color 1:")));
        let color1_widget = Rc::new(ThemeColorSelector::new(ColorSource::custom(
            config.borrow().frame.warning_color_1,
        )));
        color1_widget.set_theme_config(config.borrow().frame.theme.clone());
        color1_box.append(color1_widget.widget());

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        color1_widget.set_on_change(move |new_source| {
            let color = new_source.resolve(&config_clone.borrow().frame.theme);
            config_clone.borrow_mut().frame.warning_color_1 = color;
            combo_config_base::notify_change(&on_change_clone);
        });

        let widget_for_refresh = color1_widget.clone();
        let config_for_refresh = config.clone();
        theme_ref_refreshers.borrow_mut().push(Rc::new(move || {
            widget_for_refresh.set_theme_config(config_for_refresh.borrow().frame.theme.clone());
        }));
        page.append(&color1_box);

        // Color 2 (theme-aware)
        let color2_box = GtkBox::new(Orientation::Horizontal, 6);
        color2_box.append(&Label::new(Some("Color 2:")));
        let color2_widget = Rc::new(ThemeColorSelector::new(ColorSource::custom(
            config.borrow().frame.warning_color_2,
        )));
        color2_widget.set_theme_config(config.borrow().frame.theme.clone());
        color2_box.append(color2_widget.widget());

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        color2_widget.set_on_change(move |new_source| {
            let color = new_source.resolve(&config_clone.borrow().frame.theme);
            config_clone.borrow_mut().frame.warning_color_2 = color;
            combo_config_base::notify_change(&on_change_clone);
        });

        let widget_for_refresh = color2_widget.clone();
        let config_for_refresh = config.clone();
        theme_ref_refreshers.borrow_mut().push(Rc::new(move || {
            widget_for_refresh.set_theme_config(config_for_refresh.borrow().frame.theme.clone());
        }));
        page.append(&color2_box);

        // Stripe angle
        let angle_box = GtkBox::new(Orientation::Horizontal, 6);
        angle_box.append(&Label::new(Some("Angle (degrees):")));
        let angle_spin = SpinButton::with_range(-90.0, 90.0, 5.0);
        angle_spin.set_value(config.borrow().frame.warning_stripe_angle);
        angle_spin.set_hexpand(true);
        angle_box.append(&angle_spin);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        angle_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().frame.warning_stripe_angle = spin.value();
            combo_config_base::notify_change(&on_change_clone);
        });
        page.append(&angle_box);

        // Store widget refs - set to None since we changed to ThemeColorSelector
        *warning_widgets_out.borrow_mut() = None; // TODO: Update WarningWidgets struct

        page
    }

    fn create_header_page(
        config: &Rc<RefCell<IndustrialDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        _preview: &DrawingArea,
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
        show_header_check.connect_toggled(move |check| {
            config_clone.borrow_mut().frame.show_header = check.is_active();
            combo_config_base::notify_change(&on_change_clone);
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
        header_text_entry.connect_changed(move |entry| {
            config_clone.borrow_mut().frame.header_text = entry.text().to_string();
            combo_config_base::notify_change(&on_change_clone);
        });
        page.append(&text_box);

        // Header style
        let style_box = GtkBox::new(Orientation::Horizontal, 6);
        style_box.append(&Label::new(Some("Style:")));
        let style_list = StringList::new(&["Metal Plate", "Stencil", "Equipment Label", "None"]);
        let header_style_dropdown = DropDown::new(Some(style_list), None::<gtk4::Expression>);
        let style_idx = match config.borrow().frame.header_style {
            HeaderStyle::Plate => 0,
            HeaderStyle::Stencil => 1,
            HeaderStyle::Label => 2,
            HeaderStyle::None => 3,
        };
        header_style_dropdown.set_selected(style_idx);
        header_style_dropdown.set_hexpand(true);
        style_box.append(&header_style_dropdown);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        header_style_dropdown.connect_selected_notify(move |dropdown| {
            let selected = dropdown.selected();
            if selected == gtk4::INVALID_LIST_POSITION {
                return;
            }
            config_clone.borrow_mut().frame.header_style = match selected {
                0 => HeaderStyle::Plate,
                1 => HeaderStyle::Stencil,
                2 => HeaderStyle::Label,
                _ => HeaderStyle::None,
            };
            combo_config_base::notify_change(&on_change_clone);
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
        header_height_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().frame.header_height = spin.value();
            combo_config_base::notify_change(&on_change_clone);
        });
        page.append(&height_box);

        // Font section
        let font_label = Label::new(Some("Typography"));
        font_label.set_halign(gtk4::Align::Start);
        font_label.add_css_class("heading");
        font_label.set_margin_top(12);
        page.append(&font_label);

        // Header font (theme-aware)
        let header_font_selector = Rc::new(ThemeFontSelector::new(FontSource::custom(
            config.borrow().frame.header_font.clone(),
            config.borrow().frame.header_font_size,
        )));
        header_font_selector.set_theme_config(config.borrow().frame.theme.clone());
        page.append(header_font_selector.widget());

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        header_font_selector.set_on_change(move |font_source| {
            let (family, size) = font_source.resolve(&config_clone.borrow().frame.theme);
            config_clone.borrow_mut().frame.header_font = family;
            config_clone.borrow_mut().frame.header_font_size = size;
            combo_config_base::notify_change(&on_change_clone);
        });

        let selector_for_refresh = header_font_selector.clone();
        let config_for_refresh = config.clone();
        theme_ref_refreshers.borrow_mut().push(Rc::new(move || {
            selector_for_refresh.set_theme_config(config_for_refresh.borrow().frame.theme.clone());
        }));

        // Header color (theme-aware)
        let header_color_box = GtkBox::new(Orientation::Horizontal, 6);
        header_color_box.append(&Label::new(Some("Text Color:")));
        let header_color_widget = Rc::new(ThemeColorSelector::new(ColorSource::custom(
            config.borrow().frame.header_color,
        )));
        header_color_widget.set_theme_config(config.borrow().frame.theme.clone());
        header_color_box.append(header_color_widget.widget());

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        header_color_widget.set_on_change(move |new_source| {
            let color = new_source.resolve(&config_clone.borrow().frame.theme);
            config_clone.borrow_mut().frame.header_color = color;
            combo_config_base::notify_change(&on_change_clone);
        });

        let widget_for_refresh = header_color_widget.clone();
        let config_for_refresh = config.clone();
        theme_ref_refreshers.borrow_mut().push(Rc::new(move || {
            widget_for_refresh.set_theme_config(config_for_refresh.borrow().frame.theme.clone());
        }));
        page.append(&header_color_box);

        // Store widget refs - set to None since we changed to theme-aware selectors
        *header_widgets_out.borrow_mut() = None; // TODO: Update HeaderWidgets struct

        page
    }

    fn create_layout_page(
        config: &Rc<RefCell<IndustrialDisplayConfig>>,
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
            &["Horizontal (side by side)", "Vertical (stacked)"],
            orient_idx,
            |cfg, idx| {
                cfg.frame.split_orientation = if idx == 0 {
                    SplitOrientation::Horizontal
                } else {
                    SplitOrientation::Vertical
                }
            },
        );

        let content_padding_spin = builder.spin_row(
            &page,
            "Content Padding:",
            4.0,
            48.0,
            4.0,
            config.borrow().frame.content_padding,
            |cfg, v| cfg.frame.content_padding = v,
        );

        let item_spacing_spin = builder.spin_row(
            &page,
            "Item Spacing:",
            2.0,
            32.0,
            2.0,
            config.borrow().frame.item_spacing,
            |cfg, v| cfg.frame.item_spacing = v,
        );

        // Dividers section
        let dividers_label = create_section_header("Dividers");
        dividers_label.set_margin_top(12);
        page.append(&dividers_label);

        let div_style_idx = match config.borrow().frame.divider_style {
            DividerStyle::Groove => 0,
            DividerStyle::Raised => 1,
            DividerStyle::Warning => 2,
            DividerStyle::None => 3,
        };
        let divider_style_dropdown = builder.dropdown_row(
            &page,
            "Style:",
            &["Groove", "Raised Bar", "Warning Stripes", "None"],
            div_style_idx,
            |cfg, idx| {
                cfg.frame.divider_style = match idx {
                    0 => DividerStyle::Groove,
                    1 => DividerStyle::Raised,
                    2 => DividerStyle::Warning,
                    _ => DividerStyle::None,
                }
            },
        );

        let divider_width_spin = builder.spin_row(
            &page,
            "Width:",
            2.0,
            16.0,
            1.0,
            config.borrow().frame.divider_width,
            |cfg, v| cfg.frame.divider_width = v,
        );

        // Divider color (theme-aware, resolves to Color)
        let div_color_box = GtkBox::new(Orientation::Horizontal, 6);
        div_color_box.append(&Label::new(Some("Color:")));
        let divider_color_widget = Rc::new(ThemeColorSelector::new(ColorSource::custom(
            config.borrow().frame.divider_color,
        )));
        divider_color_widget.set_theme_config(config.borrow().frame.theme.clone());
        div_color_box.append(divider_color_widget.widget());
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        divider_color_widget.set_on_change(move |new_source| {
            let color = new_source.resolve(&config_clone.borrow().frame.theme);
            config_clone.borrow_mut().frame.divider_color = color;
            combo_config_base::notify_change(&on_change_clone);
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
            |c: &mut IndustrialDisplayConfig| &mut c.frame,
            on_change,
            preview,
        );

        // Store widget refs
        *layout_widgets_out.borrow_mut() = Some(LayoutWidgets {
            split_orientation_dropdown,
            content_padding_spin,
            item_spacing_spin,
            divider_style_dropdown,
            divider_width_spin,
            divider_color_widget,
            group_settings_box,
        });

        page
    }

    /// Create the Theme configuration page
    fn create_theme_page(
        config: &Rc<RefCell<IndustrialDisplayConfig>>,
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
            "Industrial (Default)",
            "Cyberpunk",
            "Synthwave",
            "LCARS",
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
                0 => ComboThemeConfig::default_for_industrial(),
                1 => ComboThemeConfig::default_for_cyberpunk(),
                2 => ComboThemeConfig::default_for_synthwave(),
                3 => ComboThemeConfig::default_for_lcars(),
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
        config: &Rc<RefCell<IndustrialDisplayConfig>>,
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
        config: &Rc<RefCell<IndustrialDisplayConfig>>,
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

    /// Default bar config with industrial/gauge colors
    #[allow(dead_code)]
    #[allow(clippy::field_reassign_with_default)]
    fn default_bar_config_industrial() -> crate::ui::BarDisplayConfig {
        use crate::ui::background::Color;
        use crate::ui::bar_display::{
            BarBackgroundType, BarDisplayConfig, BarFillDirection, BarFillType, BarOrientation,
            BarStyle, BorderConfig,
        };

        let mut config = BarDisplayConfig::default();
        config.style = BarStyle::Full;
        config.orientation = BarOrientation::Horizontal;
        config.fill_direction = BarFillDirection::LeftToRight;

        // Industrial green/amber
        config.foreground = BarFillType::Solid {
            color: crate::ui::theme::ColorSource::custom(Color {
                r: 0.2,
                g: 0.7,
                b: 0.2,
                a: 1.0,
            }),
        };
        config.background = BarBackgroundType::Solid {
            color: crate::ui::theme::ColorSource::custom(Color {
                r: 0.2,
                g: 0.2,
                b: 0.2,
                a: 1.0,
            }),
        };
        config.border = BorderConfig {
            enabled: true,
            color: crate::ui::theme::ColorSource::custom(Color {
                r: 0.4,
                g: 0.4,
                b: 0.4,
                a: 1.0,
            }),
            width: 2.0,
        };
        config.corner_radius = 2.0;

        config
    }

    /// Default graph config with industrial colors
    #[allow(dead_code)]
    #[allow(clippy::field_reassign_with_default)]
    fn default_graph_config_industrial() -> crate::ui::GraphDisplayConfig {
        use crate::ui::background::Color;
        use crate::ui::graph_display::{FillMode, GraphDisplayConfig, GraphType, LineStyle};

        let mut config = GraphDisplayConfig::default();
        config.graph_type = GraphType::Line;
        config.line_style = LineStyle::Solid;
        config.line_width = 2.0;
        config.line_color = ColorSource::custom(Color {
            r: 0.2,
            g: 0.8,
            b: 0.2,
            a: 1.0,
        }); // Industrial green
        config.fill_mode = FillMode::Gradient;
        config.fill_gradient_start = ColorSource::custom(Color {
            r: 0.2,
            g: 0.8,
            b: 0.2,
            a: 0.3,
        });
        config.fill_gradient_end = ColorSource::custom(Color {
            r: 0.2,
            g: 0.8,
            b: 0.2,
            a: 0.0,
        });
        config.background_color = Color {
            r: 0.15,
            g: 0.15,
            b: 0.15,
            a: 1.0,
        };
        config.plot_background_color = Color {
            r: 0.1,
            g: 0.1,
            b: 0.1,
            a: 1.0,
        };
        config.x_axis.show_grid = true;
        config.x_axis.grid_color = ColorSource::custom(Color {
            r: 0.25,
            g: 0.25,
            b: 0.25,
            a: 1.0,
        });
        config.y_axis.show_grid = true;
        config.y_axis.grid_color = ColorSource::custom(Color {
            r: 0.25,
            g: 0.25,
            b: 0.25,
            a: 1.0,
        });

        config
    }

    fn create_animation_page(
        config: &Rc<RefCell<IndustrialDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        animation_widgets_out: &Rc<RefCell<Option<AnimationWidgets>>>,
    ) -> GtkBox {
        let (page, base_widgets) = combo_config_base::create_animation_page_with_widgets(
            config,
            on_change,
            |c| c.animation_enabled,
            |c, v| c.animation_enabled = v,
            |c| c.animation_speed,
            |c, v| c.animation_speed = v,
        );

        // Add any Industrial-specific options here if present...

        *animation_widgets_out.borrow_mut() = Some(AnimationWidgets {
            enable_check: base_widgets.enable_check,
            speed_spin: base_widgets.speed_spin,
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

    pub fn get_config(&self) -> IndustrialDisplayConfig {
        self.config.borrow().clone()
    }

    /// Get a reference to the internal config Rc for use in callbacks
    pub fn get_config_rc(&self) -> Rc<RefCell<IndustrialDisplayConfig>> {
        self.config.clone()
    }

    pub fn set_config(&self, config: &IndustrialDisplayConfig) {
        // IMPORTANT: Temporarily disable on_change callback to prevent signal cascade.
        let saved_callback = self.on_change.borrow_mut().take();

        *self.config.borrow_mut() = config.clone();

        // Update Surface widgets
        if let Some(widgets) = self.surface_widgets.borrow().as_ref() {
            widgets
                .texture_dropdown
                .set_selected(match config.frame.surface_texture {
                    SurfaceTexture::BrushedMetal => 0,
                    SurfaceTexture::CarbonFiber => 1,
                    SurfaceTexture::DiamondPlate => 2,
                    SurfaceTexture::Solid => 3,
                });
            widgets
                .surface_color_widget
                .set_color(config.frame.surface_color);
            widgets
                .surface_dark_widget
                .set_color(config.frame.surface_color_dark);
            widgets
                .highlight_color_widget
                .set_color(config.frame.highlight_color);
        }

        // Update Border widgets
        if let Some(widgets) = self.border_widgets.borrow().as_ref() {
            widgets
                .show_border_check
                .set_active(config.frame.show_border);
            widgets
                .border_width_spin
                .set_value(config.frame.border_width);
            widgets
                .border_color_widget
                .set_color(config.frame.border_color);
            widgets
                .corner_radius_spin
                .set_value(config.frame.corner_radius);
            widgets
                .show_bevel_check
                .set_active(config.frame.show_beveled_edge);
            widgets.bevel_width_spin.set_value(config.frame.bevel_width);
        }

        // Update Rivet widgets
        if let Some(widgets) = self.rivet_widgets.borrow().as_ref() {
            widgets
                .rivet_style_dropdown
                .set_selected(match config.frame.rivet_style {
                    RivetStyle::Hex => 0,
                    RivetStyle::Phillips => 1,
                    RivetStyle::Flat => 2,
                    RivetStyle::None => 3,
                });
            widgets.rivet_size_spin.set_value(config.frame.rivet_size);
            widgets
                .rivet_color_widget
                .set_color(config.frame.rivet_color);
            widgets
                .rivet_spacing_spin
                .set_value(config.frame.rivet_spacing);
            widgets
                .show_corner_rivets_check
                .set_active(config.frame.show_corner_rivets);
            widgets
                .show_edge_rivets_check
                .set_active(config.frame.show_edge_rivets);
        }

        // Update Warning widgets
        if let Some(widgets) = self.warning_widgets.borrow().as_ref() {
            widgets
                .position_dropdown
                .set_selected(match config.frame.warning_stripe_position {
                    WarningStripePosition::None => 0,
                    WarningStripePosition::Top => 1,
                    WarningStripePosition::Bottom => 2,
                    WarningStripePosition::Left => 3,
                    WarningStripePosition::Right => 4,
                    WarningStripePosition::All => 5,
                });
            widgets
                .stripe_width_spin
                .set_value(config.frame.warning_stripe_width);
            widgets
                .color1_widget
                .set_color(config.frame.warning_color_1);
            widgets
                .color2_widget
                .set_color(config.frame.warning_color_2);
            widgets
                .angle_spin
                .set_value(config.frame.warning_stripe_angle);
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
                    HeaderStyle::Plate => 0,
                    HeaderStyle::Stencil => 1,
                    HeaderStyle::Label => 2,
                    HeaderStyle::None => 3,
                });
            widgets
                .header_height_spin
                .set_value(config.frame.header_height);
            widgets.header_font_btn.set_label(&config.frame.header_font);
            widgets
                .header_font_size_spin
                .set_value(config.frame.header_font_size);
            widgets
                .header_color_widget
                .set_color(config.frame.header_color);
        }

        // Update Layout widgets
        if let Some(widgets) = self.layout_widgets.borrow().as_ref() {
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
                .item_spacing_spin
                .set_value(config.frame.item_spacing);
            widgets
                .divider_style_dropdown
                .set_selected(match config.frame.divider_style {
                    DividerStyle::Groove => 0,
                    DividerStyle::Raised => 1,
                    DividerStyle::Warning => 2,
                    DividerStyle::None => 3,
                });
            widgets
                .divider_width_spin
                .set_value(config.frame.divider_width);
            widgets
                .divider_color_widget
                .set_source(ColorSource::custom(config.frame.divider_color));
            widgets
                .divider_color_widget
                .set_theme_config(config.frame.theme.clone());

            combo_config_base::rebuild_combined_group_settings(
                &widgets.group_settings_box,
                &self.config,
                |c: &mut IndustrialDisplayConfig| &mut c.frame,
                &self.on_change,
                &self.preview,
            );
        }

        // Update Animation widgets
        if let Some(widgets) = self.animation_widgets.borrow().as_ref() {
            widgets.enable_check.set_active(config.animation_enabled);
            widgets.speed_spin.set_value(config.animation_speed);
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
                |c: &mut IndustrialDisplayConfig| &mut c.frame,
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
            item_spacing: config.frame.item_spacing,
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
            config.frame.item_spacing = transfer.item_spacing;
            config.animation_enabled = transfer.animation_enabled;
            config.animation_speed = transfer.animation_speed;
        }
    }

    /// Cleanup method to break reference cycles and allow garbage collection.
    pub fn cleanup(&self) {
        log::debug!("IndustrialConfigWidget::cleanup() - breaking reference cycles");
        combo_config_base::cleanup_common_fields_with_content(
            &self.on_change,
            &self.theme_ref_refreshers,
            &self.content_cleanup_callbacks,
        );
        *self.surface_widgets.borrow_mut() = None;
        *self.border_widgets.borrow_mut() = None;
        *self.rivet_widgets.borrow_mut() = None;
        *self.warning_widgets.borrow_mut() = None;
        *self.header_widgets.borrow_mut() = None;
        *self.layout_widgets.borrow_mut() = None;
        *self.animation_widgets.borrow_mut() = None;
        *self.theme_widgets.borrow_mut() = None;
    }
}

impl Default for IndustrialConfigWidget {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}
