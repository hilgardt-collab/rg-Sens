# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

rg-Sens is a Rust port of the Python gSens system monitoring dashboard. It provides a customizable grid-based interface for visualizing system metrics (CPU, GPU, memory, temperatures, etc.) using GTK4 and Cairo rendering.

**Key characteristics:**
- Performance-critical: Target <5% CPU idle, <50MB memory
- GTK4 UI with Cairo rendering for custom visualizations
- Trait-based plugin architecture separating data collection from visualization
- Thread-safe design using `Arc<Mutex<T>>` for GTK compatibility

## Build Commands

```bash
# Development build (slightly optimized for faster iteration)
cargo build

# Release build (full optimizations)
cargo build --release

# Run the application
cargo run

# Run with NVIDIA GPU support disabled
cargo run --no-default-features

# Check compilation without building
cargo check

# Run tests
cargo test

# Format code
cargo fmt

# Run linter
cargo clippy
```

## Code Architecture

### Core Trait System

The architecture separates **data collection** from **visualization** through two main traits:

**DataSource trait** (`src/core/data_source.rs`):
- Collects system metrics (CPU, GPU, memory, etc.)
- Must be `Send + Sync` for multi-threaded updates
- Implements `update()` to refresh data and `get_values()` to expose data as JSON
- Implements `fields()` to describe available data fields with metadata
- Implements `configure()` to accept source-specific configuration
- Examples: `CpuSource`, `GpuSource`, `MemorySource` in `src/sources/`

**Displayer trait** (`src/core/displayer.rs`):
- Visualizes data from any source
- Must be `Send + Sync` despite GTK widget usage
- Creates GTK widgets via `create_widget()` and renders via Cairo in `draw()`
- Examples: `TextDisplayer`, `BarDisplayer`, `ArcDisplayer` in `src/displayers/`

### Critical Threading Pattern

**IMPORTANT:** GTK widgets are NOT thread-safe, but the `Displayer` trait requires `Send + Sync`. The solution:

```rust
pub struct MyDisplayer {
    id: String,
    name: String,
    data: Arc<Mutex<DisplayData>>,  // Use Arc<Mutex>, NOT RefCell
}

// create_widget() should NOT store the DrawingArea widget
fn create_widget(&self) -> Widget {
    let drawing_area = DrawingArea::new();
    let data_clone = self.data.clone();

    drawing_area.set_draw_func(move |_, cr, width, height| {
        if let Ok(data) = data_clone.lock() {
            // Render using data...
        }
    });

    drawing_area.upcast()
}
```

**Why:** Storing GTK widgets directly breaks `Send + Sync`. Instead, store data in `Arc<Mutex<T>>` and create widgets on-demand. See `src/displayers/text.rs`, `src/displayers/bar.rs`, or `src/displayers/arc.rs` for reference implementations.

### Registration Pattern

All sources and displayers must be registered in their respective `mod.rs` files:

```rust
// In src/displayers/mod.rs
mod my_displayer;
pub use my_displayer::MyDisplayer;

pub fn register_all() {
    global_registry().register_displayer("my_id", || Box::new(MyDisplayer::new()));
}

// In src/sources/mod.rs
mod my_source;
pub use my_source::MySource;

pub fn register_all() {
    global_registry().register_source("my_id", || Box::new(MySource::new()));
}
```

Registration happens in `src/main.rs` at startup. If a displayer/source doesn't appear in the UI, check that it's registered.

### Panel System

**Panel** (`src/core/panel.rs`): Combines one source + one displayer
- Has geometry (x, y, width, height in grid cells)
- Has background configuration (solid color, gradient, image, or polygon)
- Has corner radius and border configuration
- Managed by `GridLayout` (`src/ui/grid_layout.rs`)
- Updated by `UpdateManager` (`src/core/update_manager.rs`)

**Update flow:**
1. `UpdateManager` calls `source.update()` periodically
2. Source fetches fresh system data
3. Panel calls `displayer.update_data(source.get_values())`
4. GTK triggers redraw, calling `displayer.draw()`

### UI Widget Architecture

