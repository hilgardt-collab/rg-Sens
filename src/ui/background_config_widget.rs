//! Background configuration widget

use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, DropDown, Entry, Label, Orientation, Scale, SizeGroup, SizeGroupMode,
    SpinButton, Stack, StringList,
};
use std::cell::RefCell;
use std::rc::Rc;

use crate::core::FieldMetadata;
use crate::ui::background::{
    BackgroundConfig, BackgroundType, Color, ImageDisplayMode, IndicatorBackgroundConfig,
    LinearGradientConfig, PolygonConfig, RadialGradientConfig,
};
use crate::ui::render_utils::render_checkerboard;
use crate::ui::theme::{ColorSource, ComboThemeConfig};
use crate::ui::theme_color_selector::ThemeColorSelector;
use crate::ui::GradientEditor;

/// Background configuration widget
pub struct BackgroundConfigWidget {
    container: GtkBox,
    config: Rc<RefCell<BackgroundConfig>>,
    config_stack: Stack,
    type_dropdown: DropDown,
    on_change: Rc<RefCell<Option<std::boxed::Box<dyn Fn()>>>>,
    type_dropdown_handler_id: gtk4::glib::SignalHandlerId,
    solid_color_selector: Rc<ThemeColorSelector>,
    linear_gradient_editor: Rc<GradientEditor>,
    radial_gradient_editor: Rc<GradientEditor>,
    indicator_gradient_editor: Rc<GradientEditor>,
    // Source fields for indicator configuration
    source_fields: Rc<RefCell<Vec<FieldMetadata>>>,
    is_combo_source: Rc<RefCell<bool>>,
    indicator_field_dropdown: DropDown,
    indicator_field_list: StringList,
    indicator_field_entry: Entry,
    indicator_field_dropdown_box: GtkBox,
    indicator_field_entry_box: GtkBox,
    // Flag to prevent dropdown handler from overwriting value_field during sync
    syncing_indicator_dropdown: Rc<RefCell<bool>>,
    // Theme config for polygon colors
    theme_config: Rc<RefCell<ComboThemeConfig>>,
    // Polygon color selectors (theme-aware)
    polygon_color1_selector: Rc<ThemeColorSelector>,
    polygon_color2_selector: Rc<ThemeColorSelector>,
    polygon_bg_color_selector: Rc<ThemeColorSelector>,
    // Function to update the preview (using Picture instead of DrawingArea to avoid GL issues)
    update_preview: Rc<dyn Fn()>,
}

impl BackgroundConfigWidget {
    pub fn new() -> Self {
        // Delegate to full implementation
        Self::new_full()
    }

    // Test version with limited functionality (only solid color)
    #[allow(dead_code)]
    fn new_test_solid_only() -> Self {
        let container = GtkBox::new(Orientation::Vertical, 12);
        container.set_margin_start(12);
        container.set_margin_end(12);
        container.set_margin_top(12);
        container.set_margin_bottom(12);

        let config = Rc::new(RefCell::new(BackgroundConfig::default()));
        let on_change: Rc<RefCell<Option<std::boxed::Box<dyn Fn()>>>> = Rc::new(RefCell::new(None));
        let theme_config = Rc::new(RefCell::new(ComboThemeConfig::default()));

        // Type selector (limited to Solid Color only for testing)
        let type_box = GtkBox::new(Orientation::Horizontal, 6);
        type_box.append(&Label::new(Some("Background Type:")));

        let type_options = StringList::new(&["Solid Color"]);
        let type_dropdown = DropDown::new(Some(type_options), Option::<gtk4::Expression>::None);
        type_dropdown.set_selected(0);
        type_box.append(&type_dropdown);
        container.append(&type_box);

        // Preview using Picture + MemoryTexture (no DrawingArea)
        let preview_picture = gtk4::Picture::new();
        preview_picture.set_content_fit(gtk4::ContentFit::Contain);
        preview_picture.set_size_request(200, 200);
        preview_picture.set_hexpand(true);

        let preview_picture_rc = Rc::new(preview_picture.clone());

        let update_preview = {
            let config = config.clone();
            let theme_config = theme_config.clone();
            let picture = preview_picture_rc.clone();
            Rc::new(move || {
                use crate::ui::background::render_background_with_theme;

                let size = 200i32;

                let mut surface = cairo::ImageSurface::create(cairo::Format::ARgb32, size, size)
                    .expect("Failed to create surface");
                {
                    let cr = cairo::Context::new(&surface).expect("Failed to create context");
                    render_checkerboard(&cr, size as f64, size as f64);
                    let cfg = config.borrow();
                    let theme = theme_config.borrow();
                    let _ = render_background_with_theme(
                        &cr,
                        &cfg,
                        size as f64,
                        size as f64,
                        Some(&theme),
                    );
                }
                surface.flush();

                let data = surface.data().expect("Failed to get surface data");
                let bytes = gtk4::glib::Bytes::from(&data[..]);
                let texture = gtk4::gdk::MemoryTexture::new(
                    size,
                    size,
                    gtk4::gdk::MemoryFormat::B8g8r8a8Premultiplied,
                    &bytes,
                    (size * 4) as usize,
                );
                picture.set_paintable(Some(&texture));
            })
        };

        update_preview();
        container.append(&preview_picture);

        // Solid color configuration
        let color_box = GtkBox::new(Orientation::Horizontal, 6);
        color_box.append(&Label::new(Some("Color:")));

        let initial_color_source = ColorSource::custom(Color::new(0.15, 0.15, 0.15, 1.0));
        let solid_color_selector = Rc::new(ThemeColorSelector::new(initial_color_source));
        solid_color_selector.set_theme_config(theme_config.borrow().clone());
        color_box.append(solid_color_selector.widget());
        container.append(&color_box);

        // Connect color selector change
        let config_clone = config.clone();
        let update_preview_clone = update_preview.clone();
        let on_change_clone = on_change.clone();
        solid_color_selector.set_on_change(move |new_color_source| {
            let mut cfg = config_clone.borrow_mut();
            cfg.background = BackgroundType::Solid {
                color: new_color_source,
            };
            drop(cfg);
            update_preview_clone();
            if let Some(callback) = on_change_clone.borrow().as_ref() {
                callback();
            }
        });

        let type_dropdown_handler_id = type_dropdown.connect_selected_notify(|_| {});

        // Dummy GradientEditors (not added to UI)
        let linear_gradient_editor = Rc::new(GradientEditor::new());
        let radial_gradient_editor = Rc::new(GradientEditor::new_without_angle());
        let indicator_gradient_editor = Rc::new(GradientEditor::new_linear_no_angle());

        let config_stack = Stack::new();
        let source_fields = Rc::new(RefCell::new(Vec::new()));
        let is_combo_source = Rc::new(RefCell::new(false));
        let indicator_field_list = StringList::new(&["value"]);
        let indicator_field_dropdown = DropDown::new(
            Some(indicator_field_list.clone()),
            Option::<gtk4::Expression>::None,
        );
        let indicator_field_entry = Entry::new();
        let indicator_field_dropdown_box = GtkBox::new(Orientation::Horizontal, 6);
        let indicator_field_entry_box = GtkBox::new(Orientation::Horizontal, 6);
        let syncing_indicator_dropdown = Rc::new(RefCell::new(false));
        let polygon_color1_selector = Rc::new(ThemeColorSelector::new(ColorSource::custom(
            Color::default(),
        )));
        let polygon_color2_selector = Rc::new(ThemeColorSelector::new(ColorSource::custom(
            Color::default(),
        )));
        let polygon_bg_color_selector = Rc::new(ThemeColorSelector::new(ColorSource::custom(
            Color::default(),
        )));

        Self {
            container,
            config,
            config_stack,
            type_dropdown,
            on_change,
            type_dropdown_handler_id,
            solid_color_selector,
            linear_gradient_editor,
            radial_gradient_editor,
            indicator_gradient_editor,
            source_fields,
            is_combo_source,
            indicator_field_dropdown,
            indicator_field_list,
            indicator_field_entry,
            indicator_field_dropdown_box,
            indicator_field_entry_box,
            syncing_indicator_dropdown,
            theme_config,
            polygon_color1_selector,
            polygon_color2_selector,
            polygon_bg_color_selector,
            update_preview,
        }
    }

