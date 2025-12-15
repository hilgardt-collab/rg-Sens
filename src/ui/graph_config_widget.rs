//! Graph displayer configuration widget

use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, CheckButton, DropDown, Label, Notebook, Orientation,
    SpinButton, StringList,
};
use std::cell::RefCell;
use std::rc::Rc;

use super::color_button_widget::ColorButtonWidget;
use super::graph_display::{FillMode, GraphDisplayConfig, GraphType, LineStyle};
use super::text_line_config_widget::TextLineConfigWidget;

pub struct GraphConfigWidget {
    widget: GtkBox,
    config: Rc<RefCell<GraphDisplayConfig>>,

    // Graph type controls
    graph_type_combo: DropDown,
    line_style_combo: DropDown,
    line_width_spin: SpinButton,
    line_color_widget: Rc<ColorButtonWidget>,

    // Fill controls
    fill_mode_combo: DropDown,
    fill_color_widget: Rc<ColorButtonWidget>,
    fill_gradient_start_widget: Rc<ColorButtonWidget>,
    fill_gradient_end_widget: Rc<ColorButtonWidget>,
    fill_opacity_spin: SpinButton,

    // Data points
    max_points_spin: SpinButton,
    show_points_check: CheckButton,
    point_radius_spin: SpinButton,
    point_color_widget: Rc<ColorButtonWidget>,

    // Scaling
    auto_scale_check: CheckButton,
    min_value_spin: SpinButton,
    max_value_spin: SpinButton,
    value_padding_spin: SpinButton,

    // Axes
    y_axis_show_check: CheckButton,
    y_axis_show_labels_check: CheckButton,
    y_axis_show_grid_check: CheckButton,
    y_axis_color_widget: Rc<ColorButtonWidget>,
    y_axis_grid_color_widget: Rc<ColorButtonWidget>,
    y_axis_label_color_widget: Rc<ColorButtonWidget>,
    y_axis_label_font_button: Button,
    y_axis_label_font_size_spin: SpinButton,
    y_axis_label_bold_check: CheckButton,
    y_axis_label_italic_check: CheckButton,

    x_axis_show_check: CheckButton,
    x_axis_show_grid_check: CheckButton,
    x_axis_color_widget: Rc<ColorButtonWidget>,
    x_axis_grid_color_widget: Rc<ColorButtonWidget>,
    x_axis_label_color_widget: Rc<ColorButtonWidget>,
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
    background_color_widget: Rc<ColorButtonWidget>,
    plot_background_color_widget: Rc<ColorButtonWidget>,

    // Animation and smoothing
    smooth_lines_check: CheckButton,
    animate_new_points_check: CheckButton,

    // Text overlay
    text_config_widgets: Vec<Rc<TextLineConfigWidget>>,

    // Change callback
    on_change: Rc<RefCell<Option<Box<dyn Fn()>>>>,
}

/// Type alias for the on_change callback
type OnChangeCallback = Rc<RefCell<Option<Box<dyn Fn()>>>>;

/// Helper function to notify that config has changed (static version)
fn notify_change_static(on_change: &OnChangeCallback) {
    if let Some(ref callback) = *on_change.borrow() {
        callback();
    }
}

