//! Background configuration widget

use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, DropDown, DrawingArea, Entry, Label, Orientation, Scale, SpinButton, Stack, StringList};
use std::cell::RefCell;
use std::rc::Rc;

use crate::ui::background::{BackgroundConfig, BackgroundType, Color, ImageDisplayMode, LinearGradientConfig, RadialGradientConfig, PolygonConfig};
use crate::ui::color_button_widget::ColorButtonWidget;
use crate::ui::GradientEditor;

/// Background configuration widget
pub struct BackgroundConfigWidget {
    container: GtkBox,
    config: Rc<RefCell<BackgroundConfig>>,
    preview: DrawingArea,
    config_stack: Stack,
    type_dropdown: DropDown,
    on_change: Rc<RefCell<Option<std::boxed::Box<dyn Fn()>>>>,
    type_dropdown_handler_id: gtk4::glib::SignalHandlerId,
    linear_gradient_editor: Rc<GradientEditor>,
    radial_gradient_editor: Rc<GradientEditor>,
}

impl BackgroundConfigWidget {
    pub fn new() -> Self {
        let container = GtkBox::new(Orientation::Vertical, 12);
        container.set_margin_start(12);
        container.set_margin_end(12);
        container.set_margin_top(12);
        container.set_margin_bottom(12);

        let config = Rc::new(RefCell::new(BackgroundConfig::default()));
        let on_change = Rc::new(RefCell::new(None));

        // Type selector
        let type_box = GtkBox::new(Orientation::Horizontal, 6);
        type_box.append(&Label::new(Some("Background Type:")));

        let type_options = StringList::new(&[
            "Solid Color",
            "Linear Gradient",
            "Radial Gradient",
            "Image",
            "Tessellated Polygons",
        ]);
        let type_dropdown = DropDown::new(Some(type_options), Option::<gtk4::Expression>::None);
        type_dropdown.set_selected(0); // Default to Solid Color

        type_box.append(&type_dropdown);
        container.append(&type_box);

        // Preview
        let preview = DrawingArea::new();
        preview.set_content_height(150);
        preview.set_vexpand(false);

        let config_clone = config.clone();
        preview.set_draw_func(move |_, cr, width, height| {
            use crate::ui::background::render_background;

            // Render checkerboard pattern to show transparency
            Self::render_checkerboard(cr, width as f64, height as f64);

            let cfg = config_clone.borrow();
            let _ = render_background(cr, &cfg, width as f64, height as f64);
        });

        container.append(&preview);

        // Configuration stack (different UI for each type)
        let config_stack = Stack::new();
        config_stack.set_vexpand(true);

        // Solid color configuration
        let solid_page = Self::create_solid_config(&config, &preview, &on_change);
        config_stack.add_named(&solid_page, Some("solid"));

        // Linear gradient configuration
        let (linear_page, linear_gradient_editor) = Self::create_linear_gradient_config(&config, &preview, &on_change);
        config_stack.add_named(&linear_page, Some("linear_gradient"));

        // Radial gradient configuration
        let (radial_page, radial_gradient_editor) = Self::create_radial_gradient_config(&config, &preview, &on_change);
        config_stack.add_named(&radial_page, Some("radial_gradient"));

        // Image configuration
        let image_page = Self::create_image_config(&config, &preview, &on_change);
        config_stack.add_named(&image_page, Some("image"));

        // Polygon configuration
        let polygon_page = Self::create_polygon_config(&config, &preview, &on_change);
        config_stack.add_named(&polygon_page, Some("polygons"));

        container.append(&config_stack);

        // Connect type selector
        let config_clone = config.clone();
        let stack_clone = config_stack.clone();
        let preview_clone = preview.clone();
        let on_change_clone = on_change.clone();

        let type_dropdown_handler_id = type_dropdown.connect_selected_notify(move |dropdown| {
            let selected = dropdown.selected();
            let page_name = match selected {
                0 => "solid",
                1 => "linear_gradient",
                2 => "radial_gradient",
                3 => "image",
                4 => "polygons",
                _ => "solid",
            };

            stack_clone.set_visible_child_name(page_name);

            // Check if the type actually changed before resetting to defaults
            // This prevents losing customizations when the dropdown is refreshed
            let current_type_index = {
                let cfg = config_clone.borrow();
                match &cfg.background {
                    BackgroundType::Solid { .. } => 0,
                    BackgroundType::LinearGradient(_) => 1,
                    BackgroundType::RadialGradient(_) => 2,
                    BackgroundType::Image { .. } => 3,
                    BackgroundType::Polygons(_) => 4,
                }
            };

            // Only reset to defaults if the type actually changed
            if selected != current_type_index {
                let background_type = match selected {
                    0 => BackgroundType::Solid {
                        color: Color::new(0.15, 0.15, 0.15, 1.0),
                    },
                    1 => BackgroundType::LinearGradient(LinearGradientConfig::default()),
                    2 => BackgroundType::RadialGradient(RadialGradientConfig::default()),
                    3 => BackgroundType::Image {
                        path: String::new(),
                        display_mode: ImageDisplayMode::Fit,
                        alpha: 1.0,
                    },
                    4 => BackgroundType::Polygons(PolygonConfig::default()),
                    _ => BackgroundType::default(),
                };

                let mut cfg = config_clone.borrow_mut();
                cfg.background = background_type;
                drop(cfg);

                if let Some(callback) = on_change_clone.borrow().as_ref() {
                    callback();
                }
            }

            preview_clone.queue_draw();
        });

        Self {
            container,
            config,
            preview,
            config_stack,
            type_dropdown,
            on_change,
            type_dropdown_handler_id,
            linear_gradient_editor,
            radial_gradient_editor,
        }
    }

