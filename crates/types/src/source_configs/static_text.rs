//! Static text source configuration types.

use serde::{Deserialize, Serialize};

/// A single configurable text line
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StaticTextLine {
    /// Field ID used as key in get_values() (e.g., "line1", "line2")
    pub field_id: String,
    /// The actual text content to display
    pub text: String,
    /// Human-readable label for UI
    pub label: String,
}

impl Default for StaticTextLine {
    fn default() -> Self {
        Self {
            field_id: "line1".to_string(),
            text: "Static Text".to_string(),
            label: "Line 1".to_string(),
        }
    }
}

fn default_update_interval() -> u64 {
    1000
}

/// Configuration for the static text source
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StaticTextSourceConfig {
    /// Configurable text lines
    pub lines: Vec<StaticTextLine>,
    /// Update interval in milliseconds
    #[serde(default = "default_update_interval")]
    pub update_interval_ms: u64,
    /// Custom caption for the source
    #[serde(default)]
    pub custom_caption: Option<String>,
}

impl Default for StaticTextSourceConfig {
    fn default() -> Self {
        Self {
            lines: vec![StaticTextLine::default()],
            update_interval_ms: default_update_interval(),
            custom_caption: None,
        }
    }
}

/// Template for a new line appended as the `n`th item (0-based).
fn new_line(n: usize) -> serde_json::Value {
    let number = n + 1;
    serde_json::json!({
        "field_id": format!("line{}", number),
        "text": format!("Line {}", number),
        "label": format!("Line {}", number),
    })
}

impl crate::schema::Schema for StaticTextSourceConfig {
    fn props() -> Vec<crate::schema::Prop> {
        use crate::schema::{Prop, PropKind};
        vec![
            Prop::new(
                "lines",
                "Text lines",
                PropKind::List {
                    item: vec![
                        Prop::new(
                            "text",
                            "Text",
                            PropKind::Text {
                                placeholder: "The text to display",
                            },
                        ),
                        Prop::new(
                            "label",
                            "Label",
                            PropKind::Text {
                                placeholder: "Human-readable name",
                            },
                        )
                        .help("Shown in displayer field pickers"),
                        Prop::new(
                            "field_id",
                            "Field ID",
                            PropKind::Text {
                                placeholder: "e.g., line1",
                            },
                        )
                        .help("Key displayers bind to; must be unique within the source"),
                    ],
                    new_item: new_line,
                    min_items: 1,
                    item_label: "Line",
                    add_label: "Add line",
                },
            )
            .help("The first line is the source's primary value"),
            Prop::new(
                "custom_caption",
                "Caption",
                PropKind::OptionalText {
                    placeholder: "(auto-generated from first line)",
                },
            ),
            Prop::new(
                "update_interval_ms",
                "Update interval (ms)",
                PropKind::Int {
                    min: 100,
                    max: 60000,
                    step: 100,
                },
            ),
        ]
    }
}

#[cfg(test)]
mod schema_tests {
    use super::*;
    use crate::schema::{check_props_fit, Schema};

    #[test]
    fn props_fit_the_type() {
        check_props_fit::<StaticTextSourceConfig>(&StaticTextSourceConfig::props()).unwrap();
    }
}
