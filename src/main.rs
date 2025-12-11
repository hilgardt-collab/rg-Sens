use gtk4::gdk::Display;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, CssProvider};
use log::{error, info, warn};
use rg_sens::config::{AppConfig, PanelConfig};
use rg_sens::core::{Panel, PanelGeometry, UpdateManager};
use rg_sens::ui::{GridConfig as UiGridConfig, GridLayout};
use rg_sens::{displayers, sources};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

const APP_ID: &str = "com.github.hilgardt_collab.rg_sens";

fn main() {
    // Initialize logger
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    info!("Starting rg-Sens v{}", env!("CARGO_PKG_VERSION"));

    // Initialize shared sensor caches early (before any source creation)
    sources::initialize_sensors();

    // Register all built-in sources and displayers
    sources::register_all();
    displayers::register_all();

    // Create GTK application
    let app = Application::builder().application_id(APP_ID).build();

    app.connect_activate(build_ui);

    // Run the application
    let args: Vec<String> = std::env::args().collect();
    app.run_with_args(&args);
}

fn build_ui(app: &Application) {
    info!("Building UI");

    // Load CSS for selection styling
    load_css();

    // Load user colors for color picker
    if let Err(e) = rg_sens::ui::custom_color_picker::CustomColorPicker::load_colors() {
        warn!("Failed to load user colors: {}", e);
    }

    // Load configuration from disk (or use defaults)
    let app_config = match AppConfig::load() {
        Ok(config) => {
            info!("Loaded configuration from disk");
            config
        }
        Err(e) => {
            warn!("Failed to load config, using defaults: {}", e);
            AppConfig::default()
        }
    };

    // Wrap app_config in Rc<RefCell<>> for shared mutable access
    let app_config = Rc::new(RefCell::new(app_config));

    // Create grid configuration from loaded config
    let grid_config = {
        let cfg = app_config.borrow();
        UiGridConfig {
            rows: cfg.grid.rows,
            columns: cfg.grid.columns,
            cell_width: cfg.grid.cell_width,
            cell_height: cfg.grid.cell_height,
            spacing: cfg.grid.spacing,
        }
    };

    // Create the main window with saved dimensions
    let window = {
        let cfg = app_config.borrow();
        let builder = gtk4::ApplicationWindow::builder()
            .application(app)
            .title("rg-Sens - System Monitor")
            .default_width(cfg.window.width)
            .default_height(cfg.window.height)
            .decorated(!cfg.window.borderless);
        builder.build()
    };

    // Restore window position if saved
    {
        let cfg = app_config.borrow();
        if let (Some(x), Some(y)) = (cfg.window.x, cfg.window.y) {
            // Note: GTK4 doesn't directly support setting window position
            // This would need to be handled via window manager hints
            info!("Window position saved as ({}, {}), but GTK4 doesn't support direct positioning", x, y);
        }
    }

    // Create grid layout
    let mut grid_layout = GridLayout::new(grid_config);

    // Create registry
    let registry = rg_sens::core::global_registry();

    // Create update manager
    let update_manager = Arc::new(UpdateManager::new());

    let mut panels = Vec::new();

    // Create panels from configuration
    let panel_configs = app_config.borrow().panels.clone();
    if panel_configs.is_empty() {
        info!("No panels in config, creating default panels");

        // Create default panels
        let default_panels = vec![
            ("panel-1", 0, 0, 1, 1, "cpu", "text"),
            ("panel-2", 1, 0, 1, 1, "cpu", "text"),
            ("panel-3", 0, 1, 2, 1, "cpu", "text"),
        ];

        // Get window defaults for panels
        let default_corner_radius = app_config.borrow().window.panel_corner_radius;
        let default_border = app_config.borrow().window.panel_border.clone();

        for (id, x, y, width, height, source_id, displayer_id) in default_panels {
            match create_panel_from_config(
                id,
                x,
                y,
                width,
                height,
                source_id,
                displayer_id,
                Default::default(),
                default_corner_radius,
                default_border.clone(),
                HashMap::new(),
                &registry,
            ) {
                Ok(panel) => {
                    grid_layout.add_panel(panel.clone());
                    panels.push(panel);
                }
                Err(e) => {
                    warn!("Failed to create default panel {}: {}", id, e);
                }
            }
        }
    } else {
        info!("Loading {} panels from config", panel_configs.len());

        for panel_config in &panel_configs {
            match create_panel_from_config(
                &panel_config.id,
                panel_config.x,
                panel_config.y,
                panel_config.width,
                panel_config.height,
                &panel_config.source,
                &panel_config.displayer,
                panel_config.background.clone(),
                panel_config.corner_radius,
                panel_config.border.clone(),
                panel_config.settings.clone(),
                &registry,
            ) {
                Ok(panel) => {
                    grid_layout.add_panel(panel.clone());
                    panels.push(panel);
                }
                Err(e) => {
                    warn!("Failed to create panel {}: {}", panel_config.id, e);
                }
            }
        }
    }

    // Create window background
    let window_background = gtk4::DrawingArea::new();
    let window_bg_config = app_config.borrow().window.background.clone();
    window_background.set_draw_func(move |_, cr, width, height| {
        use rg_sens::ui::background::render_background;
        let _ = render_background(cr, &window_bg_config, width as f64, height as f64);
    });

    // Create overlay to show background behind grid
    let window_overlay = gtk4::Overlay::new();
    window_overlay.set_child(Some(&window_background));
    window_overlay.add_overlay(&grid_layout.widget());

    // Set overlay as window content
    window.set_child(Some(&window_overlay));

    // Set initial fullscreen state from config
    if app_config.borrow().window.fullscreen_enabled {
        window.fullscreen();
    }

    // Track if configuration has changed (dirty flag)
    let config_dirty = Rc::new(RefCell::new(false));

    // Mark config as dirty when panels are moved
    let config_dirty_clone = config_dirty.clone();
    grid_layout.set_on_change(move || {
        *config_dirty_clone.borrow_mut() = true;
    });

    // === Borderless window move/resize support ===
    // Create overlay for resize handles (only visible when shift is pressed and borderless)
    let resize_overlay = gtk4::DrawingArea::new();
    resize_overlay.set_can_focus(false);
    resize_overlay.set_visible(false);
    // Don't capture pointer events - the drag gesture is on the GridLayout container
    resize_overlay.set_can_target(false);

    // Track shift state and resize zone
    let shift_pressed = Rc::new(RefCell::new(false));
    let resize_zone = Rc::new(RefCell::new(ResizeZone::None));

    // Set up draw function for resize handles
    let resize_zone_for_draw = resize_zone.clone();
    resize_overlay.set_draw_func(move |_, cr, width, height| {
        let zone = *resize_zone_for_draw.borrow();
        draw_resize_handles(cr, width as f64, height as f64, zone);
    });

    // Add resize overlay to window overlay
    window_overlay.add_overlay(&resize_overlay);

    // Set up borderless drag callback on GridLayout
    // This callback is called from GridLayout's container gesture (on the Fixed widget)
    // Having the gesture on a child widget (not on an overlay) avoids window snapping
    let shift_pressed_for_drag = shift_pressed.clone();
    let resize_zone_for_drag = resize_zone.clone();
    let app_config_for_drag = app_config.clone();
    let window_for_drag = window.clone();

    grid_layout.set_on_borderless_drag(move |gesture, x, y| {
        // Check if shift is pressed and borderless mode is enabled
        let is_shift = *shift_pressed_for_drag.borrow();
        let is_borderless = app_config_for_drag.borrow().window.borderless;
        let is_fullscreen = window_for_drag.is_fullscreen();

        if !is_shift || !is_borderless || is_fullscreen {
            return false; // Don't handle - let normal selection proceed
        }

        let zone = *resize_zone_for_drag.borrow();
        log::info!("Borderless drag on GridLayout container: ({}, {}), zone={:?}", x, y, zone);

        // Get event parameters
        let event = gesture.current_event();
        let timestamp = event.as_ref().map(|e| e.time()).unwrap_or(0);
        let device = gesture.device();
        let button = gesture.current_button() as i32;

        // Translate coordinates from grid_layout to window
        // The gesture widget is GridLayout's container (Fixed), which is inside the overlay
        let gesture_widget = match gesture.widget() {
            Some(w) => w,
            None => {
                log::info!("  -> no gesture widget");
                return false;
            }
        };
        let coords = gesture_widget.translate_coordinates(&window_for_drag, x, y);

        let (win_x, win_y) = match coords {
            Some((wx, wy)) => (wx, wy),
            None => {
                log::info!("  -> coordinate translation failed");
                return false;
            }
        };

        log::info!("  translated coords: ({}, {})", win_x, win_y);

        // Get surface from window
        use gtk4::prelude::NativeExt;
        let surface = match window_for_drag.surface() {
            Some(s) => s,
            None => {
                log::info!("  -> no surface");
                return false;
            }
        };

        let toplevel = match surface.downcast_ref::<gtk4::gdk::Toplevel>() {
            Some(t) => t,
            None => {
                log::info!("  -> not a toplevel");
                return false;
            }
        };

        let dev = match device.as_ref() {
            Some(d) => d,
            None => {
                log::info!("  -> no device");
                return false;
            }
        };

        match zone {
            ResizeZone::None | ResizeZone::Move => {
                // Use native window manager move
                log::info!("  -> Calling begin_move: button={}, x={}, y={}, ts={}", button, win_x, win_y, timestamp);
                toplevel.begin_move(dev, button, win_x, win_y, timestamp);
            }
            _ => {
                // Use native window manager resize with appropriate edge
                let edge = match zone {
                    ResizeZone::Top => gtk4::gdk::SurfaceEdge::North,
                    ResizeZone::Bottom => gtk4::gdk::SurfaceEdge::South,
                    ResizeZone::Left => gtk4::gdk::SurfaceEdge::West,
                    ResizeZone::Right => gtk4::gdk::SurfaceEdge::East,
                    ResizeZone::TopLeft => gtk4::gdk::SurfaceEdge::NorthWest,
                    ResizeZone::TopRight => gtk4::gdk::SurfaceEdge::NorthEast,
                    ResizeZone::BottomLeft => gtk4::gdk::SurfaceEdge::SouthWest,
                    ResizeZone::BottomRight => gtk4::gdk::SurfaceEdge::SouthEast,
                    _ => gtk4::gdk::SurfaceEdge::SouthEast,
                };
                log::info!("  -> Calling begin_resize: edge={:?}, button={}, x={}, y={}, ts={}", edge, button, win_x, win_y, timestamp);
                toplevel.begin_resize(edge, device.as_ref(), button, win_x, win_y, timestamp);
            }
        }

        // Claim the gesture
        gesture.set_state(gtk4::EventSequenceState::Claimed);
        true // Handled
    });

    // Wrap grid_layout in Rc<RefCell<>> for sharing across closures
    let grid_layout = Rc::new(RefCell::new(grid_layout));

    // Mark config as dirty when window is resized
    let config_dirty_clone2 = config_dirty.clone();
    window.connect_default_width_notify(move |_| {
        *config_dirty_clone2.borrow_mut() = true;
    });

    let config_dirty_clone3 = config_dirty.clone();
    window.connect_default_height_notify(move |_| {
        *config_dirty_clone3.borrow_mut() = true;
    });

    // Add double-click gesture on overlay to toggle fullscreen
    let double_click_gesture = gtk4::GestureClick::new();
    double_click_gesture.set_button(gtk4::gdk::BUTTON_PRIMARY);

    let window_for_fullscreen = window.clone();
    let app_config_for_fullscreen = app_config.clone();
    let config_dirty_for_fullscreen = config_dirty.clone();

    double_click_gesture.connect_pressed(move |_gesture, n_press, _x, _y| {
        // Only respond to double-clicks
        if n_press == 2 {
            let is_fullscreen = window_for_fullscreen.is_fullscreen();
            if is_fullscreen {
                window_for_fullscreen.unfullscreen();
                app_config_for_fullscreen.borrow_mut().window.fullscreen_enabled = false;
            } else {
                window_for_fullscreen.fullscreen();
                app_config_for_fullscreen.borrow_mut().window.fullscreen_enabled = true;
            }
            // Mark config as dirty
            *config_dirty_for_fullscreen.borrow_mut() = true;
        }
    });

    window_overlay.add_controller(double_click_gesture);

    // Key controller for tracking Shift press/release
    let key_controller_shift = gtk4::EventControllerKey::new();
    let shift_pressed_for_key = shift_pressed.clone();
    let resize_overlay_for_key = resize_overlay.clone();
    let app_config_for_shift = app_config.clone();
    let window_for_shift = window.clone();

    key_controller_shift.connect_key_pressed(move |_, key, _code, _modifiers| {
        if key == gtk4::gdk::Key::Shift_L || key == gtk4::gdk::Key::Shift_R {
            let is_borderless = app_config_for_shift.borrow().window.borderless;
            let is_fullscreen = window_for_shift.is_fullscreen();
            if is_borderless && !is_fullscreen {
                *shift_pressed_for_key.borrow_mut() = true;
                resize_overlay_for_key.set_visible(true);
                resize_overlay_for_key.queue_draw();
            }
        }
        glib::Propagation::Proceed
    });

    let shift_pressed_for_release = shift_pressed.clone();
    let resize_overlay_for_release = resize_overlay.clone();
    key_controller_shift.connect_key_released(move |_, key, _code, _modifiers| {
        if key == gtk4::gdk::Key::Shift_L || key == gtk4::gdk::Key::Shift_R {
            *shift_pressed_for_release.borrow_mut() = false;
            resize_overlay_for_release.set_visible(false);
        }
    });

    window.add_controller(key_controller_shift);

    // Motion controller for updating resize zone based on cursor position
    let motion_controller = gtk4::EventControllerMotion::new();
    let resize_zone_for_motion = resize_zone.clone();
    let resize_overlay_for_motion = resize_overlay.clone();
    let shift_pressed_for_motion = shift_pressed.clone();
    let window_for_motion = window.clone();

    motion_controller.connect_motion(move |_, x, y| {
        if !*shift_pressed_for_motion.borrow() {
            return;
        }

        let width = window_for_motion.width() as f64;
        let height = window_for_motion.height() as f64;
        let new_zone = detect_resize_zone(x, y, width, height);

        let mut zone = resize_zone_for_motion.borrow_mut();
        if *zone != new_zone {
            *zone = new_zone;
            resize_overlay_for_motion.queue_draw();
        }
    });

    window_overlay.add_controller(motion_controller);

    // Setup save-on-close confirmation
    let grid_layout_for_close = grid_layout.clone();
    let config_dirty_clone4 = config_dirty.clone();
    let app_config_for_close = app_config.clone();

    window.connect_close_request(move |window| {
        let is_dirty = *config_dirty_clone4.borrow();

        if is_dirty {
            // Show save confirmation dialog
            show_save_dialog(window, &grid_layout_for_close, &app_config_for_close);
            glib::Propagation::Stop // Prevent immediate close
        } else {
            glib::Propagation::Proceed // Close without saving
        }
    });

    // Spawn tokio runtime for update loop
    let update_manager_clone = update_manager.clone();
    std::thread::spawn(move || {
        let rt = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(e) => {
                error!("Failed to create tokio runtime: {}", e);
                return;
            }
        };
        rt.block_on(async {
            // Add all panels to update manager
            for panel in panels {
                update_manager_clone.add_panel(panel).await;
            }

            // Run update loop (base interval for checking, each panel updates at its own configured interval)
            info!("Starting update loop");
            update_manager_clone.run(Duration::from_millis(100)).await;
        });
    });

    // Add keyboard shortcut for settings (Ctrl+Comma)
    let key_controller = gtk4::EventControllerKey::new();
    let window_clone_for_settings = window.clone();
    let app_config_for_settings = app_config.clone();
    let window_bg_for_settings = window_background.clone();
    let grid_layout_for_settings = grid_layout.clone();
    let config_dirty_for_settings = config_dirty.clone();

    // Add right-click gesture for context menu
    let gesture_click = gtk4::GestureClick::new();
    gesture_click.set_button(gtk4::gdk::BUTTON_SECONDARY);

    // Clone variables for context menu
    let window_for_menu = window.clone();
    let app_config_for_menu = app_config.clone();
    let window_bg_for_menu = window_background.clone();
    let grid_layout_for_menu = grid_layout_for_settings.clone();
    let config_dirty_for_menu = config_dirty.clone();

    gesture_click.connect_pressed(move |gesture, _, x, y| {
        use gtk4::gio;
        use gtk4::PopoverMenu;

        let menu = gio::Menu::new();

        // Section 1: New panel
        let section1 = gio::Menu::new();
        section1.append(Some("New Panel"), Some("window.new-panel"));
        menu.append_section(None, &section1);

        // Section 2: Save layout
        let section2 = gio::Menu::new();
        section2.append(Some("Save Layout"), Some("window.save-layout"));
        menu.append_section(None, &section2);

        // Section 3: Save/Load from file
        let section3 = gio::Menu::new();
        section3.append(Some("Save Layout to File..."), Some("window.save-to-file"));
        section3.append(Some("Load Layout from File..."), Some("window.load-from-file"));
        menu.append_section(None, &section3);

        // Section 4: Options and Quit
        let section4 = gio::Menu::new();
        section4.append(Some("Options"), Some("window.options"));
        section4.append(Some("Quit"), Some("window.quit"));
        menu.append_section(None, &section4);

        let popover = PopoverMenu::from_model(Some(&menu));
        popover.set_parent(&window_for_menu);
        popover.set_has_arrow(false);

        // Position the popover while being aware of window and screen bounds
        // Calculate preferred position
        let mut menu_x = x as i32;
        let mut menu_y = y as i32;

        // Get window and screen dimensions for boundary checking
        if let Some(surface) = window_for_menu.surface() {
            let display = surface.display();
            // Get monitor geometry
            if let Some(monitor) = display.monitor_at_surface(&surface) {
                let monitor_geom = monitor.geometry();
                let window_width = window_for_menu.default_width();
                let window_height = window_for_menu.default_height();

                // Estimate menu size (PopoverMenu doesn't expose size before showing)
                let estimated_menu_width = 250;
                let estimated_menu_height = 300;

                // Check if menu would go off the right edge of window
                if menu_x + estimated_menu_width > window_width {
                    menu_x = (window_width - estimated_menu_width).max(0);
                }

                // Check if menu would go off the bottom edge of window
                if menu_y + estimated_menu_height > window_height {
                    menu_y = (window_height - estimated_menu_height).max(0);
                }

                // Ensure menu stays within screen bounds too
                if menu_x + estimated_menu_width > monitor_geom.width() {
                    menu_x = (monitor_geom.width() - estimated_menu_width).max(0);
                }
                if menu_y + estimated_menu_height > monitor_geom.height() {
                    menu_y = (monitor_geom.height() - estimated_menu_height).max(0);
                }
            }
        }

        popover.set_pointing_to(Some(&gtk4::gdk::Rectangle::new(
            menu_x,
            menu_y,
            1,
            1,
        )));

        // Setup action group
        let action_group = gio::SimpleActionGroup::new();

        // New panel action (with mouse coordinates)
        let window_for_new = window_for_menu.clone();
        let grid_layout_for_new = grid_layout_for_menu.clone();
        let config_dirty_for_new = config_dirty_for_menu.clone();
        let app_config_for_new = app_config_for_menu.clone();
        let mouse_x = x;
        let mouse_y = y;
        let new_panel_action = gio::SimpleAction::new("new-panel", None);
        new_panel_action.connect_activate(move |_, _| {
            info!("New panel requested at ({}, {})", mouse_x, mouse_y);
            show_new_panel_dialog(
                &window_for_new,
                &grid_layout_for_new,
                &config_dirty_for_new,
                &app_config_for_new,
                Some((mouse_x, mouse_y)),
            );
        });
        action_group.add_action(&new_panel_action);

        // Save layout action
        let grid_layout_for_save = grid_layout_for_menu.clone();
        let app_config_for_save = app_config_for_menu.clone();
        let window_for_save = window_for_menu.clone();
        let config_dirty_for_save = config_dirty_for_menu.clone();
        let save_layout_action = gio::SimpleAction::new("save-layout", None);
        save_layout_action.connect_activate(move |_, _| {
            info!("Save layout requested");
            // Get current panels from GridLayout (not the stale clone)
            let current_panels = grid_layout_for_save.borrow().get_panels();
            save_config_with_app_config(&app_config_for_save.borrow(), &window_for_save, &current_panels);
            *config_dirty_for_save.borrow_mut() = false;
        });
        action_group.add_action(&save_layout_action);

        // Save to file action
        let window_for_save_file = window_for_menu.clone();
        let grid_layout_for_save_file = grid_layout_for_menu.clone();
        let app_config_for_save_file = app_config_for_menu.clone();
        let config_dirty_for_save_file = config_dirty_for_menu.clone();
        let save_to_file_action = gio::SimpleAction::new("save-to-file", None);
        save_to_file_action.connect_activate(move |_, _| {
            info!("Save to file requested");
            let window = window_for_save_file.clone();
            let grid_layout = grid_layout_for_save_file.clone();
            let app_config = app_config_for_save_file.clone();
            let config_dirty = config_dirty_for_save_file.clone();

            gtk4::glib::MainContext::default().spawn_local(async move {
                use gtk4::FileDialog;

                let file_dialog = FileDialog::builder()
                    .title("Save Layout to File")
                    .modal(true)
                    .build();

                if let Ok(file) = file_dialog.save_future(Some(&window)).await {
                    if let Some(path) = file.path() {
                        info!("Saving layout to {:?}", path);

                        // Get current panels
                        let current_panels = grid_layout.borrow().get_panels();

                        // Create config
                        let (width, height) = (window.default_width(), window.default_height());
                        let panel_configs: Vec<PanelConfig> = current_panels
                            .iter()
                            .filter_map(|panel| {
                                if let Ok(panel_guard) = panel.try_read() {
                                    Some(PanelConfig {
                                        id: panel_guard.id.clone(),
                                        x: panel_guard.geometry.x,
                                        y: panel_guard.geometry.y,
                                        width: panel_guard.geometry.width,
                                        height: panel_guard.geometry.height,
                                        source: panel_guard.source.metadata().id.clone(),
                                        displayer: panel_guard.displayer.id().to_string(),
                                        background: panel_guard.background.clone(),
                                        corner_radius: panel_guard.corner_radius,
                                        border: panel_guard.border.clone(),
                                        settings: panel_guard.config.clone(),
                                    })
                                } else {
                                    None
                                }
                            })
                            .collect();

                        let mut config = app_config.borrow().clone();
                        config.window.width = width;
                        config.window.height = height;
                        config.panels = panel_configs;

                        match config.save_to_path(&path) {
                            Ok(()) => {
                                info!("Layout saved successfully to {:?}", path);
                                *config_dirty.borrow_mut() = false;
                            }
                            Err(e) => {
                                warn!("Failed to save layout: {}", e);
                            }
                        }
                    }
                }
            });
        });
        action_group.add_action(&save_to_file_action);

        // Load from file action
        let window_for_load_file = window_for_menu.clone();
        let grid_layout_for_load_file = grid_layout_for_menu.clone();
        let app_config_for_load_file = app_config_for_menu.clone();
        let config_dirty_for_load_file = config_dirty_for_menu.clone();
        let load_from_file_action = gio::SimpleAction::new("load-from-file", None);
        load_from_file_action.connect_activate(move |_, _| {
            info!("Load from file requested");
            let window = window_for_load_file.clone();
            let grid_layout = grid_layout_for_load_file.clone();
            let app_config = app_config_for_load_file.clone();
            let config_dirty = config_dirty_for_load_file.clone();

            gtk4::glib::MainContext::default().spawn_local(async move {
                use gtk4::FileDialog;

                let file_dialog = FileDialog::builder()
                    .title("Load Layout from File")
                    .modal(true)
                    .build();

                if let Ok(file) = file_dialog.open_future(Some(&window)).await {
                    if let Some(path) = file.path() {
                        info!("Loading layout from {:?}", path);

                        match AppConfig::load_from_path(&path) {
                            Ok(loaded_config) => {
                                info!("Layout loaded successfully from {:?}", path);

                                // Update app config
                                *app_config.borrow_mut() = loaded_config.clone();

                                // Clear current panels
                                grid_layout.borrow_mut().clear_all_panels();

                                // Create panels from loaded config
                                let registry = rg_sens::core::global_registry();
                                for panel_config in &loaded_config.panels {
                                    match create_panel_from_config(
                                        &panel_config.id,
                                        panel_config.x,
                                        panel_config.y,
                                        panel_config.width,
                                        panel_config.height,
                                        &panel_config.source,
                                        &panel_config.displayer,
                                        panel_config.background.clone(),
                                        panel_config.corner_radius,
                                        panel_config.border.clone(),
                                        panel_config.settings.clone(),
                                        &registry,
                                    ) {
                                        Ok(panel) => {
                                            grid_layout.borrow_mut().add_panel(panel);
                                        }
                                        Err(e) => {
                                            warn!("Failed to create panel {}: {}", panel_config.id, e);
                                        }
                                    }
                                }

                                // Update grid configuration
                                grid_layout.borrow_mut().update_grid_size(
                                    loaded_config.grid.cell_width,
                                    loaded_config.grid.cell_height,
                                    loaded_config.grid.spacing,
                                );

                                // Mark config as clean since we just loaded
                                *config_dirty.borrow_mut() = false;
                            }
                            Err(e) => {
                                warn!("Failed to load layout: {}", e);
                            }
                        }
                    }
                }
            });
        });
        action_group.add_action(&load_from_file_action);

        // Options action
        let window_for_options = window_for_menu.clone();
        let app_config_for_options = app_config_for_menu.clone();
        let window_bg_for_options = window_bg_for_menu.clone();
        let grid_layout_for_options = grid_layout_for_menu.clone();
        let config_dirty_for_options = config_dirty_for_menu.clone();
        let options_action = gio::SimpleAction::new("options", None);
        options_action.connect_activate(move |_, _| {
            show_window_settings_dialog(
                &window_for_options,
                &app_config_for_options,
                &window_bg_for_options,
                &grid_layout_for_options,
                &config_dirty_for_options,
            );
        });
        action_group.add_action(&options_action);

        // Quit action
        let window_for_quit = window_for_menu.clone();
        let quit_action = gio::SimpleAction::new("quit", None);
        quit_action.connect_activate(move |_, _| {
            window_for_quit.close();
        });
        action_group.add_action(&quit_action);

        window_for_menu.insert_action_group("window", Some(&action_group));

        popover.popup();
        gesture.set_state(gtk4::EventSequenceState::Claimed);
    });

    window.add_controller(gesture_click);

    // Clone for closure
    let grid_layout_for_key = grid_layout_for_settings.clone();

    key_controller.connect_key_pressed(move |_, key, _code, modifiers| {
        if modifiers.contains(gtk4::gdk::ModifierType::CONTROL_MASK)
            && key == gtk4::gdk::Key::comma {
            show_window_settings_dialog(
                &window_clone_for_settings,
                &app_config_for_settings,
                &window_bg_for_settings,
                &grid_layout_for_key,
                &config_dirty_for_settings,
            );
            glib::Propagation::Stop
        } else {
            glib::Propagation::Proceed
        }
    });

    window.add_controller(key_controller);

    window.present();
}

