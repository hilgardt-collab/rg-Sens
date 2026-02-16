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
