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
    render_synthwave_frame, SynthwaveColorScheme, SynthwaveFrameStyle,
    GridStyle, SynthwaveHeaderStyle, SynthwaveDividerStyle,
};
use crate::ui::lcars_display::{ContentDisplayType, ContentItemConfig, SplitOrientation, StaticDisplayConfig};
use crate::ui::background::Color;
use crate::ui::{
    BarConfigWidget, GraphConfigWidget, TextLineConfigWidget, CoreBarsConfigWidget,
    BackgroundConfigWidget, ArcConfigWidget, SpeedometerConfigWidget,
};
use crate::displayers::SynthwaveDisplayConfig;
use crate::core::{FieldMetadata, FieldType, FieldPurpose};

/// Holds references to Colors tab widgets
struct ColorsWidgets {
    scheme_dropdown: DropDown,
    custom_primary_widget: Rc<ColorButtonWidget>,
    custom_secondary_widget: Rc<ColorButtonWidget>,
    custom_accent_widget: Rc<ColorButtonWidget>,
    custom_colors_box: GtkBox,
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
    header_font_btn: Button,
    header_font_size_spin: SpinButton,
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
    colors_widgets: Rc<RefCell<Option<ColorsWidgets>>>,
    frame_widgets: Rc<RefCell<Option<FrameWidgets>>>,
    grid_widgets: Rc<RefCell<Option<GridWidgets>>>,
    header_widgets: Rc<RefCell<Option<HeaderWidgets>>>,
    layout_widgets: Rc<RefCell<Option<LayoutWidgets>>>,
    animation_widgets: Rc<RefCell<Option<AnimationWidgets>>>,
}

