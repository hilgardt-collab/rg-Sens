//! Unified Claude usage data source.
//!
//! Surfaces Claude usage through one source with a configurable metric. Two
//! local data paths sit behind it:
//!
//! - **Plan usage** — percentage of plan limits, fetched live from Anthropic's
//!   `/api/oauth/usage` endpoint using the local Claude Code OAuth token. This is
//!   the same data the `/usage` screen shows. The endpoint's current shape is a
//!   `limits` array (`kind` = `session` / `weekly_all` / `weekly_scoped`), which
//!   this source reads in preference to the older top-level `five_hour` /
//!   `seven_day` blocks (kept as a fallback). The fixed per-model weekly blocks
//!   (`seven_day_opus` / `seven_day_sonnet`) were retired: Anthropic now reports a
//!   single `weekly_scoped` limit tagged with whichever model the plan currently
//!   meters, exposed as `weekly_scoped_pct` + `weekly_scoped_model`.
//! - **Token counts** — raw tokens parsed from local `~/.claude/projects/**.jsonl`
//!   transcripts, de-duplicated and bucketed by model family.
//!
//! ## Honesty & safety
//!
//! - The usage endpoint is **undocumented**; Anthropic may change/remove it,
//!   after which the percentage metrics report `status` errors.
//! - The OAuth token is read **read-only** from `~/.claude/.credentials.json`.
//!   This source NEVER writes that file and NEVER refreshes the token (refresh
//!   tokens can rotate; refreshing here could desync Claude Code's own login).
//!   On 401 the percentages go stale until Claude Code refreshes the token.
//! - Token counts are a *local* proxy: only Claude Code usage on this machine.

use rg_sens_core::{
    DataSource, FieldMetadata, FieldPurpose, FieldType, SourceConfig, SourceMetadata,
};
use rg_sens_types::source_configs::claude::{ClaudeMetric, ClaudeSourceConfig};

use anyhow::Result;
use once_cell::sync::Lazy;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime};

const USAGE_URL: &str = "https://api.anthropic.com/api/oauth/usage";
const OAUTH_BETA: &str = "oauth-2025-04-20";
/// Minimum wall-clock between live usage fetches, regardless of update interval,
/// so a fast panel refresh can't hammer the endpoint. The endpoint's quota is
/// small and unpublished: polling every 30s got the source locked out with a
/// `Retry-After` of ~56 minutes. The figures are percentages of 5-hour / 7-day
/// windows, so a few minutes of staleness is invisible on a panel.
const MIN_FETCH_INTERVAL: Duration = Duration::from_secs(300);

/// Retry delay after a fetch that got no HTTP response (no credentials, DNS,
/// connect, timeout). Those cost the endpoint nothing, so retry sooner than
/// `MIN_FETCH_INTERVAL` — e.g. the app autostarting before the network is up.
const NO_RESPONSE_RETRY_INTERVAL: Duration = Duration::from_secs(30);

/// Back-off after an HTTP 429 that carried no usable `Retry-After`, and the
/// floor applied to one that did.
const RATE_LIMIT_BACKOFF: Duration = Duration::from_secs(300);

/// Ceiling on a server-supplied `Retry-After`, so a bogus header can't park the
/// source for days.
const MAX_RATE_LIMIT_BACKOFF: Duration = Duration::from_secs(2 * 3600);

/// Why a usage fetch produced no payload. Each variant is retried on its own
/// schedule — see `plan_usage`.
enum FetchError {
    /// HTTP 429, with the wait the server asked for in `Retry-After` (if any).
    RateLimited(Option<Duration>),
    /// No HTTP response at all (no credentials, DNS, connect, timeout).
    NoResponse(String),
    /// Any other HTTP or body failure. The message is the `status` text.
    Http(String),
}

/// Process-wide cache for the plan-usage response, shared across every
/// `ClaudeSource` instance so that N panels make at most one fetch per
/// `MIN_FETCH_INTERVAL` (per-instance throttling alone is not enough — the
/// endpoint rate-limits aggressively).
struct UsageCache {
    /// Earliest instant a new fetch is allowed; `None` means "fetch now".
    next_allowed: Option<Instant>,
    payload: Option<Value>,
    status: String,
}

static USAGE_CACHE: Lazy<Mutex<UsageCache>> = Lazy::new(|| {
    Mutex::new(UsageCache {
        next_allowed: None,
        payload: None,
        status: "not yet fetched".to_string(),
    })
});

/// Model families we bucket token usage into. `Other` catches anything else.
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

/// A single de-duplicated token-usage record.
#[derive(Debug, Clone, Copy)]
struct Entry {
    /// Unix timestamp in milliseconds.
    ts: i64,
    family: Family,
    tokens: u64,
}

/// Cached parse result for one transcript file, keyed by (mtime, len) so an
/// unchanged file is never re-read and a grown (append-only) file is only
/// tail-parsed from the previously cached length.
struct FileCache {
    mtime: SystemTime,
    /// Bytes of the file actually consumed by parsing (tail parses resume here).
    len: u64,
    /// All-time token totals for this file, indexed by `Family::index`.
    alltime: [u64; 4],
    /// De-duplicated entries, retained for session-window reconstruction.
    entries: Vec<Entry>,
    /// Dedup keys (`"id:req"`) claimed by this file, kept so appended tails
    /// and *other* files (resumed/branched sessions duplicate lines across
    /// transcripts) can de-duplicate against it.
    seen: HashSet<String>,
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
/// LEARNING-MODE CONTRIBUTION — the one genuine judgement call for token counts.
///
/// A Claude Code turn reports four token counts:
///   • `input`          — fresh prompt tokens sent this turn
///   • `output`         — tokens generated this turn
///   • `cache_creation` — prompt tokens written into the prompt cache
///   • `cache_read`     — prompt tokens served from the cache (cheap, and the
///                        single largest number in a typical Claude Code turn)
///
/// What should "tokens used" mean? Options:
///   (a) Everything:      input + output + cache_creation + cache_read
///   (b) Billable-ish:    input + output + cache_creation   (ignore cheap hits)
///   (c) Generation only: input + output
///
/// Default is (a). Change this body to (b)/(c) — or weight the terms — to pick a
/// different definition; it applies to both the session and all-time figures.
/// ─────────────────────────────────────────────────────────────────────────
fn count_tokens(u: &Usage) -> u64 {
    u.input + u.output + u.cache_creation + u.cache_read
}

/// Unified Claude usage source.
pub struct ClaudeSource {
    metadata: SourceMetadata,
    config: ClaudeSourceConfig,

