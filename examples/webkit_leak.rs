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
//!   values=FILE             JSON object {"index": "text"} shown as fixed values
//!                           (placeholder N is bound to slot group1_N+1, field
//!                           "value"); replaces the changing feed
//!   size=WxH                window size (default 900x600)
//!   snapshot=FILE.png       save what the widget drew after 4 s
//!   resize=WxH[,WxH...]     the widget sits in a Fixed at `size`, as a panel
//!                           does in the grid, and is given each of these
//!                           sizes in turn, 3 s apart; the snapshot is taken
//!                           2 s after the last one
//!   reapply=on              also re-apply the panel's config at each resize,
//!                           as Panel Properties does (the page reloads)
//!   settle_ms=N             how long after the last size the snapshot is
//!                           taken (default 2000)
//!   resize_ms=N             time between those sizes (default 3000; a few
//!                           tens of ms imitates dragging a panel's edge)
//!   animation=on|off        the panel's Animation switch (default on)
//!   options=ID=VALUE,...    values for the template's own options
//!   layout=FILE             JSON of a custom layout (`CssLayoutConfig`)
//!   scan=on                 set the panel up as Scan Placeholders does for a
//!                           new one: the template's sensors as slots, its
//!                           placeholders bound, made-up values for every
//!                           slot. `scan=custom` also turns on the custom
//!                           layout a fresh panel starts with (templates
//!                           with prototypes); `layout=FILE` overrides it and
//!                           `layout_out=FILE` writes it out to start from;
//!                           `html_out=FILE` writes the page as built
//!   feed=FILE               JSON object of source values by their own keys
//!                           (`group2_1_value`), fed as they are; for the
//!                           widgets of a custom layout
//! ```
//!
//! The backend's own cadence is set through the environment:
//! `RG_SENS_WEBKIT_TICK_MS` (default 100) and `RG_SENS_WEBKIT_RECYCLE_EVERY`
//! (value updates between WebView recycles; default 0 = never recycle).

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

