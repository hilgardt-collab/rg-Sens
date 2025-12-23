# Combo Panel Implementation - Lessons Learned

This document records bugs discovered after implementing the Cyberpunk combo panel, to help prevent the same mistakes when creating future combo panels.

## 1. Multi-Item Group Rendering

**Bug**: Only the first item in each group was displayed, none of the others.

**Root Cause**: `set_source_summaries()` wasn't updating `group_item_counts` from the summaries.

**Fix**: Extract group configuration from summaries and update `group_count`, `group_item_counts`, and `group_size_weights`:

```rust
pub fn set_source_summaries(&self, summaries: Vec<(String, String, usize, u32)>) {
    // Extract group configuration from summaries
    let mut group_item_counts: HashMap<usize, u32> = HashMap::new();
    for (_, _, group_num, item_idx) in &summaries {
        let current_max = group_item_counts.entry(*group_num).or_insert(0);
        if *item_idx > *current_max {
            *current_max = *item_idx;
        }
    }
    // Update config with group_count, group_item_counts, group_size_weights...
}
```

**Prevention**: Always ensure `set_source_summaries()` updates all group-related config fields.

---

## 2. Field Selection Missing in Embedded Widgets

**Bug**: Arc/Speedometer text overlay config didn't have selectable fields - dropdowns were empty.

**Root Cause**: Embedded config widgets (ArcConfigWidget, SpeedometerConfigWidget) were created with `vec![]` instead of `slot_fields.clone()`.

**Fix**:
```rust
// WRONG:
let arc_widget = ArcConfigWidget::new(vec![]);

// CORRECT:
let arc_widget = ArcConfigWidget::new(slot_fields.clone());
```

**Prevention**: Always pass `slot_fields.clone()` to any embedded widget that needs field selection.

---

## 3. Text Config Changes Not Applying

**Bug**: Changing text options (font, size, etc.) in Arc/Speedometer had no effect.

**Root Cause**: TextLineConfigWidget's `on_change` callback wasn't connected to update the parent config and trigger redraws.

**Fix**: Wrap text widget in `Rc` and connect `set_on_change`:
```rust
let text_widget_rc = Rc::new(text_widget);
let text_widget_for_callback = text_widget_rc.clone();
text_widget_rc.set_on_change(move || {
    config_for_text.borrow_mut().text_overlay.text_config = text_widget_for_callback.get_config();
    if let Some(cb) = on_change_for_text.borrow().as_ref() {
        cb();
    }
});
```

**Prevention**: Always connect `on_change` callbacks for ALL embedded config widgets, not just the main ones.

---

## 4. Group Size Weights Not Updating Dynamically

**Bug**: Group Size Weights spinners in Layout tab didn't update when source groups were added/removed.

**Root Cause**: `set_source_summaries()` only called `rebuild_content_tabs()`, not `rebuild_group_spinners()`.

**Fix**: Add call to rebuild group spinners when summaries change:
```rust
pub fn set_source_summaries(&self, summaries: ...) {
    // ... update config ...

    // Rebuild group weight spinners in Layout tab
    if let Some(widgets) = self.layout_widgets.borrow().as_ref() {
        Self::rebuild_group_spinners(
            &self.config,
            &self.on_change,
            &self.preview,
            &widgets.group_weights_box,
        );
    }

    Self::rebuild_content_tabs(...);
}
```

**Prevention**: When source configuration changes, rebuild ALL dynamic UI elements, not just content tabs.

---

## 5. Missing Copy/Paste for Gradients

**Bug**: Speedometer track gradient didn't have Copy/Paste buttons like other gradient editors.

**Root Cause**: Simply forgot to add the buttons when creating the track page.

**Fix**: Add copy/paste buttons using the standard pattern:
```rust
let copy_paste_box = GtkBox::new(Orientation::Horizontal, 6);
let copy_gradient_btn = Button::with_label("Copy Gradient");
let paste_gradient_btn = Button::with_label("Paste Gradient");

copy_gradient_btn.connect_clicked(move |_| {
    use crate::ui::CLIPBOARD;
    if let Ok(mut clipboard) = CLIPBOARD.lock() {
        clipboard.copy_gradient_stops(config.borrow().track_color_stops.clone());
    }
});

paste_gradient_btn.connect_clicked(move |_| {
    use crate::ui::CLIPBOARD;
    if let Ok(clipboard) = CLIPBOARD.lock() {
        if let Some(stops) = clipboard.paste_gradient_stops() {
            config.borrow_mut().track_color_stops = stops.clone();
            gradient_editor.set_gradient(&LinearGradientConfig { angle: 0.0, stops });
            // trigger redraw...
        }
    }
});
```

**Prevention**: Use a checklist when adding gradient editors - always include copy/paste buttons.

