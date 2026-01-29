#!/usr/bin/env python3
"""
Remove set_draw_func calls from preview DrawingAreas.
This eliminates memory leaks caused by closures capturing Rc references.
The preview DrawingArea is kept but will just be blank/transparent.
"""

import re
import sys
import os

def remove_draw_func_from_file(filepath):
    """Remove set_draw_func blocks from preview DrawingAreas."""

    with open(filepath, 'r') as f:
        lines = f.readlines()

    original_lines = lines.copy()
    changes = []
    lines_to_remove = set()

    # Track set_draw_func blocks
    in_set_draw_func = False
    brace_count = 0
    paren_count = 0

    for i, line in enumerate(lines):
        # Track set_draw_func block
        if 'preview.set_draw_func(move |' in line:
            in_set_draw_func = True
            paren_count = line.count('(') - line.count(')')
            brace_count = line.count('{') - line.count('}')
            lines_to_remove.add(i)

            # Look backwards for config_clone/theme_clone declarations
            # that are specifically for this preview set_draw_func
            j = i - 1
            while j >= 0 and j >= i - 3:
                prev_line = lines[j].strip()
                if prev_line == '':
                    j -= 1
                    continue
                if (re.match(r'let config_clone = config\.clone\(\);', prev_line) or
                    re.match(r'let theme_clone = theme\.clone\(\);', prev_line)):
                    lines_to_remove.add(j)
                    j -= 1
                else:
                    break

            changes.append("Removed set_draw_func block")
            continue

        if in_set_draw_func:
            paren_count += line.count('(') - line.count(')')
            brace_count += line.count('{') - line.count('}')
            lines_to_remove.add(i)
            if paren_count <= 0 and brace_count <= 0:
                in_set_draw_func = False
            continue

    # Build new content
    new_lines = [line for i, line in enumerate(lines) if i not in lines_to_remove]
    content = ''.join(new_lines)

    # Remove render_checkerboard import (only used for preview drawing)
    old_content = content
    content = re.sub(r'use crate::ui::render_utils::render_checkerboard;\n', '', content)
    if content != old_content:
        changes.append("Removed render_checkerboard import")

    # Remove unused render_xxx imports if they're only used in imports
    for render_fn in ['render_bar', 'render_arc', 'render_speedometer', 'render_core_bars',
                      'render_lcars_frame', 'render_synthwave_frame', 'render_cyberpunk_frame',
                      'render_material_frame', 'render_industrial_frame', 'render_retro_terminal_frame',
                      'render_fighter_hud_frame', 'render_art_deco_frame', 'render_art_nouveau_frame',
                      'render_steampunk_frame', 'render_indicator']:
        # Count all occurrences vs import occurrences
        all_matches = list(re.finditer(rf'\b{render_fn}\b', content))
        import_matches = list(re.finditer(rf'use\s+[^;]*\b{render_fn}\b', content))

        if len(all_matches) > 0 and len(all_matches) == len(import_matches):
            old_content = content
            content = re.sub(rf',\s*{render_fn}\b', '', content)
            content = re.sub(rf'\b{render_fn}\s*,\s*', '', content)
            if content != old_content:
                changes.append(f"Removed unused {render_fn} import")

    # Clean up multiple consecutive blank lines
    content = re.sub(r'\n\n\n+', '\n\n', content)

    original_content = ''.join(original_lines)
    if content != original_content:
        with open(filepath, 'w') as f:
            f.write(content)
        return True, list(set(changes))
    return False, []

def main():
    if len(sys.argv) < 2:
        ui_dir = "/home/sakkie/Documents/GitHub/rg-Sens/src/ui"
        files = [
            os.path.join(ui_dir, f) for f in os.listdir(ui_dir)
            if f.endswith('_config_widget.rs')
        ]
    else:
        files = sys.argv[1:]

    total_modified = 0
    for filepath in sorted(files):
        if not os.path.exists(filepath):
            print(f"File not found: {filepath}")
            continue

        modified, changes = remove_draw_func_from_file(filepath)
        if modified:
            print(f"Modified: {os.path.basename(filepath)}")
            for change in changes:
                print(f"  - {change}")
            total_modified += 1
        else:
            print(f"No changes: {os.path.basename(filepath)}")

    print(f"\nTotal files modified: {total_modified}")

if __name__ == '__main__':
    main()
