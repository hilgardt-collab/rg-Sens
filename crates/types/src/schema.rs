//! Declarative property schemas (Phase 3 of the config UI redesign).
//!
//! A config type that implements [`Schema`] describes its editable
//! properties as [`Prop`]s. The UI renders rows from them over the flat
//! JSON form of the config, so such a type needs no hand-written widget.
//! Keys are the serde field names; enum option values are the serde
//! variant names verbatim.
//!
//! This crate has no GTK: anything visual that a plain kind cannot express
//! is a *name* ([`PropKind::DynamicChoice`], [`PropKind::IndexChoice`],
//! [`PropKind::Custom`]) that the UI resolves through a registry.

use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;

/// What kind of control a property needs, with its bounds.
#[derive(Debug, Clone)]
pub enum PropKind {
    Bool,
    /// Integer field (stored as a JSON integer, never as a float).
    Int {
        min: i64,
        max: i64,
        step: i64,
    },
    Float {
        min: f64,
        max: f64,
        step: f64,
        digits: u32,
    },
    /// `Option<f64>`: `null` when unset.
    OptionalFloat {
        min: f64,
        max: f64,
        step: f64,
        digits: u32,
    },
    Text {
        placeholder: &'static str,
    },
    /// `Option<String>`: an empty entry stores `null`, never `""`.
    OptionalText {
        placeholder: &'static str,
    },
    /// A fixed choice: `(serde variant name, label)` pairs.
    Enum {
        options: Vec<(&'static str, &'static str)>,
    },
    /// A choice list the UI resolves at build time by provider name
    /// (network interfaces, mounted disks, …); the value is the chosen
    /// entry's string. The saved value is kept even when the provider does
    /// not list it.
    DynamicChoice {
        provider: &'static str,
    },
    /// Like [`PropKind::DynamicChoice`] but the value is the *position* in
    /// the provider's list, stored as an integer (CPU sensor index, GPU
    /// index). A saved index past the end of the list is kept as its own row.
    IndexChoice {
        provider: &'static str,
    },
    /// A `Vec` of structs, each described by `item`. `new_item(n)` is the
    /// JSON object appended as the `n`th item (0-based); the UI refuses to
    /// drop below `min_items`.
    List {
        item: Vec<Prop>,
        new_item: fn(usize) -> Value,
        min_items: usize,
        /// Heading of each item, numbered from 1 ("Line" → "Line 1").
        item_label: &'static str,
        add_label: &'static str,
        /// Optional short description of an item for its heading or tree
        /// node ("Line 1 · value").
        item_summary: Option<fn(&Value) -> String>,
        /// Integer key of the same object kept equal to the list length on
        /// every structural edit (LCARS `segment_count`).
        count_key: Option<&'static str>,
    },
    /// A `Vec` of a plain enum with one row per element ("Group 1 …"):
    /// the row count follows the integer under `count_key`, elements past
    /// the stored length show the value under `default_key` (or the first
    /// option) and are written when changed.
    EnumList {
        options: Vec<(&'static str, &'static str)>,
        count_key: &'static str,
        default_key: Option<&'static str>,
        item_label: &'static str,
    },
    /// An editor the UI registers under this name (timezone picker, sensor
    /// picker, …). It may read and write any keys of the flat config, not
    /// only `key`.
    Custom {
        editor: &'static str,
    },
    /// Explanatory text with no value; `key` is unused (see [`Prop::note`]).
    Note,
    /// A nested struct under `key`; `props` edit its fields.
    Object {
        props: Vec<Prop>,
    },
    /// An internally tagged enum with payload (`#[serde(tag = "...")]`).
    /// Choosing a variant replaces the whole object with that variant's
    /// template, so keys of the previous variant never linger.
    Variant {
        tag: &'static str,
        variants: Vec<Variant>,
    },
    /// A theme-or-custom colour (`ColorSource`).
    Color,
    /// Gradient stops: `Vec<ColorStopSource>` when `themed`, plain
    /// `Vec<ColorStop>` otherwise. `linear` picks the preview: a linear ramp
    /// (bar fills) or the arc-style sweep (arc colour stops). `angle_key`
    /// names the sibling key holding the ramp's angle in degrees; the editor
    /// then carries the angle control and its preview follows it. `theme_key`
    /// names the sibling boolean that makes the field draw with the theme
    /// gradient's stops (`theme::resolve_stops`); the row then offers a
    /// "Theme gradient" toggle. The flag is additive: an absent key is `false`.
    ColorStops {
        linear: bool,
        themed: bool,
        angle_key: Option<&'static str>,
        theme_key: Option<&'static str>,
    },
    /// A theme-or-custom font (`FontSource`, possibly `Option`). When the
    /// value is `null`, `legacy` names the `(family, size)` keys of the same
    /// object that older configs carry instead.
    Font {
        legacy: Option<(&'static str, &'static str)>,
    },
    /// A field id of the panel's source, chosen from the fields the editor
    /// was given. A saved id the source does not expose is kept as its own
    /// row.
    Field,
    /// A plain colour (`Color { r, g, b, a }`), not theme-aware.
    Rgba,
    /// A plain font: `key` holds the family, `size_key` (same object) the size.
    PlainFont {
        size_key: &'static str,
    },
}

