//! Alarm and Timer control dialog
//!
//! This dialog provides full control over the clock source's alarm and timer features.
//! It can be launched by clicking an icon on the clock panel.

use gtk4::prelude::*;
use gtk4::{
    Adjustment, Box as GtkBox, Button, CheckButton, Entry, Frame, Label, Orientation,
    Separator, SpinButton, StringList, DropDown, Window,
};
use std::cell::RefCell;
use std::rc::Rc;

use crate::sources::{AlarmConfig, TimerConfig, TimerMode, TimerState};

/// Callback type for when alarm/timer settings change
pub type OnChangeCallback = Box<dyn Fn(&AlarmConfig, &TimerConfig)>;

/// Callback type for timer control actions
pub type TimerActionCallback = Box<dyn Fn(TimerAction)>;

/// Callback type for alarm dismiss action
pub type AlarmDismissCallback = Box<dyn Fn()>;

/// Timer control actions
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TimerAction {
    Start,
    Pause,
    Resume,
    Stop,
}

/// Alarm and Timer control dialog
pub struct AlarmTimerDialog {
    window: Window,
    alarm_config: Rc<RefCell<AlarmConfig>>,
    timer_config: Rc<RefCell<TimerConfig>>,
    on_change: Rc<RefCell<Option<OnChangeCallback>>>,
    on_timer_action: Rc<RefCell<Option<TimerActionCallback>>>,
    on_alarm_dismiss: Rc<RefCell<Option<AlarmDismissCallback>>>,
    // Timer control buttons for state updates
    start_button: Button,
    pause_button: Button,
    resume_button: Button,
    stop_button: Button,
    timer_state_label: Label,
    // Alarm dismiss button
    dismiss_button: Button,
    alarm_status_label: Label,
}