    /// Render a checkerboard pattern to show transparency
    fn render_checkerboard(cr: &gtk4::cairo::Context, width: f64, height: f64) {
        let square_size = 10.0;
        let light_gray = 0.8;
        let dark_gray = 0.6;

        for y in 0..((height / square_size).ceil() as i32) {
            for x in 0..((width / square_size).ceil() as i32) {
                let is_light = (x + y) % 2 == 0;
                let gray = if is_light { light_gray } else { dark_gray };

                cr.set_source_rgb(gray, gray, gray);
                cr.rectangle(
                    x as f64 * square_size,
                    y as f64 * square_size,
                    square_size,
                    square_size,
                );
                let _ = cr.fill();
            }
        }
    }

    fn create_solid_config(
        config: &Rc<RefCell<BackgroundConfig>>,
        preview: &DrawingArea,
        on_change: &Rc<RefCell<Option<std::boxed::Box<dyn Fn()>>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 6);

        // Solid color - using ColorButtonWidget
        let color_box = GtkBox::new(Orientation::Horizontal, 6);
        color_box.append(&Label::new(Some("Color:")));

        let initial_color = if let BackgroundType::Solid { color } = config.borrow().background {
            color
        } else {
            Color::default()
        };
        let color_widget = ColorButtonWidget::new(initial_color);
        color_box.append(color_widget.widget());
        page.append(&color_box);

        let config_clone = config.clone();
        let preview_clone = preview.clone();
        let on_change_clone = on_change.clone();
        color_widget.set_on_change(move |new_color| {
            config_clone.borrow_mut().background = BackgroundType::Solid { color: new_color };
            preview_clone.queue_draw();
            if let Some(callback) = on_change_clone.borrow().as_ref() {
                callback();
            }
        });

