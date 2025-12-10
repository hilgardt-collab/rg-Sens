//! Graph displayer configuration widget

use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, CheckButton, ColorButton, DropDown, Label, Notebook, Orientation,
    SpinButton, StringList,
};
use std::cell::RefCell;
use std::rc::Rc;

use super::graph_display::{
    AxisConfig, FillMode, GraphDisplayConfig, GraphType, LineStyle, Margin,
};
use super::background::Color;
use super::text_line_config_widget::TextLineConfigWidget;

pub struct GraphConfigWidget {
    widget: GtkBox,
    config: Rc<RefCell<GraphDisplayConfig>>,

    // Graph type controls
    graph_type_combo: DropDown,
    line_style_combo: DropDown,
    line_width_spin: SpinButton,
    line_color_button: ColorButton,

    // Fill controls
    fill_mode_combo: DropDown,
    fill_color_button: ColorButton,
    fill_gradient_start_button: ColorButton,
    fill_gradient_end_button: ColorButton,
    fill_opacity_spin: SpinButton,

    // Data points
    max_points_spin: SpinButton,
    show_points_check: CheckButton,
    point_radius_spin: SpinButton,
    point_color_button: ColorButton,

    // Scaling
    auto_scale_check: CheckButton,
    min_value_spin: SpinButton,
    max_value_spin: SpinButton,
    value_padding_spin: SpinButton,

    // Axes
    y_axis_show_check: CheckButton,
    y_axis_show_labels_check: CheckButton,
    y_axis_show_grid_check: CheckButton,
    y_axis_color_button: ColorButton,
    y_axis_grid_color_button: ColorButton,

    x_axis_show_check: CheckButton,
    x_axis_show_grid_check: CheckButton,
    x_axis_color_button: ColorButton,
    x_axis_grid_color_button: ColorButton,

    // Margins
    margin_top_spin: SpinButton,
    margin_right_spin: SpinButton,
    margin_bottom_spin: SpinButton,
    margin_left_spin: SpinButton,

    // Backgrounds
    background_color_button: ColorButton,
    plot_background_color_button: ColorButton,

    // Animation and smoothing
    smooth_lines_check: CheckButton,
    animate_new_points_check: CheckButton,

    // Text overlay
    text_config_widgets: Vec<Rc<TextLineConfigWidget>>,
}

impl GraphConfigWidget {
    pub fn new(available_fields: Vec<crate::core::FieldMetadata>) -> Self {
        let widget = GtkBox::new(Orientation::Vertical, 0);
        let config = Rc::new(RefCell::new(GraphDisplayConfig::default()));

        let notebook = Notebook::new();
        notebook.set_scrollable(true);

        // === Tab 1: Graph Style ===
        let style_page = create_style_page(config.clone());
        notebook.append_page(&style_page.widget, Some(&Label::new(Some("Style"))));

        // === Tab 2: Data & Scaling ===
        let data_page = create_data_page(config.clone());
        notebook.append_page(&data_page.widget, Some(&Label::new(Some("Data"))));

        // === Tab 3: Axes ===
        let axes_page = create_axes_page(config.clone());
        notebook.append_page(&axes_page.widget, Some(&Label::new(Some("Axes"))));

        // === Tab 4: Layout ===
        let layout_page = create_layout_page(config.clone());
        notebook.append_page(&layout_page.widget, Some(&Label::new(Some("Layout"))));

        // === Tab 5: Text Overlay ===
        let text_page = create_text_overlay_page(config.clone(), available_fields);
        notebook.append_page(&text_page.widget, Some(&Label::new(Some("Text"))));

        widget.append(&notebook);

        Self {
            widget,
            config,
            graph_type_combo: style_page.graph_type_combo,
            line_style_combo: style_page.line_style_combo,
            line_width_spin: style_page.line_width_spin,
            line_color_button: style_page.line_color_button,
            fill_mode_combo: style_page.fill_mode_combo,
            fill_color_button: style_page.fill_color_button,
            fill_gradient_start_button: style_page.fill_gradient_start_button,
            fill_gradient_end_button: style_page.fill_gradient_end_button,
            fill_opacity_spin: style_page.fill_opacity_spin,
            max_points_spin: data_page.max_points_spin,
            show_points_check: data_page.show_points_check,
            point_radius_spin: data_page.point_radius_spin,
            point_color_button: data_page.point_color_button,
            auto_scale_check: data_page.auto_scale_check,
            min_value_spin: data_page.min_value_spin,
            max_value_spin: data_page.max_value_spin,
            value_padding_spin: data_page.value_padding_spin,
            y_axis_show_check: axes_page.y_axis_show_check,
            y_axis_show_labels_check: axes_page.y_axis_show_labels_check,
            y_axis_show_grid_check: axes_page.y_axis_show_grid_check,
            y_axis_color_button: axes_page.y_axis_color_button,
            y_axis_grid_color_button: axes_page.y_axis_grid_color_button,
            x_axis_show_check: axes_page.x_axis_show_check,
            x_axis_show_grid_check: axes_page.x_axis_show_grid_check,
            x_axis_color_button: axes_page.x_axis_color_button,
            x_axis_grid_color_button: axes_page.x_axis_grid_color_button,
            margin_top_spin: layout_page.margin_top_spin,
            margin_right_spin: layout_page.margin_right_spin,
            margin_bottom_spin: layout_page.margin_bottom_spin,
            margin_left_spin: layout_page.margin_left_spin,
            background_color_button: layout_page.background_color_button,
            plot_background_color_button: layout_page.plot_background_color_button,
            smooth_lines_check: style_page.smooth_lines_check,
            animate_new_points_check: style_page.animate_new_points_check,
            text_config_widgets: text_page.text_config_widgets,
        }
    }

