# AUR Package for rg-Sens

This directory contains files for building and publishing rg-sens to the Arch User Repository (AUR).

## Files

| File | Purpose |
|------|---------|
| `PKGBUILD` | Release version (from tagged releases) |
| `PKGBUILD-git` | Git version (latest from main branch) |
| `rg-sens.install` | Post-install messages |
| `build-local.sh` | Build from local source for testing |

## Local Testing

Build and install from your local source:

```bash
cd aur
./build-local.sh
```

## Publishing to AUR

### First-time setup

1. Create an AUR account at https://aur.archlinux.org/
2. Add your SSH key to your AUR account
3. Clone the AUR package repo:
   ```bash
   git clone ssh://aur@aur.archlinux.org/rg-sens.git aur-repo
   # or for -git version:
   git clone ssh://aur@aur.archlinux.org/rg-sens-git.git aur-repo-git
   ```

### Publishing a release

1. Copy the appropriate PKGBUILD:
   ```bash
   cp PKGBUILD aur-repo/
   cp rg-sens.install aur-repo/
   ```

2. Update the version and generate checksums:
   ```bash
   cd aur-repo
   updpkgsums  # Updates sha256sums
   ```

3. Generate .SRCINFO:
   ```bash
   makepkg --printsrcinfo > .SRCINFO
   ```

4. Commit and push:
   ```bash
   git add PKGBUILD .SRCINFO rg-sens.install
   git commit -m "Update to version X.Y.Z"
   git push
   ```

### Publishing git version

Same process but use `PKGBUILD-git` and the `rg-sens-git` AUR repo.

## Dependencies

**Required:**
- `gtk4` - GTK4 UI framework
- `cairo` - 2D graphics
- `pango` - Text rendering
- `glib2` - GLib utilities

**Optional:**
- `nvidia-utils` - Required for NVIDIA GPU monitoring
- `webkit2gtk-4.1` - Required for CSS Template panels with WebView

## Testing the PKGBUILD

```bash
# Test build without installing
makepkg -s

# Test build and install
makepkg -si

# Check for common issues
namcap PKGBUILD
namcap rg-sens-*.pkg.tar.zst
```
