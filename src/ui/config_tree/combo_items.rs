//! The per-item display editor of a schema-driven combo editor, over JSON:
//! the item's display type and height on a head widget shown on the item
//! page, and the selected type's schema editor (`content_items[slot]` of the
//! frame model), whose pages the tree shows as child nodes of the item.
//! [`item_tree_pages`] derives those pages from the config alone, so the tree
//! lists every item's nodes before any item editor exists (a 29-item panel
//! must open as fast as with no item editors at all).

use super::schema_editor::{Model, SchemaEditor};
use crate::core::FieldMetadata;
use crate::ui::editor_registry::{ChangeCallback, EditorPage};
use crate::ui::theme::ComboThemeConfig;
use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, CheckButton, DropDown, Label, Orientation, SpinButton, StringList, Widget,
};
use once_cell::sync::Lazy;
use rg_sens_types::display_configs::{
    AnalogClockConfig, ArcDisplayConfig, BarDisplayConfig, ContentDisplayType, ContentItemConfig,
    CoreBarsConfig, GraphDisplayConfig, SpeedometerConfig, StaticDisplayConfig,
};
use rg_sens_types::schema::{is_sectioned, tree_pages, Prop, Schema, TreePage};
use rg_sens_types::text::TextOverlayConfig;
use serde_json::{Map, Value};
use std::cell::RefCell;
use std::rc::Rc;

/// `ContentDisplayType` serde names and labels, in the dropdown's order, for
/// an item whose source is a metric.
const METRIC_TYPES: [(&str, &str); 8] = [
    ("bar", "Bar"),
    ("level_bar", "Level Bar"),
    ("text", "Text"),
    ("graph", "Graph"),
    ("core_bars", "Core Bars"),
    ("static", "Static"),
    ("arc", "Arc"),
    ("speedometer", "Speedometer"),
];

/// For an item whose source is the clock: its time as text, or a clock face.
const CLOCK_TYPES: [(&str, &str); 2] = [("text", "Text"), ("clock_analog", "Analog")];

/// The display types offered for an item, by its source: a clock is shown as
/// text or as an analog face, every other source gets the metric types.
pub fn types_for_source(source_id: &str) -> &'static [(&'static str, &'static str)] {
    match source_id {
        "clock" => &CLOCK_TYPES,
        _ => &METRIC_TYPES,
    }
}

/// The display type `source_id` forces on an item showing as `current`:
/// `None` when the source offers `current`, else the first type it offers
/// (text for a clock, bar for a metric).
fn coercion_for(current: &str, source_id: &str) -> Option<&'static str> {
    let offered = types_for_source(source_id);
    if offered.iter().any(|(tag, _)| *tag == current) {
        None
    } else {
        offered.first().map(|(tag, _)| *tag)
    }
}

/// The dropdown label of a display type ("Level Bar", "Analog").
pub fn display_type_label(display_as: ContentDisplayType) -> String {
    let tag = serde_json::to_value(display_as)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default();
    METRIC_TYPES
        .iter()
        .chain(CLOCK_TYPES.iter())
        .find(|(t, _)| *t == tag)
        .map(|(_, label)| label.to_string())
        .unwrap_or(tag)
}

/// The sub-config one display type edits, and where it lives in the item.
struct Sub {
    path: &'static [&'static str],
    shown_for: &'static [&'static str],
    props: fn() -> Vec<Prop>,
}

const SUBS: [Sub; 8] = [
    Sub {
        path: &["bar_config"],
        shown_for: &["bar", "level_bar"],
        props: BarDisplayConfig::props,
    },
    Sub {
        path: &["bar_config", "text_overlay"],
        shown_for: &["text"],
        props: TextOverlayConfig::props,
    },
    Sub {
        path: &["graph_config"],
        shown_for: &["graph"],
        props: GraphDisplayConfig::props,
    },
    Sub {
        path: &["core_bars_config"],
        shown_for: &["core_bars"],
        props: CoreBarsConfig::props,
    },
    Sub {
        path: &["arc_config"],
        shown_for: &["arc"],
        props: ArcDisplayConfig::props,
    },
    Sub {
        path: &["speedometer_config"],
        shown_for: &["speedometer"],
        props: SpeedometerConfig::props,
    },
    Sub {
        path: &["static_config"],
        shown_for: &["static"],
        props: StaticDisplayConfig::props,
    },
    Sub {
        path: &["clock_analog_config"],
        shown_for: &["clock_analog"],
        props: clock_item_props,
    },
];