/// Plausible values for every field a slot's source could report, so both
/// a template's own bindings and generated widgets have something to show.
fn made_up_values(slots: &[rg_sens::ui::css_template_display::SlotKind]) -> HashMap<String, Value> {
    let mut out = HashMap::new();
    for (i, slot) in slots.iter().enumerate() {
        let mut put = |field: &str, value: Value| {
            out.insert(format!("{}_{field}", slot.prefix), value);
        };
        if slot.source == "clock" {
            put("time", json!("17:14:05"));
            put("hour", json!(17));
            put("minute", json!(14));
            put("second", json!(5));
            put("date", json!("2026-11-13"));
            put("day_name", json!("Friday"));
            put("day", json!(13));
            put("month", json!(11));
            put("month_name", json!("November"));
            put("year", json!(2026));
            put("alarm_enabled", json!(false));
            put("caption", json!("Clock"));
            put("value", json!("17:14"));
            continue;
        }
        if slot.source == "uptime" {
            let seconds = 4 * 86_400 + 7 * 3_600 + 12 * 60;
            put("caption", json!("Uptime"));
            put("value", json!("4d 07h 12m"));
            put("text", json!("4 days, 7 hours"));
            put("compact", json!("4d 07h"));
            put("seconds", json!(seconds));
            put("days", json!(4));
            put("hours", json!(7));
            put("minutes", json!(12));
            put("unit", json!(""));
            continue;
        }
        let metric = slot.metric.to_lowercase();
        let label = slot.label.to_lowercase();
        let step = (i * 13 % 50) as f64;
        let (value, unit, max) = if slot.source == "uptime" {
            (json!("4d 07h 12m"), "", 0.0)
        } else if metric.contains("temp") || slot.source == "system_temp" {
            (json!(48.0 + step * 0.6), "°C", 100.0)
        } else if slot.source == "fan_speed" || label.contains("fan") {
            (json!(860.0 + step * 20.0), "RPM", 2400.0)
        } else if metric.contains("freq") || metric.contains("clock") {
            (json!(2.1 + step * 0.05), "GHz", 5.5)
        } else if metric.contains("power") {
            (json!(45.0 + step * 2.0), "W", 300.0)
        } else if slot.source == "network" {
            (json!(1.1 + step * 0.3), "MB/s", 16.0)
        } else if (metric.contains("used") || slot.source == "disk") && !metric.contains("percent")
        {
            (json!(6.1 + step * 0.4), "GB", 32.0)
        } else {
            (json!(24.0 + step * 1.3), "%", 100.0)
        };
        put("caption", json!(slot.label.clone()));
        put("numerical_value", value.clone());
        put("value", value);
        put("unit", json!(unit));
        put("max", json!(max));
        put("max_limit", json!(max));
        put("min", json!(0));
        put("min_limit", json!(0));
        put("total", json!(max));
        put("percent", json!(24.0 + step * 1.3));
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
    let fixed: Option<HashMap<String, String>> =
        Some(arg("values", "")).filter(|p| !p.is_empty()).map(|p| {
            let text = std::fs::read_to_string(&p).expect("values file");
            serde_json::from_str(&text).expect("values file is a JSON object of strings")
        });
    let read_json = |name: &str| -> Option<Value> {
        Some(arg(name, "")).filter(|p| !p.is_empty()).map(|p| {
            let text = std::fs::read_to_string(&p).expect("json file");
            serde_json::from_str(&text).expect("valid json")
        })
    };
    let mut layout = read_json("layout");
    let mut feed: Option<HashMap<String, Value>> =
        read_json("feed").map(|v| serde_json::from_value(v).expect("feed is an object"));
    let scan = arg("scan", "off");
    let mut scanned_mappings: Option<Value> = None;
    if scan != "off" {
        let html = std::fs::read_to_string(&template).expect("template file");
        let scanned = rg_sens::ui::css_template_layout::scan_template(&html);
        let spec = rg_sens::ui::css_template_layout::layout_spec(&html);
        eprintln!(
            "scan: {} slots, {} bindings; prototypes: {} widgets [{}], {} groups, usable={}",
            scanned.slots.len(),
            scanned.mappings.len(),
            spec.widgets.len(),
            spec.widgets
                .iter()
                .map(|w| format!("{}/{}", w.kind, w.size))
                .collect::<Vec<_>>()
                .join(" "),
            spec.groups.len(),
            spec.usable(),
        );
        for slot in &scanned.slots {
            let widget = scanned
                .layout
                .widgets
                .get(&slot.prefix)
                .map(|w| w.kind.as_str());
            eprintln!(
                "  {} {} [{}] -> {}",
                slot.prefix,
                slot.label,
                slot.source,
                widget.unwrap_or("-")
            );
        }
        if scan == "custom" && layout.is_none() {
            assert!(
                scanned.layout.custom,
                "the template has no usable prototypes"
            );
            layout = Some(serde_json::to_value(&scanned.layout).expect("layout json"));
        }
        let html_out = arg("html_out", "");
        if !html_out.is_empty() {
            let layout: rg_sens_types::display_configs::css_template::CssLayoutConfig = layout
                .clone()
                .map(|l| serde_json::from_value(l).expect("layout"))
                .unwrap_or_else(|| scanned.layout.clone());
            let built = rg_sens::ui::css_template_layout::expand_layout(&html, &layout)
                .map_or(html.clone(), |e| e.html);
            std::fs::write(html_out, built).expect("html_out");
        }
        let out = arg("layout_out", "");
        if !out.is_empty() {
            let text = serde_json::to_string_pretty(&scanned.layout).expect("layout json");
            std::fs::write(out, text).expect("layout_out");
        }
        if feed.is_none() {
            feed = Some(made_up_values(&scanned.slots));
        }
        scanned_mappings = Some(serde_json::to_value(&scanned.mappings).expect("mappings json"));
    }
    let snapshot = arg("snapshot", "");
    let animation = arg("animation", "on") == "on";
    let reapply = arg("reapply", "off") == "on";
    let settle_ms: u64 = arg("settle_ms", "2000").parse().unwrap_or(2000);
    let resize_ms: u64 = arg("resize_ms", "3000").parse().unwrap_or(3000);
    let resizes: Vec<(i32, i32)> = arg("resize", "")
        .split(',')
        .filter_map(|size| size.split_once('x'))
        .filter_map(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?)))
        .collect();
    let (win_w, win_h) = arg("size", "900x600")
        .split_once('x')
        .and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?)))
        .unwrap_or((900, 600));

    let app = Application::builder()
        .application_id("com.github.rg_sens.webkit_leak")
        .flags(gtk4::gio::ApplicationFlags::NON_UNIQUE)
        .build();

    app.connect_activate(move |app| {
        let window = ApplicationWindow::builder()
            .application(app)
            .title("rg-Sens WebKit leak harness")
            .default_width(win_w)
            .default_height(win_h)
            .build();

        // 40 placeholders bound to group1_1 … group4_10, field "value".
        let slots: Vec<String> = match &fixed {
            Some(values) => {
                let count = values.keys().filter_map(|k| k.parse::<usize>().ok()).max();
                (0..=count.unwrap_or(0)).map(|i| format!("group1_{}", i + 1)).collect()
            }
            None => (1..=4)
                .flat_map(|g| (1..=10).map(move |m| format!("group{g}_{m}")))
                .collect(),
        };
        let mappings: Vec<Value> = slots
            .iter()
            .enumerate()
            .map(|(i, slot)| json!({"index": i, "slot_prefix": slot, "field": "value"}))
            .collect();
        let mut css_config = json!({
            "mappings": scanned_mappings.clone().unwrap_or(json!(mappings)),
            "hot_reload": false,
            "animation_enabled": animation,
        });
        if template != "default" {
            let path = std::fs::canonicalize(&template).expect("template path");
            css_config["html_path"] = json!(path);
        }
        let mut displayer = CssTemplateDisplayer::with_webkit_backend();
        let mut config = HashMap::new();
        // options=id=value,id=value sets the template's own options
        let options: serde_json::Map<String, Value> = arg("options", "")
            .split(',')
            .filter_map(|pair| pair.split_once('='))
            .map(|(id, value)| (id.to_string(), json!(value)))
            .collect();
        if !options.is_empty() {
            css_config["options"] = Value::Object(options);
        }
        if let Some(layout) = layout.clone() {
            css_config["layout"] = layout;
        }
        config.insert("css_template_config".to_string(), css_config);
        displayer.apply_config(&config).expect("config applies");
        let displayer = Rc::new(RefCell::new(displayer));

        let widget = displayer.borrow().create_widget();
        if resizes.is_empty() {
            window.set_child(Some(&widget));
        } else {
            // the way the grid holds a panel: a Fixed and a size request
            let fixed = gtk4::Fixed::new();
            widget.set_size_request(win_w, win_h);
            fixed.put(&widget, 0.0, 0.0);
            // in a scrolled window, so a panel larger than the window is
            // still given its size
            let scrolled = gtk4::ScrolledWindow::new();
            scrolled.set_child(Some(&fixed));
            window.set_child(Some(&scrolled));
            for (i, (w, h)) in resizes.iter().copied().enumerate() {
                let widget = widget.clone();
                let (displayer, config) = (displayer.clone(), config.clone());
                glib::timeout_add_local_once(Duration::from_millis(resize_ms * (i as u64 + 1)), move || {
                    if reapply {
                        displayer.borrow_mut().apply_config(&config).expect("config applies");
                    }
                    widget.set_size_request(w, h);
                    widget.queue_resize();
                });
            }
        }
        window.present();

        if !snapshot.is_empty() {
            let (window, path) = (window.clone(), snapshot.clone());
            // Frames the window was given: a window the compositor is not
            // showing gets none, is never laid out and draws nothing.
            let frames = Rc::new(Cell::new(0u32));
            window.add_tick_callback({
                let frames = frames.clone();
                move |_, _| {
                    frames.set(frames.get() + 1);
                    glib::ControlFlow::Continue
                }
            });
            let after = if resizes.is_empty() { 4000 } else { resize_ms * resizes.len() as u64 + settle_ms };
            glib::timeout_add_local_once(Duration::from_millis(after), move || {
                let paintable = gtk4::WidgetPaintable::new(Some(&widget));
                let (w, h) = (widget.width() as f64, widget.height() as f64);
                let snap = gtk4::Snapshot::new();
                paintable.snapshot(&snap, w, h);
                let saved = snap
                    .to_node()
                    .zip(window.renderer())
                    .map(|(node, renderer)| renderer.render_texture(&node, None).save_to_png(&path));
                eprintln!("snapshot {w}x{h} after {} frames -> {path}: {saved:?}", frames.get());
            });
        }

        // Feed the fixed values, or changing ones.
        if let Some(feed) = feed.clone() {
            let displayer = displayer.clone();
            glib::timeout_add_local(Duration::from_millis(feed_ms), move || {
                displayer.borrow_mut().update_data(&feed);
                glib::ControlFlow::Continue
            });
        } else if let Some(fixed) = fixed.clone() {
            let displayer = displayer.clone();
            let slots = slots.clone();
            glib::timeout_add_local(Duration::from_millis(feed_ms), move || {
                let values: HashMap<String, Value> = slots
                    .iter()
                    .enumerate()
                    .filter_map(|(i, slot)| {
                        let text = fixed.get(&i.to_string())?;
                        Some((format!("{slot}_value"), json!(text)))
                    })
                    .collect();
                displayer.borrow_mut().update_data(&values);
                glib::ControlFlow::Continue
            });
        } else if updates {
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
