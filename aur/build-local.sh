#!/bin/bash
# Build and install rg-sens from local source
# Run this from the aur/ directory

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(dirname "$SCRIPT_DIR")"

echo "=== Building rg-sens from local source ==="

# Create a temporary PKGBUILD for local build
cat > "$SCRIPT_DIR/PKGBUILD-local" << 'EOF'
# Local build PKGBUILD
pkgname=rg-sens
pkgver=0.5.0
pkgrel=1
pkgdesc="A fast, customizable system monitoring dashboard for Linux"
arch=('x86_64')
url="https://github.com/hilgardt-collab/rg-Sens"
license=('MIT' 'Apache-2.0')
depends=(
    'gtk4'
    'cairo'
    'pango'
    'glib2'
    'hicolor-icon-theme'
)
makedepends=(
    'rust'
    'cargo'
    'pkgconf'
)
optdepends=(
    'nvidia-utils: NVIDIA GPU monitoring support'
    'webkit2gtk-4.1: CSS Template panel with WebView support'
)
install=rg-sens.install

build() {
    cd "$startdir/.."
    export RUSTUP_TOOLCHAIN=stable
    cargo build --release
}

package() {
    cd "$startdir/.."

    install -Dm755 "target/release/rg-sens" "$pkgdir/usr/bin/rg-sens"

    install -Dm644 "data/rg-sens.desktop" \
        "$pkgdir/usr/share/applications/rg-sens.desktop"

    install -Dm644 "rg-sens.png" \
        "$pkgdir/usr/share/icons/hicolor/256x256/apps/rg-sens.png"

    install -Dm644 "data/rg-sens.metainfo.xml" \
        "$pkgdir/usr/share/metainfo/rg-sens.metainfo.xml"
}
EOF

cd "$SCRIPT_DIR"
makepkg -si --noconfirm -p PKGBUILD-local

echo ""
echo "=== Build complete! ==="
echo "Run with: rg-sens"
