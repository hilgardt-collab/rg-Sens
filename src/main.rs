use clap::Parser;
use gtk4::gdk::Display;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, CssProvider};
use log::{error, info, warn};
use rg_sens::config::AppConfig;
use rg_sens::core::{Panel, PanelData, PanelGeometry, UpdateManager};
use rg_sens::ui::{GridConfig as UiGridConfig, GridLayout};
use rg_sens::{displayers, sources};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::sync::RwLock;

const APP_ID: &str = "com.github.hilgardt_collab.rg_sens";

/// rg-Sens - A fast, customizable system monitoring dashboard for Linux
#[derive(Parser, Debug, Clone)]
#[command(name = "rg-sens")]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Launch in fullscreen mode. Optionally specify monitor number (e.g., -f=1)
    #[arg(short = 'f', long = "fullscreen", value_name = "MONITOR")]
    fullscreen: Option<Option<i32>>,

    /// Launch in borderless mode. Optionally specify monitor number (e.g., -b=1)
    #[arg(short = 'b', long = "borderless", value_name = "MONITOR")]
    borderless: Option<Option<i32>>,

    /// Force normal windowed mode (overrides saved fullscreen/borderless/maximized state)
    #[arg(short = 'w', long = "windowed")]
    windowed: bool,

    /// Launch window at specific coordinates (e.g., -a=50,50 or --at=50,50)
    #[arg(short = 'a', long = "at", value_name = "X,Y", value_parser = parse_coordinates)]
    at: Option<(i32, i32)>,

    /// List available monitors
    #[arg(short = 'l', long = "list")]
    list_monitors: bool,

    /// Debug verbosity level (0=quiet, 1=info, 2=debug, 3=trace)
    #[arg(short = 'd', long = "debug", value_name = "LEVEL", default_value = "0")]
    debug: u8,

    /// Layout file to load at startup
    #[arg(value_name = "LAYOUT_FILE")]
    layout_file: Option<String>,
}

/// Parse coordinate string "X,Y" into (i32, i32)
fn parse_coordinates(s: &str) -> Result<(i32, i32), String> {
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() != 2 {
        return Err(format!("Expected format: X,Y (e.g., 50,50), got: {}", s));
    }
    let x = parts[0].trim().parse::<i32>()
        .map_err(|e| format!("Invalid X coordinate: {}", e))?;
    let y = parts[1].trim().parse::<i32>()
        .map_err(|e| format!("Invalid Y coordinate: {}", e))?;
    Ok((x, y))
}

/// Global CLI options accessible from build_ui
static CLI_OPTIONS: std::sync::OnceLock<Cli> = std::sync::OnceLock::new();

fn main() {
    // Parse command line arguments
    let cli = Cli::parse();

    // Initialize logger with verbosity based on -d/--debug flag
    // Level 0 (default): warn only (quiet, shows only important === messages)
    // Level 1: info (normal verbosity)
    // Level 2: debug (detailed)
    // Level 3+: trace (very detailed)
    let log_level = match cli.debug {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };
    // Allow RUST_LOG to override CLI setting
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level)).init();

    warn!("Starting rg-Sens v{}", env!("CARGO_PKG_VERSION"));

    // Handle --list option (list monitors and exit)
    if cli.list_monitors {
        list_available_monitors();
        return;
    }

    // Store CLI options for access in build_ui
    CLI_OPTIONS.set(cli).expect("CLI options already set");

    // Initialize shared sensor caches early (before any source creation)
    sources::initialize_sensors();

    // Register all built-in sources and displayers
    sources::register_all();
    displayers::register_all();

    // Create GTK application
    let app = Application::builder().application_id(APP_ID).build();

    app.connect_activate(build_ui);

    // Run the application (pass empty args since we already parsed them)
    app.run_with_args(&["rg-sens"]);
}

/// List available monitors to stdout
fn list_available_monitors() {
    // We need to initialize GTK to query monitors
    gtk4::init().expect("Failed to initialize GTK");

    if let Some(display) = gtk4::gdk::Display::default() {
        let n_monitors = display.monitors().n_items();
        println!("Available monitors ({}):", n_monitors);
        println!();

        for i in 0..n_monitors {
            if let Some(obj) = display.monitors().item(i) {
                if let Ok(monitor) = obj.downcast::<gtk4::gdk::Monitor>() {
                    let connector = monitor.connector()
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| format!("Monitor {}", i));

                    let model = monitor.model()
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "Unknown".to_string());

                    let manufacturer = monitor.manufacturer()
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "Unknown".to_string());

                    let geometry = monitor.geometry();
                    let scale = monitor.scale_factor();
                    let refresh = monitor.refresh_rate() as f64 / 1000.0;

                    println!("  {} - {} {}", i, manufacturer, model);
                    println!("      Connector: {}", connector);
                    println!("      Resolution: {}x{} @ {:.0}Hz", geometry.width(), geometry.height(), refresh);
                    println!("      Position: ({}, {})", geometry.x(), geometry.y());
                    println!("      Scale factor: {}", scale);
                    println!();
                }
            }
        }
    } else {
        eprintln!("Error: Could not connect to display");
        std::process::exit(1);
    }
}