/// One alternative of a [`PropKind::Variant`].
#[derive(Debug, Clone)]
pub struct Variant {
    /// The tag's serde value.
    pub value: &'static str,
    pub label: &'static str,
    /// Rows shown while this variant is selected; they edit keys of the
    /// variant object. Conditions inside refer to that object's keys.
    pub props: Vec<Prop>,
    /// The complete object for this variant, tag included.
    pub template: fn() -> Value,
}

/// When a property's row is shown.
#[derive(Debug, Clone, PartialEq)]
pub enum Condition {
    /// The named key (an enum) holds one of these variant names.
    Equals {
        key: &'static str,
        values: Vec<&'static str>,
    },
    /// The named boolean key is false or absent.
    IsFalse { key: &'static str },
    /// The named boolean key is true.
    IsTrue { key: &'static str },
}

impl Condition {
    pub fn key(&self) -> &'static str {
        match self {
            Condition::Equals { key, .. }
            | Condition::IsFalse { key }
            | Condition::IsTrue { key } => key,
        }
    }
}

/// One editable property of a config type.
#[derive(Debug, Clone)]
pub struct Prop {
    /// Serde field name.
    pub key: &'static str,
    pub label: &'static str,
    /// Tooltip; empty for none.
    pub help: &'static str,
    /// Rows with the same section are grouped under a heading.
    pub section: Option<&'static str>,
    pub kind: PropKind,
    pub visible_when: Option<Condition>,
}

impl Prop {
    pub fn new(key: &'static str, label: &'static str, kind: PropKind) -> Self {
        Self {
            key,
            label,
            help: "",
            section: None,
            kind,
            visible_when: None,
        }
    }

    /// A row of explanatory text (`label` is the text).
    pub fn note(text: &'static str) -> Self {
        Self::new("", text, PropKind::Note)
    }

    pub fn help(mut self, help: &'static str) -> Self {
        self.help = help;
        self
    }

    pub fn section(mut self, section: &'static str) -> Self {
        self.section = Some(section);
        self
    }

    pub fn visible_when(mut self, condition: Condition) -> Self {
        self.visible_when = Some(condition);
        self
    }
}

/// A config type that describes its own editor.
pub trait Schema {
    fn props() -> Vec<Prop>;
}

// ----------------------------------------------------------------------------
// Tree editor pages (pure: the tree dialog derives a combo item's child nodes
// from these without building the item's editor)
// ----------------------------------------------------------------------------

/// Name of the page a prop's rows land on when an editor is split into one
/// page per section.
pub fn section_name(prop: &Prop) -> &'static str {
    prop.section.unwrap_or("General")
}

/// Whether an editor over `props` splits into pages: more than one distinct
/// section (a `None` section counts as one, shown as "General"). A single
/// section keeps its rows on the parent page.
pub fn is_sectioned(props: &[Prop]) -> bool {
    let mut sections: Vec<Option<&str>> = props.iter().map(|p| p.section).collect();
    sections.sort_unstable();
    sections.dedup();
    sections.len() > 1
}

