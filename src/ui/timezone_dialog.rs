//! Timezone selection dialog with search functionality

use gtk4::glib::WeakRef;
use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, Entry, Label, ListBox, ListBoxRow, Orientation, ScrolledWindow, Window,
};
use std::cell::RefCell;
use std::rc::Rc;

thread_local! {
    static TIMEZONE_DIALOG: RefCell<Option<WeakRef<Window>>> = const { RefCell::new(None) };
}

/// Close the timezone dialog if it's open
pub fn close_timezone_dialog() {
    TIMEZONE_DIALOG.with(|dialog_ref| {
        let mut dialog_opt = dialog_ref.borrow_mut();
        if let Some(weak) = dialog_opt.take() {
            if let Some(dialog) = weak.upgrade() {
                dialog.close();
            }
        }
    });
}

/// Result state for the async dialog
#[derive(Clone, Debug)]
enum DialogResult {
    Pending,
    Selected(String),
    Cancelled,
}

/// All available timezones from chrono-tz, grouped by region
const TIMEZONES: &[&str] = &[
    // Special
    "Local",
    "UTC",
    // Africa
    "Africa/Abidjan",
    "Africa/Accra",
    "Africa/Addis_Ababa",
    "Africa/Algiers",
    "Africa/Cairo",
    "Africa/Casablanca",
    "Africa/Johannesburg",
    "Africa/Lagos",
    "Africa/Nairobi",
    "Africa/Tunis",
    // America
    "America/Anchorage",
    "America/Argentina/Buenos_Aires",
    "America/Bogota",
    "America/Chicago",
    "America/Denver",
    "America/Detroit",
    "America/Edmonton",
    "America/Halifax",
    "America/Havana",
    "America/Lima",
    "America/Los_Angeles",
    "America/Mexico_City",
    "America/Montreal",
    "America/New_York",
    "America/Panama",
    "America/Phoenix",
    "America/Santiago",
    "America/Sao_Paulo",
    "America/St_Johns",
    "America/Toronto",
    "America/Vancouver",
    // Asia
    "Asia/Almaty",
    "Asia/Baghdad",
    "Asia/Bangkok",
    "Asia/Beirut",
    "Asia/Calcutta",
    "Asia/Dhaka",
    "Asia/Dubai",
    "Asia/Ho_Chi_Minh",
    "Asia/Hong_Kong",
    "Asia/Istanbul",
    "Asia/Jakarta",
    "Asia/Jerusalem",
    "Asia/Kabul",
    "Asia/Karachi",
    "Asia/Kathmandu",
    "Asia/Kolkata",
    "Asia/Kuala_Lumpur",
    "Asia/Kuwait",
    "Asia/Manila",
    "Asia/Riyadh",
    "Asia/Seoul",
    "Asia/Shanghai",
    "Asia/Singapore",
    "Asia/Taipei",
    "Asia/Tehran",
    "Asia/Tokyo",
    // Atlantic
    "Atlantic/Azores",
    "Atlantic/Canary",
    "Atlantic/Reykjavik",
    // Australia
    "Australia/Adelaide",
    "Australia/Brisbane",
    "Australia/Darwin",
    "Australia/Hobart",
    "Australia/Melbourne",
    "Australia/Perth",
    "Australia/Sydney",
    // Europe
    "Europe/Amsterdam",
    "Europe/Athens",
    "Europe/Belgrade",
    "Europe/Berlin",
    "Europe/Brussels",
    "Europe/Bucharest",
    "Europe/Budapest",
    "Europe/Copenhagen",
    "Europe/Dublin",
    "Europe/Helsinki",
    "Europe/Kyiv",
    "Europe/Lisbon",
    "Europe/London",
    "Europe/Madrid",
    "Europe/Moscow",
    "Europe/Oslo",
    "Europe/Paris",
    "Europe/Prague",
    "Europe/Rome",
    "Europe/Stockholm",
    "Europe/Vienna",
    "Europe/Warsaw",
    "Europe/Zurich",
    // Indian
    "Indian/Maldives",
    "Indian/Mauritius",
    // Pacific
    "Pacific/Auckland",
    "Pacific/Fiji",
    "Pacific/Guam",
    "Pacific/Honolulu",
    "Pacific/Noumea",
    "Pacific/Pago_Pago",
    "Pacific/Port_Moresby",
    "Pacific/Tahiti",
    "Pacific/Tongatapu",
];

