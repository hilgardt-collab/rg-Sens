//! Claude Code token-usage data source.
//!
//! Reads local Claude Code transcripts (`~/.claude/projects/**/*.jsonl`) and
//! reports token usage for the current rolling session and all-time, bucketed by
//! model family (Opus / Sonnet / Haiku / Other).
//!
//! ## Scope & honesty
//!
//! This is a **local proxy**: it only sees Claude Code usage recorded on *this*
//! machine. It is not a mirror of the account-wide plan-usage screen, which is a
//! usage-against-a-limit figure, not a raw token count. Treat the numbers as
//! "Claude Code tokens seen locally in the last N hours", not billing truth.
//!
//! ## How usage is counted
//!
//! Transcripts log each assistant turn, sometimes repeating the same message
//! across streaming lines, so entries are de-duplicated by `message.id` +
//! `requestId`. The current-session figure reconstructs Claude's rolling
//! "session" the way the plan-usage screen does: a 5-hour block anchored at the
//! first message of the block (floored to the hour). See [`current_session`].

use rg_sens_core::{
    DataSource, FieldMetadata, FieldPurpose, FieldType, SourceConfig, SourceMetadata,
};
use rg_sens_types::source_configs::claude::ClaudeSourceConfig;

use anyhow::Result;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// Model families we bucket usage into. `Other` catches anything unrecognised.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Family {
    Opus,
    Sonnet,
    Haiku,
    Other,
}

impl Family {
    /// All families in a stable order, paired with their value-key suffix.
    const ALL: [(Family, &'static str); 4] = [
        (Family::Opus, "opus"),
        (Family::Sonnet, "sonnet"),
        (Family::Haiku, "haiku"),
        (Family::Other, "other"),
    ];

    fn index(self) -> usize {
        match self {
            Family::Opus => 0,
            Family::Sonnet => 1,
            Family::Haiku => 2,
            Family::Other => 3,
        }
    }

    /// Map a model id to a family. Returns `None` for pseudo-models (e.g.
    /// `<synthetic>`) that should not count toward usage.
    fn from_model(model: &str) -> Option<Family> {
        if model.is_empty() || model.starts_with('<') {
            return None;
        }
        if model.contains("opus") {
            Some(Family::Opus)
        } else if model.contains("sonnet") {
            Some(Family::Sonnet)
        } else if model.contains("haiku") {
            Some(Family::Haiku)
        } else {
            Some(Family::Other)
        }
    }
}

/// A single de-duplicated usage record.
#[derive(Debug, Clone, Copy)]
struct Entry {
    /// Unix timestamp in milliseconds.
    ts: i64,
    family: Family,
    tokens: u64,
}

/// Cached parse result for one transcript file, keyed by (mtime, len) so an
/// unchanged file is never re-read.
struct FileCache {
    mtime: SystemTime,
    len: u64,
    /// All-time token totals for this file, indexed by `Family::index`.
    alltime: [u64; 4],
    /// De-duplicated entries, retained for session-window reconstruction.
    entries: Vec<Entry>,
}

/// Token usage as logged in a transcript `usage` object.
struct Usage {
    input: u64,
    output: u64,
    cache_creation: u64,
    cache_read: u64,
}

/// Decide how many tokens a single assistant turn contributes to the totals.
///
/// ─────────────────────────────────────────────────────────────────────────
/// LEARNING-MODE CONTRIBUTION — this is the one genuine judgement call here.
///
/// A Claude Code turn reports four token counts:
///   • `input`          — fresh prompt tokens sent this turn
///   • `output`         — tokens generated this turn
///   • `cache_creation` — prompt tokens written into the prompt cache
///   • `cache_read`     — prompt tokens served from the cache (cheap, and the
///                        single largest number in a typical Claude Code turn)
///
/// What should "tokens used" mean for this sensor? Options:
///   (a) Everything:      input + output + cache_creation + cache_read
///                        → biggest number; dominated by cache reads.
///   (b) Billable-ish:    input + output + cache_creation
///                        → excludes cheap cache hits; closer to "work done".
///   (c) Generation only: input + output
///                        → ignores all caching.
///
/// The default below is (a). Change this body to (b)/(c) — or weight the terms —
/// if you prefer a different definition. Whatever you pick is applied uniformly
/// to both the session and all-time figures.
/// ─────────────────────────────────────────────────────────────────────────
fn count_tokens(u: &Usage) -> u64 {
    u.input + u.output + u.cache_creation + u.cache_read
}

/// Claude Code token-usage source.
pub struct ClaudeSource {
    metadata: SourceMetadata,
    config: ClaudeSourceConfig,

