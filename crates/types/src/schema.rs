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
    },
    /// An editor the UI registers under this name (timezone picker, sensor
    /// picker, …). It may read and write any keys of the flat config, not
    /// only `key`.
    Custom {
        editor: &'static str,
    },
    /// Explanatory text with no value; `key` is unused (see [`Prop::note`]).
    Note,
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
}

impl Condition {
    fn key(&self) -> &'static str {
        match self {
            Condition::Equals { key, .. } | Condition::IsFalse { key } => key,
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

/// The one failure a stringly schema can hide from the compiler: a key,
/// enum variant name or condition value that does not match the type.
///
/// Checks that every prop key exists in the serialized default, that every
/// enum option and every `Condition::Equals` value deserializes back into
/// the type, that an `IndexChoice` default is an integer, and that a `List`
/// default is an array whose template item carries every item key and
/// deserializes.
pub fn check_props_fit<T>(props: &[Prop]) -> Result<(), String>
where
    T: Default + Serialize + DeserializeOwned,
{
    let default = serde_json::to_value(T::default()).map_err(|e| e.to_string())?;
    let object = default
        .as_object()
        .ok_or_else(|| "config does not serialize to an object".to_string())?;
    let accepts = |key: &str, value: Value| -> Result<(), String> {
        let mut candidate = object.clone();
        candidate.insert(key.to_string(), value);
        serde_json::from_value::<T>(Value::Object(candidate))
            .map(|_| ())
            .map_err(|e| e.to_string())
    };
    for prop in props {
        if matches!(prop.kind, PropKind::Note) {
            continue;
        }
        if !object.contains_key(prop.key) {
            return Err(format!("prop key '{}' is not a field", prop.key));
        }
        match &prop.kind {
            PropKind::Enum { options } => {
                for (value, _) in options {
                    accepts(prop.key, Value::String(value.to_string())).map_err(|e| {
                        format!(
                            "enum option '{}' is not a variant of '{}': {}",
                            value, prop.key, e
                        )
                    })?;
                }
            }
            PropKind::IndexChoice { .. } => {
                let v = &object[prop.key];
                if !(v.is_u64() || v.is_i64()) {
                    return Err(format!("index choice '{}' is not an integer", prop.key));
                }
            }
            PropKind::List { item, new_item, .. } => {
                if !object[prop.key].is_array() {
                    return Err(format!("list prop '{}' is not an array", prop.key));
                }
                let template = new_item(0);
                let fields = template
                    .as_object()
                    .ok_or_else(|| format!("list '{}' template is not an object", prop.key))?;
                for p in item {
                    if !matches!(p.kind, PropKind::Note) && !fields.contains_key(p.key) {
                        return Err(format!(
                            "item key '{}' of list '{}' is not in the template",
                            p.key, prop.key
                        ));
                    }
                }
                accepts(prop.key, Value::Array(vec![template]))
                    .map_err(|e| format!("list '{}' template does not fit: {}", prop.key, e))?;
            }
            _ => {}
        }
        if let Some(cond) = &prop.visible_when {
            let key = cond.key();
            if !object.contains_key(key) {
                return Err(format!(
                    "condition key '{}' of prop '{}' is not a field",
                    key, prop.key
                ));
            }
            if let Condition::Equals { values, .. } = cond {
                for value in values {
                    accepts(key, Value::String(value.to_string())).map_err(|e| {
                        format!(
                            "condition value '{}' is not a variant of '{}' (prop '{}'): {}",
                            value, key, prop.key, e
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

    #[derive(Debug, Serialize, Deserialize)]
    struct Cfg {
        mode: Mode,
        count: u32,
        index: usize,
        items: Vec<Item>,
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
            }
        }
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
                },
            ),
            Prop::note("a note"),
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
        assert!(err.contains("item key 'nope'"), "{err}");
    }

    #[test]
    fn rejects_an_index_choice_on_a_non_integer() {
        let mut props = good();
        props[2].key = "mode";
        let err = check_props_fit::<Cfg>(&props).unwrap_err();
        assert!(err.contains("not an integer"), "{err}");
    }
}