/// Timezone selection dialog
pub struct TimezoneDialog;

impl TimezoneDialog {
    /// Show timezone picker and return selected timezone
    pub async fn pick_timezone(parent: Option<&Window>, current: &str) -> Option<String> {
        use std::future::Future;
        use std::pin::Pin;
        use std::task::{Context, Poll, Waker};

        let dialog = Window::builder()
            .title("Select Timezone")
            .modal(false)
            .default_width(400)
            .default_height(500)
            .resizable(true)
            .build();

        if let Some(parent) = parent {
            dialog.set_transient_for(Some(parent));
        }

        // Close any existing dialog (singleton pattern)
        TIMEZONE_DIALOG.with(|dialog_ref| {
            let mut dialog_opt = dialog_ref.borrow_mut();
            if let Some(weak) = dialog_opt.as_ref() {
                if let Some(existing) = weak.upgrade() {
                    existing.close();
                }
            }
            // Store the new dialog
            *dialog_opt = Some(dialog.downgrade());
        });

        let result: Rc<RefCell<DialogResult>> = Rc::new(RefCell::new(DialogResult::Pending));
        let waker: Rc<RefCell<Option<Waker>>> = Rc::new(RefCell::new(None));

        let main_box = GtkBox::new(Orientation::Vertical, 8);
        main_box.set_margin_start(12);
        main_box.set_margin_end(12);
        main_box.set_margin_top(12);
        main_box.set_margin_bottom(12);

        // Current selection label
        let current_label = Label::new(Some(&format!("Current: {}", current)));
        current_label.set_halign(gtk4::Align::Start);
        current_label.add_css_class("heading");
        main_box.append(&current_label);

        // Search entry
        let search_entry = Entry::new();
        search_entry.set_placeholder_text(Some("Search timezones..."));
        search_entry.set_margin_bottom(8);
        main_box.append(&search_entry);

        // Scrollable list of timezones
        let scrolled = ScrolledWindow::new();
        scrolled.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);
        scrolled.set_vexpand(true);

        let list_box = ListBox::new();
        list_box.set_selection_mode(gtk4::SelectionMode::Single);

        // Populate the list
        for tz in TIMEZONES.iter() {
            let row = Self::create_timezone_row(tz, tz == &current);
            list_box.append(&row);
        }

        scrolled.set_child(Some(&list_box));
        main_box.append(&scrolled);

        // Search functionality
        let list_box_for_search = list_box.clone();
        search_entry.connect_changed(move |entry| {
            let search_text = entry.text().to_lowercase();
            let mut index = 0;
            while let Some(row) = list_box_for_search.row_at_index(index) {
                if let Some(child) = row.child() {
                    if let Some(label) = child.downcast_ref::<Label>() {
                        let tz_name = label.text().to_lowercase();
                        row.set_visible(search_text.is_empty() || tz_name.contains(&search_text));
                    }
                }
                index += 1;
            }
        });

        // Buttons
        let button_box = GtkBox::new(Orientation::Horizontal, 6);
        button_box.set_halign(gtk4::Align::End);
        button_box.set_margin_top(12);

        let cancel_button = Button::with_label("Cancel");
        let ok_button = Button::with_label("OK");
        ok_button.add_css_class("suggested-action");

        button_box.append(&cancel_button);
        button_box.append(&ok_button);
        main_box.append(&button_box);

        dialog.set_child(Some(&main_box));