    pub fn widget(&self) -> &GtkBox {
        &self.widget
    }

    pub fn get_config(&self) -> GraphDisplayConfig {
        let mut config = self.config.borrow().clone();

        // Update text overlay from widgets
        config.text_overlay = self.text_config_widgets
            .iter()
            .flat_map(|w| w.get_config().lines)
            .collect();

        config
    }

    pub fn set_config(&self, config: GraphDisplayConfig) {
        // Update UI from config
        self.graph_type_combo.set_selected(match config.graph_type {
            GraphType::Line => 0,
            GraphType::Bar => 1,
            GraphType::Area => 2,
            GraphType::SteppedLine => 3,
        });

        self.line_style_combo.set_selected(match config.line_style {
            LineStyle::Solid => 0,
            LineStyle::Dashed => 1,
            LineStyle::Dotted => 2,
        });

        self.line_width_spin.set_value(config.line_width);
        self.line_color_button.set_rgba(&color_to_rgba(&config.line_color));

        self.fill_mode_combo.set_selected(match config.fill_mode {
            FillMode::None => 0,
            FillMode::Solid => 1,
            FillMode::Gradient => 2,
        });

        self.fill_color_button.set_rgba(&color_to_rgba(&config.fill_color));
        self.fill_gradient_start_button.set_rgba(&color_to_rgba(&config.fill_gradient_start));
        self.fill_gradient_end_button.set_rgba(&color_to_rgba(&config.fill_gradient_end));
        self.fill_opacity_spin.set_value(config.fill_opacity);

        self.max_points_spin.set_value(config.max_data_points as f64);
        self.show_points_check.set_active(config.show_points);
        self.point_radius_spin.set_value(config.point_radius);
        self.point_color_button.set_rgba(&color_to_rgba(&config.point_color));

        self.auto_scale_check.set_active(config.auto_scale);
        self.min_value_spin.set_value(config.min_value);
        self.max_value_spin.set_value(config.max_value);
        self.value_padding_spin.set_value(config.value_padding);

        self.y_axis_show_check.set_active(config.y_axis.show);
        self.y_axis_show_labels_check.set_active(config.y_axis.show_labels);
        self.y_axis_show_grid_check.set_active(config.y_axis.show_grid);
        self.y_axis_color_button.set_rgba(&color_to_rgba(&config.y_axis.color));
        self.y_axis_grid_color_button.set_rgba(&color_to_rgba(&config.y_axis.grid_color));

        self.x_axis_show_check.set_active(config.x_axis.show);
        self.x_axis_show_grid_check.set_active(config.x_axis.show_grid);
        self.x_axis_color_button.set_rgba(&color_to_rgba(&config.x_axis.color));
        self.x_axis_grid_color_button.set_rgba(&color_to_rgba(&config.x_axis.grid_color));

        self.margin_top_spin.set_value(config.margin.top);
        self.margin_right_spin.set_value(config.margin.right);
        self.margin_bottom_spin.set_value(config.margin.bottom);
        self.margin_left_spin.set_value(config.margin.left);

        self.background_color_button.set_rgba(&color_to_rgba(&config.background_color));
        self.plot_background_color_button.set_rgba(&color_to_rgba(&config.plot_background_color));

        self.smooth_lines_check.set_active(config.smooth_lines);
        self.animate_new_points_check.set_active(config.animate_new_points);

        // Set text overlay configs
        for (i, text_line) in config.text_overlay.iter().enumerate() {
            if i < self.text_config_widgets.len() {
                let text_displayer_config = crate::displayers::TextDisplayerConfig {
                    lines: vec![text_line.clone()],
                };
                self.text_config_widgets[i].set_config(text_displayer_config);
            }
        }

        *self.config.borrow_mut() = config;
    }
}

