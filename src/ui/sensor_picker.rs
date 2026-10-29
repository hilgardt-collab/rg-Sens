//! Sensor dropdown shared by the fan speed and system temperature config widgets.
//!
//! Row 0 is a placeholder ("(no fan selected)") so that a saved sensor that is
//! not present on this system can be shown as "nothing selected" with a
//! warning underneath. GTK's `DropDown::set_selected(INVALID_LIST_POSITION)`
//! is a no-op (verified by probe), so a real row is the only way to display
//! that state, and it also guarantees that choosing any real sensor afterwards
//! changes the selection and fires `notify::selected`. Rows 1.. map to sensor
//! indices 0.. .

use gtk4::prelude::*;
use gtk4::{Box as GtkBox, DropDown, Label, Orientation, StringList};

pub struct SensorPicker {
    row: GtkBox,
    combo: DropDown,
    warning: Label,
    /// "fan" / "sensor", for messages
    kind: &'static str,
}

impl SensorPicker {
    /// `labels` are the sensor display names in index order. `on_pick(index)`
    /// runs whenever the user (or `select`) chooses a real sensor row.
    pub fn new<F>(title: &str, kind: &'static str, labels: &[String], on_pick: F) -> Self
    where
        F: Fn(usize) + 'static,
    {
        let placeholder = format!("(no {} selected)", kind);
        let mut rows: Vec<&str> = vec![placeholder.as_str()];
        rows.extend(labels.iter().map(String::as_str));

        let combo = DropDown::new(
            Some(StringList::new(&rows)),
            Option::<gtk4::Expression>::None,
        );
        combo.set_hexpand(true);

        let row = GtkBox::new(Orientation::Horizontal, 6);
        row.append(&Label::new(Some(title)));
        row.append(&combo);

        let warning = Label::new(None);
        warning.set_halign(gtk4::Align::Start);
        warning.set_wrap(true);
        warning.add_css_class("warning");
        warning.set_visible(false);

        let warning_clone = warning.clone();
        combo.connect_selected_notify(move |combo| {
            if let Some(index) = Self::index_of_row(combo.selected()) {
                warning_clone.set_visible(false);
                on_pick(index);
            }
        });

        // A fresh widget starts on the first real sensor, like before the
        // placeholder row existed (new panels get a usable default)
        if !labels.is_empty() {
            combo.set_selected(1);
        }

        Self {
            row,
            combo,
            warning,
            kind,
        }
    }

    /// Append the dropdown row and its warning label to `container`
    pub fn append_to(&self, container: &GtkBox) {
        container.append(&self.row);
        container.append(&self.warning);
    }

    /// Sensor index of a dropdown row (None for the placeholder / no selection)
    fn index_of_row(row: u32) -> Option<usize> {
        (row != gtk4::INVALID_LIST_POSITION && row > 0).then(|| row as usize - 1)
    }

    /// Currently chosen sensor index, or None while the placeholder is showing
    pub fn selected(&self) -> Option<usize> {
        Self::index_of_row(self.combo.selected())
    }

    /// Show the resolved sensor, or the placeholder plus a warning naming the
    /// saved selection (`description` as from `describe_sensor_selection`)
    pub fn set_resolved(&self, resolved: Option<usize>, description: &str) {
        match resolved {
            Some(index) => {
                self.warning.set_visible(false);
                self.combo.set_selected(index as u32 + 1);
            }
            None => {
                self.combo.set_selected(0);
                self.warning.set_text(&format!(
                    "Saved {kind} {description} was not found on this system, so the panel shows N/A. \
                     Pick a {kind} above to replace it.",
                    kind = self.kind
                ));
                self.warning.set_visible(true);
            }
        }
    }
}
