//! Alarm and Timer list management dialog
//!
//! This dialog provides full control over multiple alarms and timers.
//! Users can add, edit, remove, and control alarms and timers from a list view.

use gtk4::prelude::*;
use gtk4::{
    Adjustment, Box as GtkBox, Button, CheckButton, Entry, Frame, Label, ListBox,
    ListBoxRow, Orientation, ScrolledWindow, Separator, SpinButton, StringList,
    DropDown, Window,
};
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

use crate::sources::{AlarmConfig, TimerConfig, TimerMode, TimerState};

/// Callback for when alarms/timers list changes
pub type OnListChangeCallback = Box<dyn Fn(&[AlarmConfig], &[TimerConfig])>;

/// Callback for timer control actions with timer ID
pub type TimerActionCallback = Box<dyn Fn(&str, TimerAction)>;

/// Callback for alarm dismiss with alarm ID
pub type AlarmDismissCallback = Box<dyn Fn(&str)>;

/// Timer control actions
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TimerAction {
    Start,
    Pause,
    Resume,
    Stop,
}

impl std::fmt::Display for TimerAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TimerAction::Start => write!(f, "start"),
            TimerAction::Pause => write!(f, "pause"),
            TimerAction::Resume => write!(f, "resume"),
            TimerAction::Stop => write!(f, "stop"),
        }
    }
}

/// Alarm and Timer list management dialog
pub struct AlarmTimerDialog {
    window: Window,
    alarms: Rc<RefCell<Vec<AlarmConfig>>>,
    timers: Rc<RefCell<Vec<TimerConfig>>>,
    triggered_alarm_ids: Rc<RefCell<HashSet<String>>>,
    alarm_list_box: ListBox,
    timer_list_box: ListBox,
    on_list_change: Rc<RefCell<Option<OnListChangeCallback>>>,
    on_timer_action: Rc<RefCell<Option<TimerActionCallback>>>,
    on_alarm_dismiss: Rc<RefCell<Option<AlarmDismissCallback>>>,
}

