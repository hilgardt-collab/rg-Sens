//! Declarative property schemas (Phase 3 of the config UI redesign).
//!
//! A config type that implements [`Schema`] describes its editable
//! properties as [`Prop`]s. The UI renders rows from them over the flat
//! JSON form of the config, so such a type needs no hand-written widget.
//! Keys are the serde field names; enum option values are the serde
//! variant names verbatim.

use serde::{de::DeserializeOwned, Serialize};

/// What kind of control a property needs, with its bounds.
#[derive(Debug, Clone, PartialEq)]
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
    /// (network interfaces, GPUs, …). The saved value is kept even when the
    /// provider does not list it.
    DynamicChoice {
        provider: &'static str,
    },
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

/// One editable property of a config type.
#[derive(Debug, Clone, PartialEq)]
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

/// The one failure a stringly schema can hide from the compiler: a key or
/// enum variant name that does not match the type. Checks that every prop
/// key exists in the serialized default and that every enum option
/// deserializes back into the type.
pub fn check_props_fit<T>(props: &[Prop]) -> Result<(), String>
where
    T: Default + Serialize + DeserializeOwned,
{
    let default = serde_json::to_value(T::default()).map_err(|e| e.to_string())?;
    let object = default
        .as_object()
        .ok_or_else(|| "config does not serialize to an object".to_string())?;
    for prop in props {
        if !object.contains_key(prop.key) {
            return Err(format!("prop key '{}' is not a field", prop.key));
        }
        if let PropKind::Enum { options } = &prop.kind {
            for (value, _) in options {
                let mut candidate = object.clone();
                candidate.insert(
                    prop.key.to_string(),
                    serde_json::Value::String(value.to_string()),
                );
                serde_json::from_value::<T>(serde_json::Value::Object(candidate)).map_err(|e| {
                    format!(
                        "enum option '{}' is not a variant of '{}': {}",
                        value, prop.key, e
                    )
                })?;
            }
        }
        if let Some(cond) = &prop.visible_when {
            let key = match cond {
                Condition::Equals { key, .. } | Condition::IsFalse { key } => key,
            };
            if !object.contains_key(*key) {
                return Err(format!(
                    "condition key '{}' of prop '{}' is not a field",
                    key, prop.key
                ));
            }
        }
    }
    Ok(())
}
