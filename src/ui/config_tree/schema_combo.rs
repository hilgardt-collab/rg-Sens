//! A schema-driven editor for the themed combo frames: rows from the frame
//! config's `Schema` (sections as tree nodes), the combo surface
//! (`ComboDisplayEditor`) over the same JSON model, and item pages from
//! `combo_items`. The frame's cached `theme`, `content_items` and group
//! caches have no props and pass through untouched.
//!
//! The displayer config is a wrapper `{ frame, animation_enabled,
//! animation_speed }`; the editor works on the frame object with the two
//! animation keys copied in (what `to_frame`/`from_frame` do).

use super::combo_items::create_item_page;
use super::schema_editor::SchemaEditor;
use crate::core::{DisplayerConfig, FieldMetadata};
use crate::ui::editor_registry::{
    ChangeCallback, ComboDisplayEditor, ComboReorder, DisplayEditor, EditorPage,
};
use crate::ui::theme::ComboThemeConfig;
use gtk4::prelude::*;
use gtk4::Widget;
use rg_sens_types::combo::TransferableComboConfig;
use rg_sens_types::schema::Prop;
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
        fields: &[FieldMetadata],
        on_change: ChangeCallback,
        theme_refreshers: &Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
        cleanup: &Rc<RefCell<Vec<Box<dyn Fn()>>>>,
    ) -> Widget {
        create_item_page(
            self.editor.model_rc(),
            slot,
            fields.to_vec(),
            self.theme.clone(),
            on_change,
            theme_refreshers,
            cleanup,
        )
    }

    fn refresh_theme_refs(&self) {
        // `set_theme` reaches every row and the item pages' refreshers run
        // from the dialog.
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
