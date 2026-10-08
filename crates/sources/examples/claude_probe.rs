//! One-shot diagnostic: drive the real `ClaudeSource` path against
//! `/api/oauth/usage` with whatever token is on disk (rg-Sens's own, or Claude
//! Code's read-only fallback) and print the outcome.
//!
//! Run:
//!   cargo run -p rg-sens-sources --example claude_probe            # one update
//!   cargo run -p rg-sens-sources --example claude_probe -- 90      # follow for 90s
//!
//! With a duration, `update()` is called every 5s for that long and the
//! status is printed whenever it changes — that exercises the shared cache's
//! back-off and the rg-Sens → Claude Code token fallback exactly as the app
//! would, without a GTK window. Only *due* updates hit the network, but every
//! fetch counts against the endpoint's small per-account quota, so don't loop
//! this for fun. Exit code is 0 only when the last status was "ok".

use rg_sens_core::DataSource;
use rg_sens_sources::{claude_auth, ClaudeSource};
use std::time::{Duration, Instant};

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let follow_secs: u64 = std::env::args()
        .nth(1)
        .and_then(|a| a.parse().ok())
        .unwrap_or(0);
    println!("rg-Sens has its own token: {}", claude_auth::is_signed_in());

    let mut src = ClaudeSource::new(); // default metric is a percentage → fetches
    let started = Instant::now();
    let mut last_status = String::new();
    loop {
        if let Err(e) = src.update() {
            eprintln!("source update errored: {e}");
            std::process::exit(1);
        }
        let v = src.get_values();
        let status = v
            .get("status")
            .and_then(|s| s.as_str())
            .unwrap_or("?")
            .to_string();
        if status != last_status {
            println!("[+{:>3}s] status = {status:?}", started.elapsed().as_secs());
            for key in [
                "session_pct",
                "weekly_pct",
                "weekly_scoped_pct",
                "weekly_scoped_model",
                "extra_usage_pct",
                "session_reset_day",
                "weekly_reset_day",
            ] {
                println!("{key:>26} = {:?}", v.get(key));
            }
            last_status = status;
        }
        if started.elapsed() >= Duration::from_secs(follow_secs) {
            break;
        }
        std::thread::sleep(Duration::from_secs(5));
    }
    std::process::exit(if last_status == "ok" { 0 } else { 1 });
}