/// Title of a list item: "Line 1", or "Line 1 · summary" when the list has a
/// summary and it is not empty.
pub fn list_item_title(
    item_label: &str,
    index: usize,
    summary: Option<fn(&Value) -> String>,
    item: &Value,
) -> String {
    let base = format!("{} {}", item_label, index + 1);
    match summary.map(|f| f(item)).filter(|s| !s.is_empty()) {
        Some(s) => format!("{} · {}", base, s),
        None => base,
    }
}

/// Labels and keys the tree search box may match, recursing into objects,
/// variants and list items.
pub fn collect_search_terms<'a>(props: impl Iterator<Item = &'a Prop>, out: &mut Vec<String>) {
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

/// One page a tree editor exposes: a section (one-part path) or a list item
/// (`[section?, list key, index]`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreePage {
    pub path: Vec<String>,
    pub title: String,
    pub search_terms: Vec<String>,
}

/// The pages a tree editor over `props` and `value` has, in its order:
/// every section first (when `sectioned`, see [`is_sectioned`]), then one
/// page per list item, filed under the list's section. Object and variant
/// keys are not path segments (a bar's overlay lines are
/// `["Text overlay", "lines", "0"]`), and lists inside list items are not
/// pages. The UI's schema editor registers exactly these paths, so a tree can
/// show an editor's nodes before the editor exists.
pub fn tree_pages(props: &[Prop], value: &Value, sectioned: bool) -> Vec<TreePage> {
    let mut out = Vec::new();
    if sectioned {
        let mut seen: Vec<&str> = Vec::new();
        for prop in props {
            let name = section_name(prop);
            if seen.contains(&name) {
                continue;
            }
            seen.push(name);
            let mut search_terms = Vec::new();
            collect_search_terms(
                props.iter().filter(|p| section_name(p) == name),
                &mut search_terms,
            );
            out.push(TreePage {
                path: vec![name.to_string()],
                title: name.to_string(),
                search_terms,
            });
        }
    }
    for prop in props {
        let base: Vec<String> = if sectioned {
            vec![section_name(prop).to_string()]
        } else {
            Vec::new()
        };
        collect_list_pages(prop, value.get(prop.key), &base, &mut out);
    }
    out
}

fn collect_list_pages(
    prop: &Prop,
    value: Option<&Value>,
    base: &[String],
    out: &mut Vec<TreePage>,
) {
    match &prop.kind {
        PropKind::List {
            item,
            item_label,
            item_summary,
            ..
        } => {
            let mut search_terms = Vec::new();
            collect_search_terms(item.iter(), &mut search_terms);
            let items = value
                .and_then(Value::as_array)
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            for (i, v) in items.iter().enumerate() {
                let mut path = base.to_vec();
                path.push(prop.key.to_string());
                path.push(i.to_string());
                out.push(TreePage {
                    path,
                    title: list_item_title(item_label, i, *item_summary, v),
                    search_terms: search_terms.clone(),
                });
            }
        }
        PropKind::Object { props } => {
            for p in props {
                collect_list_pages(p, value.and_then(|v| v.get(p.key)), base, out);
            }
        }
        PropKind::Variant { variants, .. } => {
            for variant in variants {
                for p in &variant.props {
                    collect_list_pages(p, value.and_then(|v| v.get(p.key)), base, out);
                }
            }
        }
        _ => {}
    }
}

/// The one failure a stringly schema can hide from the compiler: a key,
/// enum variant name, condition value or template that does not match the
/// type.
///
/// Walks the props over the serialized default. Every key must exist in the
/// object it belongs to; every enum option and `Condition::Equals` value is
/// substituted into the default and the whole type deserialized; an
/// `IndexChoice` default must be an integer, a `Color` default an object
/// with a `type`, a `ColorStops` default an array; `Object` props are checked
/// against the nested object; each `Variant` template must carry its tag and
/// its props' keys and must deserialize in place of the object, and its props
/// are checked against the template; a `List` default must be an array and
/// the template item must carry the item keys and deserialize as the list.
pub fn check_props_fit<T>(props: &[Prop]) -> Result<(), String>
where
    T: Default + Serialize + DeserializeOwned,
{
    let root = serde_json::to_value(T::default()).map_err(|e| e.to_string())?;
    if !root.is_object() {
        return Err("config does not serialize to an object".to_string());
    }
    let accept = |candidate: Value| -> Result<(), String> {
        serde_json::from_value::<T>(candidate)
            .map(|_| ())
            .map_err(|e| e.to_string())
    };
    check_object(props, &root, &[], &accept)
}

