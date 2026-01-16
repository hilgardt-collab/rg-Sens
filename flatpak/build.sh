#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

cd "$SCRIPT_DIR"

echo "=== rg-Sens Flatpak Build ==="

# Check for required tools
if ! command -v flatpak &> /dev/null; then
    echo "Error: flatpak not installed. Install with: sudo pacman -S flatpak"
    exit 1
fi

if ! command -v flatpak-builder &> /dev/null; then
    echo "Error: flatpak-builder not installed. Install with: sudo pacman -S flatpak-builder"
    exit 1
fi

# Install required runtimes if not present
echo "Checking for GNOME 47 runtime..."
if ! flatpak info org.gnome.Platform//47 &> /dev/null; then
    echo "Installing GNOME 47 Platform runtime..."
    flatpak install -y flathub org.gnome.Platform//47
fi

if ! flatpak info org.gnome.Sdk//47 &> /dev/null; then
    echo "Installing GNOME 47 SDK..."
    flatpak install -y flathub org.gnome.Sdk//47
fi

if ! flatpak info org.freedesktop.Sdk.Extension.rust-stable//24.08 &> /dev/null; then
    echo "Installing Rust SDK extension..."
    flatpak install -y flathub org.freedesktop.Sdk.Extension.rust-stable//24.08
fi

# Build the Flatpak
echo "Building Flatpak..."
flatpak-builder --force-clean --user --install-deps-from=flathub build-dir io.github.hilgardtcollab.RgSens.yml

echo ""
echo "=== Build complete ==="
echo "To install locally: flatpak-builder --user --install --force-clean build-dir io.github.hilgardtcollab.RgSens.yml"
echo "To run: flatpak run io.github.hilgardtcollab.RgSens"