fn build_ui(app: &Application) {
    info!("Building UI");

    // Get CLI options
    let cli = CLI_OPTIONS.get().cloned().unwrap_or(Cli {
        fullscreen: None,
        borderless: None,
        windowed: false,
        at: None,
        list_monitors: false,
        debug: 0,
        layout_file: None,
    });

    // Load CSS for selection styling
    load_css();

    // Apply system color scheme (dark/light mode)
    apply_system_color_scheme();

    // Load user colors for color picker
    if let Err(e) = rg_sens::ui::custom_color_picker::CustomColorPicker::load_colors() {
        warn!("Failed to load user colors: {}", e);
    }

    // Load configuration - from layout file if specified, otherwise from default config
    let app_config = if let Some(ref layout_path) = cli.layout_file {
        let path = std::path::PathBuf::from(layout_path);
        match AppConfig::load_from_path(&path) {
            Ok(config) => {
                info!("Loaded layout from: {}", layout_path);
                config
            }
            Err(e) => {
                warn!("Failed to load layout file '{}': {}", layout_path, e);
                // Fall back to default config
                AppConfig::load().unwrap_or_default()
            }
        }
    } else {
        match AppConfig::load() {
            Ok(config) => {
                info!("Loaded configuration from disk");
                config
            }
            Err(e) => {
                warn!("Failed to load config, using defaults: {}", e);
                AppConfig::default()
            }
        }
    };

    // Wrap app_config in Rc<RefCell<>> for shared mutable access
    let app_config = Rc::new(RefCell::new(app_config));

    // Initialize global timer manager with saved timers/alarms and global sound
    {
        let cfg = app_config.borrow();
        if let Ok(mut manager) = rg_sens::core::global_timer_manager().write() {
            manager.load_config_with_sound(
                cfg.timers.clone(),
                cfg.alarms.clone(),
                Some(cfg.global_timer_sound.clone()),
            );
            info!("Loaded {} timers and {} alarms from config", cfg.timers.len(), cfg.alarms.len());
        }
    }

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

    // Determine borderless mode - CLI options override config
    // --windowed forces normal windowed mode, overriding saved borderless
    let is_borderless = if cli.windowed {
        false
    } else {
        cli.borderless.is_some() || app_config.borrow().window.borderless
    };

    // Create the main window with saved dimensions
    let window = {
        let cfg = app_config.borrow();
        let builder = gtk4::ApplicationWindow::builder()
            .application(app)
            .title("rg-Sens - System Monitor")
            .default_width(cfg.window.width)
            .default_height(cfg.window.height)
            .decorated(!is_borderless);
        builder.build()
    };

    // Update config with CLI borderless setting
    if cli.borderless.is_some() {
        app_config.borrow_mut().window.borderless = true;
    }

    // Create grid layout
    let mut grid_layout = GridLayout::new(grid_config);

    // Create registry
    let registry = rg_sens::core::global_registry();

    // Create shared source manager (ensures each source type is polled only once)
    let shared_source_manager = Arc::new(rg_sens::core::SharedSourceManager::new());
    rg_sens::core::init_global_shared_source_manager(shared_source_manager.clone());

    // Create update manager
    let update_manager = Arc::new(UpdateManager::new());

    // Initialize global update manager so new panels can register themselves
    rg_sens::core::init_global_update_manager(update_manager.clone());

    let mut panels = Vec::new();

    // Create panels from configuration (uses new PanelData format with auto-migration)
    let panel_data_list = app_config.borrow().get_panels();
    if panel_data_list.is_empty() {
        info!("No panels in config, creating default panels");

        // Create default panels using PanelData
        let default_panels = vec![
            PanelData::with_types("panel-1".to_string(), PanelGeometry { x: 0, y: 0, width: 1, height: 1 }, "cpu", "text"),
            PanelData::with_types("panel-2".to_string(), PanelGeometry { x: 1, y: 0, width: 1, height: 1 }, "cpu", "text"),
            PanelData::with_types("panel-3".to_string(), PanelGeometry { x: 0, y: 1, width: 2, height: 1 }, "cpu", "text"),
        ];

        for panel_data in default_panels {
            match create_panel_from_data(panel_data, registry) {
                Ok(panel) => {
                    grid_layout.add_panel(panel.clone());
                    panels.push(panel);
                }
                Err(e) => {
                    warn!("Failed to create default panel: {}", e);
                }
            }
        }
    } else {
        info!("Loading {} panels from config", panel_data_list.len());

        for panel_data in panel_data_list {
            let panel_id = panel_data.id.clone();
            match create_panel_from_data(panel_data, registry) {
                Ok(panel) => {
                    grid_layout.add_panel(panel.clone());
                    panels.push(panel);
                }
                Err(e) => {
                    warn!("Failed to create panel {}: {}", panel_id, e);
                }
            }
        }
    }

    // Ensure proper z-ordering after all panels are loaded
    grid_layout.reorder_panels_by_z_index();

    // Debug: Print shared source statistics to verify source sharing
    if let Some(manager) = rg_sens::core::global_shared_source_manager() {
        manager.debug_print_sources();
    }

    // Create window background - sized to match grid content
    let window_background = gtk4::DrawingArea::new();
    let window_bg_config = app_config.borrow().window.background.clone();
    window_background.set_draw_func(move |_, cr, width, height| {
        use rg_sens::ui::background::render_background;
        let _ = render_background(cr, &window_bg_config, width as f64, height as f64);
    });

    // Set initial background size to match grid content size
    let grid_content_size = grid_layout.get_content_size();
    window_background.set_size_request(grid_content_size.0, grid_content_size.1);

    // Create overlay to show background behind grid
    let window_overlay = gtk4::Overlay::new();
    window_overlay.set_child(Some(&window_background));
    window_overlay.add_overlay(&grid_layout.widget());

    // Wrap in ScrolledWindow to allow scrolling when panels extend beyond visible area
    let scrolled_window = gtk4::ScrolledWindow::new();
    scrolled_window.set_policy(gtk4::PolicyType::Automatic, gtk4::PolicyType::Automatic);
    scrolled_window.set_child(Some(&window_overlay));
    // Allow the scrolled area to expand
    scrolled_window.set_hexpand(true);
    scrolled_window.set_vexpand(true);

    // Set scrolled window as window content
    window.set_child(Some(&scrolled_window));

    // Set initial fullscreen state - CLI overrides config
    // --windowed forces normal windowed mode, overriding saved fullscreen
    let should_fullscreen = if cli.windowed {
        false
    } else {
        cli.fullscreen.is_some() || app_config.borrow().window.fullscreen_enabled
    };
    if should_fullscreen {
        // Determine which monitor to fullscreen on
        // Priority: CLI argument > saved connector name > saved monitor index
        let monitor_index = if let Some(monitor_opt) = &cli.fullscreen {
            // CLI fullscreen option: -f (None) or -f=N (Some(N))
            *monitor_opt
        } else {
            // Try to find monitor by saved connector name first
            let cfg = app_config.borrow();
            if let Some(ref connector) = cfg.window.monitor_connector {
                find_monitor_by_connector(connector).map(|i| i as i32)
            } else {
                cfg.window.fullscreen_monitor
            }
        };

        if let Some(monitor_idx) = monitor_index {
            // Fullscreen on specific monitor
            if let Some(display) = gtk4::gdk::Display::default() {
                if let Some(mon) = display.monitors().item(monitor_idx as u32) {
                    if let Ok(monitor) = mon.downcast::<gtk4::gdk::Monitor>() {
                        info!("Fullscreen on monitor {}", monitor_idx);
                        window.fullscreen_on_monitor(&monitor);
                    }
                } else {
                    warn!("Monitor {} not found, using current monitor", monitor_idx);
                    window.fullscreen();
                }
            }
        } else {
            // Fullscreen on current monitor
            window.fullscreen();
        }
    }

    // Restore maximized state from config
    // Only restore if:
    // - Not fullscreen (CLI or config)
    // - No CLI options that override window state (--windowed, -f, -b, -a)
    let cli_overrides_maximized = cli.windowed || cli.fullscreen.is_some() || cli.borderless.is_some() || cli.at.is_some();
    let should_maximize = !should_fullscreen && !cli_overrides_maximized && app_config.borrow().window.maximized;
    if should_maximize {
        window.maximize();
        info!("Restored maximized window state");
    }

    // Apply borderless monitor selection
    // Priority: CLI argument > saved connector name
    if is_borderless && !should_fullscreen {
        let target_monitor_idx = if let Some(Some(monitor_idx)) = cli.borderless {
            Some(monitor_idx as u32)
        } else if let Some(ref connector) = app_config.borrow().window.monitor_connector {
            find_monitor_by_connector(connector)
        } else {
            None
        };

        if let Some(monitor_idx) = target_monitor_idx {
            if let Some(display) = gtk4::gdk::Display::default() {
                if let Some(mon) = display.monitors().item(monitor_idx) {
                    if let Ok(monitor) = mon.downcast::<gtk4::gdk::Monitor>() {
                        let geometry = monitor.geometry();
                        info!("Targeting borderless window on monitor {} ({}) at ({}, {})",
                              monitor_idx,
                              monitor.connector().map(|s| s.to_string()).unwrap_or_default(),
                              geometry.x(), geometry.y());
                        // Note: GTK4 doesn't have direct window positioning API
                        // The window manager will place the window, but we store
                        // the monitor info so we can restore fullscreen correctly
                    }
                }
            }
        }
    }

    // Apply CLI window position if specified
    if let Some((x, y)) = cli.at {
        info!("Positioning window at ({}, {})", x, y);
        // GTK4 doesn't provide direct window positioning API
        // We need to use the native surface after the window is realized
        let window_for_position = window.clone();
        let x_pos = x;
        let y_pos = y;
        glib::idle_add_local_once(move || {
            use gtk4::prelude::NativeExt;
            if let Some(surface) = window_for_position.surface() {
                // For X11/Wayland, we may need to use platform-specific APIs
                // GTK4's approach is to let the window manager handle positioning
                // However, we can try using the toplevel's present method with hints
                if let Ok(toplevel) = surface.downcast::<gtk4::gdk::Toplevel>() {
                    // Note: GTK4 on Wayland doesn't support positioning windows directly
                    // On X11, we might be able to use gtk4-x11 crate
                    // For now, log a warning if positioning might not work
                    info!("Window position hint set to ({}, {})", x_pos, y_pos);
                    // The actual positioning depends on the window manager and display server
                    let _ = toplevel;
                }
            }
        });
    }

    // Track if configuration has changed (dirty flag) - thread-safe
    let config_dirty = Arc::new(AtomicBool::new(false));

    // Mark config as dirty and update background size when panels are moved
    let config_dirty_clone = config_dirty.clone();
    let window_bg_for_change = window_background.clone();
    let grid_layout_for_change = Rc::new(RefCell::new(None::<Rc<RefCell<rg_sens::ui::GridLayout>>>));
    let grid_layout_for_change_clone = grid_layout_for_change.clone();
    grid_layout.set_on_change(move || {
        config_dirty_clone.store(true, Ordering::Relaxed);
        // Update background size to match grid content
        if let Some(layout) = grid_layout_for_change_clone.borrow().as_ref() {
            let content_size = layout.borrow().get_content_size();
            window_bg_for_change.set_size_request(content_size.0, content_size.1);
        }
    });

    // === Borderless window move/resize support ===
    // Set up borderless drag callback on GridLayout
    // This callback is called from GridLayout's container gesture (on the Fixed widget)
    // Using Ctrl+drag for window move/resize (Shift causes GNOME edge snapping)
    // - Ctrl+drag near edges: resize window
    // - Ctrl+drag in center: move window
    let app_config_for_drag = app_config.clone();
    let window_for_drag = window.clone();

    // Edge detection threshold in pixels
    const EDGE_THRESHOLD: f64 = 10.0;

    /// Detect which edge/corner the cursor is near, if any
    fn detect_edge(x: f64, y: f64, width: f64, height: f64, threshold: f64) -> Option<gtk4::gdk::SurfaceEdge> {
        let near_left = x < threshold;
        let near_right = x > width - threshold;
        let near_top = y < threshold;
        let near_bottom = y > height - threshold;

        match (near_left, near_right, near_top, near_bottom) {
            (true, false, true, false) => Some(gtk4::gdk::SurfaceEdge::NorthWest),
            (false, true, true, false) => Some(gtk4::gdk::SurfaceEdge::NorthEast),
            (true, false, false, true) => Some(gtk4::gdk::SurfaceEdge::SouthWest),
            (false, true, false, true) => Some(gtk4::gdk::SurfaceEdge::SouthEast),
            (true, false, false, false) => Some(gtk4::gdk::SurfaceEdge::West),
            (false, true, false, false) => Some(gtk4::gdk::SurfaceEdge::East),
            (false, false, true, false) => Some(gtk4::gdk::SurfaceEdge::North),
            (false, false, false, true) => Some(gtk4::gdk::SurfaceEdge::South),
            _ => None, // Not near any edge
        }
    }

    grid_layout.set_on_borderless_drag(move |gesture, x, y| {
        // Check if Ctrl is pressed (via event modifiers) and borderless mode is enabled
        // Note: Using Ctrl instead of Shift because Shift triggers GNOME's edge snapping
        let event = gesture.current_event();
        let is_ctrl = event.as_ref()
            .map(|e| e.modifier_state().contains(gtk4::gdk::ModifierType::CONTROL_MASK))
            .unwrap_or(false);
        let is_borderless = app_config_for_drag.borrow().window.borderless;
        let is_fullscreen = window_for_drag.is_fullscreen();

        if !is_ctrl || !is_borderless || is_fullscreen {
            return false; // Don't handle - let normal selection proceed
        }

        // Get event parameters
        let timestamp = event.as_ref().map(|e| e.time()).unwrap_or(0);
        let device = gesture.device();
        let button = gesture.current_button() as i32;

        // Translate coordinates from grid_layout to window
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

        // Get window dimensions for edge detection
        let win_width = window_for_drag.width() as f64;
        let win_height = window_for_drag.height() as f64;

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

        // Check if near an edge for resize, otherwise move
        if let Some(edge) = detect_edge(win_x, win_y, win_width, win_height, EDGE_THRESHOLD) {
            log::info!("Borderless window resize: Ctrl+drag at ({}, {}) edge {:?}", x, y, edge);
            toplevel.begin_resize(edge, Some(dev), button, win_x, win_y, timestamp);
        } else {
            log::info!("Borderless window move: Ctrl+drag at ({}, {})", x, y);
            toplevel.begin_move(dev, button, win_x, win_y, timestamp);
        }

        // Claim the gesture
        gesture.set_state(gtk4::EventSequenceState::Claimed);
        true // Handled
    });

    // === Cursor feedback for borderless resize ===
    // Show resize cursors when hovering near window edges with Ctrl held
    let motion_controller = gtk4::EventControllerMotion::new();
    let app_config_for_cursor = app_config.clone();
    let window_for_cursor = window.clone();

    /// Get cursor name for a given edge
    fn cursor_for_edge(edge: gtk4::gdk::SurfaceEdge) -> &'static str {
        use gtk4::gdk::SurfaceEdge;
        match edge {
            SurfaceEdge::North => "n-resize",
            SurfaceEdge::South => "s-resize",
            SurfaceEdge::East => "e-resize",
            SurfaceEdge::West => "w-resize",
            SurfaceEdge::NorthWest => "nw-resize",
            SurfaceEdge::NorthEast => "ne-resize",
            SurfaceEdge::SouthWest => "sw-resize",
            SurfaceEdge::SouthEast => "se-resize",
            _ => "default",
        }
    }

    motion_controller.connect_motion(move |controller, x, y| {
        let is_borderless = app_config_for_cursor.borrow().window.borderless;
        let is_fullscreen = window_for_cursor.is_fullscreen();

        if !is_borderless || is_fullscreen {
            window_for_cursor.set_cursor_from_name(None);
            return;
        }

        // Check if Ctrl is pressed
        let is_ctrl = controller
            .current_event()
            .map(|e| e.modifier_state().contains(gtk4::gdk::ModifierType::CONTROL_MASK))
            .unwrap_or(false);

        if !is_ctrl {
            window_for_cursor.set_cursor_from_name(None);
            return;
        }

        // Get window dimensions
        let win_width = window_for_cursor.width() as f64;
        let win_height = window_for_cursor.height() as f64;

        // Check if near an edge
        if let Some(edge) = detect_edge(x, y, win_width, win_height, EDGE_THRESHOLD) {
            window_for_cursor.set_cursor_from_name(Some(cursor_for_edge(edge)));
        } else {
            // In center area with Ctrl - show move cursor
            window_for_cursor.set_cursor_from_name(Some("move"));
        }
    });

    // Reset cursor when leaving the window
    let window_for_leave = window.clone();
    motion_controller.connect_leave(move |_| {
        window_for_leave.set_cursor_from_name(None);
    });

    window.add_controller(motion_controller);

    // Wrap grid_layout in Rc<RefCell<>> for sharing across closures
    let grid_layout = Rc::new(RefCell::new(grid_layout));

    // Set the grid_layout reference for the on_change callback (now that it's wrapped)
    *grid_layout_for_change.borrow_mut() = Some(grid_layout.clone());

    // Show grid overlay during window resize (with debounced hide)
    // Also marks config as dirty when window is resized
    let resize_hide_timer: Rc<RefCell<Option<glib::SourceId>>> = Rc::new(RefCell::new(None));

    let grid_layout_for_resize_w = grid_layout.clone();
    let resize_timer_w = resize_hide_timer.clone();
    let config_dirty_resize_w = config_dirty.clone();
    window.connect_default_width_notify(move |_| {
        // Mark config as dirty
        config_dirty_resize_w.store(true, Ordering::Relaxed);

        // Show grid immediately
        grid_layout_for_resize_w.borrow().set_grid_visible(true);

        // Cancel any pending hide timer
        if let Some(source_id) = resize_timer_w.borrow_mut().take() {
            source_id.remove();
        }

        // Schedule hide after 500ms of no resize events
        let grid_layout_hide = grid_layout_for_resize_w.clone();
        let timer_ref = resize_timer_w.clone();
        let source_id = glib::timeout_add_local_once(std::time::Duration::from_millis(500), move || {
            grid_layout_hide.borrow().set_grid_visible(false);
            *timer_ref.borrow_mut() = None;
        });
        *resize_timer_w.borrow_mut() = Some(source_id);
    });

    let grid_layout_for_resize_h = grid_layout.clone();
    let resize_timer_h = resize_hide_timer.clone();
    let config_dirty_resize_h = config_dirty.clone();
    window.connect_default_height_notify(move |_| {
        // Mark config as dirty
        config_dirty_resize_h.store(true, Ordering::Relaxed);

        // Show grid immediately
        grid_layout_for_resize_h.borrow().set_grid_visible(true);

        // Cancel any pending hide timer
        if let Some(source_id) = resize_timer_h.borrow_mut().take() {
            source_id.remove();
        }

        // Schedule hide after 500ms of no resize events
        let grid_layout_hide = grid_layout_for_resize_h.clone();
        let timer_ref = resize_timer_h.clone();
        let source_id = glib::timeout_add_local_once(std::time::Duration::from_millis(500), move || {
            grid_layout_hide.borrow().set_grid_visible(false);
            *timer_ref.borrow_mut() = None;
        });
        *resize_timer_h.borrow_mut() = Some(source_id);
    });

    // Set initial viewport size for drag visualization from config
    // If config values are 0, use default window dimensions
    {
        let config = app_config.borrow();
        let vp_width = if config.window.viewport_width > 0 {
            config.window.viewport_width
        } else {
            config.window.width
        };
        let vp_height = if config.window.viewport_height > 0 {
            config.window.viewport_height
        } else {
            config.window.height
        };
        grid_layout.borrow().set_viewport_size(vp_width, vp_height);
    }

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
            config_dirty_for_fullscreen.store(true, Ordering::Relaxed);
        }
    });

    scrolled_window.add_controller(double_click_gesture);

    // Setup efficient auto-scroll
    // Uses timeout_add_local_once for delay, then short animation burst
    let auto_scroll_active = Rc::new(RefCell::new(false));
    let auto_scroll_generation = Rc::new(RefCell::new(0u32));

    let scrolled_window_for_auto = scrolled_window.clone();
    let app_config_for_auto = app_config.clone();
    let grid_layout_for_auto = grid_layout.clone();
    let window_background_for_auto = window_background.clone();

    // Ease-in-out function for smooth animation
    fn ease_in_out(t: f64) -> f64 {
        if t < 0.5 {
            2.0 * t * t
        } else {
            1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
        }
    }

    // Helper to perform one scroll animation then schedule next
    // Pattern: scroll right until edge, then move down+left to start of next row, repeat
    // When at bottom-right, wrap to top-left
    // Uses a generation counter to prevent overlapping scroll cycles
    fn schedule_auto_scroll(
        scrolled: gtk4::ScrolledWindow,
        config: Rc<RefCell<AppConfig>>,
        layout: Rc<RefCell<GridLayout>>,
        active: Rc<RefCell<bool>>,
        generation: Rc<RefCell<u32>>,
        current_gen: u32,
        bg: gtk4::DrawingArea,
    ) {
        // Check if this is a stale callback from an old generation
        if *generation.borrow() != current_gen {
            return;
        }

        let cfg = config.borrow();
        if !cfg.window.auto_scroll_enabled {
            *active.borrow_mut() = false;
            return;
        }
        let delay_ms = cfg.window.auto_scroll_delay_ms;
        drop(cfg);

        *active.borrow_mut() = true;

        // Schedule the scroll after delay
        glib::timeout_add_local_once(std::time::Duration::from_millis(delay_ms), move || {
            // Check generation again - might have been reset while waiting
            if *generation.borrow() != current_gen {
                return;
            }

            let cfg = config.borrow();
            if !cfg.window.auto_scroll_enabled {
                *active.borrow_mut() = false;
                return;
            }
            drop(cfg);

            // Get scroll info
            let h_adj = scrolled.hadjustment();
            let v_adj = scrolled.vadjustment();
            let content_size = layout.borrow().get_content_size();
            let content_width = content_size.0 as f64;
            let content_height = content_size.1 as f64;
            let viewport_width = h_adj.page_size();
            let viewport_height = v_adj.page_size();

            // Check if whole pages mode is enabled
            let cfg = config.borrow();
            let whole_pages = cfg.window.auto_scroll_whole_pages;
            drop(cfg);

            // Calculate effective scroll bounds and container size
            // When whole_pages is enabled, align to complete page boundaries
            let (max_h_scroll, max_v_scroll, container_width, container_height) = if whole_pages && viewport_width > 0.0 && viewport_height > 0.0 {
                // Calculate number of complete pages needed to cover content
                let h_pages = (content_width / viewport_width).ceil() as i32;
                let v_pages = (content_height / viewport_height).ceil() as i32;
                // Max scroll position is (pages - 1) * viewport_size
                let max_h = ((h_pages - 1).max(0) as f64) * viewport_width;
                let max_v = ((v_pages - 1).max(0) as f64) * viewport_height;
                // Container size must be large enough to scroll to all page boundaries
                // Size = pages * viewport_size (so we can scroll to the last page)
                let cont_w = (h_pages as f64 * viewport_width) as i32;
                let cont_h = (v_pages as f64 * viewport_height) as i32;
                (max_h, max_v, cont_w, cont_h)
            } else {
                // Default: scroll to content bounds
                ((content_width - viewport_width).max(0.0), (content_height - viewport_height).max(0.0), content_size.0, content_size.1)
            };

            let needs_h_scroll = max_h_scroll > 1.0;
            let needs_v_scroll = max_v_scroll > 1.0;

            if !needs_h_scroll && !needs_v_scroll {
                // No scrolling needed, reschedule check
                schedule_auto_scroll(scrolled, config, layout, active, generation, current_gen, bg);
                return;
            }

            // Update background size (expanded for whole pages mode)
            bg.set_size_request(container_width, container_height);

            // Check current position against effective bounds
            let at_h_end = h_adj.value() >= max_h_scroll - 1.0;
            let at_v_end = v_adj.value() >= max_v_scroll - 1.0;

            // Determine scroll action based on position
            // Pattern: right across row, then down+left to next row, repeat
            let (h_start, h_target, v_start, v_target) = if !at_h_end && needs_h_scroll {
                // Scroll right one viewport width
                let h_start = h_adj.value();
                let h_target = (h_start + viewport_width).min(max_h_scroll);
                (h_start, h_target, v_adj.value(), v_adj.value())
            } else if at_h_end && !at_v_end && needs_v_scroll {
                // At right edge, move down one row and back to left
                let v_start = v_adj.value();
                let v_target = (v_start + viewport_height).min(max_v_scroll);
                (h_adj.value(), 0.0, v_start, v_target)
            } else {
                // At bottom-right or only horizontal content, wrap to top-left
                (h_adj.value(), 0.0, v_adj.value(), 0.0)
            };

            // Run animation (200ms total, ~12 frames)
            const ANIMATION_MS: u64 = 200;
            const FRAME_MS: u64 = 16;
            let frame_count = Rc::new(RefCell::new(0u32));
            let total_frames = (ANIMATION_MS / FRAME_MS) as u32;

            glib::timeout_add_local(std::time::Duration::from_millis(FRAME_MS), move || {
                // Check generation - stop if a new cycle was started
                if *generation.borrow() != current_gen {
                    return glib::ControlFlow::Break;
                }

                let mut frame = frame_count.borrow_mut();
                *frame += 1;

                let progress = (*frame as f64) / (total_frames as f64);
                let eased = ease_in_out(progress.min(1.0));

                // Animate both h and v positions
                let h_pos = h_start + (h_target - h_start) * eased;
                let v_pos = v_start + (v_target - v_start) * eased;
                h_adj.set_value(h_pos);
                v_adj.set_value(v_pos);

                if *frame >= total_frames {
                    // Animation done, schedule next scroll after delay
                    schedule_auto_scroll(scrolled.clone(), config.clone(), layout.clone(), active.clone(), generation.clone(), current_gen, bg.clone());
                    return glib::ControlFlow::Break;
                }

                glib::ControlFlow::Continue
            });
        });
    }

    // Function to start/restart the auto-scroll system
    let start_auto_scroll = {
        let scrolled_window = scrolled_window_for_auto.clone();
        let app_config = app_config_for_auto.clone();
        let grid_layout = grid_layout_for_auto.clone();
        let active = auto_scroll_active.clone();
        let generation = auto_scroll_generation.clone();
        let window_background = window_background_for_auto.clone();

        move || {
            *active.borrow_mut() = false;

            // Increment generation to invalidate any pending scroll cycles
            let new_gen = {
                let mut gen = generation.borrow_mut();
                *gen = gen.wrapping_add(1);
                *gen
            };

            let cfg = app_config.borrow();
            if !cfg.window.auto_scroll_enabled {
                return;
            }
            drop(cfg);

            // Reset scroll position to top-left when starting
            scrolled_window.hadjustment().set_value(0.0);
            scrolled_window.vadjustment().set_value(0.0);

            // Start the auto-scroll cycle with current generation
            schedule_auto_scroll(
                scrolled_window.clone(),
                app_config.clone(),
                grid_layout.clone(),
                active.clone(),
                generation.clone(),
                new_gen,
                window_background.clone(),
            );
        }
    };

    // Start auto-scroll if enabled in config
    start_auto_scroll();

    // Store the start function for use in settings dialog
    let start_auto_scroll = Rc::new(start_auto_scroll);

    // Setup save-on-close confirmation
    let grid_layout_for_close = grid_layout.clone();
    let config_dirty_clone4 = config_dirty.clone();
    let app_config_for_close = app_config.clone();
    let update_manager_for_close = update_manager.clone();

    window.connect_close_request(move |window| {
        // Stop the update manager gracefully
        update_manager_for_close.stop();

        let is_dirty = config_dirty_clone4.load(Ordering::Relaxed);

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
    let start_auto_scroll_for_settings = start_auto_scroll.clone();

    // Add right-click gesture for context menu
    let gesture_click = gtk4::GestureClick::new();
    gesture_click.set_button(gtk4::gdk::BUTTON_SECONDARY);

    // Clone variables for context menu
    let window_for_menu = window.clone();
    let app_config_for_menu = app_config.clone();
    let window_bg_for_menu = window_background.clone();
    let grid_layout_for_menu = grid_layout_for_settings.clone();
    let config_dirty_for_menu = config_dirty.clone();
    let start_auto_scroll_for_menu = start_auto_scroll.clone();

    gesture_click.connect_pressed(move |gesture, _, x, y| {
        use gtk4::{Popover, Box as GtkBox, Button, Separator, Orientation};

        // Create a custom popover with buttons (no ScrolledWindow, no scrolling)
        let popover = Popover::new();
        popover.set_parent(&window_for_menu);
        popover.set_has_arrow(false);
        popover.set_autohide(true);

        // Create menu content box
        let menu_box = GtkBox::new(Orientation::Vertical, 0);
        menu_box.set_margin_top(6);
        menu_box.set_margin_bottom(6);
        menu_box.set_margin_start(6);
        menu_box.set_margin_end(6);

        // Helper to create menu buttons with consistent styling
        let create_menu_button = |label: &str| -> Button {
            let btn = Button::with_label(label);
            btn.add_css_class("flat");
            btn.set_halign(gtk4::Align::Fill);
            btn
        };

        // New Panel button
        let new_panel_btn = create_menu_button("New Panel");
        menu_box.append(&new_panel_btn);

        // Load Panel from File button
        let load_panel_btn = create_menu_button("Load Panel from File...");
        menu_box.append(&load_panel_btn);

        menu_box.append(&Separator::new(Orientation::Horizontal));

        // Save Layout button
        let save_layout_btn = create_menu_button("Save Layout");
        menu_box.append(&save_layout_btn);

        menu_box.append(&Separator::new(Orientation::Horizontal));

        // Save to File button
        let save_file_btn = create_menu_button("Save Layout to File...");
        menu_box.append(&save_file_btn);

        // Load from File button
        let load_file_btn = create_menu_button("Load Layout from File...");
        menu_box.append(&load_file_btn);

        menu_box.append(&Separator::new(Orientation::Horizontal));

        // Test Source button
        let test_source_btn = create_menu_button("Test Source...");
        menu_box.append(&test_source_btn);

        // Options button
        let options_btn = create_menu_button("Options");
        menu_box.append(&options_btn);

        // Quit button
        let quit_btn = create_menu_button("Quit");
        menu_box.append(&quit_btn);

        popover.set_child(Some(&menu_box));

        // Position at click location
        popover.set_pointing_to(Some(&gtk4::gdk::Rectangle::new(
            x as i32,
            y as i32,
            1,
            1,
        )));

        // Store references for button handlers
        let popover_ref = popover.clone();

        // New Panel button handler
        let window_for_new = window_for_menu.clone();
        let grid_layout_for_new = grid_layout_for_menu.clone();
        let config_dirty_for_new = config_dirty_for_menu.clone();
        let app_config_for_new = app_config_for_menu.clone();
        let mouse_x = x;
        let mouse_y = y;
        let popover_for_new = popover_ref.clone();
        new_panel_btn.connect_clicked(move |_| {
            popover_for_new.popdown();
            info!("New panel requested at ({}, {})", mouse_x, mouse_y);
            show_new_panel_dialog(
                &window_for_new,
                &grid_layout_for_new,
                &config_dirty_for_new,
                &app_config_for_new,
                Some((mouse_x, mouse_y)),
            );
        });

        // Load Panel from File button handler
        let window_for_load_panel = window_for_menu.clone();
        let grid_layout_for_load_panel = grid_layout_for_menu.clone();
        let config_dirty_for_load_panel = config_dirty_for_menu.clone();
        let app_config_for_load_panel = app_config_for_menu.clone();
        let popover_for_load_panel = popover_ref.clone();
        let load_mouse_x = x;
        let load_mouse_y = y;
        load_panel_btn.connect_clicked(move |_| {
            popover_for_load_panel.popdown();
            info!("Load panel from file requested");
            let window = window_for_load_panel.clone();
            let grid_layout = grid_layout_for_load_panel.clone();
            let config_dirty = config_dirty_for_load_panel.clone();
            let app_config = app_config_for_load_panel.clone();
            let mouse_x = load_mouse_x;
            let mouse_y = load_mouse_y;

            gtk4::glib::MainContext::default().spawn_local(async move {
                use gtk4::FileDialog;
                use gtk4::gio;

                // Get initial directory (config dir)
                let initial_dir = directories::ProjectDirs::from("com", "github.hilgardt_collab", "rg-sens")
                    .map(|d| d.config_dir().to_path_buf())
                    .unwrap_or_else(|| std::path::PathBuf::from("/"));

                // Create file filter for JSON files
                let json_filter = gtk4::FileFilter::new();
                json_filter.set_name(Some("JSON files"));
                json_filter.add_pattern("*.json");

                let all_filter = gtk4::FileFilter::new();
                all_filter.set_name(Some("All files"));
                all_filter.add_pattern("*");

                let filters = gio::ListStore::new::<gtk4::FileFilter>();
                filters.append(&json_filter);
                filters.append(&all_filter);

                let file_dialog = FileDialog::builder()
                    .title("Load Panel from File")
                    .modal(true)
                    .initial_folder(&gio::File::for_path(&initial_dir))
                    .filters(&filters)
                    .default_filter(&json_filter)
                    .build();

                match file_dialog.open_future(Some(&window)).await {
                    Ok(file) => {
                        if let Some(path) = file.path() {
                            info!("Loading panel from {:?}", path);

                            // Read and parse the JSON file
                            match std::fs::read_to_string(&path) {
                                Ok(json) => {
                                    match serde_json::from_str::<PanelData>(&json) {
                                        Ok(mut panel_data) => {
                                            // Generate a new unique ID for the loaded panel
                                            panel_data.id = uuid::Uuid::new_v4().to_string();

                                            // Calculate grid position from mouse coordinates
                                            let cfg = app_config.borrow();
                                            let cell_width = cfg.grid.cell_width as f64;
                                            let cell_height = cfg.grid.cell_height as f64;
                                            let spacing = cfg.grid.spacing as f64;
                                            drop(cfg);

                                            let grid_x = (mouse_x / (cell_width + spacing)).floor() as u32;
                                            let grid_y = (mouse_y / (cell_height + spacing)).floor() as u32;

                                            // Check for collision at this position
                                            let has_collision = grid_layout.borrow().check_collision(
                                                grid_x,
                                                grid_y,
                                                panel_data.geometry.width,
                                                panel_data.geometry.height,
                                            );

                                            if has_collision {
                                                // Show error dialog
                                                let dialog = gtk4::AlertDialog::builder()
                                                    .message("Cannot Load Panel")
                                                    .detail("The panel cannot be placed at this position because it would overlap with existing panels.")
                                                    .modal(true)
                                                    .build();
                                                dialog.show(Some(&window));
                                                return;
                                            }

                                            panel_data.geometry.x = grid_x;
                                            panel_data.geometry.y = grid_y;

                                            info!("Placing panel at grid position ({}, {})", grid_x, grid_y);

                                            // Create the panel from data
                                            let registry = rg_sens::core::global_registry();
                                            match create_panel_from_data(panel_data, registry) {
                                                Ok(panel) => {
                                                    // Add to grid layout
                                                    grid_layout.borrow_mut().add_panel(panel.clone());

                                                    // Register with update manager
                                                    if let Some(update_manager) = rg_sens::core::global_update_manager() {
                                                        update_manager.queue_add_panel(panel);
                                                    }

                                                    // Mark config as dirty
                                                    config_dirty.store(true, Ordering::Relaxed);
                                                    info!("Panel loaded successfully from {:?}", path);
                                                }
                                                Err(e) => {
                                                    warn!("Failed to create panel from data: {}", e);
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            warn!("Failed to parse panel JSON: {}", e);
                                        }
                                    }
                                }
                                Err(e) => {
                                    warn!("Failed to read panel file: {}", e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        info!("Load panel dialog cancelled or failed: {}", e);
                    }
                }
            });
        });

        // Save Layout button handler
        let grid_layout_for_save = grid_layout_for_menu.clone();
        let app_config_for_save = app_config_for_menu.clone();
        let window_for_save = window_for_menu.clone();
        let config_dirty_for_save = config_dirty_for_menu.clone();
        let popover_for_save = popover_ref.clone();
        save_layout_btn.connect_clicked(move |_| {
            popover_for_save.popdown();
            info!("Save layout requested");
            let current_panels = grid_layout_for_save.borrow().get_panels();
            save_config_with_app_config(&mut app_config_for_save.borrow_mut(), &window_for_save, &current_panels);
            config_dirty_for_save.store(false, Ordering::Relaxed);
        });

        // Save to File button handler
        let window_for_save_file = window_for_menu.clone();
        let grid_layout_for_save_file = grid_layout_for_menu.clone();
        let app_config_for_save_file = app_config_for_menu.clone();
        let config_dirty_for_save_file = config_dirty_for_menu.clone();
        let popover_for_save_file = popover_ref.clone();
        save_file_btn.connect_clicked(move |_| {
            popover_for_save_file.popdown();
            info!("Save to file requested");
            let window = window_for_save_file.clone();
            let grid_layout = grid_layout_for_save_file.clone();
            let app_config = app_config_for_save_file.clone();
            let config_dirty = config_dirty_for_save_file.clone();

            gtk4::glib::MainContext::default().spawn_local(async move {
                use gtk4::FileDialog;
                use gtk4::gio;

                info!("Creating save file dialog");

                // Get initial directory (config dir)
                let initial_dir = directories::ProjectDirs::from("com", "github.hilgardt_collab", "rg-sens")
                    .map(|d| d.config_dir().to_path_buf())
                    .unwrap_or_else(|| std::path::PathBuf::from("/"));

                // Create file filter for JSON files
                let json_filter = gtk4::FileFilter::new();
                json_filter.set_name(Some("JSON files"));
                json_filter.add_pattern("*.json");

                let all_filter = gtk4::FileFilter::new();
                all_filter.set_name(Some("All files"));
                all_filter.add_pattern("*");

                let filters = gio::ListStore::new::<gtk4::FileFilter>();
                filters.append(&json_filter);
                filters.append(&all_filter);

                let file_dialog = FileDialog::builder()
                    .title("Save Layout to File")
                    .modal(true)
                    .initial_folder(&gio::File::for_path(&initial_dir))
                    .initial_name("layout.json")
                    .filters(&filters)
                    .default_filter(&json_filter)
                    .build();

                info!("Showing save file dialog");
                match file_dialog.save_future(Some(&window)).await {
                    Ok(file) => {
                        if let Some(path) = file.path() {
                            info!("Saving layout to {:?}", path);

                            let current_panels = grid_layout.borrow().get_panels();
                            let (width, height) = (window.default_width(), window.default_height());
                            // Use blocking_read to ensure all panels are saved
                            let panel_data_list: Vec<PanelData> = current_panels
                                .iter()
                                .map(|panel| {
                                    let panel_guard = panel.blocking_read();
                                    panel_guard.to_data()
                                })
                                .collect();

                            // Update config in place instead of cloning
                            {
                                let mut config = app_config.borrow_mut();
                                config.window.width = width;
                                config.window.height = height;
                                config.set_panels(panel_data_list);
                            }

                            match app_config.borrow().save_to_path(&path) {
                                Ok(()) => {
                                    info!("Layout saved successfully to {:?}", path);
                                    config_dirty.store(false, Ordering::Relaxed);
                                }
                                Err(e) => {
                                    warn!("Failed to save layout: {}", e);
                                }
                            }
                        } else {
                            warn!("File dialog returned no path");
                        }
                    }
                    Err(e) => {
                        // User cancelled or error occurred
                        info!("Save file dialog cancelled or failed: {}", e);
                    }
                }
            });
        });

        // Load from File button handler
        let window_for_load_file = window_for_menu.clone();
        let grid_layout_for_load_file = grid_layout_for_menu.clone();
        let app_config_for_load_file = app_config_for_menu.clone();
        let config_dirty_for_load_file = config_dirty_for_menu.clone();
        let popover_for_load_file = popover_ref.clone();
        load_file_btn.connect_clicked(move |_| {
            popover_for_load_file.popdown();
            info!("Load from file requested");
            let window = window_for_load_file.clone();
            let grid_layout = grid_layout_for_load_file.clone();
            let app_config = app_config_for_load_file.clone();
            let config_dirty = config_dirty_for_load_file.clone();

            gtk4::glib::MainContext::default().spawn_local(async move {
                use gtk4::FileDialog;
                use gtk4::gio;

                info!("Creating load file dialog");

                // Get initial directory (config dir)
                let initial_dir = directories::ProjectDirs::from("com", "github.hilgardt_collab", "rg-sens")
                    .map(|d| d.config_dir().to_path_buf())
                    .unwrap_or_else(|| std::path::PathBuf::from("/"));

                // Create file filter for JSON files
                let json_filter = gtk4::FileFilter::new();
                json_filter.set_name(Some("JSON files"));
                json_filter.add_pattern("*.json");

                let all_filter = gtk4::FileFilter::new();
                all_filter.set_name(Some("All files"));
                all_filter.add_pattern("*");

                let filters = gio::ListStore::new::<gtk4::FileFilter>();
                filters.append(&json_filter);
                filters.append(&all_filter);

                let file_dialog = FileDialog::builder()
                    .title("Load Layout from File")
                    .modal(true)
                    .initial_folder(&gio::File::for_path(&initial_dir))
                    .filters(&filters)
                    .default_filter(&json_filter)
                    .build();

                info!("Showing load file dialog");
                match file_dialog.open_future(Some(&window)).await {
                    Ok(file) => {
                        if let Some(path) = file.path() {
                            info!("Loading layout from {:?}", path);

                            match AppConfig::load_from_path(&path) {
                                Ok(loaded_config) => {
                                    info!("Layout loaded successfully from {:?}", path);
                                    *app_config.borrow_mut() = loaded_config.clone();
                                    grid_layout.borrow_mut().clear_all_panels();

                                    let registry = rg_sens::core::global_registry();
                                    for panel_data in loaded_config.get_panels() {
                                        let panel_id = panel_data.id.clone();
                                        match create_panel_from_data(panel_data, registry) {
                                            Ok(panel) => {
                                                grid_layout.borrow_mut().add_panel(panel.clone());

                                                // Register with update manager so panels get periodic updates
                                                if let Some(update_manager) = rg_sens::core::global_update_manager() {
                                                    update_manager.queue_add_panel(panel.clone());
                                                }
                                            }
                                            Err(e) => {
                                                warn!("Failed to create panel {}: {}", panel_id, e);
                                            }
                                        }
                                    }

                                    grid_layout.borrow_mut().update_grid_size(
                                        loaded_config.grid.cell_width,
                                        loaded_config.grid.cell_height,
                                        loaded_config.grid.spacing,
                                    );
                                    config_dirty.store(false, Ordering::Relaxed);
                                }
                                Err(e) => {
                                    warn!("Failed to load layout: {}", e);
                                }
                            }
                        } else {
                            warn!("File dialog returned no path");
                        }
                    }
                    Err(e) => {
                        // User cancelled or error occurred
                        info!("Load file dialog cancelled or failed: {}", e);
                    }
                }
            });
        });

        // Test Source button handler
        let window_for_test = window_for_menu.clone();
        let popover_for_test = popover_ref.clone();
        let grid_layout_for_test = grid_layout_for_menu.clone();
        let config_dirty_for_test = config_dirty_for_menu.clone();
        test_source_btn.connect_clicked(move |_| {
            popover_for_test.popdown();
            // Create callback to save test source config to all test panels
            let grid_layout = grid_layout_for_test.clone();
            let config_dirty = config_dirty_for_test.clone();
            let save_callback: rg_sens::ui::TestSourceSaveCallback = Box::new(move |test_config| {
                // Save to all panels that use test source
                let panels = grid_layout.borrow().get_panels();
                for panel in panels {
                    if let Ok(mut panel_guard) = panel.try_write() {
                        if panel_guard.source.metadata().id == "test" {
                            if let Ok(config_json) = serde_json::to_value(test_config) {
                                panel_guard.config.insert("test_config".to_string(), config_json);
                                log::debug!("Saved test source config to panel {}", panel_guard.id);
                            }
                        }
                    }
                }
                // Mark config as dirty so it gets saved
                config_dirty.store(true, Ordering::Relaxed);
            });
            rg_sens::ui::show_test_source_dialog_with_callback(&window_for_test, Some(save_callback));
        });

        // Options button handler
        let window_for_options = window_for_menu.clone();
        let app_config_for_options = app_config_for_menu.clone();
        let window_bg_for_options = window_bg_for_menu.clone();
        let grid_layout_for_options = grid_layout_for_menu.clone();
        let config_dirty_for_options = config_dirty_for_menu.clone();
        let start_auto_scroll_for_options = start_auto_scroll_for_menu.clone();
        let popover_for_options = popover_ref.clone();
        options_btn.connect_clicked(move |_| {
            popover_for_options.popdown();
            show_window_settings_dialog(
                &window_for_options,
                &app_config_for_options,
                &window_bg_for_options,
                &grid_layout_for_options,
                &config_dirty_for_options,
                &start_auto_scroll_for_options,
            );
        });

        // Quit button handler
        let window_for_quit = window_for_menu.clone();
        let popover_for_quit = popover_ref.clone();
        quit_btn.connect_clicked(move |_| {
            popover_for_quit.popdown();
            window_for_quit.close();
        });

        popover.popup();
        gesture.set_state(gtk4::EventSequenceState::Claimed);
    });

    window.add_controller(gesture_click);

    // Clone for closure
    let grid_layout_for_key = grid_layout_for_settings.clone();
    let grid_layout_for_space = grid_layout_for_settings.clone();
    let grid_layout_for_space_release = grid_layout_for_settings.clone();

    key_controller.connect_key_pressed(move |_, key, _code, modifiers| {
        // Ctrl+Comma opens settings
        if modifiers.contains(gtk4::gdk::ModifierType::CONTROL_MASK)
            && key == gtk4::gdk::Key::comma {
            show_window_settings_dialog(
                &window_clone_for_settings,
                &app_config_for_settings,
                &window_bg_for_settings,
                &grid_layout_for_key,
                &config_dirty_for_settings,
                &start_auto_scroll_for_settings,
            );
            return glib::Propagation::Stop;
        }

        // Space bar shows grid overlay
        if key == gtk4::gdk::Key::space {
            grid_layout_for_space.borrow().set_grid_visible(true);
            return glib::Propagation::Stop;
        }

        glib::Propagation::Proceed
    });

    // Hide grid when space is released
    key_controller.connect_key_released(move |_, key, _code, _modifiers| {
        if key == gtk4::gdk::Key::space {
            grid_layout_for_space_release.borrow().set_grid_visible(false);
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
                save_config_with_app_config(&mut app_config_clone.borrow_mut(), &window_clone, &current_panels);
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

/// Get the connector name of the monitor the window is currently on
fn get_window_monitor_connector(window: &ApplicationWindow) -> Option<String> {
    use gtk4::prelude::NativeExt;

    // Get the surface (realized window handle)
    let surface = window.surface()?;

    // Get the display
    let display = surface.display();

    // Get the monitor at the window's position
    // For GTK4, we use the surface to find which monitor it's on
    let monitors = display.monitors();
    let n_monitors = monitors.n_items();

    if n_monitors == 0 {
        return None;
    }

    // GTK4 doesn't have a direct "get monitor for window" API
    // On Wayland, window positions are not exposed to applications
    // We return the first monitor's connector as a fallback
    // The monitor will be properly detected when fullscreen is used
    if let Some(mon) = monitors.item(0) {
        if let Ok(monitor) = mon.downcast::<gtk4::gdk::Monitor>() {
            return monitor.connector().map(|s| s.to_string());
        }
    }

    None
}

/// Find monitor index by connector name
fn find_monitor_by_connector(connector: &str) -> Option<u32> {
    let display = gtk4::gdk::Display::default()?;
    let monitors = display.monitors();

    for i in 0..monitors.n_items() {
        if let Some(mon) = monitors.item(i) {
            if let Ok(monitor) = mon.downcast::<gtk4::gdk::Monitor>() {
                if let Some(mon_connector) = monitor.connector() {
                    if mon_connector == connector {
                        return Some(i);
                    }
                }
            }
        }
    }

    None
}

/// Save current configuration to disk
///
/// Updates the config in place and saves it, avoiding an extra clone.
fn save_config_with_app_config(app_config: &mut AppConfig, window: &ApplicationWindow, panels: &[Arc<RwLock<Panel>>]) {
    // Get window dimensions
    let (width, height) = (window.default_width(), window.default_height());

    // Get window state
    let is_maximized = window.is_maximized();
    let is_fullscreen = window.is_fullscreen();

    // Get the monitor the window is on (by connector name)
    let monitor_connector = get_window_monitor_connector(window);

    // Convert panels to PanelData (new unified format)
    // Use blocking_read to ensure all panels are saved
    let panel_data_list: Vec<PanelData> = panels
        .iter()
        .map(|panel| {
            let panel_guard = panel.blocking_read();
            panel_guard.to_data()
        })
        .collect();

    // Update config in place instead of cloning
    app_config.window.width = width;
    app_config.window.height = height;
    app_config.window.x = None; // GTK4 doesn't provide window position reliably
    app_config.window.y = None;
    app_config.window.maximized = is_maximized;
    app_config.window.fullscreen_enabled = is_fullscreen;
    app_config.window.monitor_connector = monitor_connector;
    app_config.set_panels(panel_data_list);

    // Save global timers, alarms, and timer sound
    if let Ok(manager) = rg_sens::core::global_timer_manager().read() {
        let (timers, alarms, global_sound) = manager.get_full_config();
        app_config.set_timers(timers);
        app_config.set_alarms(alarms);
        app_config.set_global_timer_sound(global_sound);
    }

    // Save to disk
    match app_config.save() {
        Ok(()) => {
            info!("Configuration saved successfully");
        }
        Err(e) => {
            warn!("Failed to save configuration: {}", e);
        }
    }
}

/// Show window settings dialog
fn show_window_settings_dialog<F>(
    parent_window: &ApplicationWindow,
    app_config: &Rc<RefCell<AppConfig>>,
    window_background: &gtk4::DrawingArea,
    grid_layout: &Rc<RefCell<GridLayout>>,
    config_dirty: &Arc<AtomicBool>,
    on_auto_scroll_change: &Rc<F>,
) where
    F: Fn() + 'static,
{
    use gtk4::{Box as GtkBox, Button, CheckButton, DropDown, Label, Notebook, Orientation, SpinButton, StringList, Window};
    use rg_sens::ui::BackgroundConfigWidget;
    use rg_sens::config::DefaultsConfig;

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

    // Load current defaults
    let defaults_config = Rc::new(RefCell::new(DefaultsConfig::load()));

    // === Tab 1: Defaults (merged Grid Settings + Panel Defaults) ===
    let defaults_scroll = gtk4::ScrolledWindow::new();
    defaults_scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);
    defaults_scroll.set_vexpand(true);

    let defaults_tab_box = GtkBox::new(Orientation::Vertical, 12);
    defaults_tab_box.set_margin_start(12);
    defaults_tab_box.set_margin_end(12);
    defaults_tab_box.set_margin_top(12);
    defaults_tab_box.set_margin_bottom(12);

    // --- Grid Settings Section ---
    let grid_section_label = Label::new(Some("Grid Settings"));
    grid_section_label.add_css_class("heading");
    grid_section_label.set_halign(gtk4::Align::Start);
    defaults_tab_box.append(&grid_section_label);

    // Cell Width
    let cell_width_box = GtkBox::new(Orientation::Horizontal, 6);
    cell_width_box.set_margin_start(12);
    cell_width_box.append(&Label::new(Some("Cell Width:")));
    let cell_width_spin = SpinButton::with_range(10.0, 1000.0, 10.0);
    cell_width_spin.set_value(app_config.borrow().grid.cell_width as f64);
    cell_width_spin.set_hexpand(true);
    cell_width_box.append(&cell_width_spin);
    cell_width_box.append(&Label::new(Some("px")));
    defaults_tab_box.append(&cell_width_box);

    // Cell Height
    let cell_height_box = GtkBox::new(Orientation::Horizontal, 6);
    cell_height_box.set_margin_start(12);
    cell_height_box.append(&Label::new(Some("Cell Height:")));
    let cell_height_spin = SpinButton::with_range(10.0, 1000.0, 10.0);
    cell_height_spin.set_value(app_config.borrow().grid.cell_height as f64);
    cell_height_spin.set_hexpand(true);
    cell_height_box.append(&cell_height_spin);
    cell_height_box.append(&Label::new(Some("px")));
    defaults_tab_box.append(&cell_height_box);

    // Spacing
    let spacing_box = GtkBox::new(Orientation::Horizontal, 6);
    spacing_box.set_margin_start(12);
    spacing_box.append(&Label::new(Some("Spacing:")));
    let spacing_spin = SpinButton::with_range(0.0, 50.0, 1.0);
    spacing_spin.set_value(app_config.borrow().grid.spacing as f64);
    spacing_spin.set_hexpand(true);
    spacing_box.append(&spacing_spin);
    spacing_box.append(&Label::new(Some("px")));
    defaults_tab_box.append(&spacing_box);

    // --- Default Panel Size Section ---
    let panel_size_label = Label::new(Some("Default Panel Size"));
    panel_size_label.add_css_class("heading");
    panel_size_label.set_halign(gtk4::Align::Start);
    panel_size_label.set_margin_top(12);
    defaults_tab_box.append(&panel_size_label);

    let panel_size_help = Label::new(Some("Size in grid cells for new panels"));
    panel_size_help.set_halign(gtk4::Align::Start);
    panel_size_help.set_margin_start(12);
    panel_size_help.add_css_class("dim-label");
    defaults_tab_box.append(&panel_size_help);

    let panel_size_box = GtkBox::new(Orientation::Horizontal, 12);
    panel_size_box.set_margin_start(12);

    let panel_width_box = GtkBox::new(Orientation::Horizontal, 6);
    panel_width_box.append(&Label::new(Some("Width:")));
    let panel_width_spin = SpinButton::with_range(1.0, 20.0, 1.0);
    panel_width_spin.set_value(defaults_config.borrow().general.default_panel_width as f64);
    panel_width_box.append(&panel_width_spin);
    panel_width_box.append(&Label::new(Some("cells")));
    panel_size_box.append(&panel_width_box);

    let panel_height_box = GtkBox::new(Orientation::Horizontal, 6);
    panel_height_box.append(&Label::new(Some("Height:")));
    let panel_height_spin = SpinButton::with_range(1.0, 20.0, 1.0);
    panel_height_spin.set_value(defaults_config.borrow().general.default_panel_height as f64);
    panel_height_box.append(&panel_height_spin);
    panel_height_box.append(&Label::new(Some("cells")));
    panel_size_box.append(&panel_height_box);

    defaults_tab_box.append(&panel_size_box);

    // --- Default Panel Appearance Section ---
    let panel_appearance_label = Label::new(Some("Default Panel Appearance"));
    panel_appearance_label.add_css_class("heading");
    panel_appearance_label.set_halign(gtk4::Align::Start);
    panel_appearance_label.set_margin_top(12);
    defaults_tab_box.append(&panel_appearance_label);

    // Corner radius
    let corner_radius_box = GtkBox::new(Orientation::Horizontal, 6);
    corner_radius_box.set_margin_start(12);
    corner_radius_box.append(&Label::new(Some("Corner Radius:")));
    let corner_radius_spin = SpinButton::with_range(0.0, 50.0, 1.0);
    corner_radius_spin.set_value(defaults_config.borrow().general.default_corner_radius);
    corner_radius_spin.set_hexpand(true);
    corner_radius_box.append(&corner_radius_spin);
    defaults_tab_box.append(&corner_radius_box);

    // Border enabled
    let border_enabled_check = CheckButton::with_label("Show Border on New Panels");
    border_enabled_check.set_active(defaults_config.borrow().general.default_border.enabled);
    border_enabled_check.set_margin_start(12);
    defaults_tab_box.append(&border_enabled_check);

    // Border width
    let border_width_box = GtkBox::new(Orientation::Horizontal, 6);
    border_width_box.set_margin_start(12);
    border_width_box.append(&Label::new(Some("Border Width:")));
    let border_width_spin = SpinButton::with_range(0.5, 10.0, 0.5);
    border_width_spin.set_value(defaults_config.borrow().general.default_border.width);
    border_width_spin.set_hexpand(true);
    border_width_box.append(&border_width_spin);
    defaults_tab_box.append(&border_width_box);

    // Border color button
    let border_color_btn = Button::with_label("Border Color");
    border_color_btn.set_margin_start(12);
    defaults_tab_box.append(&border_color_btn);

    // Store border color in a shared Rc<RefCell>
    let border_color = Rc::new(RefCell::new(defaults_config.borrow().general.default_border.color));

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

    // Default background
    let bg_label = Label::new(Some("Default Background:"));
    bg_label.set_halign(gtk4::Align::Start);
    bg_label.set_margin_start(12);
    bg_label.set_margin_top(6);
    defaults_tab_box.append(&bg_label);

    let default_bg_widget = BackgroundConfigWidget::new();
    default_bg_widget.set_config(defaults_config.borrow().general.default_background.clone());
    let default_bg_widget_box = GtkBox::new(Orientation::Vertical, 0);
    default_bg_widget_box.set_margin_start(12);
    default_bg_widget_box.append(default_bg_widget.widget());
    defaults_tab_box.append(&default_bg_widget_box);
    let default_bg_widget = Rc::new(default_bg_widget);

    // --- Displayer Defaults Section ---
    let displayer_defaults_label = Label::new(Some("Saved Displayer Defaults"));
    displayer_defaults_label.add_css_class("heading");
    displayer_defaults_label.set_halign(gtk4::Align::Start);
    displayer_defaults_label.set_margin_top(12);
    defaults_tab_box.append(&displayer_defaults_label);

    let displayer_help = Label::new(Some("Right-click a panel and select 'Set as Default Style' to save displayer defaults"));
    displayer_help.set_halign(gtk4::Align::Start);
    displayer_help.set_margin_start(12);
    displayer_help.add_css_class("dim-label");
    displayer_help.set_wrap(true);
    defaults_tab_box.append(&displayer_help);

    // Container for displayer defaults list
    let displayer_list_box = GtkBox::new(Orientation::Vertical, 4);
    displayer_list_box.set_margin_start(12);
    displayer_list_box.set_margin_top(6);

    // Refresh function to populate the list
    let defaults_config_for_list = defaults_config.clone();
    let displayer_list_box_clone = displayer_list_box.clone();
    let refresh_displayer_list = Rc::new(RefCell::new(None::<Box<dyn Fn()>>));
    let refresh_displayer_list_clone = refresh_displayer_list.clone();

    let refresh_fn = {
        let defaults_config = defaults_config_for_list.clone();
        let list_box = displayer_list_box_clone.clone();
        let refresh_self = refresh_displayer_list_clone.clone();
        move || {
            // Clear existing children
            while let Some(child) = list_box.first_child() {
                list_box.remove(&child);
            }

            let ids = defaults_config.borrow().get_displayer_default_ids();
            if ids.is_empty() {
                let no_defaults_label = Label::new(Some("No displayer defaults saved"));
                no_defaults_label.add_css_class("dim-label");
                no_defaults_label.set_halign(gtk4::Align::Start);
                list_box.append(&no_defaults_label);
            } else {
                for id in ids {
                    let row = GtkBox::new(Orientation::Horizontal, 6);

                    let id_label = Label::new(Some(&id));
                    id_label.set_hexpand(true);
                    id_label.set_halign(gtk4::Align::Start);
                    row.append(&id_label);

                    let clear_btn = Button::with_label("Clear");
                    clear_btn.add_css_class("destructive-action");
                    let defaults_clone = defaults_config.clone();
                    let id_clone = id.clone();
                    let refresh_clone = refresh_self.clone();
                    clear_btn.connect_clicked(move |_| {
                        defaults_clone.borrow_mut().remove_displayer_default(&id_clone);
                        if let Err(e) = defaults_clone.borrow().save() {
                            log::warn!("Failed to save defaults after clearing: {}", e);
                        }
                        // Refresh the list
                        if let Some(ref f) = *refresh_clone.borrow() {
                            f();
                        }
                    });
                    row.append(&clear_btn);

                    list_box.append(&row);
                }
            }
        }
    };

    // Store the refresh function
    *refresh_displayer_list.borrow_mut() = Some(Box::new(refresh_fn.clone()));

    // Initial population
    refresh_fn();

    defaults_tab_box.append(&displayer_list_box);

    // Clear All button
    let clear_all_btn = Button::with_label("Clear All Displayer Defaults");
    clear_all_btn.add_css_class("destructive-action");
    clear_all_btn.set_margin_start(12);
    clear_all_btn.set_margin_top(6);
    clear_all_btn.set_halign(gtk4::Align::Start);
    let defaults_for_clear_all = defaults_config.clone();
    let refresh_for_clear_all = refresh_displayer_list.clone();
    clear_all_btn.connect_clicked(move |_| {
        defaults_for_clear_all.borrow_mut().clear_displayer_defaults();
        if let Err(e) = defaults_for_clear_all.borrow().save() {
            log::warn!("Failed to save defaults after clearing all: {}", e);
        }
        // Refresh the list
        if let Some(ref f) = *refresh_for_clear_all.borrow() {
            f();
        }
    });
    defaults_tab_box.append(&clear_all_btn);

    defaults_scroll.set_child(Some(&defaults_tab_box));
    notebook.append_page(&defaults_scroll, Some(&Label::new(Some("Defaults"))));

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

    // === Tab 3: Window Mode ===
    // Wrap in ScrolledWindow since this tab has a lot of content
    let window_mode_scroll = gtk4::ScrolledWindow::new();
    window_mode_scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);
    window_mode_scroll.set_vexpand(true);

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
        "When borderless mode is active, hold Ctrl and drag:\n\
         • Near edges/corners to resize the window\n\
         • In center area to move the window"
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

    // Auto-scroll section
    let auto_scroll_label = Label::new(Some("Auto-Scroll"));
    auto_scroll_label.add_css_class("heading");
    auto_scroll_label.set_halign(gtk4::Align::Start);
    auto_scroll_label.set_margin_top(18);
    window_mode_tab_box.append(&auto_scroll_label);

    // Auto-scroll enabled
    let auto_scroll_check = CheckButton::with_label("Auto-scroll when content extends beyond window");
    auto_scroll_check.set_active(app_config.borrow().window.auto_scroll_enabled);
    auto_scroll_check.set_margin_start(12);
    window_mode_tab_box.append(&auto_scroll_check);

    // Auto-scroll delay
    let delay_box = GtkBox::new(Orientation::Horizontal, 6);
    delay_box.set_margin_start(12);
    delay_box.append(&Label::new(Some("Scroll delay:")));

    let delay_spin = SpinButton::with_range(500.0, 60000.0, 500.0);
    delay_spin.set_value(app_config.borrow().window.auto_scroll_delay_ms as f64);
    delay_spin.set_hexpand(true);
    delay_spin.set_sensitive(auto_scroll_check.is_active());
    delay_box.append(&delay_spin);
    delay_box.append(&Label::new(Some("ms")));
    window_mode_tab_box.append(&delay_box);

    // Whole pages checkbox
    let whole_pages_check = CheckButton::with_label("Scroll whole pages only");
    whole_pages_check.set_active(app_config.borrow().window.auto_scroll_whole_pages);
    whole_pages_check.set_margin_start(12);
    whole_pages_check.set_sensitive(auto_scroll_check.is_active());
    window_mode_tab_box.append(&whole_pages_check);

    // Enable/disable delay spin and whole pages based on checkbox
    let delay_spin_clone = delay_spin.clone();
    let whole_pages_check_clone = whole_pages_check.clone();
    auto_scroll_check.connect_toggled(move |check| {
        delay_spin_clone.set_sensitive(check.is_active());
        whole_pages_check_clone.set_sensitive(check.is_active());
    });

    // Auto-scroll help text
    let auto_scroll_help = Label::new(Some("Scrolls one viewport page at a time. When 'whole pages only' is enabled, scrolls through complete page grid regardless of panel positions."));
    auto_scroll_help.set_halign(gtk4::Align::Start);
    auto_scroll_help.set_margin_start(12);
    auto_scroll_help.set_margin_top(6);
    auto_scroll_help.add_css_class("dim-label");
    auto_scroll_help.set_wrap(true);
    window_mode_tab_box.append(&auto_scroll_help);

    // Viewport dimensions for auto-scroll page boundaries
    let viewport_label = Label::new(Some("Viewport Page Dimensions"));
    viewport_label.set_halign(gtk4::Align::Start);
    viewport_label.set_margin_top(12);
    viewport_label.add_css_class("heading");
    window_mode_tab_box.append(&viewport_label);

    let viewport_help = Label::new(Some("Define the page size for auto-scroll boundaries. Shown as dashed rectangles when dragging panels."));
    viewport_help.set_halign(gtk4::Align::Start);
    viewport_help.set_margin_start(12);
    viewport_help.add_css_class("dim-label");
    viewport_help.set_wrap(true);
    window_mode_tab_box.append(&viewport_help);

    // Viewport width/height inputs
    let viewport_dims_box = GtkBox::new(Orientation::Horizontal, 12);
    viewport_dims_box.set_margin_start(12);
    viewport_dims_box.set_margin_top(6);

    let vp_width_box = GtkBox::new(Orientation::Horizontal, 4);
    vp_width_box.append(&Label::new(Some("Width:")));
    let viewport_width_spin = SpinButton::with_range(0.0, 10000.0, 10.0);
    viewport_width_spin.set_value(app_config.borrow().window.viewport_width as f64);
    viewport_width_spin.set_width_chars(6);
    vp_width_box.append(&viewport_width_spin);
    vp_width_box.append(&Label::new(Some("px")));
    viewport_dims_box.append(&vp_width_box);

    let vp_height_box = GtkBox::new(Orientation::Horizontal, 4);
    vp_height_box.append(&Label::new(Some("Height:")));
    let viewport_height_spin = SpinButton::with_range(0.0, 10000.0, 10.0);
    viewport_height_spin.set_value(app_config.borrow().window.viewport_height as f64);
    viewport_height_spin.set_width_chars(6);
    vp_height_box.append(&viewport_height_spin);
    vp_height_box.append(&Label::new(Some("px")));
    viewport_dims_box.append(&vp_height_box);

    window_mode_tab_box.append(&viewport_dims_box);

    // Copy buttons
    let copy_buttons_box = GtkBox::new(Orientation::Horizontal, 6);
    copy_buttons_box.set_margin_start(12);
    copy_buttons_box.set_margin_top(6);

    let copy_window_btn = Button::with_label("Copy from Window");
    let copy_monitor_btn = Button::with_label("Copy from Monitor");

    // Monitor dropdown for copy
    let monitor_list = StringList::new(&[]);
    if let Some(display) = gtk4::gdk::Display::default() {
        let monitors = display.monitors();
        for i in 0..monitors.n_items() {
            if let Some(mon) = monitors.item(i) {
                if let Ok(monitor) = mon.downcast::<gtk4::gdk::Monitor>() {
                    let geom = monitor.geometry();
                    let name = monitor.model().map(|s| s.to_string()).unwrap_or_else(|| format!("Monitor {}", i));
                    monitor_list.append(&format!("{} ({}x{})", name, geom.width(), geom.height()));
                }
            }
        }
    }
    let vp_monitor_dropdown = DropDown::new(Some(monitor_list.clone()), None::<gtk4::Expression>);
    vp_monitor_dropdown.set_selected(0);

    copy_buttons_box.append(&copy_window_btn);
    copy_buttons_box.append(&copy_monitor_btn);
    copy_buttons_box.append(&vp_monitor_dropdown);
    window_mode_tab_box.append(&copy_buttons_box);

    // Connect copy from window button
    {
        let parent_clone = parent_window.clone();
        let vp_width = viewport_width_spin.clone();
        let vp_height = viewport_height_spin.clone();
        copy_window_btn.connect_clicked(move |_| {
            vp_width.set_value(parent_clone.width() as f64);
            vp_height.set_value(parent_clone.height() as f64);
        });
    }

    // Connect copy from monitor button
    {
        let vp_width = viewport_width_spin.clone();
        let vp_height = viewport_height_spin.clone();
        let monitor_dd = vp_monitor_dropdown.clone();
        copy_monitor_btn.connect_clicked(move |_| {
            if let Some(display) = gtk4::gdk::Display::default() {
                let selected = monitor_dd.selected();
                if let Some(mon) = display.monitors().item(selected) {
                    if let Ok(monitor) = mon.downcast::<gtk4::gdk::Monitor>() {
                        let geom = monitor.geometry();
                        vp_width.set_value(geom.width() as f64);
                        vp_height.set_value(geom.height() as f64);
                    }
                }
            }
        });
    }

    // Zero = use window size help text
    let vp_zero_help = Label::new(Some("Set to 0 to use current window dimensions"));
    vp_zero_help.set_halign(gtk4::Align::Start);
    vp_zero_help.set_margin_start(12);
    vp_zero_help.set_margin_top(4);
    vp_zero_help.add_css_class("dim-label");
    window_mode_tab_box.append(&vp_zero_help);

    // Grid overlay shortcuts help
    let grid_shortcuts_label = Label::new(Some("Grid Overlay Shortcuts"));
    grid_shortcuts_label.set_halign(gtk4::Align::Start);
    grid_shortcuts_label.set_margin_top(12);
    grid_shortcuts_label.add_css_class("heading");
    window_mode_tab_box.append(&grid_shortcuts_label);

    let grid_shortcuts_help = Label::new(Some(
        "• Hold Space to show the cell grid and viewport boundaries\n\
         • Grid also appears automatically when resizing the window\n\
         • Grid appears when dragging panels"
    ));
    grid_shortcuts_help.set_halign(gtk4::Align::Start);
    grid_shortcuts_help.set_margin_start(12);
    grid_shortcuts_help.set_margin_top(4);
    grid_shortcuts_help.add_css_class("dim-label");
    grid_shortcuts_help.set_wrap(true);
    window_mode_tab_box.append(&grid_shortcuts_help);

    // Set the scrolled window content and add to notebook
    window_mode_scroll.set_child(Some(&window_mode_tab_box));
    notebook.append_page(&window_mode_scroll, Some(&Label::new(Some("Window Mode"))));

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
    let auto_scroll_check_clone = auto_scroll_check.clone();
    let delay_spin_clone = delay_spin.clone();
    let whole_pages_check_clone = whole_pages_check.clone();
    let viewport_width_spin_clone = viewport_width_spin.clone();
    let viewport_height_spin_clone = viewport_height_spin.clone();
    let parent_window_clone = parent_window.clone();
    let on_auto_scroll_change_clone = on_auto_scroll_change.clone();
    // Clones for defaults
    let defaults_config_clone = defaults_config.clone();
    let panel_width_spin_clone = panel_width_spin.clone();
    let panel_height_spin_clone = panel_height_spin.clone();
    let default_bg_widget_clone = default_bg_widget.clone();

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

        // Update defaults.json with panel defaults
        {
            let mut defaults = defaults_config_clone.borrow_mut();
            defaults.general.grid_cell_width = new_cell_width as u32;
            defaults.general.grid_cell_height = new_cell_height as u32;
            defaults.general.grid_spacing = new_spacing as u32;
            defaults.general.default_panel_width = panel_width_spin_clone.value() as u32;
            defaults.general.default_panel_height = panel_height_spin_clone.value() as u32;
            defaults.general.default_corner_radius = corner_radius_spin_clone.value();
            defaults.general.default_border.enabled = border_enabled_check_clone.is_active();
            defaults.general.default_border.width = border_width_spin_clone.value();
            defaults.general.default_border.color = *border_color_clone.borrow();
            defaults.general.default_background = default_bg_widget_clone.get_config();
            if let Err(e) = defaults.save() {
                log::warn!("Failed to save defaults.json: {}", e);
            }
        }

        // Update app config
        let mut cfg = app_config_clone.borrow_mut();
        cfg.window.background = new_background.clone();
        cfg.grid.cell_width = new_cell_width;
        cfg.grid.cell_height = new_cell_height;
        cfg.grid.spacing = new_spacing;
        // Also update AppConfig panel defaults for backward compatibility
        cfg.window.panel_corner_radius = corner_radius_spin_clone.value();
        cfg.window.panel_border.enabled = border_enabled_check_clone.is_active();
        cfg.window.panel_border.width = border_width_spin_clone.value();
        cfg.window.panel_border.color = *border_color_clone.borrow();
        cfg.window.fullscreen_enabled = fullscreen_enabled;
        cfg.window.fullscreen_monitor = fullscreen_monitor;
        cfg.window.borderless = borderless;
        cfg.window.auto_scroll_enabled = auto_scroll_check_clone.is_active();
        cfg.window.auto_scroll_delay_ms = delay_spin_clone.value() as u64;
        cfg.window.auto_scroll_whole_pages = whole_pages_check_clone.is_active();
        cfg.window.viewport_width = viewport_width_spin_clone.value() as i32;
        cfg.window.viewport_height = viewport_height_spin_clone.value() as i32;

        // Calculate effective viewport size (use window size if set to 0)
        let vp_width = if cfg.window.viewport_width > 0 {
            cfg.window.viewport_width
        } else {
            parent_window_clone.width()
        };
        let vp_height = if cfg.window.viewport_height > 0 {
            cfg.window.viewport_height
        } else {
            parent_window_clone.height()
        };
        drop(cfg);

        // Update grid layout viewport size for drag visualization
        grid_layout_clone.borrow().set_viewport_size(vp_width, vp_height);

        // Apply borderless state to parent window
        log::info!("Setting window decorated: {} (borderless: {})", !borderless, borderless);
        parent_window_clone.set_decorated(!borderless);
        // Force layout update after decoration change
        parent_window_clone.queue_resize();

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
        config_dirty_clone.store(true, Ordering::Relaxed);

        // Restart auto-scroll timer with new settings
        on_auto_scroll_change_clone();

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
#[allow(clippy::too_many_arguments)]
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
    use rg_sens::core::{PanelData, PanelAppearance, SourceConfig, DisplayerConfig};
    use rg_sens::config::DefaultsConfig;

    // Load defaults to check for displayer-specific defaults
    let defaults = DefaultsConfig::load();

    // Try to use saved displayer default, fall back to built-in default
    let displayer_config = if let Some(saved_config) = defaults.get_displayer_default(displayer_id) {
        // Try to deserialize the saved config into the proper DisplayerConfig type
        DisplayerConfig::from_value_for_type(displayer_id, saved_config.clone())
            .unwrap_or_else(|| DisplayerConfig::default_for_type(displayer_id).unwrap_or_default())
    } else {
        DisplayerConfig::default_for_type(displayer_id).unwrap_or_default()
    };

    // Create PanelData with proper defaults for the source and displayer types
    let panel_data = PanelData {
        id: id.to_string(),
        geometry: PanelGeometry { x, y, width, height },
        source_config: SourceConfig::default_for_type(source_id).unwrap_or_default(),
        displayer_config,
        appearance: PanelAppearance {
            background,
            corner_radius,
            border,
            scale: 1.0,
            translate_x: 0.0,
            translate_y: 0.0,
            z_index: 0,
            ignore_collision: false,
        },
    };

    // Create panel from PanelData (this properly initializes panel.data)
    let mut panel = Panel::from_data_with_registry(panel_data, registry)?;

    // Apply additional settings if provided (for backward compatibility)
    if !settings.is_empty() {
        panel.apply_config(settings)?;
    }

    Ok(Arc::new(RwLock::new(panel)))
}

/// Create a panel from PanelData (new unified format)
fn create_panel_from_data(
    data: PanelData,
    registry: &rg_sens::core::Registry,
) -> anyhow::Result<Arc<RwLock<Panel>>> {
    // Use Panel::from_data_with_registry which handles everything
    let panel = Panel::from_data_with_registry(data, registry)?;
    Ok(Arc::new(RwLock::new(panel)))
}

/// Show dialog to create a new panel
fn show_new_panel_dialog(
    window: &ApplicationWindow,
    grid_layout: &Rc<RefCell<GridLayout>>,
    config_dirty: &Arc<AtomicBool>,
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

    // Size section - use defaults from DefaultsConfig
    let defaults = rg_sens::config::DefaultsConfig::load();
    let default_width = defaults.general.default_panel_width as f64;
    let default_height = defaults.general.default_panel_height as f64;

    let size_label = Label::new(Some("Size:"));
    size_label.set_halign(gtk4::Align::Start);
    vbox.append(&size_label);

    let size_box = GtkBox::new(Orientation::Horizontal, 6);
    size_box.append(&Label::new(Some("Width:")));
    let width_adj = Adjustment::new(default_width, 1.0, 512.0, 1.0, 5.0, 0.0);
    let width_spin = SpinButton::new(Some(&width_adj), 1.0, 0);
    width_spin.set_hexpand(true);
    size_box.append(&width_spin);

    size_box.append(&Label::new(Some("Height:")));
    let height_adj = Adjustment::new(default_height, 1.0, 512.0, 1.0, 5.0, 0.0);
    let height_spin = SpinButton::new(Some(&height_adj), 1.0, 0);
    height_spin.set_hexpand(true);
    size_box.append(&height_spin);
    vbox.append(&size_box);

    // Data Source
    let source_label = Label::new(Some("Data Source:"));
    source_label.set_halign(gtk4::Align::Start);
    vbox.append(&source_label);

    let registry = rg_sens::core::global_registry();
    let source_infos = registry.list_sources_with_info();
    let source_ids: Vec<String> = source_infos.iter().map(|s| s.id.clone()).collect();
    let source_display_names: Vec<String> = source_infos.iter().map(|s| s.display_name.clone()).collect();
    let source_strings: Vec<&str> = source_display_names.iter().map(|s| s.as_str()).collect();
    let source_list = StringList::new(&source_strings);
    let source_combo = DropDown::new(Some(source_list), Option::<gtk4::Expression>::None);
    source_combo.set_selected(0);
    vbox.append(&source_combo);

    // Displayer - filtered based on selected source
    let displayer_label = Label::new(Some("Display Type:"));
    displayer_label.set_halign(gtk4::Align::Start);
    vbox.append(&displayer_label);

    // Get compatible displayers for the first source
    let first_source_id = source_ids.first().map(|s| s.as_str()).unwrap_or("cpu");
    let displayer_infos = registry.get_compatible_displayers(first_source_id);
    let displayer_ids: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(
        displayer_infos.iter().map(|d| d.id.clone()).collect()
    ));
    let displayer_display_names: Vec<String> = displayer_infos.iter().map(|d| d.display_name.clone()).collect();
    let displayer_strings: Vec<&str> = displayer_display_names.iter().map(|s| s.as_str()).collect();
    let displayer_list = StringList::new(&displayer_strings);
    let displayer_combo = DropDown::new(Some(displayer_list), Option::<gtk4::Expression>::None);
    displayer_combo.set_selected(0);
    vbox.append(&displayer_combo);

    // Update displayer dropdown when source changes
    let source_ids_for_change = source_ids.clone();
    let displayer_ids_for_change = displayer_ids.clone();
    let displayer_combo_clone = displayer_combo.clone();
    source_combo.connect_selected_notify(move |combo| {
        let selected_idx = combo.selected() as usize;
        if let Some(source_id) = source_ids_for_change.get(selected_idx) {
            let new_displayer_infos = registry.get_compatible_displayers(source_id);
            let new_displayer_ids: Vec<String> = new_displayer_infos.iter().map(|d| d.id.clone()).collect();
            let new_display_names: Vec<String> = new_displayer_infos.iter().map(|d| d.display_name.clone()).collect();

            // Update stored displayer IDs
            *displayer_ids_for_change.borrow_mut() = new_displayer_ids;

            // Update dropdown model
            let display_strs: Vec<&str> = new_display_names.iter().map(|s| s.as_str()).collect();
            let new_list = StringList::new(&display_strs);
            displayer_combo_clone.set_model(Some(&new_list));
            displayer_combo_clone.set_selected(0);
        }
    });

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
    let displayer_ids_for_ok = displayer_ids.clone();
    ok_button.connect_clicked(move |_| {
        let x = x_spin.value() as u32;
        let y = y_spin.value() as u32;
        let width = width_spin.value() as u32;
        let height = height_spin.value() as u32;

        let source_id = &source_ids[source_combo.selected() as usize];
        let displayer_ids_borrowed = displayer_ids_for_ok.borrow();
        let displayer_id = &displayer_ids_borrowed[displayer_combo.selected() as usize];

        // Generate unique ID
        let id = format!("panel_{}", uuid::Uuid::new_v4());

        info!("Creating new panel: id={}, pos=({},{}), size={}x{}, source={}, displayer={}",
              id, x, y, width, height, source_id, displayer_id);

        // Load defaults for appearance
        let defaults = rg_sens::config::DefaultsConfig::load();
        let background = defaults.general.default_background.clone();
        let corner_radius = defaults.general.default_corner_radius;
        let border = defaults.general.default_border.clone();
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
            registry,
        ) {
            Ok(panel) => {
                // Add to grid (grid_layout maintains its own panels list)
                grid_layout.borrow_mut().add_panel(panel.clone());

                // Register with update manager so it gets periodic updates
                if let Some(update_manager) = rg_sens::core::global_update_manager() {
                    update_manager.queue_add_panel(panel.clone());
                }

                // Mark config as dirty
                config_dirty.store(true, Ordering::Relaxed);

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

/// Apply system color scheme preference (dark/light mode)
fn apply_system_color_scheme() {
    // Try to get the system color scheme from the freedesktop portal
    // color-scheme values: 0 = no preference, 1 = prefer dark, 2 = prefer light
    let prefer_dark = detect_system_dark_mode();

    if let Some(settings) = gtk4::Settings::default() {
        settings.set_gtk_application_prefer_dark_theme(prefer_dark);
        info!("Applied system color scheme: {}", if prefer_dark { "dark" } else { "light" });
    }

    // Listen for changes to the system color scheme
    setup_color_scheme_listener();
}

// Thread-local holder for the settings object to keep it alive for the listener
thread_local! {
    static COLOR_SCHEME_SETTINGS: RefCell<Option<gtk4::gio::Settings>> = const { RefCell::new(None) };
}

/// Set up a listener for system color scheme changes
fn setup_color_scheme_listener() {
    // Listen to GNOME settings changes if available
    let schema_source = match gtk4::gio::SettingsSchemaSource::default() {
        Some(source) => source,
        None => return,
    };

    if schema_source.lookup("org.gnome.desktop.interface", true).is_some() {
        let settings = gtk4::gio::Settings::new("org.gnome.desktop.interface");
        settings.connect_changed(Some("color-scheme"), |settings, _key| {
            let color_scheme: String = settings.string("color-scheme").to_string();
            let prefer_dark = color_scheme == "prefer-dark";

            if let Some(gtk_settings) = gtk4::Settings::default() {
                gtk_settings.set_gtk_application_prefer_dark_theme(prefer_dark);
                info!("System color scheme changed: {}", if prefer_dark { "dark" } else { "light" });
            }
        });
        // Store settings in thread-local to keep it alive for the lifetime of the app
        COLOR_SCHEME_SETTINGS.with(|cell| {
            *cell.borrow_mut() = Some(settings);
        });
    }
}

/// Detect if the system prefers dark mode
fn detect_system_dark_mode() -> bool {
    // Method 1: Check freedesktop portal settings via GSettings
    // This works on GNOME, KDE Plasma 5.24+, and other modern desktops
    if let Some(prefer_dark) = check_portal_color_scheme() {
        return prefer_dark;
    }

    // Method 2: Check GNOME settings directly
    if let Some(prefer_dark) = check_gnome_color_scheme() {
        return prefer_dark;
    }

    // Method 3: Check GTK_THEME environment variable for dark variant
    if let Ok(theme) = std::env::var("GTK_THEME") {
        if theme.to_lowercase().contains("dark") {
            return true;
        }
    }

    // Default to light mode if no preference detected
    false
}

/// Check the freedesktop portal for color scheme preference
fn check_portal_color_scheme() -> Option<bool> {
    // Try to read from the portal settings
    // org.freedesktop.appearance color-scheme: 0=no-preference, 1=dark, 2=light
    let schema_source = gtk4::gio::SettingsSchemaSource::default()?;

    // Check if the schema exists before creating Settings
    if schema_source.lookup("org.freedesktop.appearance", true).is_some() {
        let settings = gtk4::gio::Settings::new("org.freedesktop.appearance");
        let color_scheme: u32 = settings.uint("color-scheme");
        return Some(color_scheme == 1); // 1 = prefer dark
    }
    None
}

/// Check GNOME desktop settings for color scheme
fn check_gnome_color_scheme() -> Option<bool> {
    // Check org.gnome.desktop.interface color-scheme
    // Values: "default", "prefer-dark", "prefer-light"
    let schema_source = gtk4::gio::SettingsSchemaSource::default()?;

    // Check if the schema exists
    if schema_source.lookup("org.gnome.desktop.interface", true).is_some() {
        let settings = gtk4::gio::Settings::new("org.gnome.desktop.interface");
        let color_scheme: String = settings.string("color-scheme").to_string();
        return Some(color_scheme == "prefer-dark");
    }
    None
}

