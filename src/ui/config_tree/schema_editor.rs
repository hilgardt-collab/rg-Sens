//! Schema-driven editor (Phase 3): rows rendered from a config type's
//! [`Prop`]s over the flat JSON form of its config. A source or displayer
//! whose config implements `Schema` is registered through
//! [`SchemaSourceEditor`] / [`SchemaDisplayEditor`] and needs no hand-written
//! widget; the same editor serves the standalone node and combo item pages,
//! because both come from the registry.
//!
//! The editor edits the whole flat object: keys without a prop are carried
//! through untouched, so adding a prop later never loses saved data (and a
//! displayer's cached `theme` passes through, gotcha 20).
//!
//! Nested kinds (`Object`, `Variant`, `List` items) are rows over a
//! sub-object with their own model; every change writes the sub-object back
//! under its key and fires the parent. Anything visual a plain kind cannot
//! express is a `Custom` editor registered by name (`field_editors.rs`).

use super::util::{flatten_source_config, source_config_from_flat};
use crate::core::{DisplayerConfig, FieldMetadata, SourceConfig};
use crate::ui::editor_registry::{ChangeCallback, DisplayEditor, SourceEditor};
use crate::ui::theme::ComboThemeConfig;
use crate::ui::{GradientEditor, ThemeColorSelector, ThemeFontSelector, CLIPBOARD};
use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, CheckButton, DropDown, Entry, Frame, Label, Orientation, SpinButton,
    StringList, Widget,
};
use rg_sens_types::color::ColorStop;
use rg_sens_types::schema::{Condition, Prop, PropKind};
use rg_sens_types::theme::{ColorSource, ColorStopSource, FontSource};
use serde_json::{Map, Value};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

type Provider = Rc<dyn Fn() -> Vec<(String, String)>>;
type Model = Rc<RefCell<Map<String, Value>>>;
/// Sets a control from the flat config (load / rebuild).
pub type FieldRefresh = Box<dyn Fn(&Map<String, Value>)>;
type ThemeHook = Box<dyn Fn(&ComboThemeConfig)>;
type FieldEditorFactory = Rc<dyn Fn(FieldContext) -> FieldEditor>;

thread_local! {
    static PROVIDERS: RefCell<HashMap<&'static str, Provider>> = RefCell::new(HashMap::new());
    static FIELD_EDITORS: RefCell<HashMap<&'static str, FieldEditorFactory>> =
        RefCell::new(HashMap::new());
}

/// What a `PropKind::Custom` editor is handed: its prop, the slot's fields,
/// the current theme, and a way to write the flat config. Writes while the
/// editor is loading are silent (normalisation of a saved value); otherwise
/// they fire the change signal.
#[derive(Clone)]
pub struct FieldContext {
    pub prop: Prop,
    pub fields: Rc<Vec<FieldMetadata>>,
    pub theme: ComboThemeConfig,
    set: Rc<dyn Fn(Vec<(&'static str, Value)>)>,
}

impl FieldContext {
    pub fn set(&self, values: Vec<(&'static str, Value)>) {
        (self.set)(values);
    }
}

/// What a custom editor returns: its control, how to set it from the flat
/// config, and optional theme and cleanup hooks.
pub struct FieldEditor {
    pub widget: Widget,
    pub refresh: FieldRefresh,
    pub on_theme: Option<ThemeHook>,
    pub cleanup: Option<Box<dyn Fn()>>,
    /// Lay the control out as a titled block (large editors) instead of
    /// beside the label.
    pub block: bool,
}

impl FieldEditor {
    pub fn new(widget: impl IsA<Widget>, refresh: FieldRefresh) -> Self {
        Self {
            widget: widget.upcast(),
            refresh,
            on_theme: None,
            cleanup: None,
            block: false,
        }
    }

    pub fn as_block(mut self) -> Self {
        self.block = true;
        self
    }

    pub fn with_theme(mut self, f: impl Fn(&ComboThemeConfig) + 'static) -> Self {
        self.on_theme = Some(Box::new(f));
        self
    }

    pub fn with_cleanup(mut self, f: impl Fn() + 'static) -> Self {
        self.cleanup = Some(Box::new(f));
        self
    }
}

/// Register the editor a `PropKind::Custom { editor }` names.
pub fn register_field_editor(
    name: &'static str,
    f: impl Fn(FieldContext) -> FieldEditor + 'static,
) {
    FIELD_EDITORS.with(|e| {
        e.borrow_mut().insert(name, Rc::new(f));
    });
}

/// Register the `(value, label)` list a `PropKind::DynamicChoice { provider }`
/// or `PropKind::IndexChoice { provider }` offers.
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
        Condition::IsTrue { key } => model.get(*key).and_then(|v| v.as_bool()).unwrap_or(false),
    }
}

/// Per-editor state every row shares.
#[derive(Clone)]
struct Ctx {
    loading: Rc<Cell<bool>>,
    theme: Rc<RefCell<ComboThemeConfig>>,
    fields: Rc<Vec<FieldMetadata>>,
}

struct Row {
    prop: Prop,
    widget: GtkBox,
    /// Set the control from the model (load).
    refresh: FieldRefresh,
    on_theme: Option<ThemeHook>,
    cleanup: Option<Box<dyn Fn()>>,
}

impl Row {
    fn plain(prop: &Prop, widget: GtkBox, refresh: FieldRefresh) -> Self {
        Self {
            prop: prop.clone(),
            widget,
            refresh,
            on_theme: None,
            cleanup: None,
        }
    }
}

/// Rows for a list of props over one JSON object.
pub struct SchemaEditor {
    container: GtkBox,
    /// `(section, page)` when built sectioned; empty otherwise.
    pages: Vec<(String, GtkBox)>,
    model: Model,
    rows: Rc<RefCell<Vec<Row>>>,
    ctx: Ctx,
    on_change: Rc<RefCell<Option<ChangeCallback>>>,
}

impl SchemaEditor {
    /// One page, section headings inline.
    pub fn new(props: Vec<Prop>) -> Self {
        Self::build(props, Vec::new(), false)
    }

