//! Claude account block: rg-Sens's own OAuth sign-in (`claude_auth`), shown
//! below the Claude source's schema rows. Stateful (a PKCE sign-in in flight),
//! so it is a footer widget rather than a property.

use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, Entry, Label, Orientation, Separator};
use rg_sens_sources::claude_auth::{self, PendingAuth};
use std::cell::RefCell;
use std::rc::Rc;

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
         if present, but sign in here (with your claude.ai subscription account) \
         so rg-Sens can refresh its own token."
            .to_string()
    }
}

/// Status line, sign in / sign out, and the paste-code row revealed while a
/// sign-in is in progress.
pub fn account_widget() -> GtkBox {
    let widget = GtkBox::new(Orientation::Vertical, 6);
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
                "Approve access in your browser using your claude.ai subscription \
                 account (a Console/API account is refused by the usage endpoint), \
                 then paste the code shown there and click \"Complete sign-in\".",
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
                    gtk4::gio::spawn_blocking(move || claude_auth::complete(&auth, &pasted)).await;
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
                    Err(_) => {
                        status_label.set_text("Sign-in failed (internal error). Please retry.")
                    }
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

    widget
}
