//! Background configuration widget

use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, ComboBoxText, DrawingArea, Entry, FileChooserAction, FileChooserDialog, Label, Orientation, ResponseType, Scale, SpinButton, Stack, Switch};
use std::cell::RefCell;
use std::rc::Rc;

use crate::ui::background::{BackgroundConfig, BackgroundType, Color, LinearGradientConfig, RadialGradientConfig, PolygonConfig};
use crate::ui::color_picker::ColorPickerDialog;

/// Background configuration widget
pub struct BackgroundConfigWidget {
    container: GtkBox,
    config: Rc<RefCell<BackgroundConfig>>,
    preview: DrawingArea,
    config_stack: Stack,
    type_combo: ComboBoxText,
    on_change: Rc<RefCell<Option<std::boxed::Box<dyn Fn()>>>>,
    type_combo_handler_id: gtk4::glib::SignalHandlerId,
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

        let type_combo = ComboBoxText::new();
        type_combo.append(Some("solid"), "Solid Color");
        type_combo.append(Some("linear_gradient"), "Linear Gradient");
        type_combo.append(Some("radial_gradient"), "Radial Gradient");
        type_combo.append(Some("image"), "Image");
        type_combo.append(Some("polygons"), "Tessellated Polygons");
        type_combo.set_active_id(Some("solid"));

        type_box.append(&type_combo);
        container.append(&type_box);

        // Preview
        let preview = DrawingArea::new();
        preview.set_content_height(150);
        preview.set_vexpand(false);

        let config_clone = config.clone();
        preview.set_draw_func(move |_, cr, width, height| {
            use crate::ui::background::render_background;
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
        let linear_page = Self::create_linear_gradient_config(&config, &preview, &on_change);
        config_stack.add_named(&linear_page, Some("linear_gradient"));

        // Radial gradient configuration
        let radial_page = Self::create_radial_gradient_config(&config, &preview, &on_change);
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

        let type_combo_handler_id = type_combo.connect_changed(move |combo| {
            if let Some(active_id) = combo.active_id() {
                log::info!("type_combo.connect_changed handler fired with active_id: {}", active_id);
                stack_clone.set_visible_child_name(&active_id);

                // Update config type
                let mut cfg = config_clone.borrow_mut();
                log::info!("Creating new default background for type: {}", active_id);
                cfg.background = match active_id.as_str() {
                    "solid" => BackgroundType::Solid {
                        color: Color::new(0.15, 0.15, 0.15, 1.0),
                    },
                    "linear_gradient" => BackgroundType::LinearGradient(LinearGradientConfig::default()),
                    "radial_gradient" => BackgroundType::RadialGradient(RadialGradientConfig::default()),
                    "image" => BackgroundType::Image {
                        path: String::new(),
                        stretch: false,
                    },
                    "polygons" => BackgroundType::Polygons(PolygonConfig::default()),
                    _ => BackgroundType::default(),
                };
                drop(cfg);

                preview_clone.queue_draw();

                if let Some(callback) = on_change_clone.borrow().as_ref() {
                    callback();
                }
            }
        });

        Self {
            container,
            config,
            preview,
            config_stack,
            type_combo,
            on_change,
            type_combo_handler_id,
        }
    }

    fn create_solid_config(
        config: &Rc<RefCell<BackgroundConfig>>,
        preview: &DrawingArea,
        on_change: &Rc<RefCell<Option<std::boxed::Box<dyn Fn()>>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 6);

        let button = Button::with_label("Select Color");

        let config_clone = config.clone();
        let preview_clone = preview.clone();
        let on_change_clone = on_change.clone();

        button.connect_clicked(move |btn| {
            let current_color = if let BackgroundType::Solid { color } = config_clone.borrow().background {
                color
            } else {
                Color::default()
            };

            let window = btn.root().and_then(|root| root.downcast::<gtk4::Window>().ok());
            let config_clone2 = config_clone.clone();
            let preview_clone2 = preview_clone.clone();
            let on_change_clone2 = on_change_clone.clone();

            gtk4::glib::MainContext::default().spawn_local(async move {
                if let Some(new_color) = ColorPickerDialog::pick_color(window.as_ref(), current_color).await {
                    log::info!("User selected solid color: r={}, g={}, b={}, a={}",
                        new_color.r, new_color.g, new_color.b, new_color.a);
                    let mut cfg = config_clone2.borrow_mut();
                    cfg.background = BackgroundType::Solid { color: new_color };
                    drop(cfg);
                    log::info!("Updated config to solid color, verifying: {:?}", config_clone2.borrow().background);

                    preview_clone2.queue_draw();

                    if let Some(callback) = on_change_clone2.borrow().as_ref() {
                        callback();
                    }
                }
            });
        });

        page.append(&button);
        page
    }