    #[allow(dead_code)]
    fn new_minimal() -> Self {
        // MINIMAL VERSION FOR DEBUGGING - just a label, no DrawingAreas
        let container = GtkBox::new(Orientation::Vertical, 12);
        container.set_margin_start(12);
        container.set_margin_end(12);
        container.set_margin_top(12);
        container.set_margin_bottom(12);

        let debug_label = Label::new(Some("Background config widget (minimal debug version)"));
        container.append(&debug_label);

        let config = Rc::new(RefCell::new(BackgroundConfig::default()));
        let on_change: Rc<RefCell<Option<std::boxed::Box<dyn Fn()>>>> = Rc::new(RefCell::new(None));
        let theme_config = Rc::new(RefCell::new(ComboThemeConfig::default()));

        // Dummy widgets for struct fields
        let config_stack = Stack::new();
        let type_options = StringList::new(&["Solid Color"]);
        let type_dropdown = DropDown::new(Some(type_options), Option::<gtk4::Expression>::None);
        let type_dropdown_handler_id = type_dropdown.connect_selected_notify(|_| {});

        let solid_color_selector = Rc::new(ThemeColorSelector::new(ColorSource::custom(
            Color::default(),
        )));
        let linear_gradient_editor = Rc::new(GradientEditor::new());
        let radial_gradient_editor = Rc::new(GradientEditor::new_without_angle());
        let indicator_gradient_editor = Rc::new(GradientEditor::new_linear_no_angle());
        let source_fields = Rc::new(RefCell::new(Vec::new()));
        let is_combo_source = Rc::new(RefCell::new(false));
        let indicator_field_list = StringList::new(&["value"]);
        let indicator_field_dropdown = DropDown::new(
            Some(indicator_field_list.clone()),
            Option::<gtk4::Expression>::None,
        );
        let indicator_field_entry = Entry::new();
        let indicator_field_dropdown_box = GtkBox::new(Orientation::Horizontal, 6);
        let indicator_field_entry_box = GtkBox::new(Orientation::Horizontal, 6);
        let syncing_indicator_dropdown = Rc::new(RefCell::new(false));
        let polygon_color1_selector = Rc::new(ThemeColorSelector::new(ColorSource::custom(
            Color::default(),
        )));
        let polygon_color2_selector = Rc::new(ThemeColorSelector::new(ColorSource::custom(
            Color::default(),
        )));
        let polygon_bg_color_selector = Rc::new(ThemeColorSelector::new(ColorSource::custom(
            Color::new(0.1, 0.1, 0.12, 1.0),
        )));
        let update_preview: Rc<dyn Fn()> = Rc::new(|| {});

        Self {
            container,
            config,
            config_stack,
            type_dropdown,
            on_change,
            type_dropdown_handler_id,
            solid_color_selector,
            linear_gradient_editor,
            radial_gradient_editor,
            indicator_gradient_editor,
            source_fields,
            is_combo_source,
            indicator_field_dropdown,
            indicator_field_list,
            indicator_field_entry,
            indicator_field_dropdown_box,
            indicator_field_entry_box,
            syncing_indicator_dropdown,
            theme_config,
            polygon_color1_selector,
            polygon_color2_selector,
            polygon_bg_color_selector,
            update_preview,
        }
    }