        page
    }

    fn create_linear_gradient_config(
        config: &Rc<RefCell<BackgroundConfig>>,
        preview: &DrawingArea,
        on_change: &Rc<RefCell<Option<std::boxed::Box<dyn Fn()>>>>,
    ) -> (GtkBox, Rc<GradientEditor>) {
        let page = GtkBox::new(Orientation::Vertical, 12);

        // Create gradient editor first so we can reference it in paste handler
        let gradient_editor = GradientEditor::new();

        // Initialize with current config
        if let BackgroundType::LinearGradient(ref grad) = config.borrow().background {
            gradient_editor.set_gradient(grad);
        }

        let gradient_editor_ref = Rc::new(gradient_editor);

        // Copy/Paste gradient buttons
        let copy_paste_box = GtkBox::new(Orientation::Horizontal, 6);
        let copy_gradient_btn = Button::with_label("Copy Gradient");
        let paste_gradient_btn = Button::with_label("Paste Gradient");

        let config_for_copy = config.clone();
        copy_gradient_btn.connect_clicked(move |_| {
            use crate::ui::CLIPBOARD;

            let cfg = config_for_copy.borrow();
            if let BackgroundType::LinearGradient(ref grad) = cfg.background {
                if let Ok(mut clipboard) = CLIPBOARD.lock() {
                    clipboard.copy_gradient_stops(grad.stops.clone());
                    log::info!("Gradient color stops copied to clipboard");
                }
            }
        });

        let config_for_paste = config.clone();
        let preview_for_paste = preview.clone();
        let on_change_for_paste = on_change.clone();
        let gradient_editor_for_paste = gradient_editor_ref.clone();
        paste_gradient_btn.connect_clicked(move |_| {
            use crate::ui::CLIPBOARD;

            if let Ok(clipboard) = CLIPBOARD.lock() {
                if let Some(stops) = clipboard.paste_gradient_stops() {
                    let mut cfg = config_for_paste.borrow_mut();
                    if let BackgroundType::LinearGradient(ref mut grad) = cfg.background {
                        grad.stops = stops.clone();
                        drop(cfg);

                        // Update the gradient editor widget to reflect pasted stops
                        gradient_editor_for_paste.set_stops(stops);

                        preview_for_paste.queue_draw();

                        if let Some(callback) = on_change_for_paste.borrow().as_ref() {
                            callback();
                        }

                        log::info!("Gradient color stops pasted from clipboard");
                    }
                } else {
                    log::info!("No gradient color stops in clipboard");
                }
            }
        });

        copy_paste_box.append(&copy_gradient_btn);
        copy_paste_box.append(&paste_gradient_btn);
        page.append(&copy_paste_box);

        // Set up change handler
        let config_clone = config.clone();
        let preview_clone = preview.clone();
        let on_change_clone = on_change.clone();
        let gradient_editor_clone = gradient_editor_ref.clone();

        gradient_editor_ref.set_on_change(move || {
            let grad_config = gradient_editor_clone.get_gradient();
            let mut cfg = config_clone.borrow_mut();
            cfg.background = BackgroundType::LinearGradient(grad_config);
            drop(cfg);
            preview_clone.queue_draw();

            if let Some(callback) = on_change_clone.borrow().as_ref() {
                callback();
            }
        });

        page.append(gradient_editor_ref.widget());
        (page, gradient_editor_ref.clone())
    }

    fn create_radial_gradient_config(
        config: &Rc<RefCell<BackgroundConfig>>,
        preview: &DrawingArea,
        on_change: &Rc<RefCell<Option<std::boxed::Box<dyn Fn()>>>>,
    ) -> (GtkBox, Rc<GradientEditor>) {
        let page = GtkBox::new(Orientation::Vertical, 12);

        // Create gradient editor first so we can reference it in paste handler
        let gradient_editor = GradientEditor::new_without_angle();

        // Initialize with current config
        if let BackgroundType::RadialGradient(ref grad) = config.borrow().background {
            gradient_editor.set_stops(grad.stops.clone());
        }

        let gradient_editor_ref = Rc::new(gradient_editor);

        // Copy/Paste gradient buttons
        let copy_paste_box = GtkBox::new(Orientation::Horizontal, 6);
        let copy_gradient_btn = Button::with_label("Copy Gradient");
        let paste_gradient_btn = Button::with_label("Paste Gradient");

        let config_for_copy = config.clone();
        copy_gradient_btn.connect_clicked(move |_| {
            use crate::ui::CLIPBOARD;

            let cfg = config_for_copy.borrow();
            if let BackgroundType::RadialGradient(ref grad) = cfg.background {
                if let Ok(mut clipboard) = CLIPBOARD.lock() {
                    clipboard.copy_gradient_stops(grad.stops.clone());
                    log::info!("Gradient color stops copied to clipboard");
                }
            }
        });

        let config_for_paste = config.clone();
        let preview_for_paste = preview.clone();
        let on_change_for_paste = on_change.clone();
        let gradient_editor_for_paste = gradient_editor_ref.clone();
        paste_gradient_btn.connect_clicked(move |_| {
            use crate::ui::CLIPBOARD;

            if let Ok(clipboard) = CLIPBOARD.lock() {
                if let Some(stops) = clipboard.paste_gradient_stops() {
                    let mut cfg = config_for_paste.borrow_mut();
                    if let BackgroundType::RadialGradient(ref mut grad) = cfg.background {
                        grad.stops = stops.clone();
                        drop(cfg);

                        // Update the gradient editor widget to reflect pasted stops
                        gradient_editor_for_paste.set_stops(stops);

                        preview_for_paste.queue_draw();

                        if let Some(callback) = on_change_for_paste.borrow().as_ref() {
                            callback();
                        }

                        log::info!("Gradient color stops pasted from clipboard");
                    }
                } else {
                    log::info!("No gradient color stops in clipboard");
                }
            }
        });

        copy_paste_box.append(&copy_gradient_btn);
        copy_paste_box.append(&paste_gradient_btn);
        page.append(&copy_paste_box);

        // Radius control
        let radius_box = GtkBox::new(Orientation::Horizontal, 6);
        radius_box.append(&Label::new(Some("Radius:")));

        let radius_scale = Scale::with_range(Orientation::Horizontal, 0.1, 1.5, 0.05);
        radius_scale.set_hexpand(true);
        radius_scale.set_value(0.7);

        let config_clone = config.clone();
        let preview_clone = preview.clone();
        let on_change_clone = on_change.clone();

        radius_scale.connect_value_changed(move |scale| {
            let mut cfg = config_clone.borrow_mut();
            if let BackgroundType::RadialGradient(ref mut grad) = cfg.background {
                grad.radius = scale.value();
                drop(cfg);
                preview_clone.queue_draw();

                if let Some(callback) = on_change_clone.borrow().as_ref() {
                    callback();
                }
            }
        });

        radius_box.append(&radius_scale);
        page.append(&radius_box);

        // Set up change handler
        let config_clone = config.clone();
        let preview_clone = preview.clone();
        let on_change_clone = on_change.clone();
        let gradient_editor_clone = gradient_editor_ref.clone();

        gradient_editor_ref.set_on_change(move || {
            let stops = gradient_editor_clone.get_stops();
            let mut cfg = config_clone.borrow_mut();
            if let BackgroundType::RadialGradient(ref mut grad) = cfg.background {
                grad.stops = stops;
            }
            drop(cfg);
            preview_clone.queue_draw();

            if let Some(callback) = on_change_clone.borrow().as_ref() {
                callback();
            }
        });

        page.append(gradient_editor_ref.widget());
        (page, gradient_editor_ref.clone())
    }

    fn create_image_config(
        config: &Rc<RefCell<BackgroundConfig>>,
        preview: &DrawingArea,
        on_change: &Rc<RefCell<Option<std::boxed::Box<dyn Fn()>>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 12);

        let path_entry = Entry::new();
        path_entry.set_placeholder_text(Some("Image path"));
        path_entry.set_hexpand(true);

        let browse_button = Button::with_label("Browse...");

        // Display mode selector
        let mode_box = GtkBox::new(Orientation::Horizontal, 6);
        mode_box.append(&Label::new(Some("Display mode:")));

        let mode_options = StringList::new(&["Fit", "Stretch", "Zoom", "Tile"]);
        let mode_dropdown = DropDown::new(Some(mode_options), Option::<gtk4::Expression>::None);
        mode_dropdown.set_selected(0); // Default to Fit
        mode_dropdown.set_hexpand(true);
        mode_box.append(&mode_dropdown);

        // Transparency slider
        let alpha_box = GtkBox::new(Orientation::Horizontal, 6);
        alpha_box.append(&Label::new(Some("Opacity:")));

        let alpha_scale = Scale::with_range(Orientation::Horizontal, 0.0, 1.0, 0.01);
        alpha_scale.set_value(1.0);
        alpha_scale.set_hexpand(true);
        alpha_scale.set_draw_value(true);
        alpha_scale.set_value_pos(gtk4::PositionType::Right);
        alpha_box.append(&alpha_scale);

        let config_clone = config.clone();
        let preview_clone = preview.clone();
        let path_entry_clone = path_entry.clone();
        let on_change_clone = on_change.clone();

        browse_button.connect_clicked(move |btn| {
            let picker = crate::ui::ImagePicker::new("Select Background Image");

            let config_clone2 = config_clone.clone();
            let preview_clone2 = preview_clone.clone();
            let path_entry_clone2 = path_entry_clone.clone();
            let on_change_clone2 = on_change_clone.clone();

            let window = btn.root().and_downcast::<gtk4::Window>();
            picker.pick(window.as_ref(), move |path| {
                if let Some(path) = path {
                    let path_str = path.to_string_lossy().to_string();
                    path_entry_clone2.set_text(&path_str);

                    let mut cfg = config_clone2.borrow_mut();
                    if let BackgroundType::Image { ref mut path, .. } = cfg.background {
                        *path = path_str;
                        drop(cfg);
                        preview_clone2.queue_draw();

                        if let Some(callback) = on_change_clone2.borrow().as_ref() {
                            callback();
                        }
                    }
                }
            });
        });

        // Display mode change handler
        let config_clone = config.clone();
        let preview_clone = preview.clone();
        let on_change_clone = on_change.clone();

        mode_dropdown.connect_selected_notify(move |dropdown| {
            let selected = dropdown.selected();
            let display_mode = match selected {
                0 => ImageDisplayMode::Fit,
                1 => ImageDisplayMode::Stretch,
                2 => ImageDisplayMode::Zoom,
                3 => ImageDisplayMode::Tile,
                _ => ImageDisplayMode::Fit,
            };

            let mut cfg = config_clone.borrow_mut();
            if let BackgroundType::Image { display_mode: ref mut dm, .. } = cfg.background {
                *dm = display_mode;
                drop(cfg);
                preview_clone.queue_draw();

                if let Some(callback) = on_change_clone.borrow().as_ref() {
                    callback();
                }
            }
        });

        // Alpha slider handler
        let config_clone = config.clone();
        let preview_clone = preview.clone();
        let on_change_clone = on_change.clone();

        alpha_scale.connect_value_changed(move |scale| {
            let mut cfg = config_clone.borrow_mut();
            if let BackgroundType::Image { ref mut alpha, .. } = cfg.background {
                *alpha = scale.value();
                drop(cfg);
                preview_clone.queue_draw();

                if let Some(callback) = on_change_clone.borrow().as_ref() {
                    callback();
                }
            }
        });

        page.append(&path_entry);
        page.append(&browse_button);
        page.append(&mode_box);
        page.append(&alpha_box);
        page
    }

    fn create_polygon_config(
        config: &Rc<RefCell<BackgroundConfig>>,
        preview: &DrawingArea,
        on_change: &Rc<RefCell<Option<std::boxed::Box<dyn Fn()>>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 12);

        // Tile size
        let size_box = GtkBox::new(Orientation::Horizontal, 6);
        size_box.append(&Label::new(Some("Tile Size:")));

        let size_spin = SpinButton::with_range(10.0, 200.0, 5.0);
        size_spin.set_value(60.0);
        size_spin.set_hexpand(true);

        let config_clone = config.clone();
        let preview_clone = preview.clone();
        let on_change_clone = on_change.clone();

        size_spin.connect_value_changed(move |spin| {
            let mut cfg = config_clone.borrow_mut();
            if let BackgroundType::Polygons(ref mut poly) = cfg.background {
                poly.tile_size = spin.value() as u32;
                drop(cfg);
                preview_clone.queue_draw();

                if let Some(callback) = on_change_clone.borrow().as_ref() {
                    callback();
                }
            }
        });

        size_box.append(&size_spin);
        page.append(&size_box);

        // Number of sides
        let sides_box = GtkBox::new(Orientation::Horizontal, 6);
        sides_box.append(&Label::new(Some("Sides:")));

        let sides_spin = SpinButton::with_range(3.0, 12.0, 1.0);
        sides_spin.set_value(6.0);
        sides_spin.set_hexpand(true);

        let config_clone = config.clone();
        let preview_clone = preview.clone();
        let on_change_clone = on_change.clone();

        sides_spin.connect_value_changed(move |spin| {
            let mut cfg = config_clone.borrow_mut();
            if let BackgroundType::Polygons(ref mut poly) = cfg.background {
                poly.num_sides = spin.value() as u32;
                drop(cfg);
                preview_clone.queue_draw();

                if let Some(callback) = on_change_clone.borrow().as_ref() {
                    callback();
                }
            }
        });

        sides_box.append(&sides_spin);
        page.append(&sides_box);

        // Rotation angle
        let angle_box = GtkBox::new(Orientation::Horizontal, 6);
        angle_box.append(&Label::new(Some("Rotation:")));

        let angle_scale = Scale::with_range(Orientation::Horizontal, 0.0, 360.0, 5.0);
        angle_scale.set_value(0.0);
        angle_scale.set_hexpand(true);

        let config_clone = config.clone();
        let preview_clone = preview.clone();
        let on_change_clone = on_change.clone();

        angle_scale.connect_value_changed(move |scale| {
            let mut cfg = config_clone.borrow_mut();
            if let BackgroundType::Polygons(ref mut poly) = cfg.background {
                poly.rotation_angle = scale.value();
                drop(cfg);
                preview_clone.queue_draw();

                if let Some(callback) = on_change_clone.borrow().as_ref() {
                    callback();
                }
            }
        });

        angle_box.append(&angle_scale);
        page.append(&angle_box);

        // Color 1 - using ColorButtonWidget
        let color1_box = GtkBox::new(Orientation::Horizontal, 6);
        color1_box.append(&Label::new(Some("Color 1:")));

        let color1 = if let BackgroundType::Polygons(ref poly) = config.borrow().background {
            poly.colors.first().copied().unwrap_or_default()
        } else {
            Color::default()
        };
        let color1_widget = ColorButtonWidget::new(color1);
        color1_box.append(color1_widget.widget());
        page.append(&color1_box);

        let config_clone = config.clone();
        let preview_clone = preview.clone();
        let on_change_clone = on_change.clone();
        color1_widget.set_on_change(move |new_color| {
            let mut cfg = config_clone.borrow_mut();
            if let BackgroundType::Polygons(ref mut poly) = cfg.background {
                if poly.colors.is_empty() {
                    poly.colors.push(new_color);
                } else {
                    poly.colors[0] = new_color;
                }
            }
            drop(cfg);
            preview_clone.queue_draw();
            if let Some(callback) = on_change_clone.borrow().as_ref() {
                callback();
            }
        });

        // Color 2 - using ColorButtonWidget
        let color2_box = GtkBox::new(Orientation::Horizontal, 6);
        color2_box.append(&Label::new(Some("Color 2:")));

        let color2 = if let BackgroundType::Polygons(ref poly) = config.borrow().background {
            poly.colors.get(1).copied().unwrap_or_default()
        } else {
            Color::default()
        };
        let color2_widget = ColorButtonWidget::new(color2);
        color2_box.append(color2_widget.widget());
        page.append(&color2_box);

        let config_clone = config.clone();
        let preview_clone = preview.clone();
        let on_change_clone = on_change.clone();
        color2_widget.set_on_change(move |new_color| {
            let mut cfg = config_clone.borrow_mut();
            if let BackgroundType::Polygons(ref mut poly) = cfg.background {
                if poly.colors.len() < 2 {
                    poly.colors.push(new_color);
                } else {
                    poly.colors[1] = new_color;
                }
            }
            drop(cfg);
            preview_clone.queue_draw();
            if let Some(callback) = on_change_clone.borrow().as_ref() {
                callback();
            }
        });

        page
    }

    /// Get the container widget
    pub fn widget(&self) -> &GtkBox {
        &self.container
    }

    /// Set the background configuration
    pub fn set_config(&self, new_config: BackgroundConfig) {
        // Determine the type index from the config
        let type_index = match &new_config.background {
            BackgroundType::Solid { .. } => 0,
            BackgroundType::LinearGradient(_) => 1,
            BackgroundType::RadialGradient(_) => 2,
            BackgroundType::Image { .. } => 3,
            BackgroundType::Polygons(_) => 4,
        };

        // Load gradient data into editors if applicable
        if let BackgroundType::LinearGradient(ref grad) = new_config.background {
            self.linear_gradient_editor.set_gradient(grad);
        }
        if let BackgroundType::RadialGradient(ref grad) = new_config.background {
            self.radial_gradient_editor.set_stops(grad.stops.clone());
        }

        *self.config.borrow_mut() = new_config;

        // Block the signal handler to prevent it from overwriting our config
        self.type_dropdown.block_signal(&self.type_dropdown_handler_id);

        // Update the dropdown selection (this won't trigger the handler now)
        self.type_dropdown.set_selected(type_index);

        // Unblock the signal handler
        self.type_dropdown.unblock_signal(&self.type_dropdown_handler_id);

        // Update the visible stack page to match the background type
        let page_name = match type_index {
            0 => "solid",
            1 => "linear_gradient",
            2 => "radial_gradient",
            3 => "image",
            4 => "polygons",
            _ => "solid",
        };
        self.config_stack.set_visible_child_name(page_name);

        self.preview.queue_draw();
    }

    /// Get the current configuration
    pub fn get_config(&self) -> BackgroundConfig {
        self.config.borrow().clone()
    }

    /// Set callback for when configuration changes
    pub fn set_on_change<F: Fn() + 'static>(&self, callback: F) {
        *self.on_change.borrow_mut() = Some(std::boxed::Box::new(callback));
    }
}

impl Default for BackgroundConfigWidget {
    fn default() -> Self {
        Self::new()
    }
}