    // --- token-count state (local transcripts) ---
    /// Per-file parse cache, keyed by absolute path.
    file_cache: HashMap<PathBuf, FileCache>,
    /// Largest session token total seen so far, for auto gauge limits.
    running_max: f64,

    values: HashMap<String, Value>,
}

impl ClaudeSource {
    pub fn new() -> Self {
        let mut available_keys = vec![
            "caption".to_string(),
            "value".to_string(),
            "unit".to_string(),
            // plan usage
            "session_pct".to_string(),
            "weekly_pct".to_string(),
            "weekly_scoped_pct".to_string(),
            "weekly_scoped_model".to_string(),
            "extra_usage_pct".to_string(),
            "weekly_opus_pct".to_string(),
            "weekly_sonnet_pct".to_string(),
            "session_resets_at".to_string(),
            "weekly_resets_at".to_string(),
            "weekly_scoped_resets_at".to_string(),
            "session_minutes_left".to_string(),
            "weekly_minutes_left".to_string(),
            "session_resets_in_text".to_string(),
            "weekly_resets_in_text".to_string(),
            "session_reset_day".to_string(),
            "weekly_reset_day".to_string(),
            "status".to_string(),
            // token counts
            "session_total".to_string(),
            "alltime_total".to_string(),
        ];
        for (_, suffix) in Family::ALL {
            available_keys.push(format!("session_{suffix}"));
            available_keys.push(format!("alltime_{suffix}"));
        }

        let metadata = SourceMetadata {
            id: "claude".to_string(),
            name: "Claude Usage".to_string(),
            description: "Claude plan usage (% of limits) and local token counts".to_string(),
            available_keys,
            default_interval: Duration::from_millis(60_000),
        };

        Self {
            metadata,
            config: ClaudeSourceConfig::default(),
            file_cache: HashMap::new(),
            running_max: 0.0,
            values: HashMap::with_capacity(32),
        }
    }

    pub fn set_config(&mut self, config: ClaudeSourceConfig) {
        self.config = config;
    }

    pub fn get_config(&self) -> &ClaudeSourceConfig {
        &self.config
    }

    // ----- token counts (local transcripts) -----

    /// Locate the Claude config dir: `$CLAUDE_CONFIG_DIR` or `$HOME/.claude`.
    fn claude_dir() -> Option<PathBuf> {
        if let Ok(dir) = std::env::var("CLAUDE_CONFIG_DIR") {
            if !dir.is_empty() {
                return Some(PathBuf::from(dir));
            }
        }
        std::env::var("HOME")
            .ok()
            .map(|home| PathBuf::from(home).join(".claude"))
    }

    /// Collect every `*.jsonl` transcript under `root` recursively. Directory
    /// symlinks are not followed and depth is capped, so a symlink cycle (or a
    /// pathological tree) can't recurse until the stack overflows.
    fn collect_transcripts(root: &Path, out: &mut Vec<PathBuf>, depth: u32) {
        const MAX_DEPTH: u32 = 10;
        if depth > MAX_DEPTH {
            return;
        }
        let Ok(entries) = fs::read_dir(root) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            // symlink_metadata does not follow symlinks, so a symlinked
            // directory fails is_dir() here and is skipped (cycle guard).
            let Ok(meta) = fs::symlink_metadata(&path) else {
                continue;
            };
            if meta.is_dir() {
                Self::collect_transcripts(&path, out, depth + 1);
            } else if path.extension().is_some_and(|e| e == "jsonl") && path.is_file() {
                out.push(path);
            }
        }
    }

    /// Parse transcript content starting at byte `offset` into `cache`.
    ///
    /// De-duplication spans the whole update pass: `pass_seen` holds every
    /// dedup key already claimed (by this file's cached head, by unchanged
    /// files, and by files parsed earlier in the pass), so lines duplicated
    /// across resumed/branched session transcripts count once. Newly claimed
    /// keys are recorded in both `cache.seen` and `pass_seen`.
    ///
    /// Returns the number of bytes consumed (offset + parsed bytes) so the
    /// next update can tail-parse from exactly where this one stopped. A
    /// torn (partially appended) final line is left unconsumed for the next
    /// pass. On read failure returns `offset` unchanged.
    fn parse_into(
        path: &Path,
        offset: u64,
        cache: &mut FileCache,
        pass_seen: &mut HashSet<String>,
    ) -> u64 {
        use std::io::{Read, Seek, SeekFrom};

        let mut content = String::new();
        let read_ok = (|| -> std::io::Result<()> {
            let mut file = fs::File::open(path)?;
            if offset > 0 {
                file.seek(SeekFrom::Start(offset))?;
            }
            file.read_to_string(&mut content)?;
            Ok(())
        })();
        if read_ok.is_err() {
            return offset;
        }

        // If the final line has no trailing newline it may be mid-append:
        // hold it back for the next pass unless it already parses as JSON
        // (i.e. the file legitimately ends without a newline).
        if !content.ends_with('\n') && !content.is_empty() {
            let tail_start = content.rfind('\n').map(|p| p + 1).unwrap_or(0);
            if serde_json::from_str::<Value>(&content[tail_start..]).is_err() {
                content.truncate(tail_start);
            }
        }
        let consumed = offset + content.len() as u64;

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

            let model = message.get("model").and_then(Value::as_str).unwrap_or("");
            let Some(family) = Family::from_model(model) else {
                continue;
            };

            // De-duplicate streaming repeats (and cross-file duplicates from
            // resumed/branched sessions) by message id + request id.
            let id = message.get("id").and_then(Value::as_str).unwrap_or("");
            let req = obj.get("requestId").and_then(Value::as_str).unwrap_or("");
            if !id.is_empty() {
                let key = format!("{id}:{req}");
                if pass_seen.contains(&key) {
                    continue;
                }
                cache.seen.insert(key.clone());
                pass_seen.insert(key);
            }

            let u = Usage {
                input: usage_field(usage, "input_tokens"),
                output: usage_field(usage, "output_tokens"),
                cache_creation: usage_field(usage, "cache_creation_input_tokens"),
                cache_read: usage_field(usage, "cache_read_input_tokens"),
            };
            let tokens = count_tokens(&u);
            cache.alltime[family.index()] += tokens;

            if let Some(ts) = obj
                .get("timestamp")
                .and_then(Value::as_str)
                .and_then(parse_ts_millis)
            {
                cache.entries.push(Entry { ts, family, tokens });
            }
        }