    /// One page with the slot's fields available to field editors.
    pub fn with_fields(props: Vec<Prop>, fields: Vec<FieldMetadata>) -> Self {
        Self::build(props, fields, false)
    }

    /// One page per section (`pages()`); `widget()` stays empty.
    pub fn sectioned(props: Vec<Prop>, fields: Vec<FieldMetadata>) -> Self {
        Self::build(props, fields, true)
    }

    fn build(props: Vec<Prop>, fields: Vec<FieldMetadata>, sectioned: bool) -> Self {
        let container = GtkBox::new(Orientation::Vertical, 6);
        let model: Model = Rc::new(RefCell::new(Map::new()));
        let rows: Rc<RefCell<Vec<Row>>> = Rc::new(RefCell::new(Vec::new()));
        let ctx = Ctx {
            loading: Rc::new(Cell::new(false)),
            theme: Rc::new(RefCell::new(ComboThemeConfig::default())),
            fields: Rc::new(fields),
        };
        let on_change: Rc<RefCell<Option<ChangeCallback>>> = Rc::new(RefCell::new(None));

        // One closure every control calls after writing its value.
        let fire: Rc<dyn Fn()> = {
            let loading = ctx.loading.clone();
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

        // Controls are initialised while building; nothing may fire.
        ctx.loading.set(true);
        let mut pages: Vec<(String, GtkBox)> = Vec::new();
        let mut current_section: Option<&'static str> = None;
        for prop in props {
            let target: GtkBox = if sectioned {
                let name = prop.section.unwrap_or("General");
                match pages.iter().find(|(n, _)| n == name) {
                    Some((_, page)) => page.clone(),
                    None => {
                        let page = GtkBox::new(Orientation::Vertical, 6);
                        pages.push((name.to_string(), page.clone()));
                        page
                    }
                }
            } else {
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
                container.clone()
            };
            let row = build_row(&prop, &model, &ctx, &fire);
            target.append(&row.widget);
            rows.borrow_mut().push(row);
        }
        ctx.loading.set(false);

        Self {
            container,
            pages,
            model,
            rows,
            ctx,
            on_change,
        }
    }

    pub fn widget(&self) -> &GtkBox {
        &self.container
    }

    /// The section pages of a `sectioned` editor.
    pub fn pages(&self) -> Vec<(String, GtkBox)> {
        self.pages.clone()
    }

    /// Replace the model and refresh every control without firing changes.
    pub fn load(&self, map: Map<String, Value>) {
        let was_loading = self.ctx.loading.replace(true);
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
        self.ctx.loading.set(was_loading);
    }

    pub fn model(&self) -> Map<String, Value> {
        self.model.borrow().clone()
    }

    pub fn set_on_change(&self, cb: ChangeCallback) {
        *self.on_change.borrow_mut() = Some(cb);
    }

    /// Push the resolved theme to every theme-aware control. Never fires
    /// the change signal: a theme push is not an edit.
    pub fn set_theme(&self, theme: &ComboThemeConfig) {
        *self.ctx.theme.borrow_mut() = theme.clone();
        apply_theme(&self.rows.borrow(), theme);
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

    /// Labels and keys of one section's rows.
    pub fn section_search_terms(&self, section: &str) -> Vec<String> {
        let mut out = Vec::new();
        collect_search_terms(
            self.rows
                .borrow()
                .iter()
                .map(|r| &r.prop)
                .filter(|p| p.section.unwrap_or("General") == section),
            &mut out,
        );
        out
    }

    /// Break the control ↔ closure cycles.
    pub fn cleanup(&self) {
        *self.on_change.borrow_mut() = None;
        cleanup_rows(&mut self.rows.borrow_mut());
    }
}

fn collect_search_terms<'a>(props: impl Iterator<Item = &'a Prop>, out: &mut Vec<String>) {
    for prop in props {
        match &prop.kind {
            PropKind::Note => {}
            PropKind::List { item, .. } | PropKind::Object { props: item } => {
                out.push(prop.label.to_string());
                out.push(prop.key.replace('_', " "));
                collect_search_terms(item.iter(), out);
            }
            PropKind::Variant { variants, .. } => {
                out.push(prop.label.to_string());
                out.push(prop.key.replace('_', " "));
                for v in variants {
                    out.push(v.label.to_string());
                    collect_search_terms(v.props.iter(), out);
                }
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

fn apply_theme(rows: &[Row], theme: &ComboThemeConfig) {
    for row in rows {
        if let Some(hook) = &row.on_theme {
            hook(theme);
        }
    }
}

fn cleanup_rows(rows: &mut Vec<Row>) {
    for row in rows.iter() {
        if let Some(c) = &row.cleanup {
            c();
        }
    }
    rows.clear();
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

/// A block row: title (with optional trailing widgets) above its content.
fn titled_block(prop: &Prop, trailing: &[&Button]) -> (GtkBox, GtkBox) {
    let outer = GtkBox::new(Orientation::Vertical, 4);
    let header = GtkBox::new(Orientation::Horizontal, 8);
    let title = Label::new(Some(prop.label));
    title.set_xalign(0.0);
    title.set_hexpand(true);
    title.add_css_class("heading");
    if !prop.help.is_empty() {
        title.set_tooltip_text(Some(prop.help));
    }
    header.append(&title);
    for b in trailing {
        header.append(*b);
    }
    outer.append(&header);
    let body = GtkBox::new(Orientation::Vertical, 6);
    body.set_margin_start(12);
    outer.append(&body);
    (outer, body)
}

// ----------------------------------------------------------------------------
// Nested rows (Object, Variant, List items)
// ----------------------------------------------------------------------------

/// Rows over a sub-object stored at `parent[key]` (or `parent[key][index]`).
struct Nested {
    model: Model,
    rows: Rc<RefCell<Vec<Row>>>,
    container: GtkBox,
}

impl Nested {
    /// Built with `loading` held: the controls are initialised from
    /// `initial` and nothing may fire.
    fn build(
        props: &[Prop],
        initial: Map<String, Value>,
        parent: &Model,
        key: &'static str,
        index: Option<usize>,
        ctx: &Ctx,
        parent_fire: &Rc<dyn Fn()>,
    ) -> Self {
        let was_loading = ctx.loading.replace(true);
        let model: Model = Rc::new(RefCell::new(initial));
        let rows: Rc<RefCell<Vec<Row>>> = Rc::new(RefCell::new(Vec::new()));
        let container = GtkBox::new(Orientation::Vertical, 6);
        let fire: Rc<dyn Fn()> = {
            let loading = ctx.loading.clone();
            let rows = rows.clone();
            let model = model.clone();
            let parent = parent.clone();
            let parent_fire = parent_fire.clone();
            Rc::new(move || {
                if loading.get() {
                    return;
                }
                update_visibility(&rows.borrow(), &model.borrow());
                let object = Value::Object(model.borrow().clone());
                {
                    let mut m = parent.borrow_mut();
                    match index {
                        None => {
                            m.insert(key.to_string(), object);
                        }
                        Some(i) => {
                            let array = m
                                .entry(key.to_string())
                                .or_insert_with(|| Value::Array(Vec::new()));
                            if let Some(items) = array.as_array_mut() {
                                if i < items.len() {
                                    items[i] = object;
                                }
                            }
                        }
                    }
                }
                parent_fire();
            })
        };
        let snapshot = model.borrow().clone();
        for p in props {
            let row = build_row(p, &model, ctx, &fire);
            container.append(&row.widget);
            (row.refresh)(&snapshot);
            rows.borrow_mut().push(row);
        }
        update_visibility(&rows.borrow(), &snapshot);
        ctx.loading.set(was_loading);
        Self {
            model,
            rows,
            container,
        }
    }

    /// Replace the sub-object and refresh its rows (callers hold `loading`).
    fn refresh(&self, object: Map<String, Value>) {
        *self.model.borrow_mut() = object.clone();
        let rows = self.rows.borrow();
        for row in rows.iter() {
            (row.refresh)(&object);
        }
        update_visibility(&rows, &object);
    }

    fn apply_theme(&self, theme: &ComboThemeConfig) {
        apply_theme(&self.rows.borrow(), theme);
    }

    fn cleanup(&self) {
        cleanup_rows(&mut self.rows.borrow_mut());
    }
}

fn object_at(m: &Map<String, Value>, key: &str) -> Map<String, Value> {
    m.get(key)
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default()
}

// ----------------------------------------------------------------------------
// Rows
// ----------------------------------------------------------------------------

fn build_row(prop: &Prop, model: &Model, ctx: &Ctx, fire: &Rc<dyn Fn()>) -> Row {
    let key = prop.key;
    let loading = ctx.loading.clone();
    match &prop.kind {
        PropKind::Bool => {
            let check = CheckButton::new();
            let row = labelled_row(prop, &check);
            {
                let model = model.clone();
                let fire = fire.clone();
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
            Row::plain(
                prop,
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
            Row::plain(
                prop,
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
            Row::plain(
                prop,
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
            Row::plain(
                prop,
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
            Row::plain(
                prop,
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
            Row::plain(
                prop,
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
            Row::plain(
                prop,
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
            Row::plain(
                prop,
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
        PropKind::Color => {
            let selector = Rc::new(ThemeColorSelector::new(ColorSource::default()));
            selector.set_theme_config(ctx.theme.borrow().clone());
            let row = labelled_row(prop, selector.widget());
            {
                let model = model.clone();
                let fire = fire.clone();
                selector.set_on_change(move |source| {
                    if loading.get() {
                        return;
                    }
                    if let Ok(v) = serde_json::to_value(&source) {
                        model.borrow_mut().insert(key.to_string(), v);
                        fire();
                    }
                });
            }
            let refresh: FieldRefresh = {
                let selector = selector.clone();
                Box::new(move |m| {
                    if let Some(source) = m
                        .get(key)
                        .and_then(|v| serde_json::from_value::<ColorSource>(v.clone()).ok())
                    {
                        // `set_source` is silent.
                        selector.set_source(source);
                    }
                })
            };
            Row {
                prop: prop.clone(),
                widget: row,
                refresh,
                on_theme: Some(Box::new(move |t| selector.set_theme_config(t.clone()))),
                cleanup: None,
            }
        }
        PropKind::ColorStops { linear, themed } => {
            let themed = *themed;
            let copy_btn = Button::with_label("Copy gradient");
            let paste_btn = Button::with_label("Paste gradient");
            let (outer, body) = titled_block(prop, &[&copy_btn, &paste_btn]);
            let editor = Rc::new(if *linear {
                GradientEditor::new_linear_no_angle()
            } else {
                GradientEditor::new_without_angle()
            });
            editor.set_theme_config(ctx.theme.borrow().clone());
            body.append(editor.widget());
            // Themed stops keep theme references; plain stops are resolved
            // colours (`TextFillType` gradients).
            let current_value = {
                let editor = editor.clone();
                move || {
                    if themed {
                        serde_json::to_value(editor.get_stops_source())
                    } else {
                        serde_json::to_value(editor.get_stops())
                    }
                }
            };
            let set_value = {
                let editor = editor.clone();
                move |v: &Value| {
                    if themed {
                        if let Ok(stops) = serde_json::from_value::<Vec<ColorStopSource>>(v.clone())
                        {
                            editor.set_stops_source(stops);
                        }
                    } else if let Ok(stops) = serde_json::from_value::<Vec<ColorStop>>(v.clone()) {
                        editor.set_stops(stops);
                    }
                }
            };
            {
                let model = model.clone();
                let fire = fire.clone();
                let loading = loading.clone();
                let current_value = current_value.clone();
                editor.set_on_change(move || {
                    if loading.get() {
                        return;
                    }
                    if let Ok(v) = current_value() {
                        model.borrow_mut().insert(key.to_string(), v);
                        fire();
                    }
                });
            }
            {
                // The clipboard holds resolved stops, so theme links are
                // dropped on copy (as the old widgets did).
                let editor = editor.clone();
                copy_btn.connect_clicked(move |_| {
                    if let Ok(mut cb) = CLIPBOARD.lock() {
                        cb.copy_gradient_stops(editor.get_stops());
                    }
                });
            }
            {
                let editor = editor.clone();
                let model = model.clone();
                let fire = fire.clone();
                let current_value = current_value.clone();
                paste_btn.connect_clicked(move |_| {
                    let stops = CLIPBOARD
                        .lock()
                        .ok()
                        .and_then(|cb| cb.paste_gradient_stops());
                    let Some(stops) = stops else { return };
                    editor.set_stops(stops);
                    if let Ok(v) = current_value() {
                        model.borrow_mut().insert(key.to_string(), v);
                        fire();
                    }
                });
            }
            let refresh: FieldRefresh = Box::new(move |m| {
                if let Some(v) = m.get(key) {
                    // Guarded setters: they do not fire the editor's on_change.
                    set_value(v);
                }
            });
            let editor_theme = editor.clone();
            Row {
                prop: prop.clone(),
                widget: outer,
                refresh,
                on_theme: Some(Box::new(move |t| editor_theme.set_theme_config(t.clone()))),
                // The editor's own change closure holds an Rc to the editor
                // (gotcha 11): drop it when the row goes.
                cleanup: Some(Box::new(move || editor.set_on_change(|| {}))),
            }
        }
        PropKind::Font { legacy } => {
            let legacy = *legacy;
            let selector = Rc::new(ThemeFontSelector::new(FontSource::default()));
            selector.set_theme_config(ctx.theme.borrow().clone());
            let row = labelled_row(prop, selector.widget());
            let refreshing = Rc::new(Cell::new(false));
            {
                let model = model.clone();
                let fire = fire.clone();
                let refreshing = refreshing.clone();
                selector.set_on_change(move |source| {
                    if loading.get() || refreshing.get() {
                        return;
                    }
                    if let Ok(v) = serde_json::to_value(&source) {
                        model.borrow_mut().insert(key.to_string(), v);
                        fire();
                    }
                });
            }
            let refresh: FieldRefresh = {
                let selector = selector.clone();
                Box::new(move |m| {
                    let saved = m
                        .get(key)
                        .and_then(|v| serde_json::from_value::<FontSource>(v.clone()).ok());
                    let source = saved.or_else(|| {
                        // Older configs carry family/size beside a null source.
                        let (family_key, size_key) = legacy?;
                        let family = m.get(family_key)?.as_str()?.to_string();
                        let size = m.get(size_key)?.as_f64()?;
                        Some(FontSource::Custom { family, size })
                    });
                    if let Some(source) = source {
                        // `set_source` toggles buttons, which fire on_change.
                        refreshing.set(true);
                        selector.set_source(source);
                        refreshing.set(false);
                    }
                })
            };
            Row {
                prop: prop.clone(),
                widget: row,
                refresh,
                on_theme: Some(Box::new(move |t| selector.set_theme_config(t.clone()))),
                cleanup: None,
            }
        }
        PropKind::Field => {
            // Fields of the panel's source; a saved id the source does not
            // expose is kept as its own row.
            let dropdown = DropDown::new(None::<StringList>, None::<gtk4::Expression>);
            let ids: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
            let row = labelled_row(prop, &dropdown);
            {
                let model = model.clone();
                let fire = fire.clone();
                let ids = ids.clone();
                dropdown.connect_selected_notify(move |d| {
                    if loading.get() {
                        return;
                    }
                    if let Some(id) = ids.borrow().get(d.selected() as usize) {
                        model
                            .borrow_mut()
                            .insert(key.to_string(), Value::String(id.clone()));
                        fire();
                    }
                });
            }
            let d = dropdown.clone();
            let fields = ctx.fields.clone();
            Row::plain(
                prop,
                row,
                Box::new(move |m| {
                    let saved = m
                        .get(key)
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let mut names: Vec<String> = fields.iter().map(|f| f.id.clone()).collect();
                    let mut labels: Vec<String> = fields
                        .iter()
                        .map(|f| {
                            if f.name.is_empty() || f.name == f.id {
                                f.id.clone()
                            } else {
                                format!("{} ({})", f.name, f.id)
                            }
                        })
                        .collect();
                    if !saved.is_empty() && !names.contains(&saved) {
                        labels.push(format!("{} (not present)", saved));
                        names.push(saved.clone());
                    }
                    if names.is_empty() {
                        labels.push("(no fields)".to_string());
                        names.push(saved.clone());
                    }
                    let idx = names.iter().position(|n| *n == saved).unwrap_or(0);
                    let refs: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
                    *ids.borrow_mut() = names;
                    d.set_model(Some(&StringList::new(&refs)));
                    d.set_selected(idx as u32);
                }),
            )
        }
        PropKind::Object { props } => {
            let (outer, body) = titled_block(prop, &[]);
            let nested = Rc::new(Nested::build(
                props,
                Map::new(),
                model,
                key,
                None,
                ctx,
                fire,
            ));
            body.append(&nested.container);
            let refresh: FieldRefresh = {
                let nested = nested.clone();
                Box::new(move |m| nested.refresh(object_at(m, key)))
            };
            let n_theme = nested.clone();
            Row {
                prop: prop.clone(),
                widget: outer,
                refresh,
                on_theme: Some(Box::new(move |t| n_theme.apply_theme(t))),
                cleanup: Some(Box::new(move || nested.cleanup())),
            }
        }
        PropKind::Variant { tag, variants } => {
            // A dropdown for the tag; the union of the variants' rows
            // underneath, each gated to its variant. Choosing a variant
            // replaces the whole object with that variant's template.
            let tag: &'static str = tag;
            let labels: Vec<&str> = variants.iter().map(|v| v.label).collect();
            let values: Vec<&'static str> = variants.iter().map(|v| v.value).collect();
            let templates: Vec<fn() -> Value> = variants.iter().map(|v| v.template).collect();
            let sub_props: Vec<Prop> = variants
                .iter()
                .flat_map(|v| {
                    v.props.iter().map(move |p| {
                        let mut p = p.clone();
                        if p.visible_when.is_none() {
                            p.visible_when = Some(Condition::Equals {
                                key: tag,
                                values: vec![v.value],
                            });
                        }
                        p
                    })
                })
                .collect();
            let dropdown = DropDown::new(Some(StringList::new(&labels)), None::<gtk4::Expression>);
            let outer = GtkBox::new(Orientation::Vertical, 6);
            outer.append(&labelled_row(prop, &dropdown));
            let nested = Rc::new(Nested::build(
                &sub_props,
                Map::new(),
                model,
                key,
                None,
                ctx,
                fire,
            ));
            let body = GtkBox::new(Orientation::Vertical, 6);
            body.set_margin_start(12);
            body.append(&nested.container);
            outer.append(&body);
            let refreshing = Rc::new(Cell::new(false));
            {
                let model = model.clone();
                let fire = fire.clone();
                let nested = nested.clone();
                let refreshing = refreshing.clone();
                let loading = loading.clone();
                dropdown.connect_selected_notify(move |d| {
                    if loading.get() || refreshing.get() {
                        return;
                    }
                    let Some(template) = templates.get(d.selected() as usize) else {
                        return;
                    };
                    let value = template();
                    let object = value.as_object().cloned().unwrap_or_default();
                    model.borrow_mut().insert(key.to_string(), value);
                    let was = loading.replace(true);
                    nested.refresh(object);
                    loading.set(was);
                    fire();
                });
            }
            let refresh: FieldRefresh = {
                let nested = nested.clone();
                let dropdown = dropdown.clone();
                Box::new(move |m| {
                    let object = object_at(m, key);
                    let tagged = object.get(tag).and_then(|v| v.as_str()).unwrap_or("");
                    let idx = values.iter().position(|v| *v == tagged).unwrap_or(0);
                    refreshing.set(true);
                    dropdown.set_selected(idx as u32);
                    refreshing.set(false);
                    nested.refresh(object);
                })
            };
            let n_theme = nested.clone();
            Row {
                prop: prop.clone(),
                widget: outer,
                refresh,
                on_theme: Some(Box::new(move |t| n_theme.apply_theme(t))),
                cleanup: Some(Box::new(move || nested.cleanup())),
            }
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
            let context = FieldContext {
                prop: prop.clone(),
                fields: ctx.fields.clone(),
                theme: ctx.theme.borrow().clone(),
                set,
            };
            let factory = FIELD_EDITORS.with(|e| e.borrow().get(editor).cloned());
            let field = match factory {
                Some(f) => f(context),
                None => {
                    log::error!("no field editor registered as '{}'", editor);
                    let label = Label::new(Some(&format!("(no editor '{}')", editor)));
                    label.set_xalign(0.0);
                    FieldEditor::new(label, Box::new(|_: &Map<String, Value>| {}))
                }
            };
            let widget = if field.block {
                let (outer, body) = titled_block(prop, &[]);
                body.set_margin_start(0);
                body.append(&field.widget);
                outer
            } else {
                labelled_row(prop, &field.widget)
            };
            Row {
                prop: prop.clone(),
                widget,
                refresh: field.refresh,
                on_theme: field.on_theme,
                cleanup: field.cleanup,
            }
        }
        PropKind::Note => {
            let row = GtkBox::new(Orientation::Horizontal, 0);
            let label = Label::new(Some(prop.label));
            label.set_wrap(true);
            label.set_xalign(0.0);
            label.add_css_class("dim-label");
            label.set_margin_top(4);
            row.append(&label);
            Row::plain(prop, row, Box::new(|_| {}))
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
            ctx,
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
    ctx: &Ctx,
    fire: &Rc<dyn Fn()>,
) -> Row {
    let key = prop.key;
    let add_btn = Button::with_label(add_label);
    let paste_btn = Button::with_label("Paste");
    paste_btn.set_tooltip_text(Some("Append the copied item"));
    let (outer, items_box) = titled_block(prop, &[&add_btn, &paste_btn]);
    items_box.set_margin_start(0);

    type Rebuild = Rc<dyn Fn(&[Value])>;
    // The item buttons call back into the rebuild through a weak handle, so
    // the only strong owner is this row's refresh closure (see `cleanup`).
    let rebuild_slot: Rc<RefCell<Option<Rebuild>>> = Rc::new(RefCell::new(None));
    let item_props = Rc::new(item_props);
    let items: Rc<RefCell<Vec<Rc<Nested>>>> = Rc::new(RefCell::new(Vec::new()));

    let rebuild: Rebuild = {
        let items_box = items_box.clone();
        let model = model.clone();
        let ctx = ctx.clone();
        let fire = fire.clone();
        let item_props = item_props.clone();
        let items = items.clone();
        let rebuild_weak = Rc::downgrade(&rebuild_slot);
        Rc::new(move |values: &[Value]| {
            let was_loading = ctx.loading.replace(true);
            while let Some(child) = items_box.first_child() {
                items_box.remove(&child);
            }
            for n in items.borrow_mut().drain(..) {
                n.cleanup();
            }
            let count = values.len();
            for (i, value) in values.iter().enumerate() {
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
                let duplicate = Button::from_icon_name("edit-copy-symbolic");
                let copy = Button::with_label("Copy");
                let remove = Button::with_label("Remove");
                up.set_tooltip_text(Some("Move up"));
                down.set_tooltip_text(Some("Move down"));
                duplicate.set_tooltip_text(Some("Duplicate below"));
                copy.set_tooltip_text(Some(
                    "Copy this item; paste it into a list of the same kind",
                ));
                up.set_sensitive(i > 0);
                down.set_sensitive(i + 1 < count);
                remove.set_sensitive(count > min_items);
                remove.add_css_class("destructive-action");
                head.append(&name);
                head.append(&up);
                head.append(&down);
                head.append(&duplicate);
                head.append(&copy);
                head.append(&remove);
                vbox.append(&head);

                let nested = Rc::new(Nested::build(
                    &item_props,
                    value.as_object().cloned().unwrap_or_default(),
                    &model,
                    key,
                    Some(i),
                    &ctx,
                    &fire,
                ));
                vbox.append(&nested.container);
                items.borrow_mut().push(nested);

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
                duplicate.connect_clicked(restructure(Rc::new(move |items| {
                    if i < items.len() {
                        let copy = items[i].clone();
                        items.insert(i + 1, copy);
                    }
                })));
                {
                    let model = model.clone();
                    copy.connect_clicked(move |_| {
                        if let Some(item) = list_items(&model, key).get(i) {
                            if let Ok(mut cb) = CLIPBOARD.lock() {
                                cb.copy_list_item(key.to_string(), item.clone());
                            }
                        }
                    });
                }

                items_box.append(&frame);
            }
            ctx.loading.set(was_loading);
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

    {
        let model = model.clone();
        let fire = fire.clone();
        let rebuild = rebuild.clone();
        paste_btn.connect_clicked(move |_| {
            // Items are only interchangeable within lists of the same key.
            let pasted = CLIPBOARD
                .lock()
                .ok()
                .and_then(|cb| cb.paste_list_item())
                .filter(|(k, _)| k == key);
            let Some((_, item)) = pasted else { return };
            let mut items = list_items(&model, key);
            items.push(item);
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
    let items_theme = items.clone();
    Row {
        prop: prop.clone(),
        widget: outer,
        refresh,
        on_theme: Some(Box::new(move |t| {
            for n in items_theme.borrow().iter() {
                n.apply_theme(t);
            }
        })),
        cleanup: Some(Box::new(move || {
            for n in items.borrow_mut().drain(..) {
                n.cleanup();
            }
        })),
    }
}

// ----------------------------------------------------------------------------
// Registry adapters
// ----------------------------------------------------------------------------

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

/// A [`DisplayEditor`] over a schema, one page per section: the tree shows
/// the pages as child nodes of Display (`sections()`), as it does for the
/// themed combo editors. The displayer's cached `theme` key has no prop and
/// passes through untouched.
pub struct SchemaDisplayEditor {
    displayer_id: String,
    editor: SchemaEditor,
}

impl SchemaDisplayEditor {
    pub fn create(
        displayer_id: &str,
        props: Vec<Prop>,
        fields: Vec<FieldMetadata>,
    ) -> Box<dyn DisplayEditor> {
        // A single section (or none) needs no child nodes: show the rows on
        // the Display page itself.
        let mut sections: Vec<Option<&str>> = props.iter().map(|p| p.section).collect();
        sections.sort_unstable();
        sections.dedup();
        let editor = if sections.len() > 1 {
            SchemaEditor::sectioned(props, fields)
        } else {
            SchemaEditor::with_fields(props, fields)
        };
        Box::new(Self {
            displayer_id: displayer_id.to_string(),
            editor,
        })
    }
}

impl DisplayEditor for SchemaDisplayEditor {
    fn widget(&self) -> Widget {
        self.editor.widget().clone().upcast()
    }

    fn load(&self, config: &DisplayerConfig) {
        if config.displayer_type() != self.displayer_id {
            return;
        }
        if let Some(Value::Object(map)) = config.to_inner_value() {
            self.editor.load(map);
        }
    }

    fn store(&self) -> Option<DisplayerConfig> {
        DisplayerConfig::from_value_for_type(&self.displayer_id, Value::Object(self.editor.model()))
    }

    fn set_on_change(&self, cb: ChangeCallback) -> bool {
        self.editor.set_on_change(cb);
        true
    }

    fn set_theme(&self, theme: &ComboThemeConfig) {
        self.editor.set_theme(theme);
    }

    fn cleanup(&self) {
        self.editor.cleanup();
    }

    fn sections(&self) -> Vec<(String, Widget)> {
        self.editor
            .pages()
            .into_iter()
            .map(|(name, page)| (name, page.upcast()))
            .collect()
    }

    fn section_search_terms(&self, section: &str) -> Vec<String> {
        self.editor.section_search_terms(section)
    }
}