    fn create_linear_gradient_config(
        config: &Rc<RefCell<BackgroundConfig>>,
        preview: &DrawingArea,
        on_change: &Rc<RefCell<Option<std::boxed::Box<dyn Fn()>>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 12);

        // Angle control
        let angle_box = GtkBox::new(Orientation::Horizontal, 6);
        angle_box.append(&Label::new(Some("Angle:")));

        let angle_scale = Scale::with_range(Orientation::Horizontal, 0.0, 360.0, 1.0);
        angle_scale.set_hexpand(true);
        angle_scale.set_value(90.0);

        let config_clone = config.clone();
        let preview_clone = preview.clone();
        let on_change_clone = on_change.clone();

        angle_scale.connect_value_changed(move |scale| {
            let mut cfg = config_clone.borrow_mut();
            if let BackgroundType::LinearGradient(ref mut grad) = cfg.background {
                grad.angle = scale.value();
                drop(cfg);
                preview_clone.queue_draw();

                if let Some(callback) = on_change_clone.borrow().as_ref() {
                    callback();
                }
            }
        });

        angle_box.append(&angle_scale);
        page.append(&angle_box);

        // Color stops (simplified - just 2 colors for now)
        let start_button = Button::with_label("Start Color");
        let end_button = Button::with_label("End Color");

        let config_clone = config.clone();
        let preview_clone = preview.clone();
        let on_change_clone = on_change.clone();

        start_button.connect_clicked(move |btn| {
            let current_color = if let BackgroundType::LinearGradient(ref grad) = config_clone.borrow().background {
                grad.stops.first().map(|s| s.color).unwrap_or_default()
            } else {
                Color::default()
            };

            let window = btn.root().and_then(|root| root.downcast::<gtk4::Window>().ok());
            let config_clone2 = config_clone.clone();
            let preview_clone2 = preview_clone.clone();
            let on_change_clone2 = on_change_clone.clone();

            gtk4::glib::MainContext::default().spawn_local(async move {
                if let Some(new_color) = ColorPickerDialog::pick_color(window.as_ref(), current_color).await {
                    let mut cfg = config_clone2.borrow_mut();
                    if let BackgroundType::LinearGradient(ref mut grad) = cfg.background {
                        if let Some(stop) = grad.stops.first_mut() {
                            stop.color = new_color;
                        }
                        drop(cfg);
                        preview_clone2.queue_draw();

                        if let Some(callback) = on_change_clone2.borrow().as_ref() {
                            callback();
                        }
                    }
                }
            });
        });

        let config_clone = config.clone();
        let preview_clone = preview.clone();
        let on_change_clone = on_change.clone();

        end_button.connect_clicked(move |btn| {
            let current_color = if let BackgroundType::LinearGradient(ref grad) = config_clone.borrow().background {
                grad.stops.last().map(|s| s.color).unwrap_or_default()
            } else {
                Color::default()
            };

            let window = btn.root().and_then(|root| root.downcast::<gtk4::Window>().ok());
            let config_clone2 = config_clone.clone();
            let preview_clone2 = preview_clone.clone();
            let on_change_clone2 = on_change_clone.clone();

            gtk4::glib::MainContext::default().spawn_local(async move {
                if let Some(new_color) = ColorPickerDialog::pick_color(window.as_ref(), current_color).await {
                    let mut cfg = config_clone2.borrow_mut();
                    if let BackgroundType::LinearGradient(ref mut grad) = cfg.background {
                        if let Some(stop) = grad.stops.last_mut() {
                            stop.color = new_color;
                        }
                        drop(cfg);
                        preview_clone2.queue_draw();

                        if let Some(callback) = on_change_clone2.borrow().as_ref() {
                            callback();
                        }
                    }
                }
            });
        });