impl AlarmTimerDialog {
    pub fn new(parent: Option<&Window>) -> Self {
        let window = Window::builder()
            .title("Alarm & Timer")
            .modal(true)
            .default_width(400)
            .default_height(500)
            .resizable(true)
            .build();

        if let Some(parent) = parent {
            window.set_transient_for(Some(parent));
        }

        let alarm_config = Rc::new(RefCell::new(AlarmConfig::default()));
        let timer_config = Rc::new(RefCell::new(TimerConfig::default()));
        let on_change: Rc<RefCell<Option<OnChangeCallback>>> = Rc::new(RefCell::new(None));
        let on_timer_action: Rc<RefCell<Option<TimerActionCallback>>> = Rc::new(RefCell::new(None));
        let on_alarm_dismiss: Rc<RefCell<Option<AlarmDismissCallback>>> = Rc::new(RefCell::new(None));

        let main_box = GtkBox::new(Orientation::Vertical, 12);
        main_box.set_margin_start(12);
        main_box.set_margin_end(12);
        main_box.set_margin_top(12);
        main_box.set_margin_bottom(12);

        // ===== ALARM SECTION =====
        let alarm_frame = Frame::new(Some("Alarm"));
        let alarm_box = GtkBox::new(Orientation::Vertical, 8);
        alarm_box.set_margin_start(8);
        alarm_box.set_margin_end(8);
        alarm_box.set_margin_top(8);
        alarm_box.set_margin_bottom(8);

        // Alarm enabled toggle
        let alarm_enabled_check = CheckButton::with_label("Enable Alarm");
        alarm_enabled_check.set_active(false);
        alarm_box.append(&alarm_enabled_check);

        // Alarm time
        let alarm_time_box = GtkBox::new(Orientation::Horizontal, 6);
        alarm_time_box.append(&Label::new(Some("Time:")));

        let alarm_hour_adj = Adjustment::new(7.0, 0.0, 23.0, 1.0, 1.0, 0.0);
        let alarm_hour_spin = SpinButton::new(Some(&alarm_hour_adj), 1.0, 0);
        alarm_hour_spin.set_width_chars(3);
        alarm_time_box.append(&alarm_hour_spin);

        alarm_time_box.append(&Label::new(Some(":")));

        let alarm_minute_adj = Adjustment::new(0.0, 0.0, 59.0, 1.0, 5.0, 0.0);
        let alarm_minute_spin = SpinButton::new(Some(&alarm_minute_adj), 1.0, 0);
        alarm_minute_spin.set_width_chars(3);
        alarm_time_box.append(&alarm_minute_spin);

        alarm_time_box.append(&Label::new(Some(":")));

        let alarm_second_adj = Adjustment::new(0.0, 0.0, 59.0, 1.0, 5.0, 0.0);
        let alarm_second_spin = SpinButton::new(Some(&alarm_second_adj), 1.0, 0);
        alarm_second_spin.set_width_chars(3);
        alarm_time_box.append(&alarm_second_spin);

        alarm_box.append(&alarm_time_box);

        // Alarm days
        let days_label = Label::new(Some("Days:"));
        days_label.set_halign(gtk4::Align::Start);
        alarm_box.append(&days_label);

        let days_box = GtkBox::new(Orientation::Horizontal, 4);
        let day_names = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
        let day_checks: Vec<CheckButton> = day_names
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let check = CheckButton::with_label(name);
                check.set_active((1..=5).contains(&i)); // Weekdays by default
                days_box.append(&check);
                check
            })
            .collect();
        alarm_box.append(&days_box);

        // Alarm label
        let alarm_label_box = GtkBox::new(Orientation::Horizontal, 6);
        alarm_label_box.append(&Label::new(Some("Label:")));

        let alarm_label_entry = Entry::new();
        alarm_label_entry.set_placeholder_text(Some("Wake up!"));
        alarm_label_entry.set_hexpand(true);
        alarm_label_box.append(&alarm_label_entry);
        alarm_box.append(&alarm_label_box);

        // Sound settings
        let alarm_sound_box = GtkBox::new(Orientation::Horizontal, 6);
        let alarm_sound_check = CheckButton::with_label("Play sound");
        alarm_sound_check.set_active(true);
        alarm_sound_box.append(&alarm_sound_check);

        let alarm_flash_check = CheckButton::with_label("Visual flash");
        alarm_flash_check.set_active(true);
        alarm_sound_box.append(&alarm_flash_check);
        alarm_box.append(&alarm_sound_box);

        // Alarm status display (shows when alarm is triggered)
        let alarm_status_box = GtkBox::new(Orientation::Horizontal, 6);
        alarm_status_box.set_margin_top(8);
        let alarm_status_label = Label::new(None);
        alarm_status_label.add_css_class("heading");
        alarm_status_label.set_visible(false);
        alarm_status_box.append(&alarm_status_label);

        // Dismiss alarm button (only shown when alarm is triggered)
        let dismiss_button = Button::with_label("Dismiss Alarm");
        dismiss_button.add_css_class("destructive-action");
        dismiss_button.set_visible(false);
        alarm_status_box.append(&dismiss_button);
        alarm_box.append(&alarm_status_box);

        alarm_frame.set_child(Some(&alarm_box));
        main_box.append(&alarm_frame);

        // ===== TIMER SECTION =====
        let timer_frame = Frame::new(Some("Timer"));
        let timer_box = GtkBox::new(Orientation::Vertical, 8);
        timer_box.set_margin_start(8);
        timer_box.set_margin_end(8);
        timer_box.set_margin_top(8);
        timer_box.set_margin_bottom(8);

        // Timer mode
        let timer_mode_box = GtkBox::new(Orientation::Horizontal, 6);
        timer_mode_box.append(&Label::new(Some("Mode:")));

        let timer_mode_options = StringList::new(&["Countdown", "Stopwatch"]);
        let timer_mode_dropdown = DropDown::new(Some(timer_mode_options), Option::<gtk4::Expression>::None);
        timer_mode_dropdown.set_selected(0);
        timer_mode_dropdown.set_hexpand(true);
        timer_mode_box.append(&timer_mode_dropdown);
        timer_box.append(&timer_mode_box);

        // Countdown duration
        let duration_box = GtkBox::new(Orientation::Horizontal, 6);
        duration_box.append(&Label::new(Some("Duration:")));

        let duration_min_adj = Adjustment::new(5.0, 0.0, 1440.0, 1.0, 5.0, 0.0);
        let duration_min_spin = SpinButton::new(Some(&duration_min_adj), 1.0, 0);
        duration_min_spin.set_width_chars(4);
        duration_box.append(&duration_min_spin);
        duration_box.append(&Label::new(Some("min")));

        let duration_sec_adj = Adjustment::new(0.0, 0.0, 59.0, 1.0, 5.0, 0.0);
        let duration_sec_spin = SpinButton::new(Some(&duration_sec_adj), 1.0, 0);
        duration_sec_spin.set_width_chars(3);
        duration_box.append(&duration_sec_spin);
        duration_box.append(&Label::new(Some("sec")));

        timer_box.append(&duration_box);

        // Timer label
        let timer_label_box = GtkBox::new(Orientation::Horizontal, 6);
        timer_label_box.append(&Label::new(Some("Label:")));

        let timer_label_entry = Entry::new();
        timer_label_entry.set_placeholder_text(Some("Timer"));
        timer_label_entry.set_hexpand(true);
        timer_label_box.append(&timer_label_entry);
        timer_box.append(&timer_label_box);

        // Timer sound settings
        let timer_sound_box = GtkBox::new(Orientation::Horizontal, 6);
        let timer_sound_check = CheckButton::with_label("Play sound");
        timer_sound_check.set_active(true);
        timer_sound_box.append(&timer_sound_check);

        let timer_flash_check = CheckButton::with_label("Visual flash");
        timer_flash_check.set_active(true);
        timer_sound_box.append(&timer_flash_check);
        timer_box.append(&timer_sound_box);

        // Timer state display
        let timer_state_box = GtkBox::new(Orientation::Horizontal, 6);
        timer_state_box.append(&Label::new(Some("Status:")));
        let timer_state_label = Label::new(Some("Stopped"));
        timer_state_label.add_css_class("heading");
        timer_state_box.append(&timer_state_label);
        timer_box.append(&timer_state_box);

        // Timer control buttons
        let timer_controls_box = GtkBox::new(Orientation::Horizontal, 6);
        timer_controls_box.set_halign(gtk4::Align::Center);
        timer_controls_box.set_margin_top(8);

        let start_button = Button::with_label("Start");
        start_button.add_css_class("suggested-action");
        start_button.set_width_request(80);
        timer_controls_box.append(&start_button);

        let pause_button = Button::with_label("Pause");
        pause_button.set_width_request(80);
        pause_button.set_sensitive(false);
        timer_controls_box.append(&pause_button);

        let resume_button = Button::with_label("Resume");
        resume_button.add_css_class("suggested-action");
        resume_button.set_width_request(80);
        resume_button.set_visible(false);
        timer_controls_box.append(&resume_button);

        let stop_button = Button::with_label("Stop");
        stop_button.add_css_class("destructive-action");
        stop_button.set_width_request(80);
        stop_button.set_sensitive(false);
        timer_controls_box.append(&stop_button);

        timer_box.append(&timer_controls_box);

        timer_frame.set_child(Some(&timer_box));
        main_box.append(&timer_frame);

        // ===== CLOSE BUTTON =====
        main_box.append(&Separator::new(Orientation::Horizontal));

        let close_button = Button::with_label("Close");
        close_button.set_halign(gtk4::Align::End);
        close_button.set_margin_top(8);
        main_box.append(&close_button);

        window.set_child(Some(&main_box));

        // ===== SIGNAL HANDLERS =====

        // Alarm enabled
        let alarm_config_clone = alarm_config.clone();
        let on_change_clone = on_change.clone();
        let timer_config_for_alarm = timer_config.clone();
        alarm_enabled_check.connect_toggled(move |check| {
            alarm_config_clone.borrow_mut().enabled = check.is_active();
            if let Some(ref cb) = *on_change_clone.borrow() {
                cb(&alarm_config_clone.borrow(), &timer_config_for_alarm.borrow());
            }
        });

        // Alarm hour
        let alarm_config_clone = alarm_config.clone();
        let on_change_clone = on_change.clone();
        let timer_config_for_alarm = timer_config.clone();
        alarm_hour_spin.connect_value_changed(move |spin| {
            alarm_config_clone.borrow_mut().hour = spin.value() as u32;
            if let Some(ref cb) = *on_change_clone.borrow() {
                cb(&alarm_config_clone.borrow(), &timer_config_for_alarm.borrow());
            }
        });

        // Alarm minute
        let alarm_config_clone = alarm_config.clone();
        let on_change_clone = on_change.clone();
        let timer_config_for_alarm = timer_config.clone();
        alarm_minute_spin.connect_value_changed(move |spin| {
            alarm_config_clone.borrow_mut().minute = spin.value() as u32;
            if let Some(ref cb) = *on_change_clone.borrow() {
                cb(&alarm_config_clone.borrow(), &timer_config_for_alarm.borrow());
            }
        });

        // Alarm second
        let alarm_config_clone = alarm_config.clone();
        let on_change_clone = on_change.clone();
        let timer_config_for_alarm = timer_config.clone();
        alarm_second_spin.connect_value_changed(move |spin| {
            alarm_config_clone.borrow_mut().second = spin.value() as u32;
            if let Some(ref cb) = *on_change_clone.borrow() {
                cb(&alarm_config_clone.borrow(), &timer_config_for_alarm.borrow());
            }
        });

        // Day checkboxes
        for (i, check) in day_checks.iter().enumerate() {
            let alarm_config_clone = alarm_config.clone();
            let on_change_clone = on_change.clone();
            let timer_config_for_alarm = timer_config.clone();
            let day_index = i as u32;
            check.connect_toggled(move |check| {
                let mut cfg = alarm_config_clone.borrow_mut();
                if check.is_active() {
                    if !cfg.days.contains(&day_index) {
                        cfg.days.push(day_index);
                        cfg.days.sort();
                    }
                } else {
                    cfg.days.retain(|&d| d != day_index);
                }
                drop(cfg);
                if let Some(ref cb) = *on_change_clone.borrow() {
                    cb(&alarm_config_clone.borrow(), &timer_config_for_alarm.borrow());
                }
            });
        }

        // Alarm label
        let alarm_config_clone = alarm_config.clone();
        let on_change_clone = on_change.clone();
        let timer_config_for_alarm = timer_config.clone();
        alarm_label_entry.connect_changed(move |entry| {
            let text = entry.text().to_string();
            alarm_config_clone.borrow_mut().label = if text.is_empty() { None } else { Some(text) };
            if let Some(ref cb) = *on_change_clone.borrow() {
                cb(&alarm_config_clone.borrow(), &timer_config_for_alarm.borrow());
            }
        });

        // Alarm sound enabled
        let alarm_config_clone = alarm_config.clone();
        let on_change_clone = on_change.clone();
        let timer_config_for_alarm = timer_config.clone();
        alarm_sound_check.connect_toggled(move |check| {
            alarm_config_clone.borrow_mut().sound.enabled = check.is_active();
            if let Some(ref cb) = *on_change_clone.borrow() {
                cb(&alarm_config_clone.borrow(), &timer_config_for_alarm.borrow());
            }
        });

        // Alarm visual flash enabled
        let alarm_config_clone = alarm_config.clone();
        let on_change_clone = on_change.clone();
        let timer_config_for_alarm = timer_config.clone();
        alarm_flash_check.connect_toggled(move |check| {
            alarm_config_clone.borrow_mut().sound.visual_enabled = check.is_active();
            if let Some(ref cb) = *on_change_clone.borrow() {
                cb(&alarm_config_clone.borrow(), &timer_config_for_alarm.borrow());
            }
        });

        // Timer mode
        let timer_config_clone = timer_config.clone();
        let on_change_clone = on_change.clone();
        let alarm_config_for_timer = alarm_config.clone();
        let duration_box_clone = duration_box.clone();
        timer_mode_dropdown.connect_selected_notify(move |dropdown| {
            let mode = match dropdown.selected() {
                0 => TimerMode::Countdown,
                _ => TimerMode::Stopwatch,
            };
            timer_config_clone.borrow_mut().mode = mode;
            // Show/hide duration controls based on mode
            duration_box_clone.set_visible(mode == TimerMode::Countdown);
            if let Some(ref cb) = *on_change_clone.borrow() {
                cb(&alarm_config_for_timer.borrow(), &timer_config_clone.borrow());
            }
        });

        // Timer duration (minutes)
        let timer_config_clone = timer_config.clone();
        let on_change_clone = on_change.clone();
        let alarm_config_for_timer = alarm_config.clone();
        let duration_sec_spin_clone = duration_sec_spin.clone();
        duration_min_spin.connect_value_changed(move |spin| {
            let minutes = spin.value() as u64;
            let seconds = duration_sec_spin_clone.value() as u64;
            timer_config_clone.borrow_mut().countdown_duration = minutes * 60 + seconds;
            if let Some(ref cb) = *on_change_clone.borrow() {
                cb(&alarm_config_for_timer.borrow(), &timer_config_clone.borrow());
            }
        });

        // Timer duration (seconds)
        let timer_config_clone = timer_config.clone();
        let on_change_clone = on_change.clone();
        let alarm_config_for_timer = alarm_config.clone();
        let duration_min_spin_clone = duration_min_spin.clone();
        duration_sec_spin.connect_value_changed(move |spin| {
            let minutes = duration_min_spin_clone.value() as u64;
            let seconds = spin.value() as u64;
            timer_config_clone.borrow_mut().countdown_duration = minutes * 60 + seconds;
            if let Some(ref cb) = *on_change_clone.borrow() {
                cb(&alarm_config_for_timer.borrow(), &timer_config_clone.borrow());
            }
        });

        // Timer label
        let timer_config_clone = timer_config.clone();
        let on_change_clone = on_change.clone();
        let alarm_config_for_timer = alarm_config.clone();
        timer_label_entry.connect_changed(move |entry| {
            let text = entry.text().to_string();
            timer_config_clone.borrow_mut().label = if text.is_empty() { None } else { Some(text) };
            if let Some(ref cb) = *on_change_clone.borrow() {
                cb(&alarm_config_for_timer.borrow(), &timer_config_clone.borrow());
            }
        });

        // Timer sound enabled
        let timer_config_clone = timer_config.clone();
        let on_change_clone = on_change.clone();
        let alarm_config_for_timer = alarm_config.clone();
        timer_sound_check.connect_toggled(move |check| {
            timer_config_clone.borrow_mut().sound.enabled = check.is_active();
            if let Some(ref cb) = *on_change_clone.borrow() {
                cb(&alarm_config_for_timer.borrow(), &timer_config_clone.borrow());
            }
        });

        // Timer visual flash enabled
        let timer_config_clone = timer_config.clone();
        let on_change_clone = on_change.clone();
        let alarm_config_for_timer = alarm_config.clone();
        timer_flash_check.connect_toggled(move |check| {
            timer_config_clone.borrow_mut().sound.visual_enabled = check.is_active();
            if let Some(ref cb) = *on_change_clone.borrow() {
                cb(&alarm_config_for_timer.borrow(), &timer_config_clone.borrow());
            }
        });

        // Timer control buttons - each updates both callback and UI state
        let on_timer_action_clone = on_timer_action.clone();
        let start_btn_ref = start_button.clone();
        let pause_btn_ref = pause_button.clone();
        let resume_btn_ref = resume_button.clone();
        let stop_btn_ref = stop_button.clone();
        let state_label_ref = timer_state_label.clone();
        start_button.connect_clicked(move |_| {
            if let Some(ref cb) = *on_timer_action_clone.borrow() {
                cb(TimerAction::Start);
            }
            // Update UI to Running state
            state_label_ref.set_text("Running");
            start_btn_ref.set_sensitive(false);
            pause_btn_ref.set_sensitive(true);
            pause_btn_ref.set_visible(true);
            resume_btn_ref.set_visible(false);
            stop_btn_ref.set_sensitive(true);
        });

        let on_timer_action_clone = on_timer_action.clone();
        let start_btn_ref = start_button.clone();
        let pause_btn_ref = pause_button.clone();
        let resume_btn_ref = resume_button.clone();
        let stop_btn_ref = stop_button.clone();
        let state_label_ref = timer_state_label.clone();
        pause_button.connect_clicked(move |_| {
            if let Some(ref cb) = *on_timer_action_clone.borrow() {
                cb(TimerAction::Pause);
            }
            // Update UI to Paused state
            state_label_ref.set_text("Paused");
            start_btn_ref.set_visible(false);
            pause_btn_ref.set_visible(false);
            resume_btn_ref.set_visible(true);
            resume_btn_ref.set_sensitive(true);
            stop_btn_ref.set_sensitive(true);
        });

        let on_timer_action_clone = on_timer_action.clone();
        let start_btn_ref = start_button.clone();
        let pause_btn_ref = pause_button.clone();
        let resume_btn_ref = resume_button.clone();
        let stop_btn_ref = stop_button.clone();
        let state_label_ref = timer_state_label.clone();
        resume_button.connect_clicked(move |_| {
            if let Some(ref cb) = *on_timer_action_clone.borrow() {
                cb(TimerAction::Resume);
            }
            // Update UI to Running state
            state_label_ref.set_text("Running");
            start_btn_ref.set_visible(true);
            start_btn_ref.set_sensitive(false);
            pause_btn_ref.set_visible(true);
            pause_btn_ref.set_sensitive(true);
            resume_btn_ref.set_visible(false);
            stop_btn_ref.set_sensitive(true);
        });

        let on_timer_action_clone = on_timer_action.clone();
        let start_btn_ref = start_button.clone();
        let pause_btn_ref = pause_button.clone();
        let resume_btn_ref = resume_button.clone();
        let stop_btn_ref = stop_button.clone();
        let state_label_ref = timer_state_label.clone();
        stop_button.connect_clicked(move |_| {
            if let Some(ref cb) = *on_timer_action_clone.borrow() {
                cb(TimerAction::Stop);
            }
            // Update UI to Stopped state
            state_label_ref.set_text("Stopped");
            start_btn_ref.set_visible(true);
            start_btn_ref.set_sensitive(true);
            pause_btn_ref.set_visible(true);
            pause_btn_ref.set_sensitive(false);
            resume_btn_ref.set_visible(false);
            stop_btn_ref.set_sensitive(false);
        });

        // Dismiss alarm button
        let on_alarm_dismiss_clone = on_alarm_dismiss.clone();
        let dismiss_btn_ref = dismiss_button.clone();
        let alarm_status_ref = alarm_status_label.clone();
        dismiss_button.connect_clicked(move |_| {
            if let Some(ref cb) = *on_alarm_dismiss_clone.borrow() {
                cb();
            }
            // Hide the dismiss button and status after dismissing
            dismiss_btn_ref.set_visible(false);
            alarm_status_ref.set_visible(false);
        });

        // Close button
        let window_clone = window.clone();
        close_button.connect_clicked(move |_| {
            window_clone.close();
        });

        Self {
            window,
            alarm_config,
            timer_config,
            on_change,
            on_timer_action,
            on_alarm_dismiss,
            start_button,
            pause_button,
            resume_button,
            stop_button,
            timer_state_label,
            dismiss_button,
            alarm_status_label,
        }
    }

    /// Show the dialog
    pub fn present(&self) {
        self.window.present();
    }

    /// Set the callback for when alarm/timer settings change
    pub fn set_on_change<F: Fn(&AlarmConfig, &TimerConfig) + 'static>(&self, callback: F) {
        *self.on_change.borrow_mut() = Some(Box::new(callback));
    }

    /// Set the callback for timer control actions
    pub fn set_on_timer_action<F: Fn(TimerAction) + 'static>(&self, callback: F) {
        *self.on_timer_action.borrow_mut() = Some(Box::new(callback));
    }

    /// Set the callback for alarm dismiss action
    pub fn set_on_alarm_dismiss<F: Fn() + 'static>(&self, callback: F) {
        *self.on_alarm_dismiss.borrow_mut() = Some(Box::new(callback));
    }

    /// Update the alarm triggered state (shows/hides dismiss button)
    pub fn update_alarm_triggered(&self, triggered: bool, label: Option<&str>) {
        self.dismiss_button.set_visible(triggered);
        self.alarm_status_label.set_visible(triggered);
        if triggered {
            let text = if let Some(lbl) = label {
                format!("⏰ ALARM: {}", lbl)
            } else {
                "⏰ ALARM TRIGGERED!".to_string()
            };
            self.alarm_status_label.set_text(&text);
            self.alarm_status_label.add_css_class("error");
        } else {
            self.alarm_status_label.remove_css_class("error");
        }
    }

    /// Update the dialog with current alarm config
    pub fn set_alarm_config(&self, config: &AlarmConfig) {
        *self.alarm_config.borrow_mut() = config.clone();
        // Note: Would need to store widget references to update them here
    }

    /// Update the dialog with current timer config
    pub fn set_timer_config(&self, config: &TimerConfig) {
        *self.timer_config.borrow_mut() = config.clone();
    }

    /// Update the timer state display and button visibility
    pub fn update_timer_state(&self, state: TimerState) {
        let state_text = match state {
            TimerState::Stopped => "Stopped",
            TimerState::Running => "Running",
            TimerState::Paused => "Paused",
            TimerState::Finished => "Finished!",
        };
        self.timer_state_label.set_text(state_text);

        // Update button visibility/sensitivity based on state
        match state {
            TimerState::Stopped => {
                self.start_button.set_sensitive(true);
                self.start_button.set_visible(true);
                self.pause_button.set_sensitive(false);
                self.pause_button.set_visible(true);
                self.resume_button.set_visible(false);
                self.stop_button.set_sensitive(false);
            }
            TimerState::Running => {
                self.start_button.set_sensitive(false);
                self.start_button.set_visible(true);
                self.pause_button.set_sensitive(true);
                self.pause_button.set_visible(true);
                self.resume_button.set_visible(false);
                self.stop_button.set_sensitive(true);
            }
            TimerState::Paused => {
                self.start_button.set_visible(false);
                self.pause_button.set_visible(false);
                self.resume_button.set_visible(true);
                self.resume_button.set_sensitive(true);
                self.stop_button.set_sensitive(true);
            }
            TimerState::Finished => {
                self.start_button.set_sensitive(true);
                self.start_button.set_visible(true);
                self.pause_button.set_sensitive(false);
                self.pause_button.set_visible(true);
                self.resume_button.set_visible(false);
                self.stop_button.set_sensitive(true);
            }
        }
    }

    /// Get the window for external use
    pub fn window(&self) -> &Window {
        &self.window
    }
}