    /// Per-file parse cache, keyed by absolute path.
    file_cache: HashMap<PathBuf, FileCache>,
    /// Largest session total seen so far, used for auto gauge limits.
    running_max: f64,

    values: HashMap<String, Value>,
}

impl ClaudeSource {
    pub fn new() -> Self {
        let mut available_keys = vec![
            "caption".to_string(),
            "value".to_string(),
            "unit".to_string(),
            "session_total".to_string(),
            "session_active".to_string(),
            "alltime_total".to_string(),
        ];
        for (_, suffix) in Family::ALL {
            available_keys.push(format!("session_{suffix}"));
            available_keys.push(format!("alltime_{suffix}"));
        }

        let metadata = SourceMetadata {
            id: "claude".to_string(),
            name: "Claude Tokens".to_string(),
            description: "Local Claude Code token usage (current session and all-time)".to_string(),
            available_keys,
            default_interval: Duration::from_millis(10_000),
        };

        Self {
            metadata,
            config: ClaudeSourceConfig::default(),
            file_cache: HashMap::new(),
            running_max: 0.0,
            values: HashMap::with_capacity(24),
        }
    }

    pub fn set_config(&mut self, config: ClaudeSourceConfig) {
        self.config = config;
    }

    pub fn get_config(&self) -> &ClaudeSourceConfig {
        &self.config
    }

    /// Locate the Claude config directory: `$CLAUDE_CONFIG_DIR` or `$HOME/.claude`.
    fn projects_dir() -> Option<PathBuf> {
        if let Ok(dir) = std::env::var("CLAUDE_CONFIG_DIR") {
            if !dir.is_empty() {
                return Some(PathBuf::from(dir).join("projects"));
            }
        }
        std::env::var("HOME")
            .ok()
            .map(|home| PathBuf::from(home).join(".claude").join("projects"))
    }

    /// Collect every `*.jsonl` transcript under `root` (one level of project
    /// subdirectories, plus any nested ones).
    fn collect_transcripts(root: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = fs::read_dir(root) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                Self::collect_transcripts(&path, out);
            } else if path.extension().is_some_and(|e| e == "jsonl") {
                out.push(path);
            }
        }
    }

    /// Parse one transcript file into all-time totals + de-duplicated entries.
    fn parse_file(path: &Path) -> ParsedFile {
        let mut alltime = [0u64; 4];
        let mut entries = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();

        let Ok(content) = fs::read_to_string(path) else {
            return ParsedFile { alltime, entries };
        };

        for line in content.lines() {
            if line.is_empty() {
                continue;
            }
            let Ok(obj) = serde_json::from_str::<Value>(line) else {
                continue;
            };
            let message = match obj.get("message") {
                Some(Value::Object(m)) => m,
                _ => continue,
            };
            let usage = match message.get("usage") {
                Some(Value::Object(u)) => u,
                _ => continue,
            };

            // Skip pseudo-models and bucket by family.
            let model = message.get("model").and_then(Value::as_str).unwrap_or("");
            let Some(family) = Family::from_model(model) else {
                continue;
            };

            // De-duplicate streaming repeats by message id + request id.
            let id = message.get("id").and_then(Value::as_str).unwrap_or("");
            let req = obj.get("requestId").and_then(Value::as_str).unwrap_or("");
            if !id.is_empty() {
                let key = format!("{id}:{req}");
                if !seen.insert(key) {
                    continue;
                }
            }

            let u = Usage {
                input: usage_field(usage, "input_tokens"),
                output: usage_field(usage, "output_tokens"),
                cache_creation: usage_field(usage, "cache_creation_input_tokens"),
                cache_read: usage_field(usage, "cache_read_input_tokens"),
            };
            let tokens = count_tokens(&u);
            alltime[family.index()] += tokens;

            if let Some(ts) = obj
                .get("timestamp")
                .and_then(Value::as_str)
                .and_then(parse_ts_millis)
            {
                entries.push(Entry { ts, family, tokens });
            }
        }

        ParsedFile { alltime, entries }
    }

    /// Refresh `file_cache`: re-parse only files whose (mtime, len) changed, and
    /// drop entries for files that no longer exist.
    fn refresh_cache(&mut self, transcripts: &[PathBuf]) {
        let present: HashSet<&PathBuf> = transcripts.iter().collect();
        self.file_cache.retain(|path, _| present.contains(path));

        for path in transcripts {
            let Ok(meta) = fs::metadata(path) else {
                continue;
            };
            let mtime = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
            let len = meta.len();

            if let Some(cached) = self.file_cache.get(path) {
                if cached.mtime == mtime && cached.len == len {
                    continue; // unchanged — reuse cache
                }
            }

            let parsed = Self::parse_file(path);
            self.file_cache.insert(
                path.clone(),
                FileCache {
                    mtime,
                    len,
                    alltime: parsed.alltime,
                    entries: parsed.entries,
                },
            );
        }
    }
}

