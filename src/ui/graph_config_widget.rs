//! Graph displayer configuration widget

use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, CheckButton, DropDown, Label, Notebook, Orientation,
    SpinButton, StringList,
};
use std::cell::RefCell;
use std::rc::Rc;

use super::color_picker::ColorPickerDialog;
use super::graph_display::{FillMode, GraphDisplayConfig, GraphType, LineStyle};
use super::background::Color;
use super::text_line_config_widget::TextLineConfigWidget;

pub struct GraphConfigWidget {
    widget: GtkBox,
    config: Rc<RefCell<GraphDisplayConfig>>,

    // Graph type controls
    graph_type_combo: DropDown,
    line_style_combo: DropDown,
    line_width_spin: SpinButton,
    line_color_button: Button,

    // Fill controls
    fill_mode_combo: DropDown,
    fill_color_button: Button,
    fill_gradient_start_button: Button,
    fill_gradient_end_button: Button,
    fill_opacity_spin: SpinButton,

    // Data points
    max_points_spin: SpinButton,
    show_points_check: CheckButton,
    point_radius_spin: SpinButton,
    point_color_button: Button,

    // Scaling
    auto_scale_check: CheckButton,
    min_value_spin: SpinButton,
    max_value_spin: SpinButton,
    value_padding_spin: SpinButton,

    // Axes
    y_axis_show_check: CheckButton,
    y_axis_show_labels_check: CheckButton,
    y_axis_show_grid_check: CheckButton,
    y_axis_color_button: Button,
    y_axis_grid_color_button: Button,
    y_axis_label_color_button: Button,
    y_axis_label_font_button: Button,
    y_axis_label_font_size_spin: SpinButton,
    y_axis_label_bold_check: CheckButton,
    y_axis_label_italic_check: CheckButton,

    x_axis_show_check: CheckButton,
    x_axis_show_grid_check: CheckButton,
    x_axis_color_button: Button,
    x_axis_grid_color_button: Button,
    x_axis_label_color_button: Button,
    x_axis_label_font_button: Button,
    x_axis_label_font_size_spin: SpinButton,
    x_axis_label_bold_check: CheckButton,
    x_axis_label_italic_check: CheckButton,

    // Margins
    margin_top_spin: SpinButton,
    margin_right_spin: SpinButton,
    margin_bottom_spin: SpinButton,
    margin_left_spin: SpinButton,

    // Backgrounds
    background_color_button: Button,
    plot_background_color_button: Button,

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
            y_axis_label_color_button: axes_page.y_axis_label_color_button,
            y_axis_label_font_button: axes_page.y_axis_label_font_button,
            y_axis_label_font_size_spin: axes_page.y_axis_label_font_size_spin,
            y_axis_label_bold_check: axes_page.y_axis_label_bold_check,
            y_axis_label_italic_check: axes_page.y_axis_label_italic_check,
            x_axis_show_check: axes_page.x_axis_show_check,
            x_axis_show_grid_check: axes_page.x_axis_show_grid_check,
            x_axis_color_button: axes_page.x_axis_color_button,
            x_axis_grid_color_button: axes_page.x_axis_grid_color_button,
            x_axis_label_color_button: axes_page.x_axis_label_color_button,
            x_axis_label_font_button: axes_page.x_axis_label_font_button,
            x_axis_label_font_size_spin: axes_page.x_axis_label_font_size_spin,
            x_axis_label_bold_check: axes_page.x_axis_label_bold_check,
            x_axis_label_italic_check: axes_page.x_axis_label_italic_check,
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
        update_color_button(&self.line_color_button, &config.line_color);

        self.fill_mode_combo.set_selected(match config.fill_mode {
            FillMode::None => 0,
            FillMode::Solid => 1,
            FillMode::Gradient => 2,
        });

        update_color_button(&self.fill_color_button, &config.fill_color);
        update_color_button(&self.fill_gradient_start_button, &config.fill_gradient_start);
        update_color_button(&self.fill_gradient_end_button, &config.fill_gradient_end);
        self.fill_opacity_spin.set_value(config.fill_opacity);

        self.max_points_spin.set_value(config.max_data_points as f64);
        self.show_points_check.set_active(config.show_points);
        self.point_radius_spin.set_value(config.point_radius);
        update_color_button(&self.point_color_button, &config.point_color);