impl AlarmTimerDialog {
    pub fn new(
        parent: Option<&Window>,
        initial_alarms: Vec<AlarmConfig>,
        initial_timers: Vec<TimerConfig>,
        triggered_ids: HashSet<String>,
    ) -> Self {
        let window = Window::builder()
            .title("Alarms & Timers")
            .modal(true)
            .default_width(500)
            .default_height(600)
            .resizable(true)
            .build();

        if let Some(parent) = parent {
            window.set_transient_for(Some(parent));
        }

        let alarms = Rc::new(RefCell::new(initial_alarms));
        let timers = Rc::new(RefCell::new(initial_timers));
        let triggered_alarm_ids = Rc::new(RefCell::new(triggered_ids));
        let on_list_change: Rc<RefCell<Option<OnListChangeCallback>>> = Rc::new(RefCell::new(None));
        let on_timer_action: Rc<RefCell<Option<TimerActionCallback>>> = Rc::new(RefCell::new(None));
        let on_alarm_dismiss: Rc<RefCell<Option<AlarmDismissCallback>>> = Rc::new(RefCell::new(None));

        let main_box = GtkBox::new(Orientation::Vertical, 12);
        main_box.set_margin_start(12);
        main_box.set_margin_end(12);
        main_box.set_margin_top(12);
        main_box.set_margin_bottom(12);

        // ===== ALARM SECTION =====
        let alarm_frame = Frame::new(Some("Alarms"));
        let alarm_vbox = GtkBox::new(Orientation::Vertical, 8);
        alarm_vbox.set_margin_start(8);
        alarm_vbox.set_margin_end(8);
        alarm_vbox.set_margin_top(8);
        alarm_vbox.set_margin_bottom(8);

        // Scrollable alarm list
        let alarm_scroll = ScrolledWindow::new();
        alarm_scroll.set_min_content_height(150);
        alarm_scroll.set_vexpand(true);

        let alarm_list_box = ListBox::new();
        alarm_list_box.set_selection_mode(gtk4::SelectionMode::None);
        alarm_list_box.add_css_class("boxed-list");
        alarm_scroll.set_child(Some(&alarm_list_box));
        alarm_vbox.append(&alarm_scroll);

        // Add alarm button
        let add_alarm_button = Button::with_label("+ Add Alarm");
        add_alarm_button.set_halign(gtk4::Align::Start);
        alarm_vbox.append(&add_alarm_button);

        alarm_frame.set_child(Some(&alarm_vbox));
        main_box.append(&alarm_frame);

        // ===== TIMER SECTION =====
        let timer_frame = Frame::new(Some("Timers"));
        let timer_vbox = GtkBox::new(Orientation::Vertical, 8);
        timer_vbox.set_margin_start(8);
        timer_vbox.set_margin_end(8);
        timer_vbox.set_margin_top(8);
        timer_vbox.set_margin_bottom(8);

        // Scrollable timer list
        let timer_scroll = ScrolledWindow::new();
        timer_scroll.set_min_content_height(150);
        timer_scroll.set_vexpand(true);

        let timer_list_box = ListBox::new();
        timer_list_box.set_selection_mode(gtk4::SelectionMode::None);
        timer_list_box.add_css_class("boxed-list");
        timer_scroll.set_child(Some(&timer_list_box));
        timer_vbox.append(&timer_scroll);

        // Add timer button
        let add_timer_button = Button::with_label("+ Add Timer");
        add_timer_button.set_halign(gtk4::Align::Start);
        timer_vbox.append(&add_timer_button);

        timer_frame.set_child(Some(&timer_vbox));
        main_box.append(&timer_frame);

        // ===== CLOSE BUTTON =====
        main_box.append(&Separator::new(Orientation::Horizontal));

        let close_button = Button::with_label("Close");
        close_button.set_halign(gtk4::Align::End);
        close_button.set_margin_top(8);
        main_box.append(&close_button);

        window.set_child(Some(&main_box));

        let dialog = Self {
            window: window.clone(),
            alarms: alarms.clone(),
            timers: timers.clone(),
            triggered_alarm_ids: triggered_alarm_ids.clone(),
            alarm_list_box: alarm_list_box.clone(),
            timer_list_box: timer_list_box.clone(),
            on_list_change: on_list_change.clone(),
            on_timer_action: on_timer_action.clone(),
            on_alarm_dismiss: on_alarm_dismiss.clone(),
        };

        // Populate initial lists
        dialog.refresh_alarm_list();
        dialog.refresh_timer_list();

        // ===== SIGNAL HANDLERS =====

        // Add alarm button
        let alarms_clone = alarms.clone();
        let on_list_change_clone = on_list_change.clone();
        let timers_for_callback = timers.clone();
        let alarm_list_box_clone = alarm_list_box.clone();
        let triggered_ids_clone = triggered_alarm_ids.clone();
        let on_alarm_dismiss_clone = on_alarm_dismiss.clone();
        add_alarm_button.connect_clicked(move |_| {
            let new_alarm = AlarmConfig::default();
            alarms_clone.borrow_mut().push(new_alarm);

            // Refresh the list
            Self::refresh_alarm_list_static(
                &alarm_list_box_clone,
                &alarms_clone,
                &triggered_ids_clone,
                &on_list_change_clone,
                &timers_for_callback,
                &on_alarm_dismiss_clone,
            );

            // Notify change
            if let Some(ref cb) = *on_list_change_clone.borrow() {
                cb(&alarms_clone.borrow(), &timers_for_callback.borrow());
            }
        });

        // Add timer button
        let timers_clone = timers.clone();
        let on_list_change_clone = on_list_change.clone();
        let alarms_for_callback = alarms.clone();
        let timer_list_box_clone = timer_list_box.clone();
        let on_timer_action_clone = on_timer_action.clone();
        add_timer_button.connect_clicked(move |_| {
            let new_timer = TimerConfig::default();
            timers_clone.borrow_mut().push(new_timer);

            // Refresh the list
            Self::refresh_timer_list_static(
                &timer_list_box_clone,
                &timers_clone,
                &on_list_change_clone,
                &alarms_for_callback,
                &on_timer_action_clone,
            );

            // Notify change
            if let Some(ref cb) = *on_list_change_clone.borrow() {
                cb(&alarms_for_callback.borrow(), &timers_clone.borrow());
            }
        });

        // Close button
        let window_clone = window.clone();
        close_button.connect_clicked(move |_| {
            window_clone.close();
        });

        dialog
    }

    /// Refresh the alarm list display
    fn refresh_alarm_list(&self) {
        Self::refresh_alarm_list_static(
            &self.alarm_list_box,
            &self.alarms,
            &self.triggered_alarm_ids,
            &self.on_list_change,
            &self.timers,
            &self.on_alarm_dismiss,
        );
    }