/// Intermediate parse result (avoids constructing a `FileCache` without metadata).
struct ParsedFile {
    alltime: [u64; 4],
    entries: Vec<Entry>,
}

fn usage_field(usage: &serde_json::Map<String, Value>, key: &str) -> u64 {
    usage.get(key).and_then(Value::as_u64).unwrap_or(0)
}

fn parse_ts_millis(ts: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(ts)
        .ok()
        .map(|dt| dt.timestamp_millis())
}

/// Reconstruct the current usage "session" from de-duplicated entries.
///
/// Mirrors how Claude's plan-usage screen blocks usage: a session is a fixed
/// `window`-long block anchored at the first message of the block (floored to
/// the hour). A new block starts when an entry falls `window` or more past the
/// current block's start, or `window` or more after the previous entry. The
/// current session is the most recent block, but only while `now` is still
/// inside it — once the window elapses with no new activity, the session has
/// reset and the totals are zero.
///
/// Returns per-family totals (indexed by `Family::index`) and whether a session
/// is currently active.
fn current_session(entries: &[Entry], now: i64, window: i64) -> ([u64; 4], bool) {
    if entries.is_empty() || window <= 0 {
        return ([0; 4], false);
    }

    // Entries arrive per-file; sort ascending by timestamp before blocking.
    let mut sorted: Vec<Entry> = entries.to_vec();
    sorted.sort_by_key(|e| e.ts);

    let floor_hour = |ts: i64| ts - ts.rem_euclid(3_600_000);

    let mut block_start = floor_hour(sorted[0].ts);
    let mut block_sums = [0u64; 4];
    let mut last_ts = sorted[0].ts;

    for e in &sorted {
        if e.ts - block_start >= window || e.ts - last_ts >= window {
            // Gap too large — begin a fresh block.
            block_start = floor_hour(e.ts);
            block_sums = [0; 4];
        }
        block_sums[e.family.index()] += e.tokens;
        last_ts = e.ts;
    }

    let active = now - block_start < window;
    if active {
        (block_sums, true)
    } else {
        ([0; 4], false)
    }
}

impl Default for ClaudeSource {
    fn default() -> Self {
        Self::new()
    }
}