    #[allow(dead_code)]
    fn new_full() -> Self {
        let container = GtkBox::new(Orientation::Vertical, 12);
        container.set_margin_start(12);
        container.set_margin_end(12);
        container.set_margin_top(12);
        container.set_margin_bottom(12);

        let config = Rc::new(RefCell::new(BackgroundConfig::default()));
        let on_change = Rc::new(RefCell::new(None));
        let theme_config = Rc::new(RefCell::new(ComboThemeConfig::default()));

        // Type selector
        let type_box = GtkBox::new(Orientation::Horizontal, 6);
        type_box.append(&Label::new(Some("Background Type:")));

        let type_options = StringList::new(&[
            "Solid Color",
            "Linear Gradient",
            "Radial Gradient",
            "Image",
            "Tessellated Polygons",
            "Indicator",
        ]);
        let type_dropdown = DropDown::new(Some(type_options), Option::<gtk4::Expression>::None);
        type_dropdown.set_selected(0); // Default to Solid Color

        type_box.append(&type_dropdown);
        container.append(&type_box);

        // Preview using Picture + Cairo ImageSurface (avoids GL renderer issues with DrawingArea)
        let preview_picture = gtk4::Picture::new();
        preview_picture.set_content_fit(gtk4::ContentFit::Contain);
        preview_picture.set_size_request(200, 200);
        preview_picture.set_hexpand(true);

        // Function to update the preview picture
        let update_preview: Rc<dyn Fn()> = {
            let config = config.clone();
            let theme_config = theme_config.clone();
            let picture = preview_picture.clone();
            Rc::new(move || {
                use crate::ui::background::render_background_with_theme;

                let size = 200i32;

                // Create Cairo ImageSurface
                let mut surface =
                    match cairo::ImageSurface::create(cairo::Format::ARgb32, size, size) {
                        Ok(s) => s,
                        Err(e) => {
                            log::error!("Failed to create surface: {}", e);
                            return;
                        }
                    };
                {
                    let cr = match cairo::Context::new(&surface) {
                        Ok(c) => c,
                        Err(e) => {
                            log::error!("Failed to create context: {}", e);
                            return;
                        }
                    };

                    // Render checkerboard pattern
                    render_checkerboard(&cr, size as f64, size as f64);

                    // Render background
                    let cfg = config.borrow();
                    let theme = theme_config.borrow();
                    let _ = render_background_with_theme(
                        &cr,
                        &cfg,
                        size as f64,
                        size as f64,
                        Some(&theme),
                    );
                }

                // Convert to GdkTexture using MemoryTexture
                let data = match surface.data() {
                    Ok(d) => d,
                    Err(e) => {
                        log::error!("Failed to get surface data: {}", e);
                        return;
                    }
                };
                let bytes = gtk4::glib::Bytes::from(&data[..]);
                let texture = gtk4::gdk::MemoryTexture::new(
                    size,
                    size,
                    gtk4::gdk::MemoryFormat::B8g8r8a8Premultiplied,
                    &bytes,
                    (size * 4) as usize,
                );

                picture.set_paintable(Some(&texture));
            })
        };

        // Initial render
        update_preview();

        container.append(&preview_picture);

        // Configuration stack (different UI for each type)
        let config_stack = Stack::new();
        config_stack.set_vexpand(true);

        // Solid color configuration
        let (solid_page, solid_color_selector) =
            Self::create_solid_config(&config, &update_preview, &on_change, &theme_config);
        config_stack.add_named(&solid_page, Some("solid"));

        // Linear gradient configuration
        let (linear_page, linear_gradient_editor) =
            Self::create_linear_gradient_config(&config, &update_preview, &on_change);
        config_stack.add_named(&linear_page, Some("linear_gradient"));

        // Radial gradient configuration
        let (radial_page, radial_gradient_editor) =
            Self::create_radial_gradient_config(&config, &update_preview, &on_change);
        config_stack.add_named(&radial_page, Some("radial_gradient"));

        // Image configuration
        let image_page = Self::create_image_config(&config, &update_preview, &on_change);
        config_stack.add_named(&image_page, Some("image"));

        // Polygon configuration
        let (
            polygon_page,
            polygon_color1_selector,
            polygon_color2_selector,
            polygon_bg_color_selector,
        ) = Self::create_polygon_config(&config, &update_preview, &on_change, &theme_config);
        config_stack.add_named(&polygon_page, Some("polygons"));

        // Initialize source fields storage and syncing flag
        let source_fields = Rc::new(RefCell::new(Vec::new()));
        let is_combo_source = Rc::new(RefCell::new(false));
        let syncing_indicator_dropdown = Rc::new(RefCell::new(false));

        // Indicator configuration
        let (
            indicator_page,
            indicator_gradient_editor,
            indicator_field_dropdown,
            indicator_field_list,
            indicator_field_entry,
            indicator_field_dropdown_box,
            indicator_field_entry_box,
            indicator_field_dropdown_handler_id,
        ) = Self::create_indicator_config(
            &config,
            &update_preview,
            &on_change,
            &syncing_indicator_dropdown,
        );
        config_stack.add_named(&indicator_page, Some("indicator"));

        container.append(&config_stack);

        // Connect type selector
        let config_clone = config.clone();
        let stack_clone = config_stack.clone();
        let update_preview_clone = update_preview.clone();
        let on_change_clone = on_change.clone();
        // Clone indicator widgets for syncing when switching to Indicator type
        let indicator_field_dropdown_clone = indicator_field_dropdown.clone();
        let indicator_field_list_clone = indicator_field_list.clone();
        let indicator_field_entry_clone = indicator_field_entry.clone();
        let syncing_flag_for_type = syncing_indicator_dropdown.clone();

        let type_dropdown_handler_id = type_dropdown.connect_selected_notify(move |dropdown| {
            let selected = dropdown.selected();
            log::info!("Background type dropdown changed to: {}", selected);
            let page_name = match selected {
                0 => "solid",
                1 => "linear_gradient",
                2 => "radial_gradient",
                3 => "image",
                4 => "polygons",
                5 => "indicator",
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
                    BackgroundType::Indicator(_) => 5,
                }
            };

            // Only reset to defaults if the type actually changed
            if selected != current_type_index {
                let background_type = match selected {
                    0 => BackgroundType::Solid {
                        color: ColorSource::custom(Color::new(0.15, 0.15, 0.15, 1.0)),
                    },
                    1 => BackgroundType::LinearGradient(LinearGradientConfig::default()),
                    2 => BackgroundType::RadialGradient(RadialGradientConfig::default()),
                    3 => BackgroundType::Image {
                        path: String::new(),
                        display_mode: ImageDisplayMode::Fit,
                        alpha: 1.0,
                    },
                    4 => BackgroundType::Polygons(PolygonConfig::default()),
                    5 => BackgroundType::Indicator(IndicatorBackgroundConfig::default()),
                    _ => BackgroundType::default(),
                };

                let mut cfg = config_clone.borrow_mut();
                cfg.background = background_type;
                drop(cfg);

                // If switching to Indicator, sync the field dropdown to show the default value
                if selected == 5 {
                    // Get the default value_field from the new config
                    let value_field = {
                        let cfg = config_clone.borrow();
                        if let BackgroundType::Indicator(ref ind) = cfg.background {
                            ind.value_field.clone()
                        } else {
                            "value".to_string()
                        }
                    };

                    // Find and select the matching item in dropdown
                    let mut selected_index: u32 = 0;
                    let list_count = indicator_field_list_clone.n_items();
                    for i in 0..list_count {
                        if let Some(item) = indicator_field_list_clone.string(i) {
                            if item == value_field {
                                selected_index = i;
                                break;
                            }
                        }
                    }

                    // Set syncing flag and update selection
                    *syncing_flag_for_type.borrow_mut() = true;
                    indicator_field_dropdown_clone.set_selected(selected_index);
                    *syncing_flag_for_type.borrow_mut() = false;

                    // Also update entry
                    indicator_field_entry_clone.set_text(&value_field);
                }

                if let Some(callback) = on_change_clone.borrow().as_ref() {
                    callback();
                }
            }

            update_preview_clone();
        });

        // Note: indicator_field_dropdown_handler_id is captured in the handler closure
        // We don't need to store it since we use syncing_indicator_dropdown flag instead
        let _ = indicator_field_dropdown_handler_id;