        self.auto_scale_check.set_active(config.auto_scale);
        self.min_value_spin.set_value(config.min_value);
        self.max_value_spin.set_value(config.max_value);
        self.value_padding_spin.set_value(config.value_padding);

        self.y_axis_show_check.set_active(config.y_axis.show);
        self.y_axis_show_labels_check.set_active(config.y_axis.show_labels);
        self.y_axis_show_grid_check.set_active(config.y_axis.show_grid);
        update_color_button(&self.y_axis_color_button, &config.y_axis.color);
        update_color_button(&self.y_axis_grid_color_button, &config.y_axis.grid_color);
        update_color_button(&self.y_axis_label_color_button, &config.y_axis.label_color);
        self.y_axis_label_font_button.set_label(&format!("{} {:.0}", config.y_axis.label_font_family, config.y_axis.label_font_size));
        self.y_axis_label_font_size_spin.set_value(config.y_axis.label_font_size);
        self.y_axis_label_bold_check.set_active(config.y_axis.label_bold);
        self.y_axis_label_italic_check.set_active(config.y_axis.label_italic);

        self.x_axis_show_check.set_active(config.x_axis.show);
        self.x_axis_show_grid_check.set_active(config.x_axis.show_grid);
        update_color_button(&self.x_axis_color_button, &config.x_axis.color);
        update_color_button(&self.x_axis_grid_color_button, &config.x_axis.grid_color);
        update_color_button(&self.x_axis_label_color_button, &config.x_axis.label_color);
        self.x_axis_label_font_button.set_label(&format!("{} {:.0}", config.x_axis.label_font_family, config.x_axis.label_font_size));
        self.x_axis_label_font_size_spin.set_value(config.x_axis.label_font_size);
        self.x_axis_label_bold_check.set_active(config.x_axis.label_bold);
        self.x_axis_label_italic_check.set_active(config.x_axis.label_italic);

        self.margin_top_spin.set_value(config.margin.top);
        self.margin_right_spin.set_value(config.margin.right);
        self.margin_bottom_spin.set_value(config.margin.bottom);
        self.margin_left_spin.set_value(config.margin.left);

        update_color_button(&self.background_color_button, &config.background_color);
        update_color_button(&self.plot_background_color_button, &config.plot_background_color);

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
    line_color_button: Button,
    fill_mode_combo: DropDown,
    fill_color_button: Button,
    fill_gradient_start_button: Button,
    fill_gradient_end_button: Button,
    fill_opacity_spin: SpinButton,
    smooth_lines_check: CheckButton,
    animate_new_points_check: CheckButton,
}

struct DataPageWidgets {
    widget: GtkBox,
    max_points_spin: SpinButton,
    show_points_check: CheckButton,
    point_radius_spin: SpinButton,
    point_color_button: Button,
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
    y_axis_color_button: Button,
    y_axis_grid_color_button: Button,
    y_axis_label_color_button: Button,
    y_axis_label_font_button: Button,
    y_axis_label_font_size_spin: SpinButton,
    y_axis_label_bold_check: CheckButton,
    y_axis_label_italic_check: CheckButton,
    x_axis_show_check: CheckButton,
    x_axis_show_grid_check: CheckButton,
    x_axis_color_button: Button,
    x_axis_grid_color_button: Button,
    x_axis_label_color_button: Button,
    x_axis_label_font_button: Button,
    x_axis_label_font_size_spin: SpinButton,
    x_axis_label_bold_check: CheckButton,
    x_axis_label_italic_check: CheckButton,
}

struct LayoutPageWidgets {
    widget: GtkBox,
    margin_top_spin: SpinButton,
    margin_right_spin: SpinButton,
    margin_bottom_spin: SpinButton,
    margin_left_spin: SpinButton,
    background_color_button: Button,
    plot_background_color_button: Button,
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
    let line_color_button = Button::with_label("Line Color");
    update_color_button(&line_color_button, &config.borrow().line_color);
    line_color_box.append(&line_color_button);
    page.append(&line_color_box);

