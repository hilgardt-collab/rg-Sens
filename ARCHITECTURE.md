# rg-Sens Architecture

## Overview

rg-Sens follows a modular, trait-based architecture that separates concerns and allows for extensibility through both compile-time and runtime plugin systems.

## Core Concepts

### Data Sources

Data sources are responsible for collecting system metrics. They implement the `DataSource` trait:

```rust
pub trait DataSource: Send + Sync {
    fn metadata(&self) -> &SourceMetadata;
    fn update(&mut self) -> Result<()>;
    fn get_values(&self) -> HashMap<String, Value>;
    fn get_value(&self, key: &str) -> Option<Value>;
    fn is_available(&self) -> bool;
}
```

**Key Properties:**
- **Stateful**: Sources maintain internal state between updates
- **Async-safe**: `Send + Sync` allows use in async contexts
- **Self-describing**: Metadata describes available data keys
- **Availability checking**: Can determine if hardware/features are present

**Example Sources:**
- `CpuSource`: CPU usage, frequency, per-core stats
- `MemorySource`: RAM and swap usage
- `NvidiaGpuSource`: GPU utilization, temperature, memory
- `TemperatureSource`: System temperatures via lm-sensors

### Displayers

Displayers are responsible for visualizing data. They implement the `Displayer` trait:

```rust
pub trait Displayer: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn create_widget(&self) -> Widget;
    fn update_data(&mut self, data: &HashMap<String, Value>);
    fn draw(&self, cr: &Context, width: f64, height: f64) -> Result<()>;
    fn config_schema(&self) -> ConfigSchema;
    fn apply_config(&mut self, config: &HashMap<String, Value>) -> Result<()>;
    fn needs_redraw(&self) -> bool;
}
```

**Key Properties:**
- **GTK Integration**: Creates native GTK widgets
- **Cairo Rendering**: Custom drawing using Cairo
- **Configurable**: Schema-driven configuration system
- **Optimizable**: Can skip redraws when data hasn't changed

**Example Displayers:**
- `TextDisplayer`: Simple text output
- `LevelBarDisplayer`: Horizontal/vertical progress bar
- `ArcGaugeDisplayer`: Circular gauge (like speedometer)
- `LineGraphDisplayer`: Time-series line graph

### Panels

A `Panel` combines a data source with a displayer:

```rust
pub struct Panel {
    pub id: String,
    pub geometry: PanelGeometry,
    pub source: BoxedDataSource,
    pub displayer: BoxedDisplayer,
    pub config: HashMap<String, Value>,
}
```

Panels are the fundamental building blocks displayed in the grid.

### Registry

The `Registry` manages available sources and displayers:

```rust
pub struct Registry {
    sources: HashMap<String, SourceFactory>,
    displayers: HashMap<String, DisplayerFactory>,
}
```

**Registration Methods:**
- **Compile-time**: Built-in sources/displayers use macros
- **Runtime**: Future plugin system will register dynamically

**Usage:**
```rust
// Register a source
register_source!("cpu", CpuSource);

// Create an instance
let source = registry.create_source("cpu")?;
```

### Update Manager

The `UpdateManager` coordinates periodic updates:

```rust
pub struct UpdateManager {
    panels: Arc<RwLock<Vec<Arc<RwLock<Panel>>>>>,
}
```

**Responsibilities:**
- Schedule periodic updates for all panels
- Update sources in parallel using tokio tasks
- Measure and log update performance
- Handle errors gracefully

**Performance Characteristics:**
- Parallel updates using tokio
- Lock-free reading where possible
- Minimal overhead (<1ms per cycle)

## Data Flow

```
┌─────────────────┐
│  UpdateManager  │ (tokio runtime)
└────────┬────────┘
         │ periodic tick
         ▼
  ┌──────────────┐
  │   Panel(s)   │
  └──┬───────┬───┘
     │       │
     ▼       ▼
┌─────────┐ ┌───────────┐
│ Source  │ │ Displayer │
│.update()│ │.update_   │
│         │ │  data()   │
└─────────┘ └─────┬─────┘
                  │
                  ▼
            ┌──────────┐
            │   GTK    │
            │ DrawingA │
            │   rea    │
            └──────────┘
```