/// Show save confirmation dialog on close
fn show_save_dialog(window: &ApplicationWindow, grid_layout: &Rc<RefCell<GridLayout>>, app_config: &Rc<RefCell<AppConfig>>) {
    use gtk4::AlertDialog;

    let dialog = AlertDialog::builder()
        .message("Save configuration before closing?")
        .detail("Your panel layout and window size have been modified.")
        .modal(true)
        .buttons(vec!["Don't Save", "Cancel", "Save"])
        .default_button(2) // "Save" button
        .cancel_button(1) // "Cancel" button
        .build();

    let window_clone = window.clone();
    let grid_layout_clone = grid_layout.clone();
    let app_config_clone = app_config.clone();

    dialog.choose(Some(window), gtk4::gio::Cancellable::NONE, move |response| {
        match response {
            Ok(2) => {
                // Save button (index 2)
                info!("User chose to save configuration");
                // Get current panels from GridLayout (not a stale clone)
                let current_panels = grid_layout_clone.borrow().get_panels();
                save_config_with_app_config(&app_config_clone.borrow(), &window_clone, &current_panels);
                window_clone.destroy(); // Use destroy to bypass close handler
            }
            Ok(0) => {
                // Don't Save button (index 0)
                info!("User chose not to save configuration");
                window_clone.destroy(); // Use destroy to bypass close handler
            }
            Ok(1) | Err(_) => {
                // Cancel button (index 1) or dialog dismissed
                info!("User cancelled close operation");
            }
            _ => {}
        }
    });
}