impl GraphConfigWidget {
    pub fn new(available_fields: Vec<crate::core::FieldMetadata>) -> Self {
        log::info!("=== GraphConfigWidget::new() called with {} available fields ===", available_fields.len());
        let widget = GtkBox::new(Orientation::Vertical, 0);
        let config = Rc::new(RefCell::new(GraphDisplayConfig::default()));

        // Create on_change callback BEFORE creating pages so we can pass it to them
        let on_change: OnChangeCallback = Rc::new(RefCell::new(None));

        let notebook = Notebook::new();
        notebook.set_scrollable(true);

        // === Tab 1: Graph Style ===
        let style_page = create_style_page(config.clone(), on_change.clone());
        notebook.append_page(&style_page.widget, Some(&Label::new(Some("Style"))));

        // === Tab 2: Data & Scaling ===
        let data_page = create_data_page(config.clone(), on_change.clone());
        notebook.append_page(&data_page.widget, Some(&Label::new(Some("Data"))));

        // === Tab 3: Axes ===
        let axes_page = create_axes_page(config.clone(), on_change.clone());
        notebook.append_page(&axes_page.widget, Some(&Label::new(Some("Axes"))));

        // === Tab 4: Layout ===
        let layout_page = create_layout_page(config.clone(), on_change.clone());
        notebook.append_page(&layout_page.widget, Some(&Label::new(Some("Layout"))));

        // === Tab 5: Text Overlay ===
        let text_page = create_text_overlay_page(config.clone(), available_fields, on_change.clone());
        notebook.append_page(&text_page.widget, Some(&Label::new(Some("Text"))));

        // === Copy/Paste buttons for entire graph config ===
        let copy_paste_box = GtkBox::new(Orientation::Horizontal, 6);
        copy_paste_box.set_halign(gtk4::Align::End);
        copy_paste_box.set_margin_bottom(6);

        let copy_btn = Button::with_label("Copy Graph Config");
        let paste_btn = Button::with_label("Paste Graph Config");

        copy_paste_box.append(&copy_btn);
        copy_paste_box.append(&paste_btn);

        widget.append(&copy_paste_box);
        widget.append(&notebook);

        // Copy button handler
        let config_for_copy = config.clone();
        copy_btn.connect_clicked(move |_| {
            let cfg = config_for_copy.borrow().clone();
            if let Ok(mut clipboard) = crate::ui::clipboard::CLIPBOARD.lock() {
                clipboard.copy_graph_display(cfg);
            }
        });

        // Paste button handler - clone all widgets needed for updating
        let config_for_paste = config.clone();
        let on_change_for_paste = on_change.clone();
        // Style page widgets
        let graph_type_combo_p = style_page.graph_type_combo.clone();
        let line_style_combo_p = style_page.line_style_combo.clone();
        let line_width_spin_p = style_page.line_width_spin.clone();
        let line_color_widget_p = style_page.line_color_widget.clone();
        let fill_mode_combo_p = style_page.fill_mode_combo.clone();
        let fill_color_widget_p = style_page.fill_color_widget.clone();
        let fill_gradient_start_widget_p = style_page.fill_gradient_start_widget.clone();
        let fill_gradient_end_widget_p = style_page.fill_gradient_end_widget.clone();
        let fill_opacity_spin_p = style_page.fill_opacity_spin.clone();
        let smooth_lines_check_p = style_page.smooth_lines_check.clone();
        let animate_new_points_check_p = style_page.animate_new_points_check.clone();
        // Data page widgets
        let max_points_spin_p = data_page.max_points_spin.clone();
        let show_points_check_p = data_page.show_points_check.clone();
        let point_radius_spin_p = data_page.point_radius_spin.clone();
        let point_color_widget_p = data_page.point_color_widget.clone();
        let auto_scale_check_p = data_page.auto_scale_check.clone();
        let min_value_spin_p = data_page.min_value_spin.clone();
        let max_value_spin_p = data_page.max_value_spin.clone();
        let value_padding_spin_p = data_page.value_padding_spin.clone();
        // Axes page widgets
        let y_axis_show_check_p = axes_page.y_axis_show_check.clone();
        let y_axis_show_labels_check_p = axes_page.y_axis_show_labels_check.clone();
        let y_axis_show_grid_check_p = axes_page.y_axis_show_grid_check.clone();
        let y_axis_color_widget_p = axes_page.y_axis_color_widget.clone();
        let y_axis_grid_color_widget_p = axes_page.y_axis_grid_color_widget.clone();
        let y_axis_label_color_widget_p = axes_page.y_axis_label_color_widget.clone();
        let y_axis_label_font_button_p = axes_page.y_axis_label_font_button.clone();
        let y_axis_label_font_size_spin_p = axes_page.y_axis_label_font_size_spin.clone();
        let y_axis_label_bold_check_p = axes_page.y_axis_label_bold_check.clone();
        let y_axis_label_italic_check_p = axes_page.y_axis_label_italic_check.clone();
        let x_axis_show_check_p = axes_page.x_axis_show_check.clone();
        let x_axis_show_grid_check_p = axes_page.x_axis_show_grid_check.clone();
        let x_axis_color_widget_p = axes_page.x_axis_color_widget.clone();
        let x_axis_grid_color_widget_p = axes_page.x_axis_grid_color_widget.clone();
        let x_axis_label_color_widget_p = axes_page.x_axis_label_color_widget.clone();
        let x_axis_label_font_button_p = axes_page.x_axis_label_font_button.clone();
        let x_axis_label_font_size_spin_p = axes_page.x_axis_label_font_size_spin.clone();
        let x_axis_label_bold_check_p = axes_page.x_axis_label_bold_check.clone();
        let x_axis_label_italic_check_p = axes_page.x_axis_label_italic_check.clone();
        // Layout page widgets
        let margin_top_spin_p = layout_page.margin_top_spin.clone();
        let margin_right_spin_p = layout_page.margin_right_spin.clone();
        let margin_bottom_spin_p = layout_page.margin_bottom_spin.clone();
        let margin_left_spin_p = layout_page.margin_left_spin.clone();
        let background_color_widget_p = layout_page.background_color_widget.clone();
        let plot_background_color_widget_p = layout_page.plot_background_color_widget.clone();
        // Text page widgets
        let text_config_widgets_p = text_page.text_config_widgets.clone();

        paste_btn.connect_clicked(move |_| {
            let pasted = if let Ok(clipboard) = crate::ui::clipboard::CLIPBOARD.lock() {
                clipboard.paste_graph_display()
            } else {
                None
            };

            if let Some(cfg) = pasted {
                // Update stored config
                *config_for_paste.borrow_mut() = cfg.clone();

                // Update all UI widgets
                graph_type_combo_p.set_selected(match cfg.graph_type {
                    GraphType::Line => 0,
                    GraphType::Bar => 1,
                    GraphType::Area => 2,
                    GraphType::SteppedLine => 3,
                });
                line_style_combo_p.set_selected(match cfg.line_style {
                    LineStyle::Solid => 0,
                    LineStyle::Dashed => 1,
                    LineStyle::Dotted => 2,
                });
                line_width_spin_p.set_value(cfg.line_width);
                line_color_widget_p.set_color(cfg.line_color);
                fill_mode_combo_p.set_selected(match cfg.fill_mode {
                    FillMode::None => 0,
                    FillMode::Solid => 1,
                    FillMode::Gradient => 2,
                });
                fill_color_widget_p.set_color(cfg.fill_color);
                fill_gradient_start_widget_p.set_color(cfg.fill_gradient_start);
                fill_gradient_end_widget_p.set_color(cfg.fill_gradient_end);
                fill_opacity_spin_p.set_value(cfg.fill_opacity);
                smooth_lines_check_p.set_active(cfg.smooth_lines);
                animate_new_points_check_p.set_active(cfg.animate_new_points);

                max_points_spin_p.set_value(cfg.max_data_points as f64);
                show_points_check_p.set_active(cfg.show_points);
                point_radius_spin_p.set_value(cfg.point_radius);
                point_color_widget_p.set_color(cfg.point_color);
                auto_scale_check_p.set_active(cfg.auto_scale);
                min_value_spin_p.set_value(cfg.min_value);
                max_value_spin_p.set_value(cfg.max_value);
                value_padding_spin_p.set_value(cfg.value_padding);

                y_axis_show_check_p.set_active(cfg.y_axis.show);
                y_axis_show_labels_check_p.set_active(cfg.y_axis.show_labels);
                y_axis_show_grid_check_p.set_active(cfg.y_axis.show_grid);
                y_axis_color_widget_p.set_color(cfg.y_axis.color);
                y_axis_grid_color_widget_p.set_color(cfg.y_axis.grid_color);
                y_axis_label_color_widget_p.set_color(cfg.y_axis.label_color);
                y_axis_label_font_button_p.set_label(&format!("{} {:.0}", cfg.y_axis.label_font_family, cfg.y_axis.label_font_size));
                y_axis_label_font_size_spin_p.set_value(cfg.y_axis.label_font_size);
                y_axis_label_bold_check_p.set_active(cfg.y_axis.label_bold);
                y_axis_label_italic_check_p.set_active(cfg.y_axis.label_italic);

                x_axis_show_check_p.set_active(cfg.x_axis.show);
                x_axis_show_grid_check_p.set_active(cfg.x_axis.show_grid);
                x_axis_color_widget_p.set_color(cfg.x_axis.color);
                x_axis_grid_color_widget_p.set_color(cfg.x_axis.grid_color);
                x_axis_label_color_widget_p.set_color(cfg.x_axis.label_color);
                x_axis_label_font_button_p.set_label(&format!("{} {:.0}", cfg.x_axis.label_font_family, cfg.x_axis.label_font_size));
                x_axis_label_font_size_spin_p.set_value(cfg.x_axis.label_font_size);
                x_axis_label_bold_check_p.set_active(cfg.x_axis.label_bold);
                x_axis_label_italic_check_p.set_active(cfg.x_axis.label_italic);

                margin_top_spin_p.set_value(cfg.margin.top);
                margin_right_spin_p.set_value(cfg.margin.right);
                margin_bottom_spin_p.set_value(cfg.margin.bottom);
                margin_left_spin_p.set_value(cfg.margin.left);

                background_color_widget_p.set_color(cfg.background_color);
                plot_background_color_widget_p.set_color(cfg.plot_background_color);

                // Update text overlay - pass all lines to the single widget
                if !text_config_widgets_p.is_empty() {
                    let text_displayer_config = crate::displayers::TextDisplayerConfig {
                        lines: cfg.text_overlay.clone(),
                    };
                    text_config_widgets_p[0].set_config(text_displayer_config);
                }

                // Trigger on_change
                if let Some(ref callback) = *on_change_for_paste.borrow() {
                    callback();
                }
            }
        });

        Self {
            widget,
            config,
            graph_type_combo: style_page.graph_type_combo,
            line_style_combo: style_page.line_style_combo,
            line_width_spin: style_page.line_width_spin,
            line_color_widget: style_page.line_color_widget,
            fill_mode_combo: style_page.fill_mode_combo,
            fill_color_widget: style_page.fill_color_widget,
            fill_gradient_start_widget: style_page.fill_gradient_start_widget,
            fill_gradient_end_widget: style_page.fill_gradient_end_widget,
            fill_opacity_spin: style_page.fill_opacity_spin,
            max_points_spin: data_page.max_points_spin,
            show_points_check: data_page.show_points_check,
            point_radius_spin: data_page.point_radius_spin,
            point_color_widget: data_page.point_color_widget,
            auto_scale_check: data_page.auto_scale_check,
            min_value_spin: data_page.min_value_spin,
            max_value_spin: data_page.max_value_spin,
            value_padding_spin: data_page.value_padding_spin,
            y_axis_show_check: axes_page.y_axis_show_check,
            y_axis_show_labels_check: axes_page.y_axis_show_labels_check,
            y_axis_show_grid_check: axes_page.y_axis_show_grid_check,
            y_axis_color_widget: axes_page.y_axis_color_widget,
            y_axis_grid_color_widget: axes_page.y_axis_grid_color_widget,
            y_axis_label_color_widget: axes_page.y_axis_label_color_widget,
            y_axis_label_font_button: axes_page.y_axis_label_font_button,
            y_axis_label_font_size_spin: axes_page.y_axis_label_font_size_spin,
            y_axis_label_bold_check: axes_page.y_axis_label_bold_check,
            y_axis_label_italic_check: axes_page.y_axis_label_italic_check,
            x_axis_show_check: axes_page.x_axis_show_check,
            x_axis_show_grid_check: axes_page.x_axis_show_grid_check,
            x_axis_color_widget: axes_page.x_axis_color_widget,
            x_axis_grid_color_widget: axes_page.x_axis_grid_color_widget,
            x_axis_label_color_widget: axes_page.x_axis_label_color_widget,
            x_axis_label_font_button: axes_page.x_axis_label_font_button,
            x_axis_label_font_size_spin: axes_page.x_axis_label_font_size_spin,
            x_axis_label_bold_check: axes_page.x_axis_label_bold_check,
            x_axis_label_italic_check: axes_page.x_axis_label_italic_check,
            margin_top_spin: layout_page.margin_top_spin,
            margin_right_spin: layout_page.margin_right_spin,
            margin_bottom_spin: layout_page.margin_bottom_spin,
            margin_left_spin: layout_page.margin_left_spin,
            background_color_widget: layout_page.background_color_widget,
            plot_background_color_widget: layout_page.plot_background_color_widget,
            smooth_lines_check: style_page.smooth_lines_check,
            animate_new_points_check: style_page.animate_new_points_check,
            text_config_widgets: text_page.text_config_widgets,
            on_change,
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
            .flat_map(|w| {
                let lines = w.get_config().lines;
                log::debug!("GraphConfigWidget::get_config - widget has {} text lines", lines.len());
                lines
            })
            .collect();

        log::debug!("GraphConfigWidget::get_config - total text_overlay lines: {}", config.text_overlay.len());
        config
    }

    /// Set a callback that will be called when the config changes
    pub fn set_on_change<F: Fn() + 'static>(&self, callback: F) {
        *self.on_change.borrow_mut() = Some(Box::new(callback));
    }

