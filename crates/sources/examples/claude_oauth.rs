//! Throwaway CLI to prove the Claude OAuth round-trip before wiring the GTK UI.
//!
//! Run:
//!   cargo run -p rg-sens-sources --example claude_oauth
//!
//! It prints an authorize URL, waits for you to paste the `code#state` shown on
//! the callback page, exchanges it for tokens, saves them to
//! `~/.config/rg-sens/claude_auth.json`, then drives a real `ClaudeSource`
//! update so the freshly-minted token is actually exercised against
//! `/api/oauth/usage`. `status == "ok"` is the only real green light.

use rg_sens_core::DataSource;
use rg_sens_sources::{claude_auth, ClaudeSource};
use std::io::{self, Write};

fn main() {
    let pending = claude_auth::begin();
    println!("\n1. Open this URL in your browser and approve access:\n");
    println!("{}\n", pending.authorize_url());
    print!("2. Paste the code shown on the page (code#state), then Enter:\n> ");
    io::stdout().flush().ok();

    let mut line = String::new();
    if io::stdin().read_line(&mut line).is_err() {
        eprintln!("could not read input");
        std::process::exit(1);
    }

    match claude_auth::complete(&pending, &line) {
        Ok(()) => println!("\n✓ Token exchange succeeded and saved."),
        Err(e) => {
            eprintln!("\n✗ Token exchange failed: {e}");
            std::process::exit(1);
        }
    }

    match claude_auth::access_token() {
        Some(tok) => println!("✓ access_token() returned a token ({} chars).", tok.len()),
        None => {
            eprintln!("✗ access_token() returned None after sign-in");
            std::process::exit(1);
        }
    }

    // The real proof: drive a source update so the minted token is sent to
    // /api/oauth/usage. A token can exchange fine yet be rejected by the
    // endpoint (scopes/headers) — only "ok" status confirms end-to-end.
    println!("\n3. Calling /api/oauth/usage via ClaudeSource…");
    let mut src = ClaudeSource::new(); // default metric is a percentage -> triggers fetch
    if let Err(e) = src.update() {
        eprintln!("✗ source update errored: {e}");
        std::process::exit(1);
    }
    let v = src.get_values();
    let status = v.get("status").and_then(|s| s.as_str()).unwrap_or("?");
    println!("   status       = {status:?}");
    println!("   session_pct  = {:?}", v.get("session_pct"));
    println!("   weekly_pct   = {:?}", v.get("weekly_pct"));
    println!("   session_resets_in = {:?}", v.get("session_resets_in_text"));
    println!("   weekly_reset_day  = {:?}", v.get("weekly_reset_day"));

    if status == "ok" {
        println!("\n✓ END-TO-END OK — the token is accepted by the usage endpoint.");
        println!("Signed in: {}", claude_auth::is_signed_in());
    } else {
        eprintln!("\n✗ usage endpoint rejected the token (status above). The exchange");
        eprintln!("  worked but the token isn't accepted — check scopes/headers.");
        std::process::exit(1);
    }
}