        consumed
    }

    /// Refresh `file_cache`: drop entries for files that no longer exist,
    /// tail-parse only the appended data of grown files (transcripts are
    /// append-only JSONL), and fully re-parse files that shrank or changed
    /// in place. Message-id de-duplication spans all files in the pass.
    fn refresh_cache(&mut self, transcripts: &[PathBuf]) {
        let present: HashSet<&PathBuf> = transcripts.iter().collect();
        self.file_cache.retain(|path, _| present.contains(path));

        // Classify the work first, dropping fully re-parsed files from the
        // cache immediately so their old dedup keys can't suppress their own
        // re-parse below. `offset > 0` means append-only tail parse.
        let mut work: Vec<(PathBuf, SystemTime, u64)> = Vec::new();
        for path in transcripts {
            let Ok(meta) = fs::metadata(path) else {
                continue;
            };
            let mtime = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
            let len = meta.len();
            match self.file_cache.get(path) {
                Some(cached) if cached.mtime == mtime && cached.len == len => continue,
                // Grown file: parse only the appended tail.
                Some(cached) if len > cached.len => work.push((path.clone(), mtime, cached.len)),
                // Shrunk or same-length-but-touched file: full re-parse.
                _ => {
                    self.file_cache.remove(path);
                    work.push((path.clone(), mtime, 0));
                }
            }
        }
        if work.is_empty() {
            return;
        }

        // Dedup keys already claimed by cached files (unchanged files and the
        // already-parsed heads of tail-parsed files).
        let mut pass_seen: HashSet<String> = self
            .file_cache
            .values()
            .flat_map(|c| c.seen.iter().cloned())
            .collect();

        for (path, mtime, offset) in work {
            let mut cache = self.file_cache.remove(&path).unwrap_or_else(|| FileCache {
                mtime,
                len: 0,
                alltime: [0; 4],
                entries: Vec::new(),
                seen: HashSet::new(),
            });
            cache.len = Self::parse_into(&path, offset, &mut cache, &mut pass_seen);
            cache.mtime = mtime;
            self.file_cache.insert(path, cache);
        }
    }

    // ----- plan usage (network) -----

    /// Path to the Claude credentials file (read-only).
    fn credentials_path() -> Option<PathBuf> {
        Self::claude_dir().map(|d| d.join(".credentials.json"))
    }

    /// Read a usable OAuth access token, preferring rg-Sens's own (which it can
    /// refresh itself) and falling back to Claude Code's read-only token.
    ///
    /// The fallback token is read but NEVER written or refreshed — refreshing it
    /// could rotate Claude Code's refresh token and break its login. Sign in via
    /// the source config tab so rg-Sens keeps its own refreshable token.
    fn read_access_token() -> Option<String> {
        if let Some(token) = crate::claude_auth::access_token() {
            return Some(token);
        }
        // No usable rg-Sens token. If the user IS signed in, access_token()
        // returning None means a refresh failed (transient) — log it, since the
        // silent fall-through to Claude Code's read-only token is otherwise hard
        // to diagnose.
        if crate::claude_auth::is_signed_in() {
            log::debug!(
                "claude: rg-Sens token unavailable (refresh failed?); falling back to Claude Code token"
            );
        }
        Self::read_claude_code_token()
    }

    /// Read Claude Code's OAuth access token read-only from its credentials file.
    fn read_claude_code_token() -> Option<String> {
        let path = Self::credentials_path()?;
        let content = fs::read_to_string(path).ok()?;
        let json: Value = serde_json::from_str(&content).ok()?;
        json.get("claudeAiOauth")?
            .get("accessToken")?
            .as_str()
            .map(|s| s.to_string())
    }

    /// Perform the blocking GET. `update()` runs off the GTK thread (the update
    /// manager wraps sources in `spawn_blocking`), so blocking I/O is fine here.
    fn fetch(token: &str) -> Result<Value, FetchError> {
        let resp = ureq::get(USAGE_URL)
            .set("Authorization", &format!("Bearer {token}"))
            .set("anthropic-beta", OAUTH_BETA)
            .set("Content-Type", "application/json")
            // Keep this BELOW the update loop's per-source critical threshold
            // (SOURCE_UPDATE_CRITICAL_THRESHOLD) so a slow-but-healthy fetch isn't
            // misclassified as a hung source and tripped into the circuit breaker.
            .timeout(Duration::from_secs(4))
            .call();

        match resp {
            Ok(r) => r
                .into_json::<Value>()
                .map_err(|e| FetchError::Http(format!("bad response body: {e}"))),
            Err(ureq::Error::Status(429, r)) => Err(FetchError::RateLimited(
                r.header("retry-after").and_then(parse_retry_after),
            )),
            Err(ureq::Error::Status(code, _)) => Err(FetchError::Http(match code {
                401 | 403 => "token expired — open Claude Code to refresh".to_string(),
                _ => format!("HTTP {code}"),
            })),
            Err(e) => Err(FetchError::NoResponse(format!("network error: {e}"))),
        }
    }

    /// Ensure the shared plan-usage cache is fresh and return a copy of the last
    /// good payload plus the latest status.
    ///
    /// `want_fetch` is false for token-only metrics, so a panel that never shows
    /// a percentage makes no network calls. The cache is still shared, so as
    /// long as *some* panel wants a percentage, all panels see fresh data.
    ///
    /// The blocking HTTP call happens **without holding the mutex**: the slot is
    /// claimed under lock (so siblings see "not due" and return cached data
    /// immediately), the guard is dropped, the fetch runs, then the lock is
    /// re-taken to store the result. This avoids stalling sibling panels for the
    /// duration of the request.
    fn plan_usage(want_fetch: bool) -> (Option<Value>, String) {
        let lock = || match USAGE_CACHE.lock() {
            Ok(c) => c,
            Err(p) => p.into_inner(), // poisoned: recover and carry on
        };

        // Phase 1: under lock, decide whether to fetch and claim the slot.
        {
            let mut cache = lock();
            let due = want_fetch
                && cache
                    .next_allowed
                    .map(|t| Instant::now() >= t)
                    .unwrap_or(true);
            if !due {
                return (cache.payload.clone(), cache.status.clone());
            }
            // Claim the slot so concurrent siblings don't also fetch.
            cache.next_allowed = Some(Instant::now() + MIN_FETCH_INTERVAL);
        }

        // Phase 2: fetch with the lock released.
        let result = match Self::read_access_token() {
            Some(token) => Self::fetch(&token),
            None => Err(FetchError::NoResponse("no credentials found".to_string())),
        };

        // Phase 3: re-lock and store the outcome. Failures keep the last good
        // payload and only move `next_allowed` + `status`.
        let mut cache = lock();
        match result {
            Ok(payload) => {
                cache.payload = Some(payload);
                cache.status = "ok".to_string();
                cache.next_allowed = Some(Instant::now() + MIN_FETCH_INTERVAL);
            }
            Err(FetchError::RateLimited(retry_after)) => {
                let backoff = rate_limit_backoff(retry_after);
                cache.next_allowed = Some(Instant::now() + backoff);
                // Report a wall-clock time rather than a countdown: the status
                // string is cached, so a countdown would freeze at this value.
                let retry_at =
                    chrono::Local::now() + chrono::Duration::seconds(backoff.as_secs() as i64);
                cache.status = format!("rate limited — retrying at {}", retry_at.format("%H:%M"));
                log::warn!(
                    "claude: usage endpoint rate limited (Retry-After: {:?}); next fetch in {}s",
                    retry_after.map(|d| d.as_secs()),
                    backoff.as_secs()
                );
            }
            Err(FetchError::NoResponse(e)) => {
                cache.next_allowed = Some(Instant::now() + NO_RESPONSE_RETRY_INTERVAL);
                cache.status = e;
            }
            Err(FetchError::Http(e)) => {
                cache.next_allowed = Some(Instant::now() + MIN_FETCH_INTERVAL);
                cache.status = e;
            }
        }
        (cache.payload.clone(), cache.status.clone())
    }
}

