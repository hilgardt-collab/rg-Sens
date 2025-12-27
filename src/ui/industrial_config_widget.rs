//! Industrial/Gauge Panel configuration widget
//!
//! Provides a tabbed interface for configuring all aspects of the Industrial display.

use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, CheckButton, DrawingArea, DropDown, Entry, Label, Notebook, Orientation,
    Scale, SpinButton, StringList, ScrolledWindow,
};
use std::cell::RefCell;
use std::rc::Rc;

use crate::ui::shared_font_dialog::shared_font_dialog;
use crate::ui::color_button_widget::ColorButtonWidget;
use crate::ui::industrial_display::{
    render_industrial_frame, SurfaceTexture, RivetStyle, WarningStripePosition,
    HeaderStyle, DividerStyle,
};
use crate::ui::graph_config_widget::GraphConfigWidget;
use crate::ui::bar_config_widget::BarConfigWidget;
use crate::ui::core_bars_config_widget::CoreBarsConfigWidget;
use crate::ui::background_config_widget::BackgroundConfigWidget;
use crate::ui::text_line_config_widget::TextLineConfigWidget;
use crate::ui::arc_config_widget::ArcConfigWidget;
use crate::ui::speedometer_config_widget::SpeedometerConfigWidget;
use crate::ui::lcars_display::{ContentDisplayType, ContentItemConfig, StaticDisplayConfig, SplitOrientation};
use crate::displayers::IndustrialDisplayConfig;
use crate::core::{FieldMetadata, FieldType, FieldPurpose};

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
    divider_color_widget: Rc<ColorButtonWidget>,
    group_weights_box: GtkBox,
}

/// Holds references to Animation tab widgets
struct AnimationWidgets {
    enable_check: CheckButton,
    speed_scale: Scale,
}

/// Holds references to Theme tab widgets
#[allow(dead_code)]
struct ThemeWidgets {
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
    theme_ref_refreshers: Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
}

