//! Claude Code token-usage source configuration types.

use serde::{Deserialize, Serialize};

fn default_update_interval() -> u64 {
    // Token totals change slowly relative to system metrics, and parsing
    // transcripts touches the filesystem, so default to a relaxed cadence.
    10_000
}

fn default_session_hours() -> f64 {
    // Claude plan-usage "sessions" are rolling 5-hour blocks.
    5.0
}

/// Configuration for the Claude Code token-usage source.
///
/// This source reads local Claude Code transcripts (`~/.claude/projects/**.jsonl`)
/// and reports token usage for the current rolling session and all-time, bucketed
/// by model family. It is a *local* proxy: it only sees Claude Code usage on this
/// machine, not account-wide plan usage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeSourceConfig {
    /// How often to re-scan transcripts, in milliseconds.
    #[serde(default = "default_update_interval")]
    pub update_interval_ms: u64,

    /// Optional caption override; falls back to an auto-generated label.
    #[serde(default)]
    pub custom_caption: Option<String>,

    /// Length of a usage "session" window in hours (Claude default is 5).
    #[serde(default = "default_session_hours")]
    pub session_hours: f64,

    /// Optional fixed maximum for gauges; when `None`, the source auto-tracks
    /// the largest session total it has seen.
    #[serde(default)]
    pub max_limit: Option<f64>,
}

impl Default for ClaudeSourceConfig {
    fn default() -> Self {
        Self {
            update_interval_ms: default_update_interval(),
            custom_caption: None,
            session_hours: default_session_hours(),
            max_limit: None,
        }
    }
}
