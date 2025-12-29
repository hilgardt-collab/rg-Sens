//! Indicator configuration widget

use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, CheckButton, DrawingArea, DropDown, Label, Notebook, Orientation,
    Scale, SpinButton, StringList,
};
use std::cell::RefCell;
use std::rc::Rc;

use crate::displayers::{IndicatorConfig, IndicatorShape, render_indicator};
use crate::displayers::FieldMetadata;
use crate::ui::render_utils::render_checkerboard;
use crate::ui::color_button_widget::ColorButtonWidget;
use crate::ui::gradient_editor::GradientEditor;
use crate::ui::text_line_config_widget::TextLineConfigWidget;

/// Indicator configuration widget
pub struct IndicatorConfigWidget {
    container: GtkBox,
    config: Rc<RefCell<IndicatorConfig>>,
    on_change: Rc<RefCell<Option<Box<dyn Fn()>>>>,
    preview: DrawingArea,

    // Shape controls
    shape_dropdown: DropDown,
    polygon_sides_spin: SpinButton,
    shape_size_scale: Scale,
    rotation_spin: SpinButton,

    // Border controls
    border_width_spin: SpinButton,
    border_color_widget: ColorButtonWidget,

    // Text overlay
    show_text_check: CheckButton,
    text_config_widget: Option<TextLineConfigWidget>,

    // Gradient editor
    gradient_editor: Rc<GradientEditor>,
}

impl IndicatorConfigWidget {
    pub fn new(available_fields: Vec<FieldMetadata>) -> Self {
        let container = GtkBox::new(Orientation::Vertical, 12);
        let config = Rc::new(RefCell::new(IndicatorConfig::default()));
        let on_change: Rc<RefCell<Option<Box<dyn Fn()>>>> = Rc::new(RefCell::new(None));

        // Preview at the top
        let preview = DrawingArea::new();
        preview.set_content_height(200);
        preview.set_hexpand(true);
        preview.set_vexpand(false);

        let config_clone = config.clone();
        preview.set_draw_func(move |_, cr, width, height| {
            render_checkerboard(cr, width as f64, height as f64);
            let cfg = config_clone.borrow();
            let _ = render_indicator(cr, &cfg, 75.0, width as f64, height as f64);
        });

        // Create notebook for tabbed interface
        let notebook = Notebook::new();
        notebook.set_vexpand(true);

        // === Tab 1: Shape ===
        let (shape_page, shape_dropdown, polygon_sides_spin, shape_size_scale, rotation_spin) =
            Self::create_shape_page(&config, &on_change, &preview);
        notebook.append_page(&shape_page, Some(&Label::new(Some("Shape"))));

        // === Tab 2: Colors ===
        let (color_page, gradient_editor) = Self::create_color_page(&config, &on_change, &preview);
        notebook.append_page(&color_page, Some(&Label::new(Some("Colors"))));

        // === Tab 3: Style ===
        let (style_page, border_width_spin, border_color_widget) =
            Self::create_style_page(&config, &on_change, &preview);
        notebook.append_page(&style_page, Some(&Label::new(Some("Style"))));

        // === Tab 4: Text Overlay ===
        let (text_page, show_text_check, text_config_widget) =
            Self::create_text_page(&config, &on_change, available_fields);
        notebook.append_page(&text_page, Some(&Label::new(Some("Text"))));

        container.append(&preview);
        container.append(&notebook);

        Self {
            container,
            config,
            on_change,
            preview,
            shape_dropdown,
            polygon_sides_spin,
            shape_size_scale,
            rotation_spin,
            border_width_spin,
            border_color_widget,
            show_text_check,
            text_config_widget,
            gradient_editor,
        }
    }

