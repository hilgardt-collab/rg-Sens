#!/usr/bin/env python3
"""
Remove preview DrawingArea from all config widgets.
This eliminates memory leaks caused by set_draw_func closures capturing Rc references.
The live panel on the grid serves as the preview instead.
"""

import re
import sys
import os

def remove_preview_from_rust_file(filepath):
    """Remove preview DrawingArea and related code from a Rust config widget file."""

    with open(filepath, 'r') as f:
        lines = f.readlines()

    original_lines = lines.copy()
    changes = []

    # Track lines to remove
    lines_to_remove = set()

    # Track if we're inside a set_draw_func block
    in_set_draw_func = False
    set_draw_func_start = -1
    brace_count = 0
    paren_count = 0

    for i, line in enumerate(lines):
        stripped = line.strip()

        # Remove DrawingArea from imports
        if 'use gtk4::' in line and 'DrawingArea' in line:
            # Remove DrawingArea from the import list
            new_line = re.sub(r',\s*DrawingArea\b', '', line)
            new_line = re.sub(r'\bDrawingArea\s*,\s*', '', new_line)
            new_line = re.sub(r'\bDrawingArea\b', '', new_line)
            # Clean up empty braces or trailing commas
            new_line = re.sub(r'\{\s*,', '{', new_line)
            new_line = re.sub(r',\s*\}', '}', new_line)
            new_line = re.sub(r'\{\s*\}', '', new_line)
            if new_line.strip() == 'use gtk4::;' or new_line.strip() == 'use gtk4::{};':
                lines_to_remove.add(i)
            else:
                lines[i] = new_line
            changes.append("Removed DrawingArea import")
            continue

        # Remove render_checkerboard import
        if 'use crate::ui::render_utils::render_checkerboard;' in line:
            lines_to_remove.add(i)
            changes.append("Removed render_checkerboard import")
            continue

        # Remove preview field from struct
        if re.match(r'\s+preview:\s*DrawingArea,', line):
            lines_to_remove.add(i)
            changes.append("Removed preview field from struct")
            continue

        # Remove preview from Self constructor
        if re.match(r'\s+preview,?\s*$', stripped) or re.match(r'\s+preview:\s*preview,?\s*$', stripped):
            lines_to_remove.add(i)
            changes.append("Removed preview from Self constructor")
            continue

        # Remove preview creation lines
        if re.match(r'\s+let preview = DrawingArea::new\(\);', line):
            lines_to_remove.add(i)
            changes.append("Removed preview creation")
            continue

        # Remove preview setup lines
        if re.match(r'\s+preview\.set_(content_height|content_width|hexpand|vexpand|halign|valign)\(', line):
            lines_to_remove.add(i)
            continue

        # Track set_draw_func block
        if 'preview.set_draw_func(move |' in line:
            in_set_draw_func = True
            set_draw_func_start = i
            # Count initial parens and braces
            paren_count = line.count('(') - line.count(')')
            brace_count = line.count('{') - line.count('}')
            lines_to_remove.add(i)
            changes.append("Removed set_draw_func block")
            continue

        if in_set_draw_func:
            paren_count += line.count('(') - line.count(')')
            brace_count += line.count('{') - line.count('}')
            lines_to_remove.add(i)
            if paren_count <= 0 and brace_count <= 0:
                in_set_draw_func = False
            continue

        # Remove preview append lines
        if re.match(r'\s+\w+\.append\(&preview\);', line):
            lines_to_remove.add(i)
            changes.append("Removed preview append")
            continue

        # Remove queue_draw calls on preview
        if re.match(r'\s+(preview|preview_clone|self\.preview)\.queue_draw\(\);', line):
            lines_to_remove.add(i)
            changes.append("Removed preview.queue_draw()")
            continue

        # Remove preview_clone declarations
        if re.match(r'\s+let preview_clone = (preview|self\.preview)\.clone\(\);', line):
            lines_to_remove.add(i)
            changes.append("Removed preview_clone declaration")
            continue

        # Remove preview_for_paste declarations
        if re.match(r'\s+let preview_for_paste = preview\.clone\(\);', line):
            lines_to_remove.add(i)
            changes.append("Removed preview_for_paste declaration")
            continue

        # Remove config_clone and theme_clone if they're only used for preview
        # (These are typically declared right before set_draw_func)
        if re.match(r'\s+let (config_clone|theme_clone) = (config|theme)\.clone\(\);', line):
            # Check if next non-empty line is set_draw_func
            for j in range(i + 1, min(i + 5, len(lines))):
                if lines[j].strip():
                    if 'preview.set_draw_func' in lines[j]:
                        lines_to_remove.add(i)
                    break
            continue

    # Now handle function parameters and arguments containing preview
    new_lines = []
    for i, line in enumerate(lines):
        if i in lines_to_remove:
            continue

        # Remove preview parameter from function signatures
        # Handle: preview: &DrawingArea,
        line = re.sub(r',\s*preview:\s*&DrawingArea\b', '', line)
        line = re.sub(r'\bpreview:\s*&DrawingArea,\s*', '', line)

        # Remove &preview from function call arguments
        line = re.sub(r',\s*&preview\b(?=\s*[,)])', '', line)
        line = re.sub(r'&preview,\s*', '', line)

        # Remove preview.clone() from function arguments
        line = re.sub(r',\s*preview\.clone\(\)(?=\s*[,)])', '', line)
        line = re.sub(r'preview\.clone\(\),\s*', '', line)

        # Remove &self.preview from function call arguments
        line = re.sub(r',\s*&self\.preview\b(?=\s*[,)])', '', line)
        line = re.sub(r'&self\.preview,\s*', '', line)

        # Clean up empty lines with just whitespace that might be left
        if line.strip() == '':
            # Keep single blank lines, but this will be cleaned up later
            pass

        new_lines.append(line)

    # Clean up multiple consecutive blank lines
    result_lines = []
    prev_blank = False
    for line in new_lines:
        is_blank = line.strip() == ''
        if is_blank and prev_blank:
            continue
        result_lines.append(line)
        prev_blank = is_blank

    # Check for unused render_* imports
    content = ''.join(result_lines)
    for render_fn in ['render_bar', 'render_arc', 'render_speedometer', 'render_core_bars',
                      'render_lcars_frame', 'render_synthwave_frame', 'render_cyberpunk_frame',
                      'render_material_frame', 'render_industrial_frame', 'render_retro_terminal_frame',
                      'render_fighter_hud_frame', 'render_art_deco_frame', 'render_art_nouveau_frame',
                      'render_steampunk_frame', 'render_indicator']:
        # Count occurrences - if only in imports, remove it
        all_matches = list(re.finditer(rf'\b{render_fn}\b', content))
        import_matches = list(re.finditer(rf'use\s+[^;]*\b{render_fn}\b', content))

        if len(all_matches) > 0 and len(all_matches) == len(import_matches):
            # Remove the import
            old_content = content
            content = re.sub(rf',\s*{render_fn}\b', '', content)
            content = re.sub(rf'\b{render_fn}\s*,\s*', '', content)
            if content != old_content:
                changes.append(f"Removed unused {render_fn} import")

    original_content = ''.join(original_lines)
    if content != original_content:
        with open(filepath, 'w') as f:
            f.write(content)
        return True, list(set(changes))  # Deduplicate changes
    return False, []

def main():
    if len(sys.argv) < 2:
        # Process all config widget files
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

        modified, changes = remove_preview_from_rust_file(filepath)
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
