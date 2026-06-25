//! rg-Sens's own Claude OAuth sign-in (PKCE), independent of Claude Code.
//!
//! ## Why this exists
//!
//! The `ClaudeSource` plan-usage metrics need a valid OAuth access token to call
//! Anthropic's `/api/oauth/usage` endpoint. Historically the source borrowed
//! Claude Code's token read-only from `~/.claude/.credentials.json`, but that
//! token expires and only Claude Code can refresh it — so the percentages go
//! stale whenever Claude Code isn't running.
//!
//! This module lets rg-Sens hold its **own** token pair, obtained via the same
//! public OAuth client Claude Code uses, and refresh it itself.
//!
//! ## Hard safety boundary
//!
//! We store our tokens in `~/.config/rg-sens/claude_auth.json` (mode 0600) and
//! **never** touch `~/.claude/.credentials.json`. Refresh tokens rotate on use;
//! refreshing Claude Code's token would invalidate Claude Code's own login. Our
//! token pair is a separate grant, so refreshing it is safe.
//!
//! ## Flow (manual paste / "code" flow)
//!
//! 1. [`begin`] makes a PKCE verifier + random state and an authorize URL.
//! 2. The user opens the URL, approves, and the callback page shows `code#state`.
//! 3. [`complete`] verifies the state, exchanges the code for tokens, and saves.
//! 4. [`access_token`] returns a valid token, refreshing transparently when near
//!    expiry. The source falls back to Claude Code's read-only token if we have
//!    none of our own.

use base64::Engine;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

// --- OAuth client constants ----------------------------------------------
//
// Extracted verbatim from the installed Claude Code binary (the public client;
// these are not secrets). If Anthropic changes this flow these will need
// updating — the source degrades to "token expired" status, it does not crash.
const CLIENT_ID: &str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";
const AUTHORIZE_URL: &str = "https://platform.claude.com/oauth/authorize";
const TOKEN_URL: &str = "https://platform.claude.com/v1/oauth/token";
const REDIRECT_URI: &str = "https://platform.claude.com/oauth/code/callback";
const SCOPES: &str = "user:profile user:inference user:sessions:claude_code user:mcp_servers";

/// Refresh this many milliseconds *before* the token actually expires, so a
/// fetch never races the expiry boundary.
const REFRESH_SKEW_MS: i64 = 60_000;

/// Serializes refresh+save across threads. The single-use refresh token must
/// not be spent by two concurrent panels (the second call would fail and could
/// invalidate the first), so we claim the lock for the whole refresh sequence.
static AUTH_LOCK: Mutex<()> = Mutex::new(());

/// Tokens rg-Sens owns, persisted to `claude_auth.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredTokens {
    pub access_token: String,
    pub refresh_token: String,
    /// Absolute expiry as Unix epoch milliseconds.
    pub expires_at_ms: i64,
}

impl StoredTokens {
    fn is_expired(&self) -> bool {
        chrono::Utc::now().timestamp_millis() >= self.expires_at_ms - REFRESH_SKEW_MS
    }
}

/// In-flight sign-in: the PKCE verifier + state we must check against the paste.
pub struct PendingAuth {
    verifier: String,
    state: String,
}

impl PendingAuth {
    /// The URL the user should open in a browser to approve access.
    pub fn authorize_url(&self) -> String {
        let challenge = pkce_challenge(&self.verifier);
        let scope = urlencode(SCOPES);
        let redirect = urlencode(REDIRECT_URI);
        format!(
            "{AUTHORIZE_URL}?code=true&client_id={CLIENT_ID}&response_type=code\
             &redirect_uri={redirect}&scope={scope}\
             &code_challenge={challenge}&code_challenge_method=S256&state={state}",
            state = self.state,
        )
    }
}

/// Start a sign-in: fresh PKCE verifier + random state.
pub fn begin() -> PendingAuth {
    PendingAuth {
        verifier: random_token(),
        state: uuid::Uuid::new_v4().to_string(),
    }
}

/// Finish a sign-in by exchanging the pasted `code#state` for tokens and saving
/// them. Runs blocking network I/O — call off the GTK main thread.
///
/// Returns a human-readable error string suitable for showing in the config UI.
pub fn complete(pending: &PendingAuth, pasted: &str) -> Result<(), String> {
    // The callback page presents the value as `code#state`. Tolerate either a
    // bare code or the combined form, and verify state when present.
    let pasted = pasted.trim();
    let (code, state) = match pasted.split_once('#') {
        Some((c, s)) => (c, Some(s)),
        None => (pasted, None),
    };
    if code.is_empty() {
        return Err("no code pasted".to_string());
    }
    if let Some(s) = state {
        if s != pending.state {
            return Err("state mismatch — copy the whole code, then retry".to_string());
        }
    }

    let body = serde_json::json!({
        "grant_type": "authorization_code",
        "code": code,
        "state": pending.state,
        "client_id": CLIENT_ID,
        "redirect_uri": REDIRECT_URI,
        "code_verifier": pending.verifier,
    });

    let tokens = post_token(&body)?;
    save(&tokens).map_err(|e| format!("signed in, but could not save token: {e}"))?;
    Ok(())
}

/// True if rg-Sens has its own saved token (regardless of expiry — an expired
/// one can still be refreshed).
pub fn is_signed_in() -> bool {
    load().is_some()
}

/// Forget rg-Sens's token. Does not affect Claude Code.
pub fn sign_out() {
    if let Some(path) = auth_path() {
        let _ = std::fs::remove_file(path);
    }
}

