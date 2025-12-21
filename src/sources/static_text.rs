//! Static Text data source implementation
//!
//! Provides configurable static text lines for custom text overlays.

use crate::core::{DataSource, FieldMetadata, FieldPurpose, FieldType, SourceMetadata};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;

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

/// Configuration for the static text source
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StaticTextSourceConfig {
    /// Configurable text lines
    pub lines: Vec<StaticTextLine>,
    /// Update interval in milliseconds (default: 1000ms, static text doesn't need frequent updates)
    #[serde(default = "default_update_interval")]
    pub update_interval_ms: u64,
    /// Custom caption for the source
    #[serde(default)]
    pub custom_caption: Option<String>,
}

fn default_update_interval() -> u64 {
    1000
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

/// Static Text data source
///
/// Provides configurable static text lines for custom text overlays.
/// This source is useful for labels, titles, or any static text content.
pub struct StaticTextSource {
    metadata: SourceMetadata,
    config: StaticTextSourceConfig,
}

impl StaticTextSource {
    pub fn new() -> Self {
        let metadata = SourceMetadata {
            id: "static_text".to_string(),
            name: "Static Text".to_string(),
            description: "Configurable static text lines for custom overlays".to_string(),
            available_keys: vec![
                "caption".to_string(),
                "value".to_string(),
                "unit".to_string(),
            ],
            default_interval: Duration::from_millis(1000),
        };

        Self {
            metadata,
            config: StaticTextSourceConfig::default(),
        }
    }

    /// Set configuration
    pub fn set_config(&mut self, config: StaticTextSourceConfig) {
        // Ensure at least one line exists
        let config = if config.lines.is_empty() {
            log::warn!("StaticTextSource::set_config - received empty lines, using default");
            StaticTextSourceConfig {
                lines: vec![StaticTextLine::default()],
                ..config
            }
        } else {
            config
        };

        self.config = config;
        // Update available keys based on configured lines
        self.metadata.available_keys = vec![
            "caption".to_string(),
            "value".to_string(),
            "unit".to_string(),
        ];
        for line in &self.config.lines {
            if !self.metadata.available_keys.contains(&line.field_id) {
                self.metadata.available_keys.push(line.field_id.clone());
            }
        }
    }

    /// Get current configuration
    pub fn get_config(&self) -> &StaticTextSourceConfig {
        &self.config
    }
}

impl Default for StaticTextSource {
    fn default() -> Self {
        Self::new()
    }
}

impl DataSource for StaticTextSource {
    fn metadata(&self) -> &SourceMetadata {
        &self.metadata
    }

    fn fields(&self) -> Vec<FieldMetadata> {
        let mut fields = vec![
            FieldMetadata::new(
                "caption",
                "Caption",
                "Display caption (auto-generated or custom)",
                FieldType::Text,
                FieldPurpose::Caption,
            ),
            FieldMetadata::new(
                "value",
                "Value",
                "The primary text value (first line)",
                FieldType::Text,
                FieldPurpose::Value,
            ),
            FieldMetadata::new(
                "unit",
                "Unit",
                "Unit (empty for static text)",
                FieldType::Text,
                FieldPurpose::Unit,
            ),
        ];

        // Add fields for each configured line
        for line in &self.config.lines {
            fields.push(FieldMetadata::new(
                &line.field_id,
                &line.label,
                format!("Static text: {}", line.label),
                FieldType::Text,
                FieldPurpose::Value,
            ));
        }

        fields
    }

    fn update(&mut self) -> Result<()> {
        // Static text doesn't need to update from external sources
        Ok(())
    }

    fn get_values(&self) -> HashMap<String, Value> {
        let mut values = HashMap::new();

        // Ensure we have valid config - use default if lines is empty
        let lines = if self.config.lines.is_empty() {
            vec![StaticTextLine::default()]
        } else {
            self.config.lines.clone()
        };

        // Set caption
        let caption = self.config.custom_caption
            .clone()
            .unwrap_or_else(|| {
                lines.first()
                    .map(|l| l.label.clone())
                    .unwrap_or_else(|| "Static Text".to_string())
            });
        values.insert("caption".to_string(), Value::from(caption.clone()));

        // Set primary value to first line's text
        let value_text = lines.first()
            .map(|l| l.text.clone())
            .unwrap_or_else(|| "Static Text".to_string());
        values.insert("value".to_string(), Value::from(value_text.clone()));

        // Unit is empty for static text
        values.insert("unit".to_string(), Value::from(""));

        // Add all configured lines as their field_id
        for line in &lines {
            values.insert(line.field_id.clone(), Value::from(line.text.clone()));
        }

        // Set min/max limits for compatibility with displayers that expect them
        values.insert("min_limit".to_string(), Value::from(0.0));
        values.insert("max_limit".to_string(), Value::from(100.0));

        values
    }

    fn configure(&mut self, config: &HashMap<String, Value>) -> Result<()> {
        // Look for static_text_config in the configuration
        if let Some(static_text_config_value) = config.get("static_text_config") {
            if let Ok(static_text_config) = serde_json::from_value::<StaticTextSourceConfig>(static_text_config_value.clone()) {
                self.set_config(static_text_config);
            }
        }
        Ok(())
    }

    fn get_typed_config(&self) -> Option<crate::core::SourceConfig> {
        Some(crate::core::SourceConfig::StaticText(self.config.clone()))
    }
}