impl Default for GraphConfigWidget {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

// Helper structures for page creation
struct StylePageWidgets {
    widget: GtkBox,
    graph_type_combo: DropDown,
    line_style_combo: DropDown,
    line_width_spin: SpinButton,
    line_color_button: ColorButton,
    fill_mode_combo: DropDown,
    fill_color_button: ColorButton,
    fill_gradient_start_button: ColorButton,
    fill_gradient_end_button: ColorButton,
    fill_opacity_spin: SpinButton,
    smooth_lines_check: CheckButton,
    animate_new_points_check: CheckButton,
}

struct DataPageWidgets {
    widget: GtkBox,
    max_points_spin: SpinButton,
    show_points_check: CheckButton,
    point_radius_spin: SpinButton,
    point_color_button: ColorButton,
    auto_scale_check: CheckButton,
    min_value_spin: SpinButton,
    max_value_spin: SpinButton,
    value_padding_spin: SpinButton,
}

struct AxesPageWidgets {
    widget: GtkBox,
    y_axis_show_check: CheckButton,
    y_axis_show_labels_check: CheckButton,
    y_axis_show_grid_check: CheckButton,
    y_axis_color_button: ColorButton,
    y_axis_grid_color_button: ColorButton,
    x_axis_show_check: CheckButton,
    x_axis_show_grid_check: CheckButton,
    x_axis_color_button: ColorButton,
    x_axis_grid_color_button: ColorButton,
}

struct LayoutPageWidgets {
    widget: GtkBox,
    margin_top_spin: SpinButton,
    margin_right_spin: SpinButton,
    margin_bottom_spin: SpinButton,
    margin_left_spin: SpinButton,
    background_color_button: ColorButton,
    plot_background_color_button: ColorButton,
}

struct TextOverlayPageWidgets {
    widget: GtkBox,
    text_config_widgets: Vec<Rc<TextLineConfigWidget>>,
}

fn create_style_page(config: Rc<RefCell<GraphDisplayConfig>>) -> StylePageWidgets {
    let page = GtkBox::new(Orientation::Vertical, 12);
    page.set_margin_start(12);
    page.set_margin_end(12);
    page.set_margin_top(12);
    page.set_margin_bottom(12);

    // Graph type
    let type_box = GtkBox::new(Orientation::Horizontal, 6);
    type_box.append(&Label::new(Some("Graph Type:")));
    let graph_type_combo = DropDown::new(
        Some(StringList::new(&["Line", "Bar", "Area", "Stepped Line"])),
        Option::<gtk4::Expression>::None,
    );
    type_box.append(&graph_type_combo);
    page.append(&type_box);

    let config_clone = config.clone();
    graph_type_combo.connect_selected_notify(move |combo| {
        let graph_type = match combo.selected() {
            0 => GraphType::Line,
            1 => GraphType::Bar,
            2 => GraphType::Area,
            3 => GraphType::SteppedLine,
            _ => GraphType::Line,
        };
        config_clone.borrow_mut().graph_type = graph_type;
    });

    // Line style
    let line_style_box = GtkBox::new(Orientation::Horizontal, 6);
    line_style_box.append(&Label::new(Some("Line Style:")));
    let line_style_combo = DropDown::new(
        Some(StringList::new(&["Solid", "Dashed", "Dotted"])),
        Option::<gtk4::Expression>::None,
    );
    line_style_box.append(&line_style_combo);
    page.append(&line_style_box);

    let config_clone = config.clone();
    line_style_combo.connect_selected_notify(move |combo| {
        let line_style = match combo.selected() {
            0 => LineStyle::Solid,
            1 => LineStyle::Dashed,
            2 => LineStyle::Dotted,
            _ => LineStyle::Solid,
        };
        config_clone.borrow_mut().line_style = line_style;
    });

    // Line width
    let width_box = GtkBox::new(Orientation::Horizontal, 6);
    width_box.append(&Label::new(Some("Line Width:")));
    let line_width_spin = SpinButton::with_range(0.5, 10.0, 0.5);
    line_width_spin.set_value(2.0);
    line_width_spin.set_hexpand(true);
    width_box.append(&line_width_spin);
    page.append(&width_box);

    let config_clone = config.clone();
    line_width_spin.connect_value_changed(move |spin| {
        config_clone.borrow_mut().line_width = spin.value();
    });

    // Line color
    let line_color_box = GtkBox::new(Orientation::Horizontal, 6);
    line_color_box.append(&Label::new(Some("Line Color:")));
    let line_color_button = ColorButton::new();
    line_color_button.set_rgba(&gtk4::gdk::RGBA::new(0.2, 0.8, 0.4, 1.0));
    line_color_box.append(&line_color_button);
    page.append(&line_color_box);

    let config_clone = config.clone();
    line_color_button.connect_rgba_notify(move |btn| {
        let rgba = btn.rgba();
        config_clone.borrow_mut().line_color = rgba_to_color(&rgba);
    });

    // Fill mode
    let fill_mode_box = GtkBox::new(Orientation::Horizontal, 6);
    fill_mode_box.append(&Label::new(Some("Fill Mode:")));
    let fill_mode_combo = DropDown::new(
        Some(StringList::new(&["None", "Solid", "Gradient"])),
        Option::<gtk4::Expression>::None,
    );
    fill_mode_combo.set_selected(2);
    fill_mode_box.append(&fill_mode_combo);
    page.append(&fill_mode_box);

    let config_clone = config.clone();
    fill_mode_combo.connect_selected_notify(move |combo| {
        let fill_mode = match combo.selected() {
            0 => FillMode::None,
            1 => FillMode::Solid,
            2 => FillMode::Gradient,
            _ => FillMode::None,
        };
        config_clone.borrow_mut().fill_mode = fill_mode;
    });

    // Fill color
    let fill_color_box = GtkBox::new(Orientation::Horizontal, 6);
    fill_color_box.append(&Label::new(Some("Fill Color:")));
    let fill_color_button = ColorButton::new();
    fill_color_button.set_rgba(&gtk4::gdk::RGBA::new(0.2, 0.8, 0.4, 0.3));
    fill_color_box.append(&fill_color_button);
    page.append(&fill_color_box);

    let config_clone = config.clone();
    fill_color_button.connect_rgba_notify(move |btn| {
        let rgba = btn.rgba();
        config_clone.borrow_mut().fill_color = rgba_to_color(&rgba);
    });

    // Gradient colors
    let gradient_box = GtkBox::new(Orientation::Horizontal, 6);
    gradient_box.append(&Label::new(Some("Gradient Start:")));
    let fill_gradient_start_button = ColorButton::new();
    fill_gradient_start_button.set_rgba(&gtk4::gdk::RGBA::new(0.2, 0.8, 0.4, 0.6));
    gradient_box.append(&fill_gradient_start_button);

    gradient_box.append(&Label::new(Some("End:")));
    let fill_gradient_end_button = ColorButton::new();
    fill_gradient_end_button.set_rgba(&gtk4::gdk::RGBA::new(0.2, 0.8, 0.4, 0.0));
    gradient_box.append(&fill_gradient_end_button);
    page.append(&gradient_box);

    let config_clone = config.clone();
    fill_gradient_start_button.connect_rgba_notify(move |btn| {
        let rgba = btn.rgba();
        config_clone.borrow_mut().fill_gradient_start = rgba_to_color(&rgba);
    });

    let config_clone = config.clone();
    fill_gradient_end_button.connect_rgba_notify(move |btn| {
        let rgba = btn.rgba();
        config_clone.borrow_mut().fill_gradient_end = rgba_to_color(&rgba);
    });

    // Fill opacity
    let opacity_box = GtkBox::new(Orientation::Horizontal, 6);
    opacity_box.append(&Label::new(Some("Fill Opacity:")));
    let fill_opacity_spin = SpinButton::with_range(0.0, 1.0, 0.05);
    fill_opacity_spin.set_value(0.3);
    fill_opacity_spin.set_hexpand(true);
    opacity_box.append(&fill_opacity_spin);
    page.append(&opacity_box);

    let config_clone = config.clone();
    fill_opacity_spin.connect_value_changed(move |spin| {
        config_clone.borrow_mut().fill_opacity = spin.value();
    });

    // Smooth lines checkbox
    let smooth_lines_check = CheckButton::with_label("Smooth Lines (Bezier Curves)");
    smooth_lines_check.set_active(true);
    page.append(&smooth_lines_check);

    let config_clone = config.clone();
    smooth_lines_check.connect_toggled(move |check| {
        config_clone.borrow_mut().smooth_lines = check.is_active();
    });

    // Animate new points checkbox
    let animate_new_points_check = CheckButton::with_label("Animate Graph Values");
    animate_new_points_check.set_active(false);
    page.append(&animate_new_points_check);

    let config_clone = config.clone();
    animate_new_points_check.connect_toggled(move |check| {
        config_clone.borrow_mut().animate_new_points = check.is_active();
    });

    StylePageWidgets {
        widget: page,
        graph_type_combo,
        line_style_combo,
        line_width_spin,
        line_color_button,
        fill_mode_combo,
        fill_color_button,
        fill_gradient_start_button,
        fill_gradient_end_button,
        fill_opacity_spin,
        smooth_lines_check,
        animate_new_points_check,
    }
}

fn create_data_page(config: Rc<RefCell<GraphDisplayConfig>>) -> DataPageWidgets {
    let page = GtkBox::new(Orientation::Vertical, 12);
    page.set_margin_start(12);
    page.set_margin_end(12);
    page.set_margin_top(12);
    page.set_margin_bottom(12);

    // Max data points
    let points_box = GtkBox::new(Orientation::Horizontal, 6);
    points_box.append(&Label::new(Some("Max Data Points:")));
    let max_points_spin = SpinButton::with_range(10.0, 300.0, 5.0);
    max_points_spin.set_value(60.0);
    max_points_spin.set_hexpand(true);
    points_box.append(&max_points_spin);
    page.append(&points_box);

    let config_clone = config.clone();
    max_points_spin.connect_value_changed(move |spin| {
        config_clone.borrow_mut().max_data_points = spin.value() as usize;
    });

    // Show points
    let show_points_check = CheckButton::with_label("Show Data Points");
    page.append(&show_points_check);

    let config_clone = config.clone();
    show_points_check.connect_toggled(move |check| {
        config_clone.borrow_mut().show_points = check.is_active();
    });

    // Point radius
    let radius_box = GtkBox::new(Orientation::Horizontal, 6);
    radius_box.append(&Label::new(Some("Point Radius:")));
    let point_radius_spin = SpinButton::with_range(1.0, 10.0, 0.5);
    point_radius_spin.set_value(3.0);
    point_radius_spin.set_hexpand(true);
    radius_box.append(&point_radius_spin);
    page.append(&radius_box);

    let config_clone = config.clone();
    point_radius_spin.connect_value_changed(move |spin| {
        config_clone.borrow_mut().point_radius = spin.value();
    });

    // Point color
    let point_color_box = GtkBox::new(Orientation::Horizontal, 6);
    point_color_box.append(&Label::new(Some("Point Color:")));
    let point_color_button = ColorButton::new();
    point_color_button.set_rgba(&gtk4::gdk::RGBA::new(0.2, 0.8, 0.4, 1.0));
    point_color_box.append(&point_color_button);
    page.append(&point_color_box);

    let config_clone = config.clone();
    point_color_button.connect_rgba_notify(move |btn| {
        let rgba = btn.rgba();
        config_clone.borrow_mut().point_color = rgba_to_color(&rgba);
    });

    // Auto scale
    let auto_scale_check = CheckButton::with_label("Auto Scale Y-Axis");
    auto_scale_check.set_active(true);
    page.append(&auto_scale_check);

    let config_clone = config.clone();
    let min_value_spin = SpinButton::with_range(-1000.0, 10000.0, 1.0);
    let max_value_spin = SpinButton::with_range(-1000.0, 10000.0, 1.0);

    let min_spin_clone = min_value_spin.clone();
    let max_spin_clone = max_value_spin.clone();
    auto_scale_check.connect_toggled(move |check| {
        let active = check.is_active();
        config_clone.borrow_mut().auto_scale = active;
        min_spin_clone.set_sensitive(!active);
        max_spin_clone.set_sensitive(!active);
    });

    // Min/Max values
    min_value_spin.set_value(0.0);
    min_value_spin.set_sensitive(false);
    let min_box = GtkBox::new(Orientation::Horizontal, 6);
    min_box.append(&Label::new(Some("Min Value:")));
    min_box.append(&min_value_spin);
    page.append(&min_box);

    let config_clone = config.clone();
    min_value_spin.connect_value_changed(move |spin| {
        config_clone.borrow_mut().min_value = spin.value();
    });

    max_value_spin.set_value(100.0);
    max_value_spin.set_sensitive(false);
    let max_box = GtkBox::new(Orientation::Horizontal, 6);
    max_box.append(&Label::new(Some("Max Value:")));
    max_box.append(&max_value_spin);
    page.append(&max_box);

    let config_clone = config.clone();
    max_value_spin.connect_value_changed(move |spin| {
        config_clone.borrow_mut().max_value = spin.value();
    });

    // Value padding
    let padding_box = GtkBox::new(Orientation::Horizontal, 6);
    padding_box.append(&Label::new(Some("Auto Scale Padding %:")));
    let value_padding_spin = SpinButton::with_range(0.0, 50.0, 1.0);
    value_padding_spin.set_value(10.0);
    value_padding_spin.set_hexpand(true);
    padding_box.append(&value_padding_spin);
    page.append(&padding_box);

    let config_clone = config.clone();
    value_padding_spin.connect_value_changed(move |spin| {
        config_clone.borrow_mut().value_padding = spin.value();
    });

    DataPageWidgets {
        widget: page,
        max_points_spin,
        show_points_check,
        point_radius_spin,
        point_color_button,
        auto_scale_check,
        min_value_spin,
        max_value_spin,
        value_padding_spin,
    }
}

fn create_axes_page(config: Rc<RefCell<GraphDisplayConfig>>) -> AxesPageWidgets {
    let page = GtkBox::new(Orientation::Vertical, 12);
    page.set_margin_start(12);
    page.set_margin_end(12);
    page.set_margin_top(12);
    page.set_margin_bottom(12);

    // Y-Axis section
    page.append(&Label::new(Some("Y-Axis (Vertical)")));

    let y_axis_show_check = CheckButton::with_label("Show Y-Axis");
    y_axis_show_check.set_active(true);
    page.append(&y_axis_show_check);

    let config_clone = config.clone();
    y_axis_show_check.connect_toggled(move |check| {
        config_clone.borrow_mut().y_axis.show = check.is_active();
    });

    let y_axis_show_labels_check = CheckButton::with_label("Show Y-Axis Labels");
    y_axis_show_labels_check.set_active(true);
    page.append(&y_axis_show_labels_check);

    let config_clone = config.clone();
    y_axis_show_labels_check.connect_toggled(move |check| {
        config_clone.borrow_mut().y_axis.show_labels = check.is_active();
    });

    let y_axis_show_grid_check = CheckButton::with_label("Show Y-Axis Grid");
    y_axis_show_grid_check.set_active(true);
    page.append(&y_axis_show_grid_check);

    let config_clone = config.clone();
    y_axis_show_grid_check.connect_toggled(move |check| {
        config_clone.borrow_mut().y_axis.show_grid = check.is_active();
    });

    let y_color_box = GtkBox::new(Orientation::Horizontal, 6);
    y_color_box.append(&Label::new(Some("Y-Axis Color:")));
    let y_axis_color_button = ColorButton::new();
    y_axis_color_button.set_rgba(&gtk4::gdk::RGBA::new(0.7, 0.7, 0.7, 1.0));
    y_color_box.append(&y_axis_color_button);
    page.append(&y_color_box);

    let config_clone = config.clone();
    y_axis_color_button.connect_rgba_notify(move |btn| {
        let rgba = btn.rgba();
        config_clone.borrow_mut().y_axis.color = rgba_to_color(&rgba);
    });

    let y_grid_color_box = GtkBox::new(Orientation::Horizontal, 6);
    y_grid_color_box.append(&Label::new(Some("Y-Grid Color:")));
    let y_axis_grid_color_button = ColorButton::new();
    y_axis_grid_color_button.set_rgba(&gtk4::gdk::RGBA::new(0.3, 0.3, 0.3, 0.5));
    y_grid_color_box.append(&y_axis_grid_color_button);
    page.append(&y_grid_color_box);

    let config_clone = config.clone();
    y_axis_grid_color_button.connect_rgba_notify(move |btn| {
        let rgba = btn.rgba();
        config_clone.borrow_mut().y_axis.grid_color = rgba_to_color(&rgba);
    });

    // X-Axis section
    page.append(&Label::new(Some("\nX-Axis (Horizontal)")));

    let x_axis_show_check = CheckButton::with_label("Show X-Axis");
    x_axis_show_check.set_active(true);
    page.append(&x_axis_show_check);

    let config_clone = config.clone();
    x_axis_show_check.connect_toggled(move |check| {
        config_clone.borrow_mut().x_axis.show = check.is_active();
    });

    let x_axis_show_grid_check = CheckButton::with_label("Show X-Axis Grid");
    x_axis_show_grid_check.set_active(true);
    page.append(&x_axis_show_grid_check);

    let config_clone = config.clone();
    x_axis_show_grid_check.connect_toggled(move |check| {
        config_clone.borrow_mut().x_axis.show_grid = check.is_active();
    });

    let x_color_box = GtkBox::new(Orientation::Horizontal, 6);
    x_color_box.append(&Label::new(Some("X-Axis Color:")));
    let x_axis_color_button = ColorButton::new();
    x_axis_color_button.set_rgba(&gtk4::gdk::RGBA::new(0.7, 0.7, 0.7, 1.0));
    x_color_box.append(&x_axis_color_button);
    page.append(&x_color_box);

    let config_clone = config.clone();
    x_axis_color_button.connect_rgba_notify(move |btn| {
        let rgba = btn.rgba();
        config_clone.borrow_mut().x_axis.color = rgba_to_color(&rgba);
    });

    let x_grid_color_box = GtkBox::new(Orientation::Horizontal, 6);
    x_grid_color_box.append(&Label::new(Some("X-Grid Color:")));
    let x_axis_grid_color_button = ColorButton::new();
    x_axis_grid_color_button.set_rgba(&gtk4::gdk::RGBA::new(0.3, 0.3, 0.3, 0.5));
    x_grid_color_box.append(&x_axis_grid_color_button);
    page.append(&x_grid_color_box);

    let config_clone = config.clone();
    x_axis_grid_color_button.connect_rgba_notify(move |btn| {
        let rgba = btn.rgba();
        config_clone.borrow_mut().x_axis.grid_color = rgba_to_color(&rgba);
    });

    AxesPageWidgets {
        widget: page,
        y_axis_show_check,
        y_axis_show_labels_check,
        y_axis_show_grid_check,
        y_axis_color_button,
        y_axis_grid_color_button,
        x_axis_show_check,
        x_axis_show_grid_check,
        x_axis_color_button,
        x_axis_grid_color_button,
    }
}

fn create_layout_page(config: Rc<RefCell<GraphDisplayConfig>>) -> LayoutPageWidgets {
    let page = GtkBox::new(Orientation::Vertical, 12);
    page.set_margin_start(12);
    page.set_margin_end(12);
    page.set_margin_top(12);
    page.set_margin_bottom(12);

    // Margins
    page.append(&Label::new(Some("Margins:")));

    let margin_top_spin = SpinButton::with_range(0.0, 100.0, 1.0);
    margin_top_spin.set_value(10.0);
    let top_box = GtkBox::new(Orientation::Horizontal, 6);
    top_box.append(&Label::new(Some("Top:")));
    top_box.append(&margin_top_spin);
    page.append(&top_box);

    let config_clone = config.clone();
    margin_top_spin.connect_value_changed(move |spin| {
        config_clone.borrow_mut().margin.top = spin.value();
    });

    let margin_right_spin = SpinButton::with_range(0.0, 100.0, 1.0);
    margin_right_spin.set_value(10.0);
    let right_box = GtkBox::new(Orientation::Horizontal, 6);
    right_box.append(&Label::new(Some("Right:")));
    right_box.append(&margin_right_spin);
    page.append(&right_box);

    let config_clone = config.clone();
    margin_right_spin.connect_value_changed(move |spin| {
        config_clone.borrow_mut().margin.right = spin.value();
    });

    let margin_bottom_spin = SpinButton::with_range(0.0, 100.0, 1.0);
    margin_bottom_spin.set_value(30.0);
    let bottom_box = GtkBox::new(Orientation::Horizontal, 6);
    bottom_box.append(&Label::new(Some("Bottom:")));
    bottom_box.append(&margin_bottom_spin);
    page.append(&bottom_box);

    let config_clone = config.clone();
    margin_bottom_spin.connect_value_changed(move |spin| {
        config_clone.borrow_mut().margin.bottom = spin.value();
    });

    let margin_left_spin = SpinButton::with_range(0.0, 100.0, 1.0);
    margin_left_spin.set_value(50.0);
    let left_box = GtkBox::new(Orientation::Horizontal, 6);
    left_box.append(&Label::new(Some("Left:")));
    left_box.append(&margin_left_spin);
    page.append(&left_box);

    let config_clone = config.clone();
    margin_left_spin.connect_value_changed(move |spin| {
        config_clone.borrow_mut().margin.left = spin.value();
    });

    // Background colors
    page.append(&Label::new(Some("\nColors:")));

    let bg_color_box = GtkBox::new(Orientation::Horizontal, 6);
    bg_color_box.append(&Label::new(Some("Background:")));
    let background_color_button = ColorButton::new();
    background_color_button.set_rgba(&gtk4::gdk::RGBA::new(0.0, 0.0, 0.0, 0.0));
    background_color_button.set_use_alpha(true);
    bg_color_box.append(&background_color_button);
    page.append(&bg_color_box);

    let config_clone = config.clone();
    background_color_button.connect_rgba_notify(move |btn| {
        let rgba = btn.rgba();
        config_clone.borrow_mut().background_color = rgba_to_color(&rgba);
    });

    let plot_bg_color_box = GtkBox::new(Orientation::Horizontal, 6);
    plot_bg_color_box.append(&Label::new(Some("Plot Background:")));
    let plot_background_color_button = ColorButton::new();
    plot_background_color_button.set_rgba(&gtk4::gdk::RGBA::new(0.1, 0.1, 0.1, 0.5));
    plot_background_color_button.set_use_alpha(true);
    plot_bg_color_box.append(&plot_background_color_button);
    page.append(&plot_bg_color_box);

    let config_clone = config.clone();
    plot_background_color_button.connect_rgba_notify(move |btn| {
        let rgba = btn.rgba();
        config_clone.borrow_mut().plot_background_color = rgba_to_color(&rgba);
    });

    LayoutPageWidgets {
        widget: page,
        margin_top_spin,
        margin_right_spin,
        margin_bottom_spin,
        margin_left_spin,
        background_color_button,
        plot_background_color_button,
    }
}

fn create_text_overlay_page(config: Rc<RefCell<GraphDisplayConfig>>, available_fields: Vec<crate::core::FieldMetadata>) -> TextOverlayPageWidgets {
    let page = GtkBox::new(Orientation::Vertical, 12);
    page.set_margin_start(12);
    page.set_margin_end(12);
    page.set_margin_top(12);
    page.set_margin_bottom(12);

    let label = Label::new(Some("Text Overlay Lines:"));
    label.set_halign(gtk4::Align::Start);
    page.append(&label);

    // Create 3 text line config widgets
    let mut text_config_widgets = Vec::new();
    for _i in 0..3 {
        let text_widget = Rc::new(TextLineConfigWidget::new(available_fields.clone()));
        page.append(text_widget.widget());

        // Note: We'll collect configs when get_config() is called
        text_config_widgets.push(text_widget);
    }

    TextOverlayPageWidgets {
        widget: page,
        text_config_widgets,
    }
}

// Helper functions
fn color_to_rgba(color: &Color) -> gtk4::gdk::RGBA {
    gtk4::gdk::RGBA::new(color.r as f32, color.g as f32, color.b as f32, color.a as f32)
}

fn rgba_to_color(rgba: &gtk4::gdk::RGBA) -> Color {
    Color {
        r: rgba.red() as f64,
        g: rgba.green() as f64,
        b: rgba.blue() as f64,
        a: rgba.alpha() as f64,
    }
}
