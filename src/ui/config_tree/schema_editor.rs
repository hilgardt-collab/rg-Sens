//! Schema-driven editor (Phase 3): rows rendered from a config type's
//! [`Prop`]s over the flat JSON form of its config. A source type whose
//! config implements `Schema` is registered through [`SchemaSourceEditor`]
//! and needs no hand-written widget; the same editor serves the standalone
//! Source node and combo item pages, because both come from the registry.
//!
//! The editor edits the whole flat object: keys without a prop are carried
//! through untouched, so adding a prop later never loses saved data.

use super::util::{flatten_source_config, source_config_from_flat};
use crate::core::SourceConfig;
use crate::ui::editor_registry::{ChangeCallback, SourceEditor};
use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, CheckButton, DropDown, Entry, Frame, Label, Orientation, SpinButton,
    StringList, Widget,
};
use rg_sens_types::schema::{Condition, Prop, PropKind};
use serde_json::{Map, Value};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

type Provider = Rc<dyn Fn() -> Vec<(String, String)>>;
type Model = Rc<RefCell<Map<String, Value>>>;
/// Sets a control from the flat config (load / rebuild).
pub type FieldRefresh = Box<dyn Fn(&Map<String, Value>)>;
type FieldEditorFactory = Rc<dyn Fn(FieldContext) -> (Widget, FieldRefresh)>;

thread_local! {
    static PROVIDERS: RefCell<HashMap<&'static str, Provider>> = RefCell::new(HashMap::new());
    static FIELD_EDITORS: RefCell<HashMap<&'static str, FieldEditorFactory>> =
        RefCell::new(HashMap::new());
}

