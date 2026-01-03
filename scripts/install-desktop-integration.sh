#!/bin/bash
# Install rg-sens desktop integration (icon and .desktop file)
# This enables the app icon to show in Wayland and desktop menus

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

# Destination directories
ICON_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor/256x256/apps"
DESKTOP_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/applications"

echo "Installing rg-sens desktop integration..."

# Create directories if needed
mkdir -p "$ICON_DIR" "$DESKTOP_DIR"

# Copy icon
cp "$PROJECT_DIR/data/icons/hicolor/256x256/apps/rg-sens.png" "$ICON_DIR/"
echo "  Installed icon to $ICON_DIR/rg-sens.png"

# Copy desktop file
cp "$PROJECT_DIR/data/com.github.hilgardt_collab.rg_sens.desktop" "$DESKTOP_DIR/"
echo "  Installed desktop file to $DESKTOP_DIR/"

# Update icon cache (optional, may not exist on all systems)
if command -v gtk-update-icon-cache &> /dev/null; then
    gtk-update-icon-cache -f -t "${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor" 2>/dev/null || true
    echo "  Updated icon cache"
fi

# Update desktop database (optional)
if command -v update-desktop-database &> /dev/null; then
    update-desktop-database "$DESKTOP_DIR" 2>/dev/null || true
    echo "  Updated desktop database"
fi

echo "Done! You may need to log out and back in for changes to take effect."