/// Save current configuration to disk
fn save_config_with_app_config(app_config: &AppConfig, window: &ApplicationWindow, panels: &[Arc<RwLock<Panel>>]) {
    // Get window dimensions
    let (width, height) = (window.default_width(), window.default_height());

    // Convert panels to PanelConfig
    let panel_configs: Vec<PanelConfig> = panels
        .iter()
        .filter_map(|panel| {
            if let Ok(panel_guard) = panel.try_read() {
                Some(PanelConfig {
                    id: panel_guard.id.clone(),
                    x: panel_guard.geometry.x,
                    y: panel_guard.geometry.y,
                    width: panel_guard.geometry.width,
                    height: panel_guard.geometry.height,
                    source: panel_guard.source.metadata().id.clone(),
                    displayer: panel_guard.displayer.id().to_string(),
                    background: panel_guard.background.clone(),
                    corner_radius: panel_guard.corner_radius,
                    border: panel_guard.border.clone(),
                    settings: panel_guard.config.clone(),
                })
            } else {
                None
            }
        })
        .collect();

    // Create config with all settings
    let mut config = app_config.clone();
    config.window.width = width;
    config.window.height = height;
    config.window.x = None; // GTK4 doesn't provide window position
    config.window.y = None;
    config.panels = panel_configs;

    // Save to disk
    match config.save() {
        Ok(()) => {
            info!("Configuration saved successfully");
        }
        Err(e) => {
            warn!("Failed to save configuration: {}", e);
        }
    }
}

