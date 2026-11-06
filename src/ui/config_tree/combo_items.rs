//! The per-item page of a schema-driven combo editor, over JSON: the item's
//! display type, height, and the seven per-type schema editors
//! (`content_items[slot]` of the frame model). Replaces the typed
//! `create_content_item_config` for frames that have no widget.

use super::schema_editor::{Model, SchemaEditor};
use crate::core::FieldMetadata;
use crate::ui::editor_registry::ChangeCallback;
use crate::ui::theme::ComboThemeConfig;
use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, CheckButton, DropDown, Frame, Label, Orientation, SpinButton, StringList, Widget,
};
use rg_sens_types::display_configs::{
    ArcDisplayConfig, BarDisplayConfig, ContentItemConfig, CoreBarsConfig, GraphDisplayConfig,
    SpeedometerConfig, StaticDisplayConfig,
};
use rg_sens_types::schema::{Prop, Schema};
use rg_sens_types::text::TextOverlayConfig;
use serde_json::{Map, Value};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// `ContentDisplayType` serde names, in the dropdown's order.
const TYPES: [(&str, &str); 8] = [
    ("bar", "Bar"),
    ("level_bar", "Level Bar"),
    ("text", "Text"),
    ("graph", "Graph"),
    ("core_bars", "Core Bars"),
    ("static", "Static"),
    ("arc", "Arc"),
    ("speedometer", "Speedometer"),
];

struct Sub {
    title: &'static str,
    path: &'static [&'static str],
    shown_for: &'static [&'static str],
    props: fn() -> Vec<Prop>,
}

const SUBS: [Sub; 7] = [
    Sub {
        title: "Bar Configuration",
        path: &["bar_config"],
        shown_for: &["bar", "level_bar"],
        props: BarDisplayConfig::props,
    },
    Sub {
        title: "Text Overlay",
        path: &["bar_config", "text_overlay"],
        shown_for: &["text"],
        props: TextOverlayConfig::props,
    },
    Sub {
        title: "Graph Configuration",
        path: &["graph_config"],
        shown_for: &["graph"],
        props: GraphDisplayConfig::props,
    },
    Sub {
        title: "Core Bars Configuration",
        path: &["core_bars_config"],
        shown_for: &["core_bars"],
        props: CoreBarsConfig::props,
    },
    Sub {
        title: "Arc Configuration",
        path: &["arc_config"],
        shown_for: &["arc"],
        props: ArcDisplayConfig::props,
    },
    Sub {
        title: "Speedometer Configuration",
        path: &["speedometer_config"],
        shown_for: &["speedometer"],
        props: SpeedometerConfig::props,
    },
    Sub {
        title: "Static Display Configuration",
        path: &["static_config"],
        shown_for: &["static"],
        props: StaticDisplayConfig::props,
    },
];

/// The slot's item object, or a default item when the slot has none yet.
fn item_of(model: &Model, slot: &str) -> Map<String, Value> {
    model
        .borrow()
        .get("content_items")
        .and_then(|v| v.get(slot))
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_else(|| {
            serde_json::to_value(ContentItemConfig::default())
                .ok()
                .and_then(|v| v.as_object().cloned())
                .unwrap_or_default()
        })
}