    fn refresh_alarm_list_static(
        list_box: &ListBox,
        alarms: &Rc<RefCell<Vec<AlarmConfig>>>,
        triggered_ids: &Rc<RefCell<HashSet<String>>>,
        on_list_change: &Rc<RefCell<Option<OnListChangeCallback>>>,
        timers: &Rc<RefCell<Vec<TimerConfig>>>,
        on_alarm_dismiss: &Rc<RefCell<Option<AlarmDismissCallback>>>,
    ) {
        // Clear existing rows
        while let Some(child) = list_box.first_child() {
            list_box.remove(&child);
        }

        let alarms_borrowed = alarms.borrow();
        let triggered = triggered_ids.borrow();

        for (idx, alarm) in alarms_borrowed.iter().enumerate() {
            let is_triggered = triggered.contains(&alarm.id);
            let row = Self::create_alarm_row(
                alarm,
                idx,
                is_triggered,
                alarms.clone(),
                triggered_ids.clone(),
                on_list_change.clone(),
                timers.clone(),
                list_box.clone(),
                on_alarm_dismiss.clone(),
            );
            list_box.append(&row);
        }

        if alarms_borrowed.is_empty() {
            let empty_label = Label::new(Some("No alarms configured"));
            empty_label.add_css_class("dim-label");
            empty_label.set_margin_top(12);
            empty_label.set_margin_bottom(12);
            let row = ListBoxRow::new();
            row.set_child(Some(&empty_label));
            row.set_selectable(false);
            row.set_activatable(false);
            list_box.append(&row);
        }
    }