/// Parse a `Retry-After` header in its delta-seconds form (what the usage
/// endpoint sends). The HTTP-date form is not handled → `None`.
fn parse_retry_after(value: &str) -> Option<Duration> {
    value.trim().parse::<u64>().ok().map(Duration::from_secs)
}

/// How long to stay away from the usage endpoint after an HTTP 429.
///
/// The server's `Retry-After` is honoured when present: the lockout ends at a
/// fixed time (observed waits of ~56 min, and the unlock time did not move
/// across repeated rejected requests), so an earlier retry is just a wasted
/// request. It is clamped to
/// `[RATE_LIMIT_BACKOFF, MAX_RATE_LIMIT_BACKOFF]`; with no usable header the
/// floor is used.
fn rate_limit_backoff(retry_after: Option<Duration>) -> Duration {
    retry_after
        .unwrap_or(RATE_LIMIT_BACKOFF)
        .clamp(RATE_LIMIT_BACKOFF, MAX_RATE_LIMIT_BACKOFF)
}

fn usage_field(usage: &serde_json::Map<String, Value>, key: &str) -> u64 {
    usage.get(key).and_then(Value::as_u64).unwrap_or(0)
}

fn parse_ts_millis(ts: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(ts)
        .ok()
        .map(|dt| dt.timestamp_millis())
}

/// Read `block.utilization` as a float percentage; missing/null block → `None`.
fn block_pct(payload: &Value, key: &str) -> Option<f64> {
    payload.get(key)?.get("utilization")?.as_f64()
}

/// Read `block.resets_at` as an RFC3339 string.
fn block_resets_at<'a>(payload: &'a Value, key: &str) -> Option<&'a str> {
    payload.get(key)?.get("resets_at")?.as_str()
}

/// Find an entry in the newer `limits` array by its `kind`
/// (`"session"`, `"weekly_all"`, `"weekly_scoped"`).
fn limit_entry<'a>(payload: &'a Value, kind: &str) -> Option<&'a Value> {
    payload
        .get("limits")?
        .as_array()?
        .iter()
        .find(|l| l.get("kind").and_then(Value::as_str) == Some(kind))
}

/// Percentage (0–100) for a `limits[]` entry. The array reports an integer
/// `percent` rather than the legacy blocks' float `utilization`.
fn limit_pct(payload: &Value, kind: &str) -> Option<f64> {
    limit_entry(payload, kind)?.get("percent")?.as_f64()
}

/// `resets_at` (RFC3339) for a `limits[]` entry.
fn limit_resets_at<'a>(payload: &'a Value, kind: &str) -> Option<&'a str> {
    limit_entry(payload, kind)?.get("resets_at")?.as_str()
}

/// Session usage %, preferring the `limits` array and falling back to the
/// legacy top-level `five_hour` block for older endpoint/token shapes.
fn session_pct_of(payload: &Value) -> Option<f64> {
    limit_pct(payload, "session").or_else(|| block_pct(payload, "five_hour"))
}

/// Weekly (all-usage) %, `limits` array preferred, legacy `seven_day` fallback.
fn weekly_pct_of(payload: &Value) -> Option<f64> {
    limit_pct(payload, "weekly_all").or_else(|| block_pct(payload, "seven_day"))
}

fn session_resets_of(payload: &Value) -> Option<String> {
    limit_resets_at(payload, "session")
        .or_else(|| block_resets_at(payload, "five_hour"))
        .map(str::to_string)
}

fn weekly_resets_of(payload: &Value) -> Option<String> {
    limit_resets_at(payload, "weekly_all")
        .or_else(|| block_resets_at(payload, "seven_day"))
        .map(str::to_string)
}

/// Display name of the model the `weekly_scoped` limit currently meters
/// (e.g. `"Fable"`), if any.
fn scoped_model_name(payload: &Value) -> Option<String> {
    limit_entry(payload, "weekly_scoped")?
        .get("scope")?
        .get("model")?
        .get("display_name")?
        .as_str()
        .map(str::to_string)
}

/// Overage/extra-usage credit consumption as a percentage of the credit limit.
/// `null` (credits disabled or never enabled) → `None`.
fn extra_usage_pct(payload: &Value) -> Option<f64> {
    payload.get("extra_usage")?.get("utilization")?.as_f64()
}

