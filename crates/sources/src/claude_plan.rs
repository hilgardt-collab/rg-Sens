//! Claude plan-usage data source.
//!
//! Queries Anthropic's account usage endpoint (`/api/oauth/usage`) — the same
//! data the `/usage` screen shows — using the local Claude Code OAuth token, and
//! reports usage as a **percentage of your plan limits** (not raw token counts).
//!
//! ## Honesty & safety
//!
//! - This is an **undocumented** endpoint; Anthropic may change or remove it
//!   without notice, in which case this source goes "unavailable".
//! - The OAuth token is read **read-only** from `~/.claude/.credentials.json`.
//!   This source NEVER writes that file and NEVER refreshes the token: refresh
//!   tokens can rotate, and refreshing here could desync Claude Code's own login.
//!   When the access token expires (~24h) and Claude Code hasn't refreshed it,
//!   the call returns 401 and this source reports stale/unavailable until you
//!   next use Claude Code (which refreshes the token).
//! - It makes a network call on each update; poll gently (default 60s).

use rg_sens_core::{
    DataSource, FieldMetadata, FieldPurpose, FieldType, SourceConfig, SourceMetadata,
};
use rg_sens_types::source_configs::claude_plan::ClaudePlanSourceConfig;

use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

const USAGE_URL: &str = "https://api.anthropic.com/api/oauth/usage";
const OAUTH_BETA: &str = "oauth-2025-04-20";

/// Claude plan-usage source.
pub struct ClaudePlanSource {
    metadata: SourceMetadata,
    config: ClaudePlanSourceConfig,
    /// Last successfully fetched usage payload, retained so transient failures
    /// keep showing the most recent good numbers rather than blanking out.
    last_good: Option<Value>,
    /// Human-readable status for the most recent fetch attempt.
    status: String,
    values: HashMap<String, Value>,
}

impl ClaudePlanSource {
    pub fn new() -> Self {
        let metadata = SourceMetadata {
            id: "claude_plan".to_string(),
            name: "Claude Plan Usage".to_string(),
            description: "Account plan usage as a percentage of limits (session and weekly)"
                .to_string(),
            available_keys: vec![
                "caption".to_string(),
                "value".to_string(),
                "unit".to_string(),
                "session_pct".to_string(),
                "session_resets_at".to_string(),
                "session_minutes_left".to_string(),
                "weekly_pct".to_string(),
                "weekly_resets_at".to_string(),
                "weekly_minutes_left".to_string(),
                "weekly_opus_pct".to_string(),
                "weekly_sonnet_pct".to_string(),
                "status".to_string(),
            ],
            default_interval: Duration::from_millis(60_000),
        };

        Self {
            metadata,
            config: ClaudePlanSourceConfig::default(),
            last_good: None,
            status: "not yet fetched".to_string(),
            values: HashMap::with_capacity(16),
        }
    }

    pub fn set_config(&mut self, config: ClaudePlanSourceConfig) {
        self.config = config;
    }

    pub fn get_config(&self) -> &ClaudePlanSourceConfig {
        &self.config
    }

    /// Path to the Claude credentials file: `$CLAUDE_CONFIG_DIR/.credentials.json`
    /// or `$HOME/.claude/.credentials.json`.
    fn credentials_path() -> Option<PathBuf> {
        if let Ok(dir) = std::env::var("CLAUDE_CONFIG_DIR") {
            if !dir.is_empty() {
                return Some(PathBuf::from(dir).join(".credentials.json"));
            }
        }
        std::env::var("HOME")
            .ok()
            .map(|home| PathBuf::from(home).join(".claude").join(".credentials.json"))
    }

    /// Read the current OAuth access token (read-only; never written back).
    fn read_access_token() -> Option<String> {
        let path = Self::credentials_path()?;
        let content = std::fs::read_to_string(path).ok()?;
        let json: Value = serde_json::from_str(&content).ok()?;
        json.get("claudeAiOauth")?
            .get("accessToken")?
            .as_str()
            .map(|s| s.to_string())
    }

    /// Perform the blocking GET. `update()` is already run off the GTK thread by
    /// the update manager's `spawn_blocking`, so a blocking client is fine here.
    fn fetch(token: &str) -> Result<Value, String> {
        let resp = ureq::get(USAGE_URL)
            .set("Authorization", &format!("Bearer {token}"))
            .set("anthropic-beta", OAUTH_BETA)
            .set("Content-Type", "application/json")
            .timeout(Duration::from_secs(10))
            .call();

        match resp {
            Ok(r) => r
                .into_json::<Value>()
                .map_err(|e| format!("bad response body: {e}")),
            Err(ureq::Error::Status(code, _)) => Err(match code {
                401 | 403 => "token expired — open Claude Code to refresh".to_string(),
                _ => format!("HTTP {code}"),
            }),
            Err(e) => Err(format!("network error: {e}")),
        }
    }
}