        Self {
            container,
            config,
            config_stack,
            type_dropdown,
            on_change,
            type_dropdown_handler_id,
            solid_color_selector,
            linear_gradient_editor,
            radial_gradient_editor,
            indicator_gradient_editor,
            source_fields,
            is_combo_source,
            indicator_field_dropdown,
            indicator_field_list,
            indicator_field_entry,
            indicator_field_dropdown_box,
            indicator_field_entry_box,
            syncing_indicator_dropdown,
            theme_config,
            polygon_color1_selector,
            polygon_color2_selector,
            polygon_bg_color_selector,
            update_preview,
        }
    }

    fn create_solid_config(
        config: &Rc<RefCell<BackgroundConfig>>,
        update_preview: &Rc<dyn Fn()>,
        on_change: &Rc<RefCell<Option<std::boxed::Box<dyn Fn()>>>>,
        theme_config: &Rc<RefCell<ComboThemeConfig>>,
    ) -> (GtkBox, Rc<ThemeColorSelector>) {
        let page = GtkBox::new(Orientation::Vertical, 6);

        // Solid color - using ThemeColorSelector (theme-aware)
        let color_box = GtkBox::new(Orientation::Horizontal, 6);
        color_box.append(&Label::new(Some("Color:")));

        let initial_color_source =
            if let BackgroundType::Solid { color } = &config.borrow().background {
                color.clone()
            } else {
                ColorSource::custom(Color::default())
            };
        let color_selector = Rc::new(ThemeColorSelector::new(initial_color_source));
        color_selector.set_theme_config(theme_config.borrow().clone());
        color_box.append(color_selector.widget());
        page.append(&color_box);

        let config_clone = config.clone();
        let update_preview_clone = update_preview.clone();
        let on_change_clone = on_change.clone();
        color_selector.set_on_change(move |new_color_source| {
            config_clone.borrow_mut().background = BackgroundType::Solid {
                color: new_color_source,
            };
            update_preview_clone();
            if let Some(callback) = on_change_clone.borrow().as_ref() {
                callback();
            }
        });

        (page, color_selector)
    }

    fn create_linear_gradient_config(
        config: &Rc<RefCell<BackgroundConfig>>,
        update_preview: &Rc<dyn Fn()>,
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
        let update_preview_for_paste = update_preview.clone();
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

                        update_preview_for_paste();

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
        let update_preview_clone = update_preview.clone();
        let on_change_clone = on_change.clone();
        let gradient_editor_clone = gradient_editor_ref.clone();

        gradient_editor_ref.set_on_change(move || {
            let grad_config = gradient_editor_clone.get_gradient();
            let mut cfg = config_clone.borrow_mut();
            cfg.background = BackgroundType::LinearGradient(grad_config);
            drop(cfg);
            update_preview_clone();

            if let Some(callback) = on_change_clone.borrow().as_ref() {
                callback();
            }
        });

        page.append(gradient_editor_ref.widget());
        (page, gradient_editor_ref.clone())
    }

    fn create_radial_gradient_config(
        config: &Rc<RefCell<BackgroundConfig>>,
        update_preview: &Rc<dyn Fn()>,
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
        let update_preview_for_paste = update_preview.clone();
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

                        update_preview_for_paste();

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
        let update_preview_clone = update_preview.clone();
        let on_change_clone = on_change.clone();

        radius_scale.connect_value_changed(move |scale| {
            let mut cfg = config_clone.borrow_mut();
            if let BackgroundType::RadialGradient(ref mut grad) = cfg.background {
                grad.radius = scale.value();
                drop(cfg);
                update_preview_clone();

                if let Some(callback) = on_change_clone.borrow().as_ref() {
                    callback();
                }
            }
        });

        radius_box.append(&radius_scale);
        page.append(&radius_box);

        // Set up change handler
        let config_clone = config.clone();
        let update_preview_clone2 = update_preview.clone();
        let on_change_clone = on_change.clone();
        let gradient_editor_clone = gradient_editor_ref.clone();

        gradient_editor_ref.set_on_change(move || {
            let stops = gradient_editor_clone.get_stops();
            let mut cfg = config_clone.borrow_mut();
            if let BackgroundType::RadialGradient(ref mut grad) = cfg.background {
                grad.stops = stops;
            }
            drop(cfg);
            update_preview_clone2();

            if let Some(callback) = on_change_clone.borrow().as_ref() {
                callback();
            }
        });

        page.append(gradient_editor_ref.widget());
        (page, gradient_editor_ref.clone())
    }

    fn create_image_config(
        config: &Rc<RefCell<BackgroundConfig>>,
        update_preview: &Rc<dyn Fn()>,
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
        let update_preview_clone = update_preview.clone();
        let path_entry_clone = path_entry.clone();
        let on_change_clone = on_change.clone();

        browse_button.connect_clicked(move |btn| {
            let config_clone2 = config_clone.clone();
            let update_preview_clone2 = update_preview_clone.clone();
            let path_entry_clone2 = path_entry_clone.clone();
            let on_change_clone2 = on_change_clone.clone();

            let window = btn.root().and_downcast::<gtk4::Window>();

            // Use GTK4 FileDialog for native file selection
            let dialog = gtk4::FileDialog::builder()
                .title("Select Background Image")
                .modal(true)
                .build();

            // Set up image file filter
            let filter = gtk4::FileFilter::new();
            filter.set_name(Some("Image Files"));
            filter.add_mime_type("image/*");
            filter.add_suffix("png");
            filter.add_suffix("jpg");
            filter.add_suffix("jpeg");
            filter.add_suffix("gif");
            filter.add_suffix("bmp");
            filter.add_suffix("webp");
            filter.add_suffix("svg");

            let filters = gtk4::gio::ListStore::new::<gtk4::FileFilter>();
            filters.append(&filter);
            dialog.set_filters(Some(&filters));
            dialog.set_default_filter(Some(&filter));

            gtk4::glib::MainContext::default().spawn_local(async move {
                if let Ok(file) = dialog.open_future(window.as_ref()).await {
                    if let Some(path) = file.path() {
                        let path_str = path.to_string_lossy().to_string();
                        path_entry_clone2.set_text(&path_str);

                        let mut cfg = config_clone2.borrow_mut();
                        if let BackgroundType::Image { ref mut path, .. } = cfg.background {
                            *path = path_str;
                            drop(cfg);
                            update_preview_clone2();

                            if let Some(callback) = on_change_clone2.borrow().as_ref() {
                                callback();
                            }
                        }
                    }
                }
            });
        });

        // Display mode change handler
        let config_clone = config.clone();
        let update_preview_clone = update_preview.clone();
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
            if let BackgroundType::Image {
                display_mode: ref mut dm,
                ..
            } = cfg.background
            {
                *dm = display_mode;
                drop(cfg);
                update_preview_clone();

                if let Some(callback) = on_change_clone.borrow().as_ref() {
                    callback();
                }
            }
        });

        // Alpha slider handler
        let config_clone = config.clone();
        let update_preview_clone = update_preview.clone();
        let on_change_clone = on_change.clone();

        alpha_scale.connect_value_changed(move |scale| {
            let mut cfg = config_clone.borrow_mut();
            if let BackgroundType::Image { ref mut alpha, .. } = cfg.background {
                *alpha = scale.value();
                drop(cfg);
                update_preview_clone();

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
        update_preview: &Rc<dyn Fn()>,
        on_change: &Rc<RefCell<Option<std::boxed::Box<dyn Fn()>>>>,
        theme_config: &Rc<RefCell<ComboThemeConfig>>,
    ) -> (
        GtkBox,
        Rc<ThemeColorSelector>,
        Rc<ThemeColorSelector>,
        Rc<ThemeColorSelector>,
    ) {
        let page = GtkBox::new(Orientation::Vertical, 12);

        // Tile size
        let size_box = GtkBox::new(Orientation::Horizontal, 6);
        size_box.append(&Label::new(Some("Tile Size:")));

        let size_spin = SpinButton::with_range(10.0, 200.0, 5.0);
        size_spin.set_value(60.0);
        size_spin.set_hexpand(true);

        let config_clone = config.clone();
        let update_preview_clone = update_preview.clone();
        let on_change_clone = on_change.clone();

        size_spin.connect_value_changed(move |spin| {
            let mut cfg = config_clone.borrow_mut();
            if let BackgroundType::Polygons(ref mut poly) = cfg.background {
                poly.tile_size = spin.value() as u32;
                drop(cfg);
                update_preview_clone();

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
        let update_preview_clone = update_preview.clone();
        let on_change_clone = on_change.clone();

        sides_spin.connect_value_changed(move |spin| {
            let mut cfg = config_clone.borrow_mut();
            if let BackgroundType::Polygons(ref mut poly) = cfg.background {
                poly.num_sides = spin.value() as u32;
                drop(cfg);
                update_preview_clone();

                if let Some(callback) = on_change_clone.borrow().as_ref() {
                    callback();
                }
            }
        });

        sides_box.append(&sides_spin);
        page.append(&sides_box);

        // Rotation angle with slider and spinner
        let angle_box = GtkBox::new(Orientation::Horizontal, 6);
        angle_box.append(&Label::new(Some("Rotation:")));

        let angle_scale = Scale::with_range(Orientation::Horizontal, -360.0, 360.0, 5.0);
        angle_scale.set_value(0.0);
        angle_scale.set_hexpand(true);

        let angle_spin = SpinButton::with_range(-360.0, 360.0, 1.0);
        angle_spin.set_value(0.0);
        angle_spin.set_width_chars(6);

        // Flag to prevent recursive updates between scale and spin
        let syncing = Rc::new(RefCell::new(false));

        // Sync scale -> spin
        let angle_spin_clone = angle_spin.clone();
        let config_clone = config.clone();
        let update_preview_clone = update_preview.clone();
        let on_change_clone = on_change.clone();
        let syncing_clone = syncing.clone();
        angle_scale.connect_value_changed(move |scale| {
            if *syncing_clone.borrow() {
                return;
            }
            *syncing_clone.borrow_mut() = true;
            angle_spin_clone.set_value(scale.value());
            *syncing_clone.borrow_mut() = false;

            let mut cfg = config_clone.borrow_mut();
            if let BackgroundType::Polygons(ref mut poly) = cfg.background {
                poly.rotation_angle = scale.value();
                drop(cfg);
                update_preview_clone();
                if let Some(callback) = on_change_clone.borrow().as_ref() {
                    callback();
                }
            }
        });

        // Sync spin -> scale
        let angle_scale_clone = angle_scale.clone();
        let config_clone = config.clone();
        let update_preview_clone = update_preview.clone();
        let on_change_clone = on_change.clone();
        let syncing_clone = syncing.clone();
        angle_spin.connect_value_changed(move |spin| {
            if *syncing_clone.borrow() {
                return;
            }
            *syncing_clone.borrow_mut() = true;
            angle_scale_clone.set_value(spin.value());
            *syncing_clone.borrow_mut() = false;

            let mut cfg = config_clone.borrow_mut();
            if let BackgroundType::Polygons(ref mut poly) = cfg.background {
                poly.rotation_angle = spin.value();
                drop(cfg);
                update_preview_clone();
                if let Some(callback) = on_change_clone.borrow().as_ref() {
                    callback();
                }
            }
        });

        angle_box.append(&angle_scale);
        angle_box.append(&angle_spin);
        page.append(&angle_box);

        // Create SizeGroup to align color labels
        let label_size_group = SizeGroup::new(SizeGroupMode::Horizontal);

        // Color 1 - using ThemeColorSelector (theme-aware)
        let color1_box = GtkBox::new(Orientation::Horizontal, 6);
        let color1_label = Label::new(Some("Color 1:"));
        color1_label.set_xalign(0.0);
        label_size_group.add_widget(&color1_label);
        color1_box.append(&color1_label);

        let color1_source = if let BackgroundType::Polygons(ref poly) = config.borrow().background {
            poly.colors.first().cloned().unwrap_or_default()
        } else {
            ColorSource::default()
        };
        let color1_selector = Rc::new(ThemeColorSelector::new(color1_source));
        color1_selector.set_theme_config(theme_config.borrow().clone());
        color1_box.append(color1_selector.widget());
        page.append(&color1_box);

        let config_clone = config.clone();
        let update_preview_clone = update_preview.clone();
        let on_change_clone = on_change.clone();
        color1_selector.set_on_change(move |new_source| {
            let mut cfg = config_clone.borrow_mut();
            if let BackgroundType::Polygons(ref mut poly) = cfg.background {
                if poly.colors.is_empty() {
                    poly.colors.push(new_source);
                } else {
                    poly.colors[0] = new_source;
                }
            }
            drop(cfg);
            update_preview_clone();
            if let Some(callback) = on_change_clone.borrow().as_ref() {
                callback();
            }
        });

        // Color 2 - using ThemeColorSelector (theme-aware)
        let color2_box = GtkBox::new(Orientation::Horizontal, 6);
        let color2_label = Label::new(Some("Color 2:"));
        color2_label.set_xalign(0.0);
        label_size_group.add_widget(&color2_label);
        color2_box.append(&color2_label);

        let color2_source = if let BackgroundType::Polygons(ref poly) = config.borrow().background {
            poly.colors.get(1).cloned().unwrap_or_default()
        } else {
            ColorSource::default()
        };
        let color2_selector = Rc::new(ThemeColorSelector::new(color2_source));
        color2_selector.set_theme_config(theme_config.borrow().clone());
        color2_box.append(color2_selector.widget());
        page.append(&color2_box);

        let config_clone = config.clone();
        let update_preview_clone = update_preview.clone();
        let on_change_clone = on_change.clone();
        color2_selector.set_on_change(move |new_source| {
            let mut cfg = config_clone.borrow_mut();
            if let BackgroundType::Polygons(ref mut poly) = cfg.background {
                if poly.colors.len() < 2 {
                    poly.colors.push(new_source);
                } else {
                    poly.colors[1] = new_source;
                }
            }
            drop(cfg);
            update_preview_clone();
            if let Some(callback) = on_change_clone.borrow().as_ref() {
                callback();
            }
        });

        // Background Color (fills gaps between polygons)
        let bg_color_box = GtkBox::new(Orientation::Horizontal, 6);
        let bg_color_label = Label::new(Some("Background:"));
        bg_color_label.set_xalign(0.0);
        label_size_group.add_widget(&bg_color_label);
        bg_color_box.append(&bg_color_label);

        let bg_color_source = if let BackgroundType::Polygons(ref poly) = config.borrow().background
        {
            poly.background_color.clone()
        } else {
            ColorSource::custom(Color::new(0.1, 0.1, 0.12, 1.0))
        };
        let bg_color_selector = Rc::new(ThemeColorSelector::new(bg_color_source));
        bg_color_selector.set_theme_config(theme_config.borrow().clone());
        bg_color_box.append(bg_color_selector.widget());
        page.append(&bg_color_box);

        let config_clone = config.clone();
        let update_preview_clone = update_preview.clone();
        let on_change_clone = on_change.clone();
        bg_color_selector.set_on_change(move |new_source| {
            let mut cfg = config_clone.borrow_mut();
            if let BackgroundType::Polygons(ref mut poly) = cfg.background {
                poly.background_color = new_source;
            }
            drop(cfg);
            update_preview_clone();
            if let Some(callback) = on_change_clone.borrow().as_ref() {
                callback();
            }
        });

        (page, color1_selector, color2_selector, bg_color_selector)
    }

    /// Create indicator configuration page
    /// Returns (page, gradient_editor, field_dropdown, field_list, field_entry, dropdown_box, entry_box, dropdown_handler_id)
    fn create_indicator_config(
        config: &Rc<RefCell<BackgroundConfig>>,
        update_preview: &Rc<dyn Fn()>,
        on_change: &Rc<RefCell<Option<std::boxed::Box<dyn Fn()>>>>,
        syncing_flag: &Rc<RefCell<bool>>,
    ) -> (
        GtkBox,
        Rc<GradientEditor>,
        DropDown,
        StringList,
        Entry,
        GtkBox,
        GtkBox,
        gtk4::glib::SignalHandlerId,
    ) {
        use crate::ui::background::IndicatorBackgroundShape;

        let page = GtkBox::new(Orientation::Vertical, 12);
        page.set_margin_start(12);
        page.set_margin_end(12);
        page.set_margin_top(12);
        page.set_margin_bottom(12);

        // Shape selection
        let shape_box = GtkBox::new(Orientation::Horizontal, 6);
        shape_box.append(&Label::new(Some("Shape:")));
        let shape_list = StringList::new(&[
            "Fill", "Circle", "Square", "Triangle", "Pentagon", "Hexagon",
        ]);
        let shape_dropdown = DropDown::new(Some(shape_list), gtk4::Expression::NONE);
        shape_dropdown.set_hexpand(true);
        shape_box.append(&shape_dropdown);
        page.append(&shape_box);

        // Shape size
        let size_box = GtkBox::new(Orientation::Horizontal, 6);
        size_box.append(&Label::new(Some("Size:")));
        let size_scale = Scale::with_range(Orientation::Horizontal, 0.1, 1.0, 0.05);
        size_scale.set_value(0.8);
        size_scale.set_hexpand(true);
        size_box.append(&size_scale);
        page.append(&size_box);

        // Rotation
        let rotation_box = GtkBox::new(Orientation::Horizontal, 6);
        rotation_box.append(&Label::new(Some("Rotation:")));
        let rotation_spin = SpinButton::with_range(-360.0, 360.0, 1.0);
        rotation_spin.set_value(0.0);
        rotation_box.append(&rotation_spin);
        page.append(&rotation_box);

        // Value field - Dropdown for single sources (shown by default)
        let field_dropdown_box = GtkBox::new(Orientation::Horizontal, 6);
        field_dropdown_box.append(&Label::new(Some("Value Field:")));
        let field_list = StringList::new(&["(none)"]);
        let field_dropdown = DropDown::new(Some(field_list.clone()), gtk4::Expression::NONE);
        field_dropdown.set_hexpand(true);
        field_dropdown_box.append(&field_dropdown);
        page.append(&field_dropdown_box);

        // Value field - Entry for combo sources (hidden by default)
        let field_entry_box = GtkBox::new(Orientation::Horizontal, 6);
        field_entry_box.append(&Label::new(Some("Value Field:")));
        let field_entry = Entry::new();
        field_entry.set_hexpand(true);
        field_entry.set_placeholder_text(Some("e.g., cpu_0_usage, gpu_0_temperature"));
        // Initialize from config
        {
            let cfg = config.borrow();
            if let BackgroundType::Indicator(ref ind) = cfg.background {
                field_entry.set_text(&ind.value_field);
            }
        }
        field_entry_box.append(&field_entry);
        field_entry_box.set_visible(false); // Hidden by default, shown for combo sources
        page.append(&field_entry_box);

        // Info label (changes based on source type)
        let info_label = Label::new(Some("Select the source field to use for indicator value"));
        info_label.set_halign(gtk4::Align::Start);
        info_label.add_css_class("dim-label");
        page.append(&info_label);

        // Static value (for preview/initial state)
        let value_box = GtkBox::new(Orientation::Horizontal, 6);
        value_box.append(&Label::new(Some("Preview/Static Value:")));
        let value_scale = Scale::with_range(Orientation::Horizontal, 0.0, 100.0, 1.0);
        value_scale.set_value(50.0);
        value_scale.set_hexpand(true);
        value_box.append(&value_scale);
        page.append(&value_box);

        // Gradient editor for color mapping (linear preview for value mapping)
        let gradient_label = Label::new(Some("Color Gradient (0%=min, 100%=max):"));
        gradient_label.set_halign(gtk4::Align::Start);
        page.append(&gradient_label);

        // Copy/Paste gradient buttons
        let copy_paste_box = GtkBox::new(Orientation::Horizontal, 6);
        copy_paste_box.set_halign(gtk4::Align::End);
        let copy_gradient_btn = Button::with_label("Copy Gradient");
        let paste_gradient_btn = Button::with_label("Paste Gradient");
        copy_paste_box.append(&copy_gradient_btn);
        copy_paste_box.append(&paste_gradient_btn);
        page.append(&copy_paste_box);

        let gradient_editor = Rc::new(GradientEditor::new_linear_no_angle());
        page.append(gradient_editor.widget());

        // Initialize gradient editor with existing stops
        {
            let cfg = config.borrow();
            if let BackgroundType::Indicator(ref ind) = cfg.background {
                gradient_editor.set_stops(ind.gradient_stops.clone());
            }
        }

        // Copy button handler - copy stops (angle is not relevant for indicator)
        let gradient_editor_copy = gradient_editor.clone();
        copy_gradient_btn.connect_clicked(move |_| {
            use crate::ui::CLIPBOARD;
            let stops = gradient_editor_copy.get_stops();
            if let Ok(mut clipboard) = CLIPBOARD.lock() {
                clipboard.gradient_stops = Some(stops);
            }
        });

        // Paste button handler - paste only stops, ignore angle
        let gradient_editor_paste = gradient_editor.clone();
        let config_paste = config.clone();
        let on_change_paste = on_change.clone();
        let update_preview_paste = update_preview.clone();
        paste_gradient_btn.connect_clicked(move |_| {
            use crate::ui::CLIPBOARD;
            if let Ok(clipboard) = CLIPBOARD.lock() {
                if let Some(ref stops) = clipboard.gradient_stops {
                    gradient_editor_paste.set_stops(stops.clone());
                    let mut cfg = config_paste.borrow_mut();
                    if let BackgroundType::Indicator(ref mut ind) = cfg.background {
                        ind.gradient_stops = stops.clone();
                    }
                    drop(cfg);
                    update_preview_paste();
                    if let Some(callback) = on_change_paste.borrow().as_ref() {
                        callback();
                    }
                }
            }
        });

        // Connect handlers
        let config_clone = config.clone();
        let update_preview_clone = update_preview.clone();
        let on_change_clone = on_change.clone();
        shape_dropdown.connect_selected_notify(move |dropdown| {
            let shape = match dropdown.selected() {
                0 => IndicatorBackgroundShape::Fill,
                1 => IndicatorBackgroundShape::Circle,
                2 => IndicatorBackgroundShape::Square,
                3 => IndicatorBackgroundShape::Polygon(3),
                4 => IndicatorBackgroundShape::Polygon(5),
                5 => IndicatorBackgroundShape::Polygon(6),
                _ => IndicatorBackgroundShape::Fill,
            };
            let mut cfg = config_clone.borrow_mut();
            if let BackgroundType::Indicator(ref mut ind) = cfg.background {
                ind.shape = shape;
            }
            drop(cfg);
            update_preview_clone();
            if let Some(callback) = on_change_clone.borrow().as_ref() {
                callback();
            }
        });

        let config_clone = config.clone();
        let update_preview_clone = update_preview.clone();
        let on_change_clone = on_change.clone();
        size_scale.connect_value_changed(move |scale| {
            let mut cfg = config_clone.borrow_mut();
            if let BackgroundType::Indicator(ref mut ind) = cfg.background {
                ind.shape_size = scale.value();
            }
            drop(cfg);
            update_preview_clone();
            if let Some(callback) = on_change_clone.borrow().as_ref() {
                callback();
            }
        });

        let config_clone = config.clone();
        let update_preview_clone = update_preview.clone();
        let on_change_clone = on_change.clone();
        rotation_spin.connect_value_changed(move |spin| {
            let mut cfg = config_clone.borrow_mut();
            if let BackgroundType::Indicator(ref mut ind) = cfg.background {
                ind.rotation_angle = spin.value();
            }
            drop(cfg);
            update_preview_clone();
            if let Some(callback) = on_change_clone.borrow().as_ref() {
                callback();
            }
        });

        // Value field dropdown change handler (for single sources)
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let field_list_clone = field_list.clone();
        let syncing_flag_clone = syncing_flag.clone();
        let field_dropdown_handler_id = field_dropdown.connect_selected_notify(move |dropdown| {
            // Skip if we're in the middle of programmatic sync
            if *syncing_flag_clone.borrow() {
                return;
            }

            let selected = dropdown.selected();
            if selected == gtk4::INVALID_LIST_POSITION {
                return;
            }
            // Get the field name from the StringList
            let field_name = if selected == 0 {
                String::new() // "(none)" selected
            } else if let Some(item) = field_list_clone.string(selected) {
                item.to_string()
            } else {
                return;
            };

            let mut cfg = config_clone.borrow_mut();
            if let BackgroundType::Indicator(ref mut ind) = cfg.background {
                ind.value_field = field_name;
            }
            drop(cfg);
            if let Some(callback) = on_change_clone.borrow().as_ref() {
                callback();
            }
        });

        // Value field entry change handler (for combo sources)
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        field_entry.connect_changed(move |entry| {
            let mut cfg = config_clone.borrow_mut();
            if let BackgroundType::Indicator(ref mut ind) = cfg.background {
                ind.value_field = entry.text().to_string();
            }
            drop(cfg);
            if let Some(callback) = on_change_clone.borrow().as_ref() {
                callback();
            }
        });

        let config_clone = config.clone();
        let update_preview_clone = update_preview.clone();
        let on_change_clone = on_change.clone();
        value_scale.connect_value_changed(move |scale| {
            let mut cfg = config_clone.borrow_mut();
            if let BackgroundType::Indicator(ref mut ind) = cfg.background {
                ind.static_value = scale.value();
            }
            drop(cfg);
            update_preview_clone();
            if let Some(callback) = on_change_clone.borrow().as_ref() {
                callback();
            }
        });

        // Connect gradient change handler to update config
        let config_clone = config.clone();
        let update_preview_clone = update_preview.clone();
        let on_change_clone = on_change.clone();
        let gradient_editor_clone = gradient_editor.clone();
        gradient_editor.set_on_change(move || {
            let stops = gradient_editor_clone.get_stops();
            let mut cfg = config_clone.borrow_mut();
            if let BackgroundType::Indicator(ref mut ind) = cfg.background {
                ind.gradient_stops = stops;
            }
            drop(cfg);
            update_preview_clone();
            if let Some(callback) = on_change_clone.borrow().as_ref() {
                callback();
            }
        });

        (
            page,
            gradient_editor,
            field_dropdown,
            field_list,
            field_entry,
            field_dropdown_box,
            field_entry_box,
            field_dropdown_handler_id,
        )
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
            BackgroundType::Indicator(_) => 5,
        };

        // Load data into widgets if applicable
        if let BackgroundType::Solid { ref color } = new_config.background {
            self.solid_color_selector.set_source(color.clone());
        }
        if let BackgroundType::LinearGradient(ref grad) = new_config.background {
            self.linear_gradient_editor.set_gradient(grad);
        }
        if let BackgroundType::RadialGradient(ref grad) = new_config.background {
            self.radial_gradient_editor.set_stops(grad.stops.clone());
        }
        if let BackgroundType::Polygons(ref poly) = new_config.background {
            // Update polygon color selectors with saved color sources
            if let Some(color1) = poly.colors.first() {
                self.polygon_color1_selector.set_source(color1.clone());
            }
            if let Some(color2) = poly.colors.get(1) {
                self.polygon_color2_selector.set_source(color2.clone());
            }
            self.polygon_bg_color_selector
                .set_source(poly.background_color.clone());
        }
        if let BackgroundType::Indicator(ref ind) = new_config.background {
            self.indicator_gradient_editor
                .set_stops(ind.gradient_stops.clone());
            // Update the field entry with saved value (for combo sources)
            self.indicator_field_entry.set_text(&ind.value_field);
        }

        *self.config.borrow_mut() = new_config;

        // Block the signal handler to prevent it from overwriting our config
        self.type_dropdown
            .block_signal(&self.type_dropdown_handler_id);

        // Update the dropdown selection (this won't trigger the handler now)
        self.type_dropdown.set_selected(type_index);

        // Unblock the signal handler
        self.type_dropdown
            .unblock_signal(&self.type_dropdown_handler_id);

        // Update the visible stack page to match the background type
        let page_name = match type_index {
            0 => "solid",
            1 => "linear_gradient",
            2 => "radial_gradient",
            3 => "image",
            4 => "polygons",
            5 => "indicator",
            _ => "solid",
        };
        self.config_stack.set_visible_child_name(page_name);

        (self.update_preview)();
    }

    /// Get the current configuration
    pub fn get_config(&self) -> BackgroundConfig {
        self.config.borrow().clone()
    }

    /// Set callback for when configuration changes
    pub fn set_on_change<F: Fn() + 'static>(&self, callback: F) {
        *self.on_change.borrow_mut() = Some(std::boxed::Box::new(callback));
    }

    /// Set whether this is a combo source (affects indicator config UI)
    pub fn set_is_combo_source(&self, is_combo: bool) {
        *self.is_combo_source.borrow_mut() = is_combo;

        // Show/hide appropriate field input widgets
        self.indicator_field_dropdown_box.set_visible(!is_combo);
        self.indicator_field_entry_box.set_visible(is_combo);

        // If switching to combo mode, copy current dropdown selection to entry
        if is_combo {
            let current_value = {
                let cfg = self.config.borrow();
                if let BackgroundType::Indicator(ref ind) = cfg.background {
                    ind.value_field.clone()
                } else {
                    String::new()
                }
            };
            self.indicator_field_entry.set_text(&current_value);
        }
    }

    /// Set available source fields for indicator configuration
    pub fn set_source_fields(&self, fields: Vec<FieldMetadata>) {
        use crate::core::{FieldPurpose, FieldType};

        *self.source_fields.borrow_mut() = fields.clone();

        // Rebuild the dropdown list with field names
        // Clear existing items
        while self.indicator_field_list.n_items() > 0 {
            self.indicator_field_list.remove(0);
        }

        // Add "(none)" as first option
        self.indicator_field_list.append("(none)");

        // Filter and add fields that are suitable for indicator values (numerical/percentage values)
        for field in &fields {
            // Only show fields that can provide numerical values for the indicator
            // Filter by FieldType (Numerical or Percentage) and FieldPurpose (Value or SecondaryValue)
            let is_numerical = matches!(
                field.field_type,
                FieldType::Numerical | FieldType::Percentage
            );
            let is_value_field = matches!(
                field.purpose,
                FieldPurpose::Value | FieldPurpose::SecondaryValue
            );

            if is_numerical && is_value_field {
                self.indicator_field_list.append(&field.id);
            }
        }

        // Sync the dropdown selection to match current config
        self.sync_indicator_field_dropdown();
    }

    /// Sync the indicator field dropdown to match the current config
    fn sync_indicator_field_dropdown(&self) {
        let current_value = {
            let cfg = self.config.borrow();
            if let BackgroundType::Indicator(ref ind) = cfg.background {
                ind.value_field.clone()
            } else {
                // If not indicator, use the default value_field so dropdown is ready
                "value".to_string()
            }
        };

        // Find the index of the current value in the dropdown
        let mut selected_index: u32 = 0;
        for i in 0..self.indicator_field_list.n_items() {
            if let Some(item) = self.indicator_field_list.string(i) {
                if item == current_value {
                    selected_index = i;
                    break;
                }
            }
        }

        // Set syncing flag and update selection
        *self.syncing_indicator_dropdown.borrow_mut() = true;
        self.indicator_field_dropdown.set_selected(selected_index);
        *self.syncing_indicator_dropdown.borrow_mut() = false;

        // Also update the entry for combo sources
        self.indicator_field_entry.set_text(&current_value);
    }

    /// Set theme config for all gradient editors, solid color, and polygon color selectors in this widget
    pub fn set_theme_config(&self, theme: crate::ui::theme::ComboThemeConfig) {
        // Store theme for preview rendering
        *self.theme_config.borrow_mut() = theme.clone();

        // Update solid color selector
        self.solid_color_selector.set_theme_config(theme.clone());

        // Update gradient editors
        self.linear_gradient_editor.set_theme_config(theme.clone());
        self.radial_gradient_editor.set_theme_config(theme.clone());
        self.indicator_gradient_editor
            .set_theme_config(theme.clone());

        // Update polygon color selectors
        self.polygon_color1_selector.set_theme_config(theme.clone());
        self.polygon_color2_selector.set_theme_config(theme.clone());
        self.polygon_bg_color_selector.set_theme_config(theme);

        // Redraw preview to reflect theme colors
        (self.update_preview)();
    }
}

impl Default for BackgroundConfigWidget {
    fn default() -> Self {
        Self::new()
    }
}
