//! Configuration widget for Test source

use gtk4::prelude::*;
use gtk4::{
    Adjustment, Box as GtkBox, Button, Label, Orientation, SpinButton,
};
use std::cell::RefCell;
use std::rc::Rc;

use crate::sources::TestSourceConfig;

/// Widget for configuring Test source
pub struct TestSourceConfigWidget {
    widget: GtkBox,
    config: Rc<RefCell<TestSourceConfig>>,
    update_interval_spin: SpinButton,
}

impl TestSourceConfigWidget {
    pub fn new() -> Self {
        let widget = GtkBox::new(Orientation::Vertical, 12);
        widget.set_margin_start(12);
        widget.set_margin_end(12);
        widget.set_margin_top(12);
        widget.set_margin_bottom(12);

        let config = Rc::new(RefCell::new(TestSourceConfig::default()));

        // Info label
        let info_label = Label::new(Some("Test source for debugging and demonstration."));
        info_label.set_halign(gtk4::Align::Start);
        info_label.add_css_class("dim-label");
        widget.append(&info_label);

        // Update interval
        let interval_box = GtkBox::new(Orientation::Horizontal, 6);
        interval_box.append(&Label::new(Some("Update Interval (ms):")));

        let interval_adjustment = Adjustment::new(100.0, 10.0, 60000.0, 10.0, 100.0, 0.0);
        let update_interval_spin = SpinButton::new(Some(&interval_adjustment), 10.0, 0);
        update_interval_spin.set_hexpand(true);

        interval_box.append(&update_interval_spin);
        widget.append(&interval_box);

        // Note about Test Source dialog
        let note_label = Label::new(Some(
            "Use the Test Source dialog (Tools menu) to configure\nvalue mode, min/max, and wave settings.",
        ));
        note_label.set_halign(gtk4::Align::Start);
        note_label.add_css_class("dim-label");
        note_label.set_margin_top(12);
        widget.append(&note_label);

        // Open Test Source Dialog button
        let open_dialog_btn = Button::with_label("Open Test Source Dialog...");
        open_dialog_btn.set_halign(gtk4::Align::Start);
        open_dialog_btn.set_margin_top(6);

        open_dialog_btn.connect_clicked(move |button| {
            // Get the toplevel window
            if let Some(root) = button.root() {
                if let Some(window) = root.downcast_ref::<gtk4::Window>() {
                    crate::ui::show_test_source_dialog(window);
                }
            }
        });

        widget.append(&open_dialog_btn);

        // Wire up handlers
        let config_clone = config.clone();
        update_interval_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().update_interval_ms = spin.value() as u64;
        });

        Self {
            widget,
            config,
            update_interval_spin,
        }
    }

    pub fn widget(&self) -> &GtkBox {
        &self.widget
    }

    pub fn get_config(&self) -> TestSourceConfig {
        // Get the global state and merge with our update interval
        // Use blocking lock for consistency with set_config()
        let mut config = if let Ok(state) = crate::sources::TEST_SOURCE_STATE.lock() {
            state.config.clone()
        } else {
            // Fall back to local config only if mutex is poisoned
            log::warn!("TestSourceConfigWidget::get_config: Lock poisoned, using local config");
            self.config.borrow().clone()
        };
        config.update_interval_ms = self.config.borrow().update_interval_ms;
        config
    }

    pub fn set_config(&self, config: &TestSourceConfig) {
        // Update local state
        self.config.borrow_mut().update_interval_ms = config.update_interval_ms;

        // Update global TEST_SOURCE_STATE so saved config is restored on load
        if let Ok(mut state) = crate::sources::TEST_SOURCE_STATE.lock() {
            state.config = config.clone();
            log::debug!("TestSourceConfigWidget::set_config: Restored global state mode={:?}", config.mode);
        }

        // Update UI
        self.update_interval_spin.set_value(config.update_interval_ms as f64);
    }
}

impl Default for TestSourceConfigWidget {
    fn default() -> Self {
        Self::new()
    }
}