**Configuration Widgets:** Each complex UI component has a paired config widget:
- `src/ui/bar_display.rs` (rendering) + `src/ui/bar_config_widget.rs` (UI)
- `src/ui/arc_display.rs` (rendering) + `src/ui/arc_config_widget.rs` (UI)
- `src/ui/background.rs` (rendering) + `src/ui/background_config_widget.rs` (UI)

**Pattern:** Rendering code is separate from GTK configuration UI. Rendering modules export:
- Data structures (e.g., `BarDisplayConfig`, `ArcDisplayConfig`)
- Render function (e.g., `pub fn render_bar(cr: &Context, config: &BarDisplayConfig, ...)`)

Config widgets create UI controls and call the render function in a preview `DrawingArea`.

### Cairo Rendering

All custom visualizations use Cairo (`cairo-rs`):
- Bar displays: `src/ui/bar_display.rs`
- Arc gauges: `src/ui/arc_display.rs`
- Backgrounds: `src/ui/background.rs` (gradients, images, polygons)
- Custom displayers: `src/displayers/*/draw()`

**Key Cairo patterns:**
```rust
// Save/restore state for isolated drawing
cr.save().ok();
cr.set_source_rgba(r, g, b, a);
cr.rectangle(x, y, width, height);
cr.fill().ok();
cr.restore().ok();

// Gradients
let gradient = cairo::LinearGradient::new(x1, y1, x2, y2);
gradient.add_color_stop_rgba(0.0, r, g, b, a);
gradient.add_color_stop_rgba(1.0, r, g, b, a);
cr.set_source(&gradient).ok();

// Arcs (for circular gauges)
cr.arc(center_x, center_y, radius, start_angle, end_angle);
cr.stroke().ok();
```

### GTK4 Modernization

**CRITICAL:** This project avoids deprecated GTK widgets:
- ❌ `Dialog` → ✅ `Window` + `HeaderBar`
- ❌ `FileChooserDialog` → ✅ `FileDialog`
- ❌ `ComboBoxText` → ✅ `DropDown` + `StringList`

When creating new UI components, use modern GTK4 APIs. Check `src/ui/image_picker.rs` for a reference implementation replacing deprecated `FileChooserDialog`.

### Configuration System

**Config location:** `~/.config/rg-sens/config.json` (respects `$XDG_CONFIG_HOME`)

**Structure:**
```rust
// src/config/settings.rs
pub struct AppConfig {
    pub window: WindowConfig,
    pub grid: GridConfig,
    pub panels: Vec<PanelConfig>,
}
```

**Serialization:** Uses `serde` + `serde_json`. All configs implement `Serialize + Deserialize`.

**Migration:** `src/config/migration.rs` handles importing from Python gSens configs.

## Module Organization

```
src/
├── core/           # Core traits and fundamental types
│   ├── data_source.rs    # DataSource trait
│   ├── displayer.rs      # Displayer trait
│   ├── panel.rs          # Panel combining source + displayer
│   ├── registry.rs       # Global source/displayer registry
│   └── update_manager.rs # Periodic update coordination
├── sources/        # Data source implementations
│   ├── cpu.rs           # CPU metrics via sysinfo
│   ├── memory.rs        # Memory metrics via sysinfo
│   └── gpu/
│       ├── mod.rs       # GPU source (multi-vendor)
│       ├── backend.rs   # GPU backend trait
│       ├── nvidia.rs    # NVIDIA GPU via nvml-wrapper
│       ├── amd.rs       # AMD GPU via sysfs
│       └── detector.rs  # GPU detection logic
├── displayers/     # Visualization implementations
│   ├── text.rs          # Text display with Pango
│   ├── bar.rs           # Bar/gauge displays
│   ├── arc.rs           # Arc gauge displays
│   └── text_config.rs   # Text display configuration types
├── ui/             # GTK UI components
│   ├── main_window.rs   # Application window
│   ├── grid_layout.rs   # Drag-drop panel grid
│   ├── config_dialog.rs # Settings dialog
│   ├── bar_display.rs   # Bar rendering logic
│   ├── bar_config_widget.rs  # Bar configuration UI
│   ├── arc_display.rs   # Arc rendering logic
│   ├── arc_config_widget.rs  # Arc configuration UI
│   ├── background.rs    # Background rendering (gradients/images)
│   ├── background_config_widget.rs  # Background configuration UI
│   └── [component]_config_widget.rs  # Config UIs for various components
├── config/         # Configuration management
├── plugin/         # Future: dynamic plugin loading
├── lib.rs          # Library exports
└── main.rs         # Application entry point
```

