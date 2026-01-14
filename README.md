# rg-Sens

A fast, customizable system monitoring dashboard for Linux, written in Rust.

![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)
![Rust](https://img.shields.io/badge/rust-1.70%2B-orange)
![GTK4](https://img.shields.io/badge/GTK-4.10%2B-green)

## Overview

rg-Sens is a Rust port of [gSens](https://github.com/hilgardt-collab/gSens), providing a highly customizable system monitoring dashboard with:

- **High Performance**: Low CPU and memory usage with native Rust implementation
- **Customizable Grid Layout**: Drag-and-drop panels with flexible sizing
- **Rich Visualizations**: Multiple displayer types including gauges, graphs, and themed combo panels
- **Multi-Monitor Support**: Fullscreen on specific monitors, borderless mode
- **Auto-Scroll**: Automatic scrolling through large dashboards
- **Native**: Built with GTK4 for seamless Linux desktop integration
- **Theme Aware**: Automatically detects system light/dark mode

## Screenshots

*Coming soon*

## Features

### Data Sources
- **CPU**: Usage, frequency, per-core stats, temperature sensors
- **Memory**: RAM and swap usage
- **GPU**: NVIDIA (via NVML) and AMD (via sysfs) support
- **System Temperature**: All system temperature sensors
- **Fan Speed**: System fan RPM monitoring
- **Disk**: Disk usage and capacity monitoring
- **Clock**: Current time with timezone support, alarms and timers
- **Static Text**: Display custom static text
- **Test Source**: Configurable test data for development and demos
- **Combo**: Combine multiple sources into themed combo panels

### Displayers
- **Text**: Customizable text display with templates
- **Bar**: Horizontal/vertical bars with gradients and segments
- **Arc**: Circular gauge displays
- **Graph**: Line graphs with history
- **Speedometer**: Analog speedometer-style gauge
- **Clock**: Digital and analog clock displays
- **CPU Cores**: Per-core usage visualization
- **Indicator**: Simple status indicator

### Themed Combo Displayers
Multi-slot panels with unique visual styles:
- **LCARS**: Star Trek LCARS-themed displays
- **Cyberpunk**: Neon cyberpunk aesthetic
- **Material**: Google Material Design style
- **Industrial**: Industrial/mechanical theme
- **Fighter HUD**: Aircraft heads-up display style
- **Retro Terminal**: Classic terminal/CRT look
- **Synthwave**: 80s synthwave aesthetic
- **Art Deco**: 1920s Art Deco style
- **Art Nouveau**: Organic Art Nouveau curves
- **Steampunk**: Victorian steampunk with brass and gears

### CSS Template Displayer
- **Custom HTML/CSS**: Create fully custom visualizations using HTML, CSS, and JavaScript
- **Hot Reload**: Automatic reload when template files change
- **Theme Integration**: Access theme colors via CSS custom properties
- **WebKit Powered**: Full web rendering capabilities

### Customization
- Panel backgrounds (solid, gradient, image, polygon)
- Configurable colors and gradients with theme support
- Custom fonts and text styling
- Border styles and corner radius
- Per-panel update intervals
- Grid cell size and spacing
- Global theme presets

### User Interface
- Drag-and-drop panel arrangement
- Multi-select panels (Ctrl+Click, box selection)
- Copy/paste panels and styles
- Right-click context menus
- Panel configuration dialog
- Window settings dialog (Ctrl+,)
- Fullscreen mode (double-click to toggle)
- Borderless window mode
- Auto-hide header menu in fullscreen
- Auto-scroll for large dashboards

### Keyboard Shortcuts
- **Ctrl+,** - Open settings dialog
- **Ctrl+C** - Copy selected panels
- **Ctrl+V** - Paste panels
- **Ctrl+A** - Select all panels
- **Space** (hold) - Show grid overlay
- **Delete** - Delete selected panels
- **Double-click** - Toggle fullscreen
- **Right-click** - Context menu

## Installation

### Arch Linux (AUR)

```bash
# Using an AUR helper (recommended)
yay -S rg-sens-git

# Or manually
git clone https://aur.archlinux.org/rg-sens-git.git
cd rg-sens-git
makepkg -si
```

### Prerequisites (Manual Build)

- Rust 1.70+ (install via [rustup](https://rustup.rs/))
- GTK4 development libraries
- WebKitGTK 6.0 (optional, for CSS Template displayer)

**Debian/Ubuntu:**
```bash
sudo apt install libgtk-4-dev libcairo2-dev libpango1.0-dev libwebkitgtk-6.0-dev
```

**Fedora:**
```bash
sudo dnf install gtk4-devel cairo-devel pango-devel webkitgtk6.0-devel
```

**Arch Linux:**
```bash
sudo pacman -S gtk4 cairo pango webkitgtk-6.0
```

### Build and Run

```bash
# Clone the repository
git clone https://github.com/hilgardt-collab/rg-Sens.git
cd rg-Sens

# Build (release mode recommended)
cargo build --release

# Run
cargo run --release
```

### Build Options

```bash
# Build without NVIDIA GPU support
cargo build --release --no-default-features

# Build without WebKit (CSS Template displayer disabled)
cargo build --release --no-default-features --features nvidia

# Run with debug logging
RUST_LOG=debug cargo run
```

## Command Line Options

```
rg-sens [OPTIONS] [LAYOUT_FILE]

Arguments:
  [LAYOUT_FILE]  Load a specific layout file instead of default config

Options:
  -f, --fullscreen[=<MONITOR>]  Start in fullscreen mode (optionally on specific monitor)
  -b, --borderless[=<MONITOR>]  Start in borderless mode (optionally on specific monitor)
  -l, --list-monitors           List available monitors and exit
  -a, --at <X,Y>                Position window at specific coordinates
  -h, --help                    Print help
  -V, --version                 Print version
```

### Examples

```bash
# Start fullscreen on monitor 1
rg-sens -f=1

# Start borderless at position 100,100
rg-sens -b -a 100,100

# Load a specific layout
rg-sens ~/my-dashboard.json

# List available monitors
rg-sens -l
```

## Configuration

Configuration is stored in `~/.config/rg-sens/config.json` (respects `$XDG_CONFIG_HOME`).

### Example Layouts

The `examples/` directory contains sample layouts for each themed combo panel style.

### Configuration Structure

```json
{
  "window": {
    "width": 800,
    "height": 600,
    "fullscreen_enabled": false,
    "borderless": false,
    "auto_scroll_enabled": false,
    "auto_scroll_delay_ms": 5000,
    "viewport_width": 0,
    "viewport_height": 0
  },
  "grid": {
    "columns": 10,
    "rows": 8,
    "cell_width": 100,
    "cell_height": 100,
    "spacing": 4
  },
  "panels": [...]
}
```

## Architecture

rg-Sens uses a modular, trait-based architecture:

- **DataSource trait**: Collects system metrics
- **Displayer trait**: Visualizes data using Cairo rendering
- **Panel**: Combines a source and displayer with geometry
- **GridLayout**: Manages panel placement with drag-and-drop
- **UpdateManager**: Coordinates periodic data updates
- **Registry**: Manages available sources and displayers

See [CLAUDE.md](CLAUDE.md) for detailed architecture documentation.

## Performance

Compared to the Python gSens implementation:
- **CPU Usage**: <5% idle
- **Memory**: <50MB typical
- **Rendering**: 60 FPS capable with smooth animations

## Contributing

Contributions are welcome! Areas where help is appreciated:

- Implementing new data sources
- Implementing new displayers
- Testing on different Linux distributions
- Documentation improvements
- Bug reports and feature requests

## Contributors

- H.G. Raubenheimer
- Claude (Anthropic)

## License

Dual-licensed under MIT OR Apache-2.0 (Rust ecosystem standard).

## Acknowledgments

- Original [gSens](https://github.com/hilgardt-collab/gSens) Python implementation
- GTK4 and cairo-rs communities
- Rust systems programming ecosystem