impl DataSource for ClaudeSource {
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
                "Current session total tokens (all models)",
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
                "session_total",
                "Session Total",
                "Tokens used in the current rolling session (all models)",
                FieldType::Numerical,
                FieldPurpose::Value,
            ),
            FieldMetadata::new(
                "alltime_total",
                "All-Time Total",
                "Tokens used across all local Claude Code sessions (all models)",
                FieldType::Numerical,
                FieldPurpose::Value,
            ),
            FieldMetadata::new(
                "session_active",
                "Session Active",
                "1 if a usage session is currently active, else 0",
                FieldType::Numerical,
                FieldPurpose::Other,
            ),
        ];

        for (family, suffix) in Family::ALL {
            let name = match family {
                Family::Opus => "Opus",
                Family::Sonnet => "Sonnet",
                Family::Haiku => "Haiku",
                Family::Other => "Other",
            };
            fields.push(FieldMetadata::new(
                format!("session_{suffix}"),
                format!("Session {name}"),
                "Session tokens for this model family",
                FieldType::Numerical,
                FieldPurpose::Value,
            ));
            fields.push(FieldMetadata::new(
                format!("alltime_{suffix}"),
                format!("All-Time {name}"),
                "All-time tokens for this model family",
                FieldType::Numerical,
                FieldPurpose::Value,
            ));
        }

        fields
    }

    fn update(&mut self) -> Result<()> {
        let mut transcripts = Vec::new();
        if let Some(dir) = Self::projects_dir() {
            Self::collect_transcripts(&dir, &mut transcripts);
        }
        self.refresh_cache(&transcripts);

        // Aggregate all-time totals and gather entries for the session window.
        let mut alltime = [0u64; 4];
        let window_ms = (self.config.session_hours * 3_600_000.0) as i64;
        let now = chrono::Utc::now().timestamp_millis();
        let session_cutoff = now - window_ms.saturating_mul(2);

        let mut session_entries: Vec<Entry> = Vec::new();
        for cache in self.file_cache.values() {
            for (i, total) in cache.alltime.iter().enumerate() {
                alltime[i] += total;
            }
            // Only entries that could fall in (or near) the window matter.
            session_entries.extend(cache.entries.iter().filter(|e| e.ts >= session_cutoff));
        }

        let (session, active) = current_session(&session_entries, now, window_ms);

        let session_total: u64 = session.iter().sum();
        let alltime_total: u64 = alltime.iter().sum();

        // Track a running max so gauges have a sensible scale by default.
        self.running_max = self.running_max.max(session_total as f64);
        let max_limit = self
            .config
            .max_limit
            .unwrap_or_else(|| self.running_max.max(1.0));

        let caption = self
            .config
            .custom_caption
            .clone()
            .unwrap_or_else(|| "Claude Tokens".to_string());

        self.values.clear();
        self.values.insert("caption".to_string(), Value::from(caption));
        self.values
            .insert("value".to_string(), Value::from(session_total));
        self.values
            .insert("unit".to_string(), Value::from("tokens"));
        self.values
            .insert("session_total".to_string(), Value::from(session_total));
        self.values
            .insert("alltime_total".to_string(), Value::from(alltime_total));
        self.values.insert(
            "session_active".to_string(),
            Value::from(if active { 1.0 } else { 0.0 }),
        );

        for (family, suffix) in Family::ALL {
            let i = family.index();
            self.values
                .insert(format!("session_{suffix}"), Value::from(session[i]));
            self.values
                .insert(format!("alltime_{suffix}"), Value::from(alltime[i]));
        }

        self.values
            .insert("min_limit".to_string(), Value::from(0.0));
        self.values
            .insert("max_limit".to_string(), Value::from(max_limit));

        Ok(())
    }

    fn get_values(&self) -> HashMap<String, Value> {
        self.values.clone()
    }

    fn values_ref(&self) -> Option<&HashMap<String, Value>> {
        Some(&self.values)
    }

    fn is_available(&self) -> bool {
        Self::projects_dir().is_some_and(|p| p.exists())
    }

    fn configure(&mut self, config: &HashMap<String, Value>) -> Result<()> {
        if let Some(claude_config_value) = config.get("claude_config") {
            if let Ok(claude_config) =
                serde_json::from_value::<ClaudeSourceConfig>(claude_config_value.clone())
            {
                self.set_config(claude_config);
            }
        }
        Ok(())
    }

    fn get_typed_config(&self) -> Option<SourceConfig> {
        Some(SourceConfig::Claude(self.config.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOUR: i64 = 3_600_000;
    const WINDOW: i64 = 5 * HOUR;

    fn e(hours: i64, family: Family, tokens: u64) -> Entry {
        Entry {
            ts: hours * HOUR,
            family,
            tokens,
        }
    }

    #[test]
    fn families_bucket_by_substring_and_skip_pseudo_models() {
        assert_eq!(Family::from_model("claude-opus-4-8"), Some(Family::Opus));
        assert_eq!(Family::from_model("claude-sonnet-4-6"), Some(Family::Sonnet));
        assert_eq!(
            Family::from_model("claude-haiku-4-5-20251001"),
            Some(Family::Haiku)
        );
        assert_eq!(Family::from_model("some-future-model"), Some(Family::Other));
        assert_eq!(Family::from_model("<synthetic>"), None);
        assert_eq!(Family::from_model(""), None);
    }

    #[test]
    fn active_session_sums_only_the_current_block() {
        // Block A at hour 0-1, then a >5h gap, block B at hours 10-11.
        let entries = [
            e(0, Family::Opus, 100),
            e(1, Family::Opus, 50),
            e(10, Family::Opus, 7),
            e(11, Family::Sonnet, 3),
        ];
        // "now" sits inside block B's window (block B anchors at hour 10).
        let (sums, active) = current_session(&entries, 11 * HOUR, WINDOW);
        assert!(active);
        assert_eq!(sums[Family::Opus.index()], 7);
        assert_eq!(sums[Family::Sonnet.index()], 3);
        assert_eq!(sums.iter().sum::<u64>(), 10);
    }

    #[test]
    fn session_expires_once_the_window_elapses() {
        let entries = [e(0, Family::Opus, 100), e(1, Family::Opus, 50)];
        // Block anchors at hour 0, resets at hour 5; "now" is hour 8.
        let (sums, active) = current_session(&entries, 8 * HOUR, WINDOW);
        assert!(!active);
        assert_eq!(sums.iter().sum::<u64>(), 0);
    }

    #[test]
    fn empty_input_is_inactive() {
        let (sums, active) = current_session(&[], 5 * HOUR, WINDOW);
        assert!(!active);
        assert_eq!(sums.iter().sum::<u64>(), 0);
    }
}