fn display_path(path: &[String]) -> String {
    if path.is_empty() {
        "the config".to_string()
    } else {
        format!("'{}'", path.join("."))
    }
}

fn get_at<'a>(root: &'a Value, path: &[String]) -> Option<&'a Value> {
    path.iter().try_fold(root, |v, seg| match v {
        Value::Array(a) => seg.parse::<usize>().ok().and_then(|i| a.get(i)),
        _ => v.get(seg),
    })
}

fn set_at(root: &mut Value, path: &[String], value: Value) -> Result<(), String> {
    let Some((last, parents)) = path.split_last() else {
        *root = value;
        return Ok(());
    };
    let mut cur = root;
    for seg in parents {
        cur = match cur {
            Value::Array(a) => seg
                .parse::<usize>()
                .ok()
                .and_then(|i| a.get_mut(i))
                .ok_or_else(|| format!("no element '{}'", seg))?,
            Value::Object(o) => o
                .get_mut(seg.as_str())
                .ok_or_else(|| format!("no key '{}'", seg))?,
            _ => return Err(format!("'{}' is not a container", seg)),
        };
    }
    match cur {
        Value::Array(a) => {
            let i = last
                .parse::<usize>()
                .map_err(|_| format!("'{}' is not an index", last))?;
            *a.get_mut(i)
                .ok_or_else(|| format!("no element '{}'", last))? = value;
        }
        Value::Object(o) => {
            o.insert(last.clone(), value);
        }
        _ => return Err(format!("'{}' is not a container", last)),
    }
    Ok(())
}

fn with_value_at(root: &Value, path: &[String], value: Value) -> Result<Value, String> {
    let mut candidate = root.clone();
    set_at(&mut candidate, path, value)?;
    Ok(candidate)
}

