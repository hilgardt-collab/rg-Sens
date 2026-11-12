//! Memory harness for the CSS Template (WebKit) displayer.
//!
//! Opens one window holding the real `CssTemplateDisplayer` widget, feeds it
//! changing values and prints one CSV line per sample with the PSS of this
//! process and of every descendant process (WebKit's web, network and GPU
//! processes), next to how many JavaScript updates and WebView recycles
//! happened. It never touches `~/.config/rg-sens`.
//!
//! ```text
//! cargo run --release --example webkit_leak -- [key=value ...]
//!   template=PATH|default   HTML template (default: built-in no-JS template)
//!   updates=on|off          feed changing values (default on)
//!   feed_ms=N               how often values change (default 100)
//!   churn=SECS              recreate the widget every SECS (default 0 = never)
//!   duration=SECS           run length (default 120)
//!   sample=SECS             sampling period (default 5)
//! ```
//!
//! The backend's own cadence is set through the environment:
//! `RG_SENS_WEBKIT_TICK_MS` (default 1000) and `RG_SENS_WEBKIT_RECYCLE_EVERY`
//! (default 300 updates, 0 = never recycle).

use gtk4::prelude::*;
use gtk4::{glib, Application, ApplicationWindow};
use rg_sens::core::Displayer;
use rg_sens::displayers::webkit_backend::{live_webviews, EVAL_COUNT, LIVE_TIMERS, RECYCLE_COUNT};
use rg_sens::displayers::CssTemplateDisplayer;
use serde_json::{json, Value};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

fn arg(name: &str, default: &str) -> String {
    let prefix = format!("{name}=");
    std::env::args()
        .find_map(|a| a.strip_prefix(&prefix).map(str::to_string))
        .unwrap_or_else(|| default.to_string())
}

/// PSS of one process in kB (0 if it is gone or unreadable).
fn pss_kb(pid: u32) -> u64 {
    std::fs::read_to_string(format!("/proc/{pid}/smaps_rollup"))
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("Pss:"))
                .and_then(|l| l.split_whitespace().nth(1)?.parse().ok())
        })
        .unwrap_or(0)
}

/// Every descendant of this process as `(pid, comm)`.
fn descendants() -> Vec<(u32, String)> {
    let mut parent: HashMap<u32, (u32, String)> = HashMap::new();
    if let Ok(dir) = std::fs::read_dir("/proc") {
        for entry in dir.flatten() {
            let Some(pid) = entry
                .file_name()
                .to_str()
                .and_then(|s| s.parse::<u32>().ok())
            else {
                continue;
            };
            let Ok(stat) = std::fs::read_to_string(format!("/proc/{pid}/stat")) else {
                continue;
            };
            // "pid (comm) state ppid ..." — comm may contain spaces.
            let (Some(open), Some(close)) = (stat.find('('), stat.rfind(')')) else {
                continue;
            };
            let comm = stat[open + 1..close].to_string();
            let ppid = stat[close + 1..]
                .split_whitespace()
                .nth(1)
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            parent.insert(pid, (ppid, comm));
        }
    }
    let me = std::process::id();
    let mut out = Vec::new();
    for (&pid, (_, comm)) in &parent {
        let mut cursor = pid;
        while let Some((pp, _)) = parent.get(&cursor) {
            if *pp == me {
                out.push((pid, comm.clone()));
                break;
            }
            if *pp <= 1 {
                break;
            }
            cursor = *pp;
        }
    }
    out
}

