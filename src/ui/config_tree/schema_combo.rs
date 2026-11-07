//! A schema-driven editor for the themed combo frames: rows from the frame
//! config's `Schema` (sections as tree nodes), the combo surface
//! (`ComboDisplayEditor`) over the same JSON model, and item display editors
//! from `combo_items` (their pages are the item's child nodes). The frame's
//! cached `theme`, `content_items` and group caches have no props and pass
//! through untouched.
//!
//! The displayer config is a wrapper `{ frame, animation_enabled,
//! animation_speed }`; the editor works on the frame object with the two
//! animation keys copied in (what `to_frame`/`from_frame` do).

use super::combo_items::{coerce_display_as, item_tree_pages, ItemDisplayEditor};
use super::schema_editor::SchemaEditor;
use crate::core::{DisplayerConfig, FieldMetadata};
use crate::ui::editor_registry::{
    ChangeCallback, ComboDisplayEditor, ComboReorder, DisplayEditor, EditorPage,
};
use crate::ui::theme::ComboThemeConfig;
use gtk4::prelude::*;
use gtk4::Widget;
use rg_sens_types::combo::{GroupTitleConfig, TransferableComboConfig};
use rg_sens_types::schema::{Prop, TreePage};
use serde_json::{Map, Value};
use std::cell::RefCell;
use std::rc::Rc;

const ANIMATION_KEYS: [&str; 2] = ["animation_enabled", "animation_speed"];

/// The frame object of a wrapper config, with the wrapper's animation keys.
pub fn frame_from_wrapper(wrapper: &Value) -> Map<String, Value> {
    let mut frame = wrapper
        .get("frame")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    for k in ANIMATION_KEYS {
        if let Some(v) = wrapper.get(k) {
            frame.insert(k.to_string(), v.clone());
        }
    }
    frame
}

/// The wrapper config for a frame object.
pub fn wrapper_from_frame(frame: &Map<String, Value>) -> Value {
    let mut wrapper = Map::new();
    for k in ANIMATION_KEYS {
        if let Some(v) = frame.get(k) {
            wrapper.insert(k.to_string(), v.clone());
        }
    }
    wrapper.insert("frame".to_string(), Value::Object(frame.clone()));
    Value::Object(wrapper)
}

/// LCARS spells the layout orientation `layout_orientation`; the others
/// `split_orientation`.
fn orientation_key(frame: &Map<String, Value>) -> &'static str {
    if frame.contains_key("layout_orientation") {
        "layout_orientation"
    } else {
        "split_orientation"
    }
}

fn get<T: serde::de::DeserializeOwned + Default>(frame: &Map<String, Value>, key: &str) -> T {
    frame
        .get(key)
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default()
}

/// What a displayer switch carries from one combo frame to another.
pub fn transferable_from_frame(frame: &Map<String, Value>) -> TransferableComboConfig {
    TransferableComboConfig {
        group_count: get(frame, "group_count"),
        group_item_counts: get(frame, "group_item_counts"),
        group_size_weights: get(frame, "group_size_weights"),
        group_item_orientations: get(frame, "group_item_orientations"),
        group_titles: get(frame, "group_titles"),
        layout_orientation: get(frame, orientation_key(frame)),
        content_items: get(frame, "content_items"),
        content_padding: get(frame, "content_padding"),
        item_spacing: get(frame, "item_spacing"),
        animation_enabled: get(frame, "animation_enabled"),
        animation_speed: get(frame, "animation_speed"),
    }
}

pub fn apply_transferable_to_frame(frame: &mut Map<String, Value>, t: &TransferableComboConfig) {
    let put = |frame: &mut Map<String, Value>, key: &str, v: Result<Value, serde_json::Error>| {
        if let Ok(v) = v {
            frame.insert(key.to_string(), v);
        }
    };
    put(frame, "group_count", serde_json::to_value(t.group_count));
    put(
        frame,
        "group_item_counts",
        serde_json::to_value(&t.group_item_counts),
    );
    put(
        frame,
        "group_size_weights",
        serde_json::to_value(&t.group_size_weights),
    );
    put(
        frame,
        "group_item_orientations",
        serde_json::to_value(&t.group_item_orientations),
    );
    put(frame, "group_titles", serde_json::to_value(&t.group_titles));
    let ok = orientation_key(frame);
    put(frame, ok, serde_json::to_value(t.layout_orientation));
    put(
        frame,
        "content_items",
        serde_json::to_value(&t.content_items),
    );
    put(
        frame,
        "content_padding",
        serde_json::to_value(t.content_padding),
    );
    put(frame, "item_spacing", serde_json::to_value(t.item_spacing));
    put(
        frame,
        "animation_enabled",
        serde_json::to_value(t.animation_enabled),
    );
    put(
        frame,
        "animation_speed",
        serde_json::to_value(t.animation_speed),
    );
}