        page.append(&start_button);
        page.append(&end_button);
        page
    }

    fn create_radial_gradient_config(
        config: &Rc<RefCell<BackgroundConfig>>,
        preview: &DrawingArea,
        on_change: &Rc<RefCell<Option<std::boxed::Box<dyn Fn()>>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 12);

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

        // Color stops (simplified)
        let center_button = Button::with_label("Center Color");
        let edge_button = Button::with_label("Edge Color");

        let config_clone = config.clone();
        let preview_clone = preview.clone();
        let on_change_clone = on_change.clone();

        center_button.connect_clicked(move |btn| {
            let current_color = if let BackgroundType::RadialGradient(ref grad) = config_clone.borrow().background {
                grad.stops.first().map(|s| s.color).unwrap_or_default()
            } else {
                Color::default()
            };

            let window = btn.root().and_then(|root| root.downcast::<gtk4::Window>().ok());
            let config_clone2 = config_clone.clone();
            let preview_clone2 = preview_clone.clone();
            let on_change_clone2 = on_change_clone.clone();

            gtk4::glib::MainContext::default().spawn_local(async move {
                if let Some(new_color) = ColorPickerDialog::pick_color(window.as_ref(), current_color).await {
                    let mut cfg = config_clone2.borrow_mut();
                    if let BackgroundType::RadialGradient(ref mut grad) = cfg.background {
                        if let Some(stop) = grad.stops.first_mut() {
                            stop.color = new_color;
                        }
                        drop(cfg);
                        preview_clone2.queue_draw();

                        if let Some(callback) = on_change_clone2.borrow().as_ref() {
                            callback();
                        }
                    }
                }
            });
        });

        let config_clone = config.clone();
        let preview_clone = preview.clone();
        let on_change_clone = on_change.clone();

        edge_button.connect_clicked(move |btn| {
            let current_color = if let BackgroundType::RadialGradient(ref grad) = config_clone.borrow().background {
                grad.stops.last().map(|s| s.color).unwrap_or_default()
            } else {
                Color::default()
            };

            let window = btn.root().and_then(|root| root.downcast::<gtk4::Window>().ok());
            let config_clone2 = config_clone.clone();
            let preview_clone2 = preview_clone.clone();
            let on_change_clone2 = on_change_clone.clone();

            gtk4::glib::MainContext::default().spawn_local(async move {
                if let Some(new_color) = ColorPickerDialog::pick_color(window.as_ref(), current_color).await {
                    let mut cfg = config_clone2.borrow_mut();
                    if let BackgroundType::RadialGradient(ref mut grad) = cfg.background {
                        if let Some(stop) = grad.stops.last_mut() {
                            stop.color = new_color;
                        }
                        drop(cfg);
                        preview_clone2.queue_draw();

                        if let Some(callback) = on_change_clone2.borrow().as_ref() {
                            callback();
                        }
                    }
                }
            });
        });

        page.append(&center_button);
        page.append(&edge_button);
        page
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

        let stretch_box = GtkBox::new(Orientation::Horizontal, 6);
        stretch_box.append(&Label::new(Some("Stretch:")));
        let stretch_switch = Switch::new();
        stretch_box.append(&stretch_switch);

        let config_clone = config.clone();
        let preview_clone = preview.clone();
        let path_entry_clone = path_entry.clone();
        let on_change_clone = on_change.clone();

        browse_button.connect_clicked(move |btn| {
            let dialog = FileChooserDialog::new(
                Some("Select Image"),
                btn.root().and_downcast_ref::<gtk4::Window>(),
                FileChooserAction::Open,
                &[("Cancel", ResponseType::Cancel), ("Open", ResponseType::Accept)],
            );

            let config_clone2 = config_clone.clone();
            let preview_clone2 = preview_clone.clone();
            let path_entry_clone2 = path_entry_clone.clone();
            let on_change_clone2 = on_change_clone.clone();

            dialog.connect_response(move |dialog, response| {
                if response == ResponseType::Accept {
                    if let Some(file) = dialog.file() {
                        if let Some(path) = file.path() {
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
                    }
                }
                dialog.close();
            });

            dialog.show();
        });

        let config_clone = config.clone();
        let preview_clone = preview.clone();
        let on_change_clone = on_change.clone();

        stretch_switch.connect_state_set(move |_, state| {
            let mut cfg = config_clone.borrow_mut();
            if let BackgroundType::Image { ref mut stretch, .. } = cfg.background {
                *stretch = state;
                drop(cfg);
                preview_clone.queue_draw();

                if let Some(callback) = on_change_clone.borrow().as_ref() {
                    callback();
                }
            }
            gtk4::glib::Propagation::Proceed
        });

        page.append(&path_entry);
        page.append(&browse_button);
        page.append(&stretch_box);
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

        // Color buttons
        page.append(&Label::new(Some("Colors:")));

        let color1_button = Button::with_label("Color 1");
        let color2_button = Button::with_label("Color 2");

        let config_clone = config.clone();
        let preview_clone = preview.clone();
        let on_change_clone = on_change.clone();

        color1_button.connect_clicked(move |btn| {
            let current_color = if let BackgroundType::Polygons(ref poly) = config_clone.borrow().background {
                poly.colors.first().copied().unwrap_or_default()
            } else {
                Color::default()
            };

            let window = btn.root().and_then(|root| root.downcast::<gtk4::Window>().ok());
            let config_clone2 = config_clone.clone();
            let preview_clone2 = preview_clone.clone();
            let on_change_clone2 = on_change_clone.clone();

            gtk4::glib::MainContext::default().spawn_local(async move {
                if let Some(new_color) = ColorPickerDialog::pick_color(window.as_ref(), current_color).await {
                    let mut cfg = config_clone2.borrow_mut();
                    if let BackgroundType::Polygons(ref mut poly) = cfg.background {
                        if poly.colors.is_empty() {
                            poly.colors.push(new_color);
                        } else {
                            poly.colors[0] = new_color;
                        }
                        drop(cfg);
                        preview_clone2.queue_draw();

                        if let Some(callback) = on_change_clone2.borrow().as_ref() {
                            callback();
                        }
                    }
                }
            });
        });

        let config_clone = config.clone();
        let preview_clone = preview.clone();
        let on_change_clone = on_change.clone();

        color2_button.connect_clicked(move |btn| {
            let current_color = if let BackgroundType::Polygons(ref poly) = config_clone.borrow().background {
                poly.colors.get(1).copied().unwrap_or_default()
            } else {
                Color::default()
            };

            let window = btn.root().and_then(|root| root.downcast::<gtk4::Window>().ok());
            let config_clone2 = config_clone.clone();
            let preview_clone2 = preview_clone.clone();
            let on_change_clone2 = on_change_clone.clone();

            gtk4::glib::MainContext::default().spawn_local(async move {
                if let Some(new_color) = ColorPickerDialog::pick_color(window.as_ref(), current_color).await {
                    let mut cfg = config_clone2.borrow_mut();
                    if let BackgroundType::Polygons(ref mut poly) = cfg.background {
                        if poly.colors.len() < 2 {
                            poly.colors.push(new_color);
                        } else {
                            poly.colors[1] = new_color;
                        }
                        drop(cfg);
                        preview_clone2.queue_draw();

                        if let Some(callback) = on_change_clone2.borrow().as_ref() {
                            callback();
                        }
                    }
                }
            });
        });

        page.append(&color1_button);
        page.append(&color2_button);

        page
    }

    /// Get the container widget
    pub fn widget(&self) -> &GtkBox {
        &self.container
    }

    /// Set the background configuration
    pub fn set_config(&self, new_config: BackgroundConfig) {
        log::info!("BackgroundConfigWidget::set_config called with: {:?}", new_config);

        // Determine the type ID from the config
        let type_id = match &new_config.background {
            BackgroundType::Solid { .. } => "solid",
            BackgroundType::LinearGradient(_) => "linear_gradient",
            BackgroundType::RadialGradient(_) => "radial_gradient",
            BackgroundType::Image { .. } => "image",
            BackgroundType::Polygons(_) => "polygons",
        };

        *self.config.borrow_mut() = new_config;
        log::info!("Config stored, verifying: {:?}", self.config.borrow().background);

        // Block the signal handler to prevent it from overwriting our config
        self.type_combo.block_signal(&self.type_combo_handler_id);

        // Update the combo box selection (this won't trigger the handler now)
        self.type_combo.set_active_id(Some(type_id));

        // Unblock the signal handler
        self.type_combo.unblock_signal(&self.type_combo_handler_id);

        // Update the visible stack page to match the background type
        self.config_stack.set_visible_child_name(type_id);

        self.preview.queue_draw();
    }

    /// Get the current configuration
    pub fn get_config(&self) -> BackgroundConfig {
        let config = self.config.borrow().clone();
        log::info!("BackgroundConfigWidget::get_config returning: {:?}", config);
        config
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