impl IndustrialConfigWidget {
    pub fn new(available_fields: Vec<FieldMetadata>) -> Self {
        let container = GtkBox::new(Orientation::Vertical, 12);
        let config = Rc::new(RefCell::new(IndustrialDisplayConfig::default()));
        let on_change: Rc<RefCell<Option<Box<dyn Fn()>>>> = Rc::new(RefCell::new(None));
        let source_summaries: Rc<RefCell<Vec<(String, String, usize, u32)>>> = Rc::new(RefCell::new(Vec::new()));
        let available_fields: Rc<RefCell<Vec<FieldMetadata>>> = Rc::new(RefCell::new(available_fields));
        let surface_widgets: Rc<RefCell<Option<SurfaceWidgets>>> = Rc::new(RefCell::new(None));
        let border_widgets: Rc<RefCell<Option<BorderWidgets>>> = Rc::new(RefCell::new(None));
        let rivet_widgets: Rc<RefCell<Option<RivetWidgets>>> = Rc::new(RefCell::new(None));
        let warning_widgets: Rc<RefCell<Option<WarningWidgets>>> = Rc::new(RefCell::new(None));
        let header_widgets: Rc<RefCell<Option<HeaderWidgets>>> = Rc::new(RefCell::new(None));
        let layout_widgets: Rc<RefCell<Option<LayoutWidgets>>> = Rc::new(RefCell::new(None));
        let animation_widgets: Rc<RefCell<Option<AnimationWidgets>>> = Rc::new(RefCell::new(None));
        let theme_widgets: Rc<RefCell<Option<ThemeWidgets>>> = Rc::new(RefCell::new(None));
        let theme_ref_refreshers: Rc<RefCell<Vec<Rc<dyn Fn()>>>> = Rc::new(RefCell::new(Vec::new()));

        // Preview at the top
        let preview = DrawingArea::new();
        preview.set_content_height(200);
        preview.set_vexpand(false);

        let config_clone = config.clone();
        preview.set_draw_func(move |_, cr, width, height| {
            if width < 10 || height < 10 {
                return;
            }

            let cfg = config_clone.borrow();
            let _ = render_industrial_frame(cr, &cfg.frame, width as f64, height as f64);
        });

        // Create notebook for tabbed interface
        let notebook = Notebook::new();
        notebook.set_vexpand(true);

        // Tab 1: Surface
        let surface_page = Self::create_surface_page(&config, &on_change, &preview, &surface_widgets);
        notebook.append_page(&surface_page, Some(&Label::new(Some("Surface"))));

        // Tab 2: Border
        let border_page = Self::create_border_page(&config, &on_change, &preview, &border_widgets);
        notebook.append_page(&border_page, Some(&Label::new(Some("Border"))));

        // Tab 3: Rivets
        let rivet_page = Self::create_rivet_page(&config, &on_change, &preview, &rivet_widgets);
        notebook.append_page(&rivet_page, Some(&Label::new(Some("Rivets"))));

        // Tab 4: Warning Stripes
        let warning_page = Self::create_warning_page(&config, &on_change, &preview, &warning_widgets);
        notebook.append_page(&warning_page, Some(&Label::new(Some("Warning"))));

        // Tab 5: Header
        let header_page = Self::create_header_page(&config, &on_change, &preview, &header_widgets);
        notebook.append_page(&header_page, Some(&Label::new(Some("Header"))));

        // Tab 6: Layout
        let layout_page = Self::create_layout_page(&config, &on_change, &preview, &layout_widgets);
        notebook.append_page(&layout_page, Some(&Label::new(Some("Layout"))));

        // Tab 7: Theme
        let theme_page = Self::create_theme_page(&config, &on_change, &preview, &theme_widgets, &theme_ref_refreshers);
        notebook.append_page(&theme_page, Some(&Label::new(Some("Theme"))));

        // Tab 8: Content
        let content_notebook = Rc::new(RefCell::new(Notebook::new()));
        let content_page = Self::create_content_page(&config, &on_change, &preview, &content_notebook, &source_summaries, &available_fields, &theme_ref_refreshers);
        notebook.append_page(&content_page, Some(&Label::new(Some("Content"))));

        // Tab 9: Animation
        let animation_page = Self::create_animation_page(&config, &on_change, &animation_widgets);
        notebook.append_page(&animation_page, Some(&Label::new(Some("Animation"))));

        container.append(&preview);
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
        }
    }

    fn set_page_margins(page: &GtkBox) {
        page.set_margin_start(12);
        page.set_margin_end(12);
        page.set_margin_top(12);
        page.set_margin_bottom(12);
    }

    fn queue_redraw(
        preview: &DrawingArea,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
    ) {
        preview.queue_draw();
        if let Some(cb) = on_change.borrow().as_ref() {
            cb();
        }
    }

    fn create_surface_page(
        config: &Rc<RefCell<IndustrialDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        surface_widgets_out: &Rc<RefCell<Option<SurfaceWidgets>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        Self::set_page_margins(&page);

        // Surface texture
        let texture_box = GtkBox::new(Orientation::Horizontal, 6);
        texture_box.append(&Label::new(Some("Texture:")));
        let texture_list = StringList::new(&["Brushed Metal", "Carbon Fiber", "Diamond Plate", "Solid"]);
        let texture_dropdown = DropDown::new(Some(texture_list), None::<gtk4::Expression>);
        let texture_idx = match config.borrow().frame.surface_texture {
            SurfaceTexture::BrushedMetal => 0,
            SurfaceTexture::CarbonFiber => 1,
            SurfaceTexture::DiamondPlate => 2,
            SurfaceTexture::Solid => 3,
        };
        texture_dropdown.set_selected(texture_idx);
        texture_dropdown.set_hexpand(true);
        texture_box.append(&texture_dropdown);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
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
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&texture_box);

        // Colors section
        let colors_label = Label::new(Some("Surface Colors"));
        colors_label.set_halign(gtk4::Align::Start);
        colors_label.add_css_class("heading");
        colors_label.set_margin_top(12);
        page.append(&colors_label);

        // Surface color
        let surface_box = GtkBox::new(Orientation::Horizontal, 6);
        surface_box.append(&Label::new(Some("Base Color:")));
        let surface_color_widget = Rc::new(ColorButtonWidget::new(config.borrow().frame.surface_color));
        surface_box.append(surface_color_widget.widget());

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        surface_color_widget.set_on_change(move |color| {
            config_clone.borrow_mut().frame.surface_color = color;
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&surface_box);

        // Dark surface color
        let surface_dark_box = GtkBox::new(Orientation::Horizontal, 6);
        surface_dark_box.append(&Label::new(Some("Dark Color:")));
        let surface_dark_widget = Rc::new(ColorButtonWidget::new(config.borrow().frame.surface_color_dark));
        surface_dark_box.append(surface_dark_widget.widget());

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        surface_dark_widget.set_on_change(move |color| {
            config_clone.borrow_mut().frame.surface_color_dark = color;
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&surface_dark_box);

        // Highlight color
        let highlight_box = GtkBox::new(Orientation::Horizontal, 6);
        highlight_box.append(&Label::new(Some("Highlight:")));
        let highlight_color_widget = Rc::new(ColorButtonWidget::new(config.borrow().frame.highlight_color));
        highlight_box.append(highlight_color_widget.widget());

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        highlight_color_widget.set_on_change(move |color| {
            config_clone.borrow_mut().frame.highlight_color = color;
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&highlight_box);

        // Store widget refs
        *surface_widgets_out.borrow_mut() = Some(SurfaceWidgets {
            texture_dropdown,
            surface_color_widget,
            surface_dark_widget,
            highlight_color_widget,
        });

        page
    }

    fn create_border_page(
        config: &Rc<RefCell<IndustrialDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        border_widgets_out: &Rc<RefCell<Option<BorderWidgets>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        Self::set_page_margins(&page);

        // Show border
        let show_border_check = CheckButton::with_label("Show Border");
        show_border_check.set_active(config.borrow().frame.show_border);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        show_border_check.connect_toggled(move |check| {
            config_clone.borrow_mut().frame.show_border = check.is_active();
            Self::queue_redraw(&preview_clone, &on_change_clone);
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
        let preview_clone = preview.clone();
        border_width_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().frame.border_width = spin.value();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&width_box);

        // Border color
        let color_box = GtkBox::new(Orientation::Horizontal, 6);
        color_box.append(&Label::new(Some("Border Color:")));
        let border_color_widget = Rc::new(ColorButtonWidget::new(config.borrow().frame.border_color));
        color_box.append(border_color_widget.widget());

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        border_color_widget.set_on_change(move |color| {
            config_clone.borrow_mut().frame.border_color = color;
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
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
        let preview_clone = preview.clone();
        corner_radius_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().frame.corner_radius = spin.value();
            Self::queue_redraw(&preview_clone, &on_change_clone);
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
        let preview_clone = preview.clone();
        show_bevel_check.connect_toggled(move |check| {
            config_clone.borrow_mut().frame.show_beveled_edge = check.is_active();
            Self::queue_redraw(&preview_clone, &on_change_clone);
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
        let preview_clone = preview.clone();
        bevel_width_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().frame.bevel_width = spin.value();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&bevel_width_box);

        // Store widget refs
        *border_widgets_out.borrow_mut() = Some(BorderWidgets {
            show_border_check,
            border_width_spin,
            border_color_widget,
            corner_radius_spin,
            show_bevel_check,
            bevel_width_spin,
        });

        page
    }

    fn create_rivet_page(
        config: &Rc<RefCell<IndustrialDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        rivet_widgets_out: &Rc<RefCell<Option<RivetWidgets>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        Self::set_page_margins(&page);

        // Rivet style
        let style_box = GtkBox::new(Orientation::Horizontal, 6);
        style_box.append(&Label::new(Some("Rivet Style:")));
        let style_list = StringList::new(&["Hex Bolt", "Phillips Screw", "Flat Rivet", "None"]);
        let rivet_style_dropdown = DropDown::new(Some(style_list), None::<gtk4::Expression>);
        let style_idx = match config.borrow().frame.rivet_style {
            RivetStyle::Hex => 0,
            RivetStyle::Phillips => 1,
            RivetStyle::Flat => 2,
            RivetStyle::None => 3,
        };
        rivet_style_dropdown.set_selected(style_idx);
        rivet_style_dropdown.set_hexpand(true);
        style_box.append(&rivet_style_dropdown);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
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
            Self::queue_redraw(&preview_clone, &on_change_clone);
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
        let preview_clone = preview.clone();
        rivet_size_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().frame.rivet_size = spin.value();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&size_box);

        // Rivet color
        let color_box = GtkBox::new(Orientation::Horizontal, 6);
        color_box.append(&Label::new(Some("Color:")));
        let rivet_color_widget = Rc::new(ColorButtonWidget::new(config.borrow().frame.rivet_color));
        color_box.append(rivet_color_widget.widget());

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        rivet_color_widget.set_on_change(move |color| {
            config_clone.borrow_mut().frame.rivet_color = color;
            Self::queue_redraw(&preview_clone, &on_change_clone);
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
        let preview_clone = preview.clone();
        rivet_spacing_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().frame.rivet_spacing = spin.value();
            Self::queue_redraw(&preview_clone, &on_change_clone);
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
        let preview_clone = preview.clone();
        show_corner_rivets_check.connect_toggled(move |check| {
            config_clone.borrow_mut().frame.show_corner_rivets = check.is_active();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&show_corner_rivets_check);

        // Edge rivets
        let show_edge_rivets_check = CheckButton::with_label("Show Edge Rivets");
        show_edge_rivets_check.set_active(config.borrow().frame.show_edge_rivets);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        show_edge_rivets_check.connect_toggled(move |check| {
            config_clone.borrow_mut().frame.show_edge_rivets = check.is_active();
            Self::queue_redraw(&preview_clone, &on_change_clone);
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
        preview: &DrawingArea,
        warning_widgets_out: &Rc<RefCell<Option<WarningWidgets>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        Self::set_page_margins(&page);

        // Position
        let position_box = GtkBox::new(Orientation::Horizontal, 6);
        position_box.append(&Label::new(Some("Position:")));
        let position_list = StringList::new(&["None", "Top", "Bottom", "Left", "Right", "All Edges"]);
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
        let preview_clone = preview.clone();
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
            Self::queue_redraw(&preview_clone, &on_change_clone);
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
        let preview_clone = preview.clone();
        stripe_width_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().frame.warning_stripe_width = spin.value();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&width_box);

        // Colors section
        let colors_label = Label::new(Some("Stripe Colors"));
        colors_label.set_halign(gtk4::Align::Start);
        colors_label.add_css_class("heading");
        colors_label.set_margin_top(12);
        page.append(&colors_label);

        // Color 1 (yellow)
        let color1_box = GtkBox::new(Orientation::Horizontal, 6);
        color1_box.append(&Label::new(Some("Color 1:")));
        let color1_widget = Rc::new(ColorButtonWidget::new(config.borrow().frame.warning_color_1));
        color1_box.append(color1_widget.widget());

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        color1_widget.set_on_change(move |color| {
            config_clone.borrow_mut().frame.warning_color_1 = color;
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&color1_box);

        // Color 2 (black)
        let color2_box = GtkBox::new(Orientation::Horizontal, 6);
        color2_box.append(&Label::new(Some("Color 2:")));
        let color2_widget = Rc::new(ColorButtonWidget::new(config.borrow().frame.warning_color_2));
        color2_box.append(color2_widget.widget());

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        color2_widget.set_on_change(move |color| {
            config_clone.borrow_mut().frame.warning_color_2 = color;
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
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
        let preview_clone = preview.clone();
        angle_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().frame.warning_stripe_angle = spin.value();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&angle_box);

        // Store widget refs
        *warning_widgets_out.borrow_mut() = Some(WarningWidgets {
            position_dropdown,
            stripe_width_spin,
            color1_widget,
            color2_widget,
            angle_spin,
        });

        page
    }

    fn create_header_page(
        config: &Rc<RefCell<IndustrialDisplayConfig>>,
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
        let preview_clone = preview.clone();
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

        // Font section
        let font_label = Label::new(Some("Typography"));
        font_label.set_halign(gtk4::Align::Start);
        font_label.add_css_class("heading");
        font_label.set_margin_top(12);
        page.append(&font_label);

        // Header font
        let font_box = GtkBox::new(Orientation::Horizontal, 6);
        font_box.append(&Label::new(Some("Font:")));
        let header_font_btn = Button::with_label(&config.borrow().frame.header_font);
        header_font_btn.set_hexpand(true);
        font_box.append(&header_font_btn);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let font_btn_clone = header_font_btn.clone();
        header_font_btn.connect_clicked(move |btn| {
            let root = btn.root();
            let window = root.as_ref().and_then(|r| r.downcast_ref::<gtk4::Window>());
            let current_font = config_clone.borrow().frame.header_font.clone();
            let config_for_cb = config_clone.clone();
            let on_change_for_cb = on_change_clone.clone();
            let preview_for_cb = preview_clone.clone();
            let font_btn_for_cb = font_btn_clone.clone();

            let font_desc = gtk4::pango::FontDescription::from_string(&current_font);

            shared_font_dialog().choose_font(
                window,
                Some(&font_desc),
                gtk4::gio::Cancellable::NONE,
                move |result| {
                    if let Ok(font_desc) = result {
                        let family = font_desc.family().map(|s| s.to_string()).unwrap_or_else(|| "Sans".to_string());
                        config_for_cb.borrow_mut().frame.header_font = family.clone();
                        font_btn_for_cb.set_label(&family);
                        Self::queue_redraw(&preview_for_cb, &on_change_for_cb);
                    }
                },
            );
        });
        page.append(&font_box);

        // Header font size
        let size_box = GtkBox::new(Orientation::Horizontal, 6);
        size_box.append(&Label::new(Some("Font Size:")));
        let header_font_size_spin = SpinButton::with_range(10.0, 32.0, 1.0);
        header_font_size_spin.set_value(config.borrow().frame.header_font_size);
        header_font_size_spin.set_hexpand(true);
        size_box.append(&header_font_size_spin);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        header_font_size_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().frame.header_font_size = spin.value();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&size_box);

        // Header color
        let header_color_box = GtkBox::new(Orientation::Horizontal, 6);
        header_color_box.append(&Label::new(Some("Text Color:")));
        let header_color_widget = Rc::new(ColorButtonWidget::new(config.borrow().frame.header_color));
        header_color_box.append(header_color_widget.widget());

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        header_color_widget.set_on_change(move |color| {
            config_clone.borrow_mut().frame.header_color = color;
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&header_color_box);

        // Store widget refs
        *header_widgets_out.borrow_mut() = Some(HeaderWidgets {
            show_header_check,
            header_text_entry,
            header_style_dropdown,
            header_height_spin,
            header_font_btn,
            header_font_size_spin,
            header_color_widget,
        });

        page
    }

    fn create_layout_page(
        config: &Rc<RefCell<IndustrialDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        layout_widgets_out: &Rc<RefCell<Option<LayoutWidgets>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        Self::set_page_margins(&page);

        // Split orientation (horizontal vs vertical group layout)
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
        let content_padding_box = GtkBox::new(Orientation::Horizontal, 6);
        content_padding_box.append(&Label::new(Some("Content Padding:")));
        let content_padding_spin = SpinButton::with_range(4.0, 48.0, 4.0);
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
        let item_spacing_spin = SpinButton::with_range(2.0, 32.0, 2.0);
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
        let div_style_list = StringList::new(&["Groove", "Raised Bar", "Warning Stripes", "None"]);
        let divider_style_dropdown = DropDown::new(Some(div_style_list), None::<gtk4::Expression>);
        let div_style_idx = match config.borrow().frame.divider_style {
            DividerStyle::Groove => 0,
            DividerStyle::Raised => 1,
            DividerStyle::Warning => 2,
            DividerStyle::None => 3,
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
                0 => DividerStyle::Groove,
                1 => DividerStyle::Raised,
                2 => DividerStyle::Warning,
                _ => DividerStyle::None,
            };
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&div_style_box);

        // Divider width
        let div_width_box = GtkBox::new(Orientation::Horizontal, 6);
        div_width_box.append(&Label::new(Some("Width:")));
        let divider_width_spin = SpinButton::with_range(2.0, 16.0, 1.0);
        divider_width_spin.set_value(config.borrow().frame.divider_width);
        divider_width_spin.set_hexpand(true);
        div_width_box.append(&divider_width_spin);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        divider_width_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().frame.divider_width = spin.value();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&div_width_box);

        // Divider color
        let div_color_box = GtkBox::new(Orientation::Horizontal, 6);
        div_color_box.append(&Label::new(Some("Color:")));
        let divider_color_widget = Rc::new(ColorButtonWidget::new(config.borrow().frame.divider_color));
        div_color_box.append(divider_color_widget.widget());

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        divider_color_widget.set_on_change(move |color| {
            config_clone.borrow_mut().frame.divider_color = color;
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
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
            split_orientation_dropdown,
            content_padding_spin,
            item_spacing_spin,
            divider_style_dropdown,
            divider_width_spin,
            divider_color_widget,
            group_weights_box,
        });

        page
    }

    fn rebuild_group_spinners(
        config: &Rc<RefCell<IndustrialDisplayConfig>>,
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
                Self::queue_redraw(&preview_clone, &on_change_clone);
            });

            weights_box.append(&row);
        }
    }

    fn refresh_theme_refs(refreshers: &Rc<RefCell<Vec<Rc<dyn Fn()>>>>) {
        for refresh_fn in refreshers.borrow().iter() {
            refresh_fn();
        }
    }

    /// Create the Theme configuration page
    fn create_theme_page(
        config: &Rc<RefCell<IndustrialDisplayConfig>>,
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
        let info_label = Label::new(Some("Configure theme colors, gradient, and fonts.\nThese can be referenced in content items for consistent styling."));
        info_label.set_halign(gtk4::Align::Start);
        info_label.add_css_class("dim-label");
        info_label.set_wrap(true);
        inner_box.append(&info_label);

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
        gradient_editor.set_gradient(&config.borrow().frame.theme.gradient);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let refreshers_clone = theme_ref_refreshers.clone();
        let gradient_editor_for_cb = gradient_editor.clone();
        gradient_editor.set_on_change(move || {
            let gradient_config = gradient_editor_for_cb.get_gradient();
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
                                .unwrap_or_else(|| "sans-serif".to_string());
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
                                .unwrap_or_else(|| "sans-serif".to_string());
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
            // Update UI widgets
            if color_widgets_clone.len() >= 4 {
                color_widgets_clone[0].set_color(theme.color1);
                color_widgets_clone[1].set_color(theme.color2);
                color_widgets_clone[2].set_color(theme.color3);
                color_widgets_clone[3].set_color(theme.color4);
            }
            gradient_editor_for_preset.set_gradient(&theme.gradient);
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
    fn create_theme_reference_section(
        config: &Rc<RefCell<IndustrialDisplayConfig>>,
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
            let gradient_config = config_for_gradient.borrow().frame.theme.gradient.clone();
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
            let stops = config_for_gradient_copy.borrow().frame.theme.gradient.stops.clone();
            if let Ok(mut clipboard) = CLIPBOARD.lock() {
                clipboard.copy_gradient_stops(stops);
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
            let config_for_copy = config.clone();
            let font_idx = *idx;
            copy_btn.connect_clicked(move |_| {
                let (family, size) = config_for_copy.borrow().frame.theme.get_font(font_idx);
                if let Ok(mut clipboard) = CLIPBOARD.lock() {
                    clipboard.copy_font(family, size, false, false);
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
        config: &Rc<RefCell<IndustrialDisplayConfig>>,
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
        config: &Rc<RefCell<IndustrialDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        content_notebook: &Rc<RefCell<Notebook>>,
        source_summaries: &Rc<RefCell<Vec<(String, String, usize, u32)>>>,
        available_fields: &Rc<RefCell<Vec<FieldMetadata>>>,
        theme_ref_refreshers: &Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
    ) {
        let notebook = content_notebook.borrow();

        // Clear existing tabs and theme refreshers
        while notebook.n_pages() > 0 {
            notebook.remove_page(Some(0));
        }
        theme_ref_refreshers.borrow_mut().clear();

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
                    let (tab_box, theme_refresh_cb) = Self::create_slot_config_tab(&slot_name, config, on_change, preview, available_fields);
                    theme_ref_refreshers.borrow_mut().push(theme_refresh_cb);
                    items_notebook.append_page(&tab_box, Some(&Label::new(Some(&tab_label))));
                }

                group_box.append(&items_notebook);
                notebook.append_page(&group_box, Some(&Label::new(Some(&format!("Group {}", group_num)))));
            }
        }
    }

    fn create_slot_config_tab(
        slot_name: &str,
        config: &Rc<RefCell<IndustrialDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        available_fields: &Rc<RefCell<Vec<FieldMetadata>>>,
    ) -> (GtkBox, Rc<dyn Fn()>) {
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

        // Theme reference section (shows theme colors, gradient, fonts with copy buttons)
        let (theme_ref_section, theme_refresh_cb) = Self::create_theme_reference_section(config);
        inner_box.append(&theme_ref_section);

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
                .unwrap_or_else(Self::default_bar_config_industrial)
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
                .unwrap_or_else(Self::default_graph_config_industrial)
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

        (tab, theme_refresh_cb)
    }

    /// Default bar config with industrial/gauge colors
    #[allow(clippy::field_reassign_with_default)]
    fn default_bar_config_industrial() -> crate::ui::BarDisplayConfig {
        use crate::ui::bar_display::{BarDisplayConfig, BarStyle, BarOrientation, BarFillDirection, BarFillType, BarBackgroundType, BorderConfig};
        use crate::ui::background::Color;

        let mut config = BarDisplayConfig::default();
        config.style = BarStyle::Full;
        config.orientation = BarOrientation::Horizontal;
        config.fill_direction = BarFillDirection::LeftToRight;

        // Industrial green/amber
        config.foreground = BarFillType::Solid {
            color: Color { r: 0.2, g: 0.7, b: 0.2, a: 1.0 }
        };
        config.background = BarBackgroundType::Solid {
            color: Color { r: 0.2, g: 0.2, b: 0.2, a: 1.0 }
        };
        config.border = BorderConfig {
            enabled: true,
            color: Color { r: 0.4, g: 0.4, b: 0.4, a: 1.0 },
            width: 2.0,
        };
        config.corner_radius = 2.0;

        config
    }

    /// Default graph config with industrial colors
    #[allow(clippy::field_reassign_with_default)]
    fn default_graph_config_industrial() -> crate::ui::GraphDisplayConfig {
        use crate::ui::graph_display::{GraphDisplayConfig, GraphType, LineStyle, FillMode};
        use crate::ui::background::Color;

        let mut config = GraphDisplayConfig::default();
        config.graph_type = GraphType::Line;
        config.line_style = LineStyle::Solid;
        config.line_width = 2.0;
        config.line_color = Color { r: 0.2, g: 0.8, b: 0.2, a: 1.0 };  // Industrial green
        config.fill_mode = FillMode::Gradient;
        config.fill_gradient_start = Color { r: 0.2, g: 0.8, b: 0.2, a: 0.3 };
        config.fill_gradient_end = Color { r: 0.2, g: 0.8, b: 0.2, a: 0.0 };
        config.background_color = Color { r: 0.15, g: 0.15, b: 0.15, a: 1.0 };
        config.plot_background_color = Color { r: 0.1, g: 0.1, b: 0.1, a: 1.0 };
        config.x_axis.show_grid = true;
        config.x_axis.grid_color = Color { r: 0.25, g: 0.25, b: 0.25, a: 1.0 };
        config.y_axis.show_grid = true;
        config.y_axis.grid_color = Color { r: 0.25, g: 0.25, b: 0.25, a: 1.0 };

        config
    }

    fn create_animation_page(
        config: &Rc<RefCell<IndustrialDisplayConfig>>,
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

    pub fn get_config(&self) -> IndustrialDisplayConfig {
        self.config.borrow().clone()
    }

    pub fn set_config(&self, config: &IndustrialDisplayConfig) {
        *self.config.borrow_mut() = config.clone();

        // Update Surface widgets
        if let Some(widgets) = self.surface_widgets.borrow().as_ref() {
            widgets.texture_dropdown.set_selected(match config.frame.surface_texture {
                SurfaceTexture::BrushedMetal => 0,
                SurfaceTexture::CarbonFiber => 1,
                SurfaceTexture::DiamondPlate => 2,
                SurfaceTexture::Solid => 3,
            });
            widgets.surface_color_widget.set_color(config.frame.surface_color);
            widgets.surface_dark_widget.set_color(config.frame.surface_color_dark);
            widgets.highlight_color_widget.set_color(config.frame.highlight_color);
        }

        // Update Border widgets
        if let Some(widgets) = self.border_widgets.borrow().as_ref() {
            widgets.show_border_check.set_active(config.frame.show_border);
            widgets.border_width_spin.set_value(config.frame.border_width);
            widgets.border_color_widget.set_color(config.frame.border_color);
            widgets.corner_radius_spin.set_value(config.frame.corner_radius);
            widgets.show_bevel_check.set_active(config.frame.show_beveled_edge);
            widgets.bevel_width_spin.set_value(config.frame.bevel_width);
        }

        // Update Rivet widgets
        if let Some(widgets) = self.rivet_widgets.borrow().as_ref() {
            widgets.rivet_style_dropdown.set_selected(match config.frame.rivet_style {
                RivetStyle::Hex => 0,
                RivetStyle::Phillips => 1,
                RivetStyle::Flat => 2,
                RivetStyle::None => 3,
            });
            widgets.rivet_size_spin.set_value(config.frame.rivet_size);
            widgets.rivet_color_widget.set_color(config.frame.rivet_color);
            widgets.rivet_spacing_spin.set_value(config.frame.rivet_spacing);
            widgets.show_corner_rivets_check.set_active(config.frame.show_corner_rivets);
            widgets.show_edge_rivets_check.set_active(config.frame.show_edge_rivets);
        }

        // Update Warning widgets
        if let Some(widgets) = self.warning_widgets.borrow().as_ref() {
            widgets.position_dropdown.set_selected(match config.frame.warning_stripe_position {
                WarningStripePosition::None => 0,
                WarningStripePosition::Top => 1,
                WarningStripePosition::Bottom => 2,
                WarningStripePosition::Left => 3,
                WarningStripePosition::Right => 4,
                WarningStripePosition::All => 5,
            });
            widgets.stripe_width_spin.set_value(config.frame.warning_stripe_width);
            widgets.color1_widget.set_color(config.frame.warning_color_1);
            widgets.color2_widget.set_color(config.frame.warning_color_2);
            widgets.angle_spin.set_value(config.frame.warning_stripe_angle);
        }

        // Update Header widgets
        if let Some(widgets) = self.header_widgets.borrow().as_ref() {
            widgets.show_header_check.set_active(config.frame.show_header);
            widgets.header_text_entry.set_text(&config.frame.header_text);
            widgets.header_style_dropdown.set_selected(match config.frame.header_style {
                HeaderStyle::Plate => 0,
                HeaderStyle::Stencil => 1,
                HeaderStyle::Label => 2,
                HeaderStyle::None => 3,
            });
            widgets.header_height_spin.set_value(config.frame.header_height);
            widgets.header_font_btn.set_label(&config.frame.header_font);
            widgets.header_font_size_spin.set_value(config.frame.header_font_size);
            widgets.header_color_widget.set_color(config.frame.header_color);
        }

        // Update Layout widgets
        if let Some(widgets) = self.layout_widgets.borrow().as_ref() {
            widgets.split_orientation_dropdown.set_selected(match config.frame.split_orientation {
                SplitOrientation::Horizontal => 0,
                SplitOrientation::Vertical => 1,
            });
            widgets.content_padding_spin.set_value(config.frame.content_padding);
            widgets.item_spacing_spin.set_value(config.frame.item_spacing);
            widgets.divider_style_dropdown.set_selected(match config.frame.divider_style {
                DividerStyle::Groove => 0,
                DividerStyle::Raised => 1,
                DividerStyle::Warning => 2,
                DividerStyle::None => 3,
            });
            widgets.divider_width_spin.set_value(config.frame.divider_width);
            widgets.divider_color_widget.set_color(config.frame.divider_color);

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

        // Rebuild content tabs to reflect new field options
        Self::rebuild_content_tabs(
            &self.config,
            &self.on_change,
            &self.preview,
            &self.content_notebook,
            &self.source_summaries,
            &self.available_fields,
            &self.theme_ref_refreshers,
        );
    }
}

impl Default for IndustrialConfigWidget {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}
