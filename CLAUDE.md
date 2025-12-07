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
```

## Code Architecture

### Core Trait System

The architecture separates **data collection** from **visualization** through two main traits:

**DataSource trait** (`src/core/data_source.rs`):
- Collects system metrics (CPU, GPU, memory, etc.)
- Must be `Send + Sync` for multi-threaded updates
- Implements `update()` to refresh data and `get_values()` to expose data as JSON
- Examples: `CpuSource`, `NvidiaGpuSource` in `src/sources/`

**Displayer trait** (`src/core/displayer.rs`):
- Visualizes data from any source
- Must be `Send + Sync` despite GTK widget usage
- Creates GTK widgets via `create_widget()` and renders via Cairo in `draw()`
- Examples: `TextDisplayer`, `BarDisplayer` in `src/displayers/`

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

**Why:** Storing GTK widgets directly breaks `Send + Sync`. Instead, store data in `Arc<Mutex<T>>` and create widgets on-demand. See `src/displayers/text.rs` or `src/displayers/bar.rs` for reference implementations.

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
- `src/ui/background.rs` (rendering) + `src/ui/background_config_widget.rs` (UI)

**Pattern:** Rendering code is separate from GTK configuration UI. Rendering modules export:
- Data structures (e.g., `BarDisplayConfig`)
- Render function (e.g., `pub fn render_bar(cr: &Context, config: &BarDisplayConfig, ...)`)

Config widgets create UI controls and call the render function in a preview `DrawingArea`.

### Cairo Rendering

All custom visualizations use Cairo (`cairo-rs`):
- Bar displays: `src/ui/bar_display.rs`
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
│   └── gpu.rs           # NVIDIA GPU via nvml-wrapper
├── displayers/     # Visualization implementations
│   ├── text.rs          # Text display with Pango
│   ├── bar.rs           # Bar/gauge displays
│   └── text_config.rs   # Text display configuration types
├── ui/             # GTK UI components
│   ├── main_window.rs   # Application window
│   ├── grid_layout.rs   # Drag-drop panel grid
│   ├── config_dialog.rs # Settings dialog
│   ├── bar_display.rs   # Bar rendering logic
│   ├── bar_config_widget.rs  # Bar configuration UI
│   ├── background.rs    # Background rendering (gradients/images)
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
use crate::core::{DataSource, SourceMetadata};

pub struct MySource {
    metadata: SourceMetadata,
    values: HashMap<String, Value>,
    system: System,  // Or other system API
}

impl DataSource for MySource {
    fn update(&mut self) -> Result<()> {
        self.system.refresh_all();
        // Update self.values...
        Ok(())
    }
    // ... other trait methods
}
```

2. Register in `src/sources/mod.rs`
3. Add config widget in `src/ui/` if needed

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

## Performance Considerations

- **Update frequency:** Default 1 second, configurable per panel
- **Parallel updates:** Sources update concurrently via tokio
- **Smart redraws:** Only redraw when data changes (check `needs_redraw()`)
- **Profile compilation:** Dev builds use `opt-level = 1` for faster debug iterations

## Common Gotchas

1. **Importing from core:** Use `crate::core::{Displayer, ...}`, not `crate::Displayer`
2. **GTK thread safety:** Never store GTK widgets in `Send + Sync` structs
3. **Deprecated widgets:** Always check GTK4 docs before using UI components
4. **Gradient/color stops:** Must be sorted by position (0.0 to 1.0)
5. **Panel registration:** Forgetting to register in `mod.rs::register_all()` will make it invisible in UI
6. **Cairo state:** Always `save()`/`restore()` to avoid polluting other draws
7. **Temperature sensors:** AMD Ryzen uses `Tctl`/`Tccd` labels, Intel uses `Package`/`Core`

## Dependencies

**Core:**
- `gtk4`: UI framework (v4.10+ features)
- `cairo-rs`: 2D graphics rendering
- `tokio`: Async runtime for updates
- `sysinfo`: Cross-platform system info

**Optional:**
- `nvml-wrapper`: NVIDIA GPU support (feature: `nvidia`, enabled by default)

**Build time:** Requires GTK4 dev libraries (`libgtk-4-dev` on Debian/Ubuntu)