fn check_object(
    props: &[Prop],
    root: &Value,
    path: &[String],
    accept: &dyn Fn(Value) -> Result<(), String>,
) -> Result<(), String> {
    let object = get_at(root, path)
        .and_then(Value::as_object)
        .ok_or_else(|| format!("{} is not an object", display_path(path)))?;
    for prop in props {
        if matches!(prop.kind, PropKind::Note) {
            continue;
        }
        if !object.contains_key(prop.key) {
            return Err(format!(
                "prop key '{}' is not a field of {}",
                prop.key,
                display_path(path)
            ));
        }
        let mut here = path.to_vec();
        here.push(prop.key.to_string());
        let current = &object[prop.key];
        match &prop.kind {
            PropKind::Enum { options } => {
                for (value, _) in options {
                    accept(with_value_at(
                        root,
                        &here,
                        Value::String(value.to_string()),
                    )?)
                    .map_err(|e| {
                        format!(
                            "enum option '{}' is not a variant of {}: {}",
                            value,
                            display_path(&here),
                            e
                        )
                    })?;
                }
            }
            PropKind::IndexChoice { .. } => {
                if !(current.is_u64() || current.is_i64()) {
                    return Err(format!(
                        "index choice {} is not an integer",
                        display_path(&here)
                    ));
                }
            }
            PropKind::Color => {
                if current.get("type").is_none() {
                    return Err(format!("colour {} has no 'type'", display_path(&here)));
                }
            }
            PropKind::ColorStops {
                angle_key,
                theme_key,
                ..
            } => {
                // The flag is skipped when false, so the default need not
                // carry it; a present key must be a boolean.
                if let Some(k) = theme_key {
                    if !object.get(*k).map(Value::is_boolean).unwrap_or(true) {
                        return Err(format!(
                            "theme key '{}' of {} is not a boolean field",
                            k,
                            display_path(&here)
                        ));
                    }
                }
                if !current.is_array() {
                    return Err(format!(
                        "colour stops {} are not an array",
                        display_path(&here)
                    ));
                }
                if let Some(k) = angle_key {
                    if !object.get(*k).map(Value::is_number).unwrap_or(false) {
                        return Err(format!(
                            "angle key '{}' of {} is not a number field",
                            k,
                            display_path(&here)
                        ));
                    }
                }
            }
            PropKind::Font { legacy } => {
                if !current.is_null() && current.get("type").is_none() {
                    return Err(format!("font {} has no 'type'", display_path(&here)));
                }
                if let Some((family, size)) = legacy {
                    for k in [family, size] {
                        if !object.contains_key(*k) {
                            return Err(format!(
                                "legacy font key '{}' of {} is not a field",
                                k,
                                display_path(&here)
                            ));
                        }
                    }
                }
            }
            PropKind::Field => {
                if !current.is_string() {
                    return Err(format!("field id {} is not a string", display_path(&here)));
                }
            }
            PropKind::Rgba => {
                if !["r", "g", "b", "a"]
                    .iter()
                    .all(|k| current.get(*k).map(Value::is_number).unwrap_or(false))
                {
                    return Err(format!("colour {} is not r/g/b/a", display_path(&here)));
                }
            }
            PropKind::PlainFont { size_key } => {
                if !current.is_string() {
                    return Err(format!(
                        "font family {} is not a string",
                        display_path(&here)
                    ));
                }
                if !object.get(*size_key).map(Value::is_number).unwrap_or(false) {
                    return Err(format!(
                        "font size key '{}' of {} is not a number",
                        size_key,
                        display_path(&here)
                    ));
                }
            }
            PropKind::Object { props } => {
                check_object(props, root, &here, accept)?;
            }
            PropKind::Variant { tag, variants } => {
                let tagged = current
                    .get(*tag)
                    .and_then(Value::as_str)
                    .ok_or_else(|| format!("{} has no '{}' tag", display_path(&here), tag))?;
                if !variants.iter().any(|v| v.value == tagged) {
                    return Err(format!(
                        "default tag '{}' of {} is not a listed variant",
                        tagged,
                        display_path(&here)
                    ));
                }
                for variant in variants {
                    let template = (variant.template)();
                    let fields = template.as_object().ok_or_else(|| {
                        format!(
                            "template of variant '{}' of {} is not an object",
                            variant.value,
                            display_path(&here)
                        )
                    })?;
                    if fields.get(*tag).and_then(Value::as_str) != Some(variant.value) {
                        return Err(format!(
                            "template of variant '{}' of {} does not carry its tag",
                            variant.value,
                            display_path(&here)
                        ));
                    }
                    let candidate = with_value_at(root, &here, template.clone())?;
                    accept(candidate.clone()).map_err(|e| {
                        format!(
                            "template of variant '{}' of {} does not fit: {}",
                            variant.value,
                            display_path(&here),
                            e
                        )
                    })?;
                    check_object(&variant.props, &candidate, &here, accept)?;
                }
            }
            PropKind::EnumList {
                options,
                count_key,
                default_key,
                ..
            } => {
                if !current.is_array() {
                    return Err(format!("enum list {} is not an array", display_path(&here)));
                }
                if !object
                    .get(*count_key)
                    .map(Value::is_number)
                    .unwrap_or(false)
                {
                    return Err(format!(
                        "count key '{}' of {} is not a number",
                        count_key,
                        display_path(&here)
                    ));
                }
                if let Some(k) = default_key {
                    if !object.contains_key(*k) {
                        return Err(format!(
                            "default key '{}' of {} is not a field",
                            k,
                            display_path(&here)
                        ));
                    }
                }
                for (value, _) in options {
                    accept(with_value_at(
                        root,
                        &here,
                        Value::Array(vec![Value::String(value.to_string())]),
                    )?)
                    .map_err(|e| {
                        format!(
                            "enum list option '{}' is not a variant of {}: {}",
                            value,
                            display_path(&here),
                            e
                        )
                    })?;
                }
            }
            PropKind::List {
                item,
                new_item,
                count_key,
                ..
            } => {
                if !current.is_array() {
                    return Err(format!("list {} is not an array", display_path(&here)));
                }
                if let Some(k) = count_key {
                    if !object.get(*k).map(Value::is_number).unwrap_or(false) {
                        return Err(format!(
                            "count key '{}' of {} is not a number",
                            k,
                            display_path(&here)
                        ));
                    }
                }
                let template = new_item(0);
                if !template.is_object() {
                    return Err(format!(
                        "list {} template is not an object",
                        display_path(&here)
                    ));
                }
                let candidate = with_value_at(root, &here, Value::Array(vec![template]))?;
                accept(candidate.clone()).map_err(|e| {
                    format!("list {} template does not fit: {}", display_path(&here), e)
                })?;
                let mut first = here.clone();
                first.push("0".to_string());
                check_object(item, &candidate, &first, accept)?;
            }
            _ => {}
        }
        if let Some(cond) = &prop.visible_when {
            let key = cond.key();
            if !object.contains_key(key) {
                return Err(format!(
                    "condition key '{}' of prop '{}' is not a field of {}",
                    key,
                    prop.key,
                    display_path(path)
                ));
            }
            if let Condition::Equals { values, .. } = cond {
                let mut cond_path = path.to_vec();
                cond_path.push(key.to_string());
                for value in values {
                    accept(with_value_at(
                        root,
                        &cond_path,
                        Value::String(value.to_string()),
                    )?)
                    .map_err(|e| {
                        format!(
                            "condition value '{}' is not a variant of {} (prop '{}'): {}",
                            value,
                            display_path(&cond_path),
                            prop.key,
                            e
                        )
                    })?;
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[derive(Default, serde::Serialize, serde::Deserialize)]
    struct Ramp {
        stops: Vec<crate::color::ColorStop>,
        angle: f64,
    }

    #[test]
    fn colour_stops_angle_key_must_be_a_number_field() {
        use super::{check_props_fit, Prop, PropKind};
        let stops = |angle_key| {
            vec![Prop::new(
                "stops",
                "Stops",
                PropKind::ColorStops {
                    linear: true,
                    themed: false,
                    angle_key,
                    theme_key: None,
                },
            )]
        };
        check_props_fit::<Ramp>(&stops(Some("angle"))).unwrap();
        check_props_fit::<Ramp>(&stops(None)).unwrap();
        assert!(check_props_fit::<Ramp>(&stops(Some("missing"))).is_err());
        assert!(check_props_fit::<Ramp>(&stops(Some("stops"))).is_err());
    }

    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
    enum Mode {
        #[default]
        A,
        B,
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Item {
        id: String,
        text: String,
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Inner {
        on: bool,
        width: f64,
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    #[serde(tag = "type")]
    enum Fill {
        #[serde(rename = "solid")]
        Solid { level: u32 },
        #[serde(rename = "striped")]
        Striped { stripes: Vec<u32>, angle: f64 },
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct Cfg {
        mode: Mode,
        count: u32,
        index: usize,
        items: Vec<Item>,
        inner: Inner,
        fill: Fill,
    }

    impl Default for Cfg {
        fn default() -> Self {
            Self {
                mode: Mode::A,
                count: 1,
                index: 0,
                items: vec![Item {
                    id: "i1".into(),
                    text: "one".into(),
                }],
                inner: Inner {
                    on: true,
                    width: 1.0,
                },
                fill: Fill::Solid { level: 1 },
            }
        }
    }

    fn solid() -> Value {
        serde_json::json!({ "type": "solid", "level": 1 })
    }

    fn striped() -> Value {
        serde_json::json!({ "type": "striped", "stripes": [1, 2], "angle": 45.0 })
    }

    fn int(key: &'static str, label: &'static str) -> Prop {
        Prop::new(
            key,
            label,
            PropKind::Int {
                min: 0,
                max: 9,
                step: 1,
            },
        )
    }

    fn item(n: usize) -> Value {
        serde_json::json!({ "id": format!("i{}", n + 1), "text": "" })
    }

    fn good() -> Vec<Prop> {
        vec![
            Prop::new(
                "mode",
                "Mode",
                PropKind::Enum {
                    options: vec![("A", "a"), ("B", "b")],
                },
            ),
            Prop::new(
                "count",
                "Count",
                PropKind::Int {
                    min: 0,
                    max: 9,
                    step: 1,
                },
            )
            .visible_when(Condition::Equals {
                key: "mode",
                values: vec!["B"],
            }),
            Prop::new("index", "Index", PropKind::IndexChoice { provider: "p" }),
            Prop::new(
                "items",
                "Items",
                PropKind::List {
                    item: vec![
                        Prop::new("id", "Id", PropKind::Text { placeholder: "" }),
                        Prop::new("text", "Text", PropKind::Text { placeholder: "" }),
                    ],
                    new_item: item,
                    min_items: 1,
                    item_label: "Item",
                    add_label: "Add",
                    item_summary: None,
                    count_key: None,
                },
            ),
            Prop::note("a note"),
            Prop::new(
                "inner",
                "Inner",
                PropKind::Object {
                    props: vec![
                        Prop::new("on", "On", PropKind::Bool),
                        Prop::new(
                            "width",
                            "Width",
                            PropKind::Float {
                                min: 0.0,
                                max: 9.0,
                                step: 1.0,
                                digits: 1,
                            },
                        )
                        .visible_when(Condition::IsTrue { key: "on" }),
                    ],
                },
            ),
            Prop::new(
                "fill",
                "Fill",
                PropKind::Variant {
                    tag: "type",
                    variants: vec![
                        Variant {
                            value: "solid",
                            label: "Solid",
                            props: vec![int("level", "Level")],
                            template: solid,
                        },
                        Variant {
                            value: "striped",
                            label: "Striped",
                            props: vec![Prop::new(
                                "angle",
                                "Angle",
                                PropKind::Float {
                                    min: 0.0,
                                    max: 360.0,
                                    step: 1.0,
                                    digits: 0,
                                },
                            )],
                            template: striped,
                        },
                    ],
                },
            ),
        ]
    }

    #[test]
    fn accepts_a_fitting_schema() {
        check_props_fit::<Cfg>(&good()).unwrap();
    }

    #[test]
    fn rejects_a_condition_value_that_is_not_a_variant() {
        let mut props = good();
        props[1].visible_when = Some(Condition::Equals {
            key: "mode",
            values: vec!["C"],
        });
        let err = check_props_fit::<Cfg>(&props).unwrap_err();
        assert!(err.contains("condition value 'C'"), "{err}");
    }

    #[test]
    fn rejects_a_list_template_missing_an_item_key() {
        let mut props = good();
        if let PropKind::List { item, .. } = &mut props[3].kind {
            item.push(Prop::new(
                "nope",
                "Nope",
                PropKind::Text { placeholder: "" },
            ));
        }
        let err = check_props_fit::<Cfg>(&props).unwrap_err();
        assert!(err.contains("'nope' is not a field of 'items.0'"), "{err}");
    }

    #[test]
    fn rejects_a_nested_key_missing_from_the_object() {
        let mut props = good();
        if let PropKind::Object { props: inner } = &mut props[5].kind {
            inner.push(Prop::new("nope", "Nope", PropKind::Bool));
        }
        let err = check_props_fit::<Cfg>(&props).unwrap_err();
        assert!(err.contains("'nope' is not a field of 'inner'"), "{err}");
    }

    #[test]
    fn rejects_a_variant_template_that_does_not_fit() {
        let mut props = good();
        if let PropKind::Variant { variants, .. } = &mut props[6].kind {
            // "striped" without its stripes cannot deserialize
            variants[1].template = || serde_json::json!({ "type": "striped", "angle": 1.0 });
        }
        let err = check_props_fit::<Cfg>(&props).unwrap_err();
        assert!(err.contains("template of variant 'striped'"), "{err}");
    }

    #[test]
    fn rejects_a_variant_prop_key_missing_from_its_template() {
        let mut props = good();
        if let PropKind::Variant { variants, .. } = &mut props[6].kind {
            variants[0].props.push(int("nope", "Nope"));
        }
        let err = check_props_fit::<Cfg>(&props).unwrap_err();
        assert!(err.contains("'nope' is not a field of 'fill'"), "{err}");
    }

    #[test]
    fn rejects_an_index_choice_on_a_non_integer() {
        let mut props = good();
        props[2].key = "mode";
        let err = check_props_fit::<Cfg>(&props).unwrap_err();
        assert!(err.contains("not an integer"), "{err}");
    }
}