---

## 6. Percentage Values Not Scaled in Paste

**Bug**: When copy/pasting bar config, width% and height% always showed 10%.

**Root Cause**: Paste callback set values directly without multiplying by 100 to convert from stored format (0.0-1.0) to display format (10%-100%).

**Fix**:
```rust
// WRONG:
rect_width_spin_paste.set_value(cfg.rectangle_width);

// CORRECT:
rect_width_spin_paste.set_value(cfg.rectangle_width * 100.0);
```

**Prevention**: When implementing paste, check if `set_config()` does any scaling - paste should match that scaling.

---

## 7. Hardcoded Layout Values

**Bug**: Divider padding was hardcoded to 4px/8px, not configurable.

**Root Cause**: Used magic numbers instead of config fields during initial implementation.

**Fix**: Add config field with default function:
```rust
fn default_divider_padding() -> f64 { 4.0 }

pub struct FrameConfig {
    #[serde(default = "default_divider_padding")]
    pub divider_padding: f64,
    // ...
}
```

Then replace all hardcoded values:
```rust
// WRONG:
let divider_space = divider_count as f64 * (config.divider_width + 8.0);

// CORRECT:
let divider_space = divider_count as f64 * (config.divider_width + config.divider_padding * 2.0);
```

**Prevention**: Never use magic numbers for layout. Always create config fields, even if using a sensible default initially.

---

## 8. Displayer ID Must Be Fixed String

**Bug**: Opening config dialog changed displayer away from "industrial", wiping all settings.

**Root Cause**: Displayer's `id` field was set to a random UUID instead of the fixed string "industrial":
```rust
// WRONG:
id: uuid::Uuid::new_v4().to_string(),

// CORRECT:
id: "industrial".to_string(),
```

**Fix**: Use the displayer's registered ID (e.g., "industrial", "cyberpunk") as the id field value.

**Prevention**: Always use a fixed string matching the registered displayer ID, never a UUID.

---

## 9. Caption Field Name Mismatch

**Bug**: Content items not displaying - captions were empty.

**Root Cause**: `get_item_data()` looked for `{prefix}_label` but combo sources generate `{prefix}_caption`.

**Fix**:
```rust
// WRONG:
let caption = values.get(&format!("{}_label", prefix))

// CORRECT:
let caption = values.get(&format!("{}_caption", prefix))
```

**Prevention**: Check existing displayers (Cyberpunk, LCARS) for field name conventions before implementing.

---

## 10. Numerical Value Fallback Missing

**Bug**: Bars not animating because `numerical_value` was always 0.

**Root Cause**: Looking for `{prefix}_numerical_value` which doesn't exist - sources provide `{prefix}_value`.

**Fix**: Fall back to `_value` field:
```rust
let numerical_value = values
    .get(&format!("{}_numerical_value", prefix))
    .or_else(|| values.get(&format!("{}_value", prefix)))
    .and_then(|v| v.as_f64())
    .unwrap_or(0.0);
```

**Prevention**: Always provide fallback when looking for optional fields.

---

## 11. Group Layout Tuple Mismatch

**Bug**: Content items not drawing at all.

**Root Cause**: `calculate_group_layouts()` returns `(x, y, w, h, item_count)` but loop treated 5th element as `group_idx`.

**Fix**:
```rust
// WRONG:
for (group_x, group_y, group_w, group_h, group_idx) in &group_layouts {
    let item_count = config.group_item_counts.get(*group_idx)...

// CORRECT:
for (group_idx, (group_x, group_y, group_w, group_h, item_count)) in group_layouts.iter().enumerate() {
    // item_count comes from tuple, group_idx from enumerate
```

**Prevention**: Document tuple field order clearly. Use named structs instead of tuples for complex return types.

---

## Checklist for New Combo Panels

When creating a new combo panel, verify:

- [ ] Displayer `id` field is a fixed string matching the registered ID (NOT a UUID)
- [ ] `get_item_data()` uses `_caption` (not `_label`) for caption field
- [ ] `get_item_data()` falls back to `_value` when `_numerical_value` not found
- [ ] Group layout loops use `enumerate()` for group index, tuple element for item count
- [ ] `set_source_summaries()` updates `group_count`, `group_item_counts`, and `group_size_weights`
- [ ] `set_source_summaries()` rebuilds group weight spinners (not just content tabs)
- [ ] All embedded widgets receive `slot_fields.clone()` for field selection
- [ ] All embedded widgets have their `on_change` callbacks connected
- [ ] All gradient editors have Copy/Paste buttons
- [ ] Paste callbacks scale percentage values correctly (match `set_config()` scaling)
- [ ] No hardcoded magic numbers for layout - use config fields with defaults
- [ ] Default impl includes all new config fields