impl SynthwaveConfigWidget {
    pub fn new(available_fields: Vec<FieldMetadata>) -> Self {
        let container = GtkBox::new(Orientation::Vertical, 12);
        let config = Rc::new(RefCell::new(SynthwaveDisplayConfig::default()));
        let on_change: Rc<RefCell<Option<Box<dyn Fn()>>>> = Rc::new(RefCell::new(None));
        let source_summaries: Rc<RefCell<Vec<(String, String, usize, u32)>>> = Rc::new(RefCell::new(Vec::new()));
        let available_fields: Rc<RefCell<Vec<FieldMetadata>>> = Rc::new(RefCell::new(available_fields));
        let colors_widgets: Rc<RefCell<Option<ColorsWidgets>>> = Rc::new(RefCell::new(None));
        let frame_widgets: Rc<RefCell<Option<FrameWidgets>>> = Rc::new(RefCell::new(None));
        let grid_widgets: Rc<RefCell<Option<GridWidgets>>> = Rc::new(RefCell::new(None));
        let header_widgets: Rc<RefCell<Option<HeaderWidgets>>> = Rc::new(RefCell::new(None));
        let layout_widgets: Rc<RefCell<Option<LayoutWidgets>>> = Rc::new(RefCell::new(None));
        let animation_widgets: Rc<RefCell<Option<AnimationWidgets>>> = Rc::new(RefCell::new(None));

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

        // Create notebook for tabbed interface
        let notebook = Notebook::new();
        notebook.set_vexpand(true);

        // Tab 1: Colors
        let colors_page = Self::create_colors_page(&config, &on_change, &preview, &colors_widgets);
        notebook.append_page(&colors_page, Some(&Label::new(Some("Colors"))));

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
        let content_page = Self::create_content_page(&config, &on_change, &preview, &content_notebook, &source_summaries, &available_fields);
        notebook.append_page(&content_page, Some(&Label::new(Some("Content"))));

        // Tab 7: Animation
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
            colors_widgets,
            frame_widgets,
            grid_widgets,
            header_widgets,
            layout_widgets,
            animation_widgets,
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

    fn create_colors_page(
        config: &Rc<RefCell<SynthwaveDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        colors_widgets_out: &Rc<RefCell<Option<ColorsWidgets>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        Self::set_page_margins(&page);

        // Color scheme preset
        let scheme_box = GtkBox::new(Orientation::Horizontal, 6);
        scheme_box.append(&Label::new(Some("Color Scheme:")));
        let scheme_list = StringList::new(&["Classic", "Sunset", "Night Drive", "Miami", "Custom"]);
        let scheme_dropdown = DropDown::new(Some(scheme_list), None::<gtk4::Expression>);
        let scheme_idx = match &config.borrow().frame.color_scheme {
            SynthwaveColorScheme::Classic => 0,
            SynthwaveColorScheme::Sunset => 1,
            SynthwaveColorScheme::NightDrive => 2,
            SynthwaveColorScheme::Miami => 3,
            SynthwaveColorScheme::Custom { .. } => 4,
        };
        scheme_dropdown.set_selected(scheme_idx);
        scheme_dropdown.set_hexpand(true);
        scheme_box.append(&scheme_dropdown);
        page.append(&scheme_box);

        // Custom colors (shown only when Custom is selected)
        let custom_colors_box = GtkBox::new(Orientation::Vertical, 6);
        custom_colors_box.set_margin_top(8);

        // Primary color
        let primary_box = GtkBox::new(Orientation::Horizontal, 6);
        primary_box.append(&Label::new(Some("Primary:")));
        let custom_primary = if let SynthwaveColorScheme::Custom { primary, .. } = &config.borrow().frame.color_scheme {
            *primary
        } else {
            Color { r: 0.58, g: 0.0, b: 0.83, a: 1.0 }
        };
        let custom_primary_widget = Rc::new(ColorButtonWidget::new(custom_primary));
        primary_box.append(custom_primary_widget.widget());
        custom_colors_box.append(&primary_box);

        // Secondary color
        let secondary_box = GtkBox::new(Orientation::Horizontal, 6);
        secondary_box.append(&Label::new(Some("Secondary:")));
        let custom_secondary = if let SynthwaveColorScheme::Custom { secondary, .. } = &config.borrow().frame.color_scheme {
            *secondary
        } else {
            Color { r: 1.0, g: 0.08, b: 0.58, a: 1.0 }
        };
        let custom_secondary_widget = Rc::new(ColorButtonWidget::new(custom_secondary));
        secondary_box.append(custom_secondary_widget.widget());
        custom_colors_box.append(&secondary_box);

        // Accent color
        let accent_box = GtkBox::new(Orientation::Horizontal, 6);
        accent_box.append(&Label::new(Some("Accent:")));
        let custom_accent = if let SynthwaveColorScheme::Custom { accent, .. } = &config.borrow().frame.color_scheme {
            *accent
        } else {
            Color { r: 0.0, g: 1.0, b: 1.0, a: 1.0 }
        };
        let custom_accent_widget = Rc::new(ColorButtonWidget::new(custom_accent));
        accent_box.append(custom_accent_widget.widget());
        custom_colors_box.append(&accent_box);

        custom_colors_box.set_visible(scheme_idx == 4);
        page.append(&custom_colors_box);

        // Connect scheme dropdown
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let custom_box_clone = custom_colors_box.clone();
        let primary_widget_clone = custom_primary_widget.clone();
        let secondary_widget_clone = custom_secondary_widget.clone();
        let accent_widget_clone = custom_accent_widget.clone();
        scheme_dropdown.connect_selected_notify(move |dropdown| {
            let selected = dropdown.selected();
            if selected == gtk4::INVALID_LIST_POSITION {
                return;
            }
            custom_box_clone.set_visible(selected == 4);
            config_clone.borrow_mut().frame.color_scheme = match selected {
                0 => SynthwaveColorScheme::Classic,
                1 => SynthwaveColorScheme::Sunset,
                2 => SynthwaveColorScheme::NightDrive,
                3 => SynthwaveColorScheme::Miami,
                _ => SynthwaveColorScheme::Custom {
                    primary: primary_widget_clone.color(),
                    secondary: secondary_widget_clone.color(),
                    accent: accent_widget_clone.color(),
                },
            };
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });

        // Connect custom color widgets
        let make_color_callback = |config: Rc<RefCell<SynthwaveDisplayConfig>>, on_change: Rc<RefCell<Option<Box<dyn Fn()>>>>, preview: DrawingArea, primary: Rc<ColorButtonWidget>, secondary: Rc<ColorButtonWidget>, accent: Rc<ColorButtonWidget>| {
            move |_color: Color| {
                config.borrow_mut().frame.color_scheme = SynthwaveColorScheme::Custom {
                    primary: primary.color(),
                    secondary: secondary.color(),
                    accent: accent.color(),
                };
                Self::queue_redraw(&preview, &on_change);
            }
        };

        custom_primary_widget.set_on_change(make_color_callback(config.clone(), on_change.clone(), preview.clone(), custom_primary_widget.clone(), custom_secondary_widget.clone(), custom_accent_widget.clone()));
        custom_secondary_widget.set_on_change(make_color_callback(config.clone(), on_change.clone(), preview.clone(), custom_primary_widget.clone(), custom_secondary_widget.clone(), custom_accent_widget.clone()));
        custom_accent_widget.set_on_change(make_color_callback(config.clone(), on_change.clone(), preview.clone(), custom_primary_widget.clone(), custom_secondary_widget.clone(), custom_accent_widget.clone()));

        // Neon glow intensity
        let glow_label = Label::new(Some("Effects"));
        glow_label.set_halign(gtk4::Align::Start);
        glow_label.add_css_class("heading");
        glow_label.set_margin_top(12);
        page.append(&glow_label);

        let glow_box = GtkBox::new(Orientation::Horizontal, 6);
        glow_box.append(&Label::new(Some("Neon Glow:")));
        let glow_scale = Scale::with_range(Orientation::Horizontal, 0.0, 1.0, 0.05);
        glow_scale.set_value(config.borrow().frame.neon_glow_intensity);
        glow_scale.set_hexpand(true);
        glow_scale.set_draw_value(true);
        glow_box.append(&glow_scale);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        glow_scale.connect_value_changed(move |scale| {
            config_clone.borrow_mut().frame.neon_glow_intensity = scale.value();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&glow_box);

        // Store widget refs
        *colors_widgets_out.borrow_mut() = Some(ColorsWidgets {
            scheme_dropdown,
            custom_primary_widget,
            custom_secondary_widget,
            custom_accent_widget,
            custom_colors_box,
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

        // Font section
        let font_label = Label::new(Some("Font"));
        font_label.set_halign(gtk4::Align::Start);
        font_label.add_css_class("heading");
        font_label.set_margin_top(12);
        page.append(&font_label);

        // Font button
        let font_box = GtkBox::new(Orientation::Horizontal, 6);
        let header_font_btn = Button::with_label(&config.borrow().frame.header_font);
        header_font_btn.set_hexpand(true);
        font_box.append(&header_font_btn);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let btn_clone = header_font_btn.clone();
        header_font_btn.connect_clicked(move |button| {
            let config_for_cb = config_clone.clone();
            let on_change_for_cb = on_change_clone.clone();
            let preview_for_cb = preview_clone.clone();
            let font_btn_for_cb = btn_clone.clone();
            let current_font = config_for_cb.borrow().frame.header_font.clone();

            if let Some(root) = button.root() {
                if let Some(window) = root.downcast_ref::<gtk4::Window>() {
                    let font_desc = gtk4::pango::FontDescription::from_string(&current_font);

                    shared_font_dialog().choose_font(
                        Some(window),
                        Some(&font_desc),
                        gtk4::gio::Cancellable::NONE,
                        move |result| {
                            if let Ok(font_desc) = result {
                                let family = font_desc.family().map(|s| s.to_string()).unwrap_or_else(|| "sans-serif".to_string());
                                config_for_cb.borrow_mut().frame.header_font = family.clone();
                                font_btn_for_cb.set_label(&family);
                                Self::queue_redraw(&preview_for_cb, &on_change_for_cb);
                            }
                        },
                    );
                }
            }
        });
        page.append(&font_box);

        // Font size
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

        // Store widget refs
        *header_widgets_out.borrow_mut() = Some(HeaderWidgets {
            show_header_check,
            header_text_entry,
            header_style_dropdown,
            header_height_spin,
            header_font_btn,
            header_font_size_spin,
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

    fn create_content_page(
        config: &Rc<RefCell<SynthwaveDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        content_notebook: &Rc<RefCell<Notebook>>,
        source_summaries: &Rc<RefCell<Vec<(String, String, usize, u32)>>>,
        available_fields: &Rc<RefCell<Vec<FieldMetadata>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        Self::set_page_margins(&page);

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
        Self::rebuild_content_tabs(config, on_change, preview, content_notebook, source_summaries, available_fields);

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

    fn rebuild_content_tabs(
        config: &Rc<RefCell<SynthwaveDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        content_notebook: &Rc<RefCell<Notebook>>,
        source_summaries: &Rc<RefCell<Vec<(String, String, usize, u32)>>>,
        available_fields: &Rc<RefCell<Vec<FieldMetadata>>>,
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
                    let tab_box = Self::create_content_item_config(
                        config,
                        on_change,
                        preview,
                        &slot_name,
                        available_fields.borrow().clone(),
                    );
                    items_notebook.append_page(&tab_box, Some(&Label::new(Some(&tab_label))));
                }

                group_box.append(&items_notebook);
                notebook.append_page(&group_box, Some(&Label::new(Some(&format!("Group {}", group_num)))));
            }
        }
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
        let scheme_idx = match &config.frame.color_scheme {
            SynthwaveColorScheme::Classic => 0,
            SynthwaveColorScheme::Sunset => 1,
            SynthwaveColorScheme::NightDrive => 2,
            SynthwaveColorScheme::Miami => 3,
            SynthwaveColorScheme::Custom { .. } => 4,
        };
        let (custom_primary, custom_secondary, custom_accent) = if let SynthwaveColorScheme::Custom { primary, secondary, accent } = &config.frame.color_scheme {
            (Some(*primary), Some(*secondary), Some(*accent))
        } else {
            (None, None, None)
        };
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
        if let Some(ref widgets) = *self.colors_widgets.borrow() {
            widgets.scheme_dropdown.set_selected(scheme_idx);
            widgets.custom_colors_box.set_visible(scheme_idx == 4);
            if let Some(c) = custom_primary {
                widgets.custom_primary_widget.set_color(c);
            }
            if let Some(c) = custom_secondary {
                widgets.custom_secondary_widget.set_color(c);
            }
            if let Some(c) = custom_accent {
                widgets.custom_accent_widget.set_color(c);
            }
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
            widgets.header_font_btn.set_label(&header_font);
            widgets.header_font_size_spin.set_value(header_font_size);
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
        Self::rebuild_content_tabs(
            &self.config,
            &self.on_change,
            &self.preview,
            &self.content_notebook,
            &self.source_summaries,
            &self.available_fields,
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