    fn create_shape_page(
        config: &Rc<RefCell<IndicatorConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
    ) -> (GtkBox, DropDown, SpinButton, Scale, SpinButton) {
        let page = GtkBox::new(Orientation::Vertical, 12);
        page.set_margin_start(12);
        page.set_margin_end(12);
        page.set_margin_top(12);
        page.set_margin_bottom(12);

        // Shape selection
        let shape_box = GtkBox::new(Orientation::Horizontal, 12);
        shape_box.append(&Label::new(Some("Shape:")));

        let shape_list = StringList::new(&[
            "Fill",
            "Circle",
            "Square",
            "Triangle (3)",
            "Pentagon (5)",
            "Hexagon (6)",
            "Heptagon (7)",
            "Octagon (8)",
        ]);
        let shape_dropdown = DropDown::new(Some(shape_list), gtk4::Expression::NONE);
        shape_dropdown.set_hexpand(true);
        shape_box.append(&shape_dropdown);
        page.append(&shape_box);

        // Polygon sides (for custom polygon)
        let sides_box = GtkBox::new(Orientation::Horizontal, 12);
        sides_box.append(&Label::new(Some("Custom Sides:")));
        let polygon_sides_spin = SpinButton::with_range(3.0, 20.0, 1.0);
        polygon_sides_spin.set_value(6.0);
        sides_box.append(&polygon_sides_spin);
        page.append(&sides_box);

        // Shape size
        let size_box = GtkBox::new(Orientation::Horizontal, 12);
        size_box.append(&Label::new(Some("Size:")));
        let shape_size_scale = Scale::with_range(Orientation::Horizontal, 0.1, 1.0, 0.05);
        shape_size_scale.set_value(0.8);
        shape_size_scale.set_hexpand(true);
        size_box.append(&shape_size_scale);

        let size_label = Label::new(Some("80%"));
        size_box.append(&size_label);
        page.append(&size_box);

        // Rotation
        let rotation_box = GtkBox::new(Orientation::Horizontal, 12);
        rotation_box.append(&Label::new(Some("Rotation:")));
        let rotation_spin = SpinButton::with_range(-360.0, 360.0, 1.0);
        rotation_spin.set_value(0.0);
        rotation_spin.set_digits(0);
        rotation_box.append(&rotation_spin);
        rotation_box.append(&Label::new(Some("°")));
        page.append(&rotation_box);

        // Connect handlers
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let polygon_sides_spin_clone = polygon_sides_spin.clone();
        shape_dropdown.connect_selected_notify(move |dropdown| {
            let mut cfg = config_clone.borrow_mut();
            cfg.shape = match dropdown.selected() {
                0 => IndicatorShape::Fill,
                1 => IndicatorShape::Circle,
                2 => IndicatorShape::Square,
                3 => IndicatorShape::Polygon(3),
                4 => IndicatorShape::Polygon(5),
                5 => IndicatorShape::Polygon(6),
                6 => IndicatorShape::Polygon(7),
                7 => IndicatorShape::Polygon(8),
                _ => IndicatorShape::Polygon(polygon_sides_spin_clone.value() as u32),
            };
            drop(cfg);
            preview_clone.queue_draw();
            if let Some(callback) = on_change_clone.borrow().as_ref() {
                callback();
            }
        });

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        polygon_sides_spin.connect_value_changed(move |spin| {
            let mut cfg = config_clone.borrow_mut();
            if let IndicatorShape::Polygon(_) = cfg.shape {
                cfg.shape = IndicatorShape::Polygon(spin.value() as u32);
            }
            drop(cfg);
            preview_clone.queue_draw();
            if let Some(callback) = on_change_clone.borrow().as_ref() {
                callback();
            }
        });

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let size_label_clone = size_label.clone();
        shape_size_scale.connect_value_changed(move |scale| {
            let value = scale.value();
            config_clone.borrow_mut().shape_size = value;
            size_label_clone.set_text(&format!("{}%", (value * 100.0) as i32));
            preview_clone.queue_draw();
            if let Some(callback) = on_change_clone.borrow().as_ref() {
                callback();
            }
        });

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        rotation_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().rotation_angle = spin.value();
            preview_clone.queue_draw();
            if let Some(callback) = on_change_clone.borrow().as_ref() {
                callback();
            }
        });

        (page, shape_dropdown, polygon_sides_spin, shape_size_scale, rotation_spin)
    }

    fn create_color_page(
        config: &Rc<RefCell<IndicatorConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
    ) -> (GtkBox, Rc<GradientEditor>) {
        use gtk4::Button;
        use crate::ui::clipboard::CLIPBOARD;

        let page = GtkBox::new(Orientation::Vertical, 12);
        page.set_margin_start(12);
        page.set_margin_end(12);
        page.set_margin_top(12);
        page.set_margin_bottom(12);

        // Explanation label
        let label = Label::new(Some("Configure the color gradient for value mapping.\n0% position = minimum value, 100% = maximum value"));
        label.set_halign(gtk4::Align::Start);
        label.set_wrap(true);
        page.append(&label);

        // Copy/Paste buttons
        let button_box = GtkBox::new(Orientation::Horizontal, 6);
        button_box.set_halign(gtk4::Align::End);

        let copy_button = Button::with_label("Copy Gradient");
        let paste_button = Button::with_label("Paste Gradient");
        button_box.append(&copy_button);
        button_box.append(&paste_button);
        page.append(&button_box);

        // Gradient editor with linear preview (for value mapping)
        let gradient_editor = Rc::new(GradientEditor::new_linear_no_angle());
        page.append(gradient_editor.widget());

        // Set default gradient stops
        {
            let cfg = config.borrow();
            gradient_editor.set_stops(cfg.gradient_stops.clone());
        }

        // Connect gradient change handler
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let gradient_editor_clone = gradient_editor.clone();
        gradient_editor.set_on_change(move || {
            let stops = gradient_editor_clone.get_stops();
            config_clone.borrow_mut().gradient_stops = stops;
            preview_clone.queue_draw();
            if let Some(callback) = on_change_clone.borrow().as_ref() {
                callback();
            }
        });

        // Copy button handler - copy stops with angle=0 (ignored here)
        let gradient_editor_copy = gradient_editor.clone();
        copy_button.connect_clicked(move |_| {
            let stops = gradient_editor_copy.get_stops();
            if let Ok(mut clipboard) = CLIPBOARD.lock() {
                clipboard.gradient_stops = Some(stops);
            }
        });

        // Paste button handler - paste only stops, ignore angle
        let gradient_editor_paste = gradient_editor.clone();
        let config_paste = config.clone();
        let on_change_paste = on_change.clone();
        let preview_paste = preview.clone();
        paste_button.connect_clicked(move |_| {
            if let Ok(clipboard) = CLIPBOARD.lock() {
                if let Some(stops) = &clipboard.gradient_stops {
                    gradient_editor_paste.set_stops(stops.clone());
                    config_paste.borrow_mut().gradient_stops = stops.clone();
                    preview_paste.queue_draw();
                    if let Some(callback) = on_change_paste.borrow().as_ref() {
                        callback();
                    }
                }
            }
        });

        (page, gradient_editor)
    }

    fn create_style_page(
        config: &Rc<RefCell<IndicatorConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
    ) -> (GtkBox, SpinButton, ColorButtonWidget) {
        let page = GtkBox::new(Orientation::Vertical, 12);
        page.set_margin_start(12);
        page.set_margin_end(12);
        page.set_margin_top(12);
        page.set_margin_bottom(12);

        // Border width
        let border_box = GtkBox::new(Orientation::Horizontal, 12);
        border_box.append(&Label::new(Some("Border Width:")));
        let border_width_spin = SpinButton::with_range(0.0, 20.0, 0.5);
        border_width_spin.set_value(0.0);
        border_width_spin.set_digits(1);
        border_box.append(&border_width_spin);
        page.append(&border_box);

        // Border color
        let border_color_box = GtkBox::new(Orientation::Horizontal, 12);
        border_color_box.append(&Label::new(Some("Border Color:")));
        let border_color = config.borrow().border_color;
        let border_color_widget = ColorButtonWidget::new(border_color);
        border_color_box.append(border_color_widget.widget());
        page.append(&border_color_box);

        // Connect handlers
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        border_width_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().border_width = spin.value();
            preview_clone.queue_draw();
            if let Some(callback) = on_change_clone.borrow().as_ref() {
                callback();
            }
        });

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        border_color_widget.set_on_change(move |color| {
            config_clone.borrow_mut().border_color = color;
            preview_clone.queue_draw();
            if let Some(callback) = on_change_clone.borrow().as_ref() {
                callback();
            }
        });

        (page, border_width_spin, border_color_widget)
    }

    fn create_text_page(
        config: &Rc<RefCell<IndicatorConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        available_fields: Vec<FieldMetadata>,
    ) -> (GtkBox, CheckButton, Option<TextLineConfigWidget>) {
        let page = GtkBox::new(Orientation::Vertical, 12);
        page.set_margin_start(12);
        page.set_margin_end(12);
        page.set_margin_top(12);
        page.set_margin_bottom(12);

        // Show text overlay checkbox
        let show_text_check = CheckButton::with_label("Show Text Overlay");
        show_text_check.set_active(config.borrow().show_text);
        page.append(&show_text_check);

        // Text configuration widget
        let text_config_widget = if !available_fields.is_empty() {
            let widget = TextLineConfigWidget::new(available_fields);
            widget.set_config(config.borrow().text_config.clone());
            page.append(widget.widget());

            // Connect text config change handler
            let on_change_clone = on_change.clone();
            widget.set_on_change(move || {
                if let Some(callback) = on_change_clone.borrow().as_ref() {
                    callback();
                }
            });

            Some(widget)
        } else {
            None
        };

        // Connect show text handler
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        show_text_check.connect_toggled(move |check| {
            config_clone.borrow_mut().show_text = check.is_active();
            if let Some(callback) = on_change_clone.borrow().as_ref() {
                callback();
            }
        });

        (page, show_text_check, text_config_widget)
    }

    /// Get the container widget
    pub fn widget(&self) -> &GtkBox {
        &self.container
    }

    /// Set the configuration
    pub fn set_config(&self, cfg: &IndicatorConfig) {
        *self.config.borrow_mut() = cfg.clone();

        // Update shape dropdown
        let shape_index = match cfg.shape {
            IndicatorShape::Fill => 0,
            IndicatorShape::Circle => 1,
            IndicatorShape::Square => 2,
            IndicatorShape::Polygon(3) => 3,
            IndicatorShape::Polygon(5) => 4,
            IndicatorShape::Polygon(6) => 5,
            IndicatorShape::Polygon(7) => 6,
            IndicatorShape::Polygon(8) => 7,
            IndicatorShape::Polygon(n) => {
                self.polygon_sides_spin.set_value(n as f64);
                5 // Default to hexagon for non-standard
            }
        };
        self.shape_dropdown.set_selected(shape_index);

        if let IndicatorShape::Polygon(n) = cfg.shape {
            self.polygon_sides_spin.set_value(n as f64);
        }

        self.shape_size_scale.set_value(cfg.shape_size);
        self.rotation_spin.set_value(cfg.rotation_angle);
        self.border_width_spin.set_value(cfg.border_width);
        self.border_color_widget.set_color(cfg.border_color);
        self.show_text_check.set_active(cfg.show_text);

        self.gradient_editor.set_stops(cfg.gradient_stops.clone());

        if let Some(ref widget) = self.text_config_widget {
            widget.set_config(cfg.text_config.clone());
        }

        self.preview.queue_draw();
    }

    /// Get the current configuration
    pub fn get_config(&self) -> IndicatorConfig {
        let mut cfg = self.config.borrow().clone();

        // Get text config from widget if available
        if let Some(ref widget) = self.text_config_widget {
            cfg.text_config = widget.get_config();
        }

        cfg
    }

    /// Set callback for when configuration changes
    pub fn set_on_change<F: Fn() + 'static>(&self, callback: F) {
        *self.on_change.borrow_mut() = Some(Box::new(callback));
    }

    /// Update preview
    pub fn update_preview(&self) {
        self.preview.queue_draw();
    }
}