/// Weekly % for a legacy per-model block (`seven_day_opus` / `seven_day_sonnet`),
/// falling back to the `weekly_scoped` limit when it happens to meter a model
/// whose display name contains `model_hint` (case-insensitive).
fn legacy_model_weekly_pct(payload: &Value, legacy_key: &str, model_hint: &str) -> Option<f64> {
    if let Some(pct) = block_pct(payload, legacy_key) {
        return Some(pct);
    }
    let name = scoped_model_name(payload)?;
    if name.to_ascii_lowercase().contains(&model_hint.to_ascii_lowercase()) {
        limit_pct(payload, "weekly_scoped")
    } else {
        None
    }
}

/// Minutes from now until `resets_at`, clamped to >= 0. `None` if unparseable.
fn minutes_until(resets_at: Option<&str>) -> Option<f64> {
    let ts = resets_at?;
    let target = chrono::DateTime::parse_from_rfc3339(ts).ok()?;
    let now = chrono::Utc::now();
    let mins = (target.timestamp_millis() - now.timestamp_millis()) as f64 / 60_000.0;
    Some(mins.max(0.0))
}

/// Format a minutes-until-reset value as a compact countdown.
///
/// Granularity drops the smallest unit as the duration grows, so the string
/// stays short on a panel: `"3d 4h"`, `"2h 35m"`, `"12m"`, `"<1m"`, `"now"`.
fn format_countdown(minutes: f64) -> String {
    if minutes <= 0.0 {
        return "now".to_string();
    }
    let total = minutes.round() as i64;
    if total == 0 {
        return "<1m".to_string();
    }
    let days = total / 1440;
    let hours = (total % 1440) / 60;
    let mins = total % 60;
    if days > 0 {
        format!("{days}d {hours}h")
    } else if hours > 0 {
        format!("{hours}h {mins}m")
    } else {
        format!("{mins}m")
    }
}

/// Format an RFC3339 reset timestamp as a local weekday + 24h clock, e.g.
/// `"Wed 11:00"`. Empty string if missing/unparseable. Uses the machine's local
/// timezone (the user reads resets in their own clock, not UTC).
fn format_reset_day(resets_at: Option<&str>) -> String {
    let Some(ts) = resets_at else {
        return String::new();
    };
    match chrono::DateTime::parse_from_rfc3339(ts) {
        Ok(dt) => dt
            .with_timezone(&chrono::Local)
            .format("%a %H:%M")
            .to_string(),
        Err(_) => String::new(),
    }
}