impl Default for ClaudePlanSource {
    fn default() -> Self {
        Self::new()
    }
}

/// Read `block.utilization` as a float percentage, treating a missing/null block
/// as `None` (so callers can default it to 0).
fn block_pct(payload: &Value, key: &str) -> Option<f64> {
    payload.get(key)?.get("utilization")?.as_f64()
}

/// Read `block.resets_at` as an RFC3339 string.
fn block_resets_at<'a>(payload: &'a Value, key: &str) -> Option<&'a str> {
    payload.get(key)?.get("resets_at")?.as_str()
}

/// Minutes from now until `resets_at`, clamped to >= 0. `None` if unparseable.
fn minutes_until(resets_at: Option<&str>) -> Option<f64> {
    let ts = resets_at?;
    let target = chrono::DateTime::parse_from_rfc3339(ts).ok()?;
    let now = chrono::Utc::now();
    let mins = (target.timestamp_millis() - now.timestamp_millis()) as f64 / 60_000.0;
    Some(mins.max(0.0))
}

impl DataSource for ClaudePlanSource {
    fn metadata(&self) -> &SourceMetadata {
        &self.metadata
    }

    fn fields(&self) -> Vec<FieldMetadata> {
        vec![
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
                "Current session usage (% of limit)",
                FieldType::Numerical,
                FieldPurpose::Value,
            ),
            FieldMetadata::new(
                "unit",
                "Unit",
                "The unit of measurement",
                FieldType::Text,
                FieldPurpose::Unit,
            ),
            FieldMetadata::new(
                "session_pct",
                "Session Usage %",
                "Current 5-hour session usage as a percentage of the plan limit",
                FieldType::Numerical,
                FieldPurpose::Value,
            ),
            FieldMetadata::new(
                "session_minutes_left",
                "Session Resets In (min)",
                "Minutes until the current session limit resets",
                FieldType::Numerical,
                FieldPurpose::Value,
            ),
            FieldMetadata::new(
                "session_resets_at",
                "Session Resets At",
                "Timestamp when the current session limit resets",
                FieldType::Text,
                FieldPurpose::Other,
            ),
            FieldMetadata::new(
                "weekly_pct",
                "Weekly Usage %",
                "Weekly (7-day) usage as a percentage of the plan limit",
                FieldType::Numerical,
                FieldPurpose::Value,
            ),
            FieldMetadata::new(
                "weekly_minutes_left",
                "Weekly Resets In (min)",
                "Minutes until the weekly limit resets",
                FieldType::Numerical,
                FieldPurpose::Value,
            ),
            FieldMetadata::new(
                "weekly_resets_at",
                "Weekly Resets At",
                "Timestamp when the weekly limit resets",
                FieldType::Text,
                FieldPurpose::Other,
            ),
            FieldMetadata::new(
                "weekly_opus_pct",
                "Weekly Opus %",
                "Weekly Opus usage as a percentage of its limit (0 if not applicable)",
                FieldType::Numerical,
                FieldPurpose::Value,
            ),
            FieldMetadata::new(
                "weekly_sonnet_pct",
                "Weekly Sonnet %",
                "Weekly Sonnet usage as a percentage of its limit (0 if not applicable)",
                FieldType::Numerical,
                FieldPurpose::Value,
            ),
            FieldMetadata::new(
                "status",
                "Status",
                "Most recent fetch status (ok / error message)",
                FieldType::Text,
                FieldPurpose::Other,
            ),
        ]
    }

    fn update(&mut self) -> Result<()> {
        match Self::read_access_token() {
            Some(token) => match Self::fetch(&token) {
                Ok(payload) => {
                    self.last_good = Some(payload);
                    self.status = "ok".to_string();
                }
                Err(e) => {
                    self.status = e;
                    // Keep showing last_good if we have it.
                }
            },
            None => {
                self.status = "no credentials found".to_string();
            }
        }

        self.values.clear();

        let caption = self
            .config
            .custom_caption
            .clone()
            .unwrap_or_else(|| "Claude Session".to_string());
        self.values.insert("caption".to_string(), Value::from(caption));
        self.values.insert("unit".to_string(), Value::from("%"));
        self.values
            .insert("status".to_string(), Value::from(self.status.clone()));
        // Plan usage is a percentage, so gauges get a natural 0–100 scale.
        self.values.insert("min_limit".to_string(), Value::from(0.0));
        self.values.insert("max_limit".to_string(), Value::from(100.0));

        if let Some(payload) = &self.last_good {
            let session_pct = block_pct(payload, "five_hour").unwrap_or(0.0);
            let weekly_pct = block_pct(payload, "seven_day").unwrap_or(0.0);
            let opus_pct = block_pct(payload, "seven_day_opus").unwrap_or(0.0);
            let sonnet_pct = block_pct(payload, "seven_day_sonnet").unwrap_or(0.0);

            let session_reset = block_resets_at(payload, "five_hour");
            let weekly_reset = block_resets_at(payload, "seven_day");

            self.values
                .insert("value".to_string(), Value::from(session_pct));
            self.values
                .insert("session_pct".to_string(), Value::from(session_pct));
            self.values
                .insert("weekly_pct".to_string(), Value::from(weekly_pct));
            self.values
                .insert("weekly_opus_pct".to_string(), Value::from(opus_pct));
            self.values
                .insert("weekly_sonnet_pct".to_string(), Value::from(sonnet_pct));

            self.values.insert(
                "session_resets_at".to_string(),
                Value::from(session_reset.unwrap_or("")),
            );
            self.values.insert(
                "weekly_resets_at".to_string(),
                Value::from(weekly_reset.unwrap_or("")),
            );
            self.values.insert(
                "session_minutes_left".to_string(),
                Value::from(minutes_until(session_reset).unwrap_or(0.0)),
            );
            self.values.insert(
                "weekly_minutes_left".to_string(),
                Value::from(minutes_until(weekly_reset).unwrap_or(0.0)),
            );
        } else {
            // No good data yet — expose zeros so displayers render something.
            for key in [
                "value",
                "session_pct",
                "weekly_pct",
                "weekly_opus_pct",
                "weekly_sonnet_pct",
                "session_minutes_left",
                "weekly_minutes_left",
            ] {
                self.values.insert(key.to_string(), Value::from(0.0));
            }
            self.values
                .insert("session_resets_at".to_string(), Value::from(""));
            self.values
                .insert("weekly_resets_at".to_string(), Value::from(""));
        }

        Ok(())
    }

    fn get_values(&self) -> HashMap<String, Value> {
        self.values.clone()
    }

    fn values_ref(&self) -> Option<&HashMap<String, Value>> {
        Some(&self.values)
    }

    fn is_available(&self) -> bool {
        Self::credentials_path().is_some_and(|p| p.exists())
    }

    fn configure(&mut self, config: &HashMap<String, Value>) -> Result<()> {
        if let Some(cfg_value) = config.get("claude_plan_config") {
            if let Ok(cfg) = serde_json::from_value::<ClaudePlanSourceConfig>(cfg_value.clone()) {
                self.set_config(cfg);
            }
        }
        Ok(())
    }

    fn get_typed_config(&self) -> Option<SourceConfig> {
        Some(SourceConfig::ClaudePlan(self.config.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A trimmed copy of a real `/api/oauth/usage` response, used to pin the
    // schema mapping so a future endpoint change is caught by a failing test.
    const SAMPLE: &str = r#"{
        "five_hour": { "utilization": 5.0, "resets_at": "2026-06-24T20:49:59.494649+00:00" },
        "seven_day": { "utilization": 40.0, "resets_at": "2026-06-26T10:59:59.494671+00:00" },
        "seven_day_opus": null,
        "seven_day_sonnet": { "utilization": 0.0, "resets_at": null }
    }"#;

    #[test]
    fn maps_real_payload_to_percentages() {
        let payload: Value = serde_json::from_str(SAMPLE).unwrap();
        assert_eq!(block_pct(&payload, "five_hour"), Some(5.0));
        assert_eq!(block_pct(&payload, "seven_day"), Some(40.0));
        // Null block → None (callers default to 0).
        assert_eq!(block_pct(&payload, "seven_day_opus"), None);
        assert_eq!(block_pct(&payload, "seven_day_sonnet"), Some(0.0));
        assert_eq!(
            block_resets_at(&payload, "five_hour"),
            Some("2026-06-24T20:49:59.494649+00:00")
        );
        // resets_at present but null → None.
        assert_eq!(block_resets_at(&payload, "seven_day_sonnet"), None);
    }

    // Live end-to-end check; ignored by default (needs network + valid token).
    // Run with: cargo test -p rg-sens-sources live_fetch -- --ignored --nocapture
    #[test]
    #[ignore]
    fn live_fetch_populates_session_pct() {
        let mut src = ClaudePlanSource::new();
        src.update().unwrap();
        let v = src.get_values();
        eprintln!("status = {:?}", v.get("status"));
        eprintln!("session_pct = {:?}", v.get("session_pct"));
        eprintln!("weekly_pct  = {:?}", v.get("weekly_pct"));
        eprintln!("session_resets_at = {:?}", v.get("session_resets_at"));
        assert_eq!(v.get("status").and_then(Value::as_str), Some("ok"));
        assert!(v.get("session_pct").and_then(Value::as_f64).is_some());
    }

    #[test]
    fn minutes_until_is_nonnegative_and_zero_for_past() {
        // A timestamp far in the past must clamp to 0, never go negative.
        assert_eq!(minutes_until(Some("2000-01-01T00:00:00+00:00")), Some(0.0));
        assert_eq!(minutes_until(None), None);
        assert_eq!(minutes_until(Some("not-a-date")), None);
    }
}