    fn create_alarm_row(
        alarm: &AlarmConfig,
        _idx: usize,
        is_triggered: bool,
        alarms: Rc<RefCell<Vec<AlarmConfig>>>,
        triggered_ids: Rc<RefCell<HashSet<String>>>,
        on_list_change: Rc<RefCell<Option<OnListChangeCallback>>>,
        timers: Rc<RefCell<Vec<TimerConfig>>>,
        list_box: ListBox,
        on_alarm_dismiss: Rc<RefCell<Option<AlarmDismissCallback>>>,
    ) -> ListBoxRow {
        let row = ListBoxRow::new();
        row.set_selectable(false);
        row.set_activatable(false);

        let alarm_id = alarm.id.clone();

        let hbox = GtkBox::new(Orientation::Horizontal, 8);
        hbox.set_margin_start(8);
        hbox.set_margin_end(8);
        hbox.set_margin_top(8);
        hbox.set_margin_bottom(8);

        // Enable toggle
        let enabled_check = CheckButton::new();
        enabled_check.set_active(alarm.enabled);
        enabled_check.set_tooltip_text(Some("Enable/Disable"));
        hbox.append(&enabled_check);

        // Time display
        let time_str = format!("{:02}:{:02}:{:02}", alarm.hour, alarm.minute, alarm.second);
        let time_label = Label::new(Some(&time_str));
        time_label.add_css_class("heading");
        time_label.set_width_chars(8);
        hbox.append(&time_label);

        // Days display
        let days_str = Self::format_days(&alarm.days);
        let days_label = Label::new(Some(&days_str));
        days_label.set_width_chars(12);
        days_label.add_css_class("dim-label");
        hbox.append(&days_label);

        // Label
        let label_text = alarm.label.clone().unwrap_or_default();
        let label_label = Label::new(Some(&label_text));
        label_label.set_hexpand(true);
        label_label.set_halign(gtk4::Align::Start);
        label_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        hbox.append(&label_label);

        // Triggered indicator and dismiss button
        if is_triggered {
            let triggered_label = Label::new(Some("RINGING"));
            triggered_label.add_css_class("error");
            hbox.append(&triggered_label);

            let dismiss_button = Button::with_label("Dismiss");
            dismiss_button.add_css_class("destructive-action");
            let alarm_id_for_dismiss = alarm_id.clone();
            let on_dismiss_clone = on_alarm_dismiss.clone();
            let triggered_ids_for_dismiss = triggered_ids.clone();
            let alarms_for_dismiss = alarms.clone();
            let timers_for_dismiss = timers.clone();
            let on_list_change_for_dismiss = on_list_change.clone();
            let list_box_for_dismiss = list_box.clone();
            let on_alarm_dismiss_for_dismiss = on_alarm_dismiss.clone();
            dismiss_button.connect_clicked(move |_| {
                if let Some(ref cb) = *on_dismiss_clone.borrow() {
                    cb(&alarm_id_for_dismiss);
                }
                triggered_ids_for_dismiss.borrow_mut().remove(&alarm_id_for_dismiss);
                Self::refresh_alarm_list_static(
                    &list_box_for_dismiss,
                    &alarms_for_dismiss,
                    &triggered_ids_for_dismiss,
                    &on_list_change_for_dismiss,
                    &timers_for_dismiss,
                    &on_alarm_dismiss_for_dismiss,
                );
            });
            hbox.append(&dismiss_button);
        }

        // Edit button
        let edit_button = Button::with_label("Edit");
        let alarm_clone = alarm.clone();
        let alarms_for_edit = alarms.clone();
        let triggered_for_edit = triggered_ids.clone();
        let on_list_change_for_edit = on_list_change.clone();
        let timers_for_edit = timers.clone();
        let list_box_for_edit = list_box.clone();
        let on_alarm_dismiss_for_edit = on_alarm_dismiss.clone();
        edit_button.connect_clicked(move |_| {
            Self::show_alarm_edit_dialog(
                &alarm_clone,
                alarms_for_edit.clone(),
                triggered_for_edit.clone(),
                on_list_change_for_edit.clone(),
                timers_for_edit.clone(),
                list_box_for_edit.clone(),
                on_alarm_dismiss_for_edit.clone(),
            );
        });
        hbox.append(&edit_button);

        // Delete button
        let delete_button = Button::with_label("Delete");
        delete_button.add_css_class("destructive-action");
        let alarm_id_for_delete = alarm_id.clone();
        let alarms_for_delete = alarms.clone();
        let triggered_for_delete = triggered_ids.clone();
        let on_list_change_for_delete = on_list_change.clone();
        let timers_for_delete = timers.clone();
        let list_box_for_delete = list_box.clone();
        let on_alarm_dismiss_for_delete = on_alarm_dismiss.clone();
        delete_button.connect_clicked(move |_| {
            alarms_for_delete.borrow_mut().retain(|a| a.id != alarm_id_for_delete);
            Self::refresh_alarm_list_static(
                &list_box_for_delete,
                &alarms_for_delete,
                &triggered_for_delete,
                &on_list_change_for_delete,
                &timers_for_delete,
                &on_alarm_dismiss_for_delete,
            );
            if let Some(ref cb) = *on_list_change_for_delete.borrow() {
                cb(&alarms_for_delete.borrow(), &timers_for_delete.borrow());
            }
        });
        hbox.append(&delete_button);

        // Enable toggle handler
        let alarm_id_for_toggle = alarm_id.clone();
        let alarms_for_toggle = alarms.clone();
        let on_list_change_for_toggle = on_list_change.clone();
        let timers_for_toggle = timers.clone();
        enabled_check.connect_toggled(move |check| {
            if let Some(alarm) = alarms_for_toggle.borrow_mut().iter_mut().find(|a| a.id == alarm_id_for_toggle) {
                alarm.enabled = check.is_active();
            }
            if let Some(ref cb) = *on_list_change_for_toggle.borrow() {
                cb(&alarms_for_toggle.borrow(), &timers_for_toggle.borrow());
            }
        });

        row.set_child(Some(&hbox));
        row
    }

    fn format_days(days: &[u32]) -> String {
        if days.is_empty() {
            return "Every day".to_string();
        }
        let day_names = ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"];
        let mut sorted_days = days.to_vec();
        sorted_days.sort();
        sorted_days.iter()
            .map(|&d| *day_names.get(d as usize).unwrap_or(&"?"))
            .collect::<Vec<_>>()
            .join(",")
    }