1. **Update Manager** ticks at regular intervals (e.g., 1 second)
2. For each **Panel**:
   - Call `source.update()` to refresh data
   - Get updated values from source
   - Call `displayer.update_data()` with new values
3. **GTK** triggers draw events
4. **Displayer** renders using Cairo

## Threading Model

### GTK Main Thread
- UI creation and event handling
- Drawing operations (Cairo)
- User interaction

### Tokio Runtime
- Asynchronous update scheduling
- Parallel source updates
- Background tasks

### Synchronization
- `Arc<RwLock<T>>` for shared panel state
- `arc_swap::ArcSwap` for lock-free reads (future optimization)
- Channels for cross-thread communication

## Configuration System

### File Format

Configuration is stored as JSON:

```json
{
  "version": 1,
  "window": {
    "width": 800,
    "height": 600
  },
  "grid": {
    "columns": 4,
    "rows": 3,
    "spacing": 4
  },
  "panels": [
    {
      "id": "panel-1",
      "x": 0,
      "y": 0,
      "width": 2,
      "height": 1,
      "source": "cpu",
      "displayer": "arc_gauge",
      "settings": {
        "color": "#00ff00",
        "show_percentage": true
      }
    }
  ]
}
```

### Location

- Linux: `~/.config/rg-sens/config.json`
- Respects `$XDG_CONFIG_HOME`

### Migration

Python gSens configs can be migrated:
- Read from `~/.config/gtk-system-monitor/`
- Map source/displayer IDs
- Convert to Rust format

## Plugin System (Future)

### Architecture

Two-tier system:
1. **Built-in**: Compiled into binary (fast, zero overhead)
2. **Dynamic**: Loaded at runtime (flexible, user-extensible)

### Dynamic Loading

Uses `libloading` with C ABI:

```rust
#[repr(C)]
pub struct PluginVTable {
    create: extern "C" fn() -> *mut c_void,
    update: extern "C" fn(*mut c_void) -> i32,
    destroy: extern "C" fn(*mut c_void),
}
```

**Plugin Discovery:**
- Scan `~/.local/share/rg-sens/plugins/`
- Load `.so` files with plugin manifest
- Validate ABI version compatibility

**Safety:**
- Plugins run in separate process (isolation)
- IPC via JSON-RPC (structured communication)
- Timeout and crash recovery

## Performance Optimizations

### 1. Smart Redraws
Only redraw widgets when data changes:
```rust
fn needs_redraw(&self) -> bool {
    self.data_changed || self.resize_pending
}
```

### 2. Batched System Calls
Read multiple metrics in one pass:
```rust
// Single read of /proc/stat, /proc/meminfo
struct SystemSnapshot {
    cpu: CpuInfo,
    memory: MemInfo,
}
```

### 3. Lock-Free Reads
Use `arc_swap::ArcSwap` for hot paths:
```rust
let data = self.cpu_data.load();  // No mutex!
```

### 4. Parallel Updates
Update sources concurrently:
```rust
for panel in panels {
    tokio::spawn(async move {
        panel.update().await;
    });
}
```

### 5. Optimized Rendering
- Pre-compute static elements
- Use Cairo surface caching
- Minimize state changes

## Error Handling

### Strategy
- `Result<T, E>` for fallible operations
- `anyhow::Error` for application errors
- `thiserror` for custom error types
- Log errors, don't panic

### Graceful Degradation
- Missing GPU? Skip GPU panels
- Sensor unavailable? Show "N/A"
- Plugin crash? Unload and continue

## Testing Strategy

### Unit Tests
- Test traits with mock implementations
- Test configuration serialization
- Test registry operations

### Integration Tests
- Test full update cycle
- Test panel creation and updates
- Test configuration loading/saving

### Performance Tests
- Benchmark update cycle time
- Benchmark rendering performance
- Profile memory usage

## Future Enhancements

1. **Multi-window support**: Multiple monitor layouts
2. **Themes**: Pre-configured layouts and colors
3. **Remote monitoring**: Display metrics from other machines
4. **Data export**: Save metrics to file/database
5. **Alerts**: Notify on threshold breach
6. **Web interface**: Browser-based remote view