/// Show window settings dialog
fn show_window_settings_dialog(
    parent_window: &ApplicationWindow,
    app_config: &Rc<RefCell<AppConfig>>,
    window_background: &gtk4::DrawingArea,
    grid_layout: &Rc<RefCell<GridLayout>>,
    config_dirty: &Rc<RefCell<bool>>,
) {
    use gtk4::{Box as GtkBox, Button, CheckButton, DropDown, Label, Notebook, Orientation, SpinButton, StringList, Window};
    use rg_sens::ui::BackgroundConfigWidget;

    let dialog = Window::builder()
        .title("Window Settings")
        .transient_for(parent_window)
        .modal(false)
        .default_width(550)
        .default_height(650)
        .build();

    let vbox = GtkBox::new(Orientation::Vertical, 12);
    vbox.set_margin_start(12);
    vbox.set_margin_end(12);
    vbox.set_margin_top(12);
    vbox.set_margin_bottom(12);

    // Create notebook for tabs
    let notebook = Notebook::new();
    notebook.set_vexpand(true);

    // === Tab 1: Grid Settings ===
    let grid_tab_box = GtkBox::new(Orientation::Vertical, 12);
    grid_tab_box.set_margin_start(12);
    grid_tab_box.set_margin_end(12);
    grid_tab_box.set_margin_top(12);
    grid_tab_box.set_margin_bottom(12);

    // Cell Width
    let cell_width_box = GtkBox::new(Orientation::Horizontal, 6);
    cell_width_box.append(&Label::new(Some("Cell Width:")));
    let cell_width_spin = SpinButton::with_range(10.0, 1000.0, 10.0);
    cell_width_spin.set_value(app_config.borrow().grid.cell_width as f64);
    cell_width_spin.set_hexpand(true);
    cell_width_box.append(&cell_width_spin);
    grid_tab_box.append(&cell_width_box);

    // Cell Height
    let cell_height_box = GtkBox::new(Orientation::Horizontal, 6);
    cell_height_box.append(&Label::new(Some("Cell Height:")));
    let cell_height_spin = SpinButton::with_range(10.0, 1000.0, 10.0);
    cell_height_spin.set_value(app_config.borrow().grid.cell_height as f64);
    cell_height_spin.set_hexpand(true);
    cell_height_box.append(&cell_height_spin);
    grid_tab_box.append(&cell_height_box);

    // Spacing
    let spacing_box = GtkBox::new(Orientation::Horizontal, 6);
    spacing_box.append(&Label::new(Some("Spacing:")));
    let spacing_spin = SpinButton::with_range(0.0, 50.0, 1.0);
    spacing_spin.set_value(app_config.borrow().grid.spacing as f64);
    spacing_spin.set_hexpand(true);
    spacing_box.append(&spacing_spin);
    grid_tab_box.append(&spacing_box);

    notebook.append_page(&grid_tab_box, Some(&Label::new(Some("Grid"))));

    // === Tab 2: Background ===
    let bg_tab_box = GtkBox::new(Orientation::Vertical, 12);
    bg_tab_box.set_margin_start(12);
    bg_tab_box.set_margin_end(12);
    bg_tab_box.set_margin_top(12);
    bg_tab_box.set_margin_bottom(12);

    let background_widget = BackgroundConfigWidget::new();
    background_widget.set_config(app_config.borrow().window.background.clone());
    bg_tab_box.append(background_widget.widget());

    let background_widget = Rc::new(background_widget);

    notebook.append_page(&bg_tab_box, Some(&Label::new(Some("Background"))));

    // === Tab 3: Panel Defaults ===
    let panel_tab_box = GtkBox::new(Orientation::Vertical, 12);
    panel_tab_box.set_margin_start(12);
    panel_tab_box.set_margin_end(12);
    panel_tab_box.set_margin_top(12);
    panel_tab_box.set_margin_bottom(12);

    // Corner radius
    let corner_radius_label = Label::new(Some("Panel Corner Radius"));
    corner_radius_label.add_css_class("heading");
    panel_tab_box.append(&corner_radius_label);

    let corner_radius_box = GtkBox::new(Orientation::Horizontal, 6);
    corner_radius_box.set_margin_start(12);
    corner_radius_box.append(&Label::new(Some("Radius:")));
    let corner_radius_spin = SpinButton::with_range(0.0, 50.0, 1.0);
    corner_radius_spin.set_value(app_config.borrow().window.panel_corner_radius);
    corner_radius_spin.set_hexpand(true);
    corner_radius_box.append(&corner_radius_spin);
    panel_tab_box.append(&corner_radius_box);

    // Border section
    let border_label = Label::new(Some("Panel Border"));
    border_label.add_css_class("heading");
    border_label.set_margin_top(12);
    panel_tab_box.append(&border_label);

    let border_enabled_check = CheckButton::with_label("Show Border on New Panels");
    border_enabled_check.set_active(app_config.borrow().window.panel_border.enabled);
    border_enabled_check.set_margin_start(12);
    panel_tab_box.append(&border_enabled_check);

    let border_width_box = GtkBox::new(Orientation::Horizontal, 6);
    border_width_box.set_margin_start(12);
    border_width_box.append(&Label::new(Some("Width:")));
    let border_width_spin = SpinButton::with_range(0.5, 10.0, 0.5);
    border_width_spin.set_value(app_config.borrow().window.panel_border.width);
    border_width_spin.set_hexpand(true);
    border_width_box.append(&border_width_spin);
    panel_tab_box.append(&border_width_box);

    let border_color_btn = Button::with_label("Border Color");
    border_color_btn.set_margin_start(12);
    panel_tab_box.append(&border_color_btn);

    // Store border color in a shared Rc<RefCell>
    let border_color = Rc::new(RefCell::new(app_config.borrow().window.panel_border.color));

    // Border color button handler
    {
        let border_color_clone = border_color.clone();
        let dialog_clone = dialog.clone();
        border_color_btn.connect_clicked(move |_| {
            let current_color = *border_color_clone.borrow();
            let window_opt = dialog_clone.clone().upcast::<Window>();
            let border_color_clone2 = border_color_clone.clone();

            gtk4::glib::MainContext::default().spawn_local(async move {
                if let Some(new_color) = rg_sens::ui::ColorPickerDialog::pick_color(Some(&window_opt), current_color).await {
                    *border_color_clone2.borrow_mut() = new_color;
                }
            });
        });
    }

    notebook.append_page(&panel_tab_box, Some(&Label::new(Some("Panel Defaults"))));

    // === Tab 4: Window Mode ===
    let window_mode_tab_box = GtkBox::new(Orientation::Vertical, 12);
    window_mode_tab_box.set_margin_start(12);
    window_mode_tab_box.set_margin_end(12);
    window_mode_tab_box.set_margin_top(12);
    window_mode_tab_box.set_margin_bottom(12);

    // Fullscreen section
    let fullscreen_label = Label::new(Some("Fullscreen"));
    fullscreen_label.add_css_class("heading");
    fullscreen_label.set_halign(gtk4::Align::Start);
    window_mode_tab_box.append(&fullscreen_label);

    // Fullscreen enabled
    let fullscreen_enabled_check = CheckButton::with_label("Start in fullscreen mode");
    fullscreen_enabled_check.set_active(app_config.borrow().window.fullscreen_enabled);
    fullscreen_enabled_check.set_margin_start(12);
    window_mode_tab_box.append(&fullscreen_enabled_check);

    // Fullscreen monitor selection
    let monitor_box = GtkBox::new(Orientation::Horizontal, 6);
    monitor_box.set_margin_start(12);
    monitor_box.append(&Label::new(Some("Monitor:")));

    // Get list of available monitors with their names
    let monitor_names = if let Some(display) = gtk4::gdk::Display::default() {
        let n_monitors = display.monitors().n_items();
        (0..n_monitors)
            .filter_map(|i| {
                display.monitors().item(i)
                    .and_then(|obj| obj.downcast::<gtk4::gdk::Monitor>().ok())
                    .map(|monitor| {
                        // Try to get connector name (e.g., "HDMI-1", "DP-1")
                        let connector = monitor.connector()
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| format!("Monitor {}", i));

                        // Get model name if available
                        let model = monitor.model()
                            .map(|s| s.to_string());

                        // Combine connector and model for a descriptive name
                        match model {
                            Some(m) if !m.is_empty() => format!("{} ({})", connector, m),
                            _ => connector,
                        }
                    })
            })
            .collect::<Vec<_>>()
    } else {
        vec!["Monitor 0".to_string()]
    };

    let mut monitor_strings: Vec<String> = vec!["Current Monitor".to_string()];
    monitor_strings.extend(monitor_names);

    let monitor_string_refs: Vec<&str> = monitor_strings.iter().map(|s| s.as_str()).collect();
    let monitor_list = StringList::new(&monitor_string_refs);
    let monitor_dropdown = DropDown::new(Some(monitor_list), Option::<gtk4::Expression>::None);
    monitor_dropdown.set_hexpand(true);

    // Set selected monitor from config
    let selected_idx = match app_config.borrow().window.fullscreen_monitor {
        None => 0, // "Current Monitor"
        Some(idx) => (idx + 1) as u32, // Offset by 1 for "Current Monitor" option
    };
    monitor_dropdown.set_selected(selected_idx);
    monitor_box.append(&monitor_dropdown);
    window_mode_tab_box.append(&monitor_box);

    // Help text for fullscreen
    let fullscreen_help_label = Label::new(Some("Tip: Double-click the window background to toggle fullscreen"));
    fullscreen_help_label.set_halign(gtk4::Align::Start);
    fullscreen_help_label.set_margin_start(12);
    fullscreen_help_label.set_margin_top(6);
    fullscreen_help_label.add_css_class("dim-label");
    window_mode_tab_box.append(&fullscreen_help_label);

    // Borderless section
    let borderless_label = Label::new(Some("Borderless Mode"));
    borderless_label.add_css_class("heading");
    borderless_label.set_halign(gtk4::Align::Start);
    borderless_label.set_margin_top(18);
    window_mode_tab_box.append(&borderless_label);

    // Borderless enabled
    let borderless_check = CheckButton::with_label("Remove window decorations (title bar, borders)");
    borderless_check.set_active(app_config.borrow().window.borderless);
    borderless_check.set_margin_start(12);
    window_mode_tab_box.append(&borderless_check);

    // Info box for borderless mode
    let borderless_info_frame = gtk4::Frame::new(None);
    borderless_info_frame.set_margin_start(12);
    borderless_info_frame.set_margin_top(6);
    borderless_info_frame.add_css_class("view");

    let borderless_info_box = GtkBox::new(Orientation::Horizontal, 8);
    borderless_info_box.set_margin_start(8);
    borderless_info_box.set_margin_end(8);
    borderless_info_box.set_margin_top(8);
    borderless_info_box.set_margin_bottom(8);

    let info_icon = Label::new(Some("\u{2139}"));  // ℹ info symbol
    info_icon.add_css_class("dim-label");
    borderless_info_box.append(&info_icon);

    let borderless_info_label = Label::new(Some(
        "When borderless mode is active, hold Shift and drag:\n\
         \u{2022} Drag window edges to resize\n\
         \u{2022} Drag center area to move window"
    ));
    borderless_info_label.set_halign(gtk4::Align::Start);
    borderless_info_label.set_wrap(true);
    borderless_info_label.add_css_class("dim-label");
    borderless_info_box.append(&borderless_info_label);

    borderless_info_frame.set_child(Some(&borderless_info_box));

    // Show/hide info based on checkbox state
    borderless_info_frame.set_visible(borderless_check.is_active());
    let borderless_info_frame_clone = borderless_info_frame.clone();
    borderless_check.connect_toggled(move |check| {
        borderless_info_frame_clone.set_visible(check.is_active());
    });

    window_mode_tab_box.append(&borderless_info_frame);

    notebook.append_page(&window_mode_tab_box, Some(&Label::new(Some("Window Mode"))));

    vbox.append(&notebook);

    // Buttons
    let button_box = GtkBox::new(Orientation::Horizontal, 6);
    button_box.set_halign(gtk4::Align::End);
    button_box.set_margin_top(12);

    let cancel_button = Button::with_label("Cancel");
    let apply_button = Button::with_label("Apply");
    let accept_button = Button::with_label("Accept");
    accept_button.add_css_class("suggested-action");

    let dialog_clone = dialog.clone();
    cancel_button.connect_clicked(move |_| {
        dialog_clone.close();
    });

    // Apply logic
    let app_config_clone = app_config.clone();
    let background_widget_clone = background_widget.clone();
    let window_background_clone = window_background.clone();
    let grid_layout_clone = grid_layout.clone();
    let config_dirty_clone = config_dirty.clone();
    let corner_radius_spin_clone = corner_radius_spin.clone();
    let border_enabled_check_clone = border_enabled_check.clone();
    let border_width_spin_clone = border_width_spin.clone();
    let border_color_clone = border_color.clone();
    let fullscreen_enabled_check_clone = fullscreen_enabled_check.clone();
    let monitor_dropdown_clone = monitor_dropdown.clone();
    let borderless_check_clone = borderless_check.clone();
    let parent_window_clone = parent_window.clone();

    let apply_changes = Rc::new(move || {
        let new_background = background_widget_clone.get_config();
        let new_cell_width = cell_width_spin.value() as i32;
        let new_cell_height = cell_height_spin.value() as i32;
        let new_spacing = spacing_spin.value() as i32;

        // Get fullscreen settings
        let fullscreen_enabled = fullscreen_enabled_check_clone.is_active();
        let fullscreen_monitor = {
            let selected = monitor_dropdown_clone.selected();
            if selected == 0 {
                None // "Current Monitor"
            } else {
                Some((selected - 1) as i32) // Offset by 1 for "Current Monitor" option
            }
        };

        // Get borderless setting
        let borderless = borderless_check_clone.is_active();

        // Update app config
        let mut cfg = app_config_clone.borrow_mut();
        cfg.window.background = new_background.clone();
        cfg.grid.cell_width = new_cell_width;
        cfg.grid.cell_height = new_cell_height;
        cfg.grid.spacing = new_spacing;
        cfg.window.panel_corner_radius = corner_radius_spin_clone.value();
        cfg.window.panel_border.enabled = border_enabled_check_clone.is_active();
        cfg.window.panel_border.width = border_width_spin_clone.value();
        cfg.window.panel_border.color = *border_color_clone.borrow();
        cfg.window.fullscreen_enabled = fullscreen_enabled;
        cfg.window.fullscreen_monitor = fullscreen_monitor;
        cfg.window.borderless = borderless;
        drop(cfg);

        // Apply borderless state to parent window
        parent_window_clone.set_decorated(!borderless);

        // Apply fullscreen state to parent window
        if fullscreen_enabled {
            if let Some(monitor) = fullscreen_monitor {
                // Fullscreen on specific monitor
                if let Some(display) = gtk4::gdk::Display::default() {
                    if let Some(mon) = display.monitors().item(monitor as u32) {
                        if let Ok(monitor) = mon.downcast::<gtk4::gdk::Monitor>() {
                            parent_window_clone.fullscreen_on_monitor(&monitor);
                        }
                    }
                }
            } else {
                // Fullscreen on current monitor
                parent_window_clone.fullscreen();
            }
        } else {
            parent_window_clone.unfullscreen();
        }

        // Update window background rendering
        let bg_config = new_background;
        window_background_clone.set_draw_func(move |_, cr, width, height| {
            use rg_sens::ui::background::render_background;
            let _ = render_background(cr, &bg_config, width as f64, height as f64);
        });
        window_background_clone.queue_draw();

        // Update grid layout
        grid_layout_clone.borrow_mut().update_grid_size(new_cell_width, new_cell_height, new_spacing);

        // Mark config as dirty
        *config_dirty_clone.borrow_mut() = true;

        info!("Window settings applied");
    });

    // Apply button
    let apply_changes_clone = apply_changes.clone();
    apply_button.connect_clicked(move |_| {
        apply_changes_clone();
    });

    // Accept button
    let apply_changes_clone2 = apply_changes.clone();
    let dialog_clone2 = dialog.clone();
    accept_button.connect_clicked(move |_| {
        apply_changes_clone2();
        dialog_clone2.close();
    });

    button_box.append(&cancel_button);
    button_box.append(&apply_button);
    button_box.append(&accept_button);

    vbox.append(&button_box);

    dialog.set_child(Some(&vbox));
    dialog.present();
}

