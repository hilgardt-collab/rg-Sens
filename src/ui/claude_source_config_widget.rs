//! Claude usage source configuration widget

use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, DropDown, Entry, Label, Orientation, Separator, SpinButton, StringList,
};
use std::cell::RefCell;
use std::rc::Rc;

use crate::ui::widget_builder::create_page_container;
use rg_sens_sources::claude_auth::{self, PendingAuth};

// Re-export Claude source config types from rg-sens-types
pub use rg_sens_types::source_configs::claude::{ClaudeMetric, ClaudeSourceConfig};

/// Open a URL in the user's default browser (best-effort; xdg-open on Linux).
fn open_url(url: &str) {
    let _ = gtk4::gio::AppInfo::launch_default_for_uri(
        url,
        Option::<&gtk4::gio::AppLaunchContext>::None,
    );
}

/// Account status line, reflecting whether rg-Sens holds its own token.
fn account_status_text(signed_in: bool) -> String {
    if signed_in {
        "✓ Signed in to rg-Sens — the token refreshes automatically.".to_string()
    } else {
        "Not signed in. Plan-usage % falls back to Claude Code's login (read-only) \
         if present, but sign in here so rg-Sens can refresh its own token."
            .to_string()
    }
}

/// Widget for configuring the unified Claude usage source.
#[allow(dead_code)]
pub struct ClaudeSourceConfigWidget {
    widget: GtkBox,
    config: Rc<RefCell<ClaudeSourceConfig>>,
    caption_entry: Entry,
    metric_combo: DropDown,
    update_interval_spin: SpinButton,
}

/// Map a dropdown index to a metric (mirrors `ClaudeMetric::ALL` order).
fn metric_from_index(i: u32) -> ClaudeMetric {
    ClaudeMetric::ALL
        .get(i as usize)
        .map(|(m, _)| *m)
        .unwrap_or_default()
}

/// Map a metric to its dropdown index.
fn index_from_metric(metric: ClaudeMetric) -> u32 {
    ClaudeMetric::ALL
        .iter()
        .position(|(m, _)| *m == metric)
        .unwrap_or(0) as u32
}

