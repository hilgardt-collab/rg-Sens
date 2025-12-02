# rg-Sens

A fast, customizable system monitoring dashboard for Linux, written in Rust.

## Overview

rg-Sens is a Rust port of [gSens](https://github.com/hilgardt-collab/gSens), designed to provide:
- **High Performance**: Lower CPU and memory usage compared to Python implementation
- **Customizable Grid Layout**: Drag-and-drop panels with flexible sizing
- **Rich Visualizations**: Multiple displayer types (gauges, graphs, LCARS themes, etc.)
- **Extensible**: Plugin system for custom data sources and displayers
- **Native**: Built with GTK4 for seamless Linux desktop integration

## Status

🚧 **Early Development** - The project is in its initial stages. Core architecture is being established.

### Roadmap

- [x] Project structure and core traits
- [ ] Basic GTK4 window and grid layout
- [ ] First data source (CPU)
- [ ] First displayer (text/level bar)
- [ ] Configuration loading/saving
- [ ] Additional sources (Memory, GPU, Temps, etc.)
- [ ] Additional displayers (gauges, graphs, etc.)
- [ ] Drag-and-drop panel arrangement
- [ ] Configuration dialog
- [ ] Plugin system (dynamic loading)
- [ ] Python gSens config migration

## Building

### Prerequisites

- Rust 1.70+ (install via [rustup](https://rustup.rs/))
- GTK4 development libraries
- System monitoring libraries

**On Debian/Ubuntu:**
```bash
sudo apt install libgtk-4-dev libcairo2-dev libpango1.0-dev lm-sensors
```

**On Fedora:**
```bash
sudo dnf install gtk4-devel cairo-devel pango-devel lm_sensors
```

**On Arch:**
```bash
sudo pacman -S gtk4 cairo pango lm_sensors
```

### Build and Run

```bash
# Clone the repository
git clone https://github.com/hilgardt-collab/rg-Sens.git
cd rg-Sens

# Build
cargo build --release

# Run
cargo run --release
```

## Architecture

rg-Sens is built around a modular architecture:

- **Data Sources**: Collect system metrics (CPU, memory, GPU, temps, etc.)
- **Displayers**: Visualize data (text, gauges, graphs, etc.)
- **Panels**: Container combining a source and displayer
- **Grid Layout**: Manages panel placement and sizing
- **Update Manager**: Coordinates periodic data updates
- **Registry**: Manages available sources and displayers

See [ARCHITECTURE.md](ARCHITECTURE.md) for detailed information.

## Configuration

Configuration is stored in `~/.config/rg-sens/config.json` (or `$XDG_CONFIG_HOME/rg-sens/config.json`).

The configuration format is designed to be compatible with Python gSens for easy migration.

## Features

### Current Features
- Core architecture with trait-based design
- Configuration management

### Planned Features
- Multiple data sources:
  - CPU (usage, frequency, per-core stats)
  - Memory (RAM, swap)
  - GPU (NVIDIA, AMD, Intel)
  - Temperatures (lm-sensors)
  - Disk I/O
  - Network I/O
  - System processes
  - And more...

- Multiple displayers:
  - Text display
  - Level bars
  - Arc gauges
  - Line graphs
  - Multi-core CPU grid
  - Analog clock
  - LCARS themes
  - And more...

- Customization:
  - Colors and gradients
  - Fonts and text styling
  - Border styles
  - Update intervals

## Performance Goals

Compared to the Python gSens implementation:
- **CPU Usage**: <5% idle (vs ~10-15% for Python)
- **Memory**: <50MB (vs ~100-150MB for Python)
- **Update Latency**: <10ms per source
- **Rendering**: 60 FPS capable

## Contributing

Contributions are welcome! This is a hobby project in early development.

Areas where help is appreciated:
- Implementing data sources
- Implementing displayers
- Testing on different Linux distributions
- Documentation
- Bug reports and feature requests

## License

MIT OR Apache-2.0 (same as Rust ecosystem standard)

## Acknowledgments

- Original [gSens](https://github.com/hilgardt-collab/gSens) Python implementation
- GTK and cairo-rs communities
- Rust systems programming ecosystem
