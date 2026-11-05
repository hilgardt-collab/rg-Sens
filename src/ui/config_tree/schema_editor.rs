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
    Box as GtkBox, CheckButton, DropDown, Entry, Label, Orientation, SpinButton, StringList, Widget,
};
use rg_sens_types::schema::{Condition, Prop, PropKind};
use serde_json::{Map, Value};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

type Provider = Rc<dyn Fn() -> Vec<(String, String)>>;
type Model = Rc<RefCell<Map<String, Value>>>;

thread_local! {
    static PROVIDERS: RefCell<HashMap<&'static str, Provider>> = RefCell::new(HashMap::new());
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
    refresh: Box<dyn Fn(&Map<String, Value>)>,
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

    /// Labels and keys, for the tree search box.
    pub fn search_terms(&self) -> Vec<String> {
        self.rows
            .borrow()
            .iter()
            .flat_map(|r| [r.prop.label.to_string(), r.prop.key.replace('_', " ")])
            .collect()
    }

    /// Break the control ↔ closure cycles.
    pub fn cleanup(&self) {
        *self.on_change.borrow_mut() = None;
        self.rows.borrow_mut().clear();
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
) -> (GtkBox, Box<dyn Fn(&Map<String, Value>)>) {
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
    }
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