/// Create a panel from configuration parameters
fn create_panel_from_config(
    id: &str,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    source_id: &str,
    displayer_id: &str,
    background: rg_sens::ui::background::BackgroundConfig,
    corner_radius: f64,
    border: rg_sens::core::PanelBorderConfig,
    settings: HashMap<String, serde_json::Value>,
    registry: &rg_sens::core::Registry,
) -> anyhow::Result<Arc<RwLock<Panel>>> {
    // Create source and displayer
    let source = registry.create_source(source_id)?;
    let displayer = registry.create_displayer(displayer_id)?;

    // Create panel
    let mut panel = Panel::new(
        id.to_string(),
        PanelGeometry {
            x,
            y,
            width,
            height,
        },
        source,
        displayer,
    );

    // Set background, corner radius, and border
    panel.background = background;
    panel.corner_radius = corner_radius;
    panel.border = border;

    // Apply settings if provided
    if !settings.is_empty() {
        panel.apply_config(settings)?;
    }

    Ok(Arc::new(RwLock::new(panel)))
}

/// Show dialog to create a new panel
fn show_new_panel_dialog(
    window: &ApplicationWindow,
    grid_layout: &Rc<RefCell<GridLayout>>,
    config_dirty: &Rc<RefCell<bool>>,
    app_config: &Rc<RefCell<AppConfig>>,
    mouse_coords: Option<(f64, f64)>,
) {
    use gtk4::{Adjustment, Box as GtkBox, Button, DropDown, Label, Orientation, SpinButton, StringList, Window};

    let dialog = Window::builder()
        .title("New Panel")
        .transient_for(window)
        .modal(false)
        .default_width(400)
        .default_height(350)
        .resizable(false)
        .build();

    // Content area
    let vbox = GtkBox::new(Orientation::Vertical, 12);
    vbox.set_margin_start(12);
    vbox.set_margin_end(12);
    vbox.set_margin_top(12);
    vbox.set_margin_bottom(12);
    dialog.set_child(Some(&vbox));

    // Position section
    let pos_label = Label::new(Some("Position:"));
    pos_label.set_halign(gtk4::Align::Start);
    vbox.append(&pos_label);

    // Calculate grid coordinates from mouse position if provided
    let (default_x, default_y) = if let Some((mouse_x, mouse_y)) = mouse_coords {
        let cfg = app_config.borrow();
        let cell_width = cfg.grid.cell_width as f64;
        let cell_height = cfg.grid.cell_height as f64;
        let spacing = cfg.grid.spacing as f64;

        // Calculate grid cell coordinates
        let grid_x = (mouse_x / (cell_width + spacing)).floor().max(0.0);
        let grid_y = (mouse_y / (cell_height + spacing)).floor().max(0.0);

        (grid_x, grid_y)
    } else {
        (0.0, 0.0)
    };

    let pos_box = GtkBox::new(Orientation::Horizontal, 6);
    pos_box.append(&Label::new(Some("X:")));
    let x_adj = Adjustment::new(default_x, 0.0, 100.0, 1.0, 5.0, 0.0);
    let x_spin = SpinButton::new(Some(&x_adj), 1.0, 0);
    x_spin.set_hexpand(true);
    pos_box.append(&x_spin);

    pos_box.append(&Label::new(Some("Y:")));
    let y_adj = Adjustment::new(default_y, 0.0, 100.0, 1.0, 5.0, 0.0);
    let y_spin = SpinButton::new(Some(&y_adj), 1.0, 0);
    y_spin.set_hexpand(true);
    pos_box.append(&y_spin);
    vbox.append(&pos_box);

    // Size section
    let size_label = Label::new(Some("Size:"));
    size_label.set_halign(gtk4::Align::Start);
    vbox.append(&size_label);

    let size_box = GtkBox::new(Orientation::Horizontal, 6);
    size_box.append(&Label::new(Some("Width:")));
    let width_adj = Adjustment::new(4.0, 1.0, 50.0, 1.0, 5.0, 0.0);
    let width_spin = SpinButton::new(Some(&width_adj), 1.0, 0);
    width_spin.set_hexpand(true);
    size_box.append(&width_spin);

    size_box.append(&Label::new(Some("Height:")));
    let height_adj = Adjustment::new(2.0, 1.0, 50.0, 1.0, 5.0, 0.0);
    let height_spin = SpinButton::new(Some(&height_adj), 1.0, 0);
    height_spin.set_hexpand(true);
    size_box.append(&height_spin);
    vbox.append(&size_box);

    // Data Source
    let source_label = Label::new(Some("Data Source:"));
    source_label.set_halign(gtk4::Align::Start);
    vbox.append(&source_label);

    let registry = rg_sens::core::global_registry();
    let source_ids = registry.list_sources();
    let source_strings: Vec<&str> = source_ids.iter().map(|s| s.as_str()).collect();
    let source_list = StringList::new(&source_strings);
    let source_combo = DropDown::new(Some(source_list), Option::<gtk4::Expression>::None);
    source_combo.set_selected(0);
    vbox.append(&source_combo);

    // Displayer
    let displayer_label = Label::new(Some("Displayer:"));
    displayer_label.set_halign(gtk4::Align::Start);
    vbox.append(&displayer_label);

    let displayer_ids = registry.list_displayers();
    let displayer_strings: Vec<&str> = displayer_ids.iter().map(|d| d.as_str()).collect();
    let displayer_list = StringList::new(&displayer_strings);
    let displayer_combo = DropDown::new(Some(displayer_list), Option::<gtk4::Expression>::None);
    displayer_combo.set_selected(0);
    vbox.append(&displayer_combo);

    // Buttons
    let button_box = GtkBox::new(Orientation::Horizontal, 6);
    button_box.set_halign(gtk4::Align::End);
    button_box.set_margin_top(12);

    let cancel_button = Button::with_label("Cancel");
    let ok_button = Button::with_label("Create");
    ok_button.add_css_class("suggested-action");

    button_box.append(&cancel_button);
    button_box.append(&ok_button);
    vbox.append(&button_box);

    // Cancel handler
    let dialog_clone = dialog.clone();
    cancel_button.connect_clicked(move |_| {
        dialog_clone.destroy();
    });

    // OK handler
    let dialog_clone = dialog.clone();
    let grid_layout = grid_layout.clone();
    let config_dirty = config_dirty.clone();
    let app_config = app_config.clone();
    ok_button.connect_clicked(move |_| {
        let x = x_spin.value() as u32;
        let y = y_spin.value() as u32;
        let width = width_spin.value() as u32;
        let height = height_spin.value() as u32;

        let source_id = &source_ids[source_combo.selected() as usize];
        let displayer_id = &displayer_ids[displayer_combo.selected() as usize];

        // Generate unique ID
        let id = format!("panel_{}", uuid::Uuid::new_v4());

        info!("Creating new panel: id={}, pos=({},{}), size={}x{}, source={}, displayer={}",
              id, x, y, width, height, source_id, displayer_id);

        // Create panel with default background and window defaults for corner_radius and border
        let background = rg_sens::ui::background::BackgroundConfig::default();
        let corner_radius = app_config.borrow().window.panel_corner_radius;
        let border = app_config.borrow().window.panel_border.clone();
        let settings = HashMap::new();

        match create_panel_from_config(
            &id,
            x,
            y,
            width,
            height,
            source_id,
            displayer_id,
            background,
            corner_radius,
            border,
            settings,
            &registry,
        ) {
            Ok(panel) => {
                // Add to grid (grid_layout maintains its own panels list)
                grid_layout.borrow_mut().add_panel(panel.clone());

                // Mark config as dirty
                *config_dirty.borrow_mut() = true;

                info!("New panel created successfully");
                dialog_clone.destroy();
            }
            Err(e) => {
                warn!("Failed to create panel: {}", e);
                // Could show error dialog here
            }
        }
    });

    dialog.present();
}