    /// Notify that config has changed (available for direct calls on the widget)
    #[allow(dead_code)]
    fn notify_change(&self) {
        notify_change_static(&self.on_change);
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
        self.line_color_widget.set_color(config.line_color);

        self.fill_mode_combo.set_selected(match config.fill_mode {
            FillMode::None => 0,
            FillMode::Solid => 1,
            FillMode::Gradient => 2,
        });

        self.fill_color_widget.set_color(config.fill_color);
        self.fill_gradient_start_widget.set_color(config.fill_gradient_start);
        self.fill_gradient_end_widget.set_color(config.fill_gradient_end);
        self.fill_opacity_spin.set_value(config.fill_opacity);

        self.max_points_spin.set_value(config.max_data_points as f64);
        self.show_points_check.set_active(config.show_points);
        self.point_radius_spin.set_value(config.point_radius);
        self.point_color_widget.set_color(config.point_color);

        self.auto_scale_check.set_active(config.auto_scale);
        self.min_value_spin.set_value(config.min_value);
        self.max_value_spin.set_value(config.max_value);
        self.value_padding_spin.set_value(config.value_padding);

        self.y_axis_show_check.set_active(config.y_axis.show);
        self.y_axis_show_labels_check.set_active(config.y_axis.show_labels);
        self.y_axis_show_grid_check.set_active(config.y_axis.show_grid);
        self.y_axis_color_widget.set_color(config.y_axis.color);
        self.y_axis_grid_color_widget.set_color(config.y_axis.grid_color);
        self.y_axis_label_color_widget.set_color(config.y_axis.label_color);
        self.y_axis_label_font_button.set_label(&format!("{} {:.0}", config.y_axis.label_font_family, config.y_axis.label_font_size));
        self.y_axis_label_font_size_spin.set_value(config.y_axis.label_font_size);
        self.y_axis_label_bold_check.set_active(config.y_axis.label_bold);
        self.y_axis_label_italic_check.set_active(config.y_axis.label_italic);

        self.x_axis_show_check.set_active(config.x_axis.show);
        self.x_axis_show_grid_check.set_active(config.x_axis.show_grid);
        self.x_axis_color_widget.set_color(config.x_axis.color);
        self.x_axis_grid_color_widget.set_color(config.x_axis.grid_color);
        self.x_axis_label_color_widget.set_color(config.x_axis.label_color);
        self.x_axis_label_font_button.set_label(&format!("{} {:.0}", config.x_axis.label_font_family, config.x_axis.label_font_size));
        self.x_axis_label_font_size_spin.set_value(config.x_axis.label_font_size);
        self.x_axis_label_bold_check.set_active(config.x_axis.label_bold);
        self.x_axis_label_italic_check.set_active(config.x_axis.label_italic);

        self.margin_top_spin.set_value(config.margin.top);
        self.margin_right_spin.set_value(config.margin.right);
        self.margin_bottom_spin.set_value(config.margin.bottom);
        self.margin_left_spin.set_value(config.margin.left);

        self.background_color_widget.set_color(config.background_color);
        self.plot_background_color_widget.set_color(config.plot_background_color);

        self.smooth_lines_check.set_active(config.smooth_lines);
        self.animate_new_points_check.set_active(config.animate_new_points);

        // Set text overlay configs - pass all lines to the single widget
        log::debug!(
            "GraphConfigWidget::set_config: text_overlay has {} lines, text_config_widgets count: {}",
            config.text_overlay.len(),
            self.text_config_widgets.len()
        );
        if !self.text_config_widgets.is_empty() {
            let text_displayer_config = crate::displayers::TextDisplayerConfig {
                lines: config.text_overlay.clone(),
            };
            self.text_config_widgets[0].set_config(text_displayer_config);
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
    line_color_widget: Rc<ColorButtonWidget>,
    fill_mode_combo: DropDown,
    fill_color_widget: Rc<ColorButtonWidget>,
    fill_gradient_start_widget: Rc<ColorButtonWidget>,
    fill_gradient_end_widget: Rc<ColorButtonWidget>,
    fill_opacity_spin: SpinButton,
    smooth_lines_check: CheckButton,
    animate_new_points_check: CheckButton,
}

struct DataPageWidgets {
    widget: GtkBox,
    max_points_spin: SpinButton,
    show_points_check: CheckButton,
    point_radius_spin: SpinButton,
    point_color_widget: Rc<ColorButtonWidget>,
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
    y_axis_color_widget: Rc<ColorButtonWidget>,
    y_axis_grid_color_widget: Rc<ColorButtonWidget>,
    y_axis_label_color_widget: Rc<ColorButtonWidget>,
    y_axis_label_font_button: Button,
    y_axis_label_font_size_spin: SpinButton,
    y_axis_label_bold_check: CheckButton,
    y_axis_label_italic_check: CheckButton,
    x_axis_show_check: CheckButton,
    x_axis_show_grid_check: CheckButton,
    x_axis_color_widget: Rc<ColorButtonWidget>,
    x_axis_grid_color_widget: Rc<ColorButtonWidget>,
    x_axis_label_color_widget: Rc<ColorButtonWidget>,
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
    background_color_widget: Rc<ColorButtonWidget>,
    plot_background_color_widget: Rc<ColorButtonWidget>,
}

struct TextOverlayPageWidgets {
    widget: GtkBox,
    text_config_widgets: Vec<Rc<TextLineConfigWidget>>,
}

fn create_style_page(config: Rc<RefCell<GraphDisplayConfig>>, on_change: OnChangeCallback) -> StylePageWidgets {
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
    let on_change_clone = on_change.clone();
    graph_type_combo.connect_selected_notify(move |combo| {
        let graph_type = match combo.selected() {
            0 => GraphType::Line,
            1 => GraphType::Bar,
            2 => GraphType::Area,
            3 => GraphType::SteppedLine,
            _ => GraphType::Line,
        };
        config_clone.borrow_mut().graph_type = graph_type;
        notify_change_static(&on_change_clone);
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
    let on_change_clone = on_change.clone();
    line_style_combo.connect_selected_notify(move |combo| {
        let line_style = match combo.selected() {
            0 => LineStyle::Solid,
            1 => LineStyle::Dashed,
            2 => LineStyle::Dotted,
            _ => LineStyle::Solid,
        };
        config_clone.borrow_mut().line_style = line_style;
        notify_change_static(&on_change_clone);
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
    let on_change_clone = on_change.clone();
    line_width_spin.connect_value_changed(move |spin| {
        config_clone.borrow_mut().line_width = spin.value();
        notify_change_static(&on_change_clone);
    });

    // Line color - using ColorButtonWidget
    let line_color_box = GtkBox::new(Orientation::Horizontal, 6);
    line_color_box.append(&Label::new(Some("Line Color:")));
    let line_color_widget = Rc::new(ColorButtonWidget::new(config.borrow().line_color));
    line_color_box.append(line_color_widget.widget());
    page.append(&line_color_box);

    let config_clone = config.clone();
    let on_change_clone = on_change.clone();
    line_color_widget.set_on_change(move |color| {
        config_clone.borrow_mut().line_color = color;
        notify_change_static(&on_change_clone);
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
    let on_change_clone = on_change.clone();
    fill_mode_combo.connect_selected_notify(move |combo| {
        let fill_mode = match combo.selected() {
            0 => FillMode::None,
            1 => FillMode::Solid,
            2 => FillMode::Gradient,
            _ => FillMode::None,
        };
        config_clone.borrow_mut().fill_mode = fill_mode;
        notify_change_static(&on_change_clone);
    });

    // Fill color - using ColorButtonWidget
    let fill_color_box = GtkBox::new(Orientation::Horizontal, 6);
    fill_color_box.append(&Label::new(Some("Fill Color:")));
    let fill_color_widget = Rc::new(ColorButtonWidget::new(config.borrow().fill_color));
    fill_color_box.append(fill_color_widget.widget());
    page.append(&fill_color_box);

    let config_clone = config.clone();
    let on_change_clone = on_change.clone();
    fill_color_widget.set_on_change(move |color| {
        config_clone.borrow_mut().fill_color = color;
        notify_change_static(&on_change_clone);
    });

    // Gradient start color - using ColorButtonWidget
    let gradient_start_box = GtkBox::new(Orientation::Horizontal, 6);
    gradient_start_box.append(&Label::new(Some("Gradient Start:")));
    let fill_gradient_start_widget = Rc::new(ColorButtonWidget::new(config.borrow().fill_gradient_start));
    gradient_start_box.append(fill_gradient_start_widget.widget());
    page.append(&gradient_start_box);

    let config_clone = config.clone();
    let on_change_clone = on_change.clone();
    fill_gradient_start_widget.set_on_change(move |color| {
        config_clone.borrow_mut().fill_gradient_start = color;
        notify_change_static(&on_change_clone);
    });

    // Gradient end color - using ColorButtonWidget
    let gradient_end_box = GtkBox::new(Orientation::Horizontal, 6);
    gradient_end_box.append(&Label::new(Some("Gradient End:")));
    let fill_gradient_end_widget = Rc::new(ColorButtonWidget::new(config.borrow().fill_gradient_end));
    gradient_end_box.append(fill_gradient_end_widget.widget());
    page.append(&gradient_end_box);

    let config_clone = config.clone();
    let on_change_clone = on_change.clone();
    fill_gradient_end_widget.set_on_change(move |color| {
        config_clone.borrow_mut().fill_gradient_end = color;
        notify_change_static(&on_change_clone);
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
    let on_change_clone = on_change.clone();
    fill_opacity_spin.connect_value_changed(move |spin| {
        config_clone.borrow_mut().fill_opacity = spin.value();
        notify_change_static(&on_change_clone);
    });

    // Smooth lines checkbox
    let smooth_lines_check = CheckButton::with_label("Smooth Lines (Bezier Curves)");
    smooth_lines_check.set_active(true);
    page.append(&smooth_lines_check);

    let config_clone = config.clone();
    let on_change_clone = on_change.clone();
    smooth_lines_check.connect_toggled(move |check| {
        config_clone.borrow_mut().smooth_lines = check.is_active();
        notify_change_static(&on_change_clone);
    });

    // Animate new points checkbox
    let animate_new_points_check = CheckButton::with_label("Animate Graph Values");
    animate_new_points_check.set_active(false);
    page.append(&animate_new_points_check);

    let config_clone = config.clone();
    let on_change_clone = on_change.clone();
    animate_new_points_check.connect_toggled(move |check| {
        config_clone.borrow_mut().animate_new_points = check.is_active();
        notify_change_static(&on_change_clone);
    });

    StylePageWidgets {
        widget: page,
        graph_type_combo,
        line_style_combo,
        line_width_spin,
        line_color_widget,
        fill_mode_combo,
        fill_color_widget,
        fill_gradient_start_widget,
        fill_gradient_end_widget,
        fill_opacity_spin,
        smooth_lines_check,
        animate_new_points_check,
    }
}

fn create_data_page(config: Rc<RefCell<GraphDisplayConfig>>, on_change: OnChangeCallback) -> DataPageWidgets {
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
    let on_change_clone = on_change.clone();
    max_points_spin.connect_value_changed(move |spin| {
        config_clone.borrow_mut().max_data_points = spin.value() as usize;
        notify_change_static(&on_change_clone);
    });

    // Show points
    let show_points_check = CheckButton::with_label("Show Data Points");
    page.append(&show_points_check);

    let config_clone = config.clone();
    let on_change_clone = on_change.clone();
    show_points_check.connect_toggled(move |check| {
        config_clone.borrow_mut().show_points = check.is_active();
        notify_change_static(&on_change_clone);
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
    let on_change_clone = on_change.clone();
    point_radius_spin.connect_value_changed(move |spin| {
        config_clone.borrow_mut().point_radius = spin.value();
        notify_change_static(&on_change_clone);
    });

    // Point color - using ColorButtonWidget
    let point_color_box = GtkBox::new(Orientation::Horizontal, 6);
    point_color_box.append(&Label::new(Some("Point Color:")));
    let point_color_widget = Rc::new(ColorButtonWidget::new(config.borrow().point_color));
    point_color_box.append(point_color_widget.widget());
    page.append(&point_color_box);

    let config_clone = config.clone();
    let on_change_clone = on_change.clone();
    point_color_widget.set_on_change(move |color| {
        config_clone.borrow_mut().point_color = color;
        notify_change_static(&on_change_clone);
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
    let on_change_clone = on_change.clone();
    auto_scale_check.connect_toggled(move |check| {
        let active = check.is_active();
        config_clone.borrow_mut().auto_scale = active;
        min_spin_clone.set_sensitive(!active);
        max_spin_clone.set_sensitive(!active);
        notify_change_static(&on_change_clone);
    });

    // Min/Max values
    min_value_spin.set_value(0.0);
    min_value_spin.set_sensitive(false);
    let min_box = GtkBox::new(Orientation::Horizontal, 6);
    min_box.append(&Label::new(Some("Min Value:")));
    min_box.append(&min_value_spin);
    page.append(&min_box);

    let config_clone = config.clone();
    let on_change_clone = on_change.clone();
    min_value_spin.connect_value_changed(move |spin| {
        config_clone.borrow_mut().min_value = spin.value();
        notify_change_static(&on_change_clone);
    });

    max_value_spin.set_value(100.0);
    max_value_spin.set_sensitive(false);
    let max_box = GtkBox::new(Orientation::Horizontal, 6);
    max_box.append(&Label::new(Some("Max Value:")));
    max_box.append(&max_value_spin);
    page.append(&max_box);

    let config_clone = config.clone();
    let on_change_clone = on_change.clone();
    max_value_spin.connect_value_changed(move |spin| {
        config_clone.borrow_mut().max_value = spin.value();
        notify_change_static(&on_change_clone);
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
    let on_change_clone = on_change.clone();
    value_padding_spin.connect_value_changed(move |spin| {
        config_clone.borrow_mut().value_padding = spin.value();
        notify_change_static(&on_change_clone);
    });

    DataPageWidgets {
        widget: page,
        max_points_spin,
        show_points_check,
        point_radius_spin,
        point_color_widget,
        auto_scale_check,
        min_value_spin,
        max_value_spin,
        value_padding_spin,
    }
}

fn create_axes_page(config: Rc<RefCell<GraphDisplayConfig>>, on_change: OnChangeCallback) -> AxesPageWidgets {
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
    let on_change_clone = on_change.clone();
    y_axis_show_check.connect_toggled(move |check| {
        config_clone.borrow_mut().y_axis.show = check.is_active();
        notify_change_static(&on_change_clone);
    });

    let y_axis_show_labels_check = CheckButton::with_label("Show Y-Axis Labels");
    y_axis_show_labels_check.set_active(true);
    page.append(&y_axis_show_labels_check);

    let config_clone = config.clone();
    let on_change_clone = on_change.clone();
    y_axis_show_labels_check.connect_toggled(move |check| {
        config_clone.borrow_mut().y_axis.show_labels = check.is_active();
        notify_change_static(&on_change_clone);
    });

    let y_axis_show_grid_check = CheckButton::with_label("Show Y-Axis Grid");
    y_axis_show_grid_check.set_active(true);
    page.append(&y_axis_show_grid_check);

    let config_clone = config.clone();
    let on_change_clone = on_change.clone();
    y_axis_show_grid_check.connect_toggled(move |check| {
        config_clone.borrow_mut().y_axis.show_grid = check.is_active();
        notify_change_static(&on_change_clone);
    });

    // Y-Axis color - using ColorButtonWidget
    let y_color_box = GtkBox::new(Orientation::Horizontal, 6);
    y_color_box.append(&Label::new(Some("Y-Axis Color:")));
    let y_axis_color_widget = Rc::new(ColorButtonWidget::new(config.borrow().y_axis.color));
    y_color_box.append(y_axis_color_widget.widget());
    page.append(&y_color_box);

    let config_clone = config.clone();
    let on_change_clone = on_change.clone();
    y_axis_color_widget.set_on_change(move |color| {
        config_clone.borrow_mut().y_axis.color = color;
        notify_change_static(&on_change_clone);
    });

    // Y-Grid color - using ColorButtonWidget
    let y_grid_color_box = GtkBox::new(Orientation::Horizontal, 6);
    y_grid_color_box.append(&Label::new(Some("Y-Grid Color:")));
    let y_axis_grid_color_widget = Rc::new(ColorButtonWidget::new(config.borrow().y_axis.grid_color));
    y_grid_color_box.append(y_axis_grid_color_widget.widget());
    page.append(&y_grid_color_box);

    let config_clone = config.clone();
    let on_change_clone = on_change.clone();
    y_axis_grid_color_widget.set_on_change(move |color| {
        config_clone.borrow_mut().y_axis.grid_color = color;
        notify_change_static(&on_change_clone);
    });

    // Y-Axis label color - using ColorButtonWidget
    let y_label_color_box = GtkBox::new(Orientation::Horizontal, 6);
    y_label_color_box.append(&Label::new(Some("Label Color:")));
    let y_axis_label_color_widget = Rc::new(ColorButtonWidget::new(config.borrow().y_axis.label_color));
    y_label_color_box.append(y_axis_label_color_widget.widget());
    page.append(&y_label_color_box);

    let config_clone = config.clone();
    let on_change_clone = on_change.clone();
    y_axis_label_color_widget.set_on_change(move |color| {
        config_clone.borrow_mut().y_axis.label_color = color;
        notify_change_static(&on_change_clone);
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
    let on_change_clone = on_change.clone();
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
                notify_change_static(&on_change_clone);
            }
        }
    });
    y_label_font_box.append(&y_paste_font_btn);

    page.append(&y_label_font_box);

    // Font size spinner change handler
    let config_clone = config.clone();
    let y_font_button_clone = y_axis_label_font_button.clone();
    let on_change_clone = on_change.clone();
    y_axis_label_font_size_spin.connect_value_changed(move |spin| {
        let new_size = spin.value();
        config_clone.borrow_mut().y_axis.label_font_size = new_size;
        let family = config_clone.borrow().y_axis.label_font_family.clone();
        y_font_button_clone.set_label(&format!("{} {:.0}", family, new_size));
        notify_change_static(&on_change_clone);
    });

    // Bold checkbox handler
    let config_clone = config.clone();
    let on_change_clone = on_change.clone();
    y_axis_label_bold_check.connect_toggled(move |check| {
        config_clone.borrow_mut().y_axis.label_bold = check.is_active();
        notify_change_static(&on_change_clone);
    });

    // Italic checkbox handler
    let config_clone = config.clone();
    let on_change_clone = on_change.clone();
    y_axis_label_italic_check.connect_toggled(move |check| {
        config_clone.borrow_mut().y_axis.label_italic = check.is_active();
        notify_change_static(&on_change_clone);
    });

    // Font button click handler - opens font dialog
    let config_clone = config.clone();
    let y_font_button_clone = y_axis_label_font_button.clone();
    let y_size_spin_clone = y_axis_label_font_size_spin.clone();
    let on_change_clone = on_change.clone();
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
        let on_change_clone2 = on_change_clone.clone();

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
                    notify_change_static(&on_change_clone2);
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
    let on_change_clone = on_change.clone();
    x_axis_show_check.connect_toggled(move |check| {
        config_clone.borrow_mut().x_axis.show = check.is_active();
        notify_change_static(&on_change_clone);
    });

    let x_axis_show_grid_check = CheckButton::with_label("Show X-Axis Grid");
    x_axis_show_grid_check.set_active(true);
    page.append(&x_axis_show_grid_check);

    let config_clone = config.clone();
    let on_change_clone = on_change.clone();
    x_axis_show_grid_check.connect_toggled(move |check| {
        config_clone.borrow_mut().x_axis.show_grid = check.is_active();
        notify_change_static(&on_change_clone);
    });

    // X-Axis color - using ColorButtonWidget
    let x_color_box = GtkBox::new(Orientation::Horizontal, 6);
    x_color_box.append(&Label::new(Some("X-Axis Color:")));
    let x_axis_color_widget = Rc::new(ColorButtonWidget::new(config.borrow().x_axis.color));
    x_color_box.append(x_axis_color_widget.widget());
    page.append(&x_color_box);

    let config_clone = config.clone();
    let on_change_clone = on_change.clone();
    x_axis_color_widget.set_on_change(move |color| {
        config_clone.borrow_mut().x_axis.color = color;
        notify_change_static(&on_change_clone);
    });

    // X-Grid color - using ColorButtonWidget
    let x_grid_color_box = GtkBox::new(Orientation::Horizontal, 6);
    x_grid_color_box.append(&Label::new(Some("X-Grid Color:")));
    let x_axis_grid_color_widget = Rc::new(ColorButtonWidget::new(config.borrow().x_axis.grid_color));
    x_grid_color_box.append(x_axis_grid_color_widget.widget());
    page.append(&x_grid_color_box);

    let config_clone = config.clone();
    let on_change_clone = on_change.clone();
    x_axis_grid_color_widget.set_on_change(move |color| {
        config_clone.borrow_mut().x_axis.grid_color = color;
        notify_change_static(&on_change_clone);
    });

    // X-Axis label color - using ColorButtonWidget
    let x_label_color_box = GtkBox::new(Orientation::Horizontal, 6);
    x_label_color_box.append(&Label::new(Some("Label Color:")));
    let x_axis_label_color_widget = Rc::new(ColorButtonWidget::new(config.borrow().x_axis.label_color));
    x_label_color_box.append(x_axis_label_color_widget.widget());
    page.append(&x_label_color_box);

    let config_clone = config.clone();
    let on_change_clone = on_change.clone();
    x_axis_label_color_widget.set_on_change(move |color| {
        config_clone.borrow_mut().x_axis.label_color = color;
        notify_change_static(&on_change_clone);
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
    let on_change_clone = on_change.clone();
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
                notify_change_static(&on_change_clone);
            }
        }
    });
    x_label_font_box.append(&x_paste_font_btn);

    page.append(&x_label_font_box);

    // Font size spinner change handler
    let config_clone = config.clone();
    let x_font_button_clone = x_axis_label_font_button.clone();
    let on_change_clone = on_change.clone();
    x_axis_label_font_size_spin.connect_value_changed(move |spin| {
        let new_size = spin.value();
        config_clone.borrow_mut().x_axis.label_font_size = new_size;
        let family = config_clone.borrow().x_axis.label_font_family.clone();
        x_font_button_clone.set_label(&format!("{} {:.0}", family, new_size));
        notify_change_static(&on_change_clone);
    });

    // Bold checkbox handler
    let config_clone = config.clone();
    let on_change_clone = on_change.clone();
    x_axis_label_bold_check.connect_toggled(move |check| {
        config_clone.borrow_mut().x_axis.label_bold = check.is_active();
        notify_change_static(&on_change_clone);
    });

    // Italic checkbox handler
    let config_clone = config.clone();
    let on_change_clone = on_change.clone();
    x_axis_label_italic_check.connect_toggled(move |check| {
        config_clone.borrow_mut().x_axis.label_italic = check.is_active();
        notify_change_static(&on_change_clone);
    });

    // Font button click handler - opens font dialog
    let config_clone = config.clone();
    let x_font_button_clone = x_axis_label_font_button.clone();
    let x_size_spin_clone = x_axis_label_font_size_spin.clone();
    let on_change_clone = on_change.clone();
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
        let on_change_clone2 = on_change_clone.clone();

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
                    notify_change_static(&on_change_clone2);
                }
            },
        );
    });

    AxesPageWidgets {
        widget: page,
        y_axis_show_check,
        y_axis_show_labels_check,
        y_axis_show_grid_check,
        y_axis_color_widget,
        y_axis_grid_color_widget,
        y_axis_label_color_widget,
        y_axis_label_font_button,
        y_axis_label_font_size_spin,
        y_axis_label_bold_check,
        y_axis_label_italic_check,
        x_axis_show_check,
        x_axis_show_grid_check,
        x_axis_color_widget,
        x_axis_grid_color_widget,
        x_axis_label_color_widget,
        x_axis_label_font_button,
        x_axis_label_font_size_spin,
        x_axis_label_bold_check,
        x_axis_label_italic_check,
    }
}

fn create_layout_page(config: Rc<RefCell<GraphDisplayConfig>>, on_change: OnChangeCallback) -> LayoutPageWidgets {
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
    let on_change_clone = on_change.clone();
    margin_top_spin.connect_value_changed(move |spin| {
        config_clone.borrow_mut().margin.top = spin.value();
        notify_change_static(&on_change_clone);
    });

    let margin_right_spin = SpinButton::with_range(0.0, 100.0, 1.0);
    margin_right_spin.set_value(10.0);
    let right_box = GtkBox::new(Orientation::Horizontal, 6);
    right_box.append(&Label::new(Some("Right:")));
    right_box.append(&margin_right_spin);
    page.append(&right_box);

    let config_clone = config.clone();
    let on_change_clone = on_change.clone();
    margin_right_spin.connect_value_changed(move |spin| {
        config_clone.borrow_mut().margin.right = spin.value();
        notify_change_static(&on_change_clone);
    });

    let margin_bottom_spin = SpinButton::with_range(0.0, 100.0, 1.0);
    margin_bottom_spin.set_value(30.0);
    let bottom_box = GtkBox::new(Orientation::Horizontal, 6);
    bottom_box.append(&Label::new(Some("Bottom:")));
    bottom_box.append(&margin_bottom_spin);
    page.append(&bottom_box);

    let config_clone = config.clone();
    let on_change_clone = on_change.clone();
    margin_bottom_spin.connect_value_changed(move |spin| {
        config_clone.borrow_mut().margin.bottom = spin.value();
        notify_change_static(&on_change_clone);
    });

    let margin_left_spin = SpinButton::with_range(0.0, 100.0, 1.0);
    margin_left_spin.set_value(50.0);
    let left_box = GtkBox::new(Orientation::Horizontal, 6);
    left_box.append(&Label::new(Some("Left:")));
    left_box.append(&margin_left_spin);
    page.append(&left_box);

    let config_clone = config.clone();
    let on_change_clone = on_change.clone();
    margin_left_spin.connect_value_changed(move |spin| {
        config_clone.borrow_mut().margin.left = spin.value();
        notify_change_static(&on_change_clone);
    });

    // Background colors
    page.append(&Label::new(Some("\nColors:")));

    // Background color - using ColorButtonWidget
    let bg_color_box = GtkBox::new(Orientation::Horizontal, 6);
    bg_color_box.append(&Label::new(Some("Background:")));
    let background_color_widget = Rc::new(ColorButtonWidget::new(config.borrow().background_color));
    bg_color_box.append(background_color_widget.widget());
    page.append(&bg_color_box);

    let config_clone = config.clone();
    let on_change_clone = on_change.clone();
    background_color_widget.set_on_change(move |color| {
        config_clone.borrow_mut().background_color = color;
        notify_change_static(&on_change_clone);
    });

    // Plot background color - using ColorButtonWidget
    let plot_bg_color_box = GtkBox::new(Orientation::Horizontal, 6);
    plot_bg_color_box.append(&Label::new(Some("Plot Background:")));
    let plot_background_color_widget = Rc::new(ColorButtonWidget::new(config.borrow().plot_background_color));
    plot_bg_color_box.append(plot_background_color_widget.widget());
    page.append(&plot_bg_color_box);

    let config_clone = config.clone();
    let on_change_clone = on_change.clone();
    plot_background_color_widget.set_on_change(move |color| {
        config_clone.borrow_mut().plot_background_color = color;
        notify_change_static(&on_change_clone);
    });

    LayoutPageWidgets {
        widget: page,
        margin_top_spin,
        margin_right_spin,
        margin_bottom_spin,
        margin_left_spin,
        background_color_widget,
        plot_background_color_widget,
    }
}

fn create_text_overlay_page(_config: Rc<RefCell<GraphDisplayConfig>>, available_fields: Vec<crate::core::FieldMetadata>, on_change: OnChangeCallback) -> TextOverlayPageWidgets {
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

    // Connect the text widget's on_change to propagate changes up
    let on_change_clone = on_change.clone();
    text_widget.set_on_change(move || {
        log::info!("=== GraphConfigWidget: text_widget on_change triggered ===");
        if let Some(ref callback) = *on_change_clone.borrow() {
            log::info!("    Parent callback exists, calling it");
            callback();
        } else {
            log::warn!("    Parent callback is None - changes won't be saved!");
        }
    });

    page.append(text_widget.widget());

    text_config_widgets.push(text_widget);

    TextOverlayPageWidgets {
        widget: page,
        text_config_widgets,
    }
}