fn main() {
    let template = arg("template", "default");
    let updates = arg("updates", "on") == "on";
    let feed_ms: u64 = arg("feed_ms", "100").parse().unwrap_or(100);
    let churn: u64 = arg("churn", "0").parse().unwrap_or(0);
    let duration: u64 = arg("duration", "120").parse().unwrap_or(120);
    let sample: u64 = arg("sample", "5").parse().unwrap_or(5);

    let app = Application::builder()
        .application_id("com.github.rg_sens.webkit_leak")
        .flags(gtk4::gio::ApplicationFlags::NON_UNIQUE)
        .build();

    app.connect_activate(move |app| {
        let window = ApplicationWindow::builder()
            .application(app)
            .title("rg-Sens WebKit leak harness")
            .default_width(900)
            .default_height(600)
            .build();

        // 40 placeholders bound to group1_1 … group4_10, field "value".
        let slots: Vec<String> = (1..=4)
            .flat_map(|g| (1..=10).map(move |m| format!("group{g}_{m}")))
            .collect();
        let mappings: Vec<Value> = slots
            .iter()
            .enumerate()
            .map(|(i, slot)| json!({"index": i, "slot_prefix": slot, "field": "value"}))
            .collect();
        let mut css_config = json!({"mappings": mappings, "hot_reload": false});
        if template != "default" {
            let path = std::fs::canonicalize(&template).expect("template path");
            css_config["html_path"] = json!(path);
        }
        let mut displayer = CssTemplateDisplayer::with_webkit_backend();
        let mut config = HashMap::new();
        config.insert("css_template_config".to_string(), css_config);
        displayer.apply_config(&config).expect("config applies");
        let displayer = Rc::new(RefCell::new(displayer));

        window.set_child(Some(&displayer.borrow().create_widget()));
        window.present();

        // Feed changing values.
        if updates {
            let displayer = displayer.clone();
            let n = Cell::new(0u64);
            glib::timeout_add_local(Duration::from_millis(feed_ms), move || {
                n.set(n.get() + 1);
                let mut values = HashMap::new();
                for (i, slot) in slots.iter().enumerate() {
                    let v = ((n.get() * 7 + i as u64 * 13) % 1000) as f64 / 10.0;
                    values.insert(format!("{slot}_value"), json!(v));
                    values.insert(format!("{slot}_caption"), json!(format!("Item {i}")));
                    values.insert(format!("{slot}_unit"), json!("%"));
                }
                displayer.borrow_mut().update_data(&values);
                glib::ControlFlow::Continue
            });
        }

        // Recreate the widget the way a displayer swap / Apply does.
        if churn > 0 {
            let displayer = displayer.clone();
            let window = window.clone();
            glib::timeout_add_local(Duration::from_secs(churn), move || {
                window.set_child(Some(&displayer.borrow().create_widget()));
                glib::ControlFlow::Continue
            });
        }

        // Sample memory.
        println!("t_s,evals,recycles,timers,webviews,self_kb,web_kb,web_n,net_kb,net_n,other_kb,other_n,total_kb");
        let start = Instant::now();
        let app = app.clone();
        glib::timeout_add_local(Duration::from_secs(sample), move || {
            let (mut web, mut net, mut other) = ((0u64, 0u32), (0u64, 0u32), (0u64, 0u32));
            for (pid, comm) in descendants() {
                let kb = pss_kb(pid);
                let bucket = if comm.starts_with("WebKitWebProces") {
                    &mut web
                } else if comm.starts_with("WebKitNetworkPr") {
                    &mut net
                } else {
                    &mut other
                };
                bucket.0 += kb;
                bucket.1 += 1;
            }
            let own = pss_kb(std::process::id());
            let t = start.elapsed().as_secs();
            println!(
                "{t},{},{},{},{},{own},{},{},{},{},{},{},{}",
                EVAL_COUNT.load(Ordering::Relaxed),
                RECYCLE_COUNT.load(Ordering::Relaxed),
                LIVE_TIMERS.load(Ordering::Relaxed),
                live_webviews(),
                web.0,
                web.1,
                net.0,
                net.1,
                other.0,
                other.1,
                own + web.0 + net.0 + other.0
            );
            if t >= duration {
                rg_sens::displayers::css_template_shutdown();
                app.quit();
                return glib::ControlFlow::Break;
            }
            glib::ControlFlow::Continue
        });
    });

    app.run_with_args::<&str>(&[]);
}