    fn show_alarm_edit_dialog(
        alarm: &AlarmConfig,
        alarms: Rc<RefCell<Vec<AlarmConfig>>>,
        triggered_ids: Rc<RefCell<HashSet<String>>>,
        on_list_change: Rc<RefCell<Option<OnListChangeCallback>>>,
        timers: Rc<RefCell<Vec<TimerConfig>>>,
        list_box: ListBox,
        on_alarm_dismiss: Rc<RefCell<Option<AlarmDismissCallback>>>,
    ) {
        let dialog = Window::builder()
            .title("Edit Alarm")
            .modal(true)
            .default_width(350)
            .default_height(400)
            .build();

        let alarm_id = alarm.id.clone();
        let vbox = GtkBox::new(Orientation::Vertical, 12);
        vbox.set_margin_start(12);
        vbox.set_margin_end(12);
        vbox.set_margin_top(12);
        vbox.set_margin_bottom(12);

        // Time
        let time_box = GtkBox::new(Orientation::Horizontal, 6);
        time_box.append(&Label::new(Some("Time:")));

        let hour_adj = Adjustment::new(alarm.hour as f64, 0.0, 23.0, 1.0, 1.0, 0.0);
        let hour_spin = SpinButton::new(Some(&hour_adj), 1.0, 0);
        time_box.append(&hour_spin);
        time_box.append(&Label::new(Some(":")));

        let min_adj = Adjustment::new(alarm.minute as f64, 0.0, 59.0, 1.0, 1.0, 0.0);
        let min_spin = SpinButton::new(Some(&min_adj), 1.0, 0);
        time_box.append(&min_spin);
        time_box.append(&Label::new(Some(":")));

        let sec_adj = Adjustment::new(alarm.second as f64, 0.0, 59.0, 1.0, 1.0, 0.0);
        let sec_spin = SpinButton::new(Some(&sec_adj), 1.0, 0);
        time_box.append(&sec_spin);
        vbox.append(&time_box);

        // Days
        vbox.append(&Label::new(Some("Days:")));
        let days_box = GtkBox::new(Orientation::Horizontal, 4);
        let day_names = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
        let day_checks: Vec<CheckButton> = day_names
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let check = CheckButton::with_label(name);
                check.set_active(alarm.days.contains(&(i as u32)));
                days_box.append(&check);
                check
            })
            .collect();
        vbox.append(&days_box);

        // Label
        let label_box = GtkBox::new(Orientation::Horizontal, 6);
        label_box.append(&Label::new(Some("Label:")));
        let label_entry = Entry::new();
        label_entry.set_text(&alarm.label.clone().unwrap_or_default());
        label_entry.set_hexpand(true);
        label_box.append(&label_entry);
        vbox.append(&label_box);

        // Sound enabled
        let sound_check = CheckButton::with_label("Play sound");
        sound_check.set_active(alarm.sound.enabled);
        vbox.append(&sound_check);

        // Visual flash
        let flash_check = CheckButton::with_label("Visual flash");
        flash_check.set_active(alarm.sound.visual_enabled);
        vbox.append(&flash_check);

        // Buttons
        let button_box = GtkBox::new(Orientation::Horizontal, 8);
        button_box.set_halign(gtk4::Align::End);

        let cancel_button = Button::with_label("Cancel");
        let dialog_for_cancel = dialog.clone();
        cancel_button.connect_clicked(move |_| dialog_for_cancel.close());
        button_box.append(&cancel_button);

        let save_button = Button::with_label("Save");
        save_button.add_css_class("suggested-action");
        let dialog_for_save = dialog.clone();
        save_button.connect_clicked(move |_| {
            // Update alarm
            if let Some(alarm) = alarms.borrow_mut().iter_mut().find(|a| a.id == alarm_id) {
                alarm.hour = hour_spin.value() as u32;
                alarm.minute = min_spin.value() as u32;
                alarm.second = sec_spin.value() as u32;

                let mut days = Vec::new();
                for (i, check) in day_checks.iter().enumerate() {
                    if check.is_active() {
                        days.push(i as u32);
                    }
                }
                alarm.days = days;

                let label_text = label_entry.text().to_string();
                alarm.label = if label_text.is_empty() { None } else { Some(label_text) };
                alarm.sound.enabled = sound_check.is_active();
                alarm.sound.visual_enabled = flash_check.is_active();
            }

            // Refresh list
            Self::refresh_alarm_list_static(
                &list_box,
                &alarms,
                &triggered_ids,
                &on_list_change,
                &timers,
                &on_alarm_dismiss,
            );

            // Notify change
            if let Some(ref cb) = *on_list_change.borrow() {
                cb(&alarms.borrow(), &timers.borrow());
            }

            dialog_for_save.close();
        });
        button_box.append(&save_button);
        vbox.append(&button_box);

        dialog.set_child(Some(&vbox));
        dialog.present();
    }

    /// Refresh the timer list display
    fn refresh_timer_list(&self) {
        Self::refresh_timer_list_static(
            &self.timer_list_box,
            &self.timers,
            &self.on_list_change,
            &self.alarms,
            &self.on_timer_action,
        );
    }

    fn refresh_timer_list_static(
        list_box: &ListBox,
        timers: &Rc<RefCell<Vec<TimerConfig>>>,
        on_list_change: &Rc<RefCell<Option<OnListChangeCallback>>>,
        alarms: &Rc<RefCell<Vec<AlarmConfig>>>,
        on_timer_action: &Rc<RefCell<Option<TimerActionCallback>>>,
    ) {
        // Clear existing rows
        while let Some(child) = list_box.first_child() {
            list_box.remove(&child);
        }

        let timers_borrowed = timers.borrow();

        for (idx, timer) in timers_borrowed.iter().enumerate() {
            let row = Self::create_timer_row(
                timer,
                idx,
                timers.clone(),
                on_list_change.clone(),
                alarms.clone(),
                list_box.clone(),
                on_timer_action.clone(),
            );
            list_box.append(&row);
        }

        if timers_borrowed.is_empty() {
            let empty_label = Label::new(Some("No timers configured"));
            empty_label.add_css_class("dim-label");
            empty_label.set_margin_top(12);
            empty_label.set_margin_bottom(12);
            let row = ListBoxRow::new();
            row.set_child(Some(&empty_label));
            row.set_selectable(false);
            row.set_activatable(false);
            list_box.append(&row);
        }
    }

    fn create_timer_row(
        timer: &TimerConfig,
        _idx: usize,
        timers: Rc<RefCell<Vec<TimerConfig>>>,
        on_list_change: Rc<RefCell<Option<OnListChangeCallback>>>,
        alarms: Rc<RefCell<Vec<AlarmConfig>>>,
        list_box: ListBox,
        on_timer_action: Rc<RefCell<Option<TimerActionCallback>>>,
    ) -> ListBoxRow {
        let row = ListBoxRow::new();
        row.set_selectable(false);
        row.set_activatable(false);

        let timer_id = timer.id.clone();

        let hbox = GtkBox::new(Orientation::Horizontal, 8);
        hbox.set_margin_start(8);
        hbox.set_margin_end(8);
        hbox.set_margin_top(8);
        hbox.set_margin_bottom(8);

        // Mode indicator
        let mode_str = match timer.mode {
            TimerMode::Countdown => "⏱",
            TimerMode::Stopwatch => "⏲",
        };
        let mode_label = Label::new(Some(mode_str));
        hbox.append(&mode_label);

        // Duration/time display
        let duration_str = if timer.mode == TimerMode::Countdown {
            let mins = timer.countdown_duration / 60;
            let secs = timer.countdown_duration % 60;
            format!("{:02}:{:02}", mins, secs)
        } else {
            let total_secs = timer.elapsed_ms / 1000;
            let mins = total_secs / 60;
            let secs = total_secs % 60;
            format!("{:02}:{:02}", mins, secs)
        };
        let duration_label = Label::new(Some(&duration_str));
        duration_label.add_css_class("heading");
        duration_label.set_width_chars(8);
        hbox.append(&duration_label);

        // State
        let state_str = match timer.state {
            TimerState::Stopped => "Stopped",
            TimerState::Running => "Running",
            TimerState::Paused => "Paused",
            TimerState::Finished => "Finished!",
        };
        let state_label = Label::new(Some(state_str));
        if timer.state == TimerState::Finished {
            state_label.add_css_class("error");
        } else if timer.state == TimerState::Running {
            state_label.add_css_class("success");
        }
        hbox.append(&state_label);

        // Label
        let label_text = timer.label.clone().unwrap_or_default();
        let label_label = Label::new(Some(&label_text));
        label_label.set_hexpand(true);
        label_label.set_halign(gtk4::Align::Start);
        label_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        hbox.append(&label_label);

        // Timer control buttons based on state
        let timer_id_for_action = timer_id.clone();
        let on_action_clone = on_timer_action.clone();

        match timer.state {
            TimerState::Stopped | TimerState::Finished => {
                let start_btn = Button::with_label("Start");
                start_btn.add_css_class("suggested-action");
                let tid = timer_id_for_action.clone();
                let on_act = on_action_clone.clone();
                start_btn.connect_clicked(move |_| {
                    if let Some(ref cb) = *on_act.borrow() {
                        cb(&tid, TimerAction::Start);
                    }
                });
                hbox.append(&start_btn);
            }
            TimerState::Running => {
                let pause_btn = Button::with_label("Pause");
                let tid = timer_id_for_action.clone();
                let on_act = on_action_clone.clone();
                pause_btn.connect_clicked(move |_| {
                    if let Some(ref cb) = *on_act.borrow() {
                        cb(&tid, TimerAction::Pause);
                    }
                });
                hbox.append(&pause_btn);

                let stop_btn = Button::with_label("Stop");
                stop_btn.add_css_class("destructive-action");
                let tid2 = timer_id_for_action.clone();
                let on_act2 = on_action_clone.clone();
                stop_btn.connect_clicked(move |_| {
                    if let Some(ref cb) = *on_act2.borrow() {
                        cb(&tid2, TimerAction::Stop);
                    }
                });
                hbox.append(&stop_btn);
            }
            TimerState::Paused => {
                let resume_btn = Button::with_label("Resume");
                resume_btn.add_css_class("suggested-action");
                let tid = timer_id_for_action.clone();
                let on_act = on_action_clone.clone();
                resume_btn.connect_clicked(move |_| {
                    if let Some(ref cb) = *on_act.borrow() {
                        cb(&tid, TimerAction::Resume);
                    }
                });
                hbox.append(&resume_btn);

                let stop_btn = Button::with_label("Stop");
                stop_btn.add_css_class("destructive-action");
                let tid2 = timer_id_for_action.clone();
                let on_act2 = on_action_clone.clone();
                stop_btn.connect_clicked(move |_| {
                    if let Some(ref cb) = *on_act2.borrow() {
                        cb(&tid2, TimerAction::Stop);
                    }
                });
                hbox.append(&stop_btn);
            }
        }

        // Edit button
        let edit_button = Button::with_label("Edit");
        let timer_clone = timer.clone();
        let timers_for_edit = timers.clone();
        let on_list_change_for_edit = on_list_change.clone();
        let alarms_for_edit = alarms.clone();
        let list_box_for_edit = list_box.clone();
        let on_timer_action_for_edit = on_timer_action.clone();
        edit_button.connect_clicked(move |_| {
            Self::show_timer_edit_dialog(
                &timer_clone,
                timers_for_edit.clone(),
                on_list_change_for_edit.clone(),
                alarms_for_edit.clone(),
                list_box_for_edit.clone(),
                on_timer_action_for_edit.clone(),
            );
        });
        hbox.append(&edit_button);

        // Delete button
        let delete_button = Button::with_label("Delete");
        delete_button.add_css_class("destructive-action");
        let timer_id_for_delete = timer_id.clone();
        let timers_for_delete = timers.clone();
        let on_list_change_for_delete = on_list_change.clone();
        let alarms_for_delete = alarms.clone();
        let list_box_for_delete = list_box.clone();
        let on_timer_action_for_delete = on_timer_action.clone();
        delete_button.connect_clicked(move |_| {
            timers_for_delete.borrow_mut().retain(|t| t.id != timer_id_for_delete);
            Self::refresh_timer_list_static(
                &list_box_for_delete,
                &timers_for_delete,
                &on_list_change_for_delete,
                &alarms_for_delete,
                &on_timer_action_for_delete,
            );
            if let Some(ref cb) = *on_list_change_for_delete.borrow() {
                cb(&alarms_for_delete.borrow(), &timers_for_delete.borrow());
            }
        });
        hbox.append(&delete_button);

        row.set_child(Some(&hbox));
        row
    }

    fn show_timer_edit_dialog(
        timer: &TimerConfig,
        timers: Rc<RefCell<Vec<TimerConfig>>>,
        on_list_change: Rc<RefCell<Option<OnListChangeCallback>>>,
        alarms: Rc<RefCell<Vec<AlarmConfig>>>,
        list_box: ListBox,
        on_timer_action: Rc<RefCell<Option<TimerActionCallback>>>,
    ) {
        let dialog = Window::builder()
            .title("Edit Timer")
            .modal(true)
            .default_width(350)
            .default_height(300)
            .build();

        let timer_id = timer.id.clone();
        let vbox = GtkBox::new(Orientation::Vertical, 12);
        vbox.set_margin_start(12);
        vbox.set_margin_end(12);
        vbox.set_margin_top(12);
        vbox.set_margin_bottom(12);

        // Mode
        let mode_box = GtkBox::new(Orientation::Horizontal, 6);
        mode_box.append(&Label::new(Some("Mode:")));
        let mode_options = StringList::new(&["Countdown", "Stopwatch"]);
        let mode_dropdown = DropDown::new(Some(mode_options), Option::<gtk4::Expression>::None);
        mode_dropdown.set_selected(match timer.mode {
            TimerMode::Countdown => 0,
            TimerMode::Stopwatch => 1,
        });
        mode_box.append(&mode_dropdown);
        vbox.append(&mode_box);

        // Duration
        let duration_box = GtkBox::new(Orientation::Horizontal, 6);
        duration_box.append(&Label::new(Some("Duration:")));
        let mins = timer.countdown_duration / 60;
        let secs = timer.countdown_duration % 60;

        let min_adj = Adjustment::new(mins as f64, 0.0, 1440.0, 1.0, 5.0, 0.0);
        let min_spin = SpinButton::new(Some(&min_adj), 1.0, 0);
        duration_box.append(&min_spin);
        duration_box.append(&Label::new(Some("min")));

        let sec_adj = Adjustment::new(secs as f64, 0.0, 59.0, 1.0, 5.0, 0.0);
        let sec_spin = SpinButton::new(Some(&sec_adj), 1.0, 0);
        duration_box.append(&sec_spin);
        duration_box.append(&Label::new(Some("sec")));
        vbox.append(&duration_box);

        // Label
        let label_box = GtkBox::new(Orientation::Horizontal, 6);
        label_box.append(&Label::new(Some("Label:")));
        let label_entry = Entry::new();
        label_entry.set_text(&timer.label.clone().unwrap_or_default());
        label_entry.set_hexpand(true);
        label_box.append(&label_entry);
        vbox.append(&label_box);

        // Sound enabled
        let sound_check = CheckButton::with_label("Play sound when finished");
        sound_check.set_active(timer.sound.enabled);
        vbox.append(&sound_check);

        // Buttons
        let button_box = GtkBox::new(Orientation::Horizontal, 8);
        button_box.set_halign(gtk4::Align::End);

        let cancel_button = Button::with_label("Cancel");
        let dialog_for_cancel = dialog.clone();
        cancel_button.connect_clicked(move |_| dialog_for_cancel.close());
        button_box.append(&cancel_button);

        let save_button = Button::with_label("Save");
        save_button.add_css_class("suggested-action");
        let dialog_for_save = dialog.clone();
        save_button.connect_clicked(move |_| {
            // Update timer
            if let Some(timer) = timers.borrow_mut().iter_mut().find(|t| t.id == timer_id) {
                timer.mode = match mode_dropdown.selected() {
                    0 => TimerMode::Countdown,
                    _ => TimerMode::Stopwatch,
                };
                timer.countdown_duration = (min_spin.value() as u64) * 60 + (sec_spin.value() as u64);

                let label_text = label_entry.text().to_string();
                timer.label = if label_text.is_empty() { None } else { Some(label_text) };
                timer.sound.enabled = sound_check.is_active();
            }

            // Refresh list
            Self::refresh_timer_list_static(
                &list_box,
                &timers,
                &on_list_change,
                &alarms,
                &on_timer_action,
            );

            // Notify change
            if let Some(ref cb) = *on_list_change.borrow() {
                cb(&alarms.borrow(), &timers.borrow());
            }

            dialog_for_save.close();
        });
        button_box.append(&save_button);
        vbox.append(&button_box);

        dialog.set_child(Some(&vbox));
        dialog.present();
    }

    /// Show the dialog
    pub fn present(&self) {
        self.window.present();
    }

    /// Set the callback for when alarm/timer lists change
    pub fn set_on_list_change<F: Fn(&[AlarmConfig], &[TimerConfig]) + 'static>(&self, callback: F) {
        *self.on_list_change.borrow_mut() = Some(Box::new(callback));
    }

    /// Set the callback for timer control actions
    pub fn set_on_timer_action<F: Fn(&str, TimerAction) + 'static>(&self, callback: F) {
        *self.on_timer_action.borrow_mut() = Some(Box::new(callback));
    }

    /// Set the callback for alarm dismiss action
    pub fn set_on_alarm_dismiss<F: Fn(&str) + 'static>(&self, callback: F) {
        *self.on_alarm_dismiss.borrow_mut() = Some(Box::new(callback));
    }

    /// Update triggered alarm IDs
    pub fn update_triggered_alarms(&self, triggered_ids: HashSet<String>) {
        *self.triggered_alarm_ids.borrow_mut() = triggered_ids;
        self.refresh_alarm_list();
    }

    /// Update timer states (for live display updates)
    pub fn update_timers(&self, new_timers: Vec<TimerConfig>) {
        *self.timers.borrow_mut() = new_timers;
        self.refresh_timer_list();
    }

    /// Get the window for external use
    pub fn window(&self) -> &Window {
        &self.window
    }
}
