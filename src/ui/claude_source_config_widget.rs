//! Claude usage source configuration widget

use gtk4::prelude::*;
use gtk4::{Box as GtkBox, DropDown, Entry, Label, Orientation, SpinButton, StringList};
use std::cell::RefCell;
use std::rc::Rc;

use crate::ui::widget_builder::create_page_container;

// Re-export Claude source config types from rg-sens-types
pub use rg_sens_types::source_configs::claude::{ClaudeMetric, ClaudeSourceConfig};

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
            "Percentage metrics query Anthropic's usage API using the local\n\
             Claude Code login (read-only). Token metrics read local transcripts.",
        ));
        note.set_halign(gtk4::Align::Start);
        note.add_css_class("dim-label");
        widget.append(&note);

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