    let config_clone = config.clone();
    let line_btn_clone = line_color_button.clone();
    line_color_button.connect_clicked(move |btn| {
        let current_color = config_clone.borrow().line_color;
        let window = btn.root().and_then(|root| root.downcast::<gtk4::Window>().ok());
        let config_clone2 = config_clone.clone();
        let btn_clone = line_btn_clone.clone();
        gtk4::glib::MainContext::default().spawn_local(async move {
            if let Some(new_color) = ColorPickerDialog::pick_color(window.as_ref(), current_color).await {
                config_clone2.borrow_mut().line_color = new_color;
                update_color_button(&btn_clone, &new_color);
            }
        });
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
    let fill_color_button = Button::with_label("Fill Color");
    update_color_button(&fill_color_button, &config.borrow().fill_color);
    fill_color_box.append(&fill_color_button);
    page.append(&fill_color_box);

    let config_clone = config.clone();
    let fill_btn_clone = fill_color_button.clone();
    fill_color_button.connect_clicked(move |btn| {
        let current_color = config_clone.borrow().fill_color;
        let window = btn.root().and_then(|root| root.downcast::<gtk4::Window>().ok());
        let config_clone2 = config_clone.clone();
        let btn_clone = fill_btn_clone.clone();
        gtk4::glib::MainContext::default().spawn_local(async move {
            if let Some(new_color) = ColorPickerDialog::pick_color(window.as_ref(), current_color).await {
                config_clone2.borrow_mut().fill_color = new_color;
                update_color_button(&btn_clone, &new_color);
            }
        });
    });

    // Gradient colors
    let gradient_box = GtkBox::new(Orientation::Horizontal, 6);
    gradient_box.append(&Label::new(Some("Gradient Start:")));
    let fill_gradient_start_button = Button::with_label("Start");
    update_color_button(&fill_gradient_start_button, &config.borrow().fill_gradient_start);
    gradient_box.append(&fill_gradient_start_button);

    gradient_box.append(&Label::new(Some("End:")));
    let fill_gradient_end_button = Button::with_label("End");
    update_color_button(&fill_gradient_end_button, &config.borrow().fill_gradient_end);
    gradient_box.append(&fill_gradient_end_button);
    page.append(&gradient_box);

    let config_clone = config.clone();
    let start_btn_clone = fill_gradient_start_button.clone();
    fill_gradient_start_button.connect_clicked(move |btn| {
        let current_color = config_clone.borrow().fill_gradient_start;
        let window = btn.root().and_then(|root| root.downcast::<gtk4::Window>().ok());
        let config_clone2 = config_clone.clone();
        let btn_clone = start_btn_clone.clone();
        gtk4::glib::MainContext::default().spawn_local(async move {
            if let Some(new_color) = ColorPickerDialog::pick_color(window.as_ref(), current_color).await {
                config_clone2.borrow_mut().fill_gradient_start = new_color;
                update_color_button(&btn_clone, &new_color);
            }
        });
    });

    let config_clone = config.clone();
    let end_btn_clone = fill_gradient_end_button.clone();
    fill_gradient_end_button.connect_clicked(move |btn| {
        let current_color = config_clone.borrow().fill_gradient_end;
        let window = btn.root().and_then(|root| root.downcast::<gtk4::Window>().ok());
        let config_clone2 = config_clone.clone();
        let btn_clone = end_btn_clone.clone();
        gtk4::glib::MainContext::default().spawn_local(async move {
            if let Some(new_color) = ColorPickerDialog::pick_color(window.as_ref(), current_color).await {
                config_clone2.borrow_mut().fill_gradient_end = new_color;
                update_color_button(&btn_clone, &new_color);
            }
        });
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
    let point_color_button = Button::with_label("Point Color");
    update_color_button(&point_color_button, &config.borrow().point_color);
    point_color_box.append(&point_color_button);
    page.append(&point_color_box);

    let config_clone = config.clone();
    let point_btn_clone = point_color_button.clone();
    point_color_button.connect_clicked(move |btn| {
        let current_color = config_clone.borrow().point_color;
        let window = btn.root().and_then(|root| root.downcast::<gtk4::Window>().ok());
        let config_clone2 = config_clone.clone();
        let btn_clone = point_btn_clone.clone();
        gtk4::glib::MainContext::default().spawn_local(async move {
            if let Some(new_color) = ColorPickerDialog::pick_color(window.as_ref(), current_color).await {
                config_clone2.borrow_mut().point_color = new_color;
                update_color_button(&btn_clone, &new_color);
            }
        });
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
    let y_axis_color_button = Button::with_label("Y-Axis Color");
    update_color_button(&y_axis_color_button, &config.borrow().y_axis.color);
    y_color_box.append(&y_axis_color_button);
    page.append(&y_color_box);

    let config_clone = config.clone();
    let y_axis_btn_clone = y_axis_color_button.clone();
    y_axis_color_button.connect_clicked(move |btn| {
        let current_color = config_clone.borrow().y_axis.color;
        let window = btn.root().and_then(|root| root.downcast::<gtk4::Window>().ok());
        let config_clone2 = config_clone.clone();
        let btn_clone = y_axis_btn_clone.clone();
        gtk4::glib::MainContext::default().spawn_local(async move {
            if let Some(new_color) = ColorPickerDialog::pick_color(window.as_ref(), current_color).await {
                config_clone2.borrow_mut().y_axis.color = new_color;
                update_color_button(&btn_clone, &new_color);
            }
        });
    });

    let y_grid_color_box = GtkBox::new(Orientation::Horizontal, 6);
    y_grid_color_box.append(&Label::new(Some("Y-Grid Color:")));
    let y_axis_grid_color_button = Button::with_label("Y-Grid Color");
    update_color_button(&y_axis_grid_color_button, &config.borrow().y_axis.grid_color);
    y_grid_color_box.append(&y_axis_grid_color_button);
    page.append(&y_grid_color_box);

    let config_clone = config.clone();
    let y_grid_btn_clone = y_axis_grid_color_button.clone();
    y_axis_grid_color_button.connect_clicked(move |btn| {
        let current_color = config_clone.borrow().y_axis.grid_color;
        let window = btn.root().and_then(|root| root.downcast::<gtk4::Window>().ok());
        let config_clone2 = config_clone.clone();
        let btn_clone = y_grid_btn_clone.clone();
        gtk4::glib::MainContext::default().spawn_local(async move {
            if let Some(new_color) = ColorPickerDialog::pick_color(window.as_ref(), current_color).await {
                config_clone2.borrow_mut().y_axis.grid_color = new_color;
                update_color_button(&btn_clone, &new_color);
            }
        });
    });

    // Y-Axis label color button
    let y_label_color_box = GtkBox::new(Orientation::Horizontal, 6);
    y_label_color_box.append(&Label::new(Some("Label Color:")));
    let y_axis_label_color_button = Button::with_label("Label Color");
    update_color_button(&y_axis_label_color_button, &config.borrow().y_axis.label_color);
    y_label_color_box.append(&y_axis_label_color_button);
    page.append(&y_label_color_box);

    let config_clone = config.clone();
    let y_label_btn_clone = y_axis_label_color_button.clone();
    y_axis_label_color_button.connect_clicked(move |btn| {
        let current_color = config_clone.borrow().y_axis.label_color;
        let window = btn.root().and_then(|root| root.downcast::<gtk4::Window>().ok());
        let config_clone2 = config_clone.clone();
        let btn_clone = y_label_btn_clone.clone();
        gtk4::glib::MainContext::default().spawn_local(async move {
            if let Some(new_color) = ColorPickerDialog::pick_color(window.as_ref(), current_color).await {
                config_clone2.borrow_mut().y_axis.label_color = new_color;
                update_color_button(&btn_clone, &new_color);
            }
        });
    });

    // Y-Axis label font controls (using shared font dialog pattern)
    let y_label_font_box = GtkBox::new(Orientation::Horizontal, 6);
    y_label_font_box.append(&Label::new(Some("Label Font:")));

    // Font selection button
    let initial_y_font_label = format!("{} {:.0}",
        config.borrow().y_axis.label_font_family,
        config.borrow().y_axis.label_font_size
    );
    let y_axis_label_font_button = gtk4::Button::with_label(&initial_y_font_label);
    y_axis_label_font_button.set_hexpand(true);
    y_label_font_box.append(&y_axis_label_font_button);

    // Font size spinner
    y_label_font_box.append(&Label::new(Some("Size:")));
    let y_axis_label_font_size_spin = SpinButton::with_range(6.0, 72.0, 1.0);
    y_axis_label_font_size_spin.set_value(config.borrow().y_axis.label_font_size);
    y_axis_label_font_size_spin.set_width_chars(4);
    y_label_font_box.append(&y_axis_label_font_size_spin);

    // Bold/Italic checkboxes
    let y_axis_label_bold_check = CheckButton::with_label("B");
    y_axis_label_bold_check.set_tooltip_text(Some("Bold"));
    y_axis_label_bold_check.set_active(config.borrow().y_axis.label_bold);
    y_label_font_box.append(&y_axis_label_bold_check);

    let y_axis_label_italic_check = CheckButton::with_label("I");
    y_axis_label_italic_check.set_tooltip_text(Some("Italic"));
    y_axis_label_italic_check.set_active(config.borrow().y_axis.label_italic);
    y_label_font_box.append(&y_axis_label_italic_check);

    // Copy font button
    let y_copy_font_btn = gtk4::Button::with_label("Copy");
    let config_clone = config.clone();
    y_copy_font_btn.connect_clicked(move |_| {
        let cfg = config_clone.borrow();
        if let Ok(mut clipboard) = crate::ui::clipboard::CLIPBOARD.lock() {
            clipboard.copy_font(
                cfg.y_axis.label_font_family.clone(),
                cfg.y_axis.label_font_size,
                cfg.y_axis.label_bold,
                cfg.y_axis.label_italic,
            );
        }
    });
    y_label_font_box.append(&y_copy_font_btn);

    // Paste font button
    let y_paste_font_btn = gtk4::Button::with_label("Paste");
    let config_clone = config.clone();
    let y_font_button_clone = y_axis_label_font_button.clone();
    let y_size_spin_clone = y_axis_label_font_size_spin.clone();
    let y_bold_check_clone = y_axis_label_bold_check.clone();
    let y_italic_check_clone = y_axis_label_italic_check.clone();
    y_paste_font_btn.connect_clicked(move |_| {
        if let Ok(clipboard) = crate::ui::clipboard::CLIPBOARD.lock() {
            if let Some((family, size, bold, italic)) = clipboard.paste_font() {
                let mut cfg = config_clone.borrow_mut();
                cfg.y_axis.label_font_family = family.clone();
                cfg.y_axis.label_font_size = size;
                cfg.y_axis.label_bold = bold;
                cfg.y_axis.label_italic = italic;
                drop(cfg);
                // Update UI
                y_font_button_clone.set_label(&format!("{} {:.0}", family, size));
                y_size_spin_clone.set_value(size);
                y_bold_check_clone.set_active(bold);
                y_italic_check_clone.set_active(italic);
            }
        }
    });
    y_label_font_box.append(&y_paste_font_btn);

    page.append(&y_label_font_box);

    // Font size spinner change handler
    let config_clone = config.clone();
    let y_font_button_clone = y_axis_label_font_button.clone();
    y_axis_label_font_size_spin.connect_value_changed(move |spin| {
        let new_size = spin.value();
        config_clone.borrow_mut().y_axis.label_font_size = new_size;
        let family = config_clone.borrow().y_axis.label_font_family.clone();
        y_font_button_clone.set_label(&format!("{} {:.0}", family, new_size));
    });

    // Bold checkbox handler
    let config_clone = config.clone();
    y_axis_label_bold_check.connect_toggled(move |check| {
        config_clone.borrow_mut().y_axis.label_bold = check.is_active();
    });

    // Italic checkbox handler
    let config_clone = config.clone();
    y_axis_label_italic_check.connect_toggled(move |check| {
        config_clone.borrow_mut().y_axis.label_italic = check.is_active();
    });

    // Font button click handler - opens font dialog
    let config_clone = config.clone();
    let y_font_button_clone = y_axis_label_font_button.clone();
    let y_size_spin_clone = y_axis_label_font_size_spin.clone();
    y_axis_label_font_button.connect_clicked(move |btn| {
        let window = btn.root().and_then(|root| root.downcast::<gtk4::Window>().ok());

        // Get current font description
        let current_font = {
            let cfg = config_clone.borrow();
            let font_str = format!("{} {}", cfg.y_axis.label_font_family, cfg.y_axis.label_font_size as i32);
            gtk4::pango::FontDescription::from_string(&font_str)
        };

        let config_clone2 = config_clone.clone();
        let font_button_clone2 = y_font_button_clone.clone();
        let size_spin_clone2 = y_size_spin_clone.clone();

        // Use callback-based API for font selection with shared dialog
        crate::ui::shared_font_dialog::shared_font_dialog().choose_font(
            window.as_ref(),
            Some(&current_font),
            gtk4::gio::Cancellable::NONE,
            move |result| {
                if let Ok(font_desc) = result {
                    // Extract family and size from font description
                    let family = font_desc.family().map(|s| s.to_string()).unwrap_or_else(|| "Sans".to_string());
                    let size = font_desc.size() as f64 / gtk4::pango::SCALE as f64;

                    config_clone2.borrow_mut().y_axis.label_font_family = family.clone();
                    config_clone2.borrow_mut().y_axis.label_font_size = size;

                    // Update button label and size spinner
                    font_button_clone2.set_label(&format!("{} {:.0}", family, size));
                    size_spin_clone2.set_value(size);
                }
            },
        );
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
    let x_axis_color_button = Button::with_label("X-Axis Color");
    update_color_button(&x_axis_color_button, &config.borrow().x_axis.color);
    x_color_box.append(&x_axis_color_button);
    page.append(&x_color_box);

    let config_clone = config.clone();
    let x_axis_btn_clone = x_axis_color_button.clone();
    x_axis_color_button.connect_clicked(move |btn| {
        let current_color = config_clone.borrow().x_axis.color;
        let window = btn.root().and_then(|root| root.downcast::<gtk4::Window>().ok());
        let config_clone2 = config_clone.clone();
        let btn_clone = x_axis_btn_clone.clone();
        gtk4::glib::MainContext::default().spawn_local(async move {
            if let Some(new_color) = ColorPickerDialog::pick_color(window.as_ref(), current_color).await {
                config_clone2.borrow_mut().x_axis.color = new_color;
                update_color_button(&btn_clone, &new_color);
            }
        });
    });

    let x_grid_color_box = GtkBox::new(Orientation::Horizontal, 6);
    x_grid_color_box.append(&Label::new(Some("X-Grid Color:")));
    let x_axis_grid_color_button = Button::with_label("X-Grid Color");
    update_color_button(&x_axis_grid_color_button, &config.borrow().x_axis.grid_color);
    x_grid_color_box.append(&x_axis_grid_color_button);
    page.append(&x_grid_color_box);

    let config_clone = config.clone();
    let x_grid_btn_clone = x_axis_grid_color_button.clone();
    x_axis_grid_color_button.connect_clicked(move |btn| {
        let current_color = config_clone.borrow().x_axis.grid_color;
        let window = btn.root().and_then(|root| root.downcast::<gtk4::Window>().ok());
        let config_clone2 = config_clone.clone();
        let btn_clone = x_grid_btn_clone.clone();
        gtk4::glib::MainContext::default().spawn_local(async move {
            if let Some(new_color) = ColorPickerDialog::pick_color(window.as_ref(), current_color).await {
                config_clone2.borrow_mut().x_axis.grid_color = new_color;
                update_color_button(&btn_clone, &new_color);
            }
        });
    });

    // X-Axis label color button
    let x_label_color_box = GtkBox::new(Orientation::Horizontal, 6);
    x_label_color_box.append(&Label::new(Some("Label Color:")));
    let x_axis_label_color_button = Button::with_label("Label Color");
    update_color_button(&x_axis_label_color_button, &config.borrow().x_axis.label_color);
    x_label_color_box.append(&x_axis_label_color_button);
    page.append(&x_label_color_box);

    let config_clone = config.clone();
    let x_label_btn_clone = x_axis_label_color_button.clone();
    x_axis_label_color_button.connect_clicked(move |btn| {
        let current_color = config_clone.borrow().x_axis.label_color;
        let window = btn.root().and_then(|root| root.downcast::<gtk4::Window>().ok());
        let config_clone2 = config_clone.clone();
        let btn_clone = x_label_btn_clone.clone();
        gtk4::glib::MainContext::default().spawn_local(async move {
            if let Some(new_color) = ColorPickerDialog::pick_color(window.as_ref(), current_color).await {
                config_clone2.borrow_mut().x_axis.label_color = new_color;
                update_color_button(&btn_clone, &new_color);
            }
        });
    });

    // X-Axis label font controls (using shared font dialog pattern)
    let x_label_font_box = GtkBox::new(Orientation::Horizontal, 6);
    x_label_font_box.append(&Label::new(Some("Label Font:")));

    // Font selection button
    let initial_x_font_label = format!("{} {:.0}",
        config.borrow().x_axis.label_font_family,
        config.borrow().x_axis.label_font_size
    );
    let x_axis_label_font_button = gtk4::Button::with_label(&initial_x_font_label);
    x_axis_label_font_button.set_hexpand(true);
    x_label_font_box.append(&x_axis_label_font_button);

    // Font size spinner
    x_label_font_box.append(&Label::new(Some("Size:")));
    let x_axis_label_font_size_spin = SpinButton::with_range(6.0, 72.0, 1.0);
    x_axis_label_font_size_spin.set_value(config.borrow().x_axis.label_font_size);
    x_axis_label_font_size_spin.set_width_chars(4);
    x_label_font_box.append(&x_axis_label_font_size_spin);

    // Bold/Italic checkboxes
    let x_axis_label_bold_check = CheckButton::with_label("B");
    x_axis_label_bold_check.set_tooltip_text(Some("Bold"));
    x_axis_label_bold_check.set_active(config.borrow().x_axis.label_bold);
    x_label_font_box.append(&x_axis_label_bold_check);

    let x_axis_label_italic_check = CheckButton::with_label("I");
    x_axis_label_italic_check.set_tooltip_text(Some("Italic"));
    x_axis_label_italic_check.set_active(config.borrow().x_axis.label_italic);
    x_label_font_box.append(&x_axis_label_italic_check);

    // Copy font button
    let x_copy_font_btn = gtk4::Button::with_label("Copy");
    let config_clone = config.clone();
    x_copy_font_btn.connect_clicked(move |_| {
        let cfg = config_clone.borrow();
        if let Ok(mut clipboard) = crate::ui::clipboard::CLIPBOARD.lock() {
            clipboard.copy_font(
                cfg.x_axis.label_font_family.clone(),
                cfg.x_axis.label_font_size,
                cfg.x_axis.label_bold,
                cfg.x_axis.label_italic,
            );
        }
    });
    x_label_font_box.append(&x_copy_font_btn);

    // Paste font button
    let x_paste_font_btn = gtk4::Button::with_label("Paste");
    let config_clone = config.clone();
    let x_font_button_clone = x_axis_label_font_button.clone();
    let x_size_spin_clone = x_axis_label_font_size_spin.clone();
    let x_bold_check_clone = x_axis_label_bold_check.clone();
    let x_italic_check_clone = x_axis_label_italic_check.clone();
    x_paste_font_btn.connect_clicked(move |_| {
        if let Ok(clipboard) = crate::ui::clipboard::CLIPBOARD.lock() {
            if let Some((family, size, bold, italic)) = clipboard.paste_font() {
                let mut cfg = config_clone.borrow_mut();
                cfg.x_axis.label_font_family = family.clone();
                cfg.x_axis.label_font_size = size;
                cfg.x_axis.label_bold = bold;
                cfg.x_axis.label_italic = italic;
                drop(cfg);
                // Update UI
                x_font_button_clone.set_label(&format!("{} {:.0}", family, size));
                x_size_spin_clone.set_value(size);
                x_bold_check_clone.set_active(bold);
                x_italic_check_clone.set_active(italic);
            }
        }
    });
    x_label_font_box.append(&x_paste_font_btn);

    page.append(&x_label_font_box);

    // Font size spinner change handler
    let config_clone = config.clone();
    let x_font_button_clone = x_axis_label_font_button.clone();
    x_axis_label_font_size_spin.connect_value_changed(move |spin| {
        let new_size = spin.value();
        config_clone.borrow_mut().x_axis.label_font_size = new_size;
        let family = config_clone.borrow().x_axis.label_font_family.clone();
        x_font_button_clone.set_label(&format!("{} {:.0}", family, new_size));
    });

    // Bold checkbox handler
    let config_clone = config.clone();
    x_axis_label_bold_check.connect_toggled(move |check| {
        config_clone.borrow_mut().x_axis.label_bold = check.is_active();
    });

    // Italic checkbox handler
    let config_clone = config.clone();
    x_axis_label_italic_check.connect_toggled(move |check| {
        config_clone.borrow_mut().x_axis.label_italic = check.is_active();
    });

    // Font button click handler - opens font dialog
    let config_clone = config.clone();
    let x_font_button_clone = x_axis_label_font_button.clone();
    let x_size_spin_clone = x_axis_label_font_size_spin.clone();
    x_axis_label_font_button.connect_clicked(move |btn| {
        let window = btn.root().and_then(|root| root.downcast::<gtk4::Window>().ok());

        // Get current font description
        let current_font = {
            let cfg = config_clone.borrow();
            let font_str = format!("{} {}", cfg.x_axis.label_font_family, cfg.x_axis.label_font_size as i32);
            gtk4::pango::FontDescription::from_string(&font_str)
        };

        let config_clone2 = config_clone.clone();
        let font_button_clone2 = x_font_button_clone.clone();
        let size_spin_clone2 = x_size_spin_clone.clone();

        // Use callback-based API for font selection with shared dialog
        crate::ui::shared_font_dialog::shared_font_dialog().choose_font(
            window.as_ref(),
            Some(&current_font),
            gtk4::gio::Cancellable::NONE,
            move |result| {
                if let Ok(font_desc) = result {
                    // Extract family and size from font description
                    let family = font_desc.family().map(|s| s.to_string()).unwrap_or_else(|| "Sans".to_string());
                    let size = font_desc.size() as f64 / gtk4::pango::SCALE as f64;

                    config_clone2.borrow_mut().x_axis.label_font_family = family.clone();
                    config_clone2.borrow_mut().x_axis.label_font_size = size;

                    // Update button label and size spinner
                    font_button_clone2.set_label(&format!("{} {:.0}", family, size));
                    size_spin_clone2.set_value(size);
                }
            },
        );
    });

    AxesPageWidgets {
        widget: page,
        y_axis_show_check,
        y_axis_show_labels_check,
        y_axis_show_grid_check,
        y_axis_color_button,
        y_axis_grid_color_button,
        y_axis_label_color_button,
        y_axis_label_font_button,
        y_axis_label_font_size_spin,
        y_axis_label_bold_check,
        y_axis_label_italic_check,
        x_axis_show_check,
        x_axis_show_grid_check,
        x_axis_color_button,
        x_axis_grid_color_button,
        x_axis_label_color_button,
        x_axis_label_font_button,
        x_axis_label_font_size_spin,
        x_axis_label_bold_check,
        x_axis_label_italic_check,
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
    let background_color_button = Button::with_label("Background Color");
    update_color_button(&background_color_button, &config.borrow().background_color);
    bg_color_box.append(&background_color_button);
    page.append(&bg_color_box);

    let config_clone = config.clone();
    let bg_btn_clone = background_color_button.clone();
    background_color_button.connect_clicked(move |btn| {
        let current_color = config_clone.borrow().background_color;
        let window = btn.root().and_then(|root| root.downcast::<gtk4::Window>().ok());
        let config_clone2 = config_clone.clone();
        let btn_clone = bg_btn_clone.clone();
        gtk4::glib::MainContext::default().spawn_local(async move {
            if let Some(new_color) = ColorPickerDialog::pick_color(window.as_ref(), current_color).await {
                config_clone2.borrow_mut().background_color = new_color;
                update_color_button(&btn_clone, &new_color);
            }
        });
    });

    let plot_bg_color_box = GtkBox::new(Orientation::Horizontal, 6);
    plot_bg_color_box.append(&Label::new(Some("Plot Background:")));
    let plot_background_color_button = Button::with_label("Plot Background");
    update_color_button(&plot_background_color_button, &config.borrow().plot_background_color);
    plot_bg_color_box.append(&plot_background_color_button);
    page.append(&plot_bg_color_box);

    let config_clone = config.clone();
    let plot_bg_btn_clone = plot_background_color_button.clone();
    plot_background_color_button.connect_clicked(move |btn| {
        let current_color = config_clone.borrow().plot_background_color;
        let window = btn.root().and_then(|root| root.downcast::<gtk4::Window>().ok());
        let config_clone2 = config_clone.clone();
        let btn_clone = plot_bg_btn_clone.clone();
        gtk4::glib::MainContext::default().spawn_local(async move {
            if let Some(new_color) = ColorPickerDialog::pick_color(window.as_ref(), current_color).await {
                config_clone2.borrow_mut().plot_background_color = new_color;
                update_color_button(&btn_clone, &new_color);
            }
        });
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

    // Create 1 text line config widget (it has built-in multi-line support via add button)
    let mut text_config_widgets = Vec::new();
    let text_widget = Rc::new(TextLineConfigWidget::new(available_fields.clone()));
    page.append(text_widget.widget());

    // Note: We'll collect configs when get_config() is called
    text_config_widgets.push(text_widget);

    TextOverlayPageWidgets {
        widget: page,
        text_config_widgets,
    }
}

// Helper function to update color button label
fn update_color_button(btn: &Button, color: &Color) {
    btn.set_label(&format!("■ ({:.0},{:.0},{:.0})", color.r * 255.0, color.g * 255.0, color.b * 255.0));
}
