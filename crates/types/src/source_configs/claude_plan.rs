//! Claude plan-usage source configuration types.

use serde::{Deserialize, Serialize};

fn default_update_interval() -> u64 {
    // The plan-usage endpoint is a network call against an account-wide figure
    // that changes slowly; poll gently to be a good citizen.
    60_000
}

/// Configuration for the Claude plan-usage source.
///
/// This source queries Anthropic's account usage endpoint (the same data the
/// `/usage` screen shows) using the local Claude Code OAuth token. It reports
/// usage *as a percentage of your plan limits*, not raw token counts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudePlanSourceConfig {
    /// How often to poll the usage endpoint, in milliseconds.
    #[serde(default = "default_update_interval")]
    pub update_interval_ms: u64,

    /// Optional caption override; falls back to an auto-generated label.
    #[serde(default)]
    pub custom_caption: Option<String>,
}

impl Default for ClaudePlanSourceConfig {
    fn default() -> Self {
        Self {
            update_interval_ms: default_update_interval(),
            custom_caption: None,
        }
    }
}
