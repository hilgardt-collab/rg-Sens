use clap::{Parser, ValueEnum};
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::Application;
use log::{error, info, warn};
use rg_sens::config::AppConfig;
use rg_sens::core::{PanelData, PanelGeometry, UpdateManager};
use rg_sens::ui::{GridConfig as UiGridConfig, GridLayout, theme, window_settings_dialog, new_panel_dialog, config_helpers, context_menu, auto_scroll};
use rg_sens::{displayers, sources};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

const APP_ID: &str = "rg-sens";

/// Graphics renderer to use
#[derive(Debug, Clone, Copy, ValueEnum)]
enum Renderer {
    /// OpenGL renderer (recommended for stability)
    Gl,
    /// New OpenGL renderer (GTK 4.14+)
    Ngl,
    /// Vulkan renderer (may have issues with some drivers)
    Vulkan,
    /// Cairo software renderer (slowest but most compatible)
    Cairo,
}

impl std::fmt::Display for Renderer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Renderer::Gl => write!(f, "gl"),
            Renderer::Ngl => write!(f, "ngl"),
            Renderer::Vulkan => write!(f, "vulkan"),
            Renderer::Cairo => write!(f, "cairo"),
        }
    }
}

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

    /// Graphics renderer (gl, ngl, vulkan, cairo). Use 'gl' if you experience crashes in fullscreen.
    #[arg(short = 'r', long = "renderer", value_name = "RENDERER")]
    renderer: Option<Renderer>,

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