/// The analog clock's rows without its Icon section: the alarm/timer icon is
/// drawn by the standalone displayer (which also owns the click that opens
/// the alarm dialog), not by the face renderer a combo item uses, so its
/// controls would do nothing here.
fn clock_item_props() -> Vec<Prop> {
    AnalogClockConfig::props()
        .into_iter()
        .filter(|p| p.section != Some("Icon"))
        .collect()
}

/// The item a slot has before it is configured.
static DEFAULT_ITEM: Lazy<Value> = Lazy::new(|| {
    serde_json::to_value(ContentItemConfig::default()).unwrap_or_else(|_| Value::Object(Map::new()))
});

fn sub_for(display_as: &str) -> Option<&'static Sub> {
    SUBS.iter().find(|s| s.shown_for.contains(&display_as))
}

/// The slot's item object in the frame, or the default item.
fn item_ref<'a>(frame: &'a Map<String, Value>, slot: &str) -> &'a Value {
    frame
        .get("content_items")
        .and_then(|v| v.get(slot))
        .filter(|v| v.is_object())
        .unwrap_or(&DEFAULT_ITEM)
}

fn display_as_of(item: &Value) -> &str {
    item.get("display_as")
        .and_then(Value::as_str)
        .unwrap_or("bar")
}

fn value_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    path.iter().try_fold(value, |cur, seg| cur.get(seg))
}

/// The pages the slot's display editor has (or would have), from the frame
/// alone: the selected type's sections and list items, as the tree files
/// them beneath the item. Mirrors what [`ItemDisplayEditor::pages`] reports
/// once the editor exists.
pub fn item_tree_pages(frame: &Map<String, Value>, slot: &str) -> Vec<TreePage> {
    let item = item_ref(frame, slot);
    let Some(sub) = sub_for(display_as_of(item)) else {
        return Vec::new();
    };
    let props = (sub.props)();
    let empty = Value::Object(Map::new());
    let value = value_at(item, sub.path).unwrap_or(&empty);
    tree_pages(&props, value, is_sectioned(&props))
}