/// Reconstruct the current local token "session" from de-duplicated entries.
///
/// Mirrors Claude's plan-usage blocking: a session is a fixed `window`-long
/// block anchored at the first message of the block (floored to the hour). A new
/// block starts when an entry falls `window` or more past the block start, or
/// `window` or more after the previous entry. The current session is the most
/// recent block, but only while `now` is still inside it.
fn current_session(entries: &[Entry], now: i64, window: i64) -> ([u64; 4], bool) {
    if entries.is_empty() || window <= 0 {
        return ([0; 4], false);
    }

    let mut sorted: Vec<Entry> = entries.to_vec();
    sorted.sort_by_key(|e| e.ts);

    let floor_hour = |ts: i64| ts - ts.rem_euclid(3_600_000);

    let mut block_start = floor_hour(sorted[0].ts);
    let mut block_sums = [0u64; 4];
    let mut last_ts = sorted[0].ts;

    for e in &sorted {
        if e.ts - block_start >= window || e.ts - last_ts >= window {
            block_start = floor_hour(e.ts);
            block_sums = [0; 4];
        }
        block_sums[e.family.index()] += e.tokens;
        last_ts = e.ts;
    }

    if now - block_start < window {
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
                "The selected metric's value",
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
                "weekly_pct",
                "Weekly Usage %",
                "Weekly (7-day) all-usage as a percentage of the plan limit",
                FieldType::Numerical,
                FieldPurpose::Value,
            ),
            FieldMetadata::new(
                "weekly_scoped_pct",
                "Weekly Model Usage %",
                "Weekly usage for the current model-scoped limit, as a percentage",
                FieldType::Numerical,
                FieldPurpose::Value,
            ),
            FieldMetadata::new(
                "weekly_scoped_model",
                "Weekly Model Name",
                "Display name of the model the weekly scoped limit meters (e.g. Fable)",
                FieldType::Text,
                FieldPurpose::Other,
            ),
            FieldMetadata::new(
                "extra_usage_pct",
                "Extra Usage %",
                "Overage/extra-usage credits consumed as a percentage (0 if disabled)",
                FieldType::Numerical,
                FieldPurpose::Value,
            ),
            FieldMetadata::new(
                "weekly_opus_pct",
                "Weekly Opus % (legacy)",
                "Legacy Opus weekly usage; the endpoint no longer splits by model, so 0 unless the scoped limit meters Opus",
                FieldType::Numerical,
                FieldPurpose::Value,
            ),
            FieldMetadata::new(
                "weekly_sonnet_pct",
                "Weekly Sonnet % (legacy)",
                "Legacy Sonnet weekly usage; the endpoint no longer splits by model, so 0 unless the scoped limit meters Sonnet",
                FieldType::Numerical,
                FieldPurpose::Value,
            ),
            FieldMetadata::new(
                "weekly_scoped_resets_at",
                "Weekly Model Resets At",
                "Timestamp when the model-scoped weekly limit resets",
                FieldType::Text,
                FieldPurpose::Other,
            ),
            FieldMetadata::new(
                "session_minutes_left",
                "Session Resets In (min)",
                "Minutes until the current session limit resets",
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
                "session_resets_at",
                "Session Resets At",
                "Timestamp when the current session limit resets",
                FieldType::Text,
                FieldPurpose::Other,
            ),
            FieldMetadata::new(
                "weekly_resets_at",
                "Weekly Resets At",
                "Timestamp when the weekly limit resets",
                FieldType::Text,
                FieldPurpose::Other,
            ),
            FieldMetadata::new(
                "session_resets_in_text",
                "Session Resets In",
                "Compact countdown to the session reset (e.g. \"2h 35m\")",
                FieldType::Text,
                FieldPurpose::Value,
            ),
            FieldMetadata::new(
                "weekly_resets_in_text",
                "Weekly Resets In",
                "Compact countdown to the weekly reset (e.g. \"3d 4h\")",
                FieldType::Text,
                FieldPurpose::Value,
            ),
            FieldMetadata::new(
                "session_reset_day",
                "Session Reset Day",
                "Local weekday + time the session resets (e.g. \"Wed 16:49\")",
                FieldType::Text,
                FieldPurpose::Value,
            ),
            FieldMetadata::new(
                "weekly_reset_day",
                "Weekly Reset Day",
                "Local weekday + time the weekly limit resets (e.g. \"Wed 11:00\")",
                FieldType::Text,
                FieldPurpose::Value,
            ),
            FieldMetadata::new(
                "session_total",
                "Session Tokens",
                "Local tokens used in the current session window (all models)",
                FieldType::Numerical,
                FieldPurpose::Value,
            ),
            FieldMetadata::new(
                "alltime_total",
                "All-Time Tokens",
                "Local tokens used across all transcripts (all models)",
                FieldType::Numerical,
                FieldPurpose::Value,
            ),
            FieldMetadata::new(
                "status",
                "Status",
                "Most recent plan-usage fetch status (ok / error message)",
                FieldType::Text,
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
                format!("Session {name} Tokens"),
                "Session tokens for this model family",
                FieldType::Numerical,
                FieldPurpose::Value,
            ));
            fields.push(FieldMetadata::new(
                format!("alltime_{suffix}"),
                format!("All-Time {name} Tokens"),
                "All-time tokens for this model family",
                FieldType::Numerical,
                FieldPurpose::Value,
            ));
        }

        fields
    }

    fn update(&mut self) -> Result<()> {
        // --- token counts from local transcripts ---
        let mut transcripts = Vec::new();
        if let Some(dir) = Self::claude_dir() {
            Self::collect_transcripts(&dir.join("projects"), &mut transcripts, 0);
        }
        self.refresh_cache(&transcripts);

        let mut alltime = [0u64; 4];
        let window_ms = (self.config.session_hours * 3_600_000.0) as i64;
        let now = chrono::Utc::now().timestamp_millis();

        // current_session anchors the rolling block at the first message and
        // cascades window boundaries forward, so truncating too aggressively
        // re-anchors and undercounts. But feeding the entire (ever-growing)
        // history into a per-update sort is unbounded work. Compromise: keep a
        // generous lookback (48 windows ≈ 10 days at the default 5h window) — far
        // longer than any gap-free human session, so anchoring is unaffected,
        // while bounding the sorted set to recent entries.
        let session_cutoff = now - window_ms.saturating_mul(48);
        let mut session_entries: Vec<Entry> = Vec::new();
        for cache in self.file_cache.values() {
            for (i, total) in cache.alltime.iter().enumerate() {
                alltime[i] += total;
            }
            session_entries.extend(cache.entries.iter().filter(|e| e.ts >= session_cutoff).copied());
        }
        let (session, _active) = current_session(&session_entries, now, window_ms);
        let session_total: u64 = session.iter().sum();
        let alltime_total: u64 = alltime.iter().sum();
        self.running_max = self.running_max.max(session_total as f64);

        // --- plan usage from the network (shared, throttled cache) ---
        // Only drive a network fetch when this panel actually shows a
        // percentage; token-only panels stay fully offline.
        let (plan, status) = Self::plan_usage(self.config.metric.is_percentage());

        let session_pct = plan.as_ref().and_then(session_pct_of).unwrap_or(0.0);
        let weekly_pct = plan.as_ref().and_then(weekly_pct_of).unwrap_or(0.0);
        // Current model-scoped weekly limit (replaces the old fixed Opus/Sonnet
        // split) plus the model it meters.
        let weekly_scoped_pct = plan
            .as_ref()
            .and_then(|p| limit_pct(p, "weekly_scoped"))
            .unwrap_or(0.0);
        let weekly_scoped_model = plan
            .as_ref()
            .and_then(scoped_model_name)
            .unwrap_or_default();
        // Legacy per-model fields: null from the API now, so they fall back to
        // the scoped limit when it meters that model, else read 0.
        let weekly_opus_pct = plan
            .as_ref()
            .and_then(|p| legacy_model_weekly_pct(p, "seven_day_opus", "opus"))
            .unwrap_or(0.0);
        let weekly_sonnet_pct = plan
            .as_ref()
            .and_then(|p| legacy_model_weekly_pct(p, "seven_day_sonnet", "sonnet"))
            .unwrap_or(0.0);
        let extra_usage_pct = plan.as_ref().and_then(extra_usage_pct).unwrap_or(0.0);
        let session_reset = plan.as_ref().and_then(session_resets_of);
        let weekly_reset = plan.as_ref().and_then(weekly_resets_of);
        let weekly_scoped_reset = plan
            .as_ref()
            .and_then(|p| limit_resets_at(p, "weekly_scoped").map(str::to_string));

        // --- publish every field ---
        self.values.clear();
        self.values
            .insert("session_pct".to_string(), Value::from(session_pct));
        self.values
            .insert("weekly_pct".to_string(), Value::from(weekly_pct));
        self.values.insert(
            "weekly_scoped_pct".to_string(),
            Value::from(weekly_scoped_pct),
        );
        self.values.insert(
            "weekly_scoped_model".to_string(),
            Value::from(weekly_scoped_model.clone()),
        );
        self.values
            .insert("extra_usage_pct".to_string(), Value::from(extra_usage_pct));
        self.values
            .insert("weekly_opus_pct".to_string(), Value::from(weekly_opus_pct));
        self.values.insert(
            "weekly_sonnet_pct".to_string(),
            Value::from(weekly_sonnet_pct),
        );
        self.values.insert(
            "weekly_scoped_resets_at".to_string(),
            Value::from(weekly_scoped_reset.clone().unwrap_or_default()),
        );
        self.values.insert(
            "session_resets_at".to_string(),
            Value::from(session_reset.clone().unwrap_or_default()),
        );
        self.values.insert(
            "weekly_resets_at".to_string(),
            Value::from(weekly_reset.clone().unwrap_or_default()),
        );
        let session_minutes = minutes_until(session_reset.as_deref()).unwrap_or(0.0);
        let weekly_minutes = minutes_until(weekly_reset.as_deref()).unwrap_or(0.0);
        self.values.insert(
            "session_minutes_left".to_string(),
            Value::from(session_minutes),
        );
        self.values.insert(
            "weekly_minutes_left".to_string(),
            Value::from(weekly_minutes),
        );
        self.values.insert(
            "session_resets_in_text".to_string(),
            Value::from(format_countdown(session_minutes)),
        );
        self.values.insert(
            "weekly_resets_in_text".to_string(),
            Value::from(format_countdown(weekly_minutes)),
        );
        self.values.insert(
            "session_reset_day".to_string(),
            Value::from(format_reset_day(session_reset.as_deref())),
        );
        self.values.insert(
            "weekly_reset_day".to_string(),
            Value::from(format_reset_day(weekly_reset.as_deref())),
        );
        self.values
            .insert("status".to_string(), Value::from(status));
        self.values
            .insert("session_total".to_string(), Value::from(session_total));
        self.values
            .insert("alltime_total".to_string(), Value::from(alltime_total));
        for (family, suffix) in Family::ALL {
            let i = family.index();
            self.values
                .insert(format!("session_{suffix}"), Value::from(session[i]));
            self.values
                .insert(format!("alltime_{suffix}"), Value::from(alltime[i]));
        }

        // --- route the selected metric into value/caption/unit/limits ---
        // Captions are `String` because the scoped-usage label folds in the
        // current model name (e.g. "Claude Weekly (Fable)").
        let (raw_value, auto_caption): (f64, String) = match self.config.metric {
            ClaudeMetric::SessionUsage => (session_pct, "Claude Session".to_string()),
            ClaudeMetric::WeeklyUsage => (weekly_pct, "Claude Weekly".to_string()),
            ClaudeMetric::WeeklyScopedUsage => {
                let label = if weekly_scoped_model.is_empty() {
                    "Claude Weekly (Model)".to_string()
                } else {
                    format!("Claude Weekly ({weekly_scoped_model})")
                };
                (weekly_scoped_pct, label)
            }
            ClaudeMetric::ExtraUsage => (extra_usage_pct, "Claude Extra Usage".to_string()),
            ClaudeMetric::WeeklyOpusUsage => (weekly_opus_pct, "Claude Weekly Opus".to_string()),
            ClaudeMetric::WeeklySonnetUsage => {
                (weekly_sonnet_pct, "Claude Weekly Sonnet".to_string())
            }
            ClaudeMetric::SessionTokens => (session_total as f64, "Claude Session Tokens".to_string()),
            ClaudeMetric::AllTimeTokens => (alltime_total as f64, "Claude Tokens".to_string()),
            ClaudeMetric::SessionResetIn => (session_minutes, "Claude Session Reset".to_string()),
            ClaudeMetric::WeeklyResetIn => (weekly_minutes, "Claude Weekly Reset".to_string()),
        };

        let caption = self
            .config
            .custom_caption
            .clone()
            .unwrap_or(auto_caption);
        let (unit, max_limit) = if self.config.metric.is_percentage() {
            ("%", 100.0)
        } else if self.config.metric.is_reset_time() {
            // Scale the gauge against the reset window itself, not the token
            // running-max (which is a token count and meaningless for minutes).
            let window_minutes = match self.config.metric {
                ClaudeMetric::SessionResetIn => self.config.session_hours * 60.0,
                _ => 7.0 * 24.0 * 60.0, // weekly window
            };
            ("min", window_minutes.max(1.0))
        } else {
            (
                "tokens",
                self.config.max_limit.unwrap_or_else(|| self.running_max.max(1.0)),
            )
        };

        self.values.insert("caption".to_string(), Value::from(caption));
        self.values.insert("value".to_string(), Value::from(raw_value));
        self.values.insert("unit".to_string(), Value::from(unit));
        self.values.insert("min_limit".to_string(), Value::from(0.0));
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
        // Available if Claude Code data exists locally (token counts) OR rg-Sens
        // has its own sign-in (plan usage works without a local ~/.claude).
        Self::claude_dir().is_some_and(|p| p.exists()) || crate::claude_auth::is_signed_in()
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
        let entries = [
            e(0, Family::Opus, 100),
            e(1, Family::Opus, 50),
            e(10, Family::Opus, 7),
            e(11, Family::Sonnet, 3),
        ];
        let (sums, active) = current_session(&entries, 11 * HOUR, WINDOW);
        assert!(active);
        assert_eq!(sums[Family::Opus.index()], 7);
        assert_eq!(sums[Family::Sonnet.index()], 3);
        assert_eq!(sums.iter().sum::<u64>(), 10);
    }

    #[test]
    fn session_expires_once_the_window_elapses() {
        let entries = [e(0, Family::Opus, 100), e(1, Family::Opus, 50)];
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

    // A trimmed copy of a legacy `/api/oauth/usage` response (top-level blocks
    // only), to pin the fallback path.
    const SAMPLE: &str = r#"{
        "five_hour": { "utilization": 5.0, "resets_at": "2026-06-24T20:49:59.494649+00:00" },
        "seven_day": { "utilization": 40.0, "resets_at": "2026-06-26T10:59:59.494671+00:00" },
        "seven_day_opus": null,
        "seven_day_sonnet": { "utilization": 0.0, "resets_at": null }
    }"#;

    // A trimmed copy of the CURRENT response shape: the `limits` array plus the
    // retired per-model blocks (now null) and an `extra_usage` block.
    const SAMPLE_CURRENT: &str = r#"{
        "five_hour": { "utilization": 73.0, "resets_at": "2026-09-08T16:29:59.878426+00:00" },
        "seven_day": { "utilization": 32.0, "resets_at": "2026-09-11T10:59:59.878448+00:00" },
        "seven_day_opus": null,
        "seven_day_sonnet": null,
        "extra_usage": { "is_enabled": false, "utilization": null },
        "limits": [
            { "kind": "session", "percent": 73, "resets_at": "2026-09-08T16:29:59.878426+00:00" },
            { "kind": "weekly_all", "percent": 32, "resets_at": "2026-09-11T10:59:59.878448+00:00" },
            { "kind": "weekly_scoped", "percent": 57, "resets_at": "2026-09-11T10:59:59.878622+00:00",
              "scope": { "model": { "id": null, "display_name": "Fable" } } }
        ]
    }"#;

    #[test]
    fn maps_legacy_payload_to_percentages() {
        let payload: Value = serde_json::from_str(SAMPLE).unwrap();
        assert_eq!(block_pct(&payload, "five_hour"), Some(5.0));
        assert_eq!(block_pct(&payload, "seven_day"), Some(40.0));
        assert_eq!(block_pct(&payload, "seven_day_opus"), None);
        assert_eq!(block_pct(&payload, "seven_day_sonnet"), Some(0.0));
        assert_eq!(
            block_resets_at(&payload, "five_hour"),
            Some("2026-06-24T20:49:59.494649+00:00")
        );
        assert_eq!(block_resets_at(&payload, "seven_day_sonnet"), None);
        // With no `limits` array, the source falls back to the top-level blocks.
        assert_eq!(session_pct_of(&payload), Some(5.0));
        assert_eq!(weekly_pct_of(&payload), Some(40.0));
        assert_eq!(scoped_model_name(&payload), None);
    }

    #[test]
    fn maps_current_payload_from_limits_array() {
        let payload: Value = serde_json::from_str(SAMPLE_CURRENT).unwrap();
        // limits[] is preferred over the legacy blocks (both agree here).
        assert_eq!(session_pct_of(&payload), Some(73.0));
        assert_eq!(weekly_pct_of(&payload), Some(32.0));
        assert_eq!(limit_pct(&payload, "weekly_scoped"), Some(57.0));
        assert_eq!(scoped_model_name(&payload).as_deref(), Some("Fable"));
        // Retired per-model blocks are null; the Fable scope doesn't match
        // opus/sonnet, so the legacy metrics read None (→ 0 when published).
        assert_eq!(
            legacy_model_weekly_pct(&payload, "seven_day_opus", "opus"),
            None
        );
        assert_eq!(
            legacy_model_weekly_pct(&payload, "seven_day_sonnet", "sonnet"),
            None
        );
        // extra_usage disabled → utilization null → None.
        assert_eq!(extra_usage_pct(&payload), None);
        assert_eq!(
            limit_resets_at(&payload, "session"),
            Some("2026-09-08T16:29:59.878426+00:00")
        );
    }

    #[test]
    fn scoped_limit_backfills_matching_legacy_model() {
        // If the scoped limit ever meters Opus, the legacy opus metric picks it up.
        let payload: Value = serde_json::from_str(
            r#"{ "seven_day_opus": null, "limits": [
                { "kind": "weekly_scoped", "percent": 44,
                  "scope": { "model": { "display_name": "Opus" } } }
            ] }"#,
        )
        .unwrap();
        assert_eq!(
            legacy_model_weekly_pct(&payload, "seven_day_opus", "opus"),
            Some(44.0)
        );
        assert_eq!(
            legacy_model_weekly_pct(&payload, "seven_day_sonnet", "sonnet"),
            None
        );
    }

    #[test]
    fn retry_after_parses_delta_seconds_only() {
        assert_eq!(parse_retry_after("3346"), Some(Duration::from_secs(3346)));
        assert_eq!(parse_retry_after(" 60 "), Some(Duration::from_secs(60)));
        assert_eq!(parse_retry_after("-5"), None);
        assert_eq!(parse_retry_after("Sat, 03 Oct 2026 13:09:23 GMT"), None);
    }

    #[test]
    fn rate_limit_backoff_honours_retry_after_within_bounds() {
        // An observed Retry-After (~56 min), far beyond a fixed 5-minute retry.
        assert_eq!(
            rate_limit_backoff(Some(Duration::from_secs(3346))),
            Duration::from_secs(3346)
        );
        // No header, or a tiny one → floor.
        assert_eq!(rate_limit_backoff(None), RATE_LIMIT_BACKOFF);
        assert_eq!(
            rate_limit_backoff(Some(Duration::from_secs(1))),
            RATE_LIMIT_BACKOFF
        );
        // Absurd header → ceiling.
        assert_eq!(
            rate_limit_backoff(Some(Duration::from_secs(7 * 24 * 3600))),
            MAX_RATE_LIMIT_BACKOFF
        );
    }

    // Live end-to-end check; ignored by default (needs network + valid token).
    // Run: cargo test -p rg-sens-sources live_merged -- --ignored --nocapture
    #[test]
    #[ignore]
    fn live_merged_update_routes_metric() {
        let mut src = ClaudeSource::new();
        src.set_config(ClaudeSourceConfig {
            metric: ClaudeMetric::SessionUsage,
            ..Default::default()
        });
        src.update().unwrap();
        let v = src.get_values();
        eprintln!("status        = {:?}", v.get("status"));
        eprintln!("value (route) = {:?}", v.get("value"));
        eprintln!("unit          = {:?}", v.get("unit"));
        eprintln!("session_pct   = {:?}", v.get("session_pct"));
        eprintln!("weekly_pct    = {:?}", v.get("weekly_pct"));
        eprintln!("session_total = {:?}", v.get("session_total"));
        eprintln!("alltime_total = {:?}", v.get("alltime_total"));
        // SessionUsage routes session_pct -> value, unit "%".
        assert_eq!(v.get("unit").and_then(Value::as_str), Some("%"));
        assert_eq!(v.get("value"), v.get("session_pct"));
        // Token fields still populated from local transcripts regardless of metric.
        assert!(v.get("alltime_total").and_then(Value::as_u64).is_some());
    }

    #[test]
    fn minutes_until_is_nonnegative_and_zero_for_past() {
        assert_eq!(minutes_until(Some("2000-01-01T00:00:00+00:00")), Some(0.0));
        assert_eq!(minutes_until(None), None);
        assert_eq!(minutes_until(Some("not-a-date")), None);
    }

    #[test]
    fn countdown_drops_smallest_unit_as_duration_grows() {
        assert_eq!(format_countdown(0.0), "now");
        assert_eq!(format_countdown(0.4), "<1m"); // rounds to 0 but is positive
        assert_eq!(format_countdown(12.0), "12m");
        assert_eq!(format_countdown(155.0), "2h 35m");
        assert_eq!(format_countdown(4_564.0), "3d 4h"); // 3d 4h 4m -> "3d 4h"
    }

    #[test]
    fn reset_day_is_empty_for_missing_or_bad_input() {
        assert_eq!(format_reset_day(None), "");
        assert_eq!(format_reset_day(Some("not-a-date")), "");
        // A real timestamp renders as "<weekday> HH:MM" in local time; we only
        // assert the shape (3-letter day, space, HH:MM) to stay timezone-agnostic.
        let s = format_reset_day(Some("2026-06-24T20:49:59+00:00"));
        assert_eq!(s.len(), 9, "expected 'Ddd HH:MM', got {s:?}");
        assert_eq!(&s[3..4], " ");
        assert_eq!(&s[6..7], ":");
    }
}