/// Load CSS styling for the application
fn load_css() {
    let provider = CssProvider::new();
    provider.load_from_data(
        "
        frame {
            border: none;
            padding: 0;
            margin: 0;
            border-radius: 0;
        }

        overlay {
            border-radius: 0;
        }

        drawingarea {
            border-radius: 0;
        }

        .selected {
            border: 3px solid #00ff00;
        }

        .transparent-background {
            background: transparent;
            background-color: transparent;
        }
        ",
    );

    gtk4::style_context_add_provider_for_display(
        &Display::default().expect("Could not connect to a display."),
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

// === Borderless window resize support ===

/// Resize zones for borderless window
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResizeZone {
    None,
    Move,
    Top,
    Bottom,
    Left,
    Right,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

/// Width of the resize edge detection area
const RESIZE_EDGE_WIDTH: f64 = 12.0;

/// Detect which resize zone the cursor is in
fn detect_resize_zone(x: f64, y: f64, width: f64, height: f64) -> ResizeZone {
    let on_left = x < RESIZE_EDGE_WIDTH;
    let on_right = x > width - RESIZE_EDGE_WIDTH;
    let on_top = y < RESIZE_EDGE_WIDTH;
    let on_bottom = y > height - RESIZE_EDGE_WIDTH;

    match (on_left, on_right, on_top, on_bottom) {
        (true, false, true, false) => ResizeZone::TopLeft,
        (false, true, true, false) => ResizeZone::TopRight,
        (true, false, false, true) => ResizeZone::BottomLeft,
        (false, true, false, true) => ResizeZone::BottomRight,
        (true, false, false, false) => ResizeZone::Left,
        (false, true, false, false) => ResizeZone::Right,
        (false, false, true, false) => ResizeZone::Top,
        (false, false, false, true) => ResizeZone::Bottom,
        _ => ResizeZone::Move,
    }
}

/// Draw resize handles overlay
fn draw_resize_handles(cr: &gtk4::cairo::Context, width: f64, height: f64, active_zone: ResizeZone) {
    let edge = RESIZE_EDGE_WIDTH;

    // Semi-transparent highlight color
    let normal_color = (0.5, 0.5, 0.5, 0.3);
    let active_color = (0.3, 0.6, 1.0, 0.5);

    // Helper to set color based on zone
    let set_color = |cr: &gtk4::cairo::Context, zone: ResizeZone| {
        let (r, g, b, a) = if zone == active_zone { active_color } else { normal_color };
        cr.set_source_rgba(r, g, b, a);
    };

    // Draw edge rectangles
    // Top edge
    set_color(cr, ResizeZone::Top);
    cr.rectangle(edge, 0.0, width - 2.0 * edge, edge);
    let _ = cr.fill();

    // Bottom edge
    set_color(cr, ResizeZone::Bottom);
    cr.rectangle(edge, height - edge, width - 2.0 * edge, edge);
    let _ = cr.fill();

    // Left edge
    set_color(cr, ResizeZone::Left);
    cr.rectangle(0.0, edge, edge, height - 2.0 * edge);
    let _ = cr.fill();

    // Right edge
    set_color(cr, ResizeZone::Right);
    cr.rectangle(width - edge, edge, edge, height - 2.0 * edge);
    let _ = cr.fill();

    // Draw corner squares
    // Top-left
    set_color(cr, ResizeZone::TopLeft);
    cr.rectangle(0.0, 0.0, edge, edge);
    let _ = cr.fill();

    // Top-right
    set_color(cr, ResizeZone::TopRight);
    cr.rectangle(width - edge, 0.0, edge, edge);
    let _ = cr.fill();

    // Bottom-left
    set_color(cr, ResizeZone::BottomLeft);
    cr.rectangle(0.0, height - edge, edge, edge);
    let _ = cr.fill();

    // Bottom-right
    set_color(cr, ResizeZone::BottomRight);
    cr.rectangle(width - edge, height - edge, edge, edge);
    let _ = cr.fill();

    // Draw move indicator in center if in move zone
    if active_zone == ResizeZone::Move {
        cr.set_source_rgba(0.3, 0.6, 1.0, 0.2);
        cr.rectangle(edge, edge, width - 2.0 * edge, height - 2.0 * edge);
        let _ = cr.fill();

        // Draw move arrows icon in center
        cr.set_source_rgba(0.3, 0.6, 1.0, 0.6);
        let center_x = width / 2.0;
        let center_y = height / 2.0;
        let arrow_size = 20.0;

        // Draw 4-way arrow
        cr.set_line_width(3.0);

        // Up arrow
        cr.move_to(center_x, center_y - arrow_size);
        cr.line_to(center_x, center_y - 5.0);
        let _ = cr.stroke();
        cr.move_to(center_x - 5.0, center_y - arrow_size + 5.0);
        cr.line_to(center_x, center_y - arrow_size);
        cr.line_to(center_x + 5.0, center_y - arrow_size + 5.0);
        let _ = cr.stroke();

        // Down arrow
        cr.move_to(center_x, center_y + arrow_size);
        cr.line_to(center_x, center_y + 5.0);
        let _ = cr.stroke();
        cr.move_to(center_x - 5.0, center_y + arrow_size - 5.0);
        cr.line_to(center_x, center_y + arrow_size);
        cr.line_to(center_x + 5.0, center_y + arrow_size - 5.0);
        let _ = cr.stroke();

        // Left arrow
        cr.move_to(center_x - arrow_size, center_y);
        cr.line_to(center_x - 5.0, center_y);
        let _ = cr.stroke();
        cr.move_to(center_x - arrow_size + 5.0, center_y - 5.0);
        cr.line_to(center_x - arrow_size, center_y);
        cr.line_to(center_x - arrow_size + 5.0, center_y + 5.0);
        let _ = cr.stroke();

        // Right arrow
        cr.move_to(center_x + arrow_size, center_y);
        cr.line_to(center_x + 5.0, center_y);
        let _ = cr.stroke();
        cr.move_to(center_x + arrow_size - 5.0, center_y - 5.0);
        cr.line_to(center_x + arrow_size, center_y);
        cr.line_to(center_x + arrow_size - 5.0, center_y + 5.0);
        let _ = cr.stroke();
    }
}