/// The JSON of a group without a title bar.
fn default_group_title() -> Value {
    serde_json::to_value(GroupTitleConfig::default()).unwrap_or_else(|_| Value::Object(Map::new()))
}

/// The group's title bar from the frame, or the default when the frame's
/// list does not reach the group.
pub fn group_title_of(frame: &Map<String, Value>, group: usize) -> Value {
    frame
        .get("group_titles")
        .and_then(|v| v.as_array())
        .and_then(|titles| titles.get(group))
        .filter(|v| v.is_object())
        .cloned()
        .unwrap_or_else(default_group_title)
}

/// Write the group's title bar, padding the frame's list with defaults up
/// to the group (the list is parallel to the group weights).
pub fn set_group_title_in(frame: &mut Map<String, Value>, group: usize, title: Value) {
    let titles = frame
        .entry("group_titles".to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    if !titles.is_array() {
        *titles = Value::Array(Vec::new());
    }
    if let Some(titles) = titles.as_array_mut() {
        while titles.len() <= group {
            titles.push(default_group_title());
        }
        titles[group] = title;
    }
}

/// The frame's group layout (`"vertical"` unless set).
fn layout_orientation_of(frame: &Map<String, Value>) -> String {
    frame
        .get(orientation_key(frame))
        .and_then(|v| v.as_str())
        .unwrap_or("vertical")
        .to_string()
}

/// How the group lays out its items: its own entry in the frame's list, else
/// the frame's group layout (the list may stop short of the group).
pub fn group_orientation_of(frame: &Map<String, Value>, group: usize) -> String {
    frame
        .get("group_item_orientations")
        .and_then(|v| v.as_array())
        .and_then(|list| list.get(group))
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .unwrap_or_else(|| layout_orientation_of(frame))
}

/// Write the group's item orientation, padding the frame's list with the
/// group layout up to the group (the list is parallel to the group weights).
pub fn set_group_orientation_in(frame: &mut Map<String, Value>, group: usize, orientation: &str) {
    let default = layout_orientation_of(frame);
    let list = frame
        .entry("group_item_orientations".to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    if !list.is_array() {
        *list = Value::Array(Vec::new());
    }
    if let Some(list) = list.as_array_mut() {
        while list.len() <= group {
            list.push(Value::String(default.clone()));
        }
        list[group] = Value::String(orientation.to_string());
    }
}

pub struct SchemaComboEditor {
    displayer_id: String,
    editor: SchemaEditor,
    /// The resolved theme last pushed, shared with the item pages.
    theme: Rc<RefCell<ComboThemeConfig>>,
}

impl SchemaComboEditor {
    pub fn create(
        displayer_id: &str,
        props: Vec<Prop>,
        fields: Vec<FieldMetadata>,
    ) -> Box<dyn DisplayEditor> {
        Box::new(Self {
            displayer_id: displayer_id.to_string(),
            editor: SchemaEditor::for_tree(props, fields, true),
            theme: Rc::new(RefCell::new(ComboThemeConfig::default())),
        })
    }

    fn typed(&self) -> Option<DisplayerConfig> {
        DisplayerConfig::from_value_for_type(
            &self.displayer_id,
            wrapper_from_frame(&self.editor.model()),
        )
    }

    fn load_typed(&self, config: &DisplayerConfig) {
        if let Some(wrapper) = config.to_inner_value() {
            self.editor.load(frame_from_wrapper(&wrapper));
        }
    }
}

impl DisplayEditor for SchemaComboEditor {
    fn widget(&self) -> Widget {
        self.editor.widget().clone().upcast()
    }

    fn load(&self, config: &DisplayerConfig) {
        if config.displayer_type() != self.displayer_id {
            return;
        }
        self.load_typed(config);
    }

    fn store(&self) -> Option<DisplayerConfig> {
        self.typed()
    }

    fn set_on_change(&self, cb: ChangeCallback) -> bool {
        self.editor.set_on_change(cb);
        true
    }

    fn set_theme(&self, theme: &ComboThemeConfig) {
        *self.theme.borrow_mut() = theme.clone();
        // The frame's `theme` is the resolved cache (gotcha 20): keep the
        // model's copy current so the dialog's baseline reads the same value
        // the panel renders with.
        if let Ok(v) = serde_json::to_value(theme) {
            self.editor
                .model_rc()
                .borrow_mut()
                .insert("theme".to_string(), v);
        }
        self.editor.set_theme(theme);
    }

    fn cleanup(&self) {
        self.editor.cleanup();
    }

    fn sections(&self) -> Vec<EditorPage> {
        self.editor
            .pages()
            .into_iter()
            .map(|(name, page)| EditorPage {
                path: vec![name],
                widget: page.upcast(),
            })
            .chain(
                self.editor
                    .item_pages()
                    .into_iter()
                    .map(|(path, page)| EditorPage {
                        path,
                        widget: page.upcast(),
                    }),
            )
            .collect()
    }

    fn page_title(&self, path: &[String]) -> String {
        match path {
            [name] => name.clone(),
            _ => self
                .editor
                .item_page_title(path)
                .unwrap_or_else(|| path.last().cloned().unwrap_or_default()),
        }
    }

    fn page_search_terms(&self, path: &[String]) -> Vec<String> {
        match path {
            [name] => self.editor.section_search_terms(name),
            _ => self.editor.item_page_search_terms(path),
        }
    }

    fn set_on_sections_changed(&self, cb: Rc<dyn Fn(Option<Vec<String>>)>) {
        self.editor.set_on_item_pages_changed(cb);
    }

    fn combo(&self) -> Option<&dyn ComboDisplayEditor> {
        Some(self)
    }
}

impl ComboDisplayEditor for SchemaComboEditor {
    fn set_group_weights(&self, weights: &[f64]) {
        // A cache the source owns: written silently.
        if let Ok(v) = serde_json::to_value(weights) {
            self.editor
                .model_rc()
                .borrow_mut()
                .insert("group_size_weights".to_string(), v);
        }
    }

    fn transferable(&self) -> TransferableComboConfig {
        transferable_from_frame(&self.editor.model())
    }

    fn apply_transferable(&self, transfer: &TransferableComboConfig) {
        let mut frame = self.editor.model();
        apply_transferable_to_frame(&mut frame, transfer);
        self.editor.load(frame);
    }

    fn reorder(&self, op: ComboReorder) {
        let Some(mut typed) = self.typed() else {
            return;
        };
        match op {
            ComboReorder::Group { a, b } => typed.swap_groups(a, b),
            ComboReorder::Item { group, i, j } => typed.swap_items(group, i, j),
        }
        self.load_typed(&typed);
    }

    fn item_editor(
        &self,
        slot: &str,
        source_id: &str,
        fields: &[FieldMetadata],
        on_change: ChangeCallback,
    ) -> Rc<ItemDisplayEditor> {
        ItemDisplayEditor::new(
            self.editor.model_rc(),
            slot,
            source_id,
            fields.to_vec(),
            self.theme.clone(),
            on_change,
        )
    }

    fn coerce_item_display(&self, slot: &str, source_id: &str) -> bool {
        coerce_display_as(&self.editor.model_rc(), slot, source_id)
    }

    fn item_pages(&self, slot: &str) -> Vec<TreePage> {
        let model = self.editor.model_rc();
        let frame = model.borrow();
        item_tree_pages(&frame, slot)
    }

    fn content_item(&self, slot: &str) -> Option<Value> {
        let model = self.editor.model_rc();
        let frame = model.borrow();
        frame
            .get("content_items")
            .and_then(|items| items.get(slot))
            .filter(|v| v.is_object())
            .cloned()
    }

    fn set_content_item(&self, slot: &str, item: Value) {
        let model = self.editor.model_rc();
        let mut frame = model.borrow_mut();
        let items = frame
            .entry("content_items".to_string())
            .or_insert_with(|| Value::Object(Map::new()));
        if !items.is_object() {
            *items = Value::Object(Map::new());
        }
        if let Some(items) = items.as_object_mut() {
            items.insert(slot.to_string(), item);
        }
    }

    fn group_title(&self, group: usize) -> Value {
        let model = self.editor.model_rc();
        let frame = model.borrow();
        group_title_of(&frame, group)
    }

    fn set_group_title(&self, group: usize, title: Value) {
        let model = self.editor.model_rc();
        let mut frame = model.borrow_mut();
        set_group_title_in(&mut frame, group, title);
    }

    fn group_orientation(&self, group: usize) -> String {
        let model = self.editor.model_rc();
        let frame = model.borrow();
        group_orientation_of(&frame, group)
    }

    fn set_group_orientation(&self, group: usize, orientation: &str) {
        let model = self.editor.model_rc();
        let mut frame = model.borrow_mut();
        set_group_orientation_in(&mut frame, group, orientation);
    }

    fn refresh_theme_refs(&self) {
        // `set_theme` reaches every row; the dialog pushes the theme to the
        // item editors it built.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rg_sens_types::display_configs::themed_configs::SynthwaveDisplayConfig;
    use rg_sens_types::display_configs::SplitOrientation;

    #[test]
    fn wrapper_and_frame_round_trip() {
        let wrapper = serde_json::to_value(SynthwaveDisplayConfig::default()).unwrap();
        let frame = frame_from_wrapper(&wrapper);
        assert!(frame.contains_key("animation_enabled"));
        let back = wrapper_from_frame(&frame);
        let parsed: SynthwaveDisplayConfig = serde_json::from_value(back).unwrap();
        assert_eq!(
            serde_json::to_value(parsed).unwrap(),
            serde_json::to_value(SynthwaveDisplayConfig::default()).unwrap()
        );
    }

    #[test]
    fn group_titles_are_padded_to_the_group() {
        let mut frame = Map::new();
        assert_eq!(group_title_of(&frame, 2)["enabled"], Value::Bool(false));
        let mut title = group_title_of(&frame, 2);
        title["enabled"] = Value::Bool(true);
        title["text"] = Value::String("Disks".to_string());
        set_group_title_in(&mut frame, 2, title);
        let titles = transferable_from_frame(&frame).group_titles;
        assert_eq!(titles.len(), 3);
        assert!(!titles[0].enabled && !titles[1].enabled);
        assert!(titles[2].enabled);
        assert_eq!(titles[2].text, "Disks");
        // The transfer carries the titles to another frame type.
        let mut target = Map::new();
        apply_transferable_to_frame(&mut target, &transferable_from_frame(&frame));
        assert_eq!(
            group_title_of(&target, 2)["text"],
            Value::String("Disks".to_string())
        );
    }

    #[test]
    fn group_orientations_are_padded_with_the_layout() {
        let mut frame = Map::new();
        frame.insert(
            "split_orientation".to_string(),
            Value::String("horizontal".to_string()),
        );
        assert_eq!(group_orientation_of(&frame, 1), "horizontal");
        set_group_orientation_in(&mut frame, 1, "vertical");
        assert_eq!(group_orientation_of(&frame, 0), "horizontal");
        assert_eq!(group_orientation_of(&frame, 1), "vertical");
        // Beyond the list the group layout still decides
        assert_eq!(group_orientation_of(&frame, 2), "horizontal");
        let list = transferable_from_frame(&frame).group_item_orientations;
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn transferable_follows_the_orientation_spelling() {
        for key in ["split_orientation", "layout_orientation"] {
            let mut frame = Map::new();
            frame.insert(key.to_string(), serde_json::json!("horizontal"));
            frame.insert("group_count".to_string(), serde_json::json!(2));
            let t = transferable_from_frame(&frame);
            assert_eq!(t.layout_orientation, SplitOrientation::Horizontal);
            assert_eq!(t.group_count, 2);
            let mut target = Map::new();
            target.insert(key.to_string(), serde_json::json!("vertical"));
            apply_transferable_to_frame(&mut target, &t);
            assert_eq!(target[key], serde_json::json!("horizontal"));
            assert!(!target.contains_key(if key == "split_orientation" {
                "layout_orientation"
            } else {
                "split_orientation"
            }));
        }
    }
}