/// Apply `f` to the slot's item object, creating it from the default first.
fn with_item_mut(model: &Model, slot: &str, f: impl FnOnce(&mut Map<String, Value>)) {
    let mut m = model.borrow_mut();
    let items = m
        .entry("content_items".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    if !items.is_object() {
        *items = Value::Object(Map::new());
    }
    let Some(items) = items.as_object_mut() else {
        return;
    };
    let item = items
        .entry(slot.to_string())
        .or_insert_with(|| DEFAULT_ITEM.clone());
    if !item.is_object() {
        *item = DEFAULT_ITEM.clone();
    }
    if let Some(obj) = item.as_object_mut() {
        f(obj);
    }
}

/// Make the slot's item use a display type its source offers (see
/// [`types_for_source`]); returns whether the frame changed. Silent.
pub fn coerce_display_as(model: &Model, slot: &str, source_id: &str) -> bool {
    let current = {
        let m = model.borrow();
        display_as_of(item_ref(&m, slot)).to_string()
    };
    let Some(tag) = coercion_for(&current, source_id) else {
        return false;
    };
    with_item_mut(model, slot, |item| {
        item.insert("display_as".to_string(), Value::String(tag.to_string()));
    });
    true
}

fn set_object_at(item: &mut Map<String, Value>, path: &[&str], value: Map<String, Value>) {
    let mut cur: &mut Map<String, Value> = item;
    for seg in &path[..path.len() - 1] {
        let next = cur
            .entry(seg.to_string())
            .or_insert_with(|| Value::Object(Map::new()));
        if !next.is_object() {
            *next = Value::Object(Map::new());
        }
        cur = next.as_object_mut().expect("just made an object");
    }
    cur.insert(path[path.len() - 1].to_string(), Value::Object(value));
}

fn labelled(label: &str, control: &impl IsA<Widget>) -> GtkBox {
    let row = GtkBox::new(Orientation::Horizontal, 8);
    let l = Label::new(Some(label));
    l.set_width_chars(18);
    l.set_xalign(0.0);
    control.set_hexpand(true);
    row.append(&l);
    row.append(control);
    row
}

/// One content item's display editor over the frame model.
pub struct ItemDisplayEditor {
    model: Model,
    slot: String,
    fields: Vec<FieldMetadata>,
    /// The resolved theme, shared with the frame editor.
    theme: Rc<RefCell<ComboThemeConfig>>,
    on_change: ChangeCallback,
    head: GtkBox,
    /// Rows of a type with a single section (text) sit here, on the item
    /// page; its list items are still child nodes.
    inline: GtkBox,
    /// Shown instead when the type's sections are child nodes.
    sections_hint: Label,
    /// The schema editor of the current display type.
    sub: RefCell<Option<Rc<SchemaEditor>>>,
    on_pages_changed: RefCell<Option<Rc<dyn Fn(Option<Vec<String>>)>>>,
}

impl ItemDisplayEditor {
    pub(super) fn new(
        model: Model,
        slot: &str,
        source_id: &str,
        fields: Vec<FieldMetadata>,
        theme: Rc<RefCell<ComboThemeConfig>>,
        on_change: ChangeCallback,
    ) -> Rc<Self> {
        // The dropdown must show the stored type, and the stored type must be
        // one the source offers (the dialog coerces on load and on a source
        // switch; this is the safety net for any other path).
        let offered = types_for_source(source_id);
        let coerced = coerce_display_as(&model, slot, source_id);
        let (display_as, auto_height, item_height) = {
            let m = model.borrow();
            let item = item_ref(&m, slot);
            (
                display_as_of(item).to_string(),
                item.get("auto_height")
                    .and_then(Value::as_bool)
                    .unwrap_or(true),
                item.get("item_height")
                    .and_then(Value::as_f64)
                    .unwrap_or(60.0),
            )
        };

        let head = GtkBox::new(Orientation::Vertical, 8);
        let labels: Vec<&str> = offered.iter().map(|(_, l)| *l).collect();
        let type_dropdown = DropDown::new(Some(StringList::new(&labels)), None::<gtk4::Expression>);
        type_dropdown.set_selected(
            offered
                .iter()
                .position(|(v, _)| *v == display_as)
                .unwrap_or(0) as u32,
        );
        head.append(&labelled("Display as:", &type_dropdown));

        let auto_height_check = CheckButton::with_label("Auto-adjust height");
        auto_height_check.set_active(auto_height);
        head.append(&auto_height_check);
        let height_spin = SpinButton::with_range(20.0, 300.0, 5.0);
        height_spin.set_value(item_height);
        height_spin.set_sensitive(!auto_height);
        head.append(&labelled("Item height (px):", &height_spin));

        let sections_hint = Label::new(Some(
            "The display settings of this item are grouped under its child nodes.",
        ));
        sections_hint.add_css_class("dim-label");
        sections_hint.set_halign(gtk4::Align::Start);
        sections_hint.set_wrap(true);
        sections_hint.set_xalign(0.0);
        sections_hint.set_visible(false);
        head.append(&sections_hint);
        let inline = GtkBox::new(Orientation::Vertical, 6);
        head.append(&inline);

        let this = Rc::new(Self {
            model,
            slot: slot.to_string(),
            fields,
            theme,
            on_change,
            head,
            inline,
            sections_hint,
            sub: RefCell::new(None),
            on_pages_changed: RefCell::new(None),
        });
        this.build_sub();
        if coerced {
            (this.on_change)();
        }

        {
            let weak = Rc::downgrade(&this);
            type_dropdown.connect_selected_notify(move |d| {
                let Some(this) = weak.upgrade() else { return };
                if let Some((tag, _)) = offered.get(d.selected() as usize) {
                    this.set_display_as(tag);
                }
            });
        }
        {
            let weak = Rc::downgrade(&this);
            let height_spin = height_spin.clone();
            auto_height_check.connect_toggled(move |c| {
                height_spin.set_sensitive(!c.is_active());
                let Some(this) = weak.upgrade() else { return };
                let active = c.is_active();
                with_item_mut(&this.model, &this.slot, |item| {
                    item.insert("auto_height".to_string(), Value::Bool(active));
                });
                (this.on_change)();
            });
        }
        {
            let weak = Rc::downgrade(&this);
            height_spin.connect_value_changed(move |sp| {
                let Some(this) = weak.upgrade() else { return };
                let value = sp.value();
                with_item_mut(&this.model, &this.slot, |item| {
                    item.insert("item_height".to_string(), Value::from(value));
                });
                (this.on_change)();
            });
        }
        this
    }

    /// The head: display type and height, plus the rows of a single-section
    /// type.
    pub fn widget(&self) -> Widget {
        self.head.clone().upcast()
    }

    /// The current type's pages: sections first, then list items (the same
    /// paths [`item_tree_pages`] derives).
    pub fn pages(&self) -> Vec<EditorPage> {
        let Some(sub) = self.sub.borrow().clone() else {
            return Vec::new();
        };
        sub.pages()
            .into_iter()
            .map(|(name, page)| EditorPage {
                path: vec![name],
                widget: page.upcast(),
            })
            .chain(sub.item_pages().into_iter().map(|(path, page)| EditorPage {
                path,
                widget: page.upcast(),
            }))
            .collect()
    }

    pub fn page_widget(&self, path: &[String]) -> Option<Widget> {
        self.pages()
            .into_iter()
            .find(|p| p.path == path)
            .map(|p| p.widget)
    }

    /// Called with the path to select (or `None` to keep the selection)
    /// whenever `pages()` would now return a different set.
    pub fn set_on_pages_changed(&self, cb: Rc<dyn Fn(Option<Vec<String>>)>) {
        *self.on_pages_changed.borrow_mut() = Some(cb);
    }

    /// Push the resolved theme to the current type's rows (silent).
    pub fn set_theme(&self, theme: &ComboThemeConfig) {
        if let Some(sub) = self.sub.borrow().as_ref() {
            sub.set_theme(theme);
        }
    }

    /// Break the control ↔ closure cycles; safe to call twice.
    pub fn cleanup(&self) {
        *self.on_pages_changed.borrow_mut() = None;
        if let Some(sub) = self.sub.borrow_mut().take() {
            sub.cleanup();
        }
        while let Some(child) = self.inline.first_child() {
            self.inline.remove(&child);
        }
    }

    fn notify_pages(&self, select: Option<Vec<String>>) {
        let cb = self.on_pages_changed.borrow().clone();
        if let Some(cb) = cb {
            cb(select);
        }
    }

    /// Build the schema editor of the item's current display type from the
    /// model, dropping the previous one (bar and text both edit `bar_config`,
    /// so a kept editor would hold a stale copy).
    fn build_sub(self: &Rc<Self>) {
        if let Some(old) = self.sub.borrow_mut().take() {
            old.cleanup();
        }
        while let Some(child) = self.inline.first_child() {
            self.inline.remove(&child);
        }
        let (sub, initial) = {
            let m = self.model.borrow();
            let item = item_ref(&m, &self.slot);
            let Some(sub) = sub_for(display_as_of(item)) else {
                self.sections_hint.set_visible(false);
                return;
            };
            let initial = value_at(item, sub.path)
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            (sub, initial)
        };
        let props = (sub.props)();
        let sectioned = is_sectioned(&props);
        let editor = Rc::new(SchemaEditor::for_tree(
            props,
            self.fields.clone(),
            sectioned,
        ));
        editor.set_theme(&self.theme.borrow());
        editor.load(initial);
        {
            let weak = Rc::downgrade(self);
            let e = editor.clone();
            let path = sub.path;
            editor.set_on_change(Rc::new(move || {
                let Some(this) = weak.upgrade() else { return };
                let value = e.model();
                with_item_mut(&this.model, &this.slot, |item| {
                    set_object_at(item, path, value)
                });
                (this.on_change)();
            }));
        }
        {
            let weak = Rc::downgrade(self);
            editor.set_on_item_pages_changed(Rc::new(move |select| {
                if let Some(this) = weak.upgrade() {
                    this.notify_pages(select);
                }
            }));
        }
        self.sections_hint.set_visible(sectioned);
        if !sectioned {
            self.inline.append(editor.widget());
        }
        *self.sub.borrow_mut() = Some(editor);
    }

    fn set_display_as(self: &Rc<Self>, tag: &str) {
        let same = {
            let m = self.model.borrow();
            display_as_of(item_ref(&m, &self.slot)) == tag
        };
        if same {
            return;
        }
        let tag = tag.to_string();
        with_item_mut(&self.model, &self.slot, |item| {
            item.insert("display_as".to_string(), Value::String(tag));
        });
        self.build_sub();
        (self.on_change)();
        self.notify_pages(None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame_with(slot: &str, item: Value) -> Map<String, Value> {
        let mut items = Map::new();
        items.insert(slot.to_string(), item);
        let mut frame = Map::new();
        frame.insert("content_items".to_string(), Value::Object(items));
        frame
    }

    #[test]
    fn an_unconfigured_slot_has_the_default_bar_pages() {
        let pages = item_tree_pages(&Map::new(), "group1_1");
        let sections: Vec<&str> = pages
            .iter()
            .filter(|p| p.path.len() == 1)
            .map(|p| p.path[0].as_str())
            .collect();
        assert_eq!(sections[0], "Style");
        assert!(sections.contains(&"Text overlay"));
        assert!(pages.iter().any(|p| p
            .path
            .starts_with(&["Text overlay".to_string(), "lines".to_string()])));
    }

    #[test]
    fn a_text_item_files_its_lines_under_the_item() {
        let mut item = DEFAULT_ITEM.clone();
        item["display_as"] = Value::String("text".to_string());
        let frame = frame_with("group2_3", item);
        let pages = item_tree_pages(&frame, "group2_3");
        assert!(pages.iter().all(|p| p.path.len() == 2));
        assert_eq!(pages[0].path, vec!["lines".to_string(), "0".to_string()]);
        assert!(pages[0].title.starts_with("Line 1"));
    }

    #[test]
    fn a_type_without_a_sub_editor_has_no_pages() {
        let mut item = DEFAULT_ITEM.clone();
        item["display_as"] = Value::String("unknown".to_string());
        assert!(item_tree_pages(&frame_with("group1_1", item), "group1_1").is_empty());
    }

    #[test]
    fn with_item_mut_creates_the_item_from_the_default() {
        let model: Model = Rc::new(RefCell::new(Map::new()));
        with_item_mut(&model, "group1_2", |item| {
            item.insert("display_as".to_string(), Value::String("arc".to_string()));
        });
        let m = model.borrow();
        let item = item_ref(&m, "group1_2");
        assert_eq!(display_as_of(item), "arc");
        assert!(item.get("bar_config").is_some(), "default keys kept");
    }

    #[test]
    fn a_clock_source_offers_text_and_an_analog_face() {
        let tags: Vec<&str> = types_for_source("clock").iter().map(|(t, _)| *t).collect();
        assert_eq!(tags, vec!["text", "clock_analog"]);
        let metric: Vec<&str> = types_for_source("cpu").iter().map(|(t, _)| *t).collect();
        assert!(metric.contains(&"bar") && !metric.contains(&"clock_analog"));
    }

    #[test]
    fn coercion_follows_the_source() {
        let model: Model = Rc::new(RefCell::new(Map::new()));
        // A fresh (bar) item on a clock slot becomes text.
        assert!(coerce_display_as(&model, "group1_1", "clock"));
        assert_eq!(display_as_of(item_ref(&model.borrow(), "group1_1")), "text");
        // Text is offered: nothing to do.
        assert!(!coerce_display_as(&model, "group1_1", "clock"));
        // A face on a metric slot falls back to bar.
        with_item_mut(&model, "group1_1", |item| {
            item.insert(
                "display_as".to_string(),
                Value::String("clock_analog".to_string()),
            );
        });
        assert!(coerce_display_as(&model, "group1_1", "cpu"));
        assert_eq!(display_as_of(item_ref(&model.borrow(), "group1_1")), "bar");
    }

    #[test]
    fn a_clock_face_files_its_sections_without_the_icon() {
        let mut item = DEFAULT_ITEM.clone();
        item["display_as"] = Value::String("clock_analog".to_string());
        let pages = item_tree_pages(&frame_with("group1_1", item), "group1_1");
        let sections: Vec<&str> = pages
            .iter()
            .filter(|p| p.path.len() == 1)
            .map(|p| p.path[0].as_str())
            .collect();
        assert_eq!(sections, vec!["Face", "Numbers", "Ticks", "Hands"]);
    }

    #[test]
    fn labels_follow_the_dropdown() {
        assert_eq!(
            display_type_label(ContentDisplayType::ClockAnalog),
            "Analog"
        );
        assert_eq!(
            display_type_label(ContentDisplayType::LevelBar),
            "Level Bar"
        );
    }
}
