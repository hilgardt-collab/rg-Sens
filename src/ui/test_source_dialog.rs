//! Test source configuration dialog
//!
//! Provides UI for controlling the test data source

use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, DropDown, HeaderBar, Label, Orientation, Scale, SpinButton,
    StringList, Window,
};
use std::cell::RefCell;
use std::rc::Rc;

use crate::sources::{TestMode, TestSourceConfig, TEST_SOURCE_STATE};

/// Callback type for when test source config changes
pub type TestSourceSaveCallback = Box<dyn Fn(&TestSourceConfig) + 'static>;

/// Show the test source configuration dialog with optional save callback
pub fn show_test_source_dialog_with_callback(
    parent: &impl IsA<gtk4::Window>,
    on_save: Option<TestSourceSaveCallback>,
) {
    show_test_source_dialog_inner(parent, on_save);
}

/// Show the test source configuration dialog
pub fn show_test_source_dialog(parent: &impl IsA<gtk4::Window>) {
    show_test_source_dialog_inner(parent, None);
}

fn show_test_source_dialog_inner(
    parent: &impl IsA<gtk4::Window>,
    on_save: Option<TestSourceSaveCallback>,
) {
    let dialog = Window::builder()
        .title("Test Source Configuration")
        .transient_for(parent)
        .modal(false)
        .default_width(400)
        .default_height(350)
        .resizable(true)
        .build();

    // Header bar
    let header = HeaderBar::new();
    dialog.set_titlebar(Some(&header));

    // Content
    let content = GtkBox::new(Orientation::Vertical, 12);
    content.set_margin_start(12);
    content.set_margin_end(12);
    content.set_margin_top(12);
    content.set_margin_bottom(12);

    // Mode selection
    let mode_box = GtkBox::new(Orientation::Horizontal, 6);
    mode_box.append(&Label::new(Some("Mode:")));
    let mode_list = StringList::new(&["Manual", "Sine Wave", "Sawtooth", "Triangle", "Square"]);
    let mode_dropdown = DropDown::new(Some(mode_list), gtk4::Expression::NONE);
    mode_dropdown.set_hexpand(true);
    mode_box.append(&mode_dropdown);
    content.append(&mode_box);

    // Min/Max values
    let range_box = GtkBox::new(Orientation::Horizontal, 12);
    range_box.append(&Label::new(Some("Min:")));
    let min_spin = SpinButton::with_range(-1000.0, 1000.0, 1.0);
    min_spin.set_digits(1);
    range_box.append(&min_spin);
    range_box.append(&Label::new(Some("Max:")));
    let max_spin = SpinButton::with_range(-1000.0, 1000.0, 1.0);
    max_spin.set_digits(1);
    range_box.append(&max_spin);
    content.append(&range_box);

    // Manual value slider
    let manual_label = Label::new(Some("Manual Value:"));
    manual_label.set_halign(gtk4::Align::Start);
    content.append(&manual_label);

    let manual_box = GtkBox::new(Orientation::Horizontal, 6);
    let manual_scale = Scale::with_range(Orientation::Horizontal, 0.0, 100.0, 1.0);
    manual_scale.set_hexpand(true);
    manual_scale.set_draw_value(true);
    let manual_spin = SpinButton::with_range(-1000.0, 1000.0, 0.1);
    manual_spin.set_digits(1);
    manual_box.append(&manual_scale);
    manual_box.append(&manual_spin);
    content.append(&manual_box);

    // Period (for wave modes)
    let period_box = GtkBox::new(Orientation::Horizontal, 6);
    period_box.append(&Label::new(Some("Period (seconds):")));
    let period_spin = SpinButton::with_range(0.1, 60.0, 0.1);
    period_spin.set_digits(1);
    period_spin.set_hexpand(true);
    period_box.append(&period_spin);
    content.append(&period_box);

    // Current value display
    let value_box = GtkBox::new(Orientation::Horizontal, 6);
    value_box.append(&Label::new(Some("Current Value:")));
    let current_value_label = Label::new(Some("--"));
    current_value_label.set_hexpand(true);
    current_value_label.set_halign(gtk4::Align::Start);
    value_box.append(&current_value_label);
    content.append(&value_box);

    // Initialize from current state
    {
        let state = TEST_SOURCE_STATE.lock().unwrap();
        let config = &state.config;

        mode_dropdown.set_selected(match config.mode {
            TestMode::Manual => 0,
            TestMode::SineWave => 1,
            TestMode::Sawtooth => 2,
            TestMode::Triangle => 3,
            TestMode::Square => 4,
        });

        min_spin.set_value(config.min_value);
        max_spin.set_value(config.max_value);
        manual_scale.set_range(config.min_value, config.max_value);
        manual_scale.set_value(config.manual_value);
        manual_spin.set_value(config.manual_value);
        period_spin.set_value(config.period);
    }

    // Track if we're updating to avoid feedback loops
    let updating = Rc::new(RefCell::new(false));

    // Mode change handler
    let manual_label_clone = manual_label.clone();
    let manual_box_clone = manual_box.clone();
    let period_box_clone = period_box.clone();
    mode_dropdown.connect_selected_notify(move |dropdown| {
        let mode = match dropdown.selected() {
            0 => TestMode::Manual,
            1 => TestMode::SineWave,
            2 => TestMode::Sawtooth,
            3 => TestMode::Triangle,
            4 => TestMode::Square,
            _ => TestMode::Manual,
        };

        // Show/hide controls based on mode
        let is_manual = mode == TestMode::Manual;
        manual_label_clone.set_visible(is_manual);
        manual_box_clone.set_visible(is_manual);
        period_box_clone.set_visible(!is_manual);

        let mut state = TEST_SOURCE_STATE.lock().unwrap();
        state.config.mode = mode;
    });

    // Min/Max change handlers
    let manual_scale_clone = manual_scale.clone();
    let max_spin_clone = max_spin.clone();
    min_spin.connect_value_changed(move |spin| {
        let min = spin.value();
        let max = max_spin_clone.value();
        if min < max {
            manual_scale_clone.set_range(min, max);
            let mut state = TEST_SOURCE_STATE.lock().unwrap();
            state.config.min_value = min;
        }
    });

    let manual_scale_clone = manual_scale.clone();
    let min_spin_clone = min_spin.clone();
    max_spin.connect_value_changed(move |spin| {
        let max = spin.value();
        let min = min_spin_clone.value();
        if max > min {
            manual_scale_clone.set_range(min, max);
            let mut state = TEST_SOURCE_STATE.lock().unwrap();
            state.config.max_value = max;
        }
    });

    // Manual value handlers (sync scale and spin)
    let updating_clone = updating.clone();
    let manual_spin_clone = manual_spin.clone();
    manual_scale.connect_value_changed(move |scale| {
        if *updating_clone.borrow() {
            return;
        }
        *updating_clone.borrow_mut() = true;
        let value = scale.value();
        manual_spin_clone.set_value(value);
        let mut state = TEST_SOURCE_STATE.lock().unwrap();
        state.config.manual_value = value;
        *updating_clone.borrow_mut() = false;
    });

    let updating_clone = updating.clone();
    let manual_scale_clone = manual_scale.clone();
    manual_spin.connect_value_changed(move |spin| {
        if *updating_clone.borrow() {
            return;
        }
        *updating_clone.borrow_mut() = true;
        let value = spin.value();
        manual_scale_clone.set_value(value);
        let mut state = TEST_SOURCE_STATE.lock().unwrap();
        state.config.manual_value = value;
        *updating_clone.borrow_mut() = false;
    });

    // Period change handler
    period_spin.connect_value_changed(move |spin| {
        let mut state = TEST_SOURCE_STATE.lock().unwrap();
        state.config.period = spin.value();
    });

    // Update current value display periodically
    // Use weak reference so timer stops when dialog closes
    let current_value_label_weak = current_value_label.downgrade();
    gtk4::glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
        // Stop timer if label no longer exists (dialog closed)
        let Some(label) = current_value_label_weak.upgrade() else {
            return gtk4::glib::ControlFlow::Break;
        };

        // Use try_lock to avoid blocking if mutex is held
        let Ok(state) = TEST_SOURCE_STATE.try_lock() else {
            return gtk4::glib::ControlFlow::Continue;
        };
        let config = &state.config;
        let range = config.max_value - config.min_value;

        let value = match config.mode {
            TestMode::Manual => config.manual_value,
            TestMode::SineWave => {
                let elapsed = state.start_time.elapsed().as_secs_f64();
                let phase = (elapsed / config.period) * std::f64::consts::TAU;
                let normalized = (phase.sin() + 1.0) / 2.0;
                config.min_value + normalized * range
            }
            TestMode::Sawtooth => {
                let elapsed = state.start_time.elapsed().as_secs_f64();
                let normalized = (elapsed / config.period).fract();
                config.min_value + normalized * range
            }
            TestMode::Triangle => {
                let elapsed = state.start_time.elapsed().as_secs_f64();
                let phase = (elapsed / config.period).fract() * 2.0;
                let normalized = if phase <= 1.0 { phase } else { 2.0 - phase };
                config.min_value + normalized * range
            }
            TestMode::Square => {
                let elapsed = state.start_time.elapsed().as_secs_f64();
                let phase = (elapsed / config.period).fract();
                if phase < 0.5 { config.min_value } else { config.max_value }
            }
        };

        label.set_text(&format!("{:.1}", value));
        gtk4::glib::ControlFlow::Continue
    });

    // Set initial visibility based on mode
    {
        let state = TEST_SOURCE_STATE.lock().unwrap();
        let is_manual = state.config.mode == TestMode::Manual;
        manual_label.set_visible(is_manual);
        manual_box.set_visible(is_manual);
        period_box.set_visible(!is_manual);
    }

    // Wrap the save callback in Rc<RefCell> for sharing between handlers
    let on_save = Rc::new(RefCell::new(on_save));

    // Button box
    let button_box = GtkBox::new(Orientation::Horizontal, 6);
    button_box.set_halign(gtk4::Align::End);
    button_box.set_margin_top(12);

    // Save button - saves config and keeps dialog open
    let save_button = Button::with_label("Save");
    let on_save_clone = on_save.clone();
    save_button.connect_clicked(move |_| {
        if let Some(ref callback) = *on_save_clone.borrow() {
            if let Ok(state) = TEST_SOURCE_STATE.lock() {
                callback(&state.config);
            }
        }
    });
    button_box.append(&save_button);

    // Close button - saves and closes
    let close_button = Button::with_label("Close");
    let dialog_clone = dialog.clone();
    let on_save_clone = on_save.clone();
    close_button.connect_clicked(move |_| {
        // Save config before closing
        if let Some(ref callback) = *on_save_clone.borrow() {
            if let Ok(state) = TEST_SOURCE_STATE.lock() {
                callback(&state.config);
            }
        }
        dialog_clone.close();
    });
    button_box.append(&close_button);
    content.append(&button_box);

    // Also save on window close (X button)
    let on_save_for_close = on_save.clone();
    dialog.connect_close_request(move |_| {
        if let Some(ref callback) = *on_save_for_close.borrow() {
            if let Ok(state) = TEST_SOURCE_STATE.lock() {
                callback(&state.config);
            }
        }
        gtk4::glib::Propagation::Proceed
    });

    dialog.set_child(Some(&content));
    dialog.present();
}
