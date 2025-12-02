# Building rg-Sens

This document provides detailed build instructions for rg-Sens.

## Prerequisites

### Rust

Install Rust using rustup:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Minimum required version: Rust 1.70+

### System Dependencies

rg-Sens requires GTK4 and related development libraries. Install them for your distribution:

#### Debian/Ubuntu
```bash
sudo apt update
sudo apt install \
    libgtk-4-dev \
    libcairo2-dev \
    libpango1.0-dev \
    libgraphene-1.0-dev \
    libgdk-pixbuf-2.0-dev \
    libglib2.0-dev \
    lm-sensors \
    pkg-config \
    build-essential
```

#### Fedora
```bash
sudo dnf install \
    gtk4-devel \
    cairo-devel \
    pango-devel \
    graphene-devel \
    gdk-pixbuf2-devel \
    glib2-devel \
    lm_sensors \
    pkg-config \
    gcc
```

#### Arch Linux
```bash
sudo pacman -S \
    gtk4 \
    cairo \
    pango \
    graphene \
    gdk-pixbuf2 \
    glib2 \
    lm_sensors \
    pkg-config \
    base-devel
```

#### openSUSE
```bash
sudo zypper install \
    gtk4-devel \
    cairo-devel \
    pango-devel \
    libgraphene-devel \
    gdk-pixbuf-devel \
    glib2-devel \
    sensors \
    pkg-config \
    gcc
```

### Optional: NVIDIA GPU Support

For NVIDIA GPU monitoring, install NVML:

```bash
# Debian/Ubuntu
sudo apt install nvidia-utils

# Fedora
sudo dnf install nvidia-driver

# Arch
sudo pacman -S nvidia-utils
```

To disable NVIDIA support if not needed:
```bash
cargo build --no-default-features
```

## Building

### Debug Build

For development with debug symbols:
```bash
cargo build
```

Binary will be at: `target/debug/rg-sens`

### Release Build

For optimized production build:
```bash
cargo build --release
```

Binary will be at: `target/release/rg-sens`

The release build applies aggressive optimizations:
- Link-time optimization (LTO)
- Single codegen unit
- Optimization level 3
- Symbol stripping

## Running

### From Cargo
```bash
# Debug
cargo run

# Release
cargo run --release
```

### Direct Execution
```bash
# Debug
./target/debug/rg-sens

# Release
./target/release/rg-sens
```

### With Logging
```bash
# Enable info-level logging
RUST_LOG=info cargo run

# Enable debug-level logging
RUST_LOG=debug cargo run

# Enable trace-level logging for specific module
RUST_LOG=rg_sens::core=trace cargo run
```

## Testing

### Run All Tests
```bash
cargo test
```

### Run Specific Tests
```bash
# Unit tests only
cargo test --lib

# Integration tests only
cargo test --test integration_tests

# Specific test
cargo test test_placeholder
```

### With Logging in Tests
```bash
cargo test -- --nocapture
```

## Benchmarking

```bash
cargo bench
```

## Checking Code

### Quick Check (no code generation)
```bash
cargo check
```

### With Clippy (linter)
```bash
cargo clippy
```

### Format Check
```bash
cargo fmt --check
```

### Format Code
```bash
cargo fmt
```

## Installation

### System-wide Installation
```bash
cargo install --path .
```

This installs to `~/.cargo/bin/rg-sens`

### Manual Installation
```bash
# Build release binary
cargo build --release

# Copy to system binary directory
sudo cp target/release/rg-sens /usr/local/bin/

# Or to user binary directory
mkdir -p ~/.local/bin
cp target/release/rg-sens ~/.local/bin/
```

## Troubleshooting

### "gtk4.pc not found"

**Problem**: pkg-config cannot find GTK4 libraries.

**Solution**: Install GTK4 development packages for your distribution (see Prerequisites above).

### "nvml-wrapper build failed"

**Problem**: NVIDIA libraries not found.

**Solution**: Either install nvidia-utils or disable NVIDIA support:
```bash
cargo build --no-default-features
```

### "sensors.h not found"

**Problem**: lm-sensors development files not installed.

**Solution**: Install lm-sensors:
```bash
# Debian/Ubuntu
sudo apt install libsensors-dev

# Fedora
sudo dnf install lm_sensors-devel

# Arch
sudo pacman -S lm_sensors
```

### Linking Errors

**Problem**: Linker cannot find libraries.

**Solution**: Ensure `pkg-config` is installed and working:
```bash
pkg-config --cflags --libs gtk4
```

If this fails, check your PKG_CONFIG_PATH.

## Development Tips

### Watch Mode

Install cargo-watch for automatic rebuilds:
```bash
cargo install cargo-watch
cargo watch -x check -x test
```

### Faster Compilation

Use mold linker for faster linking (Linux only):
```bash
# Install mold
sudo apt install mold  # Debian/Ubuntu
sudo pacman -S mold    # Arch

# Add to ~/.cargo/config.toml
[target.x86_64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=mold"]
```

Or use lld:
```bash
sudo apt install lld
# Add to ~/.cargo/config.toml
[target.x86_64-unknown-linux-gnu]
rustflags = ["-C", "link-arg=-fuse-ld=lld"]
```

### IDE Setup

#### VS Code

Install extensions:
- rust-analyzer
- CodeLLDB (for debugging)
- crates (dependency management)

#### CLion / RustRover

Rust support is built-in, just open the project.

### Debugging

#### Command Line (gdb)
```bash
cargo build
gdb target/debug/rg-sens
```

#### VS Code (launch.json)
```json
{
    "version": "0.2.0",
    "configurations": [
        {
            "type": "lldb",
            "request": "launch",
            "name": "Debug rg-sens",
            "cargo": {
                "args": ["build", "--bin=rg-sens"],
                "filter": {
                    "name": "rg-sens",
                    "kind": "bin"
                }
            },
            "args": [],
            "cwd": "${workspaceFolder}"
        }
    ]
}
```

## Cross-Compilation

Currently, rg-Sens is Linux-only. Cross-compilation for other targets is not supported yet.

## CI/CD

GitHub Actions workflows are provided for:
- Automated testing
- Linting (clippy)
- Format checking
- Building release artifacts

See `.github/workflows/` for details.