fn write_item(model: &Model, slot: &str, item: Map<String, Value>) {
    let mut m = model.borrow_mut();
    let items = m
        .entry("content_items".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    if let Some(obj) = items.as_object_mut() {
        obj.insert(slot.to_string(), Value::Object(item));
    }
}

fn object_at(item: &Map<String, Value>, path: &[&str]) -> Map<String, Value> {
    let mut cur: Option<&Value> = item.get(path[0]);
    for seg in &path[1..] {
        cur = cur.and_then(|v| v.get(seg));
    }
    cur.and_then(|v| v.as_object()).cloned().unwrap_or_default()
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

/// Build the item page for `slot` over the frame `model`.
pub fn create_item_page(
    model: Model,
    slot: &str,
    fields: Vec<FieldMetadata>,
    theme: Rc<RefCell<ComboThemeConfig>>,
    on_change: ChangeCallback,
    theme_refreshers: &Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
    cleanup: &Rc<RefCell<Vec<Box<dyn Fn()>>>>,
) -> Widget {
    let page = GtkBox::new(Orientation::Vertical, 8);
    let slot = slot.to_string();
    let loading = Rc::new(Cell::new(true));
    let item = item_of(&model, &slot);
    let current_type = item
        .get("display_as")
        .and_then(|v| v.as_str())
        .unwrap_or("bar")
        .to_string();

    // Display type
    let labels: Vec<&str> = TYPES.iter().map(|(_, l)| *l).collect();
    let type_row = GtkBox::new(Orientation::Horizontal, 8);
    let type_label = Label::new(Some("Display as:"));
    type_label.set_width_chars(18);
    type_label.set_xalign(0.0);
    let type_dropdown = DropDown::new(Some(StringList::new(&labels)), None::<gtk4::Expression>);
    type_dropdown.set_hexpand(true);
    type_dropdown.set_selected(
        TYPES
            .iter()
            .position(|(v, _)| *v == current_type)
            .unwrap_or(0) as u32,
    );
    type_row.append(&type_label);
    type_row.append(&type_dropdown);
    page.append(&type_row);

    // Height
    let auto_height = CheckButton::with_label("Auto-adjust height");
    auto_height.set_active(
        item.get("auto_height")
            .and_then(|v| v.as_bool())
            .unwrap_or(true),
    );
    page.append(&auto_height);
    let height_row = GtkBox::new(Orientation::Horizontal, 8);
    let height_label = Label::new(Some("Item height (px):"));
    height_label.set_width_chars(18);
    height_label.set_xalign(0.0);
    let height_spin = SpinButton::with_range(20.0, 300.0, 5.0);
    height_spin.set_hexpand(true);
    height_spin.set_value(
        item.get("item_height")
            .and_then(|v| v.as_f64())
            .unwrap_or(60.0),
    );
    height_spin.set_sensitive(!auto_height.is_active());
    height_row.append(&height_label);
    height_row.append(&height_spin);
    page.append(&height_row);

    // Per-type editors
    let editors: Rc<Vec<(Rc<SchemaEditor>, &'static Sub, Frame)>> = Rc::new(
        SUBS.iter()
            .map(|sub| {
                let editor = Rc::new(SchemaEditor::with_fields((sub.props)(), fields.clone()));
                editor.set_theme(&theme.borrow());
                editor.load(object_at(&item, sub.path));
                let frame = Frame::new(Some(sub.title));
                frame.set_margin_top(12);
                frame.set_child(Some(editor.widget()));
                frame.set_visible(sub.shown_for.contains(&current_type.as_str()));
                page.append(&frame);
                (editor, sub, frame)
            })
            .collect(),
    );
    for (editor, sub, _) in editors.iter() {
        let model = model.clone();
        let slot = slot.clone();
        let on_change = on_change.clone();
        let e = editor.clone();
        let path = sub.path;
        editor.set_on_change(Rc::new(move || {
            let mut item = item_of(&model, &slot);
            set_object_at(&mut item, path, e.model());
            write_item(&model, &slot, item);
            on_change();
        }));
    }

    {
        let model = model.clone();
        let slot = slot.clone();
        let on_change = on_change.clone();
        let editors = editors.clone();
        let loading = loading.clone();
        type_dropdown.connect_selected_notify(move |d| {
            if loading.get() {
                return;
            }
            let Some((tag, _)) = TYPES.get(d.selected() as usize) else {
                return;
            };
            let mut item = item_of(&model, &slot);
            item.insert("display_as".to_string(), Value::String(tag.to_string()));
            write_item(&model, &slot, item.clone());
            // Editors of the hidden kinds hold stale copies of shared
            // sub-configs (bar and text both edit bar_config): reload all.
            for (editor, sub, frame) in editors.iter() {
                editor.load(object_at(&item, sub.path));
                frame.set_visible(sub.shown_for.contains(tag));
            }
            on_change();
        });
    }
    {
        let model = model.clone();
        let slot = slot.clone();
        let on_change = on_change.clone();
        let height_spin = height_spin.clone();
        let loading = loading.clone();
        auto_height.connect_toggled(move |c| {
            height_spin.set_sensitive(!c.is_active());
            if loading.get() {
                return;
            }
            let mut item = item_of(&model, &slot);
            item.insert("auto_height".to_string(), Value::Bool(c.is_active()));
            write_item(&model, &slot, item);
            on_change();
        });
    }
    {
        let model = model.clone();
        let slot = slot.clone();
        let on_change = on_change.clone();
        let loading = loading.clone();
        height_spin.connect_value_changed(move |sp| {
            if loading.get() {
                return;
            }
            let mut item = item_of(&model, &slot);
            item.insert("item_height".to_string(), Value::from(sp.value()));
            write_item(&model, &slot, item);
            on_change();
        });
    }

    {
        let editors = editors.clone();
        let theme = theme.clone();
        theme_refreshers.borrow_mut().push(Rc::new(move || {
            let t = theme.borrow().clone();
            for (editor, _, _) in editors.iter() {
                editor.set_theme(&t);
            }
        }));
    }
    {
        let editors = editors.clone();
        cleanup.borrow_mut().push(Box::new(move || {
            for (editor, _, _) in editors.iter() {
                editor.cleanup();
            }
        }));
    }
    loading.set(false);
    page.upcast()
}
