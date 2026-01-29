#!/usr/bin/env python3
"""Remove preview DrawingArea widgets from config widgets."""

import re
import os

# Config widget files to process (skip color picker and widget_builder)
CONFIG_WIDGETS = [
    "src/ui/bar_config_widget.rs",
    "src/ui/arc_config_widget.rs",
    "src/ui/core_bars_config_widget.rs",
    "src/ui/speedometer_config_widget.rs",
    "src/ui/indicator_config_widget.rs",
    "src/ui/synthwave_config_widget.rs",
    "src/ui/lcars_config_widget.rs",
    "src/ui/cyberpunk_config_widget.rs",
    "src/ui/material_config_widget.rs",
    "src/ui/industrial_config_widget.rs",
    "src/ui/retro_terminal_config_widget.rs",
    "src/ui/fighter_hud_config_widget.rs",
    "src/ui/art_deco_config_widget.rs",
    "src/ui/art_nouveau_config_widget.rs",
    "src/ui/steampunk_config_widget.rs",
]

def remove_preview_from_file(filepath):
    """Remove preview-related code from a config widget file."""
    if not os.path.exists(filepath):
        print(f"  Skipping {filepath} - file not found")
        return

    with open(filepath, 'r') as f:
        content = f.read()

    original = content

    # 1. Remove "preview: DrawingArea," from struct definition
    content = re.sub(r'\s*preview: DrawingArea,\n', '\n', content)

    # 2. Remove preview creation blocks like:
    #    let preview = DrawingArea::new();
    #    preview.set_content_height(...);
    #    ...
    #    style_page.append(&preview);
    # This is tricky - let's remove line by line

    lines = content.split('\n')
    new_lines = []
    skip_until_append = False
    in_preview_block = False

    for i, line in enumerate(lines):
        # Start of preview creation
        if 'let preview = DrawingArea::new()' in line:
            in_preview_block = True
            continue

        # Lines that are part of preview setup
        if in_preview_block:
            if 'preview.set_' in line or line.strip() == '':
                continue
            elif '.append(&preview)' in line:
                in_preview_block = False
                continue
            else:
                in_preview_block = False
                # This line is not preview-related, keep it

        # Remove preview_clone declarations and queue_draw calls
        if 'let preview_clone = preview.clone()' in line:
            continue
        if 'let preview_for_' in line and 'preview.clone()' in line:
            continue
        if 'preview_clone.queue_draw()' in line:
            # Check if it's a standalone statement or inside a closure
            stripped = line.strip()
            if stripped == 'preview_clone.queue_draw();':
                continue
            # It might be in a block, keep the line but remove the call
            line = line.replace('preview_clone.queue_draw();', '')
            if line.strip() == '':
                continue
        if 'preview_for_' in line and '.queue_draw()' in line:
            stripped = line.strip()
            if '.queue_draw();' in stripped and stripped.endswith(';'):
                continue
        if 'preview.queue_draw()' in line:
            stripped = line.strip()
            if stripped == 'preview.queue_draw();':
                continue

        new_lines.append(line)

    content = '\n'.join(new_lines)

    # 3. Remove "preview," or "preview: preview," from struct instantiation
    content = re.sub(r',\s*preview:\s*preview\.clone\(\)', '', content)
    content = re.sub(r',\s*preview', '', content)
    content = re.sub(r'preview,\s*\n', '', content)
    content = re.sub(r'preview:\s*preview,\s*\n', '', content)

    # 4. Remove &preview from function arguments
    content = re.sub(r',\s*&preview\)', ')', content)
    content = re.sub(r'\(&preview,', '(', content)
    content = re.sub(r',\s*&preview,', ',', content)

    # 5. Remove preview: &DrawingArea from function parameters
    content = re.sub(r',\s*preview:\s*&DrawingArea', '', content)
    content = re.sub(r'preview:\s*&DrawingArea,\s*', '', content)

    # 6. Clean up SpinChangeHandler::new calls that had preview
    content = re.sub(r'SpinChangeHandler::new\(([^,]+),\s*preview\.clone\(\),', r'SpinChangeHandler::new(\1,', content)

    # 7. Remove self.preview references
    content = re.sub(r'\s*self\.preview\.queue_draw\(\);?\n?', '\n', content)

    if content != original:
        with open(filepath, 'w') as f:
            f.write(content)
        print(f"  Modified {filepath}")
    else:
        print(f"  No changes to {filepath}")

def main():
    print("Removing preview DrawingArea widgets from config files...")
    for filepath in CONFIG_WIDGETS:
        remove_preview_from_file(filepath)
    print("Done!")

if __name__ == "__main__":
    main()
