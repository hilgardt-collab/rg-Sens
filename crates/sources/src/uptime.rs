//! Uptime data source: how long the system has been running.

use anyhow::Result;
use rg_sens_core::{DataSource, FieldMetadata, FieldPurpose, FieldType, SourceMetadata};
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;

pub use rg_sens_types::source_configs::uptime::{
    uptime_compact, uptime_words, UptimeFormat, UptimeSourceConfig,
};

/// Seconds since boot. `/proc/uptime` is one small read; sysinfo is the
/// fallback where it does not exist.
fn uptime_seconds() -> u64 {
    std::fs::read_to_string("/proc/uptime")
        .ok()
        .and_then(|s| s.split_whitespace().next()?.parse::<f64>().ok())
        .map(|s| s as u64)
        .unwrap_or_else(sysinfo::System::uptime)
}

pub struct UptimeSource {
    metadata: SourceMetadata,
    config: UptimeSourceConfig,
    values: HashMap<String, Value>,
}

impl UptimeSource {
    pub fn new() -> Self {
        Self {
            metadata: SourceMetadata {
                id: "uptime".to_string(),
                name: "Uptime".to_string(),
                description: "How long the system has been running".to_string(),
                available_keys: [
                    "caption", "value", "unit", "text", "compact", "seconds", "days", "hours",
                    "minutes",
                ]
                .iter()
                .map(|k| k.to_string())
                .collect(),
                default_interval: Duration::from_millis(10_000),
            },
            config: UptimeSourceConfig::default(),
            values: HashMap::with_capacity(12),
        }
    }

    fn fill(&mut self, seconds: u64) {
        let (words, compact) = (uptime_words(seconds), uptime_compact(seconds));
        let value = match self.config.format {
            UptimeFormat::Words => words.clone(),
            UptimeFormat::Compact => compact.clone(),
        };
        let caption = self
            .config
            .custom_caption
            .clone()
            .filter(|c| !c.is_empty())
            .unwrap_or_else(|| "Uptime".to_string());
        let v = &mut self.values;
        v.clear();
        v.insert("caption".to_string(), Value::from(caption));
        v.insert("value".to_string(), Value::from(value));
        v.insert("unit".to_string(), Value::from(""));
        v.insert("text".to_string(), Value::from(words));
        v.insert("compact".to_string(), Value::from(compact));
        v.insert("seconds".to_string(), Value::from(seconds));
        v.insert("days".to_string(), Value::from(seconds / 86_400));
        v.insert("hours".to_string(), Value::from(seconds / 3_600 % 24));
        v.insert("minutes".to_string(), Value::from(seconds / 60 % 60));
    }
}

impl Default for UptimeSource {
    fn default() -> Self {
        Self::new()
    }
}

impl DataSource for UptimeSource {
    fn metadata(&self) -> &SourceMetadata {
        &self.metadata
    }

    fn fields(&self) -> Vec<FieldMetadata> {
        let text = |id: &str, name: &str, description: &str, purpose| {
            FieldMetadata::new(id, name, description, FieldType::Text, purpose)
        };
        let number = |id: &str, name: &str, description: &str| {
            FieldMetadata::new(
                id,
                name,
                description,
                FieldType::Numerical,
                FieldPurpose::Value,
            )
        };
        vec![
            text(
                "caption",
                "Caption",
                "Label for the uptime",
                FieldPurpose::Caption,
            ),
            text(
                "value",
                "Uptime",
                "Uptime in the configured format",
                FieldPurpose::Value,
            ),
            text(
                "unit",
                "Unit",
                "Unit (empty for uptime)",
                FieldPurpose::Unit,
            ),
            text(
                "text",
                "In words",
                "Such as 4 days, 7 hours",
                FieldPurpose::Value,
            ),
            text(
                "compact",
                "Compact",
                "Such as 4d 07:12",
                FieldPurpose::Value,
            ),
            number("seconds", "Seconds", "Total seconds since boot"),
            number("days", "Days", "Whole days since boot"),
            number("hours", "Hours", "Hours beyond the whole days (0-23)"),
            number(
                "minutes",
                "Minutes",
                "Minutes beyond the whole hours (0-59)",
            ),
        ]
    }

    fn update(&mut self) -> Result<()> {
        self.fill(uptime_seconds());
        Ok(())
    }

    fn get_values(&self) -> HashMap<String, Value> {
        self.values.clone()
    }

    fn values_ref(&self) -> Option<&HashMap<String, Value>> {
        Some(&self.values)
    }

    fn configure(&mut self, config: &HashMap<String, Value>) -> Result<()> {
        if let Some(value) = config.get("uptime_config") {
            if let Ok(parsed) = serde_json::from_value::<UptimeSourceConfig>(value.clone()) {
                self.config = parsed;
            }
        }
        Ok(())
    }

    fn get_typed_config(&self) -> Option<rg_sens_core::SourceConfig> {
        Some(rg_sens_core::SourceConfig::Uptime(self.config.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_follow_the_configured_format() {
        let mut source = UptimeSource::new();
        let seconds = 4 * 86_400 + 7 * 3_600 + 12 * 60 + 30;
        source.fill(seconds);
        assert_eq!(source.values["value"], "4 days, 7 hours");
        assert_eq!(source.values["compact"], "4d 07:12");
        assert_eq!(source.values["seconds"], seconds);
        assert_eq!(source.values["hours"], 7);

        let mut config = HashMap::new();
        config.insert(
            "uptime_config".to_string(),
            serde_json::json!({"format": "Compact", "custom_caption": "Up"}),
        );
        source.configure(&config).unwrap();
        source.fill(seconds);
        assert_eq!(source.values["value"], "4d 07:12");
        assert_eq!(source.values["caption"], "Up");
    }

    #[test]
    fn reads_a_real_uptime() {
        let mut source = UptimeSource::new();
        source.update().unwrap();
        assert!(source.values["seconds"].as_u64().unwrap() > 0);
    }
}