## Important Patterns

### Adding a New Displayer

1. Create `src/displayers/my_displayer.rs`:
```rust
use crate::core::{ConfigOption, ConfigSchema, Displayer};
use std::sync::{Arc, Mutex};

pub struct MyDisplayer {
    id: String,
    name: String,
    data: Arc<Mutex<DisplayData>>,
}

#[derive(Clone)]
struct DisplayData {
    config: MyConfig,
    value: f64,
}

impl Displayer for MyDisplayer {
    fn create_widget(&self) -> Widget {
        let drawing_area = DrawingArea::new();
        let data = self.data.clone();
        drawing_area.set_draw_func(move |_, cr, w, h| {
            if let Ok(d) = data.lock() {
                // Render...
            }
        });
        drawing_area.upcast()
    }
    // ... other trait methods
}
```

2. Register in `src/displayers/mod.rs`
3. Add config widget in `src/ui/` if needed

### Adding a New Data Source

1. Create `src/sources/my_source.rs`:
```rust
use crate::core::{DataSource, FieldMetadata, SourceMetadata};

pub struct MySource {
    metadata: SourceMetadata,
    values: HashMap<String, Value>,
    system: System,  // Or other system API
}

impl DataSource for MySource {
    fn metadata(&self) -> &SourceMetadata {
        &self.metadata
    }

    fn fields(&self) -> Vec<FieldMetadata> {
        vec![
            FieldMetadata::new("value", "Value", "Description", FieldType::Numerical, FieldPurpose::Value),
            // ... more fields
        ]
    }

    fn update(&mut self) -> Result<()> {
        self.system.refresh_all();
        // Update self.values...
        Ok(())
    }

    fn get_values(&self) -> HashMap<String, Value> {
        self.values.clone()
    }

    fn configure(&mut self, config: &HashMap<String, Value>) -> Result<()> {
        // Handle source-specific configuration
        Ok(())
    }
}
```

2. Register in `src/sources/mod.rs`
3. Add config widget in `src/ui/` if needed (e.g., `src/ui/my_source_config_widget.rs`)

### Widget Callbacks with State

GTK callbacks need `'static` lifetime. Use `Rc<RefCell<T>>` for mutable state:

```rust
let state = Rc::new(RefCell::new(MyState::default()));
let state_clone = state.clone();

button.connect_clicked(move |_| {
    let mut s = state_clone.borrow_mut();
    s.value += 1;
});
```

For async operations (like color pickers), spawn on the main context:

```rust
gtk4::glib::MainContext::default().spawn_local(async move {
    if let Some(color) = ColorPickerDialog::pick_color(window, current).await {
        // Update state...
    }
});
```

### Animation in Displayers

All animations go through the global `AnimationManager` (`src/core/animation_manager.rs`). Register via `register_animation()`:

```rust
use crate::core::register_animation;

register_animation(drawing_area.downgrade(), move || {
    if let Ok(mut data) = data_for_animation.try_lock() {
        // Animation logic (lerp, etc.)
        needs_redraw  // return true if widget needs redraw
    } else {
        false  // Lock contended, skip this frame
    }
});
```

**Key design:**
- Uses GTK frame clock (`add_tick_callback`) for VSync-synchronized animation
- Falls back to `timeout_add_local_full` with `Priority::DEFAULT_IDLE` (user input takes priority)
- Generation counter prevents callback accumulation when widgets are destroyed/recreated
- Adaptive frame rate: 60fps when animating, ~4fps when idle

## Performance Considerations

- **Update frequency:** Default 1 second, configurable per panel
- **Parallel updates:** Sources update concurrently via tokio
- **Smart redraws:** Only redraw when data changes (check `needs_redraw()`)
- **Profile compilation:** Dev builds use `opt-level = 1` for faster debug iterations
- **Animation:** Use 60fps cap (16ms) for smooth animations without excessive CPU usage

## Common Gotchas

