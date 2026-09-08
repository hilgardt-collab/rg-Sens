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

**DataSource trait** (`crates/core/src/data_source.rs`, re-exported via `crate::core`):
- Collects system metrics (CPU, GPU, memory, etc.)
- Must be `Send + Sync` for multi-threaded updates
- Implements `update()` to refresh data and `get_values()` to expose data as JSON
- Implements `fields()` to describe available data fields with metadata
- Implements `configure()` to accept source-specific configuration
- Examples: `CpuSource`, `GpuSource`, `MemorySource` in `crates/sources/src/`

**Displayer trait** (`crates/core/src/displayer.rs`, re-exported via `crate::core`):
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
- `crates/render/src/bar_display.rs` (rendering) + `src/ui/bar_config_widget.rs` (UI)
- `crates/render/src/arc_display.rs` (rendering) + `src/ui/arc_config_widget.rs` (UI)
- `crates/render/src/background.rs` (rendering) + `src/ui/background_config_widget.rs` (UI)

**Pattern:** Rendering code is separate from GTK configuration UI. Rendering modules export:
- Data structures (e.g., `BarDisplayConfig`, `ArcDisplayConfig`)
- Render function (e.g., `pub fn render_bar(cr: &Context, config: &BarDisplayConfig, ...)`)

Config widgets create UI controls and call the render function in a preview `DrawingArea`.

### Cairo Rendering

All custom visualizations use Cairo (`cairo-rs`):
- Bar displays: `crates/render/src/bar_display.rs`
- Arc gauges: `crates/render/src/arc_display.rs`
- Backgrounds: `crates/render/src/background.rs` (gradients, images, polygons)
- Themed combo rendering: `crates/render/src/*_display.rs`
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

When creating new UI components, use modern GTK4 APIs. See the `FileDialog` usage in `src/ui/background_config_widget.rs` or `src/ui/css_template_config_widget.rs` for reference implementations replacing deprecated `FileChooserDialog`.

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

**Migration:** v1→v2 config migration is handled inline in `src/config/settings.rs` (`AppConfigV1::migrate_to_v2`).

**Saving:** All config writes go through `write_atomic()` in `src/config/mod.rs` (temp file + fsync + rename) — never `std::fs::write` a config file directly; a crash mid-write must not be able to truncate user data.

## Module Organization

This is a Cargo workspace. The GTK-free logic lives in `crates/`; the main crate holds GTK UI and orchestration.

```
crates/
├── core/           # DataSource + Displayer traits, registry, constants (no GTK)
├── types/          # Serde config types shared across crates
├── render/         # Pure Cairo rendering (bar/arc/background/themed *_display.rs, render_cache)
├── sources/        # Data sources: cpu, memory, disk, network, gpu/, fan_speed,
│                   #   system_temp, claude.rs + claude_auth.rs, combo, static_text
└── audio/          # Alarm/timer sound playback

src/
├── core/           # Panel, UpdateManager, SharedSourceManager, AnimationManager,
│                   #   TimerAlarmManager (+ re-exports of crates/core traits)
├── sources/        # GTK-adjacent sources (clock.rs drives the timer manager)
├── displayers/     # Displayer impls wiring sources to crates/render rendering
├── ui/             # GTK components: grid_layout.rs (drag-drop grid),
│                   #   grid_properties_dialog.rs, window_settings_dialog.rs,
│                   #   [component]_config_widget.rs config UIs, custom_color_picker.rs
├── config/         # AppConfig load/save (atomic writes), defaults, v1→v2 migration
├── plugin/         # Future: dynamic plugin loading
├── lib.rs          # Library exports
└── main.rs         # Application entry point + main window setup
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
- Single repeating `timeout_add_local_full` timer at `Priority::DEFAULT_IDLE` (user input takes priority); a `timer_active` flag prevents duplicates and the timer stops (`ControlFlow::Break`) when no entries remain
- `tick()` retains entries, dropping dead (weak `upgrade()` fails) or orphaned (`root().is_none()`) widgets
- Adaptive frame rate: 60fps when animating; idle mode skip-counts frames (~4fps) once nothing has animated recently
- Animation ticks must refresh their elapsed-time timestamp on EVERY tick (not only while animating) and cap the step factor at 1.0 — see `src/displayers/bar.rs` — otherwise the first frame after idle overshoots the target

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
14. **Lock strategy:** For `std::sync::Mutex` in draw functions, use `lock()` (not `try_lock()`) — `update_data()` holds the lock < 1ms, and painting transparent on contention causes visible flicker. Use `try_lock()` only in animation tick callbacks (skipping a frame is invisible). For `tokio::sync::RwLock`, use `blocking_read()` for one-time user actions (dialog open, save, copy, context menu). Never silently skip user-initiated operations via `try_read()` + `continue`
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
