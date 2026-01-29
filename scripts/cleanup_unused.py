#!/usr/bin/env python3
"""
Clean up unused variables and imports left over from preview removal.
"""

import re
import os

def cleanup_file(filepath):
    """Remove unused clone variables that were only used for preview."""

    with open(filepath, 'r') as f:
        content = f.read()

    original = content
    changes = []

    # Remove unused config_clone and theme_clone declarations
    # These were only used in set_draw_func blocks
    patterns = [
        (r'\n\s+let config_clone = config\.clone\(\);(?=\n)', '', 'Removed unused config_clone'),
        (r'\n\s+let theme_clone = theme\.clone\(\);(?=\n)', '', 'Removed unused theme_clone'),
        (r'\n\s+let config_for_preview = config\.clone\(\);(?=\n)', '', 'Removed unused config_for_preview'),
        (r'\n\s+let theme_for_preview = theme\.clone\(\);(?=\n)', '', 'Removed unused theme_for_preview'),
    ]

    for pattern, replacement, msg in patterns:
        old = content
        content = re.sub(pattern, replacement, content)
        if content != old:
            changes.append(msg)

    # Remove unused render_* imports from specific files
    # speedometer: remove render_speedometer_with_theme
    if 'speedometer_config_widget' in filepath:
        old = content
        content = re.sub(r'render_speedometer_with_theme,\s*', '', content)
        if content != old:
            changes.append('Removed unused render_speedometer_with_theme import')

    # lcars: remove render_content_background
    if 'lcars_config_widget' in filepath:
        old = content
        content = re.sub(r'render_content_background,\s*', '', content)
        if content != old:
            changes.append('Removed unused render_content_background import')

    # Clean up multiple blank lines
    content = re.sub(r'\n\n\n+', '\n\n', content)

    if content != original:
        with open(filepath, 'w') as f:
            f.write(content)
        return True, changes
    return False, []

def main():
    ui_dir = "/home/sakkie/Documents/GitHub/rg-Sens/src/ui"
    files = [
        os.path.join(ui_dir, f) for f in os.listdir(ui_dir)
        if f.endswith('_config_widget.rs')
    ]

    total = 0
    for filepath in sorted(files):
        modified, changes = cleanup_file(filepath)
        if modified:
            print(f"Modified: {os.path.basename(filepath)}")
            for c in changes:
                print(f"  - {c}")
            total += 1

    print(f"\nTotal files modified: {total}")

if __name__ == '__main__':
    main()