/// Set the application icon by adding icon search paths
fn set_app_icon() {
    let Some(display) = gtk4::gdk::Display::default() else {
        warn!("No display available for icon theme");
        return;
    };

    let icon_theme = gtk4::IconTheme::for_display(&display);

    // Add search paths for the icon theme
    // GTK expects: {search_path}/hicolor/{size}/apps/rg-sens.png
    let search_paths = [
        concat!(env!("CARGO_MANIFEST_DIR"), "/data/icons"),  // Development: data/icons
        "/usr/share/icons",
        "/usr/local/share/icons",
    ];

    for path in &search_paths {
        icon_theme.add_search_path(path);
        info!("Added icon search path: {}", path);
    }

    // Set the default icon name for all windows
    gtk4::Window::set_default_icon_name("rg-sens");

    // Check if icon is available
    if icon_theme.has_icon("rg-sens") {
        info!("Application icon 'rg-sens' found in icon theme");
    } else {
        warn!("Application icon 'rg-sens' not found. Search paths: {:?}", search_paths);
    }
}

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

    // Set graphics renderer before GTK initialization
    // GSK_RENDERER must be set before any GTK calls
    if let Some(renderer) = &cli.renderer {
        let renderer_str = renderer.to_string();
        std::env::set_var("GSK_RENDERER", &renderer_str);
        warn!("Using {} renderer (set via --renderer)", renderer_str);
    } else if std::env::var("GSK_RENDERER").is_err() {
        // Log the default renderer being used
        info!("Using default GTK renderer (use --renderer to change)");
    } else {
        info!("Using GSK_RENDERER={} from environment", std::env::var("GSK_RENDERER").unwrap());
    }

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

    // Set application icon (must be done after GTK is initialized)
    set_app_icon();

    // Get CLI options
    let cli = CLI_OPTIONS.get().cloned().unwrap_or(Cli {
        fullscreen: None,
        borderless: None,
        windowed: false,
        at: None,
        list_monitors: false,
        debug: 0,
        renderer: None,
        layout_file: None,
    });

    // Load CSS for selection styling
    theme::load_css();

    // Apply system color scheme (dark/light mode)
    theme::apply_system_color_scheme();

    // Pre-warm font cache and dialog (loads system fonts once at startup)
    rg_sens::ui::warm_font_cache();
    rg_sens::ui::init_shared_font_dialog();

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
    let mut grid_layout = GridLayout::new(grid_config, app_config.clone());

    // Create registry
    let registry = rg_sens::core::global_registry();

    // Create shared source manager (ensures each source type is polled only once)
    let shared_source_manager = Arc::new(rg_sens::core::SharedSourceManager::new());
    rg_sens::core::init_global_shared_source_manager(shared_source_manager.clone());

    // Create update manager
    let update_manager = Arc::new(UpdateManager::new());

    // Initialize global update manager so new panels can register themselves
    rg_sens::core::init_global_update_manager(update_manager.clone());

    // Initialize global animation manager for centralized animation timing
    rg_sens::core::init_global_animation_manager();

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
            match new_panel_dialog::create_panel_from_data(panel_data, registry) {
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
            match new_panel_dialog::create_panel_from_data(panel_data, registry) {
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

    // Apply global theme to all panels (displayers need the theme for color resolution)
    {
        let global_theme = app_config.borrow().global_theme.clone();
        let theme_value = serde_json::to_value(&global_theme).unwrap_or_default();
        let mut theme_config = std::collections::HashMap::new();
        theme_config.insert("global_theme".to_string(), theme_value);

        for panel in &panels {
            if let Ok(mut panel_guard) = panel.try_write() {
                let _ = panel_guard.displayer.apply_config(&theme_config);
            }
        }
    }

    // Debug: Print shared source statistics to verify source sharing
    if let Some(manager) = rg_sens::core::global_shared_source_manager() {
        manager.debug_print_sources();
    }

    // Create window background - sized to match grid content
    let window_background = gtk4::DrawingArea::new();
    let app_config_for_bg = app_config.clone();
    window_background.set_draw_func(move |_, cr, width, height| {
        use rg_sens::ui::background::render_background_with_theme;
        let cfg = app_config_for_bg.borrow();
        let _ = render_background_with_theme(cr, &cfg.window.background, width as f64, height as f64, Some(&cfg.global_theme));
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

    // Note: window.set_child() is called later, after determining if we need auto-hide mode

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
                config_helpers::find_monitor_by_connector(connector).map(|i| i as i32)
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
            config_helpers::find_monitor_by_connector(connector)
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

    // Setup auto-scroll using the auto_scroll module
    let start_auto_scroll = auto_scroll::create_auto_scroll_starter(
        &scrolled_window,
        &app_config,
        &grid_layout,
        &window_background,
    );

    // Start auto-scroll if enabled in config
    start_auto_scroll();

    // Store the start function for use in settings dialog
    let start_auto_scroll = Rc::new(start_auto_scroll);

    // Create header bar with menu button
    let header_bar = gtk4::HeaderBar::new();
    let menu_button = gtk4::MenuButton::new();
    menu_button.set_icon_name("open-menu-symbolic");
    menu_button.set_tooltip_text(Some("Application menu"));

    // Create the app menu popover
    let app_menu_popover = context_menu::create_app_menu_popover(
        &window,
        &app_config,
        &window_background,
        &grid_layout,
        &config_dirty,
        &start_auto_scroll,
    );
    menu_button.set_popover(Some(&app_menu_popover));
    header_bar.pack_start(&menu_button);

    // Auto-hide header: appears when mouse moves to top of window
    // Create revealer for smooth show/hide animation
    let revealer = gtk4::Revealer::new();
    revealer.set_transition_type(gtk4::RevealerTransitionType::SlideDown);
    revealer.set_transition_duration(200);
    revealer.set_reveal_child(false); // Start hidden
    revealer.set_valign(gtk4::Align::Start); // Position at top
    revealer.set_halign(gtk4::Align::Fill); // Span full width

    // Wrap headerbar in a box with styling for visibility against content
    let header_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    header_box.append(&header_bar);
    header_box.set_hexpand(true);
    header_box.add_css_class("auto-hide-header");
    revealer.set_child(Some(&header_box));

    // Add CSS for auto-hide header styling
    let css_provider = gtk4::CssProvider::new();
    css_provider.load_from_data(
        ".auto-hide-header {
            background-color: rgba(40, 40, 40, 0.95);
            border-bottom: 1px solid rgba(255, 255, 255, 0.1);
            padding: 4px 8px;
        }
        .auto-hide-header headerbar {
            background: transparent;
            border: none;
            box-shadow: none;
            min-height: 0;
        }"
    );
    gtk4::style_context_add_provider_for_display(
        &gtk4::prelude::WidgetExt::display(&window),
        &css_provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    // Create top-level overlay to hold both content and the auto-hide header
    let top_overlay = gtk4::Overlay::new();
    top_overlay.set_child(Some(&scrolled_window));
    top_overlay.add_overlay(&revealer);

    // Set up the window content and auto-hide behavior
    window.set_child(Some(&top_overlay));
    setup_auto_hide_header(&window, &revealer, &app_menu_popover);

    // Setup save-on-close confirmation
    let grid_layout_for_close = grid_layout.clone();
    let config_dirty_clone4 = config_dirty.clone();
    let app_config_for_close = app_config.clone();
    let update_manager_for_close = update_manager.clone();

    window.connect_close_request(move |window| {
        // Close all open dialogs first
        rg_sens::ui::close_all_dialogs();

        // Stop the update manager gracefully
        update_manager_for_close.stop();

        let is_dirty = config_dirty_clone4.load(Ordering::Relaxed);

        if is_dirty {
            // Show save confirmation dialog
            config_helpers::show_save_dialog(window, &grid_layout_for_close, &app_config_for_close);
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
            // 250ms check interval reduces CPU from 10 wakeups/sec to 4/sec while still being responsive
            info!("Starting update loop");
            update_manager_clone.run(Duration::from_millis(250)).await;
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
    let scrolled_window_for_menu = scrolled_window.clone();

    gesture_click.connect_pressed(move |gesture, _, x, y| {
        // Get scroll offsets to convert window coordinates to grid coordinates
        let scroll_x = scrolled_window_for_menu.hadjustment().value();
        let scroll_y = scrolled_window_for_menu.vadjustment().value();

        context_menu::show_context_menu(
            &window_for_menu,
            &app_config_for_menu,
            &window_bg_for_menu,
            &grid_layout_for_menu,
            &config_dirty_for_menu,
            &start_auto_scroll_for_menu,
            x,
            y,
            scroll_x,
            scroll_y,
        );
        gesture.set_state(gtk4::EventSequenceState::Claimed);
    });

    window.add_controller(gesture_click);

    // Clone for closure
    let grid_layout_for_key = grid_layout_for_settings.clone();
    let grid_layout_for_space = grid_layout_for_settings.clone();
    let grid_layout_for_space_release = grid_layout_for_settings.clone();
    let scrolled_window_for_keys = scrolled_window.clone();

    key_controller.connect_key_pressed(move |_, key, _code, modifiers| {
        // Ctrl+Comma opens settings
        if modifiers.contains(gtk4::gdk::ModifierType::CONTROL_MASK)
            && key == gtk4::gdk::Key::comma {
            window_settings_dialog::show_window_settings_dialog(
                &window_clone_for_settings,
                &app_config_for_settings,
                &window_bg_for_settings,
                &grid_layout_for_key,
                &config_dirty_for_settings,
                &start_auto_scroll_for_settings,
            );
            return glib::Propagation::Stop;
        }

        // Arrow keys scroll the window
        // Shift+Arrow scrolls by window width/height, plain arrow scrolls a small amount
        let is_shift = modifiers.contains(gtk4::gdk::ModifierType::SHIFT_MASK);
        let h_adj = scrolled_window_for_keys.hadjustment();
        let v_adj = scrolled_window_for_keys.vadjustment();

        // Use window dimensions for page scroll
        let page_width = window_clone_for_settings.width() as f64;
        let page_height = window_clone_for_settings.height() as f64;

        match key {
            gtk4::gdk::Key::Up => {
                let step = if is_shift { page_height } else { 50.0 };
                let new_val = (v_adj.value() - step).max(0.0);
                v_adj.set_value(new_val);
                return glib::Propagation::Stop;
            }
            gtk4::gdk::Key::Down => {
                let step = if is_shift { page_height } else { 50.0 };
                let max_val = v_adj.upper() - v_adj.page_size();
                let new_val = (v_adj.value() + step).min(max_val);
                v_adj.set_value(new_val);
                return glib::Propagation::Stop;
            }
            gtk4::gdk::Key::Left => {
                let step = if is_shift { page_width } else { 50.0 };
                let new_val = (h_adj.value() - step).max(0.0);
                h_adj.set_value(new_val);
                return glib::Propagation::Stop;
            }
            gtk4::gdk::Key::Right => {
                let step = if is_shift { page_width } else { 50.0 };
                let max_val = h_adj.upper() - h_adj.page_size();
                let new_val = (h_adj.value() + step).min(max_val);
                h_adj.set_value(new_val);
                return glib::Propagation::Stop;
            }
            _ => {}
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

/// Setup auto-hide behavior for the headerbar in fullscreen/borderless mode.
/// The headerbar slides down when the mouse moves to the top of the window,
/// and hides after a delay when the mouse moves away.
fn setup_auto_hide_header(
    window: &gtk4::ApplicationWindow,
    revealer: &gtk4::Revealer,
    popover: &gtk4::Popover,
) {
    let motion_controller = gtk4::EventControllerMotion::new();
    let revealer_motion = revealer.clone();
    let revealer_leave = revealer.clone();
    let popover_motion = popover.clone();
    let popover_leave = popover.clone();

    // Timer for delayed hide
    let hide_timer: Rc<RefCell<Option<glib::SourceId>>> = Rc::new(RefCell::new(None));
    let hide_timer_motion = hide_timer.clone();
    let hide_timer_leave = hide_timer.clone();

    // Helper to cancel any pending hide timer
    fn cancel_hide_timer(timer: &Rc<RefCell<Option<glib::SourceId>>>) {
        if let Some(id) = timer.borrow_mut().take() {
            id.remove();
        }
    }

    // Helper to schedule hide after delay
    fn schedule_hide(
        timer: &Rc<RefCell<Option<glib::SourceId>>>,
        revealer: &gtk4::Revealer,
        delay_ms: u64,
    ) {
        if timer.borrow().is_some() {
            return; // Timer already scheduled
        }
        let revealer_hide = revealer.clone();
        let timer_ref = timer.clone();
        let source_id = glib::timeout_add_local_once(
            std::time::Duration::from_millis(delay_ms),
            move || {
                revealer_hide.set_reveal_child(false);
                *timer_ref.borrow_mut() = None;
            },
        );
        *timer.borrow_mut() = Some(source_id);
    }

    motion_controller.connect_motion(move |_, _x, y| {
        let trigger_zone = 50.0; // pixels from top to trigger reveal

        if y < trigger_zone {
            // Mouse at top - show menu immediately
            cancel_hide_timer(&hide_timer_motion);
            revealer_motion.set_reveal_child(true);
        } else if revealer_motion.reveals_child() && !popover_motion.is_visible() {
            // Mouse moved away and popover is not open - schedule hide
            schedule_hide(&hide_timer_motion, &revealer_motion, 1500);
        }
    });

    motion_controller.connect_leave(move |_| {
        // Mouse left window - hide menu after delay (unless popover is open)
        if revealer_leave.reveals_child() && !popover_leave.is_visible() {
            schedule_hide(&hide_timer_leave, &revealer_leave, 1500);
        }
    });

    window.add_controller(motion_controller);

    // Also track when the popover closes to schedule hide
    let revealer_popover = revealer.clone();
    let hide_timer_popover = hide_timer.clone();
    popover.connect_closed(move |_| {
        // When popover closes, schedule hide after delay
        if revealer_popover.reveals_child() {
            let revealer_hide = revealer_popover.clone();
            let timer_ref = hide_timer_popover.clone();
            // Cancel any existing timer first
            if let Some(id) = timer_ref.borrow_mut().take() {
                id.remove();
            }
            let source_id = glib::timeout_add_local_once(
                std::time::Duration::from_millis(1500),
                move || {
                    revealer_hide.set_reveal_child(false);
                    *timer_ref.borrow_mut() = None;
                },
            );
            *hide_timer_popover.borrow_mut() = Some(source_id);
        }
    });
}