        // Handle row activation (double-click or Enter)
        let result_for_activate = result.clone();
        let waker_for_activate = waker.clone();
        let dialog_for_activate = dialog.clone();
        list_box.connect_row_activated(move |_, row| {
            if let Some(child) = row.child() {
                if let Some(label) = child.downcast_ref::<Label>() {
                    // Extract timezone name (remove the " ✓" suffix if present)
                    let mut tz = label.text().to_string();
                    if tz.ends_with(" ✓") {
                        tz = tz.trim_end_matches(" ✓").to_string();
                    }
                    *result_for_activate.borrow_mut() = DialogResult::Selected(tz);
                    dialog_for_activate.close();
                    if let Some(waker) = waker_for_activate.borrow_mut().take() {
                        waker.wake();
                    }
                }
            }
        });

        // Handle OK button
        let result_for_ok = result.clone();
        let waker_for_ok = waker.clone();
        let dialog_for_ok = dialog.clone();
        let list_box_for_ok = list_box.clone();
        ok_button.connect_clicked(move |_| {
            if let Some(row) = list_box_for_ok.selected_row() {
                if let Some(child) = row.child() {
                    if let Some(label) = child.downcast_ref::<Label>() {
                        let mut tz = label.text().to_string();
                        if tz.ends_with(" ✓") {
                            tz = tz.trim_end_matches(" ✓").to_string();
                        }
                        *result_for_ok.borrow_mut() = DialogResult::Selected(tz);
                    }
                }
            } else {
                *result_for_ok.borrow_mut() = DialogResult::Cancelled;
            }
            dialog_for_ok.close();
            if let Some(waker) = waker_for_ok.borrow_mut().take() {
                waker.wake();
            }
        });

        // Handle Cancel button
        let result_for_cancel = result.clone();
        let waker_for_cancel = waker.clone();
        let dialog_for_cancel = dialog.clone();
        cancel_button.connect_clicked(move |_| {
            *result_for_cancel.borrow_mut() = DialogResult::Cancelled;
            dialog_for_cancel.close();
            if let Some(waker) = waker_for_cancel.borrow_mut().take() {
                waker.wake();
            }
        });

        // Handle window close
        let result_for_close = result.clone();
        let waker_for_close = waker.clone();
        dialog.connect_close_request(move |_| {
            // Only set to cancelled if still pending
            if matches!(*result_for_close.borrow(), DialogResult::Pending) {
                *result_for_close.borrow_mut() = DialogResult::Cancelled;
            }
            if let Some(waker) = waker_for_close.borrow_mut().take() {
                waker.wake();
            }
            // Clear the singleton reference
            TIMEZONE_DIALOG.with(|dialog_ref| {
                *dialog_ref.borrow_mut() = None;
            });
            gtk4::glib::Propagation::Proceed
        });

        dialog.present();

        // Wait for dialog to close
        struct DialogFuture {
            result: Rc<RefCell<DialogResult>>,
            waker: Rc<RefCell<Option<Waker>>>,
        }

        impl Future for DialogFuture {
            type Output = Option<String>;

            fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                let result = self.result.borrow().clone();
                match result {
                    DialogResult::Selected(tz) => Poll::Ready(Some(tz)),
                    DialogResult::Cancelled => Poll::Ready(None),
                    DialogResult::Pending => {
                        *self.waker.borrow_mut() = Some(cx.waker().clone());
                        Poll::Pending
                    }
                }
            }
        }

        DialogFuture { result, waker }.await
    }

    fn create_timezone_row(tz: &str, is_current: bool) -> ListBoxRow {
        let row = ListBoxRow::new();
        let label = if is_current {
            Label::new(Some(&format!("{} ✓", tz)))
        } else {
            Label::new(Some(tz))
        };
        label.set_halign(gtk4::Align::Start);
        label.set_margin_start(8);
        label.set_margin_end(8);
        label.set_margin_top(4);
        label.set_margin_bottom(4);
        row.set_child(Some(&label));
        row
    }
}
