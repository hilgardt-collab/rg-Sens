//! Claude usage source configuration types.

use serde::{Deserialize, Serialize};

fn default_update_interval() -> u64 {
    // The plan-usage endpoint is a network call against a slowly-changing,
    // account-wide figure, so poll gently by default.
    60_000
}

fn default_session_hours() -> f64 {
    // Claude plan-usage "sessions" are rolling 5-hour blocks.
    5.0
}

/// Which Claude usage metric the source should surface as its primary `value`.
///
/// The source always exposes every field (so a displayer's field picker can
/// bind any of them); this selector only chooses the default `value`/`unit`/
/// caption and the gauge scale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ClaudeMetric {
    /// Current 5-hour session usage as a percentage of the plan limit.
    #[default]
    SessionUsage,
    /// Weekly (7-day) usage as a percentage of the plan limit.
    WeeklyUsage,
    /// Weekly Opus usage as a percentage of its limit.
    WeeklyOpusUsage,
    /// Weekly Sonnet usage as a percentage of its limit.
    WeeklySonnetUsage,
    /// Local token total in the current session window (all model families).
    SessionTokens,
    /// Local token total across all transcripts (all model families).
    AllTimeTokens,
    /// Minutes until the current 5-hour session limit resets (counts down).
    SessionResetIn,
    /// Minutes until the weekly limit resets (counts down).
    WeeklyResetIn,
}

impl ClaudeMetric {
    /// Stable order for UI dropdowns, paired with a human label.
    pub const ALL: [(ClaudeMetric, &'static str); 8] = [
        (ClaudeMetric::SessionUsage, "Session Usage (%)"),
        (ClaudeMetric::WeeklyUsage, "Weekly Usage (%)"),
        (ClaudeMetric::WeeklyOpusUsage, "Weekly Opus (%)"),
        (ClaudeMetric::WeeklySonnetUsage, "Weekly Sonnet (%)"),
        (ClaudeMetric::SessionTokens, "Session Tokens"),
        (ClaudeMetric::AllTimeTokens, "All-Time Tokens"),
        (ClaudeMetric::SessionResetIn, "Session Resets In (min)"),
        (ClaudeMetric::WeeklyResetIn, "Weekly Resets In (min)"),
    ];

    /// True for the percentage-of-limit metrics (0–100 scale, "%" unit).
    pub fn is_percentage(self) -> bool {
        matches!(
            self,
            ClaudeMetric::SessionUsage
                | ClaudeMetric::WeeklyUsage
                | ClaudeMetric::WeeklyOpusUsage
                | ClaudeMetric::WeeklySonnetUsage
        )
    }

    /// True for the minutes-until-reset metrics (countdown, "min" unit). These
    /// scale a gauge against their *window length*, not the token running-max.
    pub fn is_reset_time(self) -> bool {
        matches!(self, ClaudeMetric::SessionResetIn | ClaudeMetric::WeeklyResetIn)
    }
}

/// Configuration for the unified Claude usage source.
///
/// Combines two local data paths behind one source:
/// - **Plan usage** (`SessionUsage` / `WeeklyUsage` / per-model): percentage of
///   plan limits, fetched live from Anthropic's `/api/oauth/usage` endpoint
///   using the read-only local OAuth token.
/// - **Token counts** (`SessionTokens` / `AllTimeTokens`): raw tokens parsed
///   from local `~/.claude` transcripts, bucketed by model family.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeSourceConfig {
    /// Which metric drives the primary `value`.
    #[serde(default)]
    pub metric: ClaudeMetric,

    /// How often to refresh, in milliseconds.
    #[serde(default = "default_update_interval")]
    pub update_interval_ms: u64,

    /// Optional caption override; falls back to an auto-generated label.
    #[serde(default)]
    pub custom_caption: Option<String>,

    /// Length of the local token "session" window in hours (Claude default 5).
    #[serde(default = "default_session_hours")]
    pub session_hours: f64,

    /// Optional fixed maximum for token gauges; `None` auto-tracks the largest
    /// session total seen. (Percentage metrics always use 0–100.)
    #[serde(default)]
    pub max_limit: Option<f64>,
}

impl Default for ClaudeSourceConfig {
    fn default() -> Self {
        Self {
            metric: ClaudeMetric::default(),
            update_interval_ms: default_update_interval(),
            custom_caption: None,
            session_hours: default_session_hours(),
            max_limit: None,
        }
    }
}