/// What a `PropKind::Custom` editor is handed: its prop and a way to write
/// the flat config. Writes while the editor is loading are silent
/// (normalisation of a saved value); otherwise they fire the change signal.
#[derive(Clone)]
pub struct FieldContext {
    pub prop: Prop,
    set: Rc<dyn Fn(Vec<(&'static str, Value)>)>,
}

impl FieldContext {
    pub fn set(&self, values: Vec<(&'static str, Value)>) {
        (self.set)(values);
    }
}

/// Register the editor a `PropKind::Custom { editor }` names. The factory
/// returns the control and a refresh closure that reads the whole flat
/// config (an editor may span several keys).
pub fn register_field_editor(
    name: &'static str,
    f: impl Fn(FieldContext) -> (Widget, FieldRefresh) + 'static,
) {
    FIELD_EDITORS.with(|e| {
        e.borrow_mut().insert(name, Rc::new(f));
    });
}

/// Register the `(value, label)` list a `PropKind::DynamicChoice { provider }`
/// offers.
pub fn register_choice_provider(
    name: &'static str,
    f: impl Fn() -> Vec<(String, String)> + 'static,
) {
    PROVIDERS.with(|p| {
        p.borrow_mut().insert(name, Rc::new(f));
    });
}

fn provider_choices(provider: &str) -> Vec<(String, String)> {
    PROVIDERS.with(|p| p.borrow().get(provider).map(|f| f()).unwrap_or_default())
}

fn condition_holds(cond: &Condition, model: &Map<String, Value>) -> bool {
    match cond {
        Condition::Equals { key, values } => model
            .get(*key)
            .and_then(|v| v.as_str())
            .map(|s| values.contains(&s))
            .unwrap_or(false),
        Condition::IsFalse { key } => !model.get(*key).and_then(|v| v.as_bool()).unwrap_or(false),
    }
}

struct Row {
    prop: Prop,
    widget: GtkBox,
    /// Set the control from the model (load).
    refresh: FieldRefresh,
}

/// Rows for a list of props over one JSON object.
pub struct SchemaEditor {
    container: GtkBox,
    model: Model,
    rows: Rc<RefCell<Vec<Row>>>,
    loading: Rc<Cell<bool>>,
    on_change: Rc<RefCell<Option<ChangeCallback>>>,
}

impl SchemaEditor {
    pub fn new(props: Vec<Prop>) -> Self {
        let container = GtkBox::new(Orientation::Vertical, 6);
        let model: Model = Rc::new(RefCell::new(Map::new()));
        let rows: Rc<RefCell<Vec<Row>>> = Rc::new(RefCell::new(Vec::new()));
        let loading = Rc::new(Cell::new(false));
        let on_change: Rc<RefCell<Option<ChangeCallback>>> = Rc::new(RefCell::new(None));

        // One closure every control calls after writing its value.
        let fire: Rc<dyn Fn()> = {
            let loading = loading.clone();
            let on_change = on_change.clone();
            let rows = rows.clone();
            let model = model.clone();
            Rc::new(move || {
                if loading.get() {
                    return;
                }
                update_visibility(&rows.borrow(), &model.borrow());
                if let Some(cb) = on_change.borrow().as_ref() {
                    cb();
                }
            })
        };

        let mut current_section: Option<&'static str> = None;
        for prop in props {
            if prop.section != current_section {
                if let Some(s) = prop.section {
                    let heading = Label::new(Some(s));
                    heading.set_halign(gtk4::Align::Start);
                    heading.add_css_class("heading");
                    heading.set_margin_top(8);
                    container.append(&heading);
                }
                current_section = prop.section;
            }
            let (widget, refresh) = build_row(&prop, &model, &loading, &fire);
            container.append(&widget);
            rows.borrow_mut().push(Row {
                prop,
                widget,
                refresh,
            });
        }

        Self {
            container,
            model,
            rows,
            loading,
            on_change,
        }
    }

    pub fn widget(&self) -> &GtkBox {
        &self.container
    }

    /// Replace the model and refresh every control without firing changes.
    pub fn load(&self, map: Map<String, Value>) {
        self.loading.set(true);
        *self.model.borrow_mut() = map;
        // Refresh from a snapshot: setting a control emits its signal
        // synchronously, and a handler must never find the model borrowed
        // (gotcha 12). The handlers also return early while loading.
        let snapshot = self.model.borrow().clone();
        {
            let rows = self.rows.borrow();
            for row in rows.iter() {
                (row.refresh)(&snapshot);
            }
            update_visibility(&rows, &self.model.borrow());
        }
        self.loading.set(false);
    }

    pub fn model(&self) -> Map<String, Value> {
        self.model.borrow().clone()
    }

    pub fn set_on_change(&self, cb: ChangeCallback) {
        *self.on_change.borrow_mut() = Some(cb);
    }

    /// Append a widget below the rows (stateful UI that is not a property,
    /// such as the Claude sign-in block).
    pub fn append_footer(&self, footer: &impl IsA<Widget>) {
        footer.set_margin_top(8);
        self.container.append(footer);
    }

    /// Labels and keys, for the tree search box.
    pub fn search_terms(&self) -> Vec<String> {
        let mut out = Vec::new();
        collect_search_terms(self.rows.borrow().iter().map(|r| &r.prop), &mut out);
        out
    }

    /// Break the control ↔ closure cycles.
    pub fn cleanup(&self) {
        *self.on_change.borrow_mut() = None;
        self.rows.borrow_mut().clear();
    }
}

fn collect_search_terms<'a>(props: impl Iterator<Item = &'a Prop>, out: &mut Vec<String>) {
    for prop in props {
        match &prop.kind {
            PropKind::Note => {}
            PropKind::List { item, .. } => {
                out.push(prop.label.to_string());
                out.push(prop.key.replace('_', " "));
                collect_search_terms(item.iter(), out);
            }
            _ => {
                out.push(prop.label.to_string());
                out.push(prop.key.replace('_', " "));
            }
        }
    }
}

fn update_visibility(rows: &[Row], model: &Map<String, Value>) {
    for row in rows {
        let visible = row
            .prop
            .visible_when
            .as_ref()
            .map(|c| condition_holds(c, model))
            .unwrap_or(true);
        row.widget.set_visible(visible);
    }
}

fn labelled_row(prop: &Prop, control: &impl IsA<Widget>) -> GtkBox {
    let row = GtkBox::new(Orientation::Horizontal, 8);
    let label = Label::new(Some(prop.label));
    label.set_width_chars(18);
    label.set_xalign(0.0);
    if !prop.help.is_empty() {
        label.set_tooltip_text(Some(prop.help));
        control.set_tooltip_text(Some(prop.help));
    }
    control.set_hexpand(true);
    row.append(&label);
    row.append(control);
    row
}

#[allow(clippy::type_complexity)]
fn build_row(
    prop: &Prop,
    model: &Model,
    loading: &Rc<Cell<bool>>,
    fire: &Rc<dyn Fn()>,
) -> (GtkBox, FieldRefresh) {
    let key = prop.key;
    match &prop.kind {
        PropKind::Bool => {
            let check = CheckButton::new();
            let row = labelled_row(prop, &check);
            {
                let model = model.clone();
                let fire = fire.clone();
                let loading = loading.clone();
                check.connect_toggled(move |c| {
                    if loading.get() {
                        return;
                    }
                    model
                        .borrow_mut()
                        .insert(key.to_string(), Value::Bool(c.is_active()));
                    fire();
                });
            }
            let c = check.clone();
            (
                row,
                Box::new(move |m| {
                    c.set_active(m.get(key).and_then(|v| v.as_bool()).unwrap_or(false))
                }),
            )
        }
        PropKind::Int { min, max, step } => {
            let spin = SpinButton::with_range(*min as f64, *max as f64, *step as f64);
            spin.set_digits(0);
            let row = labelled_row(prop, &spin);
            {
                let model = model.clone();
                let fire = fire.clone();
                let loading = loading.clone();
                spin.connect_value_changed(move |s| {
                    if loading.get() {
                        return;
                    }
                    model
                        .borrow_mut()
                        .insert(key.to_string(), Value::from(s.value().round() as i64));
                    fire();
                });
            }
            let s = spin.clone();
            (
                row,
                Box::new(move |m| {
                    if let Some(v) = m.get(key).and_then(|v| v.as_f64()) {
                        s.set_value(v);
                    }
                }),
            )
        }
        PropKind::Float {
            min,
            max,
            step,
            digits,
        } => {
            let spin = SpinButton::with_range(*min, *max, *step);
            spin.set_digits(*digits);
            let row = labelled_row(prop, &spin);
            {
                let model = model.clone();
                let fire = fire.clone();
                let loading = loading.clone();
                spin.connect_value_changed(move |s| {
                    if loading.get() {
                        return;
                    }
                    model
                        .borrow_mut()
                        .insert(key.to_string(), Value::from(s.value()));
                    fire();
                });
            }
            let s = spin.clone();
            (
                row,
                Box::new(move |m| {
                    if let Some(v) = m.get(key).and_then(|v| v.as_f64()) {
                        s.set_value(v);
                    }
                }),
            )
        }
        PropKind::OptionalFloat {
            min,
            max,
            step,
            digits,
        } => {
            // "Set" toggle plus the value; unset stores null.
            let inner = GtkBox::new(Orientation::Horizontal, 6);
            let check = CheckButton::new();
            check.set_tooltip_text(Some("Set a value (unset leaves it automatic)"));
            let spin = SpinButton::with_range(*min, *max, *step);
            spin.set_digits(*digits);
            spin.set_hexpand(true);
            spin.set_sensitive(false);
            inner.append(&check);
            inner.append(&spin);
            let row = labelled_row(prop, &inner);
            {
                let model = model.clone();
                let fire = fire.clone();
                let loading = loading.clone();
                let spin = spin.clone();
                check.connect_toggled(move |c| {
                    spin.set_sensitive(c.is_active());
                    if loading.get() {
                        return;
                    }
                    let value = if c.is_active() {
                        Value::from(spin.value())
                    } else {
                        Value::Null
                    };
                    model.borrow_mut().insert(key.to_string(), value);
                    fire();
                });
            }
            {
                let model = model.clone();
                let fire = fire.clone();
                let loading = loading.clone();
                let check = check.clone();
                spin.connect_value_changed(move |s| {
                    if loading.get() {
                        return;
                    }
                    if check.is_active() {
                        model
                            .borrow_mut()
                            .insert(key.to_string(), Value::from(s.value()));
                        fire();
                    }
                });
            }
            let (c, s) = (check.clone(), spin.clone());
            (
                row,
                Box::new(move |m| match m.get(key).and_then(|v| v.as_f64()) {
                    Some(v) => {
                        s.set_value(v);
                        c.set_active(true);
                    }
                    None => c.set_active(false),
                }),
            )
        }
        PropKind::Text { placeholder } | PropKind::OptionalText { placeholder } => {
            let optional = matches!(prop.kind, PropKind::OptionalText { .. });
            let entry = Entry::new();
            entry.set_placeholder_text(Some(placeholder));
            let row = labelled_row(prop, &entry);
            {
                let model = model.clone();
                let fire = fire.clone();
                let loading = loading.clone();
                entry.connect_changed(move |e| {
                    if loading.get() {
                        return;
                    }
                    let text = e.text().to_string();
                    let value = if optional && text.is_empty() {
                        Value::Null
                    } else {
                        Value::String(text)
                    };
                    model.borrow_mut().insert(key.to_string(), value);
                    fire();
                });
            }
            let e = entry.clone();
            (
                row,
                Box::new(move |m| {
                    let text = m.get(key).and_then(|v| v.as_str()).unwrap_or("");
                    if e.text() != text {
                        e.set_text(text);
                    }
                }),
            )
        }
        PropKind::Enum { options } => {
            let labels: Vec<&str> = options.iter().map(|(_, l)| *l).collect();
            let values: Vec<&'static str> = options.iter().map(|(v, _)| *v).collect();
            let dropdown = DropDown::new(Some(StringList::new(&labels)), None::<gtk4::Expression>);
            let row = labelled_row(prop, &dropdown);
            {
                let model = model.clone();
                let fire = fire.clone();
                let loading = loading.clone();
                let values = values.clone();
                dropdown.connect_selected_notify(move |d| {
                    if loading.get() {
                        return;
                    }
                    if let Some(v) = values.get(d.selected() as usize) {
                        model
                            .borrow_mut()
                            .insert(key.to_string(), Value::String(v.to_string()));
                        fire();
                    }
                });
            }
            let d = dropdown.clone();
            (
                row,
                Box::new(move |m| {
                    let current = m.get(key).and_then(|v| v.as_str()).unwrap_or("");
                    let idx = values.iter().position(|v| *v == current).unwrap_or(0);
                    d.set_selected(idx as u32);
                }),
            )
        }
        PropKind::DynamicChoice { provider } => {
            // Choices come from the provider at load time; a saved value the
            // provider does not list is kept as its own row rather than
            // silently replaced (gotcha 18: a real row is needed for it).
            let provider: &'static str = provider;
            let dropdown = DropDown::new(None::<StringList>, None::<gtk4::Expression>);
            let values: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
            let row = labelled_row(prop, &dropdown);
            {
                let model = model.clone();
                let fire = fire.clone();
                let loading = loading.clone();
                let values = values.clone();
                dropdown.connect_selected_notify(move |d| {
                    if loading.get() {
                        return;
                    }
                    if let Some(v) = values.borrow().get(d.selected() as usize) {
                        model
                            .borrow_mut()
                            .insert(key.to_string(), Value::String(v.clone()));
                        fire();
                    }
                });
            }
            let d = dropdown.clone();
            (
                row,
                Box::new(move |m| {
                    let saved = m
                        .get(key)
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let (mut names, mut labels): (Vec<String>, Vec<String>) =
                        provider_choices(provider).into_iter().unzip();
                    if !saved.is_empty() && !names.contains(&saved) {
                        labels.push(format!("{} (not present)", saved));
                        names.push(saved.clone());
                    }
                    if names.is_empty() {
                        labels.push("(none detected)".to_string());
                        names.push(String::new());
                    }
                    let idx = names.iter().position(|n| *n == saved).unwrap_or(0);
                    let refs: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
                    *values.borrow_mut() = names;
                    d.set_model(Some(&StringList::new(&refs)));
                    d.set_selected(idx as u32);
                }),
            )
        }
        PropKind::IndexChoice { provider } => {
            // The value is the position in the provider's list. A saved
            // index past the end is kept as its own row, as for
            // DynamicChoice.
            let provider: &'static str = provider;
            let dropdown = DropDown::new(None::<StringList>, None::<gtk4::Expression>);
            let values: Rc<RefCell<Vec<i64>>> = Rc::new(RefCell::new(Vec::new()));
            let row = labelled_row(prop, &dropdown);
            {
                let model = model.clone();
                let fire = fire.clone();
                let loading = loading.clone();
                let values = values.clone();
                dropdown.connect_selected_notify(move |d| {
                    if loading.get() {
                        return;
                    }
                    if let Some(v) = values.borrow().get(d.selected() as usize) {
                        model.borrow_mut().insert(key.to_string(), Value::from(*v));
                        fire();
                    }
                });
            }
            let d = dropdown.clone();
            (
                row,
                Box::new(move |m| {
                    let saved = m.get(key).and_then(|v| v.as_i64()).unwrap_or(0);
                    let mut labels: Vec<String> = provider_choices(provider)
                        .into_iter()
                        .map(|(_, label)| label)
                        .collect();
                    let mut indices: Vec<i64> = (0..labels.len() as i64).collect();
                    if labels.is_empty() {
                        labels.push("(none detected)".to_string());
                        indices.push(saved);
                    } else if saved < 0 || saved >= labels.len() as i64 {
                        labels.push(format!("#{} (not present)", saved));
                        indices.push(saved);
                    }
                    let idx = indices.iter().position(|i| *i == saved).unwrap_or(0);
                    let refs: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
                    *values.borrow_mut() = indices;
                    d.set_model(Some(&StringList::new(&refs)));
                    d.set_selected(idx as u32);
                }),
            )
        }
        PropKind::Custom { editor } => {
            let set: Rc<dyn Fn(Vec<(&'static str, Value)>)> = {
                let model = model.clone();
                let fire = fire.clone();
                Rc::new(move |values| {
                    {
                        let mut m = model.borrow_mut();
                        for (k, v) in values {
                            m.insert(k.to_string(), v);
                        }
                    }
                    // `fire` is silent while loading, so a normalising
                    // write during load does not commit.
                    fire();
                })
            };
            let ctx = FieldContext {
                prop: prop.clone(),
                set,
            };
            let factory = FIELD_EDITORS.with(|e| e.borrow().get(editor).cloned());
            let (control, refresh) = match factory {
                Some(f) => f(ctx),
                None => {
                    log::error!("no field editor registered as '{}'", editor);
                    let label = Label::new(Some(&format!("(no editor '{}')", editor)));
                    label.set_xalign(0.0);
                    (
                        label.upcast(),
                        Box::new(|_: &Map<String, Value>| {}) as FieldRefresh,
                    )
                }
            };
            (labelled_row(prop, &control), refresh)
        }
        PropKind::Note => {
            let row = GtkBox::new(Orientation::Horizontal, 0);
            let label = Label::new(Some(prop.label));
            label.set_wrap(true);
            label.set_xalign(0.0);
            label.add_css_class("dim-label");
            label.set_margin_top(4);
            row.append(&label);
            (row, Box::new(|_| {}))
        }
        PropKind::List {
            item,
            new_item,
            min_items,
            item_label,
            add_label,
        } => build_list_row(
            prop,
            item.clone(),
            *new_item,
            *min_items,
            item_label,
            add_label,
            model,
            loading,
            fire,
        ),
    }
}

/// Current array under `key` (empty when absent or not an array).
fn list_items(model: &Model, key: &str) -> Vec<Value> {
    model
        .borrow()
        .get(key)
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
}

/// A `PropKind::List` row: a heading with the add button, then one frame
/// per item holding rows built from the item props over that item's own
/// object. Every item change writes the whole array back under `key`.
/// Add/remove/move rebuild the frames (index-captured closures would go
/// stale otherwise) with `loading` held, so the rebuilt controls do not
/// fire a commit per control.
#[allow(clippy::too_many_arguments)]
fn build_list_row(
    prop: &Prop,
    item_props: Vec<Prop>,
    new_item: fn(usize) -> Value,
    min_items: usize,
    item_label: &'static str,
    add_label: &'static str,
    model: &Model,
    loading: &Rc<Cell<bool>>,
    fire: &Rc<dyn Fn()>,
) -> (GtkBox, FieldRefresh) {
    let key = prop.key;
    let outer = GtkBox::new(Orientation::Vertical, 6);
    let header = GtkBox::new(Orientation::Horizontal, 8);
    let title = Label::new(Some(prop.label));
    title.set_xalign(0.0);
    title.set_hexpand(true);
    if !prop.help.is_empty() {
        title.set_tooltip_text(Some(prop.help));
    }
    let add_btn = Button::with_label(add_label);
    header.append(&title);
    header.append(&add_btn);
    outer.append(&header);
    let items_box = GtkBox::new(Orientation::Vertical, 6);
    outer.append(&items_box);

    type Rebuild = Rc<dyn Fn(&[Value])>;
    // The item buttons call back into the rebuild through a weak handle, so
    // the only strong owner is this row's refresh closure (see `cleanup`).
    let rebuild_slot: Rc<RefCell<Option<Rebuild>>> = Rc::new(RefCell::new(None));
    let item_props = Rc::new(item_props);

    let rebuild: Rebuild = {
        let items_box = items_box.clone();
        let model = model.clone();
        let loading = loading.clone();
        let fire = fire.clone();
        let item_props = item_props.clone();
        let rebuild_weak = Rc::downgrade(&rebuild_slot);
        Rc::new(move |values: &[Value]| {
            let was_loading = loading.replace(true);
            while let Some(child) = items_box.first_child() {
                items_box.remove(&child);
            }
            let count = values.len();
            for (i, value) in values.iter().enumerate() {
                let item_model: Model =
                    Rc::new(RefCell::new(value.as_object().cloned().unwrap_or_default()));
                let frame = Frame::new(None);
                let vbox = GtkBox::new(Orientation::Vertical, 6);
                vbox.set_margin_start(8);
                vbox.set_margin_end(8);
                vbox.set_margin_top(6);
                vbox.set_margin_bottom(6);
                frame.set_child(Some(&vbox));

                let head = GtkBox::new(Orientation::Horizontal, 6);
                let name = Label::new(Some(&format!("{} {}", item_label, i + 1)));
                name.set_xalign(0.0);
                name.set_hexpand(true);
                name.add_css_class("heading");
                let up = Button::from_icon_name("go-up-symbolic");
                let down = Button::from_icon_name("go-down-symbolic");
                let remove = Button::with_label("Remove");
                up.set_tooltip_text(Some("Move up"));
                down.set_tooltip_text(Some("Move down"));
                up.set_sensitive(i > 0);
                down.set_sensitive(i + 1 < count);
                remove.set_sensitive(count > min_items);
                remove.add_css_class("destructive-action");
                head.append(&name);
                head.append(&up);
                head.append(&down);
                head.append(&remove);
                vbox.append(&head);

                // Item rows write the item object, then the array.
                let item_rows: Rc<RefCell<Vec<Row>>> = Rc::new(RefCell::new(Vec::new()));
                let item_fire: Rc<dyn Fn()> = {
                    let model = model.clone();
                    let fire = fire.clone();
                    let loading = loading.clone();
                    let item_rows = item_rows.clone();
                    let item_model = item_model.clone();
                    Rc::new(move || {
                        if loading.get() {
                            return;
                        }
                        update_visibility(&item_rows.borrow(), &item_model.borrow());
                        let object = Value::Object(item_model.borrow().clone());
                        {
                            let mut m = model.borrow_mut();
                            let array = m
                                .entry(key.to_string())
                                .or_insert_with(|| Value::Array(Vec::new()));
                            if let Some(items) = array.as_array_mut() {
                                if i < items.len() {
                                    items[i] = object;
                                }
                            }
                        }
                        fire();
                    })
                };
                let snapshot = item_model.borrow().clone();
                for p in item_props.iter() {
                    let (widget, refresh) = build_row(p, &item_model, &loading, &item_fire);
                    vbox.append(&widget);
                    refresh(&snapshot);
                    item_rows.borrow_mut().push(Row {
                        prop: p.clone(),
                        widget,
                        refresh,
                    });
                }
                update_visibility(&item_rows.borrow(), &snapshot);

                // Structural edits: mutate the array, rebuild, fire.
                let restructure = |f: Rc<dyn Fn(&mut Vec<Value>)>| {
                    let model = model.clone();
                    let fire = fire.clone();
                    let rebuild_weak = rebuild_weak.clone();
                    move |_: &Button| {
                        let mut items = list_items(&model, key);
                        f(&mut items);
                        model
                            .borrow_mut()
                            .insert(key.to_string(), Value::Array(items.clone()));
                        if let Some(rebuild) = rebuild_weak
                            .upgrade()
                            .and_then(|slot| slot.borrow().clone())
                        {
                            rebuild(&items);
                        }
                        fire();
                    }
                };
                remove.connect_clicked(restructure(Rc::new(move |items| {
                    if items.len() > min_items && i < items.len() {
                        items.remove(i);
                    }
                })));
                up.connect_clicked(restructure(Rc::new(move |items| {
                    if i > 0 && i < items.len() {
                        items.swap(i - 1, i);
                    }
                })));
                down.connect_clicked(restructure(Rc::new(move |items| {
                    if i + 1 < items.len() {
                        items.swap(i, i + 1);
                    }
                })));

                items_box.append(&frame);
            }
            loading.set(was_loading);
        })
    };
    *rebuild_slot.borrow_mut() = Some(rebuild.clone());

    {
        let model = model.clone();
        let fire = fire.clone();
        let rebuild = rebuild.clone();
        add_btn.connect_clicked(move |_| {
            let mut items = list_items(&model, key);
            items.push(new_item(items.len()));
            model
                .borrow_mut()
                .insert(key.to_string(), Value::Array(items.clone()));
            rebuild(&items);
            fire();
        });
    }

    let refresh: FieldRefresh = {
        // Owns `rebuild_slot`; dropping this row drops the rebuild closure.
        let rebuild_slot = rebuild_slot;
        Box::new(move |m: &Map<String, Value>| {
            let items = m
                .get(key)
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            if let Some(rebuild) = rebuild_slot.borrow().clone() {
                rebuild(&items);
            }
        })
    };
    (outer, refresh)
}

/// A [`SourceEditor`] over a schema: load/store go through the per-slot
/// flat form, so it serves standalone panels and combo items alike.
pub struct SchemaSourceEditor {
    source_id: String,
    editor: SchemaEditor,
}

impl SchemaSourceEditor {
    pub fn create(source_id: &str, props: Vec<Prop>) -> Box<dyn SourceEditor> {
        Box::new(Self {
            source_id: source_id.to_string(),
            editor: SchemaEditor::new(props),
        })
    }

    /// As [`Self::create`], with a widget appended below the rows.
    pub fn create_with_footer(
        source_id: &str,
        props: Vec<Prop>,
        footer: &impl IsA<Widget>,
    ) -> Box<dyn SourceEditor> {
        let editor = SchemaEditor::new(props);
        editor.append_footer(footer);
        Box::new(Self {
            source_id: source_id.to_string(),
            editor,
        })
    }
}

impl SourceEditor for SchemaSourceEditor {
    fn widget(&self) -> Widget {
        self.editor.widget().clone().upcast()
    }

    fn load(&self, config: &SourceConfig) {
        if config.source_type() != self.source_id {
            return;
        }
        let flat = flatten_source_config(config);
        self.editor.load(flat.into_iter().collect());
    }

    fn store(&self) -> Option<SourceConfig> {
        let flat: HashMap<String, Value> = self.editor.model().into_iter().collect();
        source_config_from_flat(&self.source_id, &flat)
    }

    fn set_on_change(&self, cb: ChangeCallback) -> bool {
        self.editor.set_on_change(cb);
        true
    }

    fn cleanup(&self) {
        self.editor.cleanup();
    }

    fn search_terms(&self) -> Vec<String> {
        self.editor.search_terms()
    }
}