1. **Importing from core:** Use `crate::core::{Displayer, ...}`, not `crate::Displayer`
2. **GTK thread safety:** Never store GTK widgets in `Send + Sync` structs
3. **Deprecated widgets:** Always check GTK4 docs before using UI components
4. **Gradient/color stops:** Must be sorted by position (0.0 to 1.0)
5. **Panel registration:** Forgetting to register in `mod.rs::register_all()` will make it invisible in UI
6. **Cairo state:** Always `save()`/`restore()` to avoid polluting other draws
7. **Temperature sensors:** AMD Ryzen uses `Tctl`/`Tccd` labels, Intel uses `Package`/`Core`
8. **GPU detection:** GPU backends are initialized once at startup via `once_cell::Lazy` - changes to GPU hardware require app restart
9. **Multi-GPU systems:** GPU sources use an index to select which GPU to monitor (defaults to 0)
10. **AMD GPU support:** Uses sysfs files (`/sys/class/drm/card*/device/*`) - may require permissions on some systems
11. **Signal handler memory leaks:** GTK signal handlers (like `connect_map`) that capture container clones create reference cycles. Store the `SignalHandlerId` and call `container.disconnect(handler_id)` during cleanup to break the cycle
12. **RefCell + GTK signals:** Never call `dialog.close()` or similar GTK methods while holding a `borrow_mut()` on a RefCell - the resulting signal handlers may try to borrow the same RefCell, causing a panic. Extract the value first, release the borrow, then call the GTK method
13. **NEVER use `std::thread::sleep` on the GTK main thread** — freezes all event processing (context menus, auto-hide, drawing). Use `blocking_read()` instead of `try_read()` + `sleep()` loops; it returns as soon as the lock is released (~1ms vs 10ms fixed intervals)
14. **Lock strategy for `tokio::sync::RwLock`:** Use `try_read()`/`try_lock()` only for hot paths (60fps animation ticks, draw functions). Use `blocking_read()` for one-time user actions (dialog open, save, copy, context menu). Never silently skip user-initiated operations via `try_read()` + `continue`
15. **Combo displayer registration:** When adding a new combo displayer, add it to the `on_fields_updated` match in `grid_properties_dialog.rs` — otherwise content properties won't receive field metadata
16. **Popover cleanup:** Defer `popover.unparent()` in `connect_destroy` handlers via `gtk4::glib::idle_add_local_once` to avoid re-entrancy during widget teardown
17. **Animation priority:** Use `glib::Priority::DEFAULT_IDLE` for animation timers and incremental widget building (`idle_add_local_full`) so user input events are always processed first

## GPU Support

### Multi-Vendor Architecture

The GPU source (`src/sources/gpu/`) supports multiple GPU vendors through a backend trait system:

- **Backend trait** (`src/sources/gpu/backend.rs`): Defines common GPU operations
- **NVIDIA backend** (`src/sources/gpu/nvidia.rs`): Uses `nvml-wrapper` (optional feature)
- **AMD backend** (`src/sources/gpu/amd.rs`): Uses sysfs files
- **Detection** (`src/sources/gpu/detector.rs`): Auto-detects available GPUs at startup

**Key features:**
- Automatic GPU detection at startup (cached for lifetime of app)
- Multi-GPU support (select GPU by index)
- Field selection (temperature, utilization, memory, power, fan speed)
- Unit conversion (Celsius/Fahrenheit/Kelvin for temps, MB/GB for memory)
- Auto-detect limits or manual configuration

**Disabling NVIDIA support:**
```bash
cargo build --no-default-features
```

## Dependencies

**Core:**
- `gtk4`: UI framework (v4.10+ features)
- `cairo-rs`: 2D graphics rendering
- `tokio`: Async runtime for updates
- `sysinfo`: Cross-platform system info

**Optional:**
- `nvml-wrapper`: NVIDIA GPU support (feature: `nvidia`, enabled by default)

**Build time:** Requires GTK4 dev libraries (`libgtk-4-dev` on Debian/Ubuntu)

## Testing

Run tests with:
```bash
cargo test
```

For verbose output:
```bash
cargo test -- --nocapture
```

Run with logging:
```bash
RUST_LOG=debug cargo test
```