/// Return a currently-valid access token, refreshing if near expiry. `None` if
/// not signed in or the refresh failed. Runs blocking I/O on refresh — callers
/// (the source) already run off the GTK thread.
pub fn access_token() -> Option<String> {
    let tokens = load()?;
    if !tokens.is_expired() {
        return Some(tokens.access_token);
    }

    // Expired: refresh under the lock so siblings don't double-spend the token.
    let _guard = AUTH_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    // Re-load inside the lock: another thread may have just refreshed.
    let tokens = load()?;
    if !tokens.is_expired() {
        return Some(tokens.access_token);
    }
    match refresh(&tokens.refresh_token) {
        Ok(fresh) => {
            let _ = save(&fresh);
            Some(fresh.access_token)
        }
        Err(_) => None,
    }
}

// --- token endpoint ------------------------------------------------------

fn refresh(refresh_token: &str) -> Result<StoredTokens, String> {
    let body = serde_json::json!({
        "grant_type": "refresh_token",
        "refresh_token": refresh_token,
        "client_id": CLIENT_ID,
    });
    post_token(&body)
}

/// POST a token-endpoint request and parse the response into `StoredTokens`.
fn post_token(body: &serde_json::Value) -> Result<StoredTokens, String> {
    let resp = ureq::post(TOKEN_URL)
        .set("Content-Type", "application/json")
        .timeout(Duration::from_secs(15))
        .send_json(body);

    let json: serde_json::Value = match resp {
        Ok(r) => r.into_json().map_err(|e| format!("bad token response: {e}"))?,
        Err(ureq::Error::Status(code, r)) => {
            let detail = r
                .into_json::<serde_json::Value>()
                .ok()
                .and_then(|v| {
                    v.get("error_description")
                        .or_else(|| v.get("error"))
                        .and_then(|x| x.as_str())
                        .map(str::to_string)
                })
                .unwrap_or_default();
            return Err(if detail.is_empty() {
                format!("token endpoint HTTP {code}")
            } else {
                format!("HTTP {code}: {detail}")
            });
        }
        Err(e) => return Err(format!("network error: {e}")),
    };

    let access_token = json
        .get("access_token")
        .and_then(|v| v.as_str())
        .ok_or("response had no access_token")?
        .to_string();
    // A refresh response may omit refresh_token (token not rotated); reuse the
    // request's one in that case. For an auth-code exchange it's always present.
    let refresh_token = json
        .get("refresh_token")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .or_else(|| {
            body.get("refresh_token")
                .and_then(|v| v.as_str())
                .map(str::to_string)
        })
        .ok_or("response had no refresh_token")?;
    let expires_in = json
        .get("expires_in")
        .and_then(|v| v.as_i64())
        .unwrap_or(3600);
    let expires_at_ms = chrono::Utc::now().timestamp_millis() + expires_in * 1000;

    Ok(StoredTokens {
        access_token,
        refresh_token,
        expires_at_ms,
    })
}

// --- storage -------------------------------------------------------------

/// `~/.config/rg-sens/claude_auth.json` (same base dir as the app config).
fn auth_path() -> Option<PathBuf> {
    let dirs = directories::ProjectDirs::from("com", "github.hilgardt_collab", "rg-sens")?;
    Some(dirs.config_dir().join("claude_auth.json"))
}

fn load() -> Option<StoredTokens> {
    let path = auth_path()?;
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

fn save(tokens: &StoredTokens) -> Result<(), String> {
    let path = auth_path().ok_or("could not determine config directory")?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let content = serde_json::to_string_pretty(tokens).map_err(|e| e.to_string())?;
    std::fs::write(&path, content).map_err(|e| e.to_string())?;
    restrict_permissions(&path);
    Ok(())
}

/// Best-effort `chmod 600` so the token file isn't world/group readable.
#[cfg(unix)]
fn restrict_permissions(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
}

#[cfg(not(unix))]
fn restrict_permissions(_path: &std::path::Path) {}

// --- PKCE helpers --------------------------------------------------------

/// A 43-char base64url code_verifier built from 32 bytes of UUID randomness.
fn random_token() -> String {
    let mut bytes = [0u8; 32];
    bytes[..16].copy_from_slice(uuid::Uuid::new_v4().as_bytes());
    bytes[16..].copy_from_slice(uuid::Uuid::new_v4().as_bytes());
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

/// PKCE S256 challenge: base64url(sha256(verifier)).
fn pkce_challenge(verifier: &str) -> String {
    let digest = Sha256::digest(verifier.as_bytes());
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest)
}

/// Minimal percent-encoding for the few characters in our query values (the
/// space and colon in the scope, the `:` and `/` in the redirect URI).
fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_challenge_matches_rfc7636_test_vector() {
        // RFC 7636 Appendix B verifier/challenge pair.
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        let challenge = pkce_challenge(verifier);
        assert_eq!(challenge, "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
    }

    #[test]
    fn verifier_is_url_safe_and_right_length() {
        let v = random_token();
        assert_eq!(v.len(), 43); // 32 bytes -> 43 base64url chars (no pad)
        assert!(v
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_')));
    }

    #[test]
    fn authorize_url_has_required_params() {
        let p = PendingAuth {
            verifier: "verifier".to_string(),
            state: "the-state".to_string(),
        };
        let url = p.authorize_url();
        assert!(url.starts_with(AUTHORIZE_URL));
        assert!(url.contains("code=true"));
        assert!(url.contains(&format!("client_id={CLIENT_ID}")));
        assert!(url.contains("response_type=code"));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("state=the-state"));
        assert!(url.contains("scope=user%3Aprofile")); // space + colon encoded
    }

    #[test]
    fn urlencode_preserves_unreserved_and_escapes_the_rest() {
        assert_eq!(urlencode("aZ0-._~"), "aZ0-._~");
        assert_eq!(urlencode("a b"), "a%20b");
        assert_eq!(urlencode("user:profile"), "user%3Aprofile");
    }
}
