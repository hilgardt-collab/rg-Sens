//! Static Text data source implementation
//!
//! Provides configurable static text lines for custom text overlays.

use rg_sens_core::{DataSource, FieldMetadata, FieldPurpose, FieldType, SourceMetadata};
use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;

// Re-export static text source config types from rg-sens-types
pub use rg_sens_types::source_configs::static_text::{StaticTextLine, StaticTextSourceConfig};

/// Static Text data source
///
/// Provides configurable static text lines for custom text overlays.
/// This source is useful for labels, titles, or any static text content.
pub struct StaticTextSource {
    metadata: SourceMetadata,
    config: StaticTextSourceConfig,

    /// Cached output values - updated in update(), returned by reference in values_ref()
    values: HashMap<String, Value>,
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
            values: HashMap::with_capacity(8),
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
        // Build values HashMap (reuse allocation, just clear and refill)
        self.values.clear();

        // Ensure we have valid config - use default if lines is empty
        let lines = if self.config.lines.is_empty() {
            vec![StaticTextLine::default()]
        } else {
            self.config.lines.clone()
        };

        // Set caption
        let caption = self.config.custom_caption.clone().unwrap_or_else(|| {
            lines
                .first()
                .map(|l| l.label.clone())
                .unwrap_or_else(|| "Static Text".to_string())
        });
        self.values
            .insert("caption".to_string(), Value::from(caption.clone()));

        // Set primary value to first line's text
        let value_text = lines
            .first()
            .map(|l| l.text.clone())
            .unwrap_or_else(|| "Static Text".to_string());
        self.values
            .insert("value".to_string(), Value::from(value_text.clone()));

        // Unit is empty for static text
        self.values.insert("unit".to_string(), Value::from(""));

        // Add all configured lines as their field_id
        for line in &lines {
            self.values
                .insert(line.field_id.clone(), Value::from(line.text.clone()));
        }

        // Set min/max limits for compatibility with displayers that expect them
        self.values
            .insert("min_limit".to_string(), Value::from(0.0));
        self.values
            .insert("max_limit".to_string(), Value::from(100.0));

        Ok(())
    }

    fn get_values(&self) -> HashMap<String, Value> {
        self.values.clone()
    }

    fn values_ref(&self) -> Option<&HashMap<String, Value>> {
        Some(&self.values)
    }

    fn configure(&mut self, config: &HashMap<String, Value>) -> Result<()> {
        // Look for static_text_config in the configuration
        if let Some(static_text_config_value) = config.get("static_text_config") {
            if let Ok(static_text_config) =
                serde_json::from_value::<StaticTextSourceConfig>(static_text_config_value.clone())
            {
                self.set_config(static_text_config);
            }
        }
        Ok(())
    }

    fn get_typed_config(&self) -> Option<rg_sens_core::SourceConfig> {
        Some(rg_sens_core::SourceConfig::StaticText(self.config.clone()))
    }
}
