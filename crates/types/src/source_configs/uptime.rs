//! Uptime source configuration and formatting.

use serde::{Deserialize, Serialize};

/// How the source's primary `value` is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum UptimeFormat {
    /// "4 days, 7 hours"
    #[default]
    Words,
    /// "4d 07:12"
    Compact,
}

fn default_update_interval() -> u64 {
    10_000
}

/// Configuration for the uptime source
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UptimeSourceConfig {
    /// Format of the primary value
    #[serde(default)]
    pub format: UptimeFormat,
    /// Custom caption for the source
    #[serde(default)]
    pub custom_caption: Option<String>,
    /// Update interval in milliseconds
    #[serde(default = "default_update_interval")]
    pub update_interval_ms: u64,
}

impl Default for UptimeSourceConfig {
    fn default() -> Self {
        Self {
            format: UptimeFormat::default(),
            custom_caption: None,
            update_interval_ms: default_update_interval(),
        }
    }
}

fn plural(n: u64, unit: &str) -> String {
    if n == 1 {
        format!("1 {unit}")
    } else {
        format!("{n} {unit}s")
    }
}

/// The two largest units, written out: "4 days, 7 hours", "7 hours, 12
/// minutes", "12 minutes" (a zero second unit is left out: "1 day").
pub fn uptime_words(seconds: u64) -> String {
    let (days, hours, minutes) = (seconds / 86_400, seconds / 3_600 % 24, seconds / 60 % 60);
    let (first, second) = if days > 0 {
        (
            plural(days, "day"),
            (hours > 0).then(|| plural(hours, "hour")),
        )
    } else if hours > 0 {
        (
            plural(hours, "hour"),
            (minutes > 0).then(|| plural(minutes, "minute")),
        )
    } else {
        (plural(minutes, "minute"), None)
    };
    match second {
        Some(second) => format!("{first}, {second}"),
        None => first,
    }
}

/// "4d 07:12", or "07:12" within the first day.
pub fn uptime_compact(seconds: u64) -> String {
    let (days, hours, minutes) = (seconds / 86_400, seconds / 3_600 % 24, seconds / 60 % 60);
    if days > 0 {
        format!("{days}d {hours:02}:{minutes:02}")
    } else {
        format!("{hours:02}:{minutes:02}")
    }
}

impl crate::schema::Schema for UptimeSourceConfig {
    fn props() -> Vec<crate::schema::Prop> {
        use crate::schema::{Prop, PropKind};
        vec![
            Prop::new(
                "custom_caption",
                "Caption",
                PropKind::OptionalText {
                    placeholder: "Uptime",
                },
            ),
            Prop::new(
                "format",
                "Format",
                PropKind::Enum {
                    options: vec![
                        ("Words", "Words (4 days, 7 hours)"),
                        ("Compact", "Compact (4d 07:12)"),
                    ],
                },
            )
            .help("How the primary value is written; both forms are also separate fields"),
            Prop::new(
                "update_interval_ms",
                "Update interval (ms)",
                PropKind::Int {
                    min: 1000,
                    max: 60000,
                    step: 1000,
                },
            ),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{check_props_fit, Schema};

    #[test]
    fn props_fit_the_type() {
        check_props_fit::<UptimeSourceConfig>(&UptimeSourceConfig::props()).unwrap();
    }

    #[test]
    fn words() {
        assert_eq!(uptime_words(0), "0 minutes");
        assert_eq!(uptime_words(61), "1 minute");
        assert_eq!(uptime_words(3_600), "1 hour");
        assert_eq!(uptime_words(7 * 3_600 + 12 * 60), "7 hours, 12 minutes");
        assert_eq!(uptime_words(86_400), "1 day");
        assert_eq!(
            uptime_words(4 * 86_400 + 7 * 3_600 + 12 * 60),
            "4 days, 7 hours"
        );
        assert_eq!(uptime_words(86_400 + 3_600), "1 day, 1 hour");
    }

    #[test]
    fn compact() {
        assert_eq!(uptime_compact(7 * 3_600 + 12 * 60 + 59), "07:12");
        assert_eq!(uptime_compact(4 * 86_400 + 7 * 3_600 + 12 * 60), "4d 07:12");
    }
}