impl ClaudeSourceConfigWidget {
    pub fn new() -> Self {
        let widget = create_page_container();
        let config = Rc::new(RefCell::new(ClaudeSourceConfig::default()));

        // Metric selection
        let metric_box = GtkBox::new(Orientation::Horizontal, 6);
        metric_box.append(&Label::new(Some("Metric:")));
        let labels: Vec<&str> = ClaudeMetric::ALL.iter().map(|(_, l)| *l).collect();
        let metric_options = StringList::new(&labels);
        let metric_combo = DropDown::new(Some(metric_options), Option::<gtk4::Expression>::None);
        metric_combo.set_hexpand(true);
        metric_box.append(&metric_combo);
        widget.append(&metric_box);

        // Custom caption
        let caption_box = GtkBox::new(Orientation::Horizontal, 6);
        caption_box.append(&Label::new(Some("Custom Caption:")));
        let caption_entry = Entry::new();
        caption_entry.set_placeholder_text(Some("Auto-generated if empty"));
        caption_entry.set_hexpand(true);
        caption_box.append(&caption_entry);
        widget.append(&caption_box);

        // Update interval
        let interval_box = GtkBox::new(Orientation::Horizontal, 6);
        interval_box.append(&Label::new(Some("Update Interval (ms):")));
        let interval_adjustment =
            gtk4::Adjustment::new(60000.0, 5000.0, 600000.0, 1000.0, 10000.0, 0.0);
        let update_interval_spin = SpinButton::new(Some(&interval_adjustment), 1000.0, 0);
        update_interval_spin.set_hexpand(true);
        interval_box.append(&update_interval_spin);
        widget.append(&interval_box);

        // Note about plan-usage metrics
        let note = Label::new(Some(
            "Percentage / reset metrics query Anthropic's usage API. Token metrics\n\
             read local Claude Code transcripts and never go to the network.",
        ));
        note.set_halign(gtk4::Align::Start);
        note.add_css_class("dim-label");
        widget.append(&note);

        // --- Claude account / OAuth sign-in ------------------------------
        widget.append(&Separator::new(Orientation::Horizontal));
        let account_title = Label::new(Some("Claude Account"));
        account_title.set_halign(gtk4::Align::Start);
        account_title.add_css_class("heading");
        widget.append(&account_title);

        let status_label = Label::new(None);
        status_label.set_halign(gtk4::Align::Start);
        status_label.set_wrap(true);
        status_label.set_xalign(0.0);
        status_label.add_css_class("dim-label");
        widget.append(&status_label);

        let btn_row = GtkBox::new(Orientation::Horizontal, 6);
        let sign_in_btn = Button::with_label("Sign in to Claude");
        let sign_out_btn = Button::with_label("Sign out");
        btn_row.append(&sign_in_btn);
        btn_row.append(&sign_out_btn);
        widget.append(&btn_row);

        // Paste row, hidden until a sign-in is in progress.
        let paste_row = GtkBox::new(Orientation::Horizontal, 6);
        let paste_entry = Entry::new();
        paste_entry.set_placeholder_text(Some("Paste the code from the browser"));
        paste_entry.set_hexpand(true);
        let complete_btn = Button::with_label("Complete sign-in");
        paste_row.append(&paste_entry);
        paste_row.append(&complete_btn);
        paste_row.set_visible(false);
        widget.append(&paste_row);

        // Reusable UI refresher: reflect current sign-in state.
        let refresh_account: Rc<dyn Fn()> = {
            let status_label = status_label.clone();
            let sign_out_btn = sign_out_btn.clone();
            Rc::new(move || {
                let signed = claude_auth::is_signed_in();
                status_label.set_text(&account_status_text(signed));
                sign_out_btn.set_visible(signed);
            })
        };
        refresh_account();

        // In-flight PKCE state, set on "Sign in", consumed on "Complete".
        let pending: Rc<RefCell<Option<PendingAuth>>> = Rc::new(RefCell::new(None));

        // Sign in: open the browser and reveal the paste row.
        {
            let pending = pending.clone();
            let paste_row = paste_row.clone();
            let paste_entry = paste_entry.clone();
            let status_label = status_label.clone();
            sign_in_btn.connect_clicked(move |_| {
                let auth = claude_auth::begin();
                open_url(&auth.authorize_url());
                *pending.borrow_mut() = Some(auth);
                paste_row.set_visible(true);
                paste_entry.set_text("");
                paste_entry.grab_focus();
                status_label.set_text(
                    "Approve access in your browser, then paste the code shown there \
                     and click \"Complete sign-in\".",
                );
            });
        }

        // Complete: exchange the pasted code off the GTK thread, then refresh UI.
        {
            let pending = pending.clone();
            let paste_entry = paste_entry.clone();
            let paste_row = paste_row.clone();
            let status_label = status_label.clone();
            let complete_btn_inner = complete_btn.clone();
            let refresh_account = refresh_account.clone();
            complete_btn.connect_clicked(move |_| {
                let Some(auth) = pending.borrow_mut().take() else {
                    return;
                };
                let pasted = paste_entry.text().to_string();
                status_label.set_text("Signing in…");
                complete_btn_inner.set_sensitive(false);

                let paste_entry = paste_entry.clone();
                let paste_row = paste_row.clone();
                let status_label = status_label.clone();
                let complete_btn = complete_btn_inner.clone();
                let refresh_account = refresh_account.clone();
                gtk4::glib::MainContext::default().spawn_local(async move {
                    // Blocking token exchange runs off the main thread.
                    let result =
                        gtk4::gio::spawn_blocking(move || claude_auth::complete(&auth, &pasted))
                            .await;
                    complete_btn.set_sensitive(true);
                    match result {
                        Ok(Ok(())) => {
                            paste_entry.set_text("");
                            paste_row.set_visible(false);
                            refresh_account();
                        }
                        Ok(Err(e)) => status_label.set_text(&format!(
                            "Sign-in failed: {e}. Click \"Sign in to Claude\" to retry."
                        )),
                        Err(_) => status_label
                            .set_text("Sign-in failed (internal error). Please retry."),
                    }
                });
            });
        }

        // Sign out: forget rg-Sens's token (Claude Code is untouched).
        {
            let refresh_account = refresh_account.clone();
            let paste_row = paste_row.clone();
            sign_out_btn.connect_clicked(move |_| {
                claude_auth::sign_out();
                paste_row.set_visible(false);
                refresh_account();
            });
        }

        // Wire up handlers
        let config_clone = config.clone();
        metric_combo.connect_selected_notify(move |combo| {
            config_clone.borrow_mut().metric = metric_from_index(combo.selected());
        });

        let config_clone = config.clone();
        caption_entry.connect_changed(move |entry| {
            let text = entry.text().to_string();
            config_clone.borrow_mut().custom_caption =
                if text.is_empty() { None } else { Some(text) };
        });

        let config_clone = config.clone();
        update_interval_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().update_interval_ms = spin.value() as u64;
        });

        Self {
            widget,
            config,
            caption_entry,
            metric_combo,
            update_interval_spin,
        }
    }

    pub fn widget(&self) -> &GtkBox {
        &self.widget
    }

    pub fn set_config(&self, config: ClaudeSourceConfig) {
        self.metric_combo.set_selected(index_from_metric(config.metric));

        if let Some(ref caption) = config.custom_caption {
            self.caption_entry.set_text(caption);
        } else {
            self.caption_entry.set_text("");
        }

        self.update_interval_spin
            .set_value(config.update_interval_ms as f64);

        *self.config.borrow_mut() = config;
    }

    pub fn get_config(&self) -> ClaudeSourceConfig {
        self.config.borrow().clone()
    }
}

impl Default for ClaudeSourceConfigWidget {
    fn default() -> Self {
        Self::new()
    }
}
